//! The ring of fire on the battleground's leash: the edge the hero cannot cross, drawn where he
//! would otherwise meet an invisible wall.

use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

use super::{hero, units, Run, Selection};

/// `GameObjectDisplayInfo` 398, `UndeadFireLarge.mdx`: a tall standing flame.
const FIRE: u32 = 398;
const FIRE_SCALE: f32 = 1.0;
/// Yards between two fires along the ring.
const SPACING: f32 = 12.0;

/// One fire of the ring, by its place around it.
#[derive(Component)]
pub(super) struct RingFire(usize);

/// Light the ring's fires, each as soon as the ground under it has streamed in (the far side of a
/// wide leash arrives after the run starts); a run's end despawns them with the rest of the run.
pub(super) fn light_ring(
    mut commands: Commands,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    world: WorldPoint,
    lit: Query<&RingFire>,
) {
    let map = selection.stage_def();
    // Underground the cave walls and the barricades are the edge.
    if super::dungeon::is_dungeon(map) {
        return;
    }
    let count = ((std::f32::consts::TAU * map.leash) / SPACING)
        .ceil()
        .max(8.0) as usize;
    if lit.iter().len() >= count {
        return;
    }
    let mut done = vec![false; count];
    for f in &lit {
        if let Some(d) = done.get_mut(f.0) {
            *d = true;
        }
    }
    let center = hero::arena(map);
    let mut fresh = 0;
    for (i, _) in done.iter().enumerate().filter(|(_, d)| !**d) {
        let a = i as f32 / count as f32 * std::f32::consts::TAU;
        let mut p = center + Vec3::new(a.cos(), 0.0, a.sin()) * map.leash;
        let Some(h) = world.terrain_height_under(p) else {
            continue;
        };
        p.y = h;
        let serial = run.guid();
        let e = units::spawn_object(&mut commands, serial, FIRE, FIRE_SCALE, p);
        commands.entity(e).insert(RingFire(i));
        fresh += 1;
    }
    if fresh > 0 {
        debug!(
            "ring: lit {fresh} more, {} of {count}",
            lit.iter().len() + fresh
        );
    }
}
