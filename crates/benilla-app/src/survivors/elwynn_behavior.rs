//! Elwynn rules read from the installed ASR 0.124 executable.
//!
//! Selection: RVAs 0x5c7ec90 and 0x5c8a2e0.

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

/// Roaming elite rows use 4.8x health and the 0.00594 cubic; damage has no regular 1.25x.
/// Native 0x5c0062d and leaf 0x5c6a2e0; lair guardians retain their separate curves.
pub(super) fn roaming_elite_stats(hp: f32, damage: f32, speed: f32, elapsed: f32) -> (f32, f32, f32) {
    let minute = elapsed.max(0.0) / 60.0;
    let health_curve =
        1.0 + 0.10 * minute + 0.036 * minute * minute + 0.00594 * minute * minute * minute;
    let damage_curve =
        1.0 + 0.07 * minute + 0.015 * minute * minute + 0.0002 * minute * minute * minute;
    (
        hp * 4.8 * 0.82 * health_curve,
        damage * 0.82_f32.sqrt() * damage_curve,
        speed * 1.05,
    )
}

/// Boss rows: 5.75x health and flat damage, with no run-time curve (native 0x5c134da and
/// 0x5c1378f, normal mode, single hero). Speed and swing are the row's own.
pub(super) fn boss_stats(hp: f32, damage: f32) -> (f32, f32) {
    (hp * 5.75 * 0.82, damage * 0.82_f32.sqrt())
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
    fn boss_rows_use_the_native_health_factor_and_flat_damage() {
        let (hp, damage) = boss_stats(3600.0, 22.0);
        assert!((hp - 16974.0).abs() < 1.0);
        assert!((damage - 19.92).abs() < 0.01);
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
}
