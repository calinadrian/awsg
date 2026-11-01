//! Original ASR's rarity arithmetic. Loot weights are a local policy; the extracted reference
//! establishes the stat amounts, not the probability of rolling each quality.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(usize)]
pub(crate) enum Rarity {
    #[default]
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}
impl Rarity {
    #[cfg(test)]
    pub const ALL: [Self; 5] = [
        Self::Common,
        Self::Uncommon,
        Self::Rare,
        Self::Epic,
        Self::Legendary,
    ];
    pub fn label(self) -> &'static str {
        ["Common", "Uncommon", "Rare", "Epic", "Legendary"][self as usize]
    }
    pub fn roll(roll: f32, luck: f32) -> Self {
        // Explicit rebuild policy until the original roll weights have been established.
        let r = (roll / (1.0 + luck.max(0.0))).clamp(0.0, 1.0);
        if r < 0.01 {
            Self::Legendary
        } else if r < 0.06 {
            Self::Epic
        } else if r < 0.20 {
            Self::Rare
        } else if r < 0.48 {
            Self::Uncommon
        } else {
            Self::Common
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(crate) enum Upgrade {
    Power,
    Cooldown,
    Targets,
    Area,
    Duration,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct SpellProgress {
    pub power: f32,
    pub cooldown: f32,
    pub targets: u32,
    pub area: f32,
    pub duration: f32,
    pub ranks: [u32; 5],
}
impl Default for SpellProgress {
    fn default() -> Self {
        Self {
            power: 1.,
            cooldown: 1.,
            targets: 0,
            area: 1.,
            duration: 1.,
            ranks: [0; 5],
        }
    }
}
pub(super) const POWER: [f32; 5] = [0.18, 0.25, 0.34, 0.46, 0.65];
const CD: [f32; 5] = [0.07, 0.10, 0.13, 0.17, 0.23];
const AREA: [f32; 5] = [0.12, 0.18, 0.25, 0.35, 0.50];
const DURATION: [f32; 5] = [0.15, 0.22, 0.30, 0.40, 0.60];
impl SpellProgress {
    pub fn learned(rarity: Rarity, powered: bool) -> Self {
        let mut s = Self::default();
        let r = rarity as usize;
        if r > 0 {
            if powered {
                s.power *= 1. + POWER[r - 1]
            } else {
                s.cooldown *= 1. - CD[r - 1]
            }
        }
        s
    }
    pub fn apply(&mut self, stat: Upgrade, rarity: Rarity, rank: u32) {
        let r = rarity as usize;
        match stat {
            Upgrade::Power => self.power *= 1. + POWER[r] / (1. + 0.5 * self.ranks[0] as f32),
            Upgrade::Cooldown => {
                self.cooldown =
                    (self.cooldown - CD[r] * ((self.cooldown - 0.2) / 0.8).max(0.)).max(0.2)
            }
            Upgrade::Targets => self.targets = (self.targets + if r >= 3 { 2 } else { 1 }).min(12),
            Upgrade::Area => {
                self.area = (self.area + AREA[r] * ((5. - self.area) / 4.).max(0.)).min(5.)
            }
            Upgrade::Duration => {
                self.duration =
                    (self.duration + DURATION[r] * ((6. - self.duration) / 5.).max(0.)).min(6.)
            }
        }
        self.ranks[stat as usize] += 1;
        self.power *= 1. + 0.04 / (1. + 0.125 * rank.saturating_sub(2) as f32);
    }
}
pub(super) const JEWEL_SCALE: [f32; 5] = [1., 1.4, 1.8, 2.4, 3.2];
/// Return the added strength, with the original stat-specific caps and soft health/armor curve.
pub(super) fn jewel_gain(tag: u8, base: f32, rarity: Rarity, current: Option<(f32, u32)>) -> f32 {
    if tag == 16 {
        return (if rarity as usize >= 3 { 2.0_f32 } else { 1. })
            .min(2. - current.map_or(0., |v| v.0))
            .max(0.);
    }
    let (old, picks) = current.unwrap_or((0., 0));
    let raw = base * JEWEL_SCALE[rarity as usize] * if current.is_none() { 1.5 } else { 1. };
    if matches!(tag, 2 | 5 | 6) {
        let threshold = 3. * base;
        let step = raw / 16.;
        let mut v = old;
        for _ in 0..16 {
            v += if v <= threshold {
                step * (1. - 0.75 * v / threshold)
            } else {
                step * 0.25 * (threshold / v).powi(2)
            };
        }
        return v - old;
    }
    if matches!(tag, 0 | 1 | 3 | 4 | 8 | 9) {
        let cap = if tag == 9 { 0.6 } else { f32::INFINITY };
        return (raw / (1. + 0.08 * picks as f32)).min((cap - old).max(0.));
    }
    let cap = base
        * match tag {
            7 | 15 | 18 => 3.,
            19 => 4.,
            _ => 5.,
        };
    (raw * ((cap - old) / cap).clamp(0., 1.)).min((cap - old).max(0.))
}
/// Consume only the displayed reward; chests take priority over banked level-ups.
pub(super) fn consume_pending(chest: bool, levels: &mut u32, chests: &mut u32) -> Option<bool> {
    if chest {
        *chests = chests.saturating_sub(1);
    } else {
        *levels = levels.saturating_sub(1);
    }
    if *chests > 0 {
        Some(true)
    } else if *levels > 0 {
        Some(false)
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f32, b: f32) {
        assert!((a - b).abs() < 0.00001, "{a} != {b}");
    }
    #[test]
    fn chest_and_level_rewards_are_banked_independently() {
        let (mut levels, mut chests) = (2, 2);
        assert_eq!(consume_pending(true, &mut levels, &mut chests), Some(true));
        assert_eq!((levels, chests), (2, 1));
        assert_eq!(consume_pending(true, &mut levels, &mut chests), Some(false));
        assert_eq!((levels, chests), (2, 0));
        assert_eq!(
            consume_pending(false, &mut levels, &mut chests),
            Some(false)
        );
        assert_eq!(consume_pending(false, &mut levels, &mut chests), None);
        assert_eq!((levels, chests), (0, 0));
    }
    #[test]
    fn upgrade_and_new_spell_qualities_are_distinct() {
        for (i, r) in Rarity::ALL.into_iter().enumerate() {
            let mut p = SpellProgress::default();
            p.apply(Upgrade::Power, r, 2);
            near(p.power, (1. + POWER[i]) * 1.04);
            let new = SpellProgress::learned(r, true);
            near(new.power, if i == 0 { 1. } else { 1. + POWER[i - 1] });
            let mut c = SpellProgress::default();
            c.apply(Upgrade::Cooldown, r, 2);
            near(c.cooldown, 1. - CD[i]);
        }
    }
    #[test]
    fn repeated_upgrades_obey_soft_caps() {
        let mut s = SpellProgress::default();
        for n in 2..200 {
            for u in [
                Upgrade::Cooldown,
                Upgrade::Area,
                Upgrade::Duration,
                Upgrade::Targets,
            ] {
                s.apply(u, Rarity::Legendary, n);
            }
        }
        assert!((0.2..0.201).contains(&s.cooldown));
        assert!(s.area <= 5. && s.area > 4.99);
        assert!(s.duration <= 6. && s.duration > 5.99);
        assert_eq!(s.targets, 12);
    }
    #[test]
    fn chest_reference_values_and_repeated_picks() {
        near(jewel_gain(0, 0.17, Rarity::Common, None), 0.255);
        near(jewel_gain(1, 0.09, Rarity::Legendary, None), 0.432);
        near(jewel_gain(2, 0.20, Rarity::Rare, None), 0.3985994);
        near(jewel_gain(7, 1.5, Rarity::Legendary, None), 4.5);
        near(jewel_gain(18, 0.01, Rarity::Legendary, None), 0.03);
        near(
            jewel_gain(0, 0.17, Rarity::Common, Some((0.255, 1))),
            0.17 / 1.08,
        );
        near(jewel_gain(7, 1.5, Rarity::Legendary, Some((4.5, 1))), 0.);
        near(jewel_gain(16, 1., Rarity::Epic, None), 2.);
        near(jewel_gain(16, 1., Rarity::Legendary, Some((2., 1))), 0.);
    }
}
