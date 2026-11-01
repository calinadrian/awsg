//! Auto-cast: every owned ability fires on its own cooldown at the targets its kind picks, with
//! the spell's own visuals; damage lands when its missile would.

use bevy::prelude::*;

use super::data::{AbilityDef, Effect, Kind};
use super::enemies::{self, Enemy, Loot};
use super::hero::{self, Hero};
use super::units::{self, flat, Fx, Marks, UnitSpec, COLOR_HEAL, COLOR_MELEE, COLOR_SPELL};
use super::{Rng, Run, RunEntity};
use crate::net::{FieldChanged, Guid, ObjectStore};

/// One blow landing on one enemy.
#[derive(Message, Clone)]
pub(super) struct Hit {
    pub proc_depth: u8,
    pub spell: u32,
    pub dot_every: f32,
    pub target: Entity,
    pub damage: f32,
    pub dot: f32,
    pub dot_dur: f32,
    pub effect: Effect,
    pub mark: u32,
    pub mark_dur: f32,
    /// Also hits everything this close to the target, for less.
    pub splash: f32,
    pub crit: bool,
    pub color: u32,
}

impl Hit {
    pub fn direct(target: Entity, damage: f32, crit: bool) -> Self {
        Self {
            proc_depth: 0,
            spell: 0,
            dot_every: 1.0,
            target,
            damage,
            dot: 0.0,
            dot_dur: 0.0,
            effect: Effect::None,
            mark: 0,
            mark_dur: 0.0,
            splash: 0.0,
            crit,
            color: COLOR_MELEE,
        }
    }
}

/// Hits in flight, by the run time they land.
#[derive(Resource, Default)]
pub(super) struct PendingHits(pub Vec<(f32, Hit)>);

/// A ground spell's area, ticking on whatever stands in it.
#[derive(Component)]
pub(super) struct GroundArea {
    spell: u32,
    damage: f32,
    dot: f32,
    dot_dur: f32,
    pub radius: f32,
    left: f32,
    every: f32,
    next_tick: f32,
    effect: Effect,
    mark: u32,
    /// The spell whose impact plays on each enemy a tick hits, 0 for none.
    impact: u32,
}

#[derive(Component)]
pub(super) struct Totem {
    spell: u32,
    cast: u32,
    damage: f32,
    range: f32,
    radius: f32,
    left: f32,
    tick: f32,
    pulse: bool,
    count: usize,
    every: f32,
}

#[derive(Clone, Copy)]
struct Foe {
    e: Entity,
    pos: Vec3,
    dist: f32,
    radius: f32,
    priority: bool,
    marked: bool,
    health: f32,
}

/// The numbers one cast uses, from the ability's level and the hero's stats.
pub(super) struct Params {
    pub dmg: f32,
    pub dot: f32,
    pub count: usize,
    pub radius: f32,
    pub duration: f32,
    pub cooldown: f32,
}

pub(super) fn parameters(o: &super::Owned, run: &Run) -> Params {
    let def = o.def;
    let stats = run.stats();
    let t = &run.talents;
    let mult = o.damage_mult()
        * if def.kind == Kind::Buff {
            1.0
        } else {
            stats.might
        }
        * (1. + t.modifier(def, 0))
        * if matches!(def.kind, Kind::Shield | Kind::Heal | Kind::Thorns) {
            1. + run.jewel_stat(17)
        } else {
            1.
        };
    // Native spell damage helper: the common 1.4 scale and the area-kind factors.
    let native_scale = if def.native_kind == 255 || def.native_kind == 9 {
        1.0
    } else if matches!(def.native_kind, 2 | 3 | 8) {
        0.7
    } else {
        1.4
    };
    Params {
        dmg: def.damage
            * mult
            * native_scale
            * if matches!(def.self_effect, 6 | 7) {
                stats.max_hp
            } else {
                1.0
            }
            + if def.damage > 0. {
                t.modifier(def, 5) * stats.max_hp
            } else {
                0.
            },
        dot: def.dot * mult * native_scale,
        count: ((o.count() + stats.extra_targets) as f32 + t.modifier(def, 2))
            .round()
            .max(1.) as usize,
        radius: o.radius(&stats) * (1. + t.modifier(def, 3)),
        duration: o.duration() * (1. + t.modifier(def, 4)),
        cooldown: (o.cooldown(&stats) * (1. - t.modifier(def, 1)).max(0.4)).max(0.2),
    }
}

fn mark_secs(def: &AbilityDef, duration: f32) -> f32 {
    match def.effect {
        Effect::Slow(_, t) | Effect::Weaken(_, t) => t,
        Effect::Root(t) | Effect::Stun(t) | Effect::Fear(t) => t,
        _ if duration > 0.0 => duration,
        _ => 3.0,
    }
}

fn hit_for(def: &AbilityDef, p: &Params, target: Entity, damage: f32, crit: bool) -> Hit {
    Hit {
        proc_depth: 0,
        spell: def.spell,
        dot_every: if def.ticks > 0 {
            p.duration / def.ticks as f32
        } else {
            1.0
        },
        target,
        damage,
        dot: p.dot,
        dot_dur: p.duration,
        effect: def.effect,
        mark: def.mark,
        mark_dur: mark_secs(def, p.duration),
        splash: if matches!(def.kind, Kind::Bolt | Kind::Strike) {
            p.radius
        } else {
            0.0
        },
        crit,
        color: if def.kind == Kind::Melee {
            COLOR_MELEE
        } else {
            COLOR_SPELL
        },
    }
}

fn schedule(
    pending: &mut PendingHits,
    hits: &mut MessageWriter<Hit>,
    now: f32,
    delay: f32,
    hit: Hit,
) {
    if delay <= 0.0 {
        hits.write(hit);
    } else {
        pending.0.push((now + delay, hit));
    }
}

fn roll(rng: &mut Rng, stats: &super::Stats, dmg: f32) -> (f32, bool) {
    if rng.f32() < stats.crit {
        (dmg * stats.crit_mult, true)
    } else {
        (dmg, false)
    }
}

fn nearest(foes: &[Foe], range: f32, n: usize) -> Vec<Foe> {
    foes.iter()
        .filter(|f| f.dist <= range + f.radius)
        .take(n)
        .copied()
        .collect()
}

/// Range is enforced before priority: an unreachable boss cannot suppress a nearby target.
fn focused(foes: &[Foe], range: f32, n: usize, finisher: bool) -> Vec<Foe> {
    let mut v: Vec<Foe> = foes
        .iter()
        .filter(|f| f.dist <= range + f.radius)
        .copied()
        .collect();
    v.sort_by(|a, b| {
        b.marked
            .cmp(&a.marked)
            .then_with(|| b.priority.cmp(&a.priority))
            .then_with(|| {
                if finisher {
                    (b.health < 0.3).cmp(&(a.health < 0.3)).then_with(|| {
                        if a.health < 0.3 && b.health < 0.3 {
                            a.health.total_cmp(&b.health)
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    })
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then_with(|| a.dist.total_cmp(&b.dist))
    });
    v.truncate(n);
    v
}

fn around(foes: &[Foe], center: Vec3, r: f32) -> Vec<Foe> {
    foes.iter()
        .filter(|f| flat(f.pos - center).length() <= r + f.radius * 0.5)
        .copied()
        .collect()
}

/// The point among the nearest enemies in range with the most others within `r` of it.
fn densest(foes: &[Foe], range: f32, r: f32) -> Option<Vec3> {
    let near: Vec<&Foe> = foes.iter().filter(|f| f.dist <= range).take(40).collect();
    near.iter()
        .map(|f| {
            (
                f.pos,
                near.iter()
                    .filter(|o| flat(o.pos - f.pos).length() <= r)
                    .count(),
            )
        })
        .max_by_key(|&(_, n)| n)
        .map(|(p, _)| p)
}

fn entities(foes: &[Foe]) -> Vec<Entity> {
    foes.iter().map(|f| f.e).collect()
}

pub(super) fn cast_abilities(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    spells: Option<Res<crate::ui_action::Spells>>,
    hero: Query<(Entity, &Transform), With<Hero>>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    mut fx: Fx,
    mut pending: ResMut<PendingHits>,
    mut hits: MessageWriter<Hit>,
) {
    let dt = time.delta_secs();
    if run.dead_t.is_some() || dt <= 0.0 {
        return;
    }
    let Ok((me, htf)) = hero.single() else { return };
    let hpos = htf.translation;
    let stats = run.stats();
    let now = run.elapsed;
    let mut foes: Vec<Foe> = enemies
        .iter()
        .filter(|(_, _, e)| !e.dead)
        .map(|(e, tf, en)| Foe {
            e,
            pos: tf.translation,
            dist: hero::reach(hpos, tf.translation),
            radius: en.radius,
            priority: en.boss || en.home.is_some(),
            marked: run.passives.marks.get(&e).is_some_and(|m| m.hunter),
            health: en.hp / en.max_hp,
        })
        .collect();
    foes.sort_by(|a, b| a.dist.total_cmp(&b.dist));
    let passive_targets: Vec<_> = enemies
        .iter()
        .filter(|(_, _, e)| !e.dead)
        .map(|(e, tf, en)| {
            (
                e,
                tf.translation,
                en.hp,
                en.max_hp,
                en.boss || en.home.is_some(),
            )
        })
        .collect();
    for hit in super::spell_passives::tick(&mut run, dt, hpos, &passive_targets) {
        fx.impact(hit.target, hit.spell);
        hits.write(hit);
    }
    let speed_of = |spell: u32| {
        spells
            .as_deref()
            .and_then(|s| s.catalog.get(spell))
            .map_or(0.0, |d| d.speed)
    };

    for i in 0..run.abilities.len() {
        run.abilities[i].cd -= dt;
        if run.abilities[i].cd > 0.0 {
            continue;
        }
        let o = &run.abilities[i];
        let def = o.def;
        if def.kind == Kind::Passive {
            continue;
        }
        let p = parameters(o, &run);
        let speed = if def.native_kind != 255 {
            def.speed
        } else {
            speed_of(def.spell)
        };
        let fired = match def.kind {
            Kind::Bolt | Kind::Strike | Kind::Melee | Kind::Execute | Kind::Missiles => {
                let reach = if def.kind == Kind::Melee {
                    p.radius + 0.5
                } else {
                    def.range
                };
                let targets = if def.kind == Kind::Missiles {
                    let mut pool = focused(&foes, reach, usize::MAX, false);
                    if pool.iter().any(|f| f.priority) {
                        pool.retain(|f| f.priority);
                    }
                    if pool.is_empty() {
                        Vec::new()
                    } else {
                        (0..p.count).map(|_| pool[rng.index(pool.len())]).collect()
                    }
                } else {
                    focused(&foes, reach, p.count, def.kind == Kind::Execute)
                };
                if targets.is_empty() {
                    false
                } else {
                    if def.kind == Kind::Melee {
                        fx.swing(me, targets[0].e, p.dmg);
                        for f in &targets {
                            fx.impact(f.e, def.spell);
                        }
                    } else {
                        fx.cast(me, def.spell, &entities(&targets), None);
                    }
                    for f in &targets {
                        let damage = if def.kind == Kind::Execute && f.health < 0.3 {
                            p.dmg * 2.0
                        } else {
                            p.dmg
                        };
                        let (dmg, crit) = roll(&mut rng, &stats, damage);
                        let delay = if speed > 0.0 { f.dist / speed } else { 0.0 };
                        schedule(
                            &mut pending,
                            &mut hits,
                            now,
                            delay,
                            hit_for(def, &p, f.e, dmg, crit),
                        );
                    }
                    true
                }
            }
            Kind::Nova | Kind::Cone => {
                let mut targets = around(&foes, hpos, p.radius);
                if def.kind == Kind::Cone {
                    if let Some(first) = targets.first().copied() {
                        let aim = flat(first.pos - hpos).normalize_or_zero();
                        targets.retain(|f| {
                            let d = flat(f.pos - hpos);
                            d.length() < 2.0 || d.normalize_or_zero().dot(aim) >= 0.8
                        });
                    }
                }
                if targets.is_empty() {
                    false
                } else {
                    fx.cast(me, def.spell, &entities(&targets), None);
                    for f in &targets {
                        let (dmg, crit) = roll(&mut rng, &stats, p.dmg);
                        let delay = if speed > 0.0 { f.dist / speed } else { 0.0 };
                        schedule(
                            &mut pending,
                            &mut hits,
                            now,
                            delay,
                            hit_for(def, &p, f.e, dmg, crit),
                        );
                    }
                    true
                }
            }
            Kind::Ground | Kind::GroundSelf => {
                let center = if def.kind == Kind::GroundSelf {
                    foes.first()
                        .filter(|f| f.dist <= p.radius + 4.0)
                        .map(|_| hpos)
                } else {
                    densest(&foes, def.range, p.radius)
                };
                match center {
                    None => false,
                    Some(center) => {
                        fx.cast(me, def.spell, &[], Some(center));
                        let area = if def.overhead {
                            commands
                                .spawn((Transform::from_translation(center), RunEntity))
                                .id()
                        } else {
                            let serial = run.guid();
                            units::spawn_area_object(
                                &mut commands,
                                serial,
                                def.spell,
                                p.radius,
                                center,
                            )
                        };
                        let burst = def.duration <= 0.0;
                        commands.entity(area).insert(GroundArea {
                            spell: def.spell,
                            damage: if def.ticks > 0 {
                                p.dmg / def.ticks as f32
                            } else {
                                p.dmg
                            },
                            dot: p.dot,
                            dot_dur: if burst { 5.0 } else { 0.0 },
                            radius: p.radius,
                            left: if burst { 1.5 } else { p.duration },
                            every: if burst {
                                f32::INFINITY
                            } else {
                                p.duration / def.ticks.max(1) as f32
                            },
                            next_tick: if burst {
                                0.0
                            } else {
                                p.duration / def.ticks.max(1) as f32
                            },
                            effect: def.effect,
                            mark: def.mark,
                            impact: if def.overhead { def.spell } else { 0 },
                        });
                        true
                    }
                }
            }
            Kind::Chain => match focused(&foes, def.range, 1, false).first().copied() {
                None => false,
                Some(first) => {
                    let mut chain = vec![first];
                    while chain.len() < p.count + 1 {
                        let last = chain[chain.len() - 1].pos;
                        let next = foes
                            .iter()
                            .filter(|f| !chain.iter().any(|c| c.e == f.e))
                            .filter(|f| flat(f.pos - last).length() <= p.radius)
                            .min_by(|a, b| {
                                flat(a.pos - last)
                                    .length()
                                    .total_cmp(&flat(b.pos - last).length())
                            });
                        match next {
                            Some(f) => chain.push(*f),
                            None => break,
                        }
                    }
                    fx.cast(me, def.spell, &entities(&chain), None);
                    for (k, f) in chain.iter().enumerate() {
                        let (dmg, crit) = roll(&mut rng, &stats, p.dmg * 0.92f32.powi(k as i32));
                        schedule(
                            &mut pending,
                            &mut hits,
                            now,
                            0.08 * k as f32,
                            hit_for(def, &p, f.e, dmg, crit),
                        );
                    }
                    true
                }
            },
            Kind::Dot => {
                let pool: Vec<Foe> = focused(&foes, def.range, usize::MAX, false)
                    .into_iter()
                    .filter(|f| {
                        enemies
                            .get(f.e)
                            .is_ok_and(|(_, _, en)| !en.dots.iter().any(|d| d.spell == def.spell))
                    })
                    .collect();
                if pool.is_empty() {
                    false
                } else {
                    let targets: Vec<Foe> = pool.into_iter().take(p.count).collect();
                    fx.cast(me, def.spell, &entities(&targets), None);
                    for f in &targets {
                        let delay = if speed > 0.0 { f.dist / speed } else { 0.0 };
                        schedule(
                            &mut pending,
                            &mut hits,
                            now,
                            delay,
                            hit_for(def, &p, f.e, 0.0, false),
                        );
                    }
                    true
                }
            }
            Kind::Aura => {
                let targets = around(&foes, hpos, p.radius);
                if targets.is_empty() {
                    false
                } else {
                    for f in targets.iter().take(6) {
                        fx.impact(f.e, def.spell);
                    }
                    for f in &targets {
                        let (dmg, crit) = roll(&mut rng, &stats, p.dmg);
                        hits.write(hit_for(def, &p, f.e, dmg, crit));
                    }
                    true
                }
            }
            Kind::Shield => {
                if run.shield > 0.0 && run.shield_t > 0.0 {
                    continue;
                }
                run.shield = p.dmg;
                run.shield_t = if def.native_kind == 9 {
                    f32::INFINITY
                } else {
                    p.duration
                };
                fx.cast(me, def.spell, &[me], None);
                true
            }
            Kind::Immune => {
                if run.hp < stats.max_hp * 0.35 {
                    run.immune_t = p.duration;
                    fx.cast(me, def.spell, &[me], None);
                    true
                } else {
                    false
                }
            }
            Kind::Thorns => {
                if foes.first().is_some_and(|f| f.dist <= 8.0) {
                    run.thorns = p.dmg;
                    run.thorns_t = p.duration;
                    run.thorns_spell = def.spell;
                    fx.cast(me, def.spell, &[me], None);
                    true
                } else {
                    false
                }
            }
            Kind::Heal => {
                if run.hp < stats.max_hp * 0.85 {
                    if p.duration > 0.0 {
                        run.heal_rate = p.dmg / p.duration;
                        run.heal_t = p.duration;
                    } else {
                        run.hp = (run.hp + p.dmg).min(stats.max_hp);
                    }
                    fx.cast(me, def.spell, &[me], None);
                    fx.number(me, p.dmg, false, COLOR_HEAL);
                    true
                } else {
                    false
                }
            }
            Kind::Totem => {
                if foes
                    .first()
                    .is_some_and(|f| f.dist <= def.range.max(p.radius) + 6.0)
                {
                    fx.cast(me, def.spell, &[], None);
                    let a = rng.range(0.0, std::f32::consts::TAU);
                    let pos = hpos + Vec3::new(a.cos(), 0.0, a.sin()) * 2.0;
                    let serial = run.guid();
                    let totem = units::spawn_unit(
                        &mut commands,
                        &UnitSpec {
                            serial,
                            display: def.totem.0,
                            scale: 1.0,
                            level: 60,
                            faction: units::FACTION_FRIENDLY,
                            bytes0: 0,
                            weapons: &[],
                            sheath: 0,
                        },
                        pos,
                        0.0,
                    );
                    commands.entity(totem).insert(Totem {
                        spell: def.spell,
                        cast: def.totem.1,
                        damage: p.dmg,
                        range: def.range,
                        radius: p.radius,
                        left: p.duration,
                        tick: if def.ticks > 0 {
                            p.duration / def.ticks as f32
                        } else {
                            1.0
                        },
                        pulse: def.key == "magma_totem",
                        count: p.count,
                        every: if def.ticks > 0 {
                            p.duration / def.ticks as f32
                        } else {
                            1.0
                        },
                    });
                    true
                } else {
                    false
                }
            }
            Kind::Buff => {
                if foes.first().is_some_and(|f| f.dist <= 30.0) {
                    run.buffs.retain(|b| b.0 != def.key);
                    run.buffs
                        .push((def.key, def.self_effect, p.dmg, p.duration));
                    fx.cast(me, def.spell, &[me], None);
                    true
                } else {
                    false
                }
            }
            Kind::Passive => false,
        };
        run.abilities[i].cd = if fired {
            p.cooldown
                + if def.kind == Kind::Immune {
                    p.duration
                } else {
                    0.0
                }
        } else {
            0.2
        };
        if fired {
            // A talent's echo: the spell goes off again at once.
            let echo = !run.abilities[i].echoing
                && rng.f32() < (run.talents.stat(13) + run.jewel_stat(9)).min(0.6);
            run.abilities[i].echoing = echo;
            if echo {
                run.abilities[i].cd = 0.15;
            }
            super::talents::trigger(&mut run, &mut rng, 7, None);
            super::talents::trigger(&mut run, &mut rng, 3, Some(def.key));
        }
    }
}

/// Ground areas tick, and totems fight, until they run out.
#[allow(clippy::type_complexity)]
pub(super) fn tick_areas(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut areas: Query<(Entity, &Transform, &mut GroundArea)>,
    mut totems: Query<(Entity, &Transform, &mut Totem), Without<GroundArea>>,
    enemies: Query<(Entity, &Transform, &Enemy), (Without<GroundArea>, Without<Totem>)>,
    mut fx: Fx,
    mut pending: ResMut<PendingHits>,
    mut hits: MessageWriter<Hit>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let stats = run.stats();
    let now = run.elapsed;
    let live: Vec<Foe> = enemies
        .iter()
        .filter(|(_, _, e)| !e.dead)
        .map(|(e, tf, en)| Foe {
            e,
            pos: tf.translation,
            dist: 0.0,
            radius: en.radius,
            priority: en.boss || en.home.is_some(),
            marked: run.passives.marks.get(&e).is_some_and(|m| m.hunter),
            health: en.hp / en.max_hp,
        })
        .collect();

    for (e, tf, mut area) in &mut areas {
        let active = dt.min(area.left.max(0.0));
        area.left -= dt;
        area.next_tick -= active;
        while area.next_tick <= 0.00001 {
            area.next_tick += area.every;
            for (i, f) in around(&live, tf.translation, area.radius)
                .into_iter()
                .enumerate()
            {
                if area.impact != 0 && i < 12 {
                    fx.impact(f.e, area.impact);
                }
                let (dmg, crit) = roll(&mut rng, &stats, area.damage);
                hits.write(Hit {
                    proc_depth: 0,
                    spell: area.spell,
                    dot_every: 1.0,
                    target: f.e,
                    damage: dmg,
                    dot: area.dot,
                    dot_dur: area.dot_dur,
                    effect: area.effect,
                    mark: area.mark,
                    mark_dur: 1.5,
                    splash: 0.0,
                    crit,
                    color: COLOR_SPELL,
                });
            }
        }
        if area.left <= 0.0 {
            commands.entity(e).despawn();
        }
    }

    for (e, tf, mut totem) in &mut totems {
        let active = dt.min(totem.left.max(0.0));
        totem.left -= dt;
        let expired = totem.left <= 0.0;
        if expired {
            if totem.cast != 724 {
                super::spell_passives::expired_totem(&mut run, tf.translation);
            }
            commands
                .entity(e)
                .remove::<Totem>()
                .insert(benilla_world::model_fade::DespawnFade::default());
        }
        totem.tick -= active;
        while totem.tick <= 0.00001 {
            totem.tick += totem.every.max(0.01);
            let pos = tf.translation;
            if totem.pulse {
                let targets = around(&live, pos, totem.radius);
                if targets.is_empty() {
                    continue;
                }
                fx.cast(e, totem.cast, &[], None);
                for f in &targets {
                    let (dmg, crit) = roll(&mut rng, &stats, totem.damage);
                    hits.write(Hit {
                        spell: totem.spell,
                        color: COLOR_SPELL,
                        ..Hit::direct(f.e, dmg, crit)
                    });
                }
            } else {
                let mut targets: Vec<_> = live
                    .iter()
                    .map(|f| (flat(f.pos - pos).length(), f))
                    .filter(|(d, _)| *d <= totem.range)
                    .collect();
                targets.sort_by(|a, b| {
                    b.1.marked
                        .cmp(&a.1.marked)
                        .then_with(|| b.1.priority.cmp(&a.1.priority))
                        .then_with(|| a.0.total_cmp(&b.0))
                });
                for (dist, f) in targets.into_iter().take(totem.count) {
                    fx.cast(e, totem.cast, &[f.e], None);
                    let speed = spells
                        .as_deref()
                        .and_then(|s| s.catalog.get(totem.cast))
                        .map_or(0.0, |d| d.speed);
                    let delay = if speed > 0.0 { dist / speed } else { 0.0 };
                    let (dmg, crit) = roll(&mut rng, &stats, totem.damage);
                    schedule(
                        &mut pending,
                        &mut hits,
                        now,
                        delay,
                        Hit {
                            spell: totem.spell,
                            color: COLOR_SPELL,
                            ..Hit::direct(f.e, dmg, crit)
                        },
                    );
                }
            }
        }
    }
}

/// Land every hit due: damage and its number, the over-time part, crowd control, the marks, and
/// death with its loot.
pub(super) fn apply_hits(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut pending: ResMut<PendingHits>,
    mut incoming: MessageReader<Hit>,
    mut enemies: Query<(
        Entity,
        &Guid,
        &Transform,
        &mut Enemy,
        &mut ObjectStore,
        &mut Marks,
    )>,
    mut fx: Fx,
    mut loot: MessageWriter<Loot>,
    mut fields: MessageWriter<FieldChanged>,
    mut rng: ResMut<Rng>,
) {
    let dt = time.delta_secs();
    let now = run.elapsed;
    let mut due: Vec<Hit> = incoming.read().cloned().collect();
    pending.0.retain(|(t, h)| {
        if *t <= now {
            due.push(h.clone());
            false
        } else {
            true
        }
    });

    // Splash reaches the target's neighbours at seventy percent.
    let mut all = Vec::with_capacity(due.len());
    for h in due {
        if h.splash > 0.0 {
            if let Ok((_, _, center, ..)) = enemies.get(h.target) {
                let c = center.translation;
                for (e, _, tf, en, ..) in enemies.iter() {
                    if e != h.target && !en.dead && flat(tf.translation - c).length() <= h.splash {
                        all.push(Hit {
                            target: e,
                            damage: h.damage * 0.7,
                            splash: 0.0,
                            mark: 0,
                            ..h.clone()
                        });
                    }
                }
            }
        }
        all.push(Hit { splash: 0.0, ..h });
    }

    let mut kill =
        |commands: &mut Commands, run: &mut Run, e: Entity, pos: Vec3, en: &mut Enemy| {
            en.dead = true;
            en.hp = 0.0;
            en.dots.clear();
            enemies::mark_dying(commands, e);
            loot.write(Loot {
                pos,
                xp: en.xp,
                boss: en.boss,
            });
            run.kills += 1;
            let max = run.stats().max_hp;
            run.hp = (run.hp + run.talents.stat(15) + run.jewel_stat(15)).min(max);
            if en.boss {
                run.banner = Some((format!("{} is slain!", en.def.name), 3.0));
            }
            if en.def.last {
                run.slain_last = true;
            }
        };

    for h in all {
        let Ok((e, guid, tf, mut en, mut store, mut marks)) = enemies.get_mut(h.target) else {
            continue;
        };
        if en.dead {
            continue;
        }
        let mut dmg = h.damage * super::spell_passives::marked_multiplier(&run, e);
        if dmg > 0.0 {
            // The talents that hit harder on the elite, the wounded, the afflicted and the held.
            let t = &run.talents;
            let cc = en.stun_t > 0.0 || en.root_t > 0.0 || en.fear_t > 0.0 || en.slow_t > 0.0;
            dmg *=
                1.0 + if en.boss || en.home.is_some() {
                    t.stat(19)
                } else {
                    0.0
                } + if en.hp / en.max_hp < 0.3 {
                    t.stat(20)
                } else {
                    0.0
                } + if en.dots.is_empty() { 0.0 } else { t.stat(21) }
                    + if cc { t.stat(18) } else { 0.0 };
            if h.crit {
                super::talents::trigger(&mut run, &mut rng, 2, None);
            }
            let leech = run.talents.stat(14) + run.jewel_stat(18);
            if leech > 0.0 {
                let max = run.stats().max_hp;
                run.hp = (run.hp + dmg * leech).min(max);
            }
            en.hp -= dmg;
            let was_sheep = run
                .passives
                .marks
                .get(&e)
                .is_some_and(|m| m.sheep_until > run.elapsed);
            super::spell_passives::on_hit(
                &mut run,
                &mut rng,
                e,
                tf.translation,
                en.max_hp,
                en.boss || en.home.is_some(),
                dmg,
                h.crit,
                h.spell,
            );
            if was_sheep {
                en.stun_t = 0.0;
            }
            if run
                .passives
                .marks
                .get(&e)
                .is_some_and(|m| m.sheep_until > run.elapsed)
            {
                en.stun_t = en.stun_t.max(10.0);
            }
            run.damage_done += f64::from(dmg);
            fx.number(e, dmg, h.crit, h.color);
            if let Effect::Drain(frac) = h.effect {
                let max = run.stats().max_hp;
                run.hp = (run.hp + dmg * frac).min(max);
            }
        }
        if h.dot > 0.0 && h.dot_dur > 0.0 {
            en.dots.retain(|d| d.spell != h.spell);
            en.dots.push(enemies::Dot {
                spell: h.spell,
                damage: h.dot * h.dot_every,
                every: h.dot_every,
                drain: if let Effect::Drain(f) = h.effect {
                    f
                } else {
                    0.0
                },
                left: h.dot_dur,
                tick: h.dot_every,
            });
        }
        let boss = en.boss || en.home.is_some();
        match h.effect {
            Effect::Slow(m, t) => {
                let m = if boss { m.max(0.75) } else { m };
                en.slow = if en.slow_t > 0.0 { en.slow.min(m) } else { m };
                en.slow_t = en.slow_t.max(t);
            }
            Effect::Root(t) => en.root_t = en.root_t.max(if boss { t * 0.25 } else { t }),
            Effect::Stun(t) => en.stun_t = en.stun_t.max(if boss { t * 0.25 } else { t }),
            Effect::Fear(t) if !boss => en.fear_t = en.fear_t.max(t),
            Effect::Weaken(m, t) => {
                en.weaken = m;
                en.weaken_t = t;
            }
            Effect::Heal(f) => {
                let max = run.stats().max_hp;
                run.hp = (run.hp + max * f).min(max);
            }
            Effect::Shield(f) => {
                let max = run.stats().max_hp;
                run.shield = (run.shield + max * f).min(max);
                run.shield_t = f32::INFINITY;
            }
            _ => {}
        }
        if h.mark != 0 {
            marks.add(h.mark, h.mark_dur);
        }
        if en.hp <= 0.0 {
            super::spell_passives::on_death(
                &mut run,
                &mut rng,
                e,
                tf.translation,
                h.spell,
                h.proc_depth,
            );
            kill(&mut commands, &mut run, e, tf.translation, &mut en);
            units::kill_unit(&mut store);
            marks.clear();
        }
        marks.flush(e, guid.0, &mut store, &mut fields);
    }

    // The over-time ticks, once a second each.
    for (e, guid, tf, mut en, mut store, mut marks) in &mut enemies {
        if en.dead || en.dots.is_empty() {
            continue;
        }
        let mut dealt = 0.0;
        let mut drained = 0.0;
        for d in &mut en.dots {
            d.left -= dt;
            d.tick -= dt.min((d.left + dt).max(0.0)) * (1.0 + run.jewel_stat(19));
            while d.tick <= 0.00001 {
                d.tick += d.every.max(0.01);
                dealt += d.damage;
                drained += d.damage * d.drain;
            }
        }
        en.dots.retain(|d| d.left > 0.0);
        if dealt > 0.0 {
            run.hp = (run.hp + drained).min(run.stats().max_hp);
            en.hp -= dealt;
            run.damage_done += f64::from(dealt);
            fx.number(e, dealt, false, COLOR_SPELL);
            if en.hp <= 0.0 {
                super::spell_passives::on_death(&mut run, &mut rng, e, tf.translation, 0, 0);
                kill(&mut commands, &mut run, e, tf.translation, &mut en);
                units::kill_unit(&mut store);
                marks.clear();
                marks.flush(e, guid.0, &mut store, &mut fields);
            }
        }
    }
}
