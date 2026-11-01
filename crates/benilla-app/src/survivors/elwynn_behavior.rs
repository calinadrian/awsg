//! Elwynn rules read from the installed ASR 0.124 executable.
//!
//! Selection: RVAs 0x5c7ec90 and 0x5c8a2e0. Melee approach: 0x5c0bb91–0x5c0c1b9.

/// The native enemy table's budget costs (+0x44), in regular-roster order.
pub(super) const COSTS: [f32; 9] = [1.0, 1.0, 1.0, 2.0, 1.5, 3.0, 1.5, 2.0, 3.0];

/// Regular horde stats at spawn, without difficulty, co-op or endless modifiers.
/// Native RVAs 0x5c00340/0x5c6a330; Elwynn's zone multiplier is 0.82.
pub(super) fn regular_stats(hp: f32, damage: f32, speed: f32, elapsed: f32) -> (f32, f32, f32) {
    let minute = elapsed.max(0.0) / 60.0;
    let health_curve =
        1.0 + 0.10 * minute + 0.036 * minute * minute + 0.00704 * minute * minute * minute;
    let damage_curve =
        1.0 + 0.07 * minute + 0.015 * minute * minute + 0.0002 * minute * minute * minute;
    (
        hp * 4.95 * 0.82 * health_curve,
        damage * 1.25 * 0.82_f32.sqrt() * damage_curve,
        speed * 1.05,
    )
}

/// Older unlocked enemies retain weight; their cost reduces how often they are selected.
pub(super) fn weight(elapsed: f32, from: f32, cost: f32) -> f32 {
    if elapsed < from {
        return 0.0;
    }
    (1.0 + (elapsed - from).max(0.0) / 120.0).min(4.0) / cost
}

/// A stable weighted draw, including the last bucket at a rounded upper boundary.
pub(super) fn choose(weights: &[f32], unit: f32) -> Option<usize> {
    let total: f32 = weights.iter().sum();
    if total <= 0.0 {
        return None;
    }
    let mut draw = unit.clamp(0.0, 1.0) * total;
    let mut last = None;
    for (index, &weight) in weights.iter().enumerate() {
        if weight <= 0.0 {
            continue;
        }
        last = Some(index);
        if draw < weight {
            return Some(index);
        }
        draw -= weight;
    }
    last
}

/// Native melee reach is 1.2 × model scale, independent of neighbour separation.
pub(super) fn melee_reach(scale: f32) -> f32 {
    1.2 * scale
}

/// The native approach aims around the hero rather than stacking every mob at his centre.
pub(super) fn approach_radius(scale: f32) -> f32 {
    (0.8 * melee_reach(scale)).max(1.5)
}

/// The native move command is at most 14 yards, refreshed after 0.25–0.7 seconds.
pub(super) fn repath_after(distance: f32, speed: f32) -> f32 {
    (distance / speed.max(0.5)).clamp(0.25, 0.7)
}

/// A short move towards a slot around the hero, preserving the approaching side.
pub(super) fn approach_goal(pos: [f32; 2], hero: [f32; 2], angle: f32, radius: f32) -> [f32; 2] {
    let target = [
        hero[0] + angle.cos() * radius,
        hero[1] + angle.sin() * radius,
    ];
    let delta = [target[0] - pos[0], target[1] - pos[1]];
    let length = delta[0].hypot(delta[1]);
    let fraction = if length > 14.0 { 14.0 / length } else { 1.0 };
    [pos[0] + delta[0] * fraction, pos[1] + delta[1] * fraction]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_gates_and_age_weights_use_seconds() {
        assert_eq!(weight(71.99, 72.0, 1.0), 0.0);
        assert_eq!(weight(72.0, 72.0, 1.0), 1.0);
        assert_eq!(weight(192.0, 72.0, 1.0), 2.0);
        assert_eq!(weight(2000.0, 72.0, 2.0), 2.0);
        // At 9:00 the opening kobold remains eligible, unlike a four-newest pool.
        assert_eq!(weight(540.0, 0.0, COSTS[0]), 4.0);
        assert_eq!(weight(540.0, 540.0, COSTS[8]), 1.0 / 3.0);
    }

    #[test]
    fn weighted_draw_never_selects_locked_or_empty_buckets() {
        assert_eq!(choose(&[], 0.0), None);
        assert_eq!(choose(&[0.0, 0.0], 0.5), None);
        let weights = [0.0, 1.0, 0.0, 3.0];
        assert_eq!(choose(&weights, 0.0), Some(1));
        assert_eq!(choose(&weights, 0.249), Some(1));
        assert_eq!(choose(&weights, 0.25), Some(3));
        assert_eq!(choose(&weights, 1.0), Some(3));
    }

    #[test]
    fn melee_approach_respects_body_size_without_expanding_combat_reach() {
        assert!((melee_reach(0.7) - 0.84).abs() < 0.0001);
        assert_eq!(approach_radius(0.7), 1.5);
        assert!((approach_radius(3.5) - 3.36).abs() < 0.0001);
    }

    #[test]
    fn approach_keeps_opposite_sides_and_limits_each_move() {
        let east = approach_goal([30.0, 0.0], [0.0, 0.0], 0.0, 1.5);
        let west = approach_goal([-30.0, 0.0], [0.0, 0.0], std::f32::consts::PI, 1.5);
        assert_eq!(east, [16.0, 0.0]);
        assert!((west[0] + 16.0).abs() < 0.0001);
        let close = approach_goal([3.0, 0.0], [0.0, 0.0], 0.0, 1.5);
        assert_eq!(close, [1.5, 0.0]);
        assert_eq!(repath_after(0.0, 5.0), 0.25);
        assert_eq!(repath_after(14.0, 5.0), 0.7);
        assert_eq!(repath_after(14.0, 0.0), 0.7);
    }
}
