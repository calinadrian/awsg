//! Auto attack: a clean right-click on an enemy, as in the 1.12 client, starts the hero's white
//! swings at it. They land on their own clock while it is in reach, beside the auto-cast spells,
//! and stop when it dies, the hero falls or the run ends.

use benilla_world::interact::WorldRightClick;
use bevy::prelude::*;

use super::abilities::Hit;
use super::enemies::Enemy;
use super::hero::Hero;
use super::units::{flat, Fx};
use super::{Rng, Run};

/// The enemy the hero is swinging at, and the seconds until the next swing.
#[derive(Resource, Default)]
pub(super) struct AutoAttack {
    pub target: Option<Entity>,
    swing: f32,
}

/// One class's white attack: `Attack` for the melee classes, `Auto Shot` for hunters and `Shoot`
/// (the wand) for the cloth casters.
struct Swing {
    spell: u32,
    reach: f32,
    every: f32,
    damage: f32,
    ranged: bool,
}

fn swing_for(class: u8) -> Swing {
    match class {
        3 => Swing { spell: 75, reach: 30.0, every: 2.0, damage: 20.0, ranged: true },
        5 | 8 | 9 => Swing { spell: 5019, reach: 28.0, every: 1.6, damage: 14.0, ranged: true },
        _ => Swing { spell: 6603, reach: 4.0, every: 2.0, damage: 24.0, ranged: false },
    }
}

/// A clean right-click whose press was over a living enemy makes it the auto-attack target.
pub(super) fn start_on_right_click(
    mut clicks: MessageReader<WorldRightClick>,
    press: Res<crate::target::PressPick>,
    enemies: Query<&Enemy>,
    mut auto: ResMut<AutoAttack>,
) {
    if clicks.read().last().is_none() {
        return;
    }
    let Some(picked) = press.hovered.target else { return };
    if enemies.get(picked).is_ok_and(|e| !e.dead && e.hp > 0.0) && auto.target != Some(picked) {
        auto.target = Some(picked);
        // The first swing comes at once, as the client's does on a fresh target.
        auto.swing = 0.0;
    }
}

/// Swing at the target whenever the timer is up and it stands in reach.
pub(super) fn swing(
    time: Res<Time>,
    run: Res<Run>,
    mut rng: ResMut<Rng>,
    mut auto: ResMut<AutoAttack>,
    hero: Query<(Entity, &Transform), With<Hero>>,
    enemies: Query<(&Transform, &Enemy)>,
    mut fx: Fx,
    mut hits: MessageWriter<Hit>,
) {
    let dt = time.delta_secs();
    auto.swing = (auto.swing - dt).max(0.0);
    let Some(target) = auto.target else { return };
    let Some((tf, enemy)) = enemies.get(target).ok().filter(|(_, e)| !e.dead && e.hp > 0.0) else {
        auto.target = None;
        return;
    };
    if run.stunned() {
        return;
    }
    if run.dead_t.is_some() {
        auto.target = None;
        return;
    }
    let Ok((me, htf)) = hero.single() else { return };
    let kind = swing_for(run.class.id);
    let reach = kind.reach + super::hero::HERO_RADIUS + enemy.radius;
    if auto.swing > 0.0 || flat(tf.translation - htf.translation).length() > reach {
        return;
    }
    let stats = run.stats();
    auto.swing = kind.every / (1.0 + stats.haste);
    let mut damage = kind.damage * stats.might;
    let crit = rng.f32() < stats.crit;
    if crit {
        damage *= stats.crit_mult;
    }
    if kind.ranged {
        fx.cast(me, kind.spell, &[target], None);
    } else {
        fx.swing(me, target, damage);
    }
    let mut hit = Hit::direct(target, damage, crit);
    hit.spell = kind.spell;
    hits.write(hit);
}

/// How far the target frame looks for the enemy the hero's spells are focusing.
const FOCUS_RANGE: f32 = 30.0;

/// The target frame shows what the hero is attacking: the auto-attack target, else the enemy the
/// auto-cast spells pick first: boss adds, bosses, elites, then regulars; a mark and distance
/// break ties within a rank. Explicit right-click attacks retain their selected target.
pub(super) fn follow_focus(
    run: Res<Run>,
    auto: Res<AutoAttack>,
    mut selection: ResMut<crate::target::Selection>,
    hero: Query<&Transform, With<Hero>>,
    enemies: Query<(Entity, &Transform, &Enemy, &crate::net::Guid)>,
) {
    let Ok(htf) = hero.single() else { return };
    let living = |en: &Enemy| !en.dead && en.hp > 0.0;
    let dist = |tf: &Transform| super::hero::reach(htf.translation, tf.translation);
    let want = auto
        .target
        .filter(|&e| enemies.get(e).is_ok_and(|(_, _, en, _)| living(en)))
        .or_else(|| {
            enemies
                .iter()
                .filter(|(_, tf, en, _)| {
                    living(en) && !en.untargetable() && dist(tf) <= FOCUS_RANGE + en.radius
                })
                .min_by(|a, b| {
                    let marked = |e: Entity| run.passives.marks.get(&e).is_some_and(|m| m.hunter);
                    b.2.priority()
                        .cmp(&a.2.priority())
                        .then_with(|| marked(b.0).cmp(&marked(a.0)))
                        .then_with(|| dist(a.1).total_cmp(&dist(b.1)))
                })
                .map(|(e, ..)| e)
        });
    if selection.target != want {
        selection.target = want;
        selection.guid = want.and_then(|e| enemies.get(e).ok()).map(|(_, _, _, guid)| guid.0);
    }
}

/// A new run starts with nothing targeted.
pub(super) fn reset(mut auto: ResMut<AutoAttack>) {
    *auto = AutoAttack::default();
}
