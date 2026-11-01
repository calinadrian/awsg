//! The hero: its body, WASD movement over the terrain, the top-down camera, and its health.

use benilla_world::view::WorldCamera;
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

use super::data::{self, ClassDef};
use super::units::{self, flat, wow_to_bevy, yaw_toward, Fx, Marks, UnitSpec};
use super::{Phase, Run, Selection};
use crate::creature_anim::{move_flags, MovementState};
use crate::net::{FieldChanged, Guid, ObjectStore};

#[derive(Component)]
pub(super) struct Hero {
    /// Vertical velocity in yards/second while airborne; 0 on the ground.
    pub(super) vy: f32,
    /// Height above the ground while airborne. Kept here, not in the transform: the creature
    /// ground clamp (`net::motion::ground_clamp_creatures`) re-seats every unit on the floor at
    /// the top of the frame, so the arc is re-applied over the ground each frame.
    pub(super) air: f32,
}

/// The body radius enemies stop at.
pub(super) const HERO_RADIUS: f32 = 0.7;

/// The fall back to the ground off the controller (`player/state.rs`'s `GRAVITY`).
const GRAVITY: f32 = 19.291_105;

/// The menu's and the auto-run's camera: the menu frames the preview hero; an auto-run follows
/// behind the driven hero. In play by hand the player controller's camera takes over
/// ([`super::drive`]).
#[derive(Resource)]
pub(super) struct CamRig {
    pub dist: f32,
    /// Camera azimuth (radians) around the hero: 0 puts it on +Z, behind a hero facing −Z.
    pub yaw: f32,
    /// Elevation above the horizon (radians).
    pub pitch: f32,
}

impl Default for CamRig {
    fn default() -> Self {
        Self {
            dist: 14.0,
            yaw: 0.0,
            pitch: 22.0 * std::f32::consts::PI / 180.0,
        }
    }
}

/// Where a battleground's runs start.
pub(super) fn arena(map: &data::MapDef) -> Vec3 {
    wow_to_bevy(map.center)
}

/// The world is the battleground being stood in ([`Selection::stage`]): entering one on another
/// continent worldports, which raises the loading screen until the new ground is in.
pub(super) fn follow_map(
    selection: Res<Selection>,
    current: Option<ResMut<benilla_world::world_map::CurrentMap>>,
) {
    let Some(mut current) = current else {
        return;
    };
    let want = selection.stage_def().map;
    if current.0 != want {
        current.0 = want;
    }
}

/// The hero in the class trainer's look, weapons drawn.
pub(super) fn spawn_hero(
    commands: &mut Commands,
    class: &'static ClassDef,
    display: u32,
    race: u8,
    sex: u8,
    serial: u64,
    pos: Vec3,
) -> Entity {
    let bytes0 = u32::from(race)
        | u32::from(class.id) << 8
        | u32::from(sex) << 16
        | u32::from(class.power) << 24;
    let e = units::spawn_unit(
        commands,
        &UnitSpec {
            serial,
            display,
            scale: 1.0,
            level: 60,
            faction: units::FACTION_FRIENDLY,
            bytes0,
            weapons: class.weapons,
            sheath: class.sheath,
        },
        pos,
        0.0,
    );
    commands
        .entity(e)
        .insert((Hero { vy: 0.0, air: 0.0 }, Marks::buffs()));
    // Survivors auto-casts do not spend power; publish the class's initial resource pool
    // through the same unit fields the player and target frames read.
    let power = class.power;
    commands.queue(move |world: &mut World| {
        if let Some(mut store) = world.get_mut::<ObjectStore>(e) {
            let (current, maximum) = match power {
                1 => (0, 1000),
                3 => (100, 100),
                _ => (600, 600),
            };
            store
                .0
                .merge(benilla_protocol::messages::ObjectFields::from_pairs(&[
                    (23 + u16::from(power), current),
                    (29 + u16::from(power), maximum),
                ]));
        }
    });
    e
}

/// Midday over the battleground, whatever the clock says.
pub(super) const MINUTE_OF_DAY: u32 = 780;

pub(super) fn daylight(debug: Option<ResMut<benilla_world::dev_state::DebugState>>) {
    let Some(mut debug) = debug else { return };
    if debug.lighting.follow_server_time || debug.lighting.manual_minute != MINUTE_OF_DAY {
        debug.lighting.follow_server_time = false;
        debug.lighting.manual_minute = MINUTE_OF_DAY;
    }
}

/// The ground under the mouse cursor, `None` with the cursor off the window. The locked camera's
/// hero faces it.
#[derive(Resource, Default)]
pub(super) struct AimPoint(pub(super) Option<Vec3>);

/// Cast the cursor onto the terrain: the camera ray meets the plane at the hero's height, then the
/// terrain height there, a few times over, which settles on hills within a frame.
pub(super) fn aim_cursor(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cams: Query<(&Camera, &Transform), With<WorldCamera>>,
    hero: Query<&Transform, (With<Hero>, Without<WorldCamera>)>,
    world: super::dungeon::Ground,
    mut aim: ResMut<AimPoint>,
) {
    let point = (|| {
        let cursor = windows.single().ok()?.cursor_position()?;
        let (cam, tf) = cams.single().ok()?;
        let ray = cam
            .viewport_to_world(&GlobalTransform::from(*tf), cursor)
            .ok()?;
        let mut y = hero.single().ok()?.translation.y;
        let mut at = None;
        for _ in 0..4 {
            if ray.direction.y.abs() < 1e-4 {
                break;
            }
            let t = (y - ray.origin.y) / ray.direction.y;
            if t <= 0.0 {
                break;
            }
            let p = ray.origin + *ray.direction * t;
            at = Some(p);
            match world.height(p) {
                Some(h) if (h - y).abs() > 0.05 => y = h,
                _ => break,
            }
        }
        at
    })();
    if aim.0 != point {
        aim.0 = point;
    }
}

/// The hero off the controller: an auto-run's steering, the jump arc and the streaming focus at
/// the menu and between picks. While a run is played by hand the player controller owns all of it
/// ([`super::drive`]), so this stands aside.
#[allow(clippy::too_many_arguments)]
pub(super) fn drive_hero(
    time: Res<Time>,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    selection: Res<Selection>,
    mut rig: ResMut<CamRig>,
    world: super::dungeon::Ground,
    auto: Option<Res<super::auto::Auto>>,
    mut player: ResMut<crate::player::Player>,
    mut hero: Query<(&mut Transform, &mut MovementState, &mut Hero)>,
    driven: Option<Res<crate::run_mode::SelfDriven>>,
    entry: Res<super::drive::EntryPending>,
) {
    if super::drive::driving(&phase, &run, auto.is_some()) && driven.is_some() {
        return;
    }
    let focus = match hero.single_mut() {
        Ok((mut tf, mut mv, mut hm)) => {
            let dt = time.delta_secs();
            let playing = *phase.get() == Phase::Playing && run.dead_t.is_none();
            let dir = auto
                .as_deref()
                .filter(|_| playing)
                .map_or(Vec3::ZERO, super::auto::steer);
            let mut pos = tf.translation;
            if let Some(d) = dir.try_normalize().filter(|_| dt > 0.0) {
                let target = Quat::from_rotation_y(yaw_toward(d));
                tf.rotation = tf.rotation.slerp(target, (dt * 14.0).min(1.0));
                // The auto-run's camera follows him around.
                rig.yaw = yaw_toward(d);
                let speed = run.stats().speed;
                pos += d * speed * dt;
                mv.speed = speed;
                mv.flags = move_flags::FORWARD;
            } else if mv.speed != 0.0 || mv.flags != 0 {
                mv.speed = 0.0;
                mv.flags = 0;
            }
            let map = selection.stage_def();
            let center = arena(map);
            let off = flat(pos - center);
            if off.length() > map.leash * 2.0 {
                // Just entered another battleground: start at its centre.
                pos = center;
            } else if off.length() > map.leash {
                let held = center + off.normalize() * map.leash;
                pos.x = held.x;
                pos.z = held.z;
            }
            let ground = world.height(pos).unwrap_or(pos.y);
            if playing && (hm.vy != 0.0 || hm.air > 0.0) {
                hm.vy -= GRAVITY * dt;
                hm.air += hm.vy * dt;
                if hm.air <= 0.0 {
                    hm.air = 0.0;
                    hm.vy = 0.0;
                }
            } else {
                hm.air = 0.0;
                hm.vy = 0.0;
            }
            pos.y = ground + hm.air;
            mv.vertical_speed = hm.vy;
            if tf.translation != pos {
                tf.translation = pos;
            }
            pos
        }
        Err(_) => {
            let mut p = arena(selection.stage_def());
            if let Some(h) = world.height(p) {
                p.y = h;
            }
            p
        }
    };
    // The terrain streams and the loading screen waits on the ground under this.
    player.active = true;
    player.detached = false;
    player.settling = entry.0;
    player.world_stale = entry.0;
    player.pos = focus;
}

/// Let go of the mouse buttons when the page loses focus or the pointer leaves it. A release that
/// lands elsewhere (alt-tab mid-drag, a browser or OS popup) never reaches the game, so the button
/// would read as held and the unlocked camera would swing with every move until both buttons were
/// clicked again. Survivors only: the engine's mouse-look keeps its buttons across a deactivate,
/// as 1.12.1 does.
pub(super) fn drop_stray_buttons(
    mut focus: MessageReader<bevy::window::WindowFocused>,
    mut left: MessageReader<bevy::window::CursorLeft>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
) {
    let lost = focus.read().any(|f| !f.focused);
    if lost | (left.read().count() > 0) {
        mouse.release_all();
    }
}

/// A slow orbit at the menu, and behind an auto-run's hero; in play by hand, the level-up pick,
/// the pause and the results it leaves the camera where the controller put it.
pub(super) fn place_camera(
    phase: Res<State<Phase>>,
    rig: Res<CamRig>,
    auto: Option<Res<super::auto::Auto>>,
    player: Res<crate::player::Player>,
    world: WorldPoint,
    walls: avian3d::prelude::SpatialQuery,
    hero: Query<&Transform, (With<Hero>, Without<WorldCamera>)>,
    mut cams: Query<&mut Transform, With<WorldCamera>>,
) {
    let menu = *phase.get() == Phase::Menu;
    if !menu && auto.is_none() {
        return;
    }
    // With no hero yet, the focus `drive_hero` grounded.
    let focus = hero.single().map_or(player.pos, |t| t.translation);
    let (eye, look) = if menu {
        // Keep the hero facing the viewer while the player chooses their appearance.
        let angle = std::f32::consts::PI + 0.18;
        let d = 7.0;
        (
            focus + Vec3::new(angle.sin() * d, 2.4, angle.cos() * d),
            focus + Vec3::Y * 1.1,
        )
    } else {
        let (sy, cy) = rig.yaw.sin_cos();
        let (sp, cp) = rig.pitch.sin_cos();
        let look = focus + Vec3::Y * 1.4;
        let mut eye = look + Vec3::new(cp * sy, sp, cp * cy) * rig.dist;
        // Never under the ground: a low tilt on a slope would bury the lens.
        if let Some(h) = world.terrain_height_under(eye) {
            eye.y = eye.y.max(h + 0.6);
        }
        // Nor inside a wall: underground the boom stops short of the rock behind the hero.
        if let Ok(dir) = Dir3::new(eye - look) {
            let reach = (eye - look).length();
            let filter = benilla_world::collision::WorldCollision::body_filter();
            if let Some(hit) = walls.cast_ray(look, dir, reach, true, &filter) {
                eye = look + *dir * (hit.distance - 0.4).max(0.5);
            }
        }
        (eye, look)
    };
    let placed = Transform::from_translation(eye).looking_at(look, Vec3::Y);
    for mut cam in &mut cams {
        *cam = placed;
    }
}

/// Damage the hero takes from one blow, after immunity, armour and the shield; returns what got
/// through.
pub(super) fn hurt_hero(run: &mut Run, raw: f32) -> f32 {
    if run.coop_run && run.coop_unavailable.contains(&run.coop_target) { return 0.; }
    if run.coop_target != 0 {
        run.coop_damage.push((run.coop_target, raw));
        return raw;
    }
    if run.immune_t > 0.0 || run.dead_t.is_some() {
        return 0.0;
    }
    let mut dmg = (raw - run.stats().armor).max(1.0)
        * (1.0 - run.talents.stat(5) - run.jewel_stat(5)).max(0.3);
    if run.shield_t > 0.0 && run.shield > 0.0 {
        let absorbed = dmg.min(run.shield);
        run.shield -= absorbed;
        dmg -= absorbed;
    }
    run.hp -= dmg;
    dmg
}

/// The clock, regeneration, the timed buffs and their visuals, death and victory.
pub(super) fn hero_vitals(
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    mut hero: Query<(Entity, &Guid, &mut ObjectStore, &mut Marks), With<Hero>>,
    mut fields: MessageWriter<FieldChanged>,
    mut fx: Fx,
    selection: Res<super::Selection>,
    mut rng: ResMut<super::Rng>,
    coop: Res<super::coop::Coop>,
) {
    let dt = time.delta_secs();
    let stats = run.stats();
    if std::mem::take(&mut run.struck) && run.hp > 0.0 && run.dead_t.is_none() {
        super::talents::trigger(&mut run, &mut rng, 8, None);
    }
    if run.hp > 0.0 && run.dead_t.is_none() {
        super::talents::low_health(&mut run, &mut rng);
    }
    let Ok((entity, guid, mut store, mut marks)) = hero.single_mut() else {
        return;
    };
    if let Some(t) = run.dead_t.as_mut() {
        *t += dt;
        if *t > 3.0 && !coop.active() {
            next.set(Phase::GameOver);
        }
        run.hp = 0.0;
        if coop.active() && !coop.guest() {
            run.elapsed += dt;
        }
        return;
    }
    run.elapsed += dt;
    for buff in &mut run.buffs {
        buff.3 -= dt;
    }
    run.buffs.retain(|buff| buff.3 > 0.0);
    let r = &mut *run;
    for t in [&mut r.shield_t, &mut r.immune_t, &mut r.thorns_t] {
        *t = (*t - dt).max(0.0);
    }
    let mut heal = stats.regen * dt;
    if run.heal_t > 0.0 {
        heal += run.heal_rate * dt.min(run.heal_t);
        run.heal_t = (run.heal_t - dt).max(0.0);
    }
    run.heal(heal);

    // The buffs show as their own state kits: the shield bubble, the immunity glow, the orbs.
    let mut shown: Vec<u32> = Vec::new();
    if run.shield_t > 0.0 && run.shield > 0.0 {
        shown.push(17);
    }
    if run.immune_t > 0.0 {
        shown.push(642);
    }
    if run.thorns_t > 0.0 {
        shown.push(run.thorns_spell);
    }
    if run.heal_t > 0.0 {
        shown.push(774);
    }
    for owned in &run.abilities {
        if owned.def.key == "lightning_shield" {
            shown.push(owned.def.spell);
        }
    }
    if run.heal_t > 0.0 {
        shown.push(139);
    }
    marks.set(&shown);
    marks.flush(entity, guid.0, &mut store, &mut fields);

    if run.hp <= 0.0 && run.revives_used < (run.jewel_stat(16) as u32).min(2) {
        run.revives_used += 1;
        run.hp = stats.max_hp * 0.5;
        run.immune_t = 3.0;
        run.banner = Some(("Soulstone: returned to life".into(), 3.0));
    }
    if run.hp <= 0.0 {
        run.hp = 0.0;
        run.dead_t = Some(0.0);
        run.heal_t = 0.0;
        if coop.active() {
            run.banner = Some((
                "Fallen: a teammate can revive you within 5 yards for 3 seconds".into(),
                8.0,
            ));
        }
        units::kill_unit(&mut store);
        marks.clear();
        marks.flush(entity, guid.0, &mut store, &mut fields);
    } else if if data::roster(selection.map).timed {
        // The old ASR's battlegrounds are won by slaying their last boss.
        run.slain_last && !coop.guest()
    } else {
        run.elapsed >= data::VICTORY_SECS && !coop.guest()
    } {
        run.won = true;
        // A last flourish before the results.
        fx.cast(entity, 642, &[entity], None);
        next.set(Phase::GameOver);
    }
}

pub(super) fn reach(hero: Vec3, other: Vec3) -> f32 {
    flat(other - hero).length()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::window::{CursorLeft, WindowFocused};

    fn held_left() -> World {
        let mut world = World::new();
        world.init_resource::<Messages<WindowFocused>>();
        world.init_resource::<Messages<CursorLeft>>();
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);
        world.insert_resource(mouse);
        world
    }

    fn left_held(world: &World) -> bool {
        world
            .resource::<ButtonInput<MouseButton>>()
            .pressed(MouseButton::Left)
    }

    #[test]
    fn a_held_button_is_dropped_when_the_page_loses_focus() {
        let mut world = held_left();
        world.run_system_once(drop_stray_buttons).unwrap();
        assert!(left_held(&world), "nothing happened: the drag goes on");
        world.write_message(WindowFocused {
            window: Entity::PLACEHOLDER,
            focused: true,
        });
        world.run_system_once(drop_stray_buttons).unwrap();
        assert!(left_held(&world), "gaining focus is no release");
        world.write_message(WindowFocused {
            window: Entity::PLACEHOLDER,
            focused: false,
        });
        world.run_system_once(drop_stray_buttons).unwrap();
        assert!(!left_held(&world));
    }

    #[test]
    fn a_held_button_is_dropped_when_the_pointer_leaves_the_page() {
        let mut world = held_left();
        world.write_message(CursorLeft {
            window: Entity::PLACEHOLDER,
        });
        world.run_system_once(drop_stray_buttons).unwrap();
        assert!(!left_held(&world));
    }
}
