//! Mother Fang's eggs are crushed by feet, never by weapon hits. The host owns the hatch
//! deadline; snapshots carry the same eggs/countdowns to every player.
use bevy::asset::RenderAssetUsages;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::{
    data,
    enemies::{self, Enemy},
    hero::Hero,
    mechanics,
    telegraphs::{Telegraphs, Tone},
    units::flat,
    Phase, Run, RunEntity,
};
use crate::net::Guid;

pub(super) static BROODLING: data::EnemyDef = data::EnemyDef {
    scale: 0.28,
    attack: 0.8,
    ..data::en("Fang Broodling", 7536, 500.0, 12.0, 30.0, 0, 10)
};
const CRUSH_RADIUS: f32 = 1.7;
const HATCH_SECONDS: f32 = 30.0;

#[derive(Default)]
pub(super) struct Poison {
    pub until: f32,
    pub next: f32,
    pub damage: f32,
}

pub(super) fn poison(run: &mut Run, damage: f32) {
    if run.dead_t.is_some() || run.immune_t > 0.0 || run.test_god {
        return;
    }
    let now = run.elapsed;
    if run.brood_poison.until <= now {
        run.brood_poison.next = now + 1.0;
    }
    run.brood_poison.until = now + 3.0;
    run.brood_poison.damage = damage.max(0.0);
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Egg {
    id: u64,
    owner: u64,
    pos: [f32; 3],
    born: f32,
    hatch: f32,
    health: f32,
    damage: f32,
}

#[derive(Resource, Default)]
pub(super) struct Eggs(pub Vec<Egg>);

impl Eggs {
    pub(super) fn objective(&self, now: f32) -> Option<(usize, f32, f32)> {
        let first = self.0.iter().min_by(|a, b| a.hatch.total_cmp(&b.hatch))?;
        Some((
            self.0.len(),
            (first.hatch - now).max(0.0),
            first.hatch - first.born,
        ))
    }
    pub(super) fn lay(
        &mut self,
        owner: u64,
        boss_pos: Vec3,
        count: u32,
        health: f32,
        damage: f32,
        run: &mut Run,
        ground: &super::dungeon::Ground,
        tele: &mut Telegraphs,
    ) {
        let now = run.elapsed;
        let offset = (now * 0.73) % std::f32::consts::TAU;
        for i in 0..count {
            let angle = offset + i as f32 / count as f32 * std::f32::consts::TAU;
            let pos = ground.spawn_spot(
                boss_pos + Vec3::new(angle.cos(), 0.0, angle.sin()) * 6.5,
                boss_pos.y,
            );
            self.0.push(Egg {
                id: run.guid(),
                owner,
                pos: pos.to_array(),
                born: now,
                hatch: now + HATCH_SECONDS,
                health: health * 0.08,
                damage: damage * 1.1,
            });
            tele.circle(pos, CRUSH_RADIUS, now, HATCH_SECONDS);
            tele.tone(Tone::Green);
        }
        run.yell = Some(("Mother Fang lays a clutch of eggs!".into(), 4.0));
    }
}

#[derive(Resource)]
struct Art {
    mesh: Handle<Mesh>,
    shell: Handle<StandardMaterial>,
    cracked: Handle<StandardMaterial>,
}
#[derive(Component)]
struct EggModel(u64);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Eggs>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(Phase::Menu), |mut eggs: ResMut<Eggs>| {
            eggs.0.clear()
        })
        .add_systems(
            Update,
            tick.after(enemies::run_scripts)
                .run_if(super::coop::authority)
                .run_if(super::coop::world_running)
                .run_if(|time: Res<Time>| time.delta_secs() > 0.0)
                .run_if(|loading: Res<crate::loading_screen::LoadingScreen>| !loading.covering()),
        )
        .add_systems(Update, draw.after(tick).after(super::hud::show_panels));
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut mesh = Sphere::new(1.0).mesh().uv(40, 28);
    let mut colors = Vec::new();
    if let Some(VertexAttributeValues::Float32x3(points)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        let light = Vec3::new(-0.5, 0.8, 0.4).normalize();
        for p in points {
            let normal = Vec3::from_array(*p).normalize_or(Vec3::Y);
            let shade = 0.42 + 0.55 * normal.dot(light).max(0.0);
            colors.push([shade, shade, shade, 1.0]);
            let taper = 1.0 - 0.22 * p[1];
            p[0] *= 0.52 * taper;
            p[2] *= 0.46 * taper;
            p[1] *= 0.75;
        }
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    let shell = images.add(shell_texture(false));
    let cracked = images.add(shell_texture(true));
    commands.insert_resource(Art {
        mesh: meshes.add(mesh),
        shell: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(shell),
            unlit: true,
            ..default()
        }),
        cracked: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(cracked),
            unlit: true,
            ..default()
        }),
    });
}

#[allow(clippy::type_complexity)]
pub(super) fn tick(
    mut commands: Commands,
    mut eggs: ResMut<Eggs>,
    mut run: ResMut<Run>,
    coop: Res<super::coop::Coop>,
    hero: Query<(Entity, &Transform), (With<Hero>, Without<Enemy>)>,
    allies: Query<(Entity, &super::coop::Ally)>,
    owners: Query<(&Guid, &Enemy)>,
    mut tele: ResMut<Telegraphs>,
) {
    let Ok((hero, tf)) = hero.single() else {
        return;
    };
    let players = mechanics::living(&run, &coop, (hero, tf.translation), &allies);
    let now = run.elapsed;
    eggs.0.retain(|egg| {
        let pos = Vec3::from_array(egg.pos);
        if !owners
            .iter()
            .any(|(id, enemy)| id.0 == egg.owner && !enemy.dead)
        {
            tele.remove_circle(pos, CRUSH_RADIUS);
            return false;
        }
        if now < egg.hatch
            && now >= egg.born + 0.25
            && players
                .iter()
                .any(|p| flat(p.pos - pos).length() <= CRUSH_RADIUS)
        {
            tele.remove_circle(pos, CRUSH_RADIUS);
            return false;
        }
        if now >= egg.hatch {
            tele.remove_circle(pos, CRUSH_RADIUS);
            let target = players
                .iter()
                .min_by(|a, b| {
                    a.pos
                        .distance_squared(pos)
                        .total_cmp(&b.pos.distance_squared(pos))
                })
                .map_or(tf.translation, |p| p.pos);
            let entity =
                enemies::spawn_enemy(&mut commands, &mut run, &BROODLING, pos, target, false);
            let (hp, damage) = (egg.health, egg.damage);
            commands.queue(move |world: &mut World| {
                if let Some(mut enemy) = world.get_mut::<Enemy>(entity) {
                    enemy.hp = hp;
                    enemy.max_hp = hp;
                    enemy.damage = damage;
                    enemy.speed = 12.0;
                    enemy.attack_cd = 0.5;
                    enemy.boss_minion = true;
                }
            });
            run.yell = Some((
                "An egg hatched! A Fang Broodling is hunting you!".into(),
                3.0,
            ));
            return false;
        }
        true
    });
}

fn shell_texture(cracked: bool) -> Image {
    const SIDE: u32 = 128;
    let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let u = x as f32 / SIDE as f32;
            let v = y as f32 / SIDE as f32;
            let hash = (x / 8).wrapping_mul(73856093) ^ (y / 8).wrapping_mul(19349663);
            let spot = hash % 7 == 0 && (x % 8).abs_diff(3).pow(2) + (y % 8).abs_diff(4).pow(2) < 5;
            let zigzag = 0.49 + ((u * 17.0).fract() - 0.5).abs() * 0.10;
            let crack = cracked
                && ((v - zigzag).abs() < 0.008
                    || ((u - 0.29).abs() < 0.006 && (0.38..zigzag).contains(&v))
                    || ((u - 0.72).abs() < 0.006 && (zigzag..0.66).contains(&v)));
            let rgb = if crack {
                [93, 87, 76]
            } else if spot {
                [188, 183, 169]
            } else {
                let grain = ((hash + x + y) % 7) as u8;
                [238 - grain, 235 - grain, 225 - grain]
            };
            pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    Image::new(
        Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn draw(
    mut commands: Commands,
    eggs: Res<Eggs>,
    run: Res<Run>,
    phase: Res<State<Phase>>,
    art: Res<Art>,
    mut models: Query<(
        Entity,
        &EggModel,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Transform,
    )>,
) {
    let live: HashSet<_> = if matches!(phase.get(), Phase::Menu | Phase::GameOver) {
        HashSet::new()
    } else {
        eggs.0.iter().map(|e| e.id).collect()
    };
    let mut model_ids = HashSet::new();
    for (entity, id, mut material, mut tf) in &mut models {
        let Some(egg) = eggs.0.iter().find(|e| e.id == id.0 && live.contains(&e.id)) else {
            commands.entity(entity).despawn();
            continue;
        };
        model_ids.insert(id.0);
        let urgent = egg.hatch - run.elapsed <= 5.0;
        material.0 = if urgent {
            art.cracked.clone()
        } else {
            art.shell.clone()
        };
        let pulse = if urgent {
            1.0 + 0.025 * (run.elapsed * 12.0).sin()
        } else {
            1.0
        };
        let progress = ((run.elapsed - egg.born) / (egg.hatch - egg.born)).clamp(0.0, 1.0);
        let growth = (1.0 + 0.25 * progress) * pulse;
        tf.scale = Vec3::splat(growth);
        tf.translation.y = egg.pos[1] + 0.75 * growth;
    }
    for egg in eggs
        .0
        .iter()
        .filter(|e| live.contains(&e.id) && !model_ids.contains(&e.id))
    {
        commands.spawn((
            RunEntity,
            EggModel(egg.id),
            Mesh3d(art.mesh.clone()),
            MeshMaterial3d(art.shell.clone()),
            Transform::from_translation(Vec3::from_array(egg.pos) + Vec3::Y * 0.75),
        ));
    }
}
