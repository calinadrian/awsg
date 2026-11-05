//! Each class's basic gear: real items, worn from a hero's first moment, so what a character shows
//! is always what it has equipped. They carry no stats and no gear score, leaving balance untouched.
use crate::economy::{Catalog, Definition, Inventory, Item, LookItem};

/// The `Definition::zone` of every starter item; it names no real zone, so nothing drops them.
pub const ZONE: &str = "starter";

const SHIRT: &str = "Interface\\Icons\\INV_Shirt_05";
const PANTS: &str = "Interface\\Icons\\INV_Pants_02";
const BOOTS: &str = "Interface\\Icons\\INV_Boots_01";
const ROBE: &str = "Interface\\Icons\\INV_Chest_Cloth_21";
const SWORD: &str = "Interface\\Icons\\INV_Sword_04";
const MACE: &str = "Interface\\Icons\\INV_Mace_01";
const DAGGER: &str = "Interface\\Icons\\INV_Weapon_ShortBlade_01";
const STAFF: &str = "Interface\\Icons\\INV_Staff_08";
const POLEARM: &str = "Interface\\Icons\\INV_Spear_04";
const BOW: &str = "Interface\\Icons\\INV_Weapon_Bow_01";
const SHIELD: &str = "Interface\\Icons\\INV_Shield_09";

/// One piece: slot, name, `ItemDisplayInfo` id, category and icon. Armor displays are the game's
/// own starting outfits; weapons keep the look each class's trainer always carried.
type Piece = (&'static str, &'static str, u32, &'static str, &'static str);

const SETS: &[(u8, &[Piece])] = &[
    (
        1,
        &[
            ("shirt", "Recruit's Shirt", 9891, "shirt", SHIRT),
            ("legs", "Recruit's Pants", 9892, "legs", PANTS),
            ("feet", "Recruit's Boots", 10141, "feet", BOOTS),
            (
                "main_hand",
                "Worn Shortsword",
                7483,
                "one_hand_sword",
                SWORD,
            ),
            ("off_hand", "Worn Wooden Shield", 1706, "shield", SHIELD),
        ],
    ),
    (
        2,
        &[
            ("shirt", "Squire's Shirt", 3265, "shirt", SHIRT),
            ("legs", "Squire's Pants", 9937, "legs", PANTS),
            ("feet", "Squire's Boots", 10272, "feet", BOOTS),
            ("main_hand", "Worn Mace", 8011, "one_hand_mace", MACE),
            ("off_hand", "Worn Round Shield", 1705, "shield", SHIELD),
        ],
    ),
    (
        3,
        &[
            ("shirt", "Trapper's Shirt", 9996, "shirt", SHIRT),
            ("legs", "Trapper's Pants", 10002, "legs", PANTS),
            ("feet", "Trapper's Boots", 10003, "feet", BOOTS),
            ("main_hand", "Worn Spear", 7480, "polearm", POLEARM),
            ("ranged", "Worn Shortbow", 6235, "bow", BOW),
        ],
    ),
    (
        4,
        &[
            ("shirt", "Footpad's Shirt", 9906, "shirt", SHIRT),
            ("legs", "Footpad's Pants", 9913, "legs", PANTS),
            ("feet", "Footpad's Shoes", 9915, "feet", BOOTS),
            (
                "main_hand",
                "Worn Shortsword",
                7492,
                "one_hand_sword",
                SWORD,
            ),
            ("off_hand", "Worn Dagger", 6443, "dagger", DAGGER),
        ],
    ),
    (
        5,
        &[
            ("shirt", "Neophyte's Shirt", 9944, "shirt", SHIRT),
            ("chest", "Neophyte's Robe", 12679, "chest", ROBE),
            ("legs", "Neophyte's Pants", 9945, "legs", PANTS),
            ("feet", "Neophyte's Boots", 9946, "feet", BOOTS),
            ("main_hand", "Worn Staff", 1926, "staff", STAFF),
        ],
    ),
    (
        7,
        &[
            ("shirt", "Primal Shirt", 10058, "shirt", SHIRT),
            ("legs", "Primal Kilt", 10050, "legs", PANTS),
            ("main_hand", "Worn Mace", 7477, "one_hand_mace", MACE),
            ("off_hand", "Worn Round Shield", 1705, "shield", SHIELD),
        ],
    ),
    (
        8,
        &[
            ("shirt", "Apprentice's Shirt", 2163, "shirt", SHIRT),
            ("chest", "Apprentice's Robe", 12647, "chest", ROBE),
            ("legs", "Apprentice's Pants", 9924, "legs", PANTS),
            ("feet", "Apprentice's Boots", 9929, "feet", BOOTS),
            ("main_hand", "Worn Staff", 1927, "staff", STAFF),
        ],
    ),
    (
        9,
        &[
            ("shirt", "Acolyte's Shirt", 2470, "shirt", SHIRT),
            ("chest", "Acolyte's Robe", 12645, "chest", ROBE),
            ("legs", "Acolyte's Pants", 3260, "legs", PANTS),
            ("feet", "Acolyte's Shoes", 3261, "feet", BOOTS),
            ("main_hand", "Worn Staff", 5542, "staff", STAFF),
        ],
    ),
    (
        11,
        &[
            ("chest", "Grove Robe", 12684, "chest", ROBE),
            ("legs", "Grove Pants", 9987, "legs", PANTS),
            ("main_hand", "Worn Staff", 22391, "staff", STAFF),
        ],
    ),
];

/// Every class's starter items, appended to the catalog when it loads.
pub fn definitions() -> Vec<Definition> {
    let mut out = Vec::new();
    for &(class, pieces) in SETS {
        for &(slot, name, display_id, category, icon) in pieces {
            out.push(Definition {
                id: id(class, slot),
                name: name.into(),
                zone: ZONE.into(),
                slot: slot.into(),
                category: category.into(),
                quality: "common".into(),
                armor: None,
                icon: icon.into(),
                display_id,
                source_item_id: 0,
                classes: vec![class],
                two_handed: matches!(category, "staff" | "polearm"),
                spell_damage_bp: Default::default(),
                set_name: None,
                base_bonuses: Default::default(),
                upgrades: Vec::new(),
                scrap: Default::default(),
            });
        }
    }
    out
}

fn id(class: u8, slot: &str) -> String {
    format!("{ZONE}:{class}:{slot}")
}

/// Give `class` its starter set once, wearing each piece in a slot still free. A slot already filled
/// keeps its item and that piece is skipped, so a veteran's bags gain nothing they would never use.
/// Returns whether the inventory changed.
pub fn grant(inventory: &mut Inventory, class: u8, serial: &mut u64, catalog: &Catalog) -> bool {
    if !inventory.starter_classes.insert(class) {
        return false;
    }
    let worn = |items: &[Item], slot: &str| {
        items
            .iter()
            .find(|i| i.equipped_for(class) && i.slot.as_deref() == Some(slot))
            .map(|i| {
                catalog
                    .definition(&i.definition)
                    .is_some_and(|d| d.two_handed)
            })
    };
    let pieces = SETS
        .iter()
        .find(|(c, _)| *c == class)
        .map_or(&[][..], |(_, p)| *p);
    for &(slot, ..) in pieces {
        let definition = id(class, slot);
        let two_handed = catalog
            .definition(&definition)
            .is_some_and(|d| d.two_handed);
        let blocked = worn(&inventory.items, slot).is_some()
            || (two_handed && worn(&inventory.items, "off_hand").is_some())
            || (slot == "off_hand" && worn(&inventory.items, "main_hand") == Some(true));
        if blocked {
            continue;
        }
        *serial += 1;
        inventory.items.push(Item {
            id: *serial,
            definition,
            slot: Some(slot.into()),
            equipped_class: Some(class),
            ..Item::default()
        });
    }
    true
}

/// What `class` shows wearing `inventory`, or a brand new hero's set without one. It includes the
/// starter set a save has not been given yet, so a character is never bare while the ledger answers.
pub fn look(inventory: Option<&Inventory>, class: u8, catalog: &Catalog) -> Vec<LookItem> {
    let mut inventory = inventory.cloned().unwrap_or_default();
    grant(&mut inventory, class, &mut 0, catalog);
    inventory
        .items
        .iter()
        .filter(|item| item.equipped_for(class))
        .filter_map(|item| {
            let slot = item.slot.as_ref()?;
            let d = catalog.definition(&item.definition)?;
            d.fits(class, slot).then(|| LookItem {
                slot: slot.clone(),
                display: d.display_id,
                category: d.category.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        Catalog::load()
    }

    #[test]
    fn every_class_can_wear_its_whole_set_and_it_changes_nothing_in_balance() {
        let catalog = catalog();
        for &(class, pieces) in SETS {
            let mut inventory = Inventory {
                class,
                ..Inventory::default()
            };
            assert!(grant(&mut inventory, class, &mut 0, &catalog));
            assert_eq!(inventory.items.len(), pieces.len(), "class {class}");
            assert_eq!(
                look(Some(&inventory), class, &catalog).len(),
                pieces.len(),
                "class {class}"
            );
            assert_eq!(inventory.gear_score(&catalog, class), 0, "class {class}");
            assert!(inventory.bonuses(&catalog).is_empty(), "class {class}");
        }
    }

    #[test]
    fn the_set_is_given_once_so_a_removed_piece_stays_removed() {
        let catalog = catalog();
        let mut inventory = Inventory {
            class: 1,
            ..Inventory::default()
        };
        grant(&mut inventory, 1, &mut 0, &catalog);
        let count = inventory.items.len();
        inventory.items.pop();
        assert!(!grant(&mut inventory, 1, &mut 0, &catalog));
        assert_eq!(inventory.items.len(), count - 1);
    }

    #[test]
    fn worn_gear_keeps_its_slot_when_an_old_save_is_brought_forward() {
        let catalog = catalog();
        let tunic = Item {
            id: 7,
            definition: "elwynn:9757".into(),
            slot: Some("chest".into()),
            ..Item::default()
        };
        let boots = Item {
            id: 8,
            definition: "elwynn:6201".into(),
            slot: Some("feet".into()),
            equipped_class: Some(4),
            ..Item::default()
        };
        let mut inventory = Inventory {
            class: 4,
            items: vec![tunic, boots],
            ..Inventory::default()
        };
        let mut serial = 8;
        grant(&mut inventory, 4, &mut serial, &catalog);
        let worn = |slot: &str| {
            inventory
                .items
                .iter()
                .filter(|i| i.slot.as_deref() == Some(slot))
                .map(|i| i.definition.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(worn("chest"), ["elwynn:9757"]);
        assert_eq!(
            worn("feet"),
            ["elwynn:6201"],
            "the starter boots stay out of a filled slot"
        );
        assert_eq!(worn("legs"), ["starter:4:legs"], "an empty slot is filled");
        assert!(
            inventory.items.iter().all(|i| i.id != 0),
            "ids come from the ledger's serial"
        );
    }

    #[test]
    fn a_two_handed_starter_never_displaces_a_worn_off_hand() {
        let catalog = catalog();
        let shield = Item {
            id: 1,
            definition: "starter:7:off_hand".into(),
            slot: Some("off_hand".into()),
            equipped_class: Some(3),
            ..Item::default()
        };
        let mut inventory = Inventory {
            class: 3,
            items: vec![shield],
            ..Inventory::default()
        };
        grant(&mut inventory, 3, &mut 1, &catalog);
        assert!(inventory
            .items
            .iter()
            .all(|i| i.slot.as_deref() != Some("main_hand")));
    }

    #[test]
    fn a_hero_without_a_save_shows_the_set_rather_than_nothing() {
        let catalog = catalog();
        for &(class, pieces) in SETS {
            assert_eq!(
                look(None, class, &catalog).len(),
                pieces.len(),
                "class {class}"
            );
        }
    }
}
