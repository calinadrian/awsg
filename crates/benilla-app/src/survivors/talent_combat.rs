//! Executable milestone effects. Tags are shared by unrelated talents in the imported table;
//! dispatch by talent key, so a new description cannot silently inherit another class's effect.
use super::{
    Rng, Run,
    abilities::Hit,
    damage_meter::Source,
    data::{self, Effect, Kind},
    enemies::{Dot, Enemy},
};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct State {
    pub ranks: HashMap<&'static str, u8>,
    pub values: HashMap<&'static str, f32>,
    next: HashMap<&'static str, f32>,
    counts: HashMap<&'static str, u32>,
    bursts: Vec<Burst>,
    transfers: Vec<(Vec3, Dot, &'static str)>,
    hits: Vec<Hit>,
    pub summons: Vec<&'static str>,
    pub shield_peak: f32,
    pub shield_absorbed: f32,
    pub vulnerability: HashMap<Entity, (f32, f32)>,
    pub flurry_until: f32,
    pub focus: Option<Entity>,
    pub last_hits: HashMap<Entity, (u32, u8, Option<Source>)>,
    pub confirmed_deaths: Vec<(Entity, Vec3, u32, u8)>,
    pub echo_chains: HashMap<&'static str, u8>,
    hero: Vec3,
}

struct Burst {
    key: &'static str,
    spell: u32,
    origin: Vec3,
    exclude: Option<Entity>,
    radius: f32,
    count: usize,
    damage: f32,
    effect: Effect,
    forward: Option<Vec3>,
}

pub(super) struct Target {
    pub entity: Entity,
    pub pos: Vec3,
    pub priority: u8,
}

pub(super) fn implemented(tag: u32) -> bool {
    matches!(tag, 1 | 4..=11 | 14 | 17..=21 | 23 | 25..=30 | 32 | 33 | 35 | 36 | 259 | 267 | 271 | 287 | 290)
}

pub(super) fn begin(run: &mut Run) {
    if run.talents.has("leader_of_pack") || run.talents.has("cat_form") {
        if let Some(first) = run.abilities.first_mut() {
            *first = super::Owned::new(data::ability("shred"), super::rewards::Rarity::Common);
        }
    }
}

pub(super) fn ready(run: &mut Run, key: &'static str, seconds: f32) -> bool {
    if run
        .talents
        .combat
        .next
        .get(key)
        .is_some_and(|t| *t > run.elapsed)
    {
        return false;
    }
    run.talents.combat.next.insert(key, run.elapsed + seconds);
    true
}

fn nth(run: &mut Run, key: &'static str, every: u32) -> bool {
    let counter = run.talents.combat.counts.entry(key).or_default();
    *counter += 1;
    if *counter >= every.max(1) {
        *counter = 0;
        true
    } else {
        false
    }
}

fn burst(
    run: &mut Run,
    key: &'static str,
    spell: u32,
    origin: Vec3,
    exclude: Option<Entity>,
    radius: f32,
    count: usize,
    damage: f32,
    effect: Effect,
) {
    if damage > 0.0 {
        let forward = (key == "piercing_shots")
            .then(|| super::units::flat(origin - run.talents.combat.hero).normalize_or_zero());
        run.talents.combat.bursts.push(Burst {
            key,
            spell,
            origin,
            exclude,
            radius,
            count,
            damage,
            effect,
            forward,
        });
    }
}

fn extra(run: &mut Run, key: &'static str, spell: u32, target: Entity, damage: f32) {
    if damage <= 0.0 {
        return;
    }
    let mut hit = Hit::direct(target, damage, false);
    hit.source = Some(Source::Talent(key));
    hit.spell = spell;
    hit.proc_depth = 1;
    run.talents.combat.hits.push(hit);
}

pub(super) fn shield(run: &mut Run, amount: f32, cap: f32, spell: u32) {
    if amount <= 0.0 {
        return;
    }
    if run.shield_t > 0.0 && run.shield >= cap {
        return;
    }
    if run.shield_t <= 0.0 || run.shield <= 0.0 {
        run.shield = 0.0;
        run.talents.combat.shield_peak = 0.0;
        run.talents.combat.shield_absorbed = 0.0;
    }
    run.shield = (run.shield + amount).min(cap);
    run.shield_t = f32::INFINITY;
    run.shield_spell = spell;
    run.talents.combat.shield_peak = run.talents.combat.shield_peak.max(run.shield);
}

pub(super) fn absorbed(run: &mut Run, amount: f32, broke: bool) {
    run.talents.combat.shield_peak = run.talents.combat.shield_peak.max(run.shield + amount);
    run.talents.combat.shield_absorbed += amount;
    if !broke {
        return;
    }
    let absorbed = run.talents.combat.shield_absorbed;
    let peak = run.talents.combat.shield_peak;
    for (key, share) in [
        ("divine_aegis", run.talents.value("divine_aegis")),
        ("hammer_righteous", 0.5),
    ] {
        if run.talents.has(key) {
            let damage = if key == "hammer_righteous" {
                absorbed.min(peak)
            } else {
                absorbed
            };
            // Positioned on the hero when the queued burst is drained.
            burst(
                run,
                key,
                1535,
                Vec3::splat(f32::INFINITY),
                None,
                10.0,
                usize::MAX,
                damage * share / run.full_belly_multiplier(),
                Effect::None,
            );
        }
    }
    run.talents.combat.shield_peak = 0.0;
    run.talents.combat.shield_absorbed = 0.0;
}

/// Fatal protection is checked before hero_vitals can enter the downed/dead state.
pub(super) fn fatal(run: &mut Run) -> bool {
    if run.hp > 0.0 {
        return false;
    }
    for (key, health, cooldown, immunity) in [
        ("last_stand", 0.4, 90.0, 0.0),
        ("divine_intervention", 0.3, 90.0, 0.0),
        ("cheat_death", 0.25, 150.0, 3.0),
        ("reincarnation", 0.5, 120.0, 0.0),
    ] {
        if !run.talents.has(key) || !ready(run, key, cooldown) {
            continue;
        }
        run.hp = run.stats().max_hp * health;
        run.immune_t = run.immune_t.max(immunity);
        if key == "last_stand" {
            let max = run.stats().max_hp;
            shield(run, max * 0.1, max * 0.3, 23922);
            run.talents.queue("shield_slam", key);
        }
        if key == "divine_intervention" {
            // Divine Shield must protect the next blow, before the next auto-cast update.
            run.immune_t = run.immune_t.max(data::ability("divine_shield").duration);
            run.talents.queue("divine_shield", key);
            run.talents.queue("consecration", key);
        }
        return true;
    }
    false
}

fn add_dot(en: &mut Enemy, key: &'static str, spell: u32, total: f32, duration: f32) {
    if total <= 0.0 {
        return;
    }
    // Keep an equipped Corruption and Nightfall's Corruption independent, but refresh each
    // producer's own DoT. Both retain the Corruption spell ID for Nightfall tick triggers.
    let source = Some(Source::Talent(key));
    if let Some(dot) = en
        .dots
        .iter_mut()
        .find(|d| d.spell == spell && d.source == source)
    {
        let remaining = dot.damage * (dot.left / dot.every).ceil().max(0.0);
        dot.damage = (remaining + total) / duration;
        dot.left = duration;
    } else {
        en.dots.push(Dot {
            source,
            spell,
            damage: total / duration,
            every: 1.0,
            drain: 0.0,
            left: duration,
            tick: 1.0,
        });
    }
}

/// Runs for primary landed blows; secondary hits have depth 1 and cannot create themselves.
pub(super) fn hit(run: &mut Run, rng: &mut Rng, h: &Hit, pos: Vec3, en: &mut Enemy) {
    if h.damage > 0.0 && h.proc_depth == 1 {
        if matches!(
            h.source,
            Some(Source::Talent(
                "tame_beast"
                    | "bestial_wrath"
                    | "stormstrike"
                    | "force_of_nature"
                    | "water_elemental"
                    | "demonic_familiar"
                    | "soul_devourer"
                    | "soul_link"
                    | "legion_gate"
                    | "summon_infernal"
                    | "summon_doomguard"
                    | "pact_of_the_annihilan"
                    | "butcher"
            ))
        ) {
            super::talents::trigger(
                run,
                rng,
                super::talents::ProcTrigger::PetHit,
                h.spell,
                h.source,
            );
        }
        if h.source == Some(Source::Talent("soul_link")) {
            let max = run.stats().max_hp;
            let share = run.talents.value("soul_link");
            shield(run, h.damage * share, max * 0.1, 7812);
        }
        if h.source == Some(Source::Talent("soul_devourer")) {
            let best = en
                .dots
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| {
                    let rank = |d: &Dot| {
                        data::every_ability()
                            .find(|a| a.spell == d.spell)
                            .and_then(|a| run.abilities.iter().find(|o| o.def.key == a.key))
                            .map_or(1, |o| o.level)
                    };
                    rank(a)
                        .cmp(&rank(b))
                        .then_with(|| (a.damage / a.every).total_cmp(&(b.damage / b.every)))
                })
                .map(|(i, _)| i);
            if let Some(i) = best {
                let dot = &mut en.dots[i];
                let consumed = dot.left.max(0.0) * 0.1;
                dot.left -= consumed;
                extra(
                    run,
                    "soul_devourer",
                    h.spell,
                    h.target,
                    dot.damage / dot.every * consumed * 1.5,
                );
                if let Some(hit) = run.talents.combat.hits.last_mut() {
                    hit.proc_depth = 2;
                }
            }
        }
    }
    if h.proc_depth > 0 || (h.damage <= 0.0 && h.dot <= 0.0) {
        return;
    }
    let def = data::every_ability().find(|d| d.spell == h.spell);
    let key = def.map_or("", |d| d.key);
    let school = def.map_or(0, |d| d.school);
    run.talents.combat.focus = Some(h.target);
    for (talent, applies, dot_spell, duration) in [
        ("deep_wounds", h.crit, 12834, 6.0),
        (
            "deadly_poison",
            def.is_some_and(|d| matches!(d.kind, Kind::Melee | Kind::Execute))
                || matches!(h.spell, 0 | 6603),
            2818,
            6.0,
        ),
        (
            "holy_fire_talent",
            h.crit && matches!(key, "smite" | "holy_fire"),
            14914,
            6.0,
        ),
        ("ignite", h.crit && school == 2, 12654, 4.0),
        ("nightfall", key == "shadow_bolt", 172, 12.0),
        ("rake", matches!(key, "swipe" | "shred"), 1822, 6.0),
        ("blood_frenzy", h.crit, 1079, 6.0),
    ] {
        if applies && run.talents.has(talent) {
            let duration_mult = 1.0 + run.talents.periodic_modifier(4);
            let total = h.damage
                * run.talents.value(talent)
                * (1.0 + run.talents.periodic_modifier(0))
                * duration_mult;
            add_dot(en, talent, dot_spell, total, duration * duration_mult);
        }
    }
    for (talent, applies, radius, count) in [
        (
            "sweeping_strikes",
            matches!(key, "cleave" | "mortal_strike" | "execute"),
            8.0,
            2,
        ),
        ("divine_favor", key == "holy_shock", 8.0, 2),
        ("seal_of_command", key == "judgement", 8.0, 2),
        ("piercing_shots", key == "auto_shot", 8.0, 1),
        (
            "blade_flurry_talent",
            matches!(key, "sinister_strike" | "eviscerate"),
            8.0,
            2,
        ),
        ("presence_of_mind", key == "arcane_missiles", 8.0, 1),
        ("shadowburn", key == "shadow_bolt", 8.0, 2),
        ("divine_storm", key == "exorcism", 5.0, usize::MAX),
        (
            "shadow_dance",
            matches!(key, "backstab" | "eviscerate"),
            5.0,
            usize::MAX,
        ),
        ("splitting_ice", key == "frostbolt", 5.0, usize::MAX),
        (
            "conflagrate",
            matches!(key, "shadow_bolt" | "searing_pain") && en.dots.iter().any(|d| d.spell == 348),
            6.0,
            usize::MAX,
        ),
    ] {
        if applies && run.talents.has(talent) {
            let share = if talent == "presence_of_mind" {
                1.0
            } else {
                run.talents.value(talent)
            };
            burst(
                run,
                talent,
                h.spell,
                pos,
                Some(h.target),
                radius,
                count,
                h.damage * share,
                Effect::None,
            );
        }
    }
    if key == "shield_slam" && run.talents.has("shockwave") && ready(run, "shockwave", 0.05) {
        burst(
            run,
            "shockwave",
            h.spell,
            pos,
            None,
            8.0,
            usize::MAX,
            h.damage * run.talents.value("shockwave"),
            Effect::Stun(1.5),
        );
    }
    for (talent, applies, chance, repeats, share) in [
        (
            "mutilate",
            matches!(key, "sinister_strike" | "backstab"),
            1.0,
            1,
            0.5,
        ),
        (
            "righteous_hammer",
            key == "hammer_of_wrath",
            run.talents.value("righteous_hammer"),
            1,
            1.0,
        ),
        (
            "windfury",
            matches!(key, "lightning_bolt" | "earth_shock" | "frost_shock"),
            run.talents.value("windfury"),
            2,
            1.0,
        ),
    ] {
        if applies && run.talents.has(talent) && rng.f32() < chance {
            for _ in 0..repeats {
                extra(run, talent, h.spell, h.target, h.damage * share);
            }
        }
    }
    for (talent, applies, share) in [
        ("titan", key == "shield_slam", 0.02),
        ("hammer_righteous", key == "hammer_of_justice", 0.015),
        (
            "shadow_weaving",
            school == 5,
            run.talents.value("shadow_weaving"),
        ),
    ] {
        if applies && run.talents.has(talent) {
            extra(
                run,
                talent,
                h.spell,
                h.target,
                en.max_hp * share / run.full_belly_multiplier(),
            );
        }
    }
    for (talent, applies, effect) in [
        ("impact", school == 2, Effect::Stun(1.0)),
        ("blackout", school == 5, Effect::Stun(1.5)),
        ("frostbite", school == 4, Effect::Root(1.5)),
        (
            "entrapment",
            matches!(key, "immolation_trap" | "frost_trap"),
            Effect::Root(2.0),
        ),
        ("aftermath", key == "hellfire", Effect::Slow(0.6, 3.0)),
    ] {
        if !applies || !run.talents.has(talent) {
            continue;
        }
        let chance = if talent == "aftermath" {
            0.05 * f32::from(run.talents.rank(talent))
        } else {
            run.talents.value(talent)
        };
        if rng.f32() < chance {
            let mut hit = Hit::direct(h.target, 0.0, false);
            hit.source = Some(Source::Talent(talent));
            hit.effect = effect;
            hit.proc_depth = 1;
            run.talents.combat.hits.push(hit);
        }
    }
    if key == "rain_of_fire" && run.talents.has("rain_of_chaos") {
        en.slow = en.slow.min(0.6);
        en.slow_t = en.slow_t.max(1.5);
    }
    for (talent, applies, duration) in [
        ("imp_demo_shout", key == "demoralizing_shout", 6.0),
        ("judgement_crusader", key == "consecration", 3.0),
    ] {
        if applies && run.talents.has(talent) {
            let share = run.talents.value(talent);
            if talent == "imp_demo_shout" {
                if en.exposed_until <= run.elapsed {
                    en.exposed = 0.0;
                }
                en.exposed = en.exposed.max(share);
                en.exposed_until = run.elapsed + duration;
            } else {
                run.talents
                    .combat
                    .vulnerability
                    .insert(h.target, (share, run.elapsed + duration));
            }
        }
    }
    if key == "death_coil" && run.talents.has("dark_pact") {
        let remaining: f32 = en
            .dots
            .iter()
            .filter(|d| d.spell == 689)
            .map(|d| d.damage * (d.left / d.every).floor())
            .sum();
        extra(
            run,
            "dark_pact",
            h.spell,
            h.target,
            remaining * run.talents.value("dark_pact"),
        );
    }
    if h.crit {
        if school == 1 && run.talents.has("inspiration") && ready(run, "inspiration", 1.0) {
            run.heal(run.stats().max_hp * run.talents.value("inspiration"));
        }
        if run.talents.has("flurry_shaman") {
            run.talents.combat.flurry_until = run.elapsed + 5.0;
        }
        if school == 2
            && run.talents.has("hot_streak")
            && nth(run, "hot_streak", 4)
            && ready(run, "hot_streak", 0.5)
        {
            run.talents.queue("pyroblast", "hot_streak");
        }
    }
    if key == "swipe" && run.talents.has("leader_of_pack") {
        run.heal(run.stats().max_hp * 0.003);
    }
    for talent in ["summon_infernal", "summon_doomguard"] {
        if run.talents.has(talent) && rng.f32() < 0.05 {
            run.talents.combat.summons.push(talent);
        }
    }
}

pub(super) fn cast(run: &mut Run, spell: u32, healing: f32, hero: Vec3) {
    let def = data::every_ability().find(|d| d.spell == spell);
    let key = def.map_or("", |d| d.key);
    if run.talents.has("rapid_fire")
        && matches!(
            key,
            "auto_shot" | "multi_shot" | "aimed_shot" | "arcane_shot" | "concussive_shot"
        )
        && nth(run, "rapid_fire", run.talents.value("rapid_fire") as u32)
    {
        run.talents.queue("aimed_shot", "rapid_fire");
    }
    if run.talents.has("maelstrom") && nth(run, "maelstrom", 5) {
        run.talents.queue("chain_lightning", "maelstrom");
    }
    if healing > 0.0 && run.talents.has("tranquility") {
        burst(
            run,
            "tranquility",
            740,
            hero,
            None,
            12.0,
            usize::MAX,
            healing * run.talents.value("tranquility") / run.full_belly_multiplier(),
            Effect::None,
        );
    }
    let max = run.stats().max_hp;
    if key == "shield_slam" && (run.talents.has("fortified") || run.talents.has("shield_mastery")) {
        let mastery = run.talents.value("shield_mastery");
        let base = if run.talents.has("fortified") {
            0.1
        } else {
            0.0
        };
        shield(run, max * (base + mastery), max * (0.3 + mastery), spell);
    }
    if key == "consecration"
        && (run.talents.has("ardent_defender") || run.talents.has("blessed_life"))
    {
        let bonus = run.talents.value("blessed_life");
        let base = if run.talents.has("ardent_defender") {
            0.08
        } else {
            0.0
        };
        shield(run, max * (base + bonus), max * (0.3 + bonus), spell);
    }
}

pub(super) fn death(run: &mut Run, entity: Entity, pos: Vec3, en: &Enemy, spell: u32, depth: u8) {
    if run
        .talents
        .combat
        .last_hits
        .remove(&entity)
        .is_some_and(|(_, depth, source)| {
            depth == 1 && source == Some(Source::Talent("legion_gate"))
        })
    {
        run.talents.queue_scaled("death_coil", "legion_gate", 0.5);
    }
    let key = data::every_ability()
        .find(|d| d.spell == spell)
        .map_or("", |d| d.key);
    let school = data::every_ability()
        .find(|d| d.spell == spell)
        .map_or(0, |d| d.school);
    for (talent, applies, reset, seconds) in [
        (
            "mortal_wounds",
            matches!(key, "mortal_strike" | "execute"),
            key,
            1.0,
        ),
        ("kill_shot", key == "aimed_shot", "aimed_shot", 2.0),
    ] {
        if applies && run.talents.has(talent) && ready(run, talent, seconds) {
            for owned in &mut run.abilities {
                if owned.def.key == reset {
                    owned.cd = 0.0;
                }
            }
        }
    }
    if run.talents.has("adrenaline")
        && nth(run, "adrenaline", run.talents.value("adrenaline") as u32)
    {
        run.buffs.retain(|b| b.0 != "adrenaline");
        let duration = 11.0 * (1.0 + run.talents.modifier(data::ability("adrenaline_rush"), 4));
        run.buffs.push(("adrenaline", 9, 0.6, duration));
    }
    if depth < 4 {
        for (talent, applies, radius) in [
            (
                "explosive_ammo",
                matches!(key, "auto_shot" | "multi_shot"),
                6.0,
            ),
            ("pyroclasm", school == 2, 6.0),
            ("toxic_bloom", !en.dots.is_empty(), 7.0),
            ("seed_of_corruption", !en.dots.is_empty(), 6.0),
            ("shadowform", en.dots.iter().any(|d| d.spell == 589), 6.0),
        ] {
            if applies
                && run.talents.has(talent)
                && (talent != "pyroclasm" || ready(run, talent, 1.0 / 3.0))
            {
                burst(
                    run,
                    talent,
                    spell,
                    pos,
                    None,
                    radius,
                    usize::MAX,
                    en.max_hp * run.talents.value(talent) / run.full_belly_multiplier(),
                    Effect::None,
                );
            }
        }
    }
    for dot in &en.dots {
        let talent = if run.talents.has("contagion") && matches!(dot.spell, 172 | 980) {
            Some("contagion")
        } else if run.talents.has("shadowform") && dot.spell == 589 {
            Some("shadowform")
        } else {
            None
        };
        if let Some(talent) = talent {
            run.talents
                .combat
                .transfers
                .push((pos, dot.clone(), talent));
        }
    }
}

pub(super) fn tick(run: &mut Run, hero: Vec3, targets: &[Target]) -> Vec<Hit> {
    run.talents.combat.hero = hero;
    for (talent, seconds, spell) in [
        ("leader_of_pack", 3.0, "swipe"),
        ("tree_of_life", 6.0, "rejuvenation"),
    ] {
        if run.talents.has(talent) && ready(run, talent, seconds) {
            run.talents.queue(spell, talent);
        }
    }
    run.talents
        .combat
        .vulnerability
        .retain(|e, (_, until)| *until > run.elapsed && targets.iter().any(|t| t.entity == *e));
    run.talents
        .combat
        .last_hits
        .retain(|e, _| targets.iter().any(|t| t.entity == *e));
    let mut hits = std::mem::take(&mut run.talents.combat.hits);
    for b in std::mem::take(&mut run.talents.combat.bursts) {
        let origin = if b.origin.is_finite() { b.origin } else { hero };
        let mut near: Vec<_> = targets
            .iter()
            .filter(|t| {
                Some(t.entity) != b.exclude
                    && super::units::flat(t.pos - origin).length() <= b.radius
                    && b.forward.is_none_or(|aim| {
                        super::units::flat(t.pos - origin)
                            .normalize_or_zero()
                            .dot(aim)
                            >= 0.8
                    })
            })
            .collect();
        near.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then_with(|| {
                a.pos
                    .distance_squared(origin)
                    .total_cmp(&b.pos.distance_squared(origin))
            })
        });
        for t in near.into_iter().take(b.count) {
            let mut hit = Hit::direct(t.entity, b.damage, false);
            hit.source = Some(Source::Talent(b.key));
            hit.spell = b.spell;
            hit.effect = b.effect;
            hit.proc_depth = 1;
            hits.push(hit);
        }
    }
    for (pos, dot, _talent) in std::mem::take(&mut run.talents.combat.transfers) {
        if let Some(t) = targets.iter().min_by(|a, b| {
            a.pos
                .distance_squared(pos)
                .total_cmp(&b.pos.distance_squared(pos))
        }) {
            let mut hit = Hit::direct(t.entity, 0.0, false);
            hit.source = dot.source;
            hit.spell = dot.spell;
            hit.dot = dot.damage / dot.every;
            hit.dot_every = dot.every;
            hit.dot_dur = dot.left.max(dot.every);
            hit.effect = Effect::Drain(dot.drain);
            hit.proc_depth = 1;
            hits.push(hit);
        }
    }
    hits
}

/// Display fields use the engine's existing live-morph path, preserving the player entity.
pub(super) fn forms(
    run: Res<Run>,
    mut hero: Query<&mut crate::net::ObjectStore, With<super::hero::Hero>>,
) {
    let display = if run.talents.has("leader_of_pack") {
        Some(2281)
    } else if run.talents.has("cat_form") {
        Some(892)
    } else {
        None
    };
    if let (Some(display), Ok(mut store)) = (display, hero.single_mut()) {
        if store.0.unit_displayid() != Some(display as i32) {
            store
                .0
                .merge(benilla_protocol::messages::ObjectFields::from_pairs(&[(
                    131, display,
                )]));
        }
    }
}
