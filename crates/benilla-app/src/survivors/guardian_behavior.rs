//! Installed ASR's guardian records (lookup RVA 0x5ab4a20), action interpreter
//! (0x5c26e80) and lair state machine (0x5b17bb0). Fedfennel keeps his boss kit.

use bevy::prelude::*;

use super::enemies::{self, Charge, Enemy};
use super::units::{flat, Fx, COLOR_HURT};
use super::{data, dungeon::Ground, hero, telegraphs::Telegraphs, Rng, Run};

#[derive(Clone, Copy)]
enum Action {
    Charge,
    DoubleCharge,
    Summon(u32),
    Cone {
        spell: u32,
        radius: f32,
        degrees: f32,
        delay: f32,
        power: f32,
    },
    Bombs,
    Stomp,
    Pool,
}

#[derive(Clone, Copy)]
struct Move {
    action: Action,
    every: f32,
}

// The three native elite records are 56-byte repeating abilities, not boss phases.
const TASKMASTER: &[Move] = &[
    Move {
        action: Action::Charge,
        every: 9.0,
    },
    Move {
        action: Action::Bombs,
        every: 10.0,
    },
    Move {
        action: Action::Summon(4),
        every: 15.0,
    },
];
const GRUFF: &[Move] = &[
    Move {
        action: Action::DoubleCharge,
        every: 11.0,
    },
    Move {
        action: Action::Cone {
            spell: 15284,
            radius: 7.0,
            degrees: 90.0,
            delay: 1.0,
            power: 1.4,
        },
        every: 7.0,
    },
    Move {
        action: Action::Summon(3),
        every: 16.0,
    },
];
const BELLYGRUB: &[Move] = &[
    Move {
        action: Action::Stomp,
        every: 9.0,
    },
    Move {
        action: Action::Pool,
        every: 12.0,
    },
    Move {
        action: Action::Charge,
        every: 9.0,
    },
];
const FEDFENNEL: &[Move] = &[
    Move {
        action: Action::Cone {
            spell: 6304,
            radius: 11.0,
            degrees: 80.0,
            delay: 1.1,
            power: 1.8,
        },
        every: 7.0,
    },
    Move {
        action: Action::Stomp,
        every: 10.0,
    },
    Move {
        action: Action::Charge,
        every: 11.0,
    },
    Move {
        action: Action::Summon(8),
        every: 14.0,
    },
];

fn kit(name: &str) -> &'static [Move] {
    match name {
        "Kobold Taskmaster" => TASKMASTER,
        "Gruff Swiftbite" => GRUFF,
        "Bellygrub" => BELLYGRUB,
        "Fedfennel" => FEDFENNEL,
        _ => &[],
    }
}

/// The normal-mode reference guardian curves; the boss-health floor uses all
/// non-final source bosses, including Fedfennel's original 9:00 record.
pub(super) fn stats(def: &data::EnemyDef, elapsed: f32, slain: u32) -> (f32, f32) {
    // Fedfennel is the agreed roster exception: retain his boss stats, rather than
    // applying a guardian curve to an already boss-sized health pool.
    if def.name == "Fedfennel" {
        return (def.hp, def.damage);
    }
    const BOSSES: [(f32, f32); 4] = [
        (180.0, 3600.0),
        (360.0, 9000.0),
        (540.0, 17000.0),
        (720.0, 28000.0),
    ];
    // Before the opening boss, native RVA 0x5c67ad7 ramps its health floor
    // from 60% to 100%; it does not use the full first-boss health immediately.
    let mut floor = BOSSES[0].1 * (0.6 + 0.4 * (elapsed / BOSSES[0].0).clamp(0.0, 1.0));
    for pair in BOSSES.windows(2) {
        if elapsed <= BOSSES[0].0 {
            break;
        }
        let [(start, hp), (end, next)] = [pair[0], pair[1]];
        floor = hp + (next - hp) * ((elapsed - start) / (end - start)).clamp(0.0, 1.0);
        if elapsed <= end {
            break;
        }
    }
    let minute = elapsed / 60.0;
    let hp = (def.hp * 2.5 * (1.0 + 0.12 * minute + 0.009 * minute * minute)).max(0.6 * floor)
        * 0.82 // Elwynn's source zone multiplier (+0xe0).
        * 3.0
        * (1.0 + 0.25 * slain as f32);
    let damage = def.damage
        * (1.0 + 0.08 * minute + 0.004 * minute * minute)
        * 0.82_f32.sqrt()
        * 1.4
        * (1.0 + 0.05 * slain as f32);
    (hp, damage)
}

#[derive(Clone, Copy)]
enum Shape {
    Circle(f32),
    Cone { dir: Vec3, radius: f32, cosine: f32 },
}

struct Impact {
    at: f32,
    pos: Vec3,
    shape: Shape,
    damage: f32,
    spell: u32,
    // Bellygrub's six-second ground hazard deals 0.3x damage per second.
    ticks: u32,
    dot: f32,
}

#[derive(Default)]
pub(super) struct Script {
    waits: Vec<f32>,
    impacts: Vec<Impact>,
    second_charge: Option<f32>,
    busy: f32,
}

impl Script {
    pub(super) fn reset(&mut self, name: &str) {
        // Native spawn and evade both initialize cooldowns to half their period.
        self.waits = kit(name).iter().map(|m| m.every * 0.5).collect();
        self.second_charge = None;
        self.busy = 0.0;
        self.impacts.clear();
    }
}

#[allow(clippy::too_many_arguments)]
fn charge(
    en: &mut Enemy,
    pos: Vec3,
    target: Vec3,
    entity: Entity,
    fx: &mut Fx,
    ground: &Ground,
    now: f32,
    tele: &mut Telegraphs,
) -> f32 {
    let way = flat(target - pos);
    let distance = (way.length() + 6.0).max(4.0);
    let to = ground.at(pos + way.normalize_or(Vec3::X) * distance, pos.y);
    let scale = if en.home.is_some() { 1.35 } else { 1.0 };
    let half = (en.def.scale * scale * 1.2).clamp(4.0, 7.0);
    let rush = distance / 40.0;
    tele.lane(pos, to, half, now, 1.0, 1.0 + rush);
    fx.cast(entity, 100, &[], None);
    en.charge = Some(Charge {
        to,
        spell: 100,
        damage: 2.0 * en.damage * en.weaken,
        windup: 1.0,
        left: rush,
        hit: false,
    });
    1.0 + rush
}

/// Execute the native repeating kit without the port's extra shared minion timer.
#[allow(clippy::too_many_arguments)]
pub(super) fn tick(
    script: &mut Script,
    en: &mut Enemy,
    entity: Entity,
    pos: Vec3,
    hero_entity: Entity,
    target: Vec3,
    dt: f32,
    commands: &mut Commands,
    run: &mut Run,
    rng: &mut Rng,
    roster: &'static data::Roster,
    ground: &Ground,
    fx: &mut Fx,
    tele: &mut Telegraphs,
) {
    let now = run.elapsed;
    if script.waits.len() != kit(en.def.name).len() {
        script.reset(en.def.name);
    }
    if en.dead {
        script.impacts.clear();
        return;
    }
    if en.home.is_some_and(|h| !h.awake) {
        script.reset(en.def.name);
        return;
    }
    for impact in &mut script.impacts {
        if now < impact.at {
            continue;
        }
        let delta = flat(target - impact.pos);
        let hit = match impact.shape {
            Shape::Circle(radius) => delta.length() <= radius,
            Shape::Cone {
                dir,
                radius,
                cosine,
            } => {
                delta.length() <= radius
                    && (delta == Vec3::ZERO || delta.normalize().dot(dir) >= cosine)
            }
        };
        if impact.spell != 0 {
            fx.cast(entity, impact.spell, &[], Some(impact.pos));
            if impact.ticks > 0 {
                if let Shape::Circle(radius) = impact.shape {
                    tele.hazard(impact.pos, radius, now, impact.ticks as f32);
                }
            }
        }
        if hit && (run.dead_t.is_none() || run.coop_target != 0) {
            let taken = hero::hurt_hero(run, impact.damage);
            if taken > 0.0 {
                fx.number(hero_entity, taken, false, COLOR_HURT);
            }
        }
        if impact.ticks > 0 {
            impact.ticks -= 1;
            impact.at += 1.0;
            impact.damage = impact.dot;
            impact.spell = 0;
        } else {
            impact.at = f32::INFINITY;
        }
    }
    script.impacts.retain(|impact| impact.at.is_finite());
    for wait in &mut script.waits {
        *wait -= dt;
    }
    if (run.dead_t.is_some() && run.coop_target == 0) || en.stun_t > 0.0 || en.fear_t > 0.0 {
        return;
    }
    if script.second_charge.is_some_and(|at| now >= at) {
        if en.charge.is_none() {
            script.second_charge = None;
            script.busy = now + charge(en, pos, target, entity, fx, ground, now, tele);
        }
    }
    for (i, movement) in kit(en.def.name).iter().enumerate() {
        if script.waits[i] > 0.0 {
            continue;
        }
        // The native support-action mask permits summons during a movement/cast lock.
        if (en.charge.is_some() || now < script.busy || en.root_t > 0.0)
            && !matches!(movement.action, Action::Summon(_))
        {
            continue;
        }
        let power = en.damage * en.weaken;
        // Native low-health guardian cooldowns run at 70% of their normal period.
        script.waits[i] = movement.every * if en.hp < en.max_hp * 0.3 { 0.7 } else { 1.0 };
        match movement.action {
            Action::Charge | Action::DoubleCharge => {
                let duration = charge(en, pos, target, entity, fx, ground, now, tele);
                script.busy = now + duration;
                if matches!(movement.action, Action::DoubleCharge) {
                    script.second_charge = Some(now + 1.6);
                }
            }
            Action::Summon(count) => {
                run.yell = Some((format!("{} yells: Minions, to me!", en.def.name), 4.0));
                let pool: Vec<_> = roster
                    .enemies
                    .iter()
                    .enumerate()
                    .filter(|(_, def)| def.from <= now)
                    .collect();
                let weights: Vec<_> = pool
                    .iter()
                    .map(|&(i, def)| {
                        super::elwynn_behavior::weight(
                            now,
                            def.from,
                            super::elwynn_behavior::COSTS[i],
                        )
                    })
                    .collect();
                for i in 0..count {
                    if pool.is_empty() {
                        break;
                    }
                    let def =
                        pool[super::elwynn_behavior::choose(&weights, rng.f32()).unwrap_or(0)].1;
                    let angle = i as f32 / count as f32 * std::f32::consts::TAU;
                    let at = ground
                        .spawn_spot(pos + Vec3::new(angle.cos(), 0.0, angle.sin()) * 5.0, pos.y);
                    enemies::spawn_enemy(commands, run, def, at, target, false);
                }
            }
            Action::Cone {
                spell,
                radius,
                degrees,
                delay,
                power: multiplier,
            } => {
                let dir = flat(target - pos).normalize_or(Vec3::X);
                tele.cone(pos, dir, radius, degrees.to_radians() * 0.5, now, delay);
                fx.cast(entity, spell, &[], None);
                en.root_t = en.root_t.max(delay + 0.3);
                script.busy = now + delay + 0.3;
                script.impacts.push(Impact {
                    at: now + delay,
                    pos,
                    shape: Shape::Cone {
                        dir,
                        radius,
                        cosine: (degrees.to_radians() * 0.5).cos(),
                    },
                    damage: power * multiplier,
                    spell: 0,
                    ticks: 0,
                    dot: 0.0,
                });
            }
            Action::Bombs => {
                fx.cast(entity, 133, &[], None);
                script.busy = now + 1.6;
                en.root_t = en.root_t.max(1.6);
                for i in 0..3 {
                    let at = if i == 0 {
                        target
                    } else {
                        let a = rng.range(0.0, std::f32::consts::TAU);
                        ground.at(
                            target + Vec3::new(a.cos(), 0.0, a.sin()) * rng.range(4.5, 7.0),
                            target.y,
                        )
                    };
                    tele.circle(at, 3.5, now, 1.6);
                    script.impacts.push(Impact {
                        at: now + 1.6,
                        pos: at,
                        shape: Shape::Circle(3.5),
                        damage: power * 1.2,
                        spell: 133,
                        ticks: 0,
                        dot: 0.0,
                    });
                }
            }
            Action::Stomp => {
                let (radius, multiplier) = if en.def.name == "Fedfennel" {
                    (8.0, 1.4)
                } else {
                    (7.0, 1.3)
                };
                tele.circle(pos, radius, now, 1.3);
                fx.cast(entity, 20549, &[], None);
                en.root_t = en.root_t.max(1.3);
                script.busy = now + 1.3;
                script.impacts.push(Impact {
                    at: now + 1.3,
                    pos,
                    shape: Shape::Circle(radius),
                    damage: power * multiplier,
                    spell: 0,
                    ticks: 0,
                    dot: 0.0,
                });
            }
            Action::Pool => {
                tele.circle(pos, 5.0, now, 1.6);
                fx.cast(entity, 6524, &[], None);
                script.impacts.push(Impact {
                    at: now + 1.6,
                    pos,
                    shape: Shape::Circle(5.0),
                    damage: power * 1.2,
                    spell: 6524,
                    ticks: 6,
                    dot: power * 0.3,
                });
            }
        }
    }
}
