//! Local instant casts have no server cast-animation release to wait for.
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct ImmediateMissiles;

pub(super) fn prepare(mut units: Query<&mut crate::entities::BoneAttach, With<ImmediateMissiles>>) {
    for mut bones in &mut units {
        complete_markers(&mut bones);
    }
}

/// Imported character bodies retain spell-hand attachments, but may omit `$CSL`/`$CSR` events.
pub(crate) fn complete_markers(bones: &mut crate::entities::BoneAttach) {
    for (ident, tag) in [(*b"$CSL", 0x15), (*b"$CSR", 0x16)] {
        if !bones.markers.contains_key(&ident) {
            if let Some(&point) = bones.points.get(&tag) {
                bones.markers.insert(ident, point);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn absent_cast_markers_use_spell_hands_and_authored_markers_win() {
        let hand = (3, Vec3::new(0.0, 1.5, 0.0));
        let authored = (5, Vec3::new(0.0, 2.0, 0.0));
        let mut bones = crate::entities::BoneAttach {
            points: HashMap::from([(0x15, hand), (0x16, hand)]),
            markers: HashMap::from([(*b"$CSR", authored)]),
        };
        complete_markers(&mut bones);
        assert_eq!(bones.markers[&*b"$CSL"], hand);
        assert_eq!(bones.markers[&*b"$CSR"], authored);
    }
}
