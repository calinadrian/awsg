//! Talent companions are local combatants owned by each player's build. Their hits follow
//! the ordinary damage/replica submission path; they never apply damage directly to host HP.
use super::{
    Rng, Run,
    abilities::{self, Hit},
    damage_meter::Source,
    data::{self, Effect},
    enemies::Enemy,
    hero::Hero,
    units::{self, Fx, UnitSpec},
};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Wolf,
    Raptor,
    SpiritWolf,
    Treant,
    Water,
    Imp,
    Felhunter,
    Voidwalker,
    Succubus,
    LesserFelguard,
    Infernal,
    Doomguard,
    PitLord,
    Felguard,
}

impl Kind {
    fn display(self) -> u32 {
        match self {
            Self::Wolf | Self::SpiritWolf => 604,
            Self::Raptor => 2571,
            Self::Treant => 864,
            Self::Water => 1453,
            Self::Imp => 4449,
            Self::Felhunter => 8503,
            Self::Voidwalker => 1132,
            Self::Succubus => 4162,
            Self::LesserFelguard | Self::Felguard => 9017,
            Self::Infernal => 10906,
            Self::Doomguard => 4426,
            Self::PitLord => 2418,
        }
    }
    fn spell(self) -> &'static str {
        match self {
            Self::Water => "frostbolt",
            Self::Imp => "fireball",
            Self::Doomguard => "shadow_bolt",
            Self::PitLord => "searing_pain",
            Self::Infernal => "hellfire",
            _ => "cleave",
        }
    }
    fn ranged(self) -> bool {
        matches!(
            self,
            Self::Water | Self::Imp | Self::Doomguard | Self::PitLord
        )
    }
    fn demon(self) -> bool {
        !matches!(
            self,
            Self::Wolf | Self::Raptor | Self::SpiritWolf | Self::Treant | Self::Water
        )
    }
}

#[derive(Component)]
pub(super) struct Companion {
    kind: Kind,
    source: &'static str,
    until: f32,
    next: f32,
    swings: u32,
    target: Option<Entity>,
    streak: u32,
}

fn spawn(
    commands: &mut Commands,
    run: &mut Run,
    source: &'static str,
    kind: Kind,
    duration: f32,
    pos: Vec3,
    index: usize,
) {
    let angle = index as f32 * 2.4;
    let offset = Vec3::new(angle.cos(), 0.0, angle.sin()) * 3.0;
    let id = units::spawn_unit(
        commands,
        &UnitSpec {
            serial: run.guid(),
            display: kind.display(),
            scale: if matches!(kind, Kind::PitLord | Kind::Infernal) {
                0.6
            } else {
                1.0
            },
            level: 60,
            faction: units::FACTION_FRIENDLY,
            bytes0: 0,
            weapons: &[],
            sheath: 1,
        },
        pos + offset,
        0.0,
    );
    commands.entity(id).insert(Companion {
        kind,
        source,
        until: run.elapsed + duration,
        next: run.elapsed + 0.5,
        swings: 0,
        target: None,
        streak: 0,
    });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn tick(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    hero: Query<&Transform, (With<Hero>, Without<Companion>, Without<Enemy>)>,
    mut pets: Query<
        (
            Entity,
            &mut Transform,
            &mut Companion,
            &mut crate::creature_anim::MovementState,
        ),
        (Without<Hero>, Without<Enemy>),
    >,
    enemies: Query<(Entity, &Transform, &Enemy), (Without<Hero>, Without<Companion>)>,
    mut hits: MessageWriter<Hit>,
    mut fx: Fx,
) {
    let Ok(hero) = hero.single() else { return };
    if run.dead_t.is_some() {
        return;
    }
    let dt = time.delta_secs();
    let now = run.elapsed;
    let stats = run.stats();
    let mut counts = HashMap::<Kind, usize>::new();
    for (_, _, pet, _) in &pets {
        if pet.until > now {
            *counts.entry(pet.kind).or_default() += 1;
        }
    }
    let mut requests = Vec::new();
    let supremacy = run.talents.stat(32) > 0.0;
    for (source, kind, count) in [
        ("tame_beast", Kind::Wolf, 1),
        ("bestial_wrath", Kind::Raptor, 1),
        ("stormstrike", Kind::SpiritWolf, 2),
        ("force_of_nature", Kind::Treant, 2),
        ("water_elemental", Kind::Water, 1),
        ("demonic_familiar", Kind::Imp, 1),
        ("soul_devourer", Kind::Felhunter, 1),
        ("soul_link", Kind::Voidwalker, 1),
    ] {
        if run.talents.has(source) && !(supremacy && kind.demon()) {
            for _ in counts.get(&kind).copied().unwrap_or(0)..count {
                requests.push((source, kind, f32::INFINITY));
            }
        }
    }
    if run.talents.has("bestial_wrath")
        && !run.talents.has("tame_beast")
        && !counts.contains_key(&Kind::Wolf)
    {
        requests.push(("bestial_wrath", Kind::Wolf, f32::INFINITY));
    }
    if run.talents.has("legion_gate")
        && !supremacy
        && super::talent_combat::ready(run.as_mut(), "legion_gate", 4.0)
    {
        let lesser: usize = pets
            .iter()
            .filter(|(_, _, p, _)| p.source == "legion_gate" && p.until > now)
            .count();
        for _ in 0..3.min(12usize.saturating_sub(lesser)) {
            let kind = [
                Kind::Imp,
                Kind::Voidwalker,
                Kind::Succubus,
                Kind::LesserFelguard,
            ][rng.index(4)];
            requests.push(("legion_gate", kind, rng.range(12.0, 16.0)));
        }
    }
    for (source, kind) in [
        ("pact_of_the_annihilan", Kind::PitLord),
        ("butcher", Kind::Felguard),
    ] {
        if run.talents.has(source)
            && super::talent_combat::ready(
                run.as_mut(),
                source,
                (30.0 / (1.0 + stats.haste)).max(13.0),
            )
            && counts.get(&kind).copied().unwrap_or(0) < 2
        {
            requests.push((source, kind, 20.0));
        }
    }
    for source in std::mem::take(&mut run.talents.combat.summons) {
        let kind = if source == "summon_infernal" {
            Kind::Infernal
        } else {
            Kind::Doomguard
        };
        if counts.get(&kind).copied().unwrap_or(0) > 0 {
            continue;
        }
        counts.insert(kind, 1);
        requests.push((source, kind, if supremacy { f32::INFINITY } else { 15.0 }));
    }
    let permanent = pets.iter().filter(|(_, _, p, _)| p.until > now).count();
    for (i, (source, kind, duration)) in requests.into_iter().enumerate() {
        let center = if kind == Kind::Infernal {
            enemies
                .iter()
                .filter(|(_, _, en)| !en.dead && !en.untargetable())
                .min_by(|a, b| {
                    a.1.translation
                        .distance_squared(hero.translation)
                        .total_cmp(&b.1.translation.distance_squared(hero.translation))
                })
                .map_or(hero.translation, |(_, tf, _)| tf.translation)
        } else {
            hero.translation
        };
        spawn(
            &mut commands,
            &mut run,
            source,
            kind,
            duration,
            center,
            permanent + i,
        );
        if kind == Kind::Infernal {
            for (e, tf, en) in &enemies {
                if !en.dead
                    && !en.untargetable()
                    && units::flat(tf.translation - center).length() <= 8.0
                {
                    let mut hit = Hit::direct(e, 40.0 * stats.might, false);
                    hit.source = Some(Source::Talent(source));
                    hit.spell = 1949;
                    hit.effect = Effect::Stun(2.0);
                    hit.proc_depth = 1;
                    hits.write(hit);
                }
            }
        }
    }
    for (entity, mut tf, mut pet, mut movement) in &mut pets {
        if pet.until <= now
            || (supremacy
                && matches!(
                    pet.kind,
                    Kind::Imp
                        | Kind::Felhunter
                        | Kind::Voidwalker
                        | Kind::Succubus
                        | Kind::LesserFelguard
                ))
        {
            commands.entity(entity).despawn();
            continue;
        }
        let mut targets: Vec<_> = enemies
            .iter()
            .filter(|(_, tf, en)| {
                !en.dead
                    && !en.untargetable()
                    && units::flat(tf.translation - hero.translation).length() <= 40.0
            })
            .collect();
        targets.sort_by(|a, b| {
            b.2.priority()
                .cmp(&a.2.priority())
                .then_with(|| {
                    (Some(b.0) == run.talents.combat.focus)
                        .cmp(&(Some(a.0) == run.talents.combat.focus))
                })
                .then_with(|| {
                    a.1.translation
                        .distance_squared(tf.translation)
                        .total_cmp(&b.1.translation.distance_squared(tf.translation))
                })
        });
        let destination = targets
            .first()
            .map_or(hero.translation, |(_, tf, _)| tf.translation);
        let reach = if targets.is_empty() {
            3.0
        } else if pet.kind.ranged() {
            30.0
        } else {
            5.0
        };
        let delta = units::flat(destination - tf.translation);
        let moving = delta.length() > reach;
        if delta.length_squared() > 0.01 {
            tf.rotation = Quat::from_rotation_y(units::yaw_toward(delta));
        }
        if moving {
            let step = (delta.length() - reach).min(10.0 * dt);
            tf.translation += delta.normalize_or_zero() * step;
            tf.translation.y = hero.translation.y;
            tf.rotation = Quat::from_rotation_y(units::yaw_toward(delta));
        }
        movement.speed = if moving { 10.0 } else { 0.0 };
        movement.flags = if moving {
            crate::creature_anim::move_flags::FORWARD
        } else {
            0
        };
        if now < pet.next || moving || targets.is_empty() {
            continue;
        }
        pet.next = now + if pet.kind.ranged() { 2.0 } else { 1.5 };
        pet.swings += 1;
        let def = data::ability(pet.kind.spell());
        let fallback = super::Owned::new(def, super::rewards::Rarity::Common);
        let owned = run
            .abilities
            .iter()
            .find(|o| o.def.key == def.key)
            .unwrap_or(&fallback);
        let p = abilities::parameters(owned, &run);
        let mut base = p.dmg * (1.0 + run.talents.stat(17));
        if pet.kind == Kind::Imp {
            base *= 1.0 + run.talents.stat(30);
        }
        if pet.kind == Kind::Infernal {
            base *= run.talents.value("summon_infernal");
        }
        if pet.kind == Kind::Voidwalker {
            let bonus_health = run.talents.stat(2) + run.jewel_stat(2) + run.jewel_stat(6);
            base *= 1.0 + 3.0 * stats.reduction + 2.0 * bonus_health;
        }
        if pet.kind == Kind::Felguard && pet.swings % 3 == 0 {
            base *= 2.0;
        }
        let first = targets[0].0;
        if pet.kind == Kind::Doomguard {
            pet.streak = if pet.target == Some(first) && targets[0].2.priority() > 0 {
                pet.streak + 1
            } else {
                0
            };
            base *= 1.0 + (pet.streak as f32 * 0.2).min(run.talents.value("summon_doomguard"));
        }
        pet.target = Some(first);
        let count = match pet.kind {
            Kind::Felhunter | Kind::Voidwalker | Kind::Doomguard => 2,
            Kind::PitLord => 6,
            Kind::Felguard => 10,
            Kind::Infernal => usize::MAX,
            _ => 1,
        };
        let radius = match pet.kind {
            Kind::Felguard => 15.0,
            Kind::Infernal => 7.0,
            _ => reach,
        };
        let mut chosen: Vec<_> = targets
            .iter()
            .filter(|(_, target_tf, _)| {
                units::flat(target_tf.translation - tf.translation).length() <= radius
            })
            .take(count)
            .copied()
            .collect();
        if pet.kind == Kind::Imp {
            for t in &targets {
                if t.2.dots.iter().any(|d| d.spell == 348) && !chosen.iter().any(|c| c.0 == t.0) {
                    chosen.push(*t);
                }
            }
        }
        let echo = pet.kind.demon() && rng.f32() < run.talents.stat(31);
        for (index, (target, _, enemy)) in chosen.into_iter().enumerate() {
            let mut damage = base;
            if pet.kind == Kind::Felhunter {
                damage *= 1.0
                    + (1.0 - enemy.hp / enemy.max_hp) * run.talents.value("soul_devourer") * 100.0;
            }
            if pet.kind == Kind::PitLord && index == 0 {
                damage *= 1.25;
            }
            if pet.kind == Kind::Felguard && enemy.hp / enemy.max_hp < 0.3 {
                damage *= 1.5;
            }
            let crit = rng.f32()
                < (stats.crit
                    + if pet.kind.demon() {
                        run.talents.stat(29)
                    } else {
                        0.0
                    });
            if crit {
                damage *= stats.crit_mult;
            }
            let windfury =
                pet.kind == Kind::SpiritWolf && rng.f32() < run.talents.value("stormstrike");
            let repeats = 1 + usize::from(echo) + if windfury { 2 } else { 0 };
            for _ in 0..repeats {
                let mut hit = Hit::direct(target, damage, crit);
                hit.source = Some(Source::Talent(pet.source));
                hit.spell = def.spell;
                hit.proc_depth = 1;
                if pet.kind == Kind::Succubus && enemy.dots.iter().any(|d| d.spell == 172) {
                    hit.effect = Effect::Stun(1.5);
                }
                hits.write(hit);
            }
            if pet.kind.ranged() {
                fx.cast(entity, def.spell, &[target], None);
            } else {
                fx.swing(entity, target, damage);
            }
            if matches!(pet.kind, Kind::Wolf | Kind::Raptor) && run.talents.has("bestial_wrath") {
                let mut nearby: Vec<_> = targets
                    .iter()
                    .filter(|t| {
                        t.0 != target
                            && units::flat(t.1.translation - enemy_position(&targets, target))
                                .length()
                                <= 6.0
                    })
                    .collect();
                nearby.sort_by(|a, b| b.2.priority().cmp(&a.2.priority()));
                for t in nearby.into_iter().take(2) {
                    let mut hit =
                        Hit::direct(t.0, damage * run.talents.value("bestial_wrath"), false);
                    hit.source = Some(Source::Talent("bestial_wrath"));
                    hit.spell = def.spell;
                    hit.proc_depth = 2;
                    hits.write(hit);
                }
            }
        }
    }
}

fn enemy_position(targets: &[(Entity, &Transform, &Enemy)], entity: Entity) -> Vec3 {
    targets
        .iter()
        .find(|t| t.0 == entity)
        .map_or(Vec3::ZERO, |t| t.1.translation)
}
