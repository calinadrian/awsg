//! Run jewelry, separate from permanent equipment.
use super::rewards::{self, Rarity};
pub(super) struct JewelDef {
    pub name: &'static str,
    pub icon: &'static str,
    pub stat: u8,
    pub base: f32,
    pub ring: bool,
}
pub(super) static JEWELS: &[JewelDef] = &[
    JewelDef {
        name: "Band of Accuria",
        icon: "Interface\\Icons\\INV_Jewelry_Ring_15",
        stat: 0,
        base: 0.17,
        ring: true,
    },
    JewelDef {
        name: "Quick Strike Ring",
        icon: "Interface\\Icons\\INV_Jewelry_Ring_07",
        stat: 1,
        base: 0.09,
        ring: true,
    },
    JewelDef {
        name: "Heavy Dark Iron Ring",
        icon: "Interface\\Icons\\INV_Jewelry_Ring_14",
        stat: 2,
        base: 0.2,
        ring: true,
    },
    JewelDef {
        name: "Master Dragonslayer's Ring",
        icon: "Interface\\Icons\\INV_Jewelry_Ring_41",
        stat: 3,
        base: 0.04,
        ring: true,
    },
    JewelDef {
        name: "Archimtiros' Ring of Reckoning",
        icon: "Interface\\Icons\\INV_Jewelry_Ring_40",
        stat: 5,
        base: 0.035,
        ring: true,
    },
    JewelDef {
        name: "Ring of the Devoured",
        icon: "Interface\\Icons\\INV_Jewelry_Ring_AhnQiraj_05",
        stat: 7,
        base: 1.5,
        ring: true,
    },
    JewelDef {
        name: "Ring of Spell Power",
        icon: "Interface\\Icons\\INV_Jewelry_Ring_38",
        stat: 8,
        base: 0.14,
        ring: true,
    },
    JewelDef {
        name: "Seal of the Damned",
        icon: "Interface\\Icons\\INV_Jewelry_Ring_51Naxxramas",
        stat: 19,
        base: 0.1,
        ring: true,
    },
    JewelDef {
        name: "Blooddrinker's Band",
        icon: "Interface\\Icons\\INV_Jewelry_Ring_49Naxxramas",
        stat: 18,
        base: 0.01,
        ring: true,
    },
    JewelDef {
        name: "Earthstrike",
        icon: "Interface\\Icons\\Spell_Nature_AbolishMagic",
        stat: 0,
        base: 0.13,
        ring: false,
    },
    JewelDef {
        name: "Talisman of Ephemeral Power",
        icon: "Interface\\Icons\\INV_Misc_StoneTablet_11",
        stat: 1,
        base: 0.07,
        ring: false,
    },
    JewelDef {
        name: "Lifegiving Gem",
        icon: "Interface\\Icons\\INV_Misc_Gem_Pearl_05",
        stat: 2,
        base: 0.16,
        ring: false,
    },
    JewelDef {
        name: "Hand of Justice",
        icon: "Interface\\Icons\\INV_Jewelry_Talisman_01",
        stat: 9,
        base: 0.06,
        ring: false,
    },
    JewelDef {
        name: "Blackhand's Breadth",
        icon: "Interface\\Icons\\INV_Misc_ArmorKit_09",
        stat: 4,
        base: 0.195,
        ring: false,
    },
    JewelDef {
        name: "Carrot on a Stick",
        icon: "Interface\\Icons\\INV_Misc_Food_54",
        stat: 10,
        base: 0.08,
        ring: false,
    },
    JewelDef {
        name: "Gnomish Universal Remote",
        icon: "Interface\\Icons\\INV_Misc_PocketWatch_01",
        stat: 11,
        base: 0.35,
        ring: false,
    },
    JewelDef {
        name: "Insignia of the Horde",
        icon: "Interface\\Icons\\INV_Jewelry_TrinketPVP_02",
        stat: 12,
        base: 0.12,
        ring: false,
    },
    JewelDef {
        name: "Royal Seal of Eldre'Thalas",
        icon: "Interface\\Icons\\INV_Jewelry_Talisman_10",
        stat: 13,
        base: 0.25,
        ring: false,
    },
    JewelDef {
        name: "Mark of the Chosen",
        icon: "Interface\\Icons\\INV_Jewelry_Talisman_08",
        stat: 14,
        base: 0.15,
        ring: false,
    },
    JewelDef {
        name: "Onyxia Blood Talisman",
        icon: "Interface\\Icons\\Spell_Shadow_LifeDrain",
        stat: 15,
        base: 2.0,
        ring: false,
    },
    JewelDef {
        name: "Horn of the Warchief",
        icon: "Interface\\Icons\\INV_Misc_Horn_01",
        stat: 17,
        base: 0.15,
        ring: false,
    },
    JewelDef {
        name: "Major Soulstone",
        icon: "Interface\\Icons\\INV_Misc_Orb_04",
        stat: 16,
        base: 1.0,
        ring: false,
    },
    JewelDef {
        name: "Star of Mystaria",
        icon: "Interface\\Icons\\INV_Enchant_EssenceMagicLarge",
        stat: 6,
        base: 0.16666667,
        ring: false,
    },
];
#[derive(Clone, Debug)]
pub(crate) struct Jewel {
    pub index: usize,
    pub strength: f32,
    pub picks: u32,
    pub rarity: Rarity,
}
impl Jewel {
    pub fn gain(&self, r: Rarity) -> f32 {
        let d = &JEWELS[self.index];
        rewards::jewel_gain(d.stat, d.base, r, Some((self.strength, self.picks)))
    }
}
pub(super) fn label(stat: u8) -> &'static str {
    match stat {
        0 => "Damage",
        1 => "Cooldown reduction",
        2 | 6 => "Maximum health",
        3 => "Critical chance",
        4 => "Critical damage",
        5 => "Damage reduction",
        7 => "Health per second",
        8 => "Area",
        9 => "Double cast",
        10 => "Move speed",
        11 => "Pickup range",
        12 => "Experience",
        13 => "Gold",
        14 => "Luck",
        15 => "Health per kill",
        16 => "Extra lives",
        17 => "Buff power",
        18 => "Life steal",
        19 => "DoT speed",
        20 => "Reflected melee damage",
        _ => "Bonus",
    }
}
pub(super) fn amount(stat: u8, v: f32) -> String {
    if matches!(stat, 7 | 15 | 16) {
        format!("{v:.2}")
    } else {
        format!("{:.1}%", v * 100.)
    }
}

/// The original jewelry formatter (0x5ad4ee0) also exposes the linked health/defense values.
pub(super) fn effects(index: usize, strength: f32) -> [(u8, f32); 2] {
    let d = &JEWELS[index];
    match d.stat {
        5 => [(5, strength.min(3. * d.base)), (2, 4. * strength)],
        6 => [(6, strength), (5, (0.16 * strength).min(0.12))],
        _ => [(d.stat, strength), (255, 0.)],
    }
}
pub(super) fn stat(index: usize, strength: f32, stat: u8) -> f32 {
    effects(index, strength)
        .into_iter()
        .filter(|(tag, _)| *tag == stat)
        .map(|(_, v)| v)
        .sum()
}
pub(super) fn delta(index: usize, before: f32, after: f32) -> String {
    effects(index, after)
        .into_iter()
        .filter(|(tag, _)| *tag != 255)
        .map(|(tag, v)| {
            format!(
                "{}\n{}  >  {}",
                label(tag),
                amount(tag, stat(index, before, tag)),
                amount(tag, v)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
