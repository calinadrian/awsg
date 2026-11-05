//! Fewer ordinary enemies for solo play, with rewards normalized to the spawn rate.
use super::{Rng, Run};

pub(super) const DENSITY: f32 = 0.6;
// 0.6 * 1.5 = 0.9: slightly less aggregate HP, rather than identical enemies becoming sponges.
pub(super) const HEALTH: f32 = 1.5;
pub(super) const REWARD: f32 = 1.0 / DENSITY;

pub(super) fn enabled(run: &Run) -> bool {
    run.low_memory && !run.coop_run && !run.test
}

pub(super) fn density(run: &Run) -> f32 {
    if enabled(run) { DENSITY } else { 1.0 }
}

/// Unbiased rounding preserves the bonus on small gold piles.
pub(super) fn reward(base: u32, rng: &mut Rng) -> u32 {
    let scaled = base as f32 * REWARD;
    (scaled as u32).saturating_add(u32::from(rng.f32() < scaled.fract()))
}
