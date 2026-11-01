//! Original ASR gold: death roll RVA 0x5c03dc9..0x5c03ed7; collection RVA
//! 0x5c11ff8..0x5c12016 and 0x5bfe27b..0x5bfe2c3 (installed 0.124.5).
//! Ordinary enemies: 16.5%, uniform 2..5 exclusive. Boss branch: always 15.
//! Gold-find scales the pile at collection, rounded to the nearest whole gold.
//! Co-op divides it by the living-party share (0x5bfe22f..0x5bfe28d).
pub(super) fn drop_amount(boss: bool, chance: f32, amount_roll: f32) -> Option<u32> {
    if boss {
        Some(15)
    } else if chance < 0.165 {
        Some(2 + (amount_roll * 3.0).floor().clamp(0.0, 2.0) as u32)
    } else {
        None
    }
}
pub(super) fn collected(base: u32, bonus: f32, party: f32) -> u32 {
    (base as f32 * (1.0 + bonus) / party).round().max(0.0) as u32
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_drop_boundaries_and_amounts() {
        assert_eq!(drop_amount(false, 0.165, 0.0), None);
        assert_eq!(drop_amount(false, 0.164999, 0.0), Some(2));
        assert_eq!(drop_amount(false, 0.0, 0.5), Some(3));
        assert_eq!(drop_amount(false, 0.0, 0.999999), Some(4));
        assert_eq!(drop_amount(true, 0.99, 0.99), Some(15));
    }
    #[test]
    fn gold_find_is_rounded_at_collection() {
        assert_eq!(collected(3, 0.0, 1.0), 3);
        assert_eq!(collected(4, 0.375, 1.0), 6);
        assert_eq!(collected(15, 1.2, 1.0), 33);
        // Two living players share 1.336.
        assert_eq!(collected(15, 0.0, 1.336), 11);
    }
}
