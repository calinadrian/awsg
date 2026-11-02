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
}
