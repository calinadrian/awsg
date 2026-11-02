//! How hard each battleground is: its tier along the roads. Elwynn Forest is tier 1 and keeps the
//! native numbers; every zone further down a road sits a tier higher.

/// Where a battleground sits on the roads, 1 (a starting zone) to 5 (where the roads meet); `None`
/// for a place no road reaches yet.
pub(super) fn tier(zone: &str) -> Option<u8> {
    Some(match zone {
        "Elwynn Forest" | "Dun Morogh" | "Teldrassil" | "Durotar" | "Mulgore"
        | "Tirisfal Glades" => 1,
        "Westfall" | "The Barrens" | "Hall of Thanes" => 2,
        "The Deadmines" | "Wailing Caverns" => 3,
        "Duskwood" | "Stonetalon Mountains" => 4,
        "Stranglethorn Vale" => 5,
        _ => return None,
    })
}

/// The zones that open a battleground: clearing any one of them does. Starting zones need none.
pub(super) fn requires(zone: &str) -> &'static [&'static str] {
    match zone {
        "Westfall" => &["Elwynn Forest", "Dun Morogh", "Teldrassil"],
        "The Deadmines" => &["Westfall"],
        "Duskwood" => &["The Deadmines"],
        "The Barrens" => &["Durotar", "Mulgore", "Tirisfal Glades"],
        "Wailing Caverns" => &["The Barrens"],
        "Stonetalon Mountains" => &["Wailing Caverns"],
        "Stranglethorn Vale" => &["Duskwood", "Stonetalon Mountains"],
        "Hall of Thanes" => &[
            "Elwynn Forest", "Dun Morogh", "Teldrassil", "Durotar", "Mulgore", "Tirisfal Glades",
        ],
        _ => &[],
    }
}

/// Whether the account has cleared the way to `zone`.
pub(super) fn unlocked(zone: &str, cleared: &std::collections::BTreeSet<String>) -> bool {
    let needs = requires(zone);
    needs.is_empty() || needs.iter().any(|z| cleared.contains(*z))
}

/// The menus' reason a zone is shut: "Clear Westfall", "Clear Duskwood or Stonetalon Mountains".
pub(super) fn unlock_hint(zone: &str) -> String {
    match requires(zone) {
        [] => String::new(),
        [one] => format!("Clear {one}"),
        needs if needs.len() > 3 => "Clear any starting zone".into(),
        [rest @ .., last] => format!("Clear {} or {last}", rest.join(", ")),
    }
}

/// How much harder a tier's enemies are than Elwynn's at the same minute of the run. Elites and
/// bosses grow faster than the horde, so they stay the hard part of every road.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Factors {
    pub normal_hp: f32,
    pub normal_damage: f32,
    pub elite_hp: f32,
    pub elite_damage: f32,
    pub boss_hp: f32,
    pub boss_damage: f32,
    /// The spawner's budget and population floor.
    pub density: f32,
    /// The wait between roaming elites.
    pub elite_every: f32,
}

/// Tier 1 is Elwynn's own numbers; each tier past it multiplies the horde's health by 1.4 and
/// its damage by the square root of that, as the native zone multiplier does.
pub(super) fn factors(tier: u8) -> Factors {
    const ELITE: [(f32, f32); 5] = [(1.0, 1.0), (1.55, 1.25), (2.35, 1.55), (3.5, 1.9), (5.2, 2.35)];
    const BOSS: [(f32, f32); 5] = [(1.0, 1.0), (1.6, 1.25), (2.5, 1.55), (3.85, 1.95), (5.75, 2.45)];
    const DENSITY: [f32; 5] = [1.0, 1.06, 1.12, 1.18, 1.25];
    const ELITE_EVERY: [f32; 5] = [1.0, 0.9, 0.85, 0.8, 0.72];
    let i = usize::from(tier.clamp(1, 5) - 1);
    let zone = 1.4f32.powi(i as i32);
    Factors {
        normal_hp: zone,
        normal_damage: zone.sqrt(),
        elite_hp: ELITE[i].0,
        elite_damage: ELITE[i].1,
        boss_hp: BOSS[i].0,
        boss_damage: BOSS[i].1,
        density: DENSITY[i],
        elite_every: ELITE_EVERY[i],
    }
}

/// A small hall holds fewer enemies at once than open ground.
pub(super) fn crowd(leash: f32) -> f32 {
    if leash < 60.0 { 0.6 } else { 1.0 }
}

/// A tier as the menus print it.
pub(super) fn numeral(tier: u8) -> &'static str {
    ["I", "II", "III", "IV", "V"]
        .get(usize::from(tier.max(1)) - 1)
        .copied()
        .unwrap_or("V")
}

#[cfg(test)]
mod tests {
    use super::tier;

    #[test]
    fn every_road_zone_has_a_tier() {
        for name in [
            "Elwynn Forest", "Westfall", "The Deadmines", "Duskwood", "Durotar", "The Barrens",
            "Wailing Caverns", "Stonetalon Mountains", "Stranglethorn Vale",
        ] {
            assert!(super::super::data::MAPS.iter().any(|m| m.name == name), "{name}");
            assert!(tier(name).is_some(), "{name}");
        }
        assert!(tier("Elwynn Forest") < tier("Westfall"));
        assert!(tier("Duskwood") < tier("Stranglethorn Vale"));
    }

    #[test]
    fn each_road_opens_in_order() {
        let cleared = |zones: &[&str]| zones.iter().map(|z| z.to_string()).collect();
        let none = cleared(&[]);
        for start in ["Elwynn Forest", "Durotar", "Teldrassil", "Mulgore", "Tirisfal Glades", "Dun Morogh"] {
            assert!(super::unlocked(start, &none), "{start}");
        }
        assert!(!super::unlocked("Westfall", &none));
        assert!(super::unlocked("Westfall", &cleared(&["Dun Morogh"])));
        assert!(!super::unlocked("Duskwood", &cleared(&["Elwynn Forest", "Westfall"])));
        assert!(super::unlocked("Stranglethorn Vale", &cleared(&["Stonetalon Mountains"])));
        // Every prerequisite is a real zone of a lower tier.
        for map in super::super::data::MAPS.iter() {
            for need in super::requires(map.name) {
                assert!(tier(need) < tier(map.name), "{} needs {need}", map.name);
            }
        }
        assert_eq!(super::unlock_hint("Westfall"), "Clear Elwynn Forest, Dun Morogh or Teldrassil");
        assert_eq!(super::unlock_hint("Duskwood"), "Clear The Deadmines");
        assert_eq!(super::unlock_hint("Hall of Thanes"), "Clear any starting zone");
    }

    #[test]
    fn tier_one_is_elwynn_and_every_tier_is_harder() {
        let one = super::factors(1);
        assert_eq!(
            [one.normal_hp, one.normal_damage, one.elite_hp, one.elite_damage, one.boss_hp, one.boss_damage, one.density, one.elite_every],
            [1.0; 8]
        );
        for tier in 2..=5 {
            let (lower, f) = (super::factors(tier - 1), super::factors(tier));
            assert!(f.normal_hp > lower.normal_hp && f.normal_damage > lower.normal_damage);
            assert!(f.elite_hp > lower.elite_hp && f.boss_hp > lower.boss_hp);
            // Elites and bosses outgrow the horde, bosses the elites.
            assert!(f.elite_hp > f.normal_hp && f.boss_hp > f.elite_hp, "tier {tier}");
            assert!(f.density > lower.density && f.elite_every < lower.elite_every);
        }
    }
}
