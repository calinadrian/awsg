//! The horde: spawning on a ring outside the view, the chase, contact damage and dodgeable caster
//! spells, the bosses' and guardians' scripts, and death with its experience and the odd meal it
//! leaves.

use std::collections::HashMap;

use benilla_world::rig_anim::GlobalSeqDrive;
use bevy::prelude::*;

use super::abilities::Hit;
use super::data::{self, EnemyDef};
use super::hero::{self, Hero, HERO_RADIUS};
use super::units::{self, flat, yaw_toward, Fx, Marks, UnitSpec, COLOR_HURT};
use super::{Phase, Rng, Run, Selection};
use crate::creature_anim::{move_flags, Engaged, MovementState};
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
    /// Hogger's health-triggered, latched enrage; also replicated for his buff and body look.
    pub hogger_enraged: bool,
    pub enrage_at: f32,
    pub timed_enraged: bool,
    pub attack_cd: f32,
    /// Script-owned cast hold; separate from hostile roots and recovery windows.
    pub script_hold_until: f32,
    /// A ranged cast winding up: Elwynn casters then release a bolt, others a marked blast.
    cast: Option<RangedCast>,
    pub slow: f32,
    pub slow_t: f32,
    pub root_t: f32,
    pub stun_t: f32,
    pub fear_t: f32,
    pub weaken: f32,
    pub weaken_t: f32,
    pub exposed: f32,
    pub exposed_until: f32,
    pub last_hit: (u32, u32, u8),
    pub dots: Vec<Dot>,
    pub dead: bool,
    /// A guardian's lair: it sleeps there until the hero comes near, and goes back to it when he
    /// runs.
    pub home: Option<Home>,
    /// A charge under way: the wind-up, then the rush.
    pub charge: Option<Charge>,
    /// An elite's affixes, as [`super::mechanics::affix`] bits.
    pub affixes: u16,
    /// Untouchable while its guards stand.
    pub ward: bool,
    /// Raised to hold up a master's ward: every spell goes for it first.
    pub guard: bool,
    pub boss_minion: bool,
    /// Holding in melee reach: it stands and swings until the hero steps clear.
    melee: bool,
    /// Wears [`Engaged`] for the Ready stance while it holds in melee.
    engaged: bool,
    /// The level it shows, set from the hero's at spawn.
    pub level: u32,
    /// Its render size against the row's scale: below 1 where [`cap_heights`] shrank it.
    pub size: f32,
    /// A silver: a bigger, tougher horde mob that pays a chest and extra XP, with no bar or kit.
    pub silver: bool,
}

/// A silver's health, damage, size and XP against its horde row.
const SILVER_HP: f32 = 5.0;
const SILVER_DAMAGE: f32 = 1.6;
const SILVER_SCALE: f32 = 1.4;
const SILVER_XP: u32 = 3;

/// Enemies take the hero's level when they spawn, elites and bosses three above it, capped at 63.
/// Native 0x5bff540 writes `min(hero + 3 × elite, 63)` to UNIT_FIELD_LEVEL; the boss (0x5c13564)
/// and lair (0x5c664ba) spawns pass the elite flag, as does a roaming row's +0x4c.
/// A silver stands one above the hero.
fn scaled_level(hero: u32, role: Role) -> u32 {
    let above = match role {
        Role::Horde => 0,
        Role::Silver => 1,
        Role::Roaming | Role::Lair | Role::Boss => 3,
    };
    (hero + above).min(63)
}

/// Every chaser's pace against its row speed. Native 0x5c0a1e2 sends each chaser an 8-yard move
/// lasting `8 / (0.8 × speed)` seconds (0x1461bbf70 = 0.8, 0x146f873f0 = 8.0); the boss
/// (0x5c139a7) and lair (0x5c66a0a) spawns join the same record list, so it holds for them too.
/// Charges and the walk home keep their own speeds.
const CHASE_PACE: f32 = 0.8;

/// How far past its reach the hero may step before a mob in melee gives chase again.
const MELEE_LEEWAY: f32 = 1.25;

impl Enemy {
    pub(super) fn enrage_damage(&self) -> f32 {
        if self.timed_enraged { super::boss_enrage::DAMAGE } else { 1.0 }
    }
    fn enrage_speed(&self) -> f32 {
        if self.timed_enraged { super::boss_enrage::SPEED } else { 1.0 }
    }
    pub(super) fn enrage_pending(&mut self) {
        if let Some(charge) = self.charge.as_mut() { charge.damage *= super::boss_enrage::DAMAGE; }
        if let Some(cast) = self.cast.as_mut() { cast.damage *= super::boss_enrage::DAMAGE; }
    }
    /// A lair guardian not yet woken: spells pass it by until the hero comes near or strikes it.
    pub(super) fn asleep(&self) -> bool {
        self.home.is_some_and(|h| !h.awake)
    }

    /// Spells pass it by: asleep in its lair, or warded behind its guards.
    pub(super) fn untargetable(&self) -> bool {
        self.asleep() || self.ward
    }
    /// Automatic spells focus boss adds, then bosses, elites, then the ordinary horde.
    pub(super) fn priority(&self) -> u8 {
        if self.boss_minion || self.guard { 3 }
        else if self.boss { 2 }
        else if self.home.is_some() || self.def.roaming_elite || self.silver { 1 }
        else { 0 }
    }

    pub(super) fn relocated(&mut self, _pos: Vec3, _center: Vec3) {
        self.cast = None;
        self.melee = false;
    }
}

#[derive(Clone, Copy)]
struct RangedCast {
    until: f32,
    center: Vec3,
    damage: f32,
    spell: u32,
    radius: f32,
    /// Elwynn's casters release a bolt at their target instead of a blast at `center`.
    bolt: bool,
}

/// A released bolt in flight. Native 0x5c0cc31 queues the hit at release; it lands on its target
/// after the caster-to-target distance over the spell's speed, even if the caster has died.
#[derive(Clone, Copy)]
struct Bolt {
    released: f32,
    at: f32,
    peer: u32,
    damage: f32,
}

#[derive(Resource, Default)]
pub(super) struct Bolts(Vec<Bolt>);

/// Spell-speed lookup 0x5c6a380: Fireball 24 yd/s, Frostbolt 28 yd/s.
fn bolt_speed(spell: u32) -> f32 {
    if spell == 116 { 28.0 } else { 24.0 }
}

/// A scripted charge down a marked lane.
#[derive(Clone, Copy)]
pub(super) struct Charge {
    /// Princess charges apply their own hit consequences.
    pub boar_impact: Option<super::princess::Impact>,
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
    owner: Entity,
    damage_factor: f32,
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
/// The warning footprint for a ranged spell; the same 3.5-yard circle used by Taskmaster bombs.
const RANGED_BLAST_RADIUS: f32 = 3.5;

/// A boss's or a guardian's script: the next turn of [`EnemyDef::phases`] it waits on, and the
/// turns it has taken, each with its clock.
#[derive(Component, Default)]
pub(super) struct Script {
    guardian: super::guardian_behavior::Script,
    hogger: super::hogger::Script,
    pub(super) morgan: super::morgan::Script,
    pub(super) princess: super::princess::Script,
    next: usize,
    running: Vec<(data::BossAction, f32)>,
    /// Display telemetry follows the interpreter's clocks, including retries and windups.
    pub(super) cooldowns: Vec<super::boss_cooldowns::Timer>,
}

impl Script {
    pub(super) fn enrage_pending(&mut self) {
        self.hogger.enrage_pending();
        self.princess.enrage_pending();
        self.morgan.enrage_pending();
    }
}

#[derive(Clone)]
pub(super) struct Dot {
    pub source: Option<super::damage_meter::Source>,
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
    source: super::horde_spawning::Spawner,
}

/// An enemy died here, worth this much.
#[derive(Message, Clone)]
pub(super) struct Loot {
    pub pos: Vec3,
    pub xp: u32,
    pub boss: bool,
    pub boss_name: Option<String>,
    /// The creature's roster name, for achievement counts.
    pub name: &'static str,
    pub spawn_cost: f32,
    pub roaming_elite: bool,
    pub elite: bool,
    pub silver: bool,
}

#[derive(Clone, Copy)]
pub(super) enum GemKind {
    Xp(u32),
    Heal(f32),
    Gold(u32),
    Magnet,
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
        GemKind::Magnet => (0, 1.0),
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
    if !boss && (def.cost > 0.0 || def.roaming_elite) {
        if let Some(slots) = run.horde_spawn_slots.as_mut() {
            if *slots == 0 {
                return Entity::PLACEHOLDER;
            }
            *slots -= 1;
        }
    }
    let role = if boss {
        Role::Boss
    } else if def.roaming_elite {
        Role::Roaming
    } else {
        Role::Horde
    };
    spawn_with(commands, run, def, pos, hero, role, 0, None)
}

/// A silver from a horde row, taking one of the horde's spawn slots.
pub(super) fn spawn_silver(
    commands: &mut Commands,
    run: &mut Run,
    def: &'static EnemyDef,
    pos: Vec3,
    hero: Vec3,
) -> Entity {
    if let Some(slots) = run.horde_spawn_slots.as_mut() {
        if *slots == 0 {
            return Entity::PLACEHOLDER;
        }
        *slots -= 1;
    }
    spawn_with(commands, run, def, pos, hero, Role::Silver, 0, None)
}

/// How an enemy entered the fight, which picks its stat curve.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Role {
    /// An ordinary chaser from the roster's horde rows.
    Horde,
    /// A horde row sent as a silver: bigger and tougher, no bar or kit.
    Silver,
    /// An elite row sent with the horde, away from any lair.
    Roaming,
    /// An elite holding a lair.
    Lair,
    Boss,
}

/// Health, damage and speed for an enemy spawning now, before co-op scaling. Sampled once at
/// spawn so enemies already fighting do not gain health with the clock.
/// Every battleground uses Elwynn's curves; its tier multiplies them.
pub(super) fn spawn_stats(def: &EnemyDef, role: Role, elapsed: f32, slain: u32) -> (f32, f32, f32) {
    let tier = super::difficulty::factors(def.tier);
    match role {
        Role::Horde => {
            let (hp, damage, speed) =
                super::elwynn_behavior::regular_stats(def.hp, def.damage, def.speed, elapsed);
            (hp * tier.normal_hp, damage * tier.normal_damage, speed)
        }
        Role::Silver => {
            let (hp, damage, speed) =
                super::elwynn_behavior::regular_stats(def.hp, def.damage, def.speed, elapsed);
            (
                hp * tier.normal_hp * SILVER_HP,
                damage * tier.normal_damage * SILVER_DAMAGE,
                speed,
            )
        }
        Role::Roaming => {
            let (hp, damage, speed) =
                super::elwynn_behavior::roaming_elite_stats(def.hp, def.damage, def.speed, elapsed);
            (hp * tier.elite_hp, damage * tier.elite_damage, speed)
        }
        Role::Lair => {
            let (hp, damage) = super::guardian_behavior::stats(def, elapsed, slain);
            (hp * tier.elite_hp, damage * tier.elite_damage, def.speed)
        }
        Role::Boss => {
            let (hp, damage) = super::elwynn_behavior::boss_stats(def.hp, def.damage);
            (hp * tier.boss_hp, damage * tier.boss_damage, def.speed)
        }
    }
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
    if let Some(slots) = run.horde_spawn_slots.as_mut() {
        *slots = slots.saturating_sub(1);
    }
    spawn_with(commands, run, def, pos, face, Role::Lair, slain, Some(home))
}

#[allow(clippy::too_many_arguments)]
fn spawn_with(
    commands: &mut Commands,
    run: &mut Run,
    def: &'static EnemyDef,
    pos: Vec3,
    hero: Vec3,
    role: Role,
    slain: u32,
    home: Option<Home>,
) -> Entity {
    let boss = role == Role::Boss;
    let silver = role == Role::Silver;
    let grow = if home.is_some() {
        1.35
    } else if silver {
        SILVER_SCALE
    } else {
        1.0
    };
    let level = scaled_level(run.level, role);
    let serial = run.guid();
    let e = units::spawn_unit(
        commands,
        &UnitSpec {
            serial,
            display: def.display,
            scale: def.scale * grow,
            level,
            faction: units::FACTION_MONSTER,
            bytes0: 1 << 8,
            weapons: if boss && def.name == "Morgan the Collector" {
                super::morgan::WEAPONS
            } else {
                &[]
            },
            sheath: if boss && def.name == "Morgan the Collector" { 2 } else { 0 },
        },
        pos,
        yaw_toward(hero - pos),
    );
    let (hp, damage, speed) = spawn_stats(def, role, run.elapsed, slain);
    // An elite past Elwynn rolls its affixes from its serial; a juggernaut stands a third taller.
    let affixes = if matches!(role, Role::Lair | Role::Roaming) {
        super::mechanics::draw(def.affixes, serial)
    } else {
        0
    };
    let hp = if affixes & super::mechanics::affix::JUGGERNAUT != 0 {
        hp * 1.35
    } else {
        hp
    };
    // Co-op health follows the living party at spawn; damage is not party-scaled.
    let hp = hp * run.party_factor(if boss || home.is_some() { 1.04 } else { 0.72 });
    let hp = if role == Role::Horde && def.cost > 0.0 && super::low_memory::enabled(run) {
        hp * super::low_memory::HEALTH
    } else { hp };
    commands.entity(e).insert((
        Enemy {
            def,
            hp,
            max_hp: hp,
            speed,
            damage,
            radius: radius_for(def) * grow,
            xp: def.xp * if silver { SILVER_XP } else { 1 },
            boss,
            hogger_enraged: false,
            enrage_at: if boss { run.elapsed + super::boss_enrage::SECONDS } else { 0.0 },
            timed_enraged: false,
            attack_cd: if home.is_some() {
                def.attack * 0.5
            } else {
                0.5
            },
            script_hold_until: 0.0,
            cast: None,
            slow: 1.0,
            slow_t: 0.0,
            root_t: 0.0,
            stun_t: 0.0,
            fear_t: 0.0,
            weaken: 1.0,
            weaken_t: 0.0,
            exposed: 0.0,
            exposed_until: 0.0,
            last_hit: (0, 0, 0),
            dots: Vec::new(),
            dead: false,
            home,
            charge: None,
            affixes,
            ward: false,
            guard: false,
            boss_minion: false,
            melee: false,
            engaged: false,
            level,
            size: 1.0,
            silver,
        },
        Marks::debuffs(),
    ));
    if !def.phases.is_empty() || home.is_some() || def.roaming_elite
        || (boss && matches!(def.name, "Hogger" | "Princess" | "Morgan the Collector"))
        || std::ptr::eq(def, &super::princess::STAMPEDE_BOAR) {
        commands.entity(e).insert(Script::default());
    }
    if let Some(kit) = def.script.filter(|_| boss) {
        commands.entity(e).insert(super::mechanics::Mechanics::boss(kit));
    } else if affixes != 0 {
        commands.entity(e).insert(super::mechanics::Mechanics::elite(affixes));
    }
    e
}

/// Every horde row unlocked by now: older rows stay in the mix.
fn unlocked(roster: &data::Roster, elapsed: f32) -> Vec<usize> {
    (0..roster.enemies.len())
        .filter(|&i| roster.enemies[i].from <= elapsed)
        .collect()
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
    mut enemies: Query<(&mut Enemy, &mut Transform), Without<Hero>>,
    ground: super::dungeon::Ground,
    coop: Res<super::coop::Coop>,
) {
    let Ok(h) = hero.single() else { return };
    // Test Grounds spawns only what its panel asks for.
    if run.test || (run.dead_t.is_some() && !coop.living_guests()) {
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
    let due_bosses = roster.bosses.iter().filter(|b| b.from <= elapsed).count() as u32;
    let first_boss = run.bosses as usize;
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

    let new_bosses = &roster.bosses[first_boss..run.bosses as usize];
    super::horde_spawning::spawn(
        &mut commands,
        time.delta_secs(),
        &mut run,
        &mut rng,
        &mut spawner.source,
        roster,
        new_bosses,
        h.translation,
        &mut enemies,
        &ground,
        &coop,
    );
}

#[derive(Default)]
pub(super) struct Separation {
    bodies: Vec<(Entity, Vec3, f32)>,
    grid: HashMap<(i32, i32), usize>,
    cells: Vec<Vec<usize>>,
    push: HashMap<Entity, Vec3>,
}

/// The chase: straight at the hero, pushed apart from each other, stopped by crowd control, and
/// swinging when in reach.
#[allow(clippy::type_complexity)]
pub(super) fn move_enemies(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    ground: super::dungeon::Ground,
    hero: Query<(Entity, &Transform), (With<Hero>, Without<Enemy>)>,
    mut enemies: Query<(Entity, &mut Transform, &mut Enemy, &mut MovementState), Without<Hero>>,
    mut fx: Fx,
    mut hits: MessageWriter<Hit>,
    lairs: Res<super::guardians::Lairs>,
    mut tele: ResMut<super::telegraphs::Telegraphs>,
    coop: Res<super::coop::Coop>,
    allies: Query<(Entity, &super::coop::Ally)>,
    mut bolts: ResMut<Bolts>,
    mut separation: Local<Separation>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let Ok((local_hero_e, htf)) = hero.single() else {
        return;
    };
    let hpos = htf.translation;
    let hero_down = run.dead_t.is_some();

    let now = run.elapsed;

    // Bolts from an earlier run's clock are stale; due ones strike their target if it still stands.
    bolts.0.retain(|b| b.released <= now);
    let (due, flying): (Vec<Bolt>, Vec<Bolt>) = bolts.0.drain(..).partition(|b| b.at <= now);
    bolts.0 = flying;
    for bolt in due {
        let target = if bolt.peer == 0 {
            let standing = run.dead_t.is_none()
                && (!coop.running || coop.players.get(&0).is_none_or(|p| !p.paused));
            standing.then_some(local_hero_e)
        } else {
            coop.players
                .get(&bolt.peer)
                .filter(|p| p.loaded && !p.paused && p.hp > 0.0)
                .and_then(|_| allies.iter().find(|(_, ally)| ally.0 == bolt.peer))
                .map(|(entity, _)| entity)
        };
        let Some(target) = target else { continue };
        run.coop_target = bolt.peer;
        let taken = hero::hurt_hero(&mut run, bolt.damage);
        if taken > 0.0 {
            fx.number(target, taken, false, COLOR_HURT);
        }
    }
    run.coop_target = 0;

    // Separation over a coarse grid: only neighbours in the same or adjacent cells push.
    const CELL: f32 = 3.0;
    const COINCIDENT_SQ: f32 = 1e-4 * 1e-4;
    let Separation {
        bodies,
        grid,
        cells,
        push,
    } = &mut *separation;
    bodies.clear();
    bodies.extend(
        enemies
            .iter()
            .filter(|(_, _, e, _)| !e.dead)
            .map(|(ent, tf, e, _)| (ent, tf.translation, e.radius)),
    );
    let key = |p: Vec3| ((p.x / CELL).floor() as i32, (p.z / CELL).floor() as i32);
    grid.clear();
    // Reassign buckets as the horde moves while retaining each bucket's allocation.
    let mut used = 0;
    for (i, b) in bodies.iter().enumerate() {
        let cell = *grid.entry(key(b.1)).or_insert_with(|| {
            let slot = used;
            used += 1;
            if slot == cells.len() {
                cells.push(Vec::new());
            }
            cells[slot].clear();
            slot
        });
        cells[cell].push(i);
    }
    push.clear();
    for (i, &(ent, pos, r)) in bodies.iter().enumerate() {
        let (cx, cz) = key(pos);
        let mut acc = Vec3::ZERO;
        for dx in -1..=1 {
            for dz in -1..=1 {
                let Some(&cell) = grid.get(&(cx + dx, cz + dz)) else {
                    continue;
                };
                for &j in &cells[cell] {
                    if j == i {
                        continue;
                    }
                    let (_, other, or) = bodies[j];
                    let d = flat(pos - other);
                    let min = r + or;
                    let distance_sq = d.length_squared();
                    if distance_sq <= COINCIDENT_SQ {
                        let (sin, cos) = (i as f32).sin_cos();
                        acc += Vec3::new(sin, 0.0, cos) * 0.1;
                    } else if distance_sq < min * min {
                        let len = distance_sq.sqrt();
                        acc += d / len * (min - len);
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
        let (hero_e, hpos, peer, hero_down) =
            coop.target(pos, (local_hero_e, hpos), !hero_down, &allies);
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
                    let (hp, damage, _) = spawn_stats(e.def, Role::Lair, now, lairs.slain);
                    let hp = hp * run.party_factor(1.04);
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
        }

        // A charge: still while its lane fills, then a straight rush down it.
        if e.charge.is_some() && (homing.is_some() || e.stun_t > 0.0 || e.root_t > 0.0 || hero_down)
        {
            e.charge = None;
        }
        let enrage_speed = e.enrage_speed();
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
                speed = if e.home.is_some() || e.def.roaming_elite {
                    40.0
                } else {
                    CHARGE_SPEED
                };
                speed *= enrage_speed;
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
                let reach = if e.home.is_some() || e.def.roaming_elite {
                    let scale = if e.home.is_some() { 1.35 } else { 1.0 };
                    (e.def.scale * e.size * scale * 1.2).clamp(4.0, 7.0)
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
                if let Some(impact) = c.boar_impact {
                    super::princess::impact(&mut run, impact);
                }
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

        if let Some(cast) = e.cast {
            mv.speed = 0.0;
            mv.flags = 0;
            if homing.is_some() || e.stun_t > 0.0 || e.fear_t > 0.0 || hero_down {
                e.cast = None;
            } else {
                if now >= cast.until && cast.bolt {
                    // Native 0x5c0bcd0: the bolt carries the caster's damage at release.
                    e.cast = None;
                    fx.cast(ent, cast.spell, &[hero_e], None);
                    bolts.0.push(Bolt {
                        released: now,
                        at: now + dist / bolt_speed(cast.spell),
                        peer,
                        damage: e.damage * e.weaken,
                    });
                } else if now >= cast.until {
                    e.cast = None;
                    fx.cast(ent, cast.spell, &[], Some(cast.center));
                    let in_blast = |target: Vec3| {
                        flat(target - cast.center).length() <= cast.radius
                    };
                    let local_alive = run.dead_t.is_none()
                        && (!coop.running
                            || coop.players.get(&0).is_none_or(|p| !p.paused));
                    if local_alive && in_blast(hpos) {
                        run.coop_target = 0;
                        let taken = hero::hurt_hero(&mut run, cast.damage);
                        if taken > 0.0 {
                            fx.number(local_hero_e, taken, false, COLOR_HURT);
                        }
                    }
                    if coop.running {
                        for player in coop
                            .players
                            .values()
                            .filter(|p| p.peer != 0 && p.loaded && !p.paused && p.hp > 0.0)
                        {
                            if !in_blast(Vec3::from_array(player.pos)) {
                                continue;
                            }
                            run.coop_target = player.peer;
                            let taken = hero::hurt_hero(&mut run, cast.damage);
                            if taken > 0.0 {
                                if let Some((target, _)) =
                                    allies.iter().find(|(_, ally)| ally.0 == player.peer)
                                {
                                    fx.number(target, taken, false, COLOR_HURT);
                                }
                            }
                        }
                    }
                    run.coop_target = 0;
                }
                continue;
            }
        }

        let held = e.stun_t > 0.0 || e.root_t > 0.0 || now < e.script_hold_until;
        let contact_reach = if e.home.is_some() {
            2.2 + 1.2 * e.def.scale * e.size * 1.35
        } else {
            (HERO_RADIUS + e.radius + 1.0).max(3.0)
        };
        // WoW's melee hold: a mob stops a little inside its reach and gives chase again only
        // once the hero steps clear of it plus a leeway, so it never stutters between run and
        // stand at the edge.
        if dist <= contact_reach {
            e.melee = true;
        } else if dist > contact_reach + MELEE_LEEWAY {
            e.melee = false;
        }
        let mut contact = e.melee && dist <= contact_reach + MELEE_LEEWAY;
        // A caster stops once the hero is in its reach and throws from there.
        let in_reach = e
            .def
            .ranged
            .is_some_and(|r| dist <= r.range && homing.is_none());
        let mut step = Vec3::ZERO;
        let mut speed = 0.0;
        if let Some(dir) = homing {
            speed = e.speed * e.enrage_speed();
            step = dir * speed;
            tf.rotation = Quat::from_rotation_y(yaw_toward(dir));
        } else if !held && !hero_down {
            let fleeing = e.fear_t > 0.0;
            let dir = if fleeing {
                -to_hero.normalize_or_zero()
            } else {
                to_hero.normalize_or_zero()
            };
            if fleeing || (!contact && !in_reach) {
                // Straight at the hero at the full gait. The last stride is clipped to stop at
                // the hold distance, but the animation keeps the run's speed: a creeping speed
                // would freeze the run cycle mid-stride while the body glides.
                speed = e.speed * e.slow * CHASE_PACE * e.enrage_speed();
                let stride = if fleeing {
                    speed * dt
                } else {
                    (speed * dt).min((dist - contact_reach * 0.9).max(0.0))
                };
                step = dir * (stride / dt);
            }
            if dir != Vec3::ZERO {
                let target = Quat::from_rotation_y(yaw_toward(dir));
                tf.rotation = tf.rotation.slerp(target, 1.0 - (-12.0 * dt).exp());
            }
        }
        // A planted attacker or a committed cast owns its spot. Arriving neighbours must move
        // around it, rather than shoving it out of melee and repeatedly restarting its chase.
        let shove = if contact || held || in_reach { Vec3::ZERO }
            else { push.get(&ent).copied().unwrap_or_default() };
        let mut next = pos + step * dt + shove * (dt * 8.0).min(0.5);
        next = ground.at(next, pos.y);
        tf.translation = next;
        // Enter the hold and swing on arrival, rather than keeping the run animation and
        // waiting another frame to notice the player is already in reach.
        let arrived = flat(hpos - next).length();
        if homing.is_none() && !hero_down && e.fear_t <= 0.0 && arrived <= contact_reach {
            e.melee = true;
            contact = true;
            speed = 0.0;
        } else if arrived > contact_reach + MELEE_LEEWAY {
            e.melee = false;
            contact = false;
        }
        if speed > 0.0 {
            mv.speed = speed;
            mv.flags = move_flags::FORWARD;
        } else if mv.speed != 0.0 || mv.flags != 0 {
            mv.speed = 0.0;
            mv.flags = 0;
        }
        // In melee it stands in its combat Ready stance between swings, as in WoW.
        let engaged = contact && !hero_down && homing.is_none() && e.fear_t <= 0.0;
        if engaged != e.engaged {
            e.engaged = engaged;
            if engaged {
                commands.entity(ent).insert(Engaged(0));
            } else {
                commands.entity(ent).remove::<Engaged>();
            }
        }

        let free = homing.is_none()
            && e.stun_t <= 0.0
            && e.fear_t <= 0.0
            && now >= e.script_hold_until
            && e.attack_cd <= 0.0
            && !hero_down;
        let native_caster = e.def.cost > 0.0;
        let cast_reach = e.def.ranged.is_some_and(|r| {
            if native_caster {
                dist <= r.range + 4.0
            } else {
                in_reach && !contact
            }
        });
        if let Some(r) = e.def.ranged.filter(|_| free && cast_reach) {
            let cast_time = if native_caster {
                r.cooldown
            } else {
                1.0
            };
            if native_caster {
                // RVA 0x5c0bf27 uses the table swing interval (3.2 seconds), while
                // +0x0c in the ranged record is the cast duration, not a cooldown.
                e.attack_cd = e.def.attack;
            } else {
                e.attack_cd = r.cooldown;
            }
            if !native_caster {
                tele.circle(hpos, RANGED_BLAST_RADIUS, now, cast_time);
            }
            fx.cast(ent, r.spell, &[], None);
            e.cast = Some(RangedCast {
                until: now + cast_time,
                center: hpos,
                damage: e.damage * e.weaken,
                spell: r.spell,
                radius: RANGED_BLAST_RADIUS,
                bolt: native_caster,
            });
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
            if taken > 0.0 && e.affixes & super::mechanics::affix::VAMPIRIC != 0 {
                e.hp = (e.hp + raw * 3.0).min(e.max_hp);
            }
            fx.swing(ent, hero_e, taken);
            if taken > 0.0 && std::ptr::eq(e.def, &super::mother_fang::BROODLING) {
                let poison = e.damage * 0.2;
                if peer == 0 { super::mother_fang::poison(&mut run, poison); }
                else { run.coop_poison.push((peer, poison)); }
            }
            if taken > 0.0 {
                fx.number(hero_e, taken, false, COLOR_HURT);
            }
            if peer == 0 && run.thorns_t > 0.0 {
                let mut hit = Hit::direct(ent, run.thorns * run.stats().might, false);
                hit.source = Some(run.thorns_source.unwrap_or(super::damage_meter::Source::Spell(run.thorns_spell)));
                hits.write(hit);
            }
            if taken > 0.0 && peer == 0 {
                run.struck = true;
                let sources = run.effect_sources(22, 20);
                let reflect: f32 = sources.iter().map(|(_, value)| value).sum();
                if reflect > 0.0 {
                    let mut hit = Hit::direct(ent, raw * reflect, false);
                    hit.contributors = sources;
                    hits.write(hit);
                }
            }
        }
    }
    run.coop_target = 0;
}

/// Hogger standing (GnollMelee's Stand box, 1.757 yd, × his 3.5): the tallest a boss stands.
/// Bigger rigs stretch their animations badly and swallow the screen.
const BOSS_HEIGHT: f32 = 6.15;
/// Elites stand a head shorter than the bosses.
const ELITE_HEIGHT: f32 = 5.0;

/// Shrink an elite or boss whose model, at its row's scale, stands taller than its cap, as soon
/// as the model loads and its Stand-box height is known; smaller ones keep their size. Reach and
/// body follow the new size.
pub(super) fn cap_heights(
    mut fresh: Query<
        (
            &mut Enemy,
            &mut crate::net::NetEntity,
            &mut ObjectStore,
            &mut Transform,
            &crate::entities::StandBoxHeight,
        ),
        Added<crate::entities::StandBoxHeight>,
    >,
) {
    for (mut e, mut net, mut store, mut tf, stand) in &mut fresh {
        let limit = if e.boss {
            BOSS_HEIGHT
        } else if e.home.is_some() || e.def.roaming_elite {
            ELITE_HEIGHT
        } else {
            continue;
        };
        if stand.0 <= 0.0 || stand.0 * net.scale <= limit * 1.01 {
            continue;
        }
        let cap = limit / stand.0;
        let shrink = cap / net.scale;
        e.size *= shrink;
        e.radius *= shrink;
        net.scale = cap;
        tf.scale = Vec3::splat(cap);
        store.0.merge(benilla_protocol::ObjectFields::from_pairs(&[(4, cap.to_bits())]));
    }
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
    mut spawner: ResMut<Spawner>,
    mut next: ResMut<NextState<Phase>>,
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
        let kind = if d.boss { 2 } else if d.elite { 1 } else { 0 };
        run.equipment_kills[kind] = run.equipment_kills[kind].saturating_add(1);
        if kind < 2 && (run.tally.len() < super::quests::MAX_TALLY || run.tally.contains_key(d.name)) {
            let count = &mut run.tally.entry(d.name.to_owned()).or_default()[kind];
            *count = count.saturating_add(1);
        }
        if let Some(name) = &d.boss_name {
            run.boss_loot.push(name.clone());
        }
        let low_memory_mob = !d.boss && !d.elite && !d.silver && d.spawn_cost > 0.0
            && super::low_memory::enabled(&run);
        // Each reduced-density mob represents more spawn credits as well as more rewards.
        spawner.source.killed(if low_memory_mob {
            d.spawn_cost / super::low_memory::DENSITY
        } else { d.spawn_cost });
        // Regular mobs pay 1.65x, stochastically rounded; bosses pay their table XP
        // unchanged (RVA 0x5c03cd9..0x5c03dc3). No global 2x multiplier.
        let xp = if d.boss || d.roaming_elite {
            d.xp
        } else {
            let reward = if low_memory_mob {
                super::low_memory::REWARD
            } else { 1.0 };
            let scaled = d.xp as f32 * 1.65 * reward;
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
            let amount = if low_memory_mob {
                super::low_memory::reward(amount, &mut rng)
            } else { amount };
            spawn_gem(
                &mut commands,
                run.guid(),
                d.pos + Vec3::X * 0.8,
                GemKind::Gold(amount),
            );
            info!("survivors gold: dropped base={amount} balance={}", run.gold);
        }
        if d.boss {
            // A boss leaves a physical free cache; only picking it opens its reward.
            coop.cache(d.pos);
            super::caches::spawn(&mut commands, run.guid(), d.pos, false);
        } else if d.elite || d.silver {
            // An elite's or a silver's chest opens on the kill. Guests still get a free cache at
            // the spot.
            coop.cache(d.pos);
            if run.dead_t.is_none() {
                run.pending_chests += 1;
                next.set(Phase::LevelUp);
            }
        }
        if d.elite && rng.f32() < super::magnets::DROP_CHANCE {
            spawn_gem(
                &mut commands,
                run.guid(),
                d.pos + Vec3::Z * 0.8,
                GemKind::Magnet,
            );
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
    mut scripted: Query<(Entity, &Guid, &mut Transform, &mut Enemy, &mut Script), Without<Hero>>,
    unscripted: Query<(Entity, &Enemy), Without<Script>>,
    mut fx: Fx,
    mut tele: ResMut<super::telegraphs::Telegraphs>,
    mut windups: ResMut<super::boss_pacing::Windups>,
    mut eggs: ResMut<super::mother_fang::Eggs>,
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
        mut damage,
        owner,
        damage_factor,
        visual,
        ..
    } in due
    {
        let factor = scripted.get(owner).map(|(_, _, _, en, _)| en.enrage_damage()).unwrap_or(damage_factor);
        damage *= factor / damage_factor;
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

    let pool = unlocked(roster, now);
    let players = super::mechanics::living(&run, &coop, (hero_e, hpos), &allies);
    let living_adds: Vec<Entity> = unscripted.iter().filter(|(_, en)| !en.dead && en.hp > 0.0)
        .map(|(e, _)| e).collect();
    for (e, guid, mut tf, mut en, mut script) in &mut scripted {
        let (_, hpos, peer, _) = coop.target(
            tf.translation,
            (hero_e, hpos),
            run.dead_t.is_none(),
            &allies,
        );
        run.coop_target = peer;
        if en.boss && en.def.name == "Morgan the Collector" {
            script.morgan.tick(&mut commands, e, &mut en, &mut tf, &players, &living_adds,
                now, &mut run, &ground, &mut fx, &mut tele, &mut windups, &mut rng);
            script.cooldowns = if en.dead { Vec::new() }
                else { script.morgan.timers(now, en.hp / en.max_hp) };
            continue;
        }
        if en.boss && en.def.name == "Hogger" {
            let scale = data::damage_scale(now) * super::difficulty::factors(en.def.tier).boss_damage * en.enrage_damage();
            script.hogger.tick(e, &mut en, &mut tf, hpos, &players, now, scale,
                &mut run, &ground, &mut fx, &mut tele, &mut windups);
            script.cooldowns = if en.dead { Vec::new() } else { script.hogger.timers(now) };
            continue;
        }
        if (en.boss && en.def.name == "Princess") || std::ptr::eq(en.def, &super::princess::STAMPEDE_BOAR) {
            let scale = data::damage_scale(now) * super::difficulty::factors(en.def.tier).boss_damage * en.enrage_damage();
            script.princess.tick(&mut commands, e, &mut en, &mut tf, hpos, &players, now, scale,
                &mut run, &ground, &mut fx, &mut tele, &mut windups);
            script.cooldowns = if en.dead { Vec::new() } else { script.princess.timers(now) };
            continue;
        }
        if en.home.is_some() || en.def.roaming_elite {
            super::guardian_behavior::tick(
                &mut script.guardian,
                &mut en,
                e,
                tf.translation,
                hpos,
                &players,
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
            script.cooldowns.push(super::boss_cooldowns::Timer::native(
                phase.action,
                now,
                first,
            ));
        }
        let pos = tf.translation;
        let damage_scale =
            data::damage_scale(now) * super::difficulty::factors(def.tier).boss_damage * en.enrage_damage();
        let mut calls = Vec::new();
        let script = &mut *script;
        let mut busy = en.charge.is_some() || en.stun_t > 0.0 || en.fear_t > 0.0
            || script.cooldowns.iter().any(|t| t.cast_until > now);
        for ((action, wait), timer) in script.running.iter_mut().zip(&mut script.cooldowns) {
            *wait -= dt;
            timer.due = now + wait.max(0.0);
            if *wait > 0.0 {
                continue;
            }
            if busy || (en.boss && !windups.available(now)) {
                timer.ready = true;
                continue;
            }
            match *action {
                data::BossAction::Eggs { count, every } => {
                    *wait = every;
                    timer.started(now, every, 0.8);
                    en.root_t = en.root_t.max(0.8);
                    eggs.lay(guid.0, pos, count + u32::from(share <= 0.5),
                        en.max_hp, en.damage, &mut run, &ground, &mut tele);
                }
                data::BossAction::Stomp {
                    spell,
                    radius,
                    damage,
                    every,
                } => {
                    // Only with the hero close; it plants its feet while the circle fills.
                    if !players.iter().any(|p| flat(p.pos - pos).length() <= radius) || en.charge.is_some() {
                        *wait = 1.0;
                        timer.ready = true;
                        continue;
                    }
                    *wait = every;
                    timer.started(now, every, STOMP_WINDUP);
                    en.root_t = en.root_t.max(STOMP_WINDUP);
                    tele.circle(pos, radius, now, STOMP_WINDUP);
                    blasts.push(Blast {
                        owner: e,
                        damage_factor: en.enrage_damage(),
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
                    timer.started(now, every, 1.4);
                    for _ in 0..count {
                        let a = rng.range(0.0, std::f32::consts::TAU);
                        let r = rng.range(0.0, 6.0);
                        let at = ground.at(hpos + Vec3::new(a.cos(), 0.0, a.sin()) * r, hpos.y);
                        tele.circle(at, radius, now, 1.4);
                        blasts.push(Blast {
                            owner: e,
                            damage_factor: en.enrage_damage(),
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
                        timer.ready = true;
                        continue;
                    }
                    *wait = every;
                    timer.started(now, every, CHARGE_WINDUP);
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
                        boar_impact: None,
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
                        timer.ready = true;
                        continue;
                    }
                    *wait = every;
                    let at = ground.at(hpos, hpos.y);
                    fx.cast(e, spell, &[], Some(at));
                    // In the air long enough to see it coming and step out.
                    let flight = (d / 14.0).clamp(1.2, 1.8);
                    timer.started(now, every, flight);
                    tele.circle(at, radius, now, flight);
                    blasts.push(Blast {
                        owner: e,
                        damage_factor: en.enrage_damage(),
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
                    timer.started(now, every, 0.0);
                    if every <= 0.0 {
                        timer.due = f32::INFINITY;
                    }
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
            if timer.cast_until > now {
                busy = true;
                en.attack_cd = en.attack_cd.max(timer.cast_until - now);
                if en.boss { windups.take(now); }
            }
        }
        for (def, count) in calls {
            for i in 0..count {
                let a = i as f32 / count as f32 * std::f32::consts::TAU;
                let at = ground.spawn_spot(pos + Vec3::new(a.cos(), 0.0, a.sin()) * 3.0, pos.y);
                let summoned = spawn_enemy(&mut commands, &mut run, def, at, hpos, false);
                if en.boss && summoned != Entity::PLACEHOLDER {
                    commands.entity(summoned).entry::<Enemy>().and_modify(|mut add| add.boss_minion = true);
                }
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
        GemKind::Magnet => super::magnets::spawn(commands, serial, pos),
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
    // The host collects for either living player, including XP outside streamed terrain.
    let magnet_collected = gems.iter().any(|(_, tf, gem, visible)| {
        matches!(gem.kind, GemKind::Magnet)
            && *visible != Visibility::Hidden
            && ((run.dead_t.is_none()
                && !run.coop_unavailable.contains(&0)
                && flat(target - tf.translation).length() <= stats.magnet)
                || coop.players.values().any(|p| {
                    p.peer != 0
                        && p.loaded
                        && !p.paused
                        && p.hp > 0.
                        && flat(Vec3::from_array(p.pos) - tf.translation).length() <= stats.magnet
                }))
    });
    let mut collected_xp = 0u32;
    for (e, mut tf, mut g, visible) in &mut gems {
        let vacuum = magnet_collected && matches!(g.kind, GemKind::Xp(_) | GemKind::Gold(_));
        if *visible == Visibility::Hidden && !vacuum {
            continue;
        }
        if vacuum {
            g.pulled = true;
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
        if matches!(g.kind, GemKind::Magnet) {
            if flat(d).length() <= stats.magnet {
                commands.entity(e).despawn();
            }
            continue;
        }
        if let GemKind::Gold(base) = g.kind {
            // As in the original, gold is collected within pickup range. Distant piles
            // stay put unless a magnet sweeps them; only collection credits the balance and
            // applies gold-find.
            if vacuum || flat(d).length() <= stats.magnet {
                let earned =
                    super::gold::collected(base, run.gold_find(), run.party_factor(0.48));
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
        if vacuum || flat(d).length() < 0.9 || dist <= g.speed * dt || g.flight >= 0.75 {
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
                GemKind::Magnet => unreachable!("magnet handled before pickup attraction"),
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
            run.xp += (collected_xp as f32 * stats.growth / run.party_factor(0.48))
                .round()
                .max(1.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bolts_fly_at_the_native_spell_speeds() {
        assert_eq!(bolt_speed(133), 24.0);
        assert_eq!(bolt_speed(116), 28.0);
        // A Frostbolt released from the 18-yard stop lands about 0.64 s later.
        assert!((18.0 / bolt_speed(116) - 0.643).abs() < 0.001);
    }
}
