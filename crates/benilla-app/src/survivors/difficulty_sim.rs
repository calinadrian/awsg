//! The difficulty curve against the real item catalog: what a full set of each tier's gear does
//! for a hero, held against how much tougher that tier's enemies are than Elwynn's.
//!
//! Coverage is the hero's gain over Elwynn divided by the enemies' gain over Elwynn. Offence is
//! the gear damage multiplier with talents worth ×1.08 a tier, against enemy health; defence is
//! effective health (health over damage taken) against enemy damage. 1.0 plays like Elwynn
//! ungeared.

use asr_coop::economy::{Catalog, Definition};

use super::difficulty::factors;

const CLASSES: [u8; 9] = [1, 2, 3, 4, 5, 7, 8, 9, 11];
/// A mid-run hero: about level 25, and the classes' middling reduction.
const HERO_HP: f32 = 450.0;
const HERO_REDUCTION: f32 = 0.15;

/// A full set a class can gather from one tier's zones at an upgrade level: damage multiplier
/// bonus, flat health and reduction, averaged over the tier's zones.
#[derive(Clone, Copy, Default, Debug)]
struct Set {
    damage: f32,
    health: f32,
    reduction: f32,
}

fn zone_set(catalog: &Catalog, items: &[&Definition], class: u8, upgrade: u8) -> Set {
    let usable: Vec<_> = items.iter().filter(|d| d.can_use(class)).collect();
    let stat = |key: &str| {
        let best = |filter: &dyn Fn(&Definition) -> bool| {
            usable
                .iter()
                .filter(|d| filter(d))
                .filter_map(|d| d.bonuses(upgrade).get(key).copied())
                .max()
                .unwrap_or(0) as f32
        };
        let mut total = 0.0;
        for slot in ["head", "shoulders", "chest", "wrists", "hands", "belt", "legs", "feet", "back", "neck", "shirt"] {
            total += best(&|d| d.slot == slot);
        }
        total += 2.0 * best(&|d| d.slot == "finger_1") + 2.0 * best(&|d| d.slot == "trinket_1");
        let one_hand = best(&|d| d.slot == "main_hand" && !d.two_handed) + best(&|d| d.slot == "off_hand");
        let two_hand = best(&|d| d.slot == "main_hand" && d.two_handed);
        total + one_hand.max(two_hand) + best(&|d| d.slot == "ranged")
    };
    let _ = catalog;
    Set {
        damage: stat("damage_bp") / 10_000.0,
        health: stat("max_health"),
        reduction: stat("damage_reduction_bp") / 10_000.0,
    }
}

/// The class-average full set of a tier at an upgrade level.
fn tier_set(catalog: &Catalog, tier: u8, upgrade: u8) -> Set {
    let mut sum = Set::default();
    let mut count = 0.0;
    for zone in catalog.zones.values().filter(|z| z.tier == tier) {
        let items: Vec<_> = zone
            .equipment
            .iter()
            .filter_map(|drop| catalog.definition(&drop.item))
            .collect();
        for class in CLASSES {
            let set = zone_set(catalog, &items, class, upgrade);
            sum.damage += set.damage;
            sum.health += set.health;
            sum.reduction += set.reduction;
            count += 1.0;
        }
    }
    Set {
        damage: sum.damage / count,
        health: sum.health / count,
        reduction: sum.reduction / count,
    }
}

/// (offence, defence) coverage at `tier` wearing `set`, against the horde.
fn coverage(tier: u8, set: Set) -> (f32, f32) {
    let f = factors(tier);
    let talents = 1.08f32.powi(i32::from(tier) - 1);
    let offence = (1.0 + set.damage) * talents / f.normal_hp;
    let ehp = |hp: f32, reduction: f32| hp / (1.0 - reduction.min(0.6));
    let defence = ehp(HERO_HP + set.health, HERO_REDUCTION + set.reduction)
        / ehp(HERO_HP, HERO_REDUCTION)
        / f.normal_damage;
    (offence, defence)
}

#[test]
fn gear_keeps_pace_with_each_tier() {
    let catalog = Catalog::load();
    for tier in 1..=5u8 {
        let (fresh, _) = coverage(tier, tier_set(&catalog, tier, 0));
        let (upgraded, _) = coverage(tier, tier_set(&catalog, tier, 5));
        // A tier's own gear makes it a fair fight; upgraded, it is comfortable.
        assert!((0.88..=1.15).contains(&fresh), "tier {tier} on-tier +0 offence {fresh:.3}");
        assert!((1.08..=1.35).contains(&upgraded), "tier {tier} on-tier +5 offence {upgraded:.3}");
        if tier > 1 {
            // The last tier's best gear gets a hero in, but it is a push.
            let (push, _) = coverage(tier, tier_set(&catalog, tier - 1, 5));
            assert!((0.78..=0.98).contains(&push), "tier {tier} with tier {} +5: {push:.3}", tier - 1);
            assert!(push < fresh && fresh < upgraded);
        }
    }
}

#[test]
fn defence_keeps_pace_and_ungeared_heroes_fall_behind() {
    let catalog = Catalog::load();
    for tier in 1..=5u8 {
        let (_, fresh) = coverage(tier, tier_set(&catalog, tier, 0));
        assert!((0.85..=1.3).contains(&fresh), "tier {tier} on-tier +0 defence {fresh:.3}");
        if tier > 1 {
            // At equal gear every tier is harder than the one before it.
            let bare = Set::default();
            assert!(coverage(tier, bare).0 < coverage(tier - 1, bare).0);
            assert!(coverage(tier, bare).1 < coverage(tier - 1, bare).1);
        }
    }
}

#[test]
fn elites_and_bosses_stay_the_hard_part() {
    for tier in 2..=5u8 {
        let f = factors(tier);
        // Against the same gear, an elite's share of the fight grows and a boss's grows more.
        assert!(f.elite_hp / f.normal_hp > 1.05, "tier {tier}");
        assert!(f.boss_hp / f.elite_hp > 1.0, "tier {tier}");
    }
}

/// `cargo test -p benilla-app difficulty_report -- --ignored --nocapture` prints the power curve.
#[test]
#[ignore]
fn difficulty_report() {
    let catalog = Catalog::load();
    println!("tier,enemy_hp,enemy_damage,gear_damage_0,gear_damage_5,health_0,health_5,reduction_0,reduction_5,offence_prev5,offence_0,offence_5,defence_0,defence_5,elite_offence_5,boss_offence_5");
    for tier in 1..=5u8 {
        let f = factors(tier);
        let (fresh, top) = (tier_set(&catalog, tier, 0), tier_set(&catalog, tier, 5));
        let prev = if tier > 1 { coverage(tier, tier_set(&catalog, tier - 1, 5)).0 } else { 1.0 };
        let (o0, d0) = coverage(tier, fresh);
        let (o5, d5) = coverage(tier, top);
        println!(
            "{tier},{:.2},{:.2},{:.3},{:.3},{:.0},{:.0},{:.3},{:.3},{prev:.3},{o0:.3},{o5:.3},{d0:.3},{d5:.3},{:.3},{:.3}",
            f.normal_hp, f.normal_damage, fresh.damage, top.damage, fresh.health, top.health,
            fresh.reduction, top.reduction, o5 * f.normal_hp / f.elite_hp, o5 * f.normal_hp / f.boss_hp,
        );
    }
}
