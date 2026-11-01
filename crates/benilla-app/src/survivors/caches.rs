//! Caches require contact, never magnet pickup. Elite/boss/guardian drops are free;
//! arena caches spend gold once and advance the original 1.3x + 5 price curve.
use super::{
    hero::{self, Hero},
    Phase, Run, RunEntity, Selection,
};
use bevy::prelude::*;
#[derive(Component)]
pub(super) struct Cache {
    paid: bool,
}
pub(super) fn spawn(commands: &mut Commands, serial: u64, at: Vec3, paid: bool) {
    let e = super::loot_ground::spawn(commands, serial, 259, 1.3, at);
    commands.entity(e).insert((Cache { paid }, RunEntity));
}
pub(super) fn place(
    mut commands: Commands,
    selection: Res<Selection>,
    mut run: ResMut<Run>,
    ground: super::dungeon::Ground,
) {
    let center = hero::arena(selection.stage_def());
    for offset in [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z] {
        let at = ground.spawn_spot(center + offset * 18., center.y);
        spawn(&mut commands, run.guid(), at, true);
    }
}
pub(super) fn collect(
    mut commands: Commands,
    hero: Query<&Transform, With<Hero>>,
    caches: Query<(Entity, &Transform, &Cache, &Visibility), Without<Hero>>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    mut reminded: Local<f32>,
    time: Res<Time>,
) {
    let Ok(h) = hero.single() else { return };
    if run.dead_t.is_some() {
        return;
    }
    *reminded = (*reminded - time.delta_secs()).max(0.);
    for (e, tf, c, visible) in &caches {
        if *visible == Visibility::Hidden {
            continue;
        }
        if super::units::flat(tf.translation - h.translation).length() > 3. {
            continue;
        }
        if c.paid && run.gold < run.chest_cost {
            if *reminded == 0. {
                run.banner = Some((format!("Cache requires {} gold", run.chest_cost), 2.));
                *reminded = 3.;
            }
            continue;
        }
        if c.paid {
            run.gold -= run.chest_cost;
            run.chest_cost = next_cost(run.chest_cost);
        }
        run.pending_chests += 1;
        info!(
            "survivors cache: opened paid={} gold={} next_cost={} pending={}",
            c.paid, run.gold, run.chest_cost, run.pending_chests
        );
        commands.entity(e).despawn();
        next.set(Phase::LevelUp);
        break;
    }
}
pub(super) fn next_cost(cost: u32) -> u32 {
    (cost as f32 * 1.3 + 5.) as u32
}
#[cfg(test)]
mod tests {
    #[test]
    fn original_price_sequence() {
        let mut c = 20;
        for expected in [31, 45, 63, 86] {
            c = super::next_cost(c);
            assert_eq!(c, expected)
        }
    }
}
