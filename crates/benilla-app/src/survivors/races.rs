//! Local ASR roster; models and textures live in the user patch chain.
use super::data::ClassDef;
use crate::entities::Creatures;
pub(super) struct Race {
    pub id: u8,
    pub name: &'static str,
    pub faction: u8,
    pub classes: &'static [u8],
    pub portrait: (u16, u16),
}
include!("races_data.rs");

/// A racial stat: the original race effect tags 0-11, in tag order, as the stat rebuild
/// (RVA 0x5be8f80, dispatch 0x76bc59c) applies them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Bonus {
    /// A share of maximum health.
    Health,
    Damage,
    /// Cooldown recovery, as haste: cooldowns divide by one plus it.
    Cooldowns,
    Speed,
    Crit,
    Reduction,
    /// Health a second.
    Regen,
    Pickup,
    Experience,
    Gold,
    Luck,
    /// Health per kill.
    KillHeal,
}

/// A race's traits: an optional proc cast on its own clock, then its standing bonuses.
pub(super) struct Racial {
    pub id: u8,
    /// The trait's name, its [`super::data::ability`] key and what it does.
    pub proc: Option<(&'static str, &'static str, &'static str)>,
    pub bonuses: &'static [(&'static str, Bonus, f32)],
}

/// The original race rows (RVA 0x954c0b0 and the table at 0x954d688, 88 bytes each: effects
/// at +0x20, the proc key at +0x30). The two Skyborne races are ours; the original has no rows
/// for them.
pub(super) static RACIALS: &[Racial] = &[
    Racial {
        id: 1,
        proc: Some((
            "Perception",
            "perception",
            "every 30 sec, +15% damage for 10 sec",
        )),
        bonuses: &[
            ("The Human Spirit", Bonus::Experience, 0.10),
            ("Diplomacy", Bonus::Gold, 0.20),
        ],
    },
    Racial {
        id: 2,
        proc: Some((
            "Blood Fury",
            "blood_fury",
            "every 40 sec, +35% damage for 12 sec",
        )),
        bonuses: &[("Hardiness", Bonus::Health, 0.10)],
    },
    Racial {
        id: 3,
        proc: Some((
            "Stoneform",
            "stoneform",
            "every 45 sec, a shield absorbing 30% of max health",
        )),
        bonuses: &[
            ("Gun Specialization", Bonus::Damage, 0.06),
            ("Treasure Finding", Bonus::Pickup, 0.30),
        ],
    },
    Racial {
        id: 4,
        proc: Some((
            "Shadowmeld",
            "shadowmeld",
            "near death, 3 sec of immunity (75 sec cooldown)",
        )),
        bonuses: &[
            ("Quickness", Bonus::Speed, 0.08),
            ("Nature Resistance", Bonus::Reduction, 0.05),
        ],
    },
    Racial {
        id: 5,
        proc: Some((
            "Will of the Forsaken",
            "will_of_the_forsaken",
            "near death, 4 sec of immunity (60 sec cooldown)",
        )),
        bonuses: &[
            ("Cannibalize", Bonus::KillHeal, 1.0),
            ("Shadow Resistance", Bonus::Reduction, 0.04),
        ],
    },
    Racial {
        id: 6,
        proc: Some((
            "War Stomp",
            "war_stomp",
            "every 15 sec, stuns nearby enemies for 2 sec",
        )),
        bonuses: &[("Endurance", Bonus::Health, 0.15)],
    },
    Racial {
        id: 7,
        proc: Some((
            "Escape Artist",
            "escape_artist",
            "every 20 sec, roots nearby enemies for 2 sec",
        )),
        bonuses: &[
            ("Expansive Mind", Bonus::Cooldowns, 0.08),
            ("Engineering Specialization", Bonus::Luck, 0.20),
        ],
    },
    Racial {
        id: 8,
        proc: Some((
            "Berserking",
            "berserking",
            "every 35 sec, cooldowns recover 30% faster for 10 sec",
        )),
        bonuses: &[
            ("Regeneration", Bonus::Regen, 1.5),
            ("Throwing Specialization", Bonus::Crit, 0.03),
        ],
    },
    Racial {
        id: 9,
        proc: None,
        bonuses: &[
            ("Time is Money", Bonus::Cooldowns, 0.07),
            ("Best Deals Anywhere", Bonus::Gold, 0.20),
            ("Better Living Through Chemistry", Bonus::Regen, 0.8),
        ],
    },
    Racial {
        id: 10,
        // The original names this proc but has no spell row for it; ours is in `data`.
        proc: Some((
            "Arcane Torrent",
            "arcane_torrent",
            "every 20 sec, stuns nearby enemies for 2 sec",
        )),
        bonuses: &[
            ("Magic Resistance", Bonus::Reduction, 0.05),
            ("Arcane Affinity", Bonus::Cooldowns, 0.05),
        ],
    },
    Racial {
        id: 11,
        proc: None,
        bonuses: &[
            ("Heroic Presence", Bonus::Damage, 0.06),
            ("Gift of the Naaru", Bonus::Regen, 1.0),
            ("Shadow Resistance", Bonus::Reduction, 0.04),
        ],
    },
    Racial {
        id: 22,
        proc: None,
        bonuses: &[
            ("Viciousness", Bonus::Crit, 0.03),
            ("Running Wild", Bonus::Speed, 0.08),
            ("Aberration", Bonus::Reduction, 0.04),
        ],
    },
    Racial {
        id: 24,
        proc: None,
        bonuses: &[
            ("Epicurean", Bonus::Health, 0.12),
            ("Inner Peace", Bonus::Experience, 0.10),
            ("Bouncy", Bonus::Reduction, 0.05),
        ],
    },
    Racial {
        id: 27,
        proc: None,
        bonuses: &[
            ("Magical Affinity", Bonus::Damage, 0.08),
            ("Ancient History", Bonus::Experience, 0.08),
            ("Arcane Resistance", Bonus::Reduction, 0.04),
        ],
    },
    Racial {
        id: 28,
        proc: None,
        bonuses: &[
            ("Rugged Tenacity", Bonus::Reduction, 0.06),
            ("Mountaineer", Bonus::Health, 0.10),
            ("Waste Not, Want Not", Bonus::Gold, 0.15),
        ],
    },
    Racial {
        id: 29,
        proc: None,
        bonuses: &[
            ("Entropic Embrace", Bonus::Damage, 0.08),
            ("Preternatural Calm", Bonus::Cooldowns, 0.06),
            ("Chill of Night", Bonus::Reduction, 0.04),
        ],
    },
    Racial {
        id: 30,
        proc: None,
        bonuses: &[
            ("Demonbane", Bonus::Damage, 0.07),
            ("Holy Providence", Bonus::Regen, 1.0),
            ("Forge of Light", Bonus::Health, 0.08),
        ],
    },
    Racial {
        id: 31,
        proc: None,
        bonuses: &[
            ("Embrace of Pa'ku", Bonus::Crit, 0.03),
            ("City of Gold", Bonus::Gold, 0.20),
            ("Regeneratin'", Bonus::Regen, 1.0),
        ],
    },
    Racial {
        id: 32,
        proc: None,
        bonuses: &[
            ("Brush It Off", Bonus::Regen, 0.8),
            ("Rime of the Ancient Mariner", Bonus::Reduction, 0.05),
            ("Jack of All Trades", Bonus::Luck, 0.10),
        ],
    },
    Racial {
        id: 34,
        proc: None,
        bonuses: &[
            ("Forged in Flames", Bonus::Reduction, 0.05),
            ("Dungeon Delver", Bonus::Speed, 0.06),
            ("Mass Production", Bonus::Cooldowns, 0.05),
        ],
    },
    Racial {
        id: 35,
        proc: None,
        bonuses: &[
            ("Nose for Trouble", Bonus::Reduction, 0.05),
            ("Alpaca Saddlebags", Bonus::Pickup, 0.35),
            ("Bag of Tricks", Bonus::Luck, 0.15),
        ],
    },
    Racial {
        id: 36,
        proc: None,
        bonuses: &[
            ("Ancestral Call", Bonus::Damage, 0.06),
            ("Savage Blood", Bonus::Reduction, 0.05),
            ("Sympathetic Vigor", Bonus::Health, 0.10),
        ],
    },
    Racial {
        id: 37,
        proc: None,
        bonuses: &[
            ("Combat Analysis", Bonus::Damage, 0.06),
            ("Re-Arm", Bonus::Regen, 1.2),
            ("Skeleton Pinkie", Bonus::Gold, 0.15),
        ],
    },
    Racial {
        id: 52,
        proc: None,
        bonuses: &[
            ("Awakened", Bonus::Crit, 0.03),
            ("Discerning Eye", Bonus::Pickup, 0.25),
            ("Glide", Bonus::Speed, 0.08),
        ],
    },
    Racial {
        id: 84,
        proc: None,
        bonuses: &[
            ("Titan-Wrought Frame", Bonus::Reduction, 0.06),
            ("Ingest Minerals", Bonus::Health, 0.08),
            ("Wide-Eyed Wonder", Bonus::Experience, 0.12),
        ],
    },
    Racial {
        id: 86,
        proc: None,
        bonuses: &[
            ("Subterranean Predator", Bonus::Experience, 0.10),
            ("Lash Out", Bonus::Crit, 0.03),
            ("One with Nature", Bonus::Pickup, 0.25),
        ],
    },
    // High Order Skyborne: sharp-eyed sentinels of the upper air.
    Racial {
        id: 95,
        proc: None,
        bonuses: &[
            ("Sky Sentinel", Bonus::Damage, 0.06),
            ("Hawk's Eye", Bonus::Crit, 0.03),
            ("Updraft", Bonus::Speed, 0.08),
        ],
    },
    // Windshaper Skyborne: they bend the wind to carry what they need.
    Racial {
        id: 96,
        proc: None,
        bonuses: &[
            ("Tailwind", Bonus::Cooldowns, 0.06),
            ("Feathered Guard", Bonus::Reduction, 0.05),
            ("Gale Gatherer", Bonus::Pickup, 0.25),
        ],
    },
];

pub(super) fn racial(id: u8) -> Option<&'static Racial> {
    RACIALS.iter().find(|r| r.id == id)
}

/// The race's total for one bonus.
pub(super) fn bonus(id: u8, kind: Bonus) -> f32 {
    racial(id).map_or(0.0, |r| {
        r.bonuses.iter().filter(|b| b.1 == kind).map(|b| b.2).sum()
    })
}

/// One line per trait, "Name: effect", as the pick screen lists them.
pub(super) fn trait_lines(id: u8) -> Vec<String> {
    let Some(r) = racial(id) else {
        return Vec::new();
    };
    let percent = |v: f32| format!("{:.0}%", v * 100.0);
    r.proc
        .iter()
        .map(|(name, _, text)| format!("{name}: {text}"))
        .chain(r.bonuses.iter().map(|&(name, kind, v)| {
            let effect = match kind {
                Bonus::Health => format!("+{} max health", percent(v)),
                Bonus::Damage => format!("+{} damage", percent(v)),
                Bonus::Cooldowns => format!("-{} cooldowns", percent(v)),
                Bonus::Speed => format!("+{} movement speed", percent(v)),
                Bonus::Crit => format!("+{} critical strike", percent(v)),
                Bonus::Reduction => format!("+{} damage reduction", percent(v)),
                Bonus::Regen => format!("+{v} health per second"),
                Bonus::Pickup => format!("+{} pickup range", percent(v)),
                Bonus::Experience => format!("+{} experience", percent(v)),
                Bonus::Gold => format!("+{} gold", percent(v)),
                Bonus::Luck => format!("+{} luck", percent(v)),
                Bonus::KillHeal => format!("every kill heals {v} health"),
            };
            format!("{name}: {effect}")
        }))
        .collect()
}

pub(super) fn race_name(id: u8) -> &'static str {
    RACES
        .iter()
        .find(|r| r.id == id)
        .map_or("Unknown", |r| r.name)
}
pub(super) fn race_tc(id: u8, sex: u8) -> Option<[f32; 4]> {
    let r = RACES.iter().find(|r| r.id == id)?;
    let (x, y) = (
        f32::from(r.portrait.0),
        f32::from(r.portrait.1) + f32::from(sex.min(1)) * 128.0,
    );
    Some([
        x / 1024.0,
        (x + 64.0) / 1024.0,
        y / 256.0,
        (y + 64.0) / 256.0,
    ])
}
pub(super) fn looks_for(class: &ClassDef, creatures: &Creatures) -> Vec<(u32, u8, u8)> {
    let mut looks = Vec::new();
    for r in RACES.iter().filter(|r| r.classes.contains(&class.id)) {
        for sex in 0..=1 {
            let display = 300000 + u32::from(class.id) * 200 + u32::from(r.id) * 2 + u32::from(sex);
            if creatures.display_race_sex(display) == Some((r.id, sex)) {
                looks.push((display, r.id, sex));
            } else if let Some(&display) = class
                .looks
                .iter()
                .find(|&&d| creatures.display_race_sex(d) == Some((r.id, sex)))
            {
                looks.push((display, r.id, sex));
            }
        }
    }
    looks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_playable_race_has_traits_and_its_proc_exists() {
        for race in RACES {
            let r = racial(race.id).unwrap_or_else(|| panic!("{} has no traits", race.name));
            assert!(
                trait_lines(race.id).len() >= 2,
                "{} lists fewer than two traits",
                race.name
            );
            if let Some((_, key, _)) = r.proc {
                super::super::data::ability(key);
            }
        }
    }

    #[test]
    fn bonuses_follow_the_original_rows() {
        assert_eq!(bonus(1, Bonus::Experience), 0.10);
        assert_eq!(bonus(1, Bonus::Gold), 0.20);
        assert_eq!(bonus(35, Bonus::Pickup), 0.35);
        assert_eq!(bonus(8, Bonus::Regen), 1.5);
        assert_eq!(bonus(6, Bonus::Damage), 0.0);
        assert_eq!(
            trait_lines(5),
            [
                "Will of the Forsaken: near death, 4 sec of immunity (60 sec cooldown)",
                "Cannibalize: every kill heals 1 health",
                "Shadow Resistance: +4% damage reduction",
            ]
        );
    }
}
