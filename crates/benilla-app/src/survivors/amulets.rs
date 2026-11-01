//! Fixed amulets whose complete stat effects have been verified from the original.
//! Proc-bearing necklaces remain excluded until their triggers have been established.
use super::rewards::Rarity;
pub(super) struct AmuletDef {
    pub name: &'static str,
    pub icon: &'static str,
    pub rarity: Rarity,
    pub stats: &'static [(u8, f32)],
    pub targets: u32,
}
pub(super) static AMULETS: &[AmuletDef] = &[
    AmuletDef {
        name: "Mark of Fordring",
        icon: "Interface\\Icons\\INV_Jewelry_Talisman_07",
        rarity: Rarity::Common,
        stats: &[(0, 0.15)],
        targets: 0,
    },
    AmuletDef {
        name: "Talvash's Enhancing Necklace",
        icon: "Interface\\Icons\\INV_Jewelry_Necklace_02",
        rarity: Rarity::Common,
        stats: &[(1, 0.1)],
        targets: 0,
    },
    AmuletDef {
        name: "Pristine Enchanted South Seas Kelp",
        icon: "Interface\\Icons\\INV_Jewelry_Necklace_26",
        rarity: Rarity::Common,
        stats: &[(12, 0.3), (11, 0.25)],
        targets: 0,
    },
    AmuletDef {
        name: "Prestor's Talisman of Connivery",
        icon: "Interface\\Icons\\INV_Jewelry_Necklace_17",
        rarity: Rarity::Uncommon,
        stats: &[(0, 0.16), (3, 0.04)],
        targets: 0,
    },
    AmuletDef {
        name: "Animated Chain Necklace",
        icon: "Interface\\Icons\\INV_Jewelry_Necklace_01",
        rarity: Rarity::Uncommon,
        stats: &[(2, 0.25), (5, 0.08)],
        targets: 0,
    },
    AmuletDef {
        name: "Woven Ivy Necklace",
        icon: "Interface\\Icons\\INV_Misc_Herb_15",
        rarity: Rarity::Uncommon,
        stats: &[(7, 6.0), (2, 0.12), (10, 0.12)],
        targets: 0,
    },
    AmuletDef {
        name: "Amulet of the Darkmoon",
        icon: "Interface\\Icons\\INV_Jewelry_Necklace_14",
        rarity: Rarity::Uncommon,
        stats: &[(1, 0.12), (10, 0.08)],
        targets: 0,
    },
    AmuletDef {
        name: "Jeklik's Opaline Talisman",
        icon: "Interface\\Icons\\INV_Jewelry_Necklace_03",
        rarity: Rarity::Rare,
        stats: &[(18, 0.03), (15, 1.0)],
        targets: 0,
    },
    AmuletDef {
        name: "Medallion of Steadfast Might",
        icon: "Interface\\Icons\\INV_Jewelry_Amulet_03",
        rarity: Rarity::Rare,
        stats: &[(5, 0.1), (2, 0.15), (20, 1.0)],
        targets: 0,
    },
    AmuletDef {
        name: "Amulet of Vek'nilash",
        icon: "Interface\\Icons\\INV_Jewelry_Necklace_AhnQiraj_01",
        rarity: Rarity::Epic,
        stats: &[(0, 0.05)],
        targets: 1,
    },
    AmuletDef {
        name: "Malice Stone Pendant",
        icon: "Interface\\Icons\\INV_Jewelry_Necklace_29Naxxramas",
        rarity: Rarity::Epic,
        stats: &[(9, 0.2), (1, 0.08)],
        targets: 0,
    },
];
#[derive(Clone, Debug)]
pub(crate) struct Amulet {
    pub index: usize,
    pub rank: u32,
}
impl Amulet {
    pub fn scale(&self) -> f32 {
        let r = AMULETS[self.index].rarity as usize;
        3. - 2.
            * (1. - 0.5 * [0.6_f32, 0.5, 0.35, 0.25, 0.2][r])
                .powi(self.rank.saturating_sub(1).min(i32::MAX as u32) as i32)
    }
}
