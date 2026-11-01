//! Persistent equipment and a shared gold marketplace, using ASR's owned catalog.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub type Amounts = BTreeMap<String, u32>;

#[derive(Clone, Deserialize)]
pub struct Catalog {
    pub items: Vec<Definition>,
    pub zones: BTreeMap<String, Zone>,
}
#[derive(Clone, Deserialize)]
pub struct Definition {
    pub id: String,
    pub name: String,
    pub zone: String,
    pub slot: String,
    pub category: String,
    pub icon: String,
    pub display_id: u32,
    pub source_item_id: u32,
    pub classes: Vec<u8>,
    pub two_handed: bool,
    pub base_bonuses: Amounts,
    pub upgrades: Vec<Upgrade>,
    pub scrap: Amounts,
}
#[derive(Clone, Deserialize)]
pub struct Upgrade {
    pub cost: Amounts,
    pub bonuses: Amounts,
}
#[derive(Clone, Deserialize)]
pub struct Zone {
    pub name: String,
    pub equipment: Vec<Drop>,
    pub equipment_chance_bp: Amounts,
    pub material_chance_bp: Amounts,
    pub materials: Amounts,
    pub material_pool: Vec<String>,
}
#[derive(Clone, Deserialize)]
pub struct Drop {
    pub item: String,
    pub weight: u32,
}
impl Catalog {
    pub fn load() -> Self {
        serde_json::from_str(include_str!("../assets/economy_catalog.json"))
            .expect("bundled ASR item catalog")
    }
    pub fn definition(&self, id: &str) -> Option<&Definition> {
        self.items.iter().find(|d| d.id == id)
    }
}
impl Definition {
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
        self.classes.contains(&class)
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
    pub locked: bool,
    pub listed: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub name: String,
    pub class: u8,
    pub gold: u64,
    pub materials: Amounts,
    pub items: Vec<Item>,
}
impl Inventory {
    pub fn bonuses(&self, catalog: &Catalog) -> Amounts {
        let mut result = Amounts::new();
        for item in self.items.iter().filter(|i| i.slot.is_some()) {
            if let Some(d) = catalog.definition(&item.definition) {
                for (key, value) in d.bonuses(item.level) {
                    let entry = result.entry(key.clone()).or_default();
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
        gold: u32,
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
