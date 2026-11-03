//! Caches require contact, never magnet pickup. Elite/boss/guardian drops are free;
//! arena caches spend gold once and advance the original 1.3x + 5 price curve.
use super::{
    Phase, Run, RunEntity, Selection,
    hero::{self, Hero},
};
use bevy::prelude::*;
#[derive(Resource)]
pub(super) struct Spawner(f32);
impl Default for Spawner {
    fn default() -> Self {
        Self(70.0)
    }
}

fn random_spot(
    center: Vec3,
    selection: &Selection,
    ground: &super::dungeon::Ground,
    rng: &mut super::Rng,
) -> Vec3 {
    let angle = rng.range(0.0, std::f32::consts::TAU);
    let radius = rng.range(18.0, 50.0);
    let wanted = center + Vec3::new(angle.cos(), 0.0, angle.sin()) * radius;
    let arena = hero::arena(selection.stage_def());
    let offset = super::units::flat(wanted - arena).clamp_length_max(selection.stage_def().leash);
    ground.spawn_spot(arena + offset, center.y)
}

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
    mut rng: ResMut<super::Rng>,
    mut spawner: ResMut<Spawner>,
) {
    let center = hero::arena(selection.stage_def());
    spawner.0 = 70.0;
    // Native 0x5bf8386: two entry caches, each 18–50 yards from the hero.
    for _ in 0..2 {
        let at = random_spot(center, &selection, &ground, &mut rng);
        spawn(&mut commands, run.guid(), at, true);
    }
}

pub(super) fn replenish(
    mut commands: Commands,
    selection: Res<Selection>,
    mut run: ResMut<Run>,
    ground: super::dungeon::Ground,
    mut rng: ResMut<super::Rng>,
    mut spawner: ResMut<Spawner>,
    hero: Query<&Transform, With<Hero>>,
    caches: Query<&Cache>,
) {
    if run.elapsed < spawner.0 || run.dead_t.is_some() {
        return;
    }
    let Ok(hero) = hero.single() else { return };
    // Native 0x5c1798a: check every 70 seconds, with at most three paid caches.
    spawner.0 = run.elapsed + 70.0;
    if caches.iter().filter(|cache| cache.paid).count() >= 3 {
        return;
    }
    let at = random_spot(hero.translation, &selection, &ground, &mut rng);
    spawn(&mut commands, run.guid(), at, true);
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
