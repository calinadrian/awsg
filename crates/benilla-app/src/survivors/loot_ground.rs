//! Terrain-seated loot. Keep it hidden until the local terrain/floor is available;
//! re-sample as streamed map chunks arrive instead of retaining a fallback height.
use super::{dungeon::Ground, units};
use bevy::prelude::*;
#[derive(Component)]
pub(super) struct GroundedLoot;
pub(super) fn spawn(
    commands: &mut Commands,
    serial: u64,
    display: u32,
    scale: f32,
    at: Vec3,
) -> Entity {
    let e = units::spawn_object(commands, serial, display, scale, at);
    commands
        .entity(e)
        .insert((GroundedLoot, Visibility::Hidden));
    e
}
pub(super) fn seat(
    ground: Ground,
    mut loot: Query<(&mut Transform, &mut Visibility), With<GroundedLoot>>,
) {
    for (mut tf, mut visible) in &mut loot {
        if let Some(y) = ground.height(tf.translation) {
            tf.translation.y = y;
            *visible = Visibility::Inherited;
        } else {
            *visible = Visibility::Hidden;
        }
    }
}
