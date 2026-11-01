//! Event-driven class spells. Definition values come from spell_data.rs; trigger semantics
//! come from the original embedded English spell descriptions and native spell helpers.
//! These handlers do not establish full script parity: proc scaling, pet interactions and
//! sheep appearance still need their native implementations ported and compared.
use bevy::prelude::*;
use std::collections::HashMap;

use super::{
    abilities::{self, Hit},
    data, Rng, Run,
};

#[derive(Default)]
pub(super) struct State {
    pub rage: f32,
    pub combo: u32,
    pub shards: u32,
    pub rage_budget: f32,
    pub combo_budget: f32,
    pub last_hit: f32,
    pub marks: HashMap<Entity, Mark>,
    pub bursts: Vec<Burst>,
    pub next: HashMap<&'static str, f32>,
    pub transfers: Vec<Vec3>,
}

#[derive(Default)]
pub(super) struct Mark {
    pub hunter: bool,
    pub faerie: bool,
    pub sheep_until: f32,
    pub arc_at: f32,
}

pub(super) struct Burst {
    pub center: Vec3,
    pub radius: f32,
    pub damage: f32,
    pub spell: u32,
    pub depth: u8,
    pub exclude: Option<Entity>,
    pub targets: usize,
}

pub(super) fn description(key: &str) -> &'static str {
    match key {
        "bloodrage" => "Hits build rage. At 100 rage, erupts in Physical damage around you. Rage drains outside combat.",
        "redemption" => "Slain enemies can burst in Holy damage around their bodies. Burst kills can trigger one additional burst.",
        "hunters_mark" => "Marks the toughest enemy. It takes increased damage and explodes on death; then a new target is marked.",
        "seal_fate" => "Strikes build combo points, two on a critical. At five points, a free finisher prioritizes bosses and elites, then wounded enemies; damage doubles below 30% health.",
        "call_of_flame" => "Your totems explode in Fire damage when they expire.",
        "faerie_fire" => "Marks bosses and elites first. Hits on marked enemies arc Nature damage to nearby enemies. Marks transfer on death.",
        "polymorph" => "Damaging spells can sheep normal enemies. The next hit pops the sheep in an Arcane explosion. Bosses and guardians are immune.",
        "soul_fire" => "Kills gather soul shards, at most one every 1.2 sec. At five shards, a free Soulfire strikes an enemy in range.",
        _ => "A class-specific automatic spell.",
    }
}

fn ready(run: &Run, key: &'static str) -> bool {
    run.passives.next.get(key).is_none_or(|t| run.elapsed >= *t)
}

fn numbers(run: &Run, key: &str) -> Option<abilities::Params> {
    run.abilities
        .iter()
        .find(|o| o.def.key == key)
        .map(|o| abilities::parameters(o, run))
}

pub(super) fn marked_multiplier(run: &Run, target: Entity) -> f32 {
    if run.passives.marks.get(&target).is_some_and(|m| m.hunter) {
        // Original description helper RVA 0x5c50b17: 10% times spell power, capped at 20%.
        let power = run
            .abilities
            .iter()
            .find(|o| o.def.key == "hunters_mark")
            .map_or(1.0, |o| o.progress.power);
        1.0 + (0.1 * power).min(0.2)
    } else {
        1.0
    }
}

pub(super) fn on_hit(
    run: &mut Run,
    rng: &mut Rng,
    target: Entity,
    pos: Vec3,
    max_hp: f32,
    priority: bool,
    damage: f32,
    crit: bool,
    spell: u32,
) {
    if damage <= 0.0 {
        return;
    }
    let source = data::ABILITIES.iter().find(|a| a.spell == spell);
    // Proc damage cannot recursively refill the resource which produced it.
    let primary = source.is_some_and(|a| a.native_kind != 11);
    if primary {
        run.passives.last_hit = run.elapsed;
        if numbers(run, "bloodrage").is_some() {
            let gain = run.passives.rage_budget.min(2.0);
            run.passives.rage_budget -= gain;
            run.passives.rage = (run.passives.rage + gain).min(100.0);
        }
        if numbers(run, "seal_fate").is_some() {
            let gain = run
                .passives
                .combo_budget
                .min(if crit { 2.0 } else { 1.0 })
                .floor();
            run.passives.combo_budget -= gain;
            run.passives.combo = (run.passives.combo + gain as u32).min(5);
        }
    }
    let pop = run
        .passives
        .marks
        .get(&target)
        .is_some_and(|m| m.sheep_until > run.elapsed);
    if pop {
        if let Some(p) = numbers(run, "polymorph") {
            run.passives.bursts.push(Burst {
                center: pos,
                radius: p.radius,
                damage: (max_hp * 0.15).min(p.dmg),
                spell: 118,
                depth: 1,
                exclude: None,
                targets: usize::MAX,
            });
        }
        if let Some(m) = run.passives.marks.get_mut(&target) {
            m.sheep_until = 0.0;
        }
    } else if primary
        && !priority
        && ready(run, "polymorph")
        && run
            .passives
            .marks
            .values()
            .filter(|m| m.sheep_until > run.elapsed)
            .count()
            < 3
        && rng.f32() < 0.12
    {
        if let Some(p) = numbers(run, "polymorph") {
            run.passives.marks.entry(target).or_default().sheep_until = run.elapsed + 10.0;
            run.passives
                .next
                .insert("polymorph", run.elapsed + p.cooldown);
        }
    }
    if spell != 770
        && run
            .passives
            .marks
            .get(&target)
            .is_some_and(|m| m.faerie && run.elapsed >= m.arc_at)
    {
        if let Some(p) = numbers(run, "faerie_fire") {
            run.passives.bursts.push(Burst {
                center: pos,
                radius: p.radius,
                damage: (damage * 0.8).min(p.dmg),
                spell: 770,
                depth: 1,
                exclude: Some(target),
                targets: p.count,
            });
            if let Some(m) = run.passives.marks.get_mut(&target) {
                m.arc_at = run.elapsed + 0.25;
            }
        }
    }
}

pub(super) fn on_death(
    run: &mut Run,
    rng: &mut Rng,
    target: Entity,
    pos: Vec3,
    source: u32,
    depth: u8,
) {
    if ready(run, "soul_shard") && numbers(run, "soul_fire").is_some() {
        run.passives.shards = (run.passives.shards + 1).min(5);
        run.passives.next.insert("soul_shard", run.elapsed + 1.2);
    }
    if ready(run, "redemption") && !(source == 7328 && depth >= 2) {
        if let Some(p) = numbers(run, "redemption").filter(|p| {
            rng.f32()
                < (0.1 * data::ability("redemption").cooldown * 1.4 / p.cooldown.max(0.25)).min(0.2)
        }) {
            run.passives.bursts.push(Burst {
                center: pos,
                radius: p.radius,
                damage: p.dmg,
                spell: 7328,
                depth: if source == 7328 { depth + 1 } else { 1 },
                exclude: None,
                targets: usize::MAX,
            });
            run.passives.next.insert("redemption", run.elapsed + 0.5);
        }
    }
    if let Some(mark) = run.passives.marks.remove(&target) {
        if mark.faerie {
            run.passives.transfers.push(pos);
        }
        if mark.hunter {
            if let Some(p) = numbers(run, "hunters_mark") {
                if ready(run, "hunter_burst") {
                    run.passives.bursts.push(Burst {
                        center: pos,
                        radius: p.radius,
                        damage: p.dmg,
                        spell: 1130,
                        depth: 1,
                        exclude: None,
                        targets: usize::MAX,
                    });
                    run.passives
                        .next
                        .insert("hunter_burst", run.elapsed + p.cooldown);
                }
                run.passives.next.insert("hunters_mark", run.elapsed + 1.0);
            }
        }
    }
}

pub(super) fn expired_totem(run: &mut Run, pos: Vec3) {
    if ready(run, "call_of_flame") {
        if let Some(p) = numbers(run, "call_of_flame") {
            run.passives.bursts.push(Burst {
                center: pos,
                radius: p.radius,
                damage: p.dmg,
                spell: 16038,
                depth: 1,
                exclude: None,
                targets: usize::MAX,
            });
            run.passives
                .next
                .insert("call_of_flame", run.elapsed + p.cooldown);
        }
    }
}

/// Called once per cast update. Returns proc hits to use the same damage/death path as spells.
pub(super) fn tick(
    run: &mut Run,
    dt: f32,
    hero: Vec3,
    targets: &[(Entity, Vec3, f32, f32, bool)],
) -> Vec<Hit> {
    run.passives.rage_budget = (run.passives.rage_budget + dt * 10.0).min(2.0);
    run.passives.combo_budget = (run.passives.combo_budget + dt * 5.0).min(2.0);
    if run.elapsed - run.passives.last_hit > 2.0 {
        run.passives.rage = (run.passives.rage - dt * 10.0).max(0.0);
    }
    run.passives
        .marks
        .retain(|e, _| targets.iter().any(|t| t.0 == *e));
    if let Some(p) = numbers(run, "faerie_fire") {
        for origin in std::mem::take(&mut run.passives.transfers) {
            let next = targets
                .iter()
                .filter(|t| {
                    !run.passives.marks.get(&t.0).is_some_and(|m| m.faerie)
                        && super::units::flat(t.1 - origin).length() <= p.radius
                })
                .min_by(|a, b| {
                    super::units::flat(a.1 - origin)
                        .length_squared()
                        .total_cmp(&super::units::flat(b.1 - origin).length_squared())
                });
            if let Some(t) = next {
                run.passives.marks.entry(t.0).or_default().faerie = true;
            }
        }
    }
    let mut hits = Vec::new();
    for key in [
        "hunters_mark",
        "faerie_fire",
        "soul_fire",
        "bloodrage",
        "seal_fate",
    ] {
        if !ready(run, key) {
            continue;
        }
        let Some(p) = numbers(run, key) else { continue };
        let def = data::ability(key);
        let mut in_range: Vec<_> = targets
            .iter()
            .filter(|t| super::units::flat(t.1 - hero).length() <= def.range)
            .collect();
        in_range.sort_by(|a, b| {
            b.4.cmp(&a.4).then_with(|| {
                if key == "hunters_mark" {
                    b.2.total_cmp(&a.2)
                } else if key == "seal_fate" {
                    let (a_health, b_health) = (a.2 / a.3, b.2 / b.3);
                    (b_health < 0.3).cmp(&(a_health < 0.3)).then_with(|| {
                        if a_health < 0.3 && b_health < 0.3 {
                            a_health.total_cmp(&b_health)
                        } else {
                            super::units::flat(a.1 - hero)
                                .length_squared()
                                .total_cmp(&super::units::flat(b.1 - hero).length_squared())
                        }
                    })
                } else {
                    super::units::flat(a.1 - hero)
                        .length_squared()
                        .total_cmp(&super::units::flat(b.1 - hero).length_squared())
                }
            })
        });
        match key {
            "hunters_mark" if !run.passives.marks.values().any(|m| m.hunter) => {
                if let Some(t) = in_range.first() {
                    run.passives.marks.entry(t.0).or_default().hunter = true;
                }
            }
            "faerie_fire" => {
                let owned = run.passives.marks.values().filter(|m| m.faerie).count();
                let unmarked: Vec<_> = in_range
                    .into_iter()
                    .filter(|t| !run.passives.marks.get(&t.0).is_some_and(|m| m.faerie))
                    .take(p.count.saturating_sub(owned))
                    .collect();
                for t in unmarked {
                    run.passives.marks.entry(t.0).or_default().faerie = true;
                }
            }
            "bloodrage" if run.passives.rage >= 100.0 => {
                run.passives.rage = 0.0;
                run.passives.bursts.push(Burst {
                    center: hero,
                    radius: p.radius,
                    damage: p.dmg,
                    spell: def.spell,
                    depth: 1,
                    exclude: None,
                    targets: usize::MAX,
                });
                run.passives.next.insert(key, run.elapsed + p.cooldown);
            }
            "seal_fate" if run.passives.combo >= 5 && !in_range.is_empty() => {
                run.passives.combo = 0;
                for t in in_range.into_iter().take(p.count) {
                    let mut hit =
                        Hit::direct(t.0, p.dmg * if t.2 / t.3 < 0.3 { 2.0 } else { 1.0 }, false);
                    hit.spell = def.spell;
                    hits.push(hit);
                }
                run.passives.next.insert(key, run.elapsed + p.cooldown);
            }
            "soul_fire" if run.passives.shards >= 5 && !in_range.is_empty() => {
                run.passives.shards = 0;
                let mut hit = Hit::direct(in_range[0].0, p.dmg, false);
                hit.spell = def.spell;
                hits.push(hit);
                run.passives.next.insert(key, run.elapsed + p.cooldown);
            }
            _ => {}
        }
    }
    for burst in std::mem::take(&mut run.passives.bursts) {
        let mut near: Vec<_> = targets
            .iter()
            .filter(|t| {
                Some(t.0) != burst.exclude
                    && super::units::flat(t.1 - burst.center).length() <= burst.radius
            })
            .collect();
        near.sort_by(|a, b| {
            super::units::flat(a.1 - burst.center)
                .length_squared()
                .total_cmp(&super::units::flat(b.1 - burst.center).length_squared())
        });
        for t in near.into_iter().take(burst.targets) {
            let mut hit = Hit::direct(t.0, burst.damage, false);
            hit.spell = burst.spell;
            hit.proc_depth = burst.depth;
            hits.push(hit);
        }
    }
    hits
}
