//! Seed the normal incremental dressing path for a local Survivors character unit.
use bevy::prelude::*;

pub(crate) fn seed(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .insert(super::equipment::AppliedEquipment(
            super::Equipment::default(),
        ));
}
