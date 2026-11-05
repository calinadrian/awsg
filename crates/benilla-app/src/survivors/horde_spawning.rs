//! Normal-mode Elwynn chasers, derived from ASR.exe 0.124 (SHA-256 08563b3a…).
//! Budget/floor: 0x5c15374–0x5c159cf; cap/annulus: 0x5c00487–0x5c005ef.
use std::collections::HashMap;

use bevy::prelude::*;

use super::{
    Rng, Run,
    coop::Coop,
    data,
    dungeon::Ground,
    enemies::{self, Enemy},
    units::flat,
};

#[derive(Default)]
pub(super) struct Spawner {
    started: bool,
    budget: f32,
    kill_heat: f32,
    /// The run second the next silver comes.
    silver_at: f32,
    turn: usize,
    motion: HashMap<u32, (Vec3, Vec3)>,
}

impl Spawner {
    /// Only ordinary enemy-table kills replenish pressure (0x5c03cb5).
    pub(super) fn killed(&mut self, cost: f32) {
        self.kill_heat += cost;
    }
}

pub(super) fn cost(def: &data::EnemyDef) -> f32 {
    def.cost
}

/// Base credits/second; recent killed budget decays with a twelve-second time constant.
pub(super) fn rate(elapsed: f32, heat: f32, pressure: f32) -> f32 {
    let minute = elapsed.max(0.0) / 60.0;
    let base = 0.6 * (0.9 + 0.32 * minute + 0.025 * minute * minute) * pressure;
    base * (heat / 12.0 * 1.25 / base.max(0.001)).clamp(1.0, 3.0)
}

/// This floor counts ordinary chasers, not bosses, lair guardians or roaming elites.
pub(super) fn population_floor(elapsed: f32, pressure: f32, party: f32) -> usize {
    (((4.0 + 1.6 * elapsed.max(0.0) / 60.0) * pressure).min(36.0) * party) as usize
}

/// Seconds until the next roaming elite: 55 at the start, 25 from the twelfth minute. Elites here
/// wait in their lairs; this native cadence now sends the silvers.
pub(super) fn elite_interval(elapsed: f32) -> f32 {
    (55.0 - 2.5 * elapsed / 60.0).max(25.0)
}

fn boss_pressure(last: bool) -> f32 {
    if last { 0.3 } else { 0.45 }
}

#[derive(Clone, Copy)]
struct Center {
    peer: u32,
    pos: Vec3,
    velocity: Vec3,
}

/// Native online spawn scaling (0x5c15588/0x5c0048c), not the legacy Python bridge.
/// The patched chapter lookup at 0x9dc2c70 selects 0.7 for chapters below three;
/// Elwynn is chapter zero. Game+0xc88 counts living players (0x5bd91a0).
pub(super) fn party_multiplier(living: usize) -> f32 {
    1.0 + living.saturating_sub(1) as f32 * 0.7 * 0.48
}

fn point(center: Center, rng: &mut Rng, ground: &Ground, forward_chance: f32) -> Vec3 {
    // 0x5bffb35–0x5bffd3c: half the draws favor movement within ±1.2 radians,
    // once the smoothed horizontal velocity exceeds two yards/second.
    let angle = if center.velocity.length_squared() > 4.0 && rng.f32() < forward_chance {
        center.velocity.z.atan2(center.velocity.x) + rng.range(-1.2, 1.2)
    } else {
        rng.range(0.0, std::f32::consts::TAU)
    };
    let radius = rng.range(26.0, 36.0);
    ground.spawn_spot(
        center.pos + Vec3::new(angle.cos(), 0.0, angle.sin()) * radius,
        center.pos.y,
    )
}

fn choose(
    roster: &'static data::Roster,
    pool: &[usize],
    elapsed: f32,
    rng: &mut Rng,
) -> &'static data::EnemyDef {
    let weights: Vec<_> = pool
        .iter()
        .map(|&i| {
            super::elwynn_behavior::weight(
                elapsed,
                roster.enemies[i].from,
                cost(&roster.enemies[i]),
            )
        })
        .collect();
    let index = super::elwynn_behavior::choose(&weights, rng.f32()).unwrap_or(0);
    &roster.enemies[pool[index]]
}

/// Count every queued spawn immediately: Bevy inserts the entities after this system returns.
#[allow(clippy::too_many_arguments)]
fn emit(
    commands: &mut Commands,
    run: &mut Run,
    rng: &mut Rng,
    state: &mut Spawner,
    centers: &[Center],
    ground: &Ground,
    alive: &mut usize,
    cap: usize,
    def: &'static data::EnemyDef,
) -> bool {
    if *alive >= cap {
        return false;
    }
    let center = centers[state.turn % centers.len()];
    state.turn = state.turn.wrapping_add(1);
    if enemies::spawn_enemy(
        commands,
        run,
        def,
        point(center, rng, ground, 0.5),
        center.pos,
        false,
    ) == Entity::PLACEHOLDER
    {
        return false;
    }
    *alive += 1;
    true
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    commands: &mut Commands,
    dt: f32,
    run: &mut Run,
    rng: &mut Rng,
    state: &mut Spawner,
    roster: &'static data::Roster,
    new_bosses: &[data::EnemyDef],
    local: Vec3,
    enemies: &mut Query<(&mut Enemy, &mut Transform), Without<super::hero::Hero>>,
    ground: &Ground,
    coop: &Coop,
) {
    let mut centers = Vec::new();
    if run.dead_t.is_none() && run.hp > 0.0 {
        centers.push(Center {
            peer: 0,
            pos: local,
            velocity: Vec3::ZERO,
        });
    }
    if coop.active() {
        centers.extend(
            coop.players
                .values()
                .filter(|p| p.peer != 0 && p.loaded && p.hp > 0.0)
                .map(|p| Center {
                    peer: p.peer,
                    pos: Vec3::from_array(p.pos),
                    velocity: Vec3::ZERO,
                }),
        );
    }
    if centers.is_empty() {
        return;
    }
    centers.sort_by_key(|p| p.peer);
    state
        .motion
        .retain(|peer, _| centers.iter().any(|p| p.peer == *peer));
    for center in &mut centers {
        let motion = state
            .motion
            .entry(center.peer)
            .or_insert((center.pos, Vec3::ZERO));
        let velocity = flat(center.pos - motion.0) / dt;
        // Native 0x5c1510b rejects teleports at 40 yd/s and filters with 1-exp(-dt).
        if velocity.length() < 40.0 {
            motion.1 += (velocity - motion.1) * (1.0 - (-dt).exp());
        }
        motion.0 = center.pos;
        center.velocity = motion.1;
    }
    let party = party_multiplier(centers.len());
    // 0x5c82930: recover at most four ordinary/roaming mobs beyond 55 yd from
    // every living player. Bosses, lairs and active charges keep their positions.
    let mut relocated = 0;
    for (mut enemy, mut tf) in enemies.iter_mut() {
        if enemy.dead || enemy.boss || enemy.home.is_some() || enemy.charge.is_some() {
            continue;
        }
        if centers
            .iter()
            .any(|p| flat(tf.translation - p.pos).length_squared() <= 3025.0)
        {
            continue;
        }
        let center = centers[state.turn % centers.len()];
        state.turn = state.turn.wrapping_add(1);
        tf.translation = point(center, rng, ground, 1.0);
        enemy.relocated(tf.translation, center.pos);
        relocated += 1;
        if relocated == 4 {
            break;
        }
    }
    let tier = super::difficulty::factors(roster.enemies.first().map_or(1, |e| e.tier));
    // The constructor calls roundf before converting its scaled hard limit.
    let cap = (50.0 * party * roster.crowd).round() as usize;
    let mut alive = enemies.iter().filter(|(e, _)| !e.dead).count() + new_bosses.len();
    run.horde_spawn_slots = Some(cap.saturating_sub(alive));
    let mut ordinary = enemies
        .iter()
        .filter(|(e, _)| !e.dead && !e.boss && e.home.is_none() && cost(e.def) > 0.0)
        .count();
    let pressure = enemies
        .iter()
        .filter(|(e, _)| !e.dead && e.boss)
        .map(|(e, _)| boss_pressure(e.def.last))
        .chain(new_bosses.iter().map(|e| boss_pressure(e.last)))
        .fold(1.0, f32::min);
    let elapsed = run.elapsed;
    let pool: Vec<_> = roster
        .enemies
        .iter()
        .enumerate()
        .filter(|(_, e)| e.from <= elapsed)
        .map(|(i, _)| i)
        .collect();
    if pool.is_empty() {
        return;
    }
    // 0x5bf8293–0x5bf82e7 starts with four copies of regular definition zero.
    if !state.started {
        state.started = true;
        state.silver_at = elapsed + elite_interval(elapsed) * tier.elite_every;
        for _ in 0..4 {
            if emit(
                commands,
                run,
                rng,
                state,
                &centers,
                ground,
                &mut alive,
                cap,
                &roster.enemies[0],
            ) {
                ordinary += 1;
            }
        }
    }
    state.kill_heat *= (-dt / 12.0).exp();
    state.budget += dt * rate(elapsed, state.kill_heat, pressure) * party * tier.density;
    // Native work limit: at most eight successful paid spawns per update.
    for _ in 0..8 {
        if state.budget < 1.0 {
            break;
        }
        let def = choose(roster, &pool, elapsed, rng);
        let price = cost(def);
        if price > state.budget {
            break;
        }
        if !emit(
            commands, run, rng, state, &centers, ground, &mut alive, cap, def,
        ) {
            state.budget = state.budget.min(price);
            break;
        }
        state.budget -= price;
        ordinary += 1;
    }
    // A separate unpriced floor tops up at most three ordinary mobs per update.
    let fill = ((population_floor(elapsed, pressure, party) as f32 * tier.density * roster.crowd) as usize)
        .saturating_sub(ordinary)
        .min(3);
    for _ in 0..fill {
        let def = choose(roster, &pool, elapsed, rng);
        if !emit(
            commands, run, rng, state, &centers, ground, &mut alive, cap, def,
        ) {
            break;
        }
    }
    // No elite joins the horde: every elite keeps to its lair at the edge
    // ([`super::guardians`]) until a player pulls it. A silver comes instead: a melee horde row,
    // bigger and tougher.
    if elapsed >= state.silver_at {
        let def = (0..6)
            .map(|_| choose(roster, &pool, elapsed, rng))
            .find(|d| d.ranged.is_none());
        // One wave elite for the whole co-op party; wait a full interval after a blocked roll.
        let wave_elite_alive = coop.active()
            && enemies.iter().any(|(e, _)| {
                !e.dead && !e.boss && e.home.is_none() && (e.silver || e.def.roaming_elite)
            });
        if let Some(def) = def.filter(|_| alive < cap && !wave_elite_alive) {
            let center = centers[state.turn % centers.len()];
            state.turn = state.turn.wrapping_add(1);
            let at = point(center, rng, ground, 0.5);
            if enemies::spawn_silver(commands, run, def, at, center.pos) != Entity::PLACEHOLDER {
                run.banner = Some((format!("A silver {} approaches!", def.name), 3.0));
            }
        }
        state.silver_at = elapsed + elite_interval(elapsed) * tier.elite_every;
    }
}
