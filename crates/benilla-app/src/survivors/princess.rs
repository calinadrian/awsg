//! Princess owns a moving boar encounter: fixed charge lanes, a frontal tusk sweep,
//! front/flank ambushes and a staggered two-boar stampede in the second phase.
use super::{
    boss_cooldowns::Timer,
    boss_pacing::Windups,
    data,
    dungeon::Ground,
    enemies::{self, Charge, Enemy},
    mechanics::{self, Target},
    telegraphs::Telegraphs,
    units::{flat, yaw_toward, Fx},
    Run,
};
use bevy::prelude::*;

pub(super) static STAMPEDE_BOAR: data::EnemyDef = data::EnemyDef {
    scale: 1.15,
    attack: 1.2,
    ..data::en("Stampede Boar", 703, 500.0, 10.0, 25.0, 0, 11)
};
#[derive(Clone, Copy)]
pub(super) enum Impact {
    Rush,
    Ambush,
}

/// Apply only when the charge connects; guests receive absolute deadlines.
pub(super) fn impact(run: &mut Run, impact: Impact) {
    let (stun_seconds, healing_seconds) = match impact {
        Impact::Rush => (3.0, 0.0),
        Impact::Ambush => (1.0, 30.0),
    };
    let stun = run.elapsed + stun_seconds;
    let healing = if healing_seconds > 0.0 { run.elapsed + healing_seconds } else { 0.0 };
    if run.coop_unavailable.contains(&run.coop_target) {
        return;
    }
    if run.coop_target != 0 {
        run.coop_boar_impacts.push((run.coop_target, stun, healing));
    } else {
        receive_impact(run, stun, healing);
    }
}

pub(super) fn receive_impact(run: &mut Run, stun: f32, healing: f32) {
    if run.dead_t.is_some() || run.hp <= 0.0 || run.immune_t > 0.0 || run.test_god {
        return;
    }
    run.stun_until = run.stun_until.max(stun);
    run.healing_blocked_until = run.healing_blocked_until.max(healing);
}

const RUSH: usize = 0;
const SWEEP: usize = 1;
const STAMPEDE: usize = 2;
const AMBUSH: usize = 3;
const SPELLS: [u32; 4] = [6268, 15496, 8599, 6268];
const NAMES: [&str; 4] = ["Boar Rush", "Tusk Sweep", "Stampede", "Boar Ambush"];
const PERIODS: [f32; 4] = [9.0, 5.5, 22.0, 10.0];

struct Sweep {
    at: f32,
    from: f32,
    center: Vec3,
    dir: Vec3,
    damage: f32,
}
#[derive(Default)]
pub(super) struct Script {
    born: Option<f32>,
    due: [f32; 4],
    phase_two: bool,
    busy: f32,
    charging: bool,
    charge_from: f32,
    charge_until: f32,
    second: Option<f32>,
    follow_sweep: bool,
    sweep: Option<Sweep>,
    ambush_until: f32,
    ambush_side: u8,
    stampede_until: f32,
    boar_expires: Option<f32>,
    boar_rushed: bool,
    ambush_boar: bool,
}

impl Script {
    pub(super) fn enrage_pending(&mut self) {
        if let Some(sweep) = self.sweep.as_mut() { sweep.damage *= super::boss_enrage::DAMAGE; }
    }
    pub(super) fn boar(now: f32, delay: f32) -> Self {
        Self {
            born: Some(now),
            due: [now + delay, 0.0, 0.0, 0.0],
            boar_expires: Some(now + 18.0),
            ..default()
        }
    }
    fn period(&self, slot: usize) -> f32 {
        PERIODS[slot]
            * if self.phase_two && slot == RUSH {
                0.8
            } else {
                1.0
            }
    }
    pub(super) fn timers(&self, now: f32) -> Vec<Timer> {
        if self.born.is_none() || self.boar_expires.is_some() {
            return Vec::new();
        }
        (0..4)
            .filter(|&slot| slot != STAMPEDE || self.phase_two)
            .map(|slot| {
                let (until, duration) = match slot {
                    RUSH if self.charging => {
                        (self.charge_until, self.charge_until - self.charge_from)
                    }
                    SWEEP => self
                        .sweep
                        .as_ref()
                        .map_or((0.0, 0.0), |s| (s.at, s.at - s.from)),
                    STAMPEDE => (self.stampede_until, 1.0),
                    AMBUSH => (self.ambush_until, 1.5),
                    _ => (0.0, 0.0),
                };
                let due = if slot == RUSH {
                    self.second.unwrap_or(self.due[slot])
                } else {
                    self.due[slot]
                };
                Timer {
                    spell: SPELLS[slot],
                    name: NAMES[slot].into(),
                    due,
                    period: self.period(slot),
                    cast_until: until.max(0.0),
                    cast_duration: duration.max(0.1),
                    ready: due <= now,
                }
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn tick(
        &mut self,
        commands: &mut Commands,
        entity: Entity,
        enemy: &mut Enemy,
        tf: &mut Transform,
        target: Vec3,
        players: &[Target],
        now: f32,
        scale: f32,
        run: &mut Run,
        ground: &Ground,
        fx: &mut Fx,
        tele: &mut Telegraphs,
        windups: &mut Windups,
    ) {
        if enemy.dead {
            self.sweep = None;
            return;
        }
        let boar = self.boar_expires.is_some();
        if self.boar_expires.is_some_and(|until| now >= until) {
            commands.entity(entity).despawn();
            return;
        }
        if self.born.is_none() {
            self.born = Some(now);
            self.due = [now + 3.0, now + 1.5, f32::MAX, now + 6.0];
        }
        if !boar && !self.phase_two && enemy.hp <= enemy.max_hp * 0.5 {
            self.phase_two = true;
            self.due[STAMPEDE] = now;
            run.yell = Some(("Princess squeals! A stampede is coming!".into(), 3.0));
            fx.cast(entity, 8599, &[], None);
        }
        // Pursuit is near player pace; phase two adds 15%, without multiplicative stacking.
        if !boar {
            enemy.speed = 9.0 * if self.phase_two { 1.15 } else { 1.0 };
        }
        let pos = tf.translation;
        let dist = flat(target - pos).length();
        let previous = run.coop_target;
        if self.sweep.as_ref().is_some_and(|s| now >= s.at) {
            let sweep = self.sweep.take().unwrap();
            fx.cast(entity, SPELLS[SWEEP], &[], None);
            for player in players {
                let delta = flat(player.pos - sweep.center);
                if delta.length() <= 9.0
                    && (delta.length_squared() < 0.001
                        || delta.normalize().dot(sweep.dir) >= std::f32::consts::FRAC_1_SQRT_2)
                {
                    mechanics::strike(run, fx, player, sweep.damage);
                }
            }
        }
        run.coop_target = previous;
        if enemy.stun_t > 0.0 || enemy.fear_t > 0.0 {
            self.sweep = None;
            self.second = None;
            return;
        }
        if self.charging && enemy.charge.is_none() {
            self.charging = false;
            self.charge_until = 0.0;
            if self.second.is_some() {
                self.second = Some(now + 0.45);
                self.busy = now + 0.45;
            } else {
                self.busy = now + if dist <= 9.0 { 0.2 } else { 1.0 };
                self.follow_sweep = !boar && dist <= 9.0;
            }
            enemy.attack_cd = enemy.attack_cd.max(self.busy - now);
        }
        if enemy.charge.is_some()
            || self.sweep.is_some()
            || now < self.busy
            || players.is_empty()
            || enemy.root_t > 0.0
            || !windups.available(now)
        {
            return;
        }
        if !boar && self.phase_two && now >= self.due[STAMPEDE] {
            windups.take(now);
            self.due[STAMPEDE] = now + PERIODS[STAMPEDE];
            self.stampede_until = now + 1.0;
            let dir = flat(target - pos).normalize_or(Vec3::Z);
            let side = Vec3::new(-dir.z, 0.0, dir.x);
            for i in 0..2 {
                let spawn = ground.spawn_spot(pos + side * if i == 0 { -7.0 } else { 7.0 }, pos.y);
                let add = enemies::spawn_enemy(commands, run, &STAMPEDE_BOAR, spawn, target, false);
                let health = enemy.max_hp * 0.025;
                let damage = enemy.damage * 0.6;
                commands.queue(move |world: &mut World| {
                    if let Some(mut enemy) = world.get_mut::<Enemy>(add) {
                        enemy.hp = health;
                        enemy.max_hp = health;
                        enemy.damage = damage;
                        enemy.speed = 10.0;
                        enemy.boss_minion = true;
                    }
                    if let Some(mut script) = world.get_mut::<enemies::Script>(add) {
                        script.princess = Script::boar(now, 1.0 + i as f32 * 1.4);
                    }
                });
            }
            self.busy = now + 0.7;
            return;
        }
        if !boar && now >= self.due[AMBUSH] && self.second.is_none() {
            windups.take(now);
            self.due[AMBUSH] = now + PERIODS[AMBUSH];
            self.ambush_until = now + 1.5;
            // Forward is away from the chasing boss; rotate through forward and both flanks.
            let forward = flat(target - pos).normalize_or(Vec3::Z);
            let side = Vec3::new(-forward.z, 0.0, forward.x);
            let offset = match self.ambush_side % 3 {
                0 => forward * 14.0,
                1 => side * 12.0 + forward * 3.0,
                _ => -side * 12.0 + forward * 3.0,
            };
            self.ambush_side = self.ambush_side.wrapping_add(1);
            let spawn = ground.spawn_spot(target + offset, target.y);
            let add = enemies::spawn_enemy(commands, run, &STAMPEDE_BOAR, spawn, target, false);
            let health = enemy.max_hp * 0.025;
            let damage = enemy.damage * 0.6;
            commands.queue(move |world: &mut World| {
                if let Some(mut enemy) = world.get_mut::<Enemy>(add) {
                    enemy.hp = health;
                    enemy.max_hp = health;
                    enemy.damage = damage;
                    enemy.speed = 10.0;
                    enemy.boss_minion = true;
                    enemy.root_t = 0.25;
                    enemy.attack_cd = 1.5;
                }
                if let Some(mut script) = world.get_mut::<enemies::Script>(add) {
                    script.princess = Script {
                        ambush_boar: true,
                        ..Script::boar(now, 0.25)
                    };
                }
            });
            self.busy = now + 0.5;
            return;
        }
        let second = self.second.is_some_and(|at| now >= at);
        let rush_ready = if boar {
            !self.boar_rushed
        } else {
            now >= self.due[RUSH] || second
        };
        if rush_ready
            && now >= self.due[RUSH].min(self.second.unwrap_or(f32::MAX))
            && (3.0..=34.0).contains(&dist)
        {
            windups.take(now);
            let dir = flat(target - pos).normalize_or(Vec3::Z);
            let length = (dist + 4.0).max(14.0);
            let to = ground.at(pos + dir * length, target.y);
            let windup = if second { 1.1 } else { 1.25 };
            tele.lane(
                pos,
                to,
                enemy.radius.max(1.2) + 0.4,
                now,
                windup,
                windup + length / 20.0 + 0.3,
            );
            enemy.charge = Some(Charge {
                boar_impact: if !boar {
                    Some(Impact::Rush)
                } else if self.ambush_boar {
                    Some(Impact::Ambush)
                } else {
                    None
                },
                to,
                spell: SPELLS[RUSH],
                damage: if boar { enemy.damage } else { 30.0 * scale },
                windup,
                left: length / 20.0 + 0.3,
                hit: false,
            });
            self.charging = true;
            self.boar_rushed = true;
            self.charge_from = now;
            self.charge_until = now + windup;
            self.second = if !boar && self.phase_two && !second {
                Some(f32::MAX)
            } else {
                None
            };
            self.due[RUSH] = now + self.period(RUSH);
            self.follow_sweep = false;
            return;
        }
        if !boar && dist <= 9.0 && (self.follow_sweep || now >= self.due[SWEEP]) {
            windups.take(now);
            let dir = flat(target - pos).normalize_or(Vec3::Z);
            tf.rotation = Quat::from_rotation_y(yaw_toward(dir));
            tele.cone(pos, dir, 9.0, std::f32::consts::FRAC_PI_4, now, 1.15);
            self.sweep = Some(Sweep {
                at: now + 1.15,
                from: now,
                center: pos,
                dir,
                damage: 32.0 * scale,
            });
            self.busy = now + 1.65;
            enemy.root_t = enemy.root_t.max(1.15);
            enemy.attack_cd = enemy.attack_cd.max(1.65);
            self.due[SWEEP] = now + PERIODS[SWEEP];
            self.follow_sweep = false;
        } else if dist > 9.0 {
            self.follow_sweep = false;
        }
    }
}
