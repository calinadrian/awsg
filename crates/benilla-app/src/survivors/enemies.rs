//! The horde: spawning on a ring outside the view, the chase, contact damage, the casters' bolts,
//! the bosses' and guardians' scripts, and death with its experience and the odd meal it leaves.

use std::collections::HashMap;

use benilla_world::rig_anim::GlobalSeqDrive;
use bevy::prelude::*;

use super::abilities::Hit;
use super::data::{self, EnemyDef};
use super::hero::{self, HERO_RADIUS, Hero};
use super::units::{self, COLOR_HURT, Fx, Marks, UnitSpec, flat, yaw_toward};
use super::{Phase, Rng, Run, Selection};
use crate::creature_anim::{MovementState, move_flags};
use crate::net::{FieldChanged, Guid, ObjectStore};

#[derive(Component)]
pub(super) struct Enemy {
    pub def: &'static EnemyDef,
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub damage: f32,
    pub radius: f32,
    pub xp: u32,
    pub boss: bool,
    pub attack_cd: f32,
    pub slow: f32,
    pub slow_t: f32,
    pub root_t: f32,
    pub stun_t: f32,
    pub fear_t: f32,
    pub weaken: f32,
    pub weaken_t: f32,
    pub dots: Vec<Dot>,
    pub dead: bool,
    /// A guardian's lair: it sleeps there until the hero comes near, and goes back to it when he
    /// runs.
    pub home: Option<Home>,
    /// A charge under way: the wind-up, then the rush.
    pub charge: Option<Charge>,
    /// A regular Elwynn mob's next short move towards a slot around the hero.
    approach: Option<Approach>,
}

struct Approach {
    angle: f32,
    next: f32,
    goal: Vec3,
}

/// A scripted charge down a marked lane.
#[derive(Clone, Copy)]
pub(super) struct Charge {
    pub to: Vec3,
    pub spell: u32,
    pub damage: f32,
    /// Seconds still standing while the lane fills.
    pub windup: f32,
    /// Seconds the rush may still run.
    pub left: f32,
    pub hit: bool,
}

/// How long a charger stands while its lane fills, and how fast it runs.
const CHARGE_WINDUP: f32 = 1.1;
const CHARGE_SPEED: f32 = 20.0;

/// A blow marked on the ground, landing at `at` (run seconds).
#[derive(Clone, Copy)]
pub(super) struct Blast {
    at: f32,
    spot: Vec3,
    radius: f32,
    damage: f32,
    visual: Option<(Entity, u32, Option<Vec3>)>,
}

/// How long a stomp's circle shows before the ground shakes.
const STOMP_WINDUP: f32 = 1.2;

#[derive(Clone, Copy)]
pub(super) struct Home {
    pub pos: Vec3,
    pub awake: bool,
}

/// How near the hero comes before a guardian wakes.
const WAKE: f32 = 20.0;
/// How far from its lair a guardian follows him before it goes home and heals.
const LAIR_LEASH: f32 = 97.5;
/// Yards a second a caster's bolt flies.
const BOLT_SPEED: f32 = 24.0;

/// A boss's or a guardian's script: the next turn of [`EnemyDef::phases`] it waits on, and the
/// turns it has taken, each with its clock.
#[derive(Component, Default)]
pub(super) struct Script {
    guardian: super::guardian_behavior::Script,
    next: usize,
    running: Vec<(data::BossAction, f32)>,
}

pub(super) struct Dot {
    pub spell: u32,
    pub damage: f32,
    pub every: f32,
    pub drain: f32,
    pub left: f32,
    pub tick: f32,
}

/// A corpse playing its death before it fades.
#[derive(Component)]
pub(super) struct Dying(f32);

#[derive(Resource, Default)]
pub(super) struct Spawner {
    timer: f32,
}

/// An enemy died here, worth this much.
#[derive(Message, Clone, Copy)]
pub(super) struct Loot {
    pub pos: Vec3,
    pub xp: u32,
    pub boss: bool,
}

#[derive(Clone, Copy)]
pub(super) enum GemKind {
    Xp(u32),
    Heal(f32),
    Gold(u32),
}

#[derive(Component)]
pub(super) struct Gem {
    pub kind: GemKind,
    pulled: bool,
    speed: f32,
    target_peer: Option<u32>,
    flight: f32,
}

/// The display row and scale a pickup wears; XP uses `CreatureDisplayInfo`.
fn look_for(kind: GemKind) -> (u32, f32) {
    match kind {
        // Original XP wisps: larger display at 20 points (RVA 0x5c07fe9).
        GemKind::Xp(xp) => (if xp >= 20 { 16587 } else { 1825 }, 1.0),
        // `TurkeyLeg.mdx`.
        GemKind::Heal(_) => (114, 2.5),
        // Original gold-pile display 2614 at scale 0.6 (RVA 0x5c0839d).
        GemKind::Gold(_) => (2614, 0.6),
    }
}

/// Body radius: the big beasts need more room.
fn radius_for(def: &EnemyDef) -> f32 {
    (if def.big { 1.3 } else { 0.6 }) * def.scale
}

pub(super) fn spawn_enemy(
    commands: &mut Commands,
    run: &mut Run,
    def: &'static EnemyDef,
    pos: Vec3,
    hero: Vec3,
    boss: bool,
) -> Entity {
    // The old ASR's own numbers for its bosses and guardians stand as they are.
    let scale = if def.phases.is_empty() && !boss {
        data::hp_scale(run.elapsed)
    } else {
        1.0
    };
    spawn_with(commands, run, def, pos, hero, boss, scale, None)
}

/// A guardian asleep in its lair, scaled by run time and guardians already slain.
pub(super) fn spawn_guardian(
    commands: &mut Commands,
    run: &mut Run,
    def: &'static EnemyDef,
    pos: Vec3,
    face: Vec3,
    slain: u32,
) -> Entity {
    let home = Home { pos, awake: false };
    let e = spawn_with(commands, run, def, pos, face, false, 1.0, Some(home));
    let (hp, damage) = super::guardian_behavior::stats(def, run.elapsed, slain);
    commands.queue(move |world: &mut World| {
        if let Some(mut enemy) = world.get_mut::<Enemy>(e) {
            enemy.hp = hp;
            enemy.max_hp = hp;
            enemy.damage = damage;
        }
    });
    e
}

#[allow(clippy::too_many_arguments)]
fn spawn_with(
    commands: &mut Commands,
    run: &mut Run,
    def: &'static EnemyDef,
    pos: Vec3,
    hero: Vec3,
    boss: bool,
    scale: f32,
    home: Option<Home>,
) -> Entity {
    let serial = run.guid();
    let e = units::spawn_unit(
        commands,
        &UnitSpec {
            serial,
            display: def.display,
            scale: def.scale * if home.is_some() { 1.35 } else { 1.0 },
            level: def.level,
            faction: units::FACTION_MONSTER,
            bytes0: 1 << 8,
            weapons: &[],
            sheath: 0,
        },
        pos,
        yaw_toward(hero - pos),
    );
    // Only the ordinary Elwynn horde uses the reference's regular-mob curves.
    // Sample once at spawn so enemies already fighting do not gain health with the clock.
    let (hp, damage, speed) = if !boss && home.is_none() && super::elwynn::is_regular(def) {
        super::elwynn_behavior::regular_stats(def.hp, def.damage, def.speed, run.elapsed)
    } else {
        let damage = if home.is_some() {
            def.damage * scale
        } else {
            def.damage * data::damage_scale(run.elapsed)
        };
        (def.hp * scale, damage, def.speed)
    };
    commands.entity(e).insert((
        Enemy {
            def,
            hp,
            max_hp: hp,
            speed,
            damage,
            radius: radius_for(def) * if home.is_some() { 1.35 } else { 1.0 },
            xp: def.xp,
            boss,
            attack_cd: if home.is_some() {
                def.attack * 0.5
            } else {
                0.5
            },
            slow: 1.0,
            slow_t: 0.0,
            root_t: 0.0,
            stun_t: 0.0,
            fear_t: 0.0,
            weaken: 1.0,
            weaken_t: 0.0,
            dots: Vec::new(),
            dead: false,
            home,
            charge: None,
            approach: (home.is_some() || super::elwynn::is_regular(def)).then(|| {
                let side = flat(pos - hero);
                Approach {
                    angle: side.z.atan2(side.x),
                    next: 0.0,
                    goal: pos,
                }
            }),
        },
        Marks::debuffs(),
    ));
    if !def.phases.is_empty() || home.is_some() {
        commands.entity(e).insert(Script::default());
    }
    e
}

/// Elwynn keeps every unlocked regular mob; other timed rosters keep the four newest.
fn timed_pool(roster: &data::Roster, elapsed: f32) -> Vec<usize> {
    let come: Vec<usize> = (0..roster.enemies.len())
        .filter(|&i| roster.enemies[i].from <= elapsed)
        .collect();
    let skip = if roster
        .enemies
        .first()
        .is_some_and(|e| e.name == "Kobold Vermin")
    {
        0
    } else {
        come.len().saturating_sub(4)
    };
    come[skip..].to_vec()
}

fn choose_enemy<'a>(
    roster: &'a data::Roster,
    pool: &[usize],
    elapsed: f32,
    rng: &mut Rng,
) -> &'a EnemyDef {
    if roster
        .enemies
        .first()
        .is_some_and(|e| e.name == "Kobold Vermin")
    {
        let weights: Vec<f32> = pool
            .iter()
            .map(|&i| {
                super::elwynn_behavior::weight(
                    elapsed,
                    roster.enemies[i].from,
                    super::elwynn_behavior::COSTS[i],
                )
            })
            .collect();
        let at = super::elwynn_behavior::choose(&weights, rng.f32()).unwrap_or(0);
        &roster.enemies[pool[at]]
    } else {
        &roster.enemies[pool[rng.index(pool.len())]]
    }
}

/// Keep the horde topped up on a ring outside the view, with the bosses and the encircling waves
/// on the clock.
pub(super) fn spawn_enemies(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut spawner: ResMut<Spawner>,
    selection: Res<Selection>,
    hero: Query<&Transform, With<Hero>>,
    enemies: Query<&Enemy>,
    ground: super::dungeon::Ground,
    coop: Res<super::coop::Coop>,
) {
    let Ok(h) = hero.single() else { return };
    if run.dead_t.is_some() && !coop.living_guests() {
        return;
    }
    let hpos = if run.dead_t.is_some() {
        coop.players
            .values()
            .find(|p| p.peer != 0 && p.hp > 0.)
            .map_or(h.translation, |p| Vec3::from_array(p.pos))
    } else {
        h.translation
    };
    let elapsed = run.elapsed;
    let roster = data::roster(selection.map);
    let pool = if roster.timed {
        timed_pool(roster, elapsed)
    } else {
        data::wave_pool(elapsed, roster.enemies.len()).to_vec()
    };

    let due_bosses = if roster.timed {
        roster.bosses.iter().filter(|b| b.from <= elapsed).count() as u32
    } else {
        ((elapsed / data::BOSS_EVERY_SECS) as u32).min(roster.bosses.len() as u32)
    };
    while run.bosses < due_bosses {
        let def = &roster.bosses[run.bosses as usize];
        let a = rng.range(0.0, std::f32::consts::TAU);
        let pos = ground.spawn_spot(hpos + Vec3::new(a.cos(), 0.0, a.sin()) * 30.0, hpos.y);
        spawn_enemy(&mut commands, &mut run, def, pos, hpos, true);
        run.bosses += 1;
        run.banner = Some(if def.intro.is_empty() {
            (format!("{} approaches!", def.name), 4.0)
        } else {
            (def.intro.to_string(), 4.0)
        });
    }

    spawner.timer -= time.delta_secs();
    if spawner.timer > 0.0 {
        return;
    }
    spawner.timer = 0.12;
    let alive = enemies.iter().filter(|e| !e.dead).count();
    let target = data::target_alive(elapsed);
    if alive >= target {
        return;
    }
    let n = (((target - alive) as f32) / 6.0).ceil().min(4.0) as usize;
    for _ in 0..n {
        let def = choose_enemy(roster, &pool, elapsed, &mut rng);
        let a = rng.range(0.0, std::f32::consts::TAU);
        let r = rng.range(34.0, 42.0);
        let pos = ground.spawn_spot(hpos + Vec3::new(a.cos(), 0.0, a.sin()) * r, hpos.y);
        spawn_enemy(&mut commands, &mut run, def, pos, hpos, false);
    }
}

/// The chase: straight at the hero, pushed apart from each other, stopped by crowd control, and
/// swinging when in reach.
#[allow(clippy::type_complexity)]
pub(super) fn move_enemies(
    time: Res<Time>,
    mut run: ResMut<Run>,
    ground: super::dungeon::Ground,
    hero: Query<(Entity, &Transform), (With<Hero>, Without<Enemy>)>,
    mut enemies: Query<(Entity, &mut Transform, &mut Enemy, &mut MovementState), Without<Hero>>,
    mut fx: Fx,
    mut hits: MessageWriter<Hit>,
    selection: Res<Selection>,
    lairs: Res<super::guardians::Lairs>,
    mut bolts: Local<Vec<(f32, f32, u32)>>,
    coop: Res<super::coop::Coop>,
    allies: Query<(Entity, &super::coop::Ally)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let Ok((hero_e, htf)) = hero.single() else {
        return;
    };
    let hpos = htf.translation;
    let hero_down = run.dead_t.is_some();
    let elwynn = selection.stage_def().name == "Elwynn Forest";

    // Casters' bolts in flight land when due.
    let now = run.elapsed;
    let landed: Vec<(f32, u32)> = bolts
        .iter()
        .filter(|b| b.0 <= now)
        .map(|b| (b.1, b.2))
        .collect();
    bolts.retain(|b| b.0 > now);
    for (raw, peer) in landed {
        run.coop_target = peer;
        let taken = hero::hurt_hero(&mut run, raw);
        if taken > 0.0 {
            fx.number(hero_e, taken, false, COLOR_HURT);
        }
    }
    run.coop_target = 0;

    // Separation over a coarse grid: only neighbours in the same or adjacent cells push.
    const CELL: f32 = 3.0;
    let bodies: Vec<(Entity, Vec3, f32)> = enemies
        .iter()
        .filter(|(_, _, e, _)| !e.dead)
        .map(|(ent, tf, e, _)| (ent, tf.translation, e.radius))
        .collect();
    let key = |p: Vec3| ((p.x / CELL).floor() as i32, (p.z / CELL).floor() as i32);
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, b) in bodies.iter().enumerate() {
        grid.entry(key(b.1)).or_default().push(i);
    }
    let mut push: HashMap<Entity, Vec3> = HashMap::with_capacity(bodies.len());
    for (i, &(ent, pos, r)) in bodies.iter().enumerate() {
        let (cx, cz) = key(pos);
        let mut acc = Vec3::ZERO;
        for dx in -1..=1 {
            for dz in -1..=1 {
                let Some(cell) = grid.get(&(cx + dx, cz + dz)) else {
                    continue;
                };
                for &j in cell {
                    if j == i {
                        continue;
                    }
                    let (_, other, or) = bodies[j];
                    let d = flat(pos - other);
                    let len = d.length();
                    let min = r + or;
                    if len < min && len > 1e-4 {
                        acc += d / len * (min - len);
                    } else if len <= 1e-4 {
                        acc += Vec3::new((i as f32).sin(), 0.0, (i as f32).cos()) * 0.1;
                    }
                }
            }
        }
        push.insert(ent, acc);
    }

    for (ent, mut tf, mut e, mut mv) in &mut enemies {
        let e = &mut *e;
        if e.dead {
            if mv.speed != 0.0 || mv.flags != 0 {
                mv.speed = 0.0;
                mv.flags = 0;
            }
            continue;
        }
        e.attack_cd -= dt;
        for t in [
            &mut e.slow_t,
            &mut e.root_t,
            &mut e.stun_t,
            &mut e.fear_t,
            &mut e.weaken_t,
        ] {
            *t = (*t - dt).max(0.0);
        }
        if e.slow_t <= 0.0 {
            e.slow = 1.0;
        }
        if e.weaken_t <= 0.0 {
            e.weaken = 1.0;
        }
        let pos = tf.translation;
        let (hero_e, hpos, peer, hero_down) = coop.target(pos, (hero_e, hpos), !hero_down, &allies);
        run.coop_target = peer;
        let to_hero = flat(hpos - pos);
        let dist = to_hero.length();

        // A guardian sleeps in its lair until the hero comes near, follows him only so far, and
        // goes home to heal when he runs.
        let mut homing = None;
        if let Some(home) = e.home.as_mut() {
            let from_home = flat(hpos - home.pos).length();
            let back = flat(home.pos - pos);
            if home.awake && (from_home > LAIR_LEASH || hero_down) {
                // Native evade heals immediately and clears combat state before returning.
                home.awake = false;
                e.hp = e.max_hp;
                e.dots.clear();
                e.charge = None;
                e.root_t = 0.0;
                e.stun_t = 0.0;
                e.fear_t = 0.0;
                e.slow_t = 0.0;
                e.slow = 1.0;
                e.weaken_t = 0.0;
                e.weaken = 1.0;
            }
            if !home.awake {
                // Damage wakes a guardian even outside its normal proximity trigger.
                if !hero_down && (e.hp < e.max_hp || (back.length() <= 3.0 && dist <= WAKE)) {
                    home.awake = true;
                    // The native wake-event list refreshes stats once (RVA 0x5c0d688).
                    // Preserve the wound that woke it; do not heal or grow every frame.
                    let (hp, damage) = super::guardian_behavior::stats(e.def, now, lairs.slain);
                    if hp > e.max_hp {
                        e.hp = hp * (e.hp / e.max_hp).clamp(0.0, 1.0);
                        e.max_hp = hp;
                    }
                    e.damage = e.damage.max(damage);
                } else if back.length() > 2.0 {
                    homing = Some(back.normalize_or_zero());
                } else {
                    mv.speed = 0.0;
                    mv.flags = 0;
                    continue;
                }
            }
        } else if dist > 75.0 {
            // Strays far behind are brought back in front, as the reference genre does.
            let back = hpos + to_hero.normalize_or_zero() * 38.0;
            tf.translation = ground.spawn_spot(back, hpos.y);
            if let Some(approach) = e.approach.as_mut() {
                let side = flat(tf.translation - hpos);
                approach.angle = side.z.atan2(side.x);
                approach.next = 0.0;
            }
            continue;
        }

        // A charge: still while its lane fills, then a straight rush down it.
        if e.charge.is_some() && (homing.is_some() || e.stun_t > 0.0 || e.root_t > 0.0 || hero_down)
        {
            e.charge = None;
        }
        if let Some(c) = e.charge.as_mut() {
            let way = flat(c.to - pos);
            let mut speed = 0.0;
            let mut next = pos;
            if c.windup > 0.0 {
                c.windup -= dt;
                if c.windup <= 0.0 {
                    fx.cast(ent, c.spell, &[], None);
                }
            } else {
                speed = if e.home.is_some() { 40.0 } else { CHARGE_SPEED };
                let stride = (speed * dt).min(way.length());
                next = ground.at(pos + way.normalize_or_zero() * stride, pos.y);
                c.left -= dt;
            }
            if way != Vec3::ZERO {
                tf.rotation = Quat::from_rotation_y(yaw_toward(way.normalize()));
            }
            tf.translation = next;
            mv.speed = speed;
            mv.flags = if speed > 0.0 { move_flags::FORWARD } else { 0 };
            if c.windup <= 0.0 && !c.hit && {
                let reach = if e.home.is_some() {
                    (e.def.scale * 1.35 * 1.2).clamp(4.0, 7.0)
                } else {
                    HERO_RADIUS + e.radius + 0.6
                };
                // Sweep the whole stride so a 40 yd/s charge cannot skip the hero.
                let stride = flat(next - pos);
                let t = if stride.length_squared() > 0.0 {
                    (flat(hpos - pos).dot(stride) / stride.length_squared()).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                flat(hpos - (pos + stride * t)).length() <= reach
            } {
                c.hit = true;
                let taken = hero::hurt_hero(&mut run, c.damage);
                fx.swing(ent, hero_e, taken);
                if taken > 0.0 {
                    fx.number(hero_e, taken, false, COLOR_HURT);
                }
            }
            if c.windup <= 0.0 && (c.left <= 0.0 || way.length() < 0.5) {
                e.charge = None;
                e.attack_cd = e.attack_cd.max(0.6);
            }
            continue;
        }

        let held = e.stun_t > 0.0 || e.root_t > 0.0;
        let contact_reach = if e.home.is_some() {
            2.2 + 1.2 * e.def.scale * 1.35
        } else {
            HERO_RADIUS + e.radius + 0.35
        };
        let contact = dist <= contact_reach;
        // A caster stops once the hero is in its reach and throws from there.
        let in_reach = e
            .def
            .ranged
            .is_some_and(|r| dist <= r.range * if elwynn { 1.0 } else { 0.85 } && homing.is_none());
        let mut step = Vec3::ZERO;
        let mut speed = 0.0;
        let mut approach_distance = None;
        if let Some(dir) = homing {
            speed = e.speed;
            step = dir * speed;
            tf.rotation = Quat::from_rotation_y(yaw_toward(dir));
        } else if !held && !hero_down {
            let mut dir = if e.fear_t > 0.0 {
                -to_hero.normalize_or_zero()
            } else {
                to_hero.normalize_or_zero()
            };
            if elwynn && e.def.ranged.is_none() && e.fear_t <= 0.0 && !contact {
                let scale = e.def.scale;
                let speed = e.speed * e.slow;
                if let Some(approach) = e.approach.as_mut() {
                    if now >= approach.next {
                        // WoW XY -> Bevy XZ reverses the native angle's handedness.
                        approach.angle -= 0.15;
                        // The native minimum slot radius is 1.5. This renderer's smaller
                        // contact bodies need the slot inside melee reach to allow a swing.
                        let radius =
                            super::elwynn_behavior::approach_radius(scale).min(contact_reach * 0.8);
                        let goal = super::elwynn_behavior::approach_goal(
                            [pos.x, pos.z],
                            [hpos.x, hpos.z],
                            approach.angle,
                            radius,
                        );
                        approach.goal = Vec3::new(goal[0], pos.y, goal[1]);
                        approach.next = now
                            + super::elwynn_behavior::repath_after(
                                flat(approach.goal - pos).length(),
                                speed,
                            );
                    }
                    let remaining = flat(approach.goal - pos);
                    approach_distance = Some(remaining.length());
                    dir = remaining.normalize_or_zero();
                }
            }
            if (!contact && !in_reach) || e.fear_t > 0.0 {
                speed = e.speed * e.slow;
                if let Some(remaining) = approach_distance {
                    speed = speed.min(remaining / dt);
                }
                step = dir * speed;
            }
            if dir != Vec3::ZERO {
                let target = Quat::from_rotation_y(yaw_toward(dir));
                tf.rotation = tf.rotation.slerp(target, (dt * 10.0).min(1.0));
            }
        }
        let shove = push.get(&ent).copied().unwrap_or_default();
        let mut next = pos + step * dt + shove * (dt * 8.0).min(0.5);
        next = ground.at(next, pos.y);
        tf.translation = next;
        if speed > 0.0 {
            mv.speed = speed;
            mv.flags = move_flags::FORWARD;
        } else if mv.speed != 0.0 || mv.flags != 0 {
            mv.speed = 0.0;
            mv.flags = 0;
        }

        let free = homing.is_none()
            && e.stun_t <= 0.0
            && e.fear_t <= 0.0
            && e.attack_cd <= 0.0
            && !hero_down;
        if let Some(r) = e.def.ranged.filter(|_| free && in_reach && !contact) {
            e.attack_cd = r.cooldown;
            fx.cast(ent, r.spell, &[hero_e], None);
            bolts.push((now + dist / BOLT_SPEED, e.damage * e.weaken, peer));
        } else if contact && free {
            let swing = if e.def.attack > 0.0 {
                e.def.attack
            } else if e.boss {
                1.0
            } else {
                1.3
            };
            e.attack_cd = swing;
            let raw = e.damage * e.weaken;
            let taken = hero::hurt_hero(&mut run, raw);
            fx.swing(ent, hero_e, taken);
            if taken > 0.0 {
                fx.number(hero_e, taken, false, COLOR_HURT);
            }
            if peer == 0 && run.thorns_t > 0.0 {
                hits.write(Hit::direct(ent, run.thorns * run.stats().might, false));
            }
            if taken > 0.0 && peer == 0 {
                run.struck = true;
                let reflect = run.talents.stat(22) + run.jewel_stat(20);
                if reflect > 0.0 {
                    hits.write(Hit::direct(ent, raw * reflect, false));
                }
            }
        }
    }
    run.coop_target = 0;
}

/// Mark timers run down; the state kits reap with them.
pub(super) fn tick_marks(
    time: Res<Time>,
    mut units: Query<(Entity, &Guid, &mut ObjectStore, &mut Marks), With<Enemy>>,
    mut fields: MessageWriter<FieldChanged>,
) {
    let dt = time.delta_secs();
    for (entity, guid, mut store, mut marks) in &mut units {
        marks.tick(dt);
        marks.flush(entity, guid.0, &mut store, &mut fields);
    }
}

/// Corpses fade once their death has played.
pub(super) fn reap_dead(
    mut commands: Commands,
    time: Res<Time>,
    mut dying: Query<(Entity, &mut Dying)>,
) {
    for (e, mut d) in &mut dying {
        d.0 += time.delta_secs();
        if d.0 > 2.5 {
            commands
                .entity(e)
                .remove::<Dying>()
                .insert(benilla_world::model_fade::DespawnFade::default());
        }
    }
}

pub(super) fn mark_dying(commands: &mut Commands, e: Entity) {
    commands.entity(e).insert(Dying(0.0));
}

/// Original ASR leaves XP wisps at the kill; collection, not death, credits the hero.
pub(super) fn drop_loot(
    mut commands: Commands,
    mut drops: MessageReader<Loot>,
    mut rng: ResMut<Rng>,
    mut run: ResMut<Run>,
    gems: Query<(Entity, &Transform, &Gem)>,
    mut coop: ResMut<super::coop::Coop>,
) {
    if drops.is_empty() {
        return;
    }
    // Original ASR merges into the nearest loose XP wisp once 45 exist
    // (RVA 0x5c07f63..0x5c08096 and 0x5c08687). Include this frame's queued drops.
    let mut wisps: Vec<_> = gems
        .iter()
        .filter_map(|(e, tf, gem)| match gem.kind {
            GemKind::Xp(xp) if !gem.pulled => Some((Some(e), tf.translation, xp)),
            _ => None,
        })
        .collect();
    let mut xp_changed = false;
    for d in drops.read() {
        // Regular mobs pay 1.65x, stochastically rounded; bosses pay their table XP
        // unchanged (RVA 0x5c03cd9..0x5c03dc3). No global 2x multiplier.
        let xp = if d.boss {
            d.xp
        } else {
            let scaled = d.xp as f32 * 1.65;
            (scaled as u32).saturating_add(u32::from(rng.f32() < scaled.fract()))
        };
        if xp > 0 {
            xp_changed = true;
            if wisps.len() >= 45 {
                if let Some((_, _, stored)) = wisps.iter_mut().min_by(|a, b| {
                    flat(a.1 - d.pos)
                        .length_squared()
                        .total_cmp(&flat(b.1 - d.pos).length_squared())
                }) {
                    *stored = stored.saturating_add(xp);
                }
            } else {
                wisps.push((None, d.pos, xp));
            }
        }
        if let Some(amount) = super::gold::drop_amount(d.boss, rng.f32(), rng.f32()) {
            spawn_gem(
                &mut commands,
                run.guid(),
                d.pos + Vec3::X * 0.8,
                GemKind::Gold(amount),
            );
            info!("survivors gold: dropped base={amount} balance={}", run.gold);
        }
        if d.boss {
            coop.cache(d.pos);
            super::caches::spawn(&mut commands, run.guid(), d.pos, false);
        }
        if d.boss || rng.f32() < 0.015 {
            let heal = GemKind::Heal(if d.boss { 1000.0 } else { 30.0 });
            spawn_gem(&mut commands, run.guid(), d.pos + Vec3::X * 0.8, heal);
        }
    }
    if xp_changed {
        for (entity, pos, xp) in wisps {
            if let Some(e) = entity {
                commands.entity(e).insert(Gem {
                    kind: GemKind::Xp(xp),
                    pulled: false,
                    speed: 6.0,
                    target_peer: None,
                    flight: 0.0,
                });
            } else {
                spawn_gem(&mut commands, run.guid(), pos, GemKind::Xp(xp));
            }
        }
    }
}

/// Bosses and guardians take their turns as their health falls, and keep up the ones that
/// repeat: the stomps, the bombs and the calls for help.
#[allow(clippy::type_complexity)]
pub(super) fn run_scripts(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    selection: Res<Selection>,
    ground: super::dungeon::Ground,
    hero: Query<(Entity, &Transform), (With<Hero>, Without<Enemy>)>,
    mut scripted: Query<(Entity, &Transform, &mut Enemy, &mut Script), Without<Hero>>,
    mut fx: Fx,
    mut tele: ResMut<super::telegraphs::Telegraphs>,
    // Blows marked on the ground: when they land, where, how wide, how hard, and the visual they
    // land with (its caster, spell, and the spot it plays at, or on the caster).
    mut blasts: Local<Vec<Blast>>,
    coop: Res<super::coop::Coop>,
    allies: Query<(Entity, &super::coop::Ally)>,
) {
    let dt = time.delta_secs();
    let Ok((hero_e, htf)) = hero.single() else {
        return;
    };
    let hpos = htf.translation;
    let now = run.elapsed;
    let roster = data::roster(selection.map);

    // Marked blows land when their circles fill.
    let due: Vec<Blast> = blasts.iter().filter(|b| b.at <= now).copied().collect();
    blasts.retain(|b| b.at > now);
    for Blast {
        spot,
        radius,
        damage,
        visual,
        ..
    } in due
    {
        if let Some((caster, spell, dest)) = visual {
            fx.cast(caster, spell, &[], dest);
        }
        if flat(hpos - spot).length() <= radius {
            let taken = hero::hurt_hero(&mut run, damage);
            if taken > 0.0 {
                fx.number(hero_e, taken, false, COLOR_HURT);
            }
        }
        for p in coop
            .players
            .values()
            .filter(|p| p.peer != 0 && p.hp > 0. && p.loaded)
        {
            if flat(Vec3::from_array(p.pos) - spot).length() <= radius {
                run.coop_damage.push((p.peer, damage));
            }
        }
    }

    let pool = if roster.timed {
        timed_pool(roster, now)
    } else {
        data::wave_pool(now, roster.enemies.len()).to_vec()
    };
    for (e, tf, mut en, mut script) in &mut scripted {
        let (hero_e, hpos, peer, _) = coop.target(
            tf.translation,
            (hero_e, hpos),
            run.dead_t.is_none(),
            &allies,
        );
        run.coop_target = peer;
        if en.home.is_some() {
            super::guardian_behavior::tick(
                &mut script.guardian,
                &mut en,
                e,
                tf.translation,
                hero_e,
                hpos,
                dt,
                &mut commands,
                &mut run,
                &mut rng,
                roster,
                &ground,
                &mut fx,
                &mut tele,
            );
            continue;
        }
        if en.dead {
            continue;
        }
        let share = en.hp / en.max_hp;
        let def = en.def;
        while let Some(phase) = def.phases.get(script.next).filter(|p| share <= p.below) {
            script.next += 1;
            if !phase.yell.is_empty() {
                run.yell = Some((format!("{} yells: {}", def.name, phase.yell), 4.0));
            }
            // A yelled turn fires at once, then on its own clock; a guardian's quiet kit from the
            // start is staggered a few seconds apart, so no two of its moves come at once.
            let first = if phase.yell.is_empty() {
                2.0 + 3.5 * script.running.len() as f32
            } else {
                0.0
            };
            script.running.push((phase.action, first));
        }
        let pos = tf.translation;
        let damage_scale = data::damage_scale(now);
        let mut calls = Vec::new();
        for (action, wait) in &mut script.running {
            *wait -= dt;
            if *wait > 0.0 {
                continue;
            }
            match *action {
                data::BossAction::Stomp {
                    spell,
                    radius,
                    damage,
                    every,
                } => {
                    // Only with the hero close; it plants its feet while the circle fills.
                    if flat(hpos - pos).length() > radius + 6.0 || en.charge.is_some() {
                        *wait = 1.0;
                        continue;
                    }
                    *wait = every;
                    en.root_t = en.root_t.max(STOMP_WINDUP);
                    tele.circle(pos, radius, now, STOMP_WINDUP);
                    blasts.push(Blast {
                        at: now + STOMP_WINDUP,
                        spot: pos,
                        radius,
                        damage: damage * damage_scale,
                        visual: Some((e, spell, None)),
                    });
                }
                data::BossAction::Bombs {
                    spell,
                    count,
                    radius,
                    damage,
                    every,
                } => {
                    *wait = every;
                    for _ in 0..count {
                        let a = rng.range(0.0, std::f32::consts::TAU);
                        let r = rng.range(0.0, 6.0);
                        let at = ground.at(hpos + Vec3::new(a.cos(), 0.0, a.sin()) * r, hpos.y);
                        tele.circle(at, radius, now, 1.4);
                        blasts.push(Blast {
                            at: now + 1.4,
                            spot: at,
                            radius,
                            damage: damage * damage_scale,
                            visual: Some((e, spell, Some(at))),
                        });
                    }
                }
                data::BossAction::Charge {
                    spell,
                    damage,
                    every,
                } => {
                    let way = flat(hpos - pos);
                    let d = way.length();
                    // Only from a run-up: too close or too far, it tries again shortly.
                    if !(4.0..=24.0).contains(&d) || en.stun_t > 0.0 || en.root_t > 0.0 {
                        *wait = 1.0;
                        continue;
                    }
                    *wait = every;
                    let to = ground.at(pos + way / d * (d + 3.0), hpos.y);
                    let rush = (d + 3.0) / CHARGE_SPEED + 0.3;
                    tele.lane(
                        pos,
                        to,
                        en.radius.max(1.0) + 0.5,
                        now,
                        CHARGE_WINDUP,
                        CHARGE_WINDUP + rush,
                    );
                    en.charge = Some(Charge {
                        to,
                        spell,
                        damage: damage * damage_scale,
                        windup: CHARGE_WINDUP,
                        left: rush,
                        hit: false,
                    });
                }
                data::BossAction::Throw {
                    spell,
                    radius,
                    damage,
                    every,
                } => {
                    let d = flat(hpos - pos).length();
                    if d > 28.0 || en.stun_t > 0.0 || en.charge.is_some() {
                        *wait = 1.0;
                        continue;
                    }
                    *wait = every;
                    let at = ground.at(hpos, hpos.y);
                    fx.cast(e, spell, &[], Some(at));
                    // In the air long enough to see it coming and step out.
                    let flight = (d / 14.0).clamp(1.2, 1.8);
                    tele.circle(at, radius, now, flight);
                    blasts.push(Blast {
                        at: now + flight,
                        spot: at,
                        radius,
                        damage: damage * damage_scale,
                        visual: None,
                    });
                }
                data::BossAction::Summon { foe, count, every } => {
                    // Once only, unless it repeats.
                    *wait = if every > 0.0 { every } else { f32::INFINITY };
                    if foe.is_empty() {
                        for _ in 0..count {
                            if !pool.is_empty() {
                                calls.push((&roster.enemies[pool[rng.index(pool.len())]], 1));
                            }
                        }
                    } else if let Some(def) = roster.enemies.iter().find(|d| d.name == foe) {
                        calls.push((def, count));
                    }
                }
            }
        }
        for (def, count) in calls {
            for i in 0..count {
                let a = i as f32 / count as f32 * std::f32::consts::TAU;
                let at = ground.spawn_spot(pos + Vec3::new(a.cos(), 0.0, a.sin()) * 3.0, pos.y);
                spawn_enemy(&mut commands, &mut run, def, at, hpos, false);
            }
        }
    }
    run.coop_target = 0;
}

pub(super) fn spawn_gem(commands: &mut Commands, serial: u64, pos: Vec3, kind: GemKind) -> Entity {
    let (display, scale) = look_for(kind);
    let e = match kind {
        GemKind::Xp(_) => {
            let e = units::spawn_unit(
                commands,
                &UnitSpec {
                    serial,
                    display,
                    scale,
                    level: 1,
                    faction: 35,
                    bytes0: 0,
                    weapons: &[],
                    sheath: 0,
                },
                pos,
                0.0,
            );
            commands
                .entity(e)
                .insert((super::loot_ground::GroundedLoot, Visibility::Hidden));
            #[cfg(not(target_os = "macos"))]
            commands.entity(e).insert(crate::blob_shadow::NoBlobShadow);
            e
        }
        GemKind::Gold(_) => super::loot_ground::spawn(commands, serial, display, scale, pos),
        GemKind::Heal(_) => {
            units::spawn_object(commands, serial, display, scale, pos + Vec3::Y * 0.5)
        }
    };
    commands.entity(e).insert(Gem {
        kind,
        pulled: false,
        speed: 6.0,
        target_peer: None,
        flight: 0.0,
    });
    e
}

/// XP and meals accelerate toward the hero after entering pickup range.
pub(super) fn collect_gems(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    hero: Query<&Transform, (With<Hero>, Without<Gem>)>,
    mut gems: Query<(Entity, &mut Transform, &mut Gem, &Visibility)>,
    emitters: Query<(Entity, &benilla_world::particles::ParticleEmitter)>,
    mut coop: ResMut<super::coop::Coop>,
) {
    let Ok(h) = hero.single() else { return };
    if run.dead_t.is_some() && !coop.living_guests() {
        return;
    }
    let dt = time.delta_secs();
    let stats = run.stats();
    let target = h.translation + Vec3::Y * 1.0;
    let mut collected_xp = 0u32;
    for (e, mut tf, mut g, visible) in &mut gems {
        if *visible == Visibility::Hidden {
            continue;
        }
        let candidates = || {
            coop.players
                .values()
                .filter(|p| p.peer != 0 && p.loaded && !p.paused && p.hp > 0.)
                .map(|p| (p.peer, Vec3::from_array(p.pos) + Vec3::Y))
                .chain((run.dead_t.is_none() && !run.coop_unavailable.contains(&0)).then_some((0, target)))
        };
        // Once attracted, keep the recipient instead of switching between nearby
        // co-op players every frame. Retarget only if that player is no longer alive.
        let recipient = g
            .target_peer
            .and_then(|peer| candidates().find(|p| p.0 == peer));
        let (target_peer, target) = recipient
            .or_else(|| {
                candidates().min_by(|a, b| {
                    a.1.distance_squared(tf.translation)
                        .total_cmp(&b.1.distance_squared(tf.translation))
                })
            })
            .unwrap_or((0, target));
        let d = target - tf.translation;
        let dist = d.length();
        if let GemKind::Gold(base) = g.kind {
            // As in the original, gold is collected within pickup range. Distant piles
            // stay put; only collection credits the balance and applies gold-find.
            if flat(d).length() <= stats.magnet {
                let earned = super::gold::collected(base, run.jewel_stat(13));
                if run.dead_t.is_none() {
                    run.gold = run.gold.saturating_add(earned);
                }
                coop.reward(0, base);
                info!(
                    "survivors gold: collected base={base} earned={earned} balance={}",
                    run.gold
                );
                commands.entity(e).despawn();
            }
            continue;
        }
        if !g.pulled && flat(d).length() <= stats.magnet {
            g.pulled = true;
            g.target_peer = Some(target_peer);
            // Attracted wisps leave the floor and follow the hero even outside pickup range.
            commands
                .entity(e)
                .remove::<super::loot_ground::GroundedLoot>();
        }
        if !g.pulled {
            continue;
        }
        g.target_peer = Some(target_peer);
        g.flight += dt;
        g.speed += 30.0 * dt;
        // Credit on this frame's arrival, not the next frame (which may be
        // paused by a level-up). Contact is planar so floor/capsule height
        // differences cannot leave a wisp floating at shoulder height.
        if flat(d).length() < 0.9 || dist <= g.speed * dt || g.flight >= 0.75 {
            match g.kind {
                GemKind::Xp(base) => {
                    collected_xp = collected_xp.saturating_add(base);
                    // Model emitters are separate world entities. Free them with the pickup,
                    // including when receipt immediately pauses the run for a level-up.
                    for (emitter_entity, emitter) in &emitters {
                        if emitter.emit_host() == Some(e) || emitter.anchor() == Some(e) {
                            commands.entity(emitter_entity).despawn();
                        }
                    }
                }
                GemKind::Heal(hp) => {
                    if target_peer == 0 {
                        run.heal(hp);
                    } else {
                        run.coop_damage.push((target_peer, -hp));
                    }
                }
                GemKind::Gold(_) => {
                    unreachable!("gold handled before pickup attraction")
                }
            }
            commands.entity(e).despawn();
            continue;
        }
        tf.translation += d / dist * (g.speed * dt).min(dist);
    }
    // Original collects a frame's base XP, applies growth, then rounds once, minimum 1
    // (RVA 0x5c5a1d4..0x5c5a378). Stored XP remains whole points.
    if collected_xp > 0 {
        coop.reward(collected_xp, 0);
        if run.dead_t.is_none() {
            run.xp += (collected_xp as f32 * stats.growth).round().max(1.0);
        }
    }
    while run.level < data::MAX_LEVEL && run.xp >= data::xp_to_next(run.level) {
        run.xp -= data::xp_to_next(run.level);
        run.level += 1;
        run.pending_levels += 1;
    }
    if run.pending_levels > 0 && !run.coop_unavailable.contains(&0) {
        next.set(Phase::LevelUp);
    }
}

/// Pickups hang still: a wisp's own loop (its circling motes and their trails) stops on its first
/// frame, clip and global sequences alike, as soon as the model attaches.
pub(super) fn still_gems(
    mut players: Query<&mut AnimationPlayer, (With<Gem>, Added<AnimationPlayer>)>,
    mut drives: Query<&mut GlobalSeqDrive, (With<Gem>, Added<GlobalSeqDrive>)>,
) {
    for mut player in &mut players {
        for (_, active) in player.playing_animations_mut() {
            active.set_speed(0.0);
        }
    }
    for mut drive in &mut drives {
        drive.set_paused(true);
    }
}
