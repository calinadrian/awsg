//! Persistent equipment and a shared gold marketplace, using ASR's owned catalog.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub type Amounts = BTreeMap<String, u32>;
/// Elite equipment is scarce; named bosses have their own separate drop assignments.
pub const MAX_EQUIPMENT_PER_RUN: u32 = 3;

#[derive(Clone, Deserialize)]
pub struct Catalog {
    pub items: Vec<Definition>,
    pub zones: BTreeMap<String, Zone>,
    #[serde(default)]
    pub sets: BTreeMap<String, SetBonus>,
    /// Quests and achievements, from their own catalog file.
    #[serde(skip)]
    pub quests: crate::quests::QuestCatalog,
}
#[derive(Clone, Deserialize)]
pub struct SetBonus {
    pub pieces: u8,
    pub spell_damage_bp: Amounts,
}
#[derive(Clone, Deserialize)]
pub struct Definition {
    pub id: String,
    pub name: String,
    pub zone: String,
    pub slot: String,
    pub category: String,
    #[serde(default = "common_quality")]
    pub quality: String,
    #[serde(default)]
    pub armor: Option<String>,
    pub icon: String,
    pub display_id: u32,
    pub source_item_id: u32,
    pub classes: Vec<u8>,
    pub two_handed: bool,
    /// Per-spell damage bonuses in basis points; 1000 is +10% to the named spell.
    #[serde(default)]
    pub spell_damage_bp: Amounts,
    #[serde(default)]
    pub set_name: Option<String>,
    pub base_bonuses: Amounts,
    pub upgrades: Vec<Upgrade>,
    pub scrap: Amounts,
}
fn common_quality() -> String {
    "common".into()
}
fn first_tier() -> u8 {
    1
}
/// Item qualities, worst first.
pub const QUALITIES: [&str; 4] = ["common", "uncommon", "rare", "epic"];
pub fn quality_rank(quality: &str) -> u32 {
    QUALITIES.iter().position(|q| *q == quality).unwrap_or(0) as u32
}
/// A tier's item level for common gear at +0.
const TIER_LEVEL: [u32; 5] = [10, 20, 26, 32, 42];
/// A full set's slot weights summed, about fifteen for most classes.
const SET_WEIGHT: f32 = 15.0;
/// The gear score a zone is meant for: nine tenths of a full set of its own floor-quality gear.
pub fn recommended_score(tier: u8) -> u32 {
    let floor = [0, 1, 2, 2, 2][usize::from(tier.clamp(1, 5) - 1)];
    (0.9 * item_level(tier, floor, 0) as f32 * SET_WEIGHT).round() as u32
}
fn item_level(tier: u8, quality: u32, upgrade: u8) -> u32 {
    TIER_LEVEL[usize::from(tier.clamp(1, 5) - 1)] + 3 * quality + 2 * u32::from(upgrade)
}
#[derive(Clone, Deserialize)]
pub struct Upgrade {
    pub cost: Amounts,
    pub bonuses: Amounts,
}
#[derive(Clone, Deserialize)]
pub struct Zone {
    pub name: String,
    /// The zone's tier along the roads, 1 (a starting zone) to 5; its gear's power follows it.
    #[serde(default = "first_tier")]
    pub tier: u8,
    pub equipment: Vec<Drop>,
    #[serde(default)]
    pub boss_drops: Vec<BossDrop>,
    /// Encounter order, independent of gear assignments. Older tables use `boss_drops` order.
    #[serde(default)]
    pub bosses: Vec<String>,
    pub equipment_chance_bp: Amounts,
    pub material_chance_bp: Amounts,
    pub materials: Amounts,
    pub material_pool: Vec<String>,
}
impl Zone {
    pub fn boss_names(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.bosses.iter().map(String::as_str).chain(
            self.boss_drops.iter().filter(|_| self.bosses.is_empty()).map(|entry| entry.boss.as_str())
        )
    }
}
#[derive(Clone, Deserialize)]
pub struct BossDrop {
    pub boss: String,
    pub item: String,
}
#[derive(Clone, Deserialize)]
pub struct Drop {
    pub item: String,
    pub weight: u32,
}
impl Catalog {
    /// A zone's tier, 1 for a zone it does not know.
    pub fn tier(&self, zone: &str) -> u8 {
        self.zones.get(zone).map_or(1, |z| z.tier)
    }
    pub fn load() -> Self {
        let mut catalog: Self = serde_json::from_str(include_str!("../assets/economy_catalog.json"))
            .expect("bundled ASR item catalog");
        catalog.quests = crate::quests::QuestCatalog::load();
        catalog
    }
    pub fn definition(&self, id: &str) -> Option<&Definition> {
        self.items.iter().find(|d| d.id == id)
    }
}
impl Definition {
    pub fn can_use(&self, class: u8) -> bool {
        if !self.classes.contains(&class) {
            return false;
        }
        if matches!(class, 1 | 2 | 7) {
            if self.slot == "ranged" {
                return false;
            }
            if self.slot == "main_hand" && self.two_handed {
                return false;
            }
            if self.slot == "off_hand" && !self.icon.to_ascii_lowercase().contains("shield") {
                return false;
            }
        }
        if !matches!(
            self.slot.as_str(),
            "head" | "shoulders" | "chest" | "wrists" | "hands" | "belt" | "legs" | "feet"
        ) {
            return true;
        }
        let armor = match class {
            1 | 2 => "plate",
            3 | 7 => "mail",
            4 | 11 => "leather",
            5 | 8 | 9 => "cloth",
            _ => return false,
        };
        self.armor.as_deref().is_none_or(|kind| {
            kind == armor || (matches!(class, 1 | 2) && kind == "mail")
        })
    }
    /// Item level: its zone's tier, then its quality and upgrades.
    pub fn item_level(&self, catalog: &Catalog, upgrade: u8) -> u32 {
        item_level(catalog.tier(&self.zone), quality_rank(&self.quality), upgrade)
    }
    /// Its share of a hero's gear score: item level by how much of a set its slot carries.
    pub fn score(&self, catalog: &Catalog, upgrade: u8) -> u32 {
        let weight = match self.slot.as_str() {
            "main_hand" if self.two_handed => 2.2,
            "main_hand" => 1.6,
            "head" | "chest" | "legs" => 1.2,
            "shoulders" | "hands" | "feet" | "belt" => 0.9,
            "off_hand" | "ranged" => 0.8,
            "shirt" => 0.2,
            _ => 0.7,
        };
        (self.item_level(catalog, upgrade) as f32 * weight).round() as u32
    }
    pub fn bonuses(&self, level: u8) -> &Amounts {
        if level == 0 {
            &self.base_bonuses
        } else {
            self.upgrades
                .get(usize::from(level - 1))
                .map_or(&self.base_bonuses, |u| &u.bonuses)
        }
    }
    pub fn fits(&self, class: u8, slot: &str) -> bool {
        self.can_use(class)
            && (self.slot == slot
                || (self.slot == "finger_1" && matches!(slot, "finger_1" | "finger_2"))
                || (self.slot == "trinket_1" && matches!(slot, "trinket_1" | "trinket_2"))
                || (self.slot == "main_hand"
                    && slot == "off_hand"
                    && !self.two_handed
                    && matches!(class, 1 | 4 | 7)))
    }
}
pub const SLOTS: &[&str] = &[
    "head",
    "neck",
    "shoulders",
    "back",
    "chest",
    "shirt",
    "tabard",
    "wrists",
    "hands",
    "belt",
    "legs",
    "feet",
    "finger_1",
    "finger_2",
    "trinket_1",
    "trinket_2",
    "main_hand",
    "off_hand",
    "ranged",
];

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub definition: String,
    pub level: u8,
    pub slot: Option<String>,
    #[serde(default)]
    pub equipped_class: Option<u8>,
    pub locked: bool,
    pub listed: bool,
}
impl Item {
    pub fn equipped_for(&self, class: u8) -> bool {
        self.slot.is_some() && self.equipped_class.is_none_or(|owner| owner == class)
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    #[serde(default)]
    pub achievement_points: u32,
    #[serde(default)]
    pub cleared_zones: std::collections::BTreeSet<String>,
    pub name: String,
    pub class: u8,
    pub gold: u64,
    pub materials: Amounts,
    pub items: Vec<Item>,
    /// The quest log: accepted quests and their progress.
    #[serde(default)]
    pub quests: Vec<crate::quests::QuestState>,
    /// Quests turned in, and how many times.
    #[serde(default)]
    pub quests_done: BTreeMap<String, u32>,
    /// Achievements earned, and when (seconds since the Unix epoch).
    #[serde(default)]
    pub achievements: BTreeMap<String, u64>,
    /// Counts toward achievements not yet earned, one per criterion.
    #[serde(default)]
    pub achievement_progress: BTreeMap<String, Vec<u32>>,
    /// Every achievement point ever earned; `achievement_points` is what is left to spend.
    #[serde(default)]
    pub achievement_total: u32,
    /// The quest catalog revision this hero was last brought forward to.
    #[serde(default)]
    pub quest_rev: u32,
}
impl Inventory {
    /// The summed score of what this hero wears as `class`.
    pub fn gear_score(&self, catalog: &Catalog, class: u8) -> u32 {
        self.items
            .iter()
            .filter(|i| i.equipped_for(class))
            .filter_map(|i| {
                catalog
                    .definition(&i.definition)
                    .filter(|d| d.can_use(class))
                    .map(|d| d.score(catalog, i.level))
            })
            .sum()
    }
    pub fn bonuses(&self, catalog: &Catalog) -> Amounts {
        let mut result = Amounts::new();
        for item in self.items.iter().filter(|i| i.equipped_for(self.class)) {
            if let Some(d) = catalog
                .definition(&item.definition)
                .filter(|d| d.can_use(self.class))
            {
                for (key, value) in d.bonuses(item.level) {
                    let entry = result.entry(key.clone()).or_default();
                    *entry = entry.saturating_add(*value);
                }
            }
        }
        result
    }

    pub fn spell_damage_bonuses(&self, catalog: &Catalog) -> Amounts {
        let mut result = Amounts::new();
        let mut equipped_sets = BTreeMap::<String, u8>::new();
        for item in self.items.iter().filter(|i| i.equipped_for(self.class)) {
            if let Some(definition) = catalog
                .definition(&item.definition)
                .filter(|definition| definition.can_use(self.class))
            {
                for (spell, value) in &definition.spell_damage_bp {
                    let entry = result.entry(spell.clone()).or_default();
                    *entry = entry.saturating_add(*value);
                }
                if let Some(set_name) = &definition.set_name {
                    *equipped_sets.entry(set_name.clone()).or_default() += 1;
                }
            }
        }
        for (set_name, count) in equipped_sets {
            if let Some(set) = catalog.sets.get(&set_name).filter(|set| count >= set.pieces) {
                for (spell, value) in &set.spell_damage_bp {
                    let entry = result.entry(spell.clone()).or_default();
                    *entry = entry.saturating_add(*value);
                }
            }
        }
        result
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Listing {
    pub id: u64,
    pub seller: String,
    pub seller_name: String,
    pub item: Item,
    pub price: u64,
    pub expires: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    pub token: String,
    pub action: Action,
}
#[derive(Clone, Serialize, Deserialize)]
pub enum Action {
    RedeemAchievement { gear: bool },
    LinkInventory {
        tokens: Vec<String>,
    },
    Register {
        name: String,
        class: u8,
    },
    Refresh,
    Equip {
        item: u64,
        slot: Option<String>,
    },
    Lock {
        item: u64,
        locked: bool,
    },
    Upgrade {
        item: u64,
    },
    Scrap {
        item: u64,
    },
    List {
        item: u64,
        price: u64,
    },
    Buy {
        listing: u64,
    },
    Cancel {
        listing: u64,
    },
    BeginRun {
        zone: String,
    },
    /// Gameplay is player-hosted, as with co-op damage. Bounds and receipts prevent
    /// accidental duplicate rewards; this is not an authoritative combat simulation.
    Reward {
        run: String,
        normal: u32,
        elite: u32,
        boss: u32,
        #[serde(default)]
        boss_names: Vec<String>,
        gold: u32,
        /// Quest and achievement progress this batch; bosses and the clear the ledger counts
        /// itself.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        progress: Vec<crate::quests::Tick>,
    },
    AcceptQuest {
        quest: String,
    },
    AbandonQuest {
        quest: String,
    },
    TurnInQuest {
        quest: String,
    },
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Reply {
    pub id: String,
    pub token: Option<String>,
    pub inventory: Option<Inventory>,
    pub listings: Vec<Listing>,
    pub run: Option<String>,
    pub message: String,
    pub ok: bool,
    #[serde(default)]
    pub retry: bool,
}
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookItem {
    pub slot: String,
    pub display: u32,
    pub category: String,
}
impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EconomyRequest")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for Reply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EconomyReply")
            .field("id", &self.id)
            .field("ok", &self.ok)
            .field("message", &self.message)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gear is what carries a hero up the roads: each tier's weapons and armour outclass the last,
    /// and every upgrade adds to an item.
    #[test]
    fn gear_power_grows_with_tier_and_upgrades() {
        let catalog = Catalog::load();
        assert!(catalog.zones.values().all(|z| (1..=5).contains(&z.tier)));
        let average = |tier: u8, slot: &str, stat: &str| {
            let values: Vec<u32> = catalog
                .items
                .iter()
                .filter(|d| catalog.tier(&d.zone) == tier && d.slot == slot && !d.two_handed)
                .filter_map(|d| d.base_bonuses.get(stat).copied())
                .collect();
            values.iter().sum::<u32>() as f32 / values.len().max(1) as f32
        };
        for tier in 2..=5 {
            for (slot, stat) in [("main_hand", "damage_bp"), ("chest", "max_health"), ("chest", "damage_reduction_bp")] {
                assert!(
                    average(tier, slot, stat) > average(tier - 1, slot, stat),
                    "tier {tier} {slot} {stat}"
                );
            }
            assert!(recommended_score(tier) > recommended_score(tier - 1));
        }
        for item in &catalog.items {
            let mut last = &item.base_bonuses;
            assert_eq!(item.upgrades.len(), 5, "{}", item.id);
            for upgrade in &item.upgrades {
                assert!(last.iter().all(|(k, v)| upgrade.bonuses.get(k).is_some_and(|n| n >= v)), "{}", item.id);
                last = &upgrade.bonuses;
            }
            if matches!(item.slot.as_str(), "main_hand" | "off_hand" | "ranged") {
                assert!(item.base_bonuses.contains_key("damage_bp"), "{}", item.id);
            }
        }
    }

    #[test]
    fn item_level_follows_tier_quality_and_upgrade() {
        let catalog = Catalog::load();
        let item = |zone: &str| {
            catalog
                .items
                .iter()
                .find(|d| d.zone == zone && d.slot == "chest")
                .unwrap()
        };
        let (elwynn, stranglethorn) = (item("elwynn"), item("stranglethorn"));
        assert!(stranglethorn.item_level(&catalog, 0) > elwynn.item_level(&catalog, 5));
        assert_eq!(elwynn.item_level(&catalog, 3), elwynn.item_level(&catalog, 0) + 6);
        assert!(stranglethorn.score(&catalog, 0) > elwynn.score(&catalog, 0));
    }
}
