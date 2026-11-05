//! Permanent boss protection, applied at damage/CC entry points on both peers and authority.
pub(super) const NAME: &str = "Almighty Strength";
pub(super) const ICON: u32 = 642;
pub(super) const DESCRIPTION: &str = "Damage from a skill hit or damage-over-time tick cannot exceed 10% of this boss's maximum health. Immune to slows, stuns and immobilizing effects.";

pub(super) fn damage(boss: bool, maximum: f32, amount: f32) -> f32 {
    let amount = amount.max(0.0);
    if boss { amount.min(maximum.max(0.0) * 0.1) } else { amount }
}
