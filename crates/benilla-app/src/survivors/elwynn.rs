//! Elwynn's source-table identities and entry times, with Hogger as a boss and Fedfennel as an
//! elite. The ability scripts below are the existing port, not yet full native script parity.
//! Source zone record 0 holds enemies at +120 and bosses at +136.

use super::data::{BossAction, BossPhase, EnemyDef, MapDef, Ranged, Roster};

/// One of the old table's creatures: `(name, display, scale, health, damage, swing, speed, xp,
/// minute)`.
type Row = (&'static str, u32, f32, f32, f32, f32, f32, u32, f32);

const fn foe(r: Row, level: u32) -> EnemyDef {
    let (name, display, scale, hp, damage, attack, speed, xp, minute) = r;
    EnemyDef {
        scale,
        attack,
        from: minute * 60.0,
        ..super::data::en(name, display, hp, speed, damage, xp, level)
    }
}

const fn caster(r: Row, level: u32, spell: u32, cooldown: f32) -> EnemyDef {
    EnemyDef {
        ranged: Some(Ranged {
            spell,
            range: 18.0,
            cooldown,
        }),
        ..foe(r, level)
    }
}

/// The horde, in the order it arrives.
static HORDE: [EnemyDef; 9] = [
    foe(("Kobold Vermin", 26, 0.9, 24.0, 3.0, 1.6, 5.0, 1, 0.0), 1),
    foe(("Young Wolf", 165, 0.7, 18.0, 3.0, 1.3, 7.4, 1, 0.5), 2),
    foe(
        ("Murloc Streamrunner", 31, 1.0, 30.0, 4.0, 1.6, 5.6, 1, 1.2),
        3,
    ),
    // Fireball.
    caster(
        ("Kobold Geomancer", 163, 1.0, 30.0, 6.0, 3.2, 5.0, 2, 2.0),
        4,
        133,
        1.8,
    ),
    foe(("Defias Thug", 2323, 1.0, 44.0, 6.0, 1.8, 5.4, 2, 3.0), 5),
    foe(
        ("Riverpaw Brute", 175, 1.1, 120.0, 9.0, 2.2, 4.6, 4, 4.5),
        6,
    ),
    foe(("Prowler", 644, 1.3, 50.0, 7.0, 1.3, 7.6, 2, 6.0), 7),
    // Frostbolt.
    caster(
        (
            "Defias Rogue Wizard",
            2315,
            1.0,
            40.0,
            7.0,
            3.2,
            5.0,
            3,
            7.5,
        ),
        8,
        116,
        1.6,
    ),
    foe(
        ("Defias Enforcer", 2324, 1.25, 170.0, 12.0, 2.2, 4.8, 4, 9.0),
        9,
    ),
];

/// `Spell.dbc` War Stomp, the old table's stomp.
const WAR_STOMP: u32 = 20549;
/// `Spell.dbc` Explosive Trap Effect, the old table's Defias bombs.
const BOMB: u32 = 13812;

const fn boss(r: Row, level: u32, phases: &'static [BossPhase]) -> EnemyDef {
    EnemyDef {
        phases,
        ..foe(r, level)
    }
}

/// Hogger retains the reference's 3:00 boss event; Fedfennel occupies the fourth elite slot.
static BOSSES: [EnemyDef; 4] = [
    EnemyDef {
        intro: "Grrr... fresh meat!",
        ..boss(
            ("Hogger", 384, 3.5, 3600.0, 22.0, 1.0, 5.6, 35, 3.0),
            11,
            &[
                BossPhase {
                    below: 1.0,
                    yell: "",
                    action: BossAction::Throw {
                        spell: ROCK,
                        radius: 4.0,
                        damage: 18.0,
                        every: 7.0,
                    },
                },
                BossPhase {
                    below: 1.0,
                    yell: "",
                    action: BossAction::Charge {
                        spell: CHARGE,
                        damage: 20.0,
                        every: 10.0,
                    },
                },
                BossPhase {
                    below: 1.0,
                    yell: "",
                    action: BossAction::Stomp {
                        spell: WAR_STOMP,
                        radius: 7.0,
                        damage: 18.0,
                        every: 11.0,
                    },
                },
                BossPhase {
                    below: 0.5,
                    yell: "Hogger is enraged! Grrrr!",
                    action: BossAction::Stomp {
                        spell: WAR_STOMP,
                        radius: 8.0,
                        damage: 22.0,
                        every: 7.0,
                    },
                },
            ],
        )
    },
    EnemyDef {
        intro: "Mother Fang skitters out of the mine!",
        ..boss(
            ("Mother Fang", 7536, 0.83, 9000.0, 30.0, 1.0, 6.0, 60, 6.0),
            10,
            &[
                // Spits venom at the hero's feet and pounces down a lane from the start.
                BossPhase {
                    below: 1.0,
                    yell: "",
                    action: BossAction::Throw {
                        spell: VENOM_SPIT,
                        radius: 4.0,
                        damage: 16.0,
                        every: 5.0,
                    },
                },
                BossPhase {
                    below: 1.0,
                    yell: "",
                    action: BossAction::Charge {
                        spell: RUSHING_CHARGE,
                        damage: 22.0,
                        every: 9.0,
                    },
                },
                // At half, poison pools fall around the hero to break up his kiting.
                BossPhase {
                    below: 0.5,
                    yell: "Mother Fang hisses, and the ground seeps with venom!",
                    action: BossAction::Bombs {
                        spell: POISON_CLOUD,
                        count: 4,
                        radius: 3.5,
                        damage: 14.0,
                        every: 8.0,
                    },
                },
            ],
        )
    },
    EnemyDef {
        intro: "Princess joins the battle!",
        ..boss(
            ("Princess", 703, 4.46, 28000.0, 46.0, 1.0, 6.2, 100, 12.0),
            11,
            &[
                BossPhase {
                    below: 0.66,
                    yell: "You landlubbers are tougher than I thought! I'll have to improvise!",
                    action: BossAction::Stomp {
                        spell: WHIRLWIND,
                        radius: 5.0,
                        damage: 16.0,
                        every: 2.5,
                    },
                },
                BossPhase {
                    below: 0.33,
                    yell: "D'ah! Now you're making me angry!",
                    action: BossAction::Stomp {
                        spell: WAR_STOMP,
                        radius: 9.0,
                        damage: 69.0,
                        every: 13.0,
                    },
                },
            ],
        )
    },
    EnemyDef {
        last: true,
        intro: "Morgan the Collector joins the battle!",
        ..boss(
            (
                "Morgan the Collector",
                2322,
                8.24,
                52500.0,
                54.0,
                1.0,
                6.6,
                150,
                16.0,
            ),
            12,
            &[
                BossPhase {
                    below: 0.5,
                    yell: "Fools! Our cause is righteous!",
                    action: BossAction::Summon {
                        foe: "Defias Thug",
                        count: 5,
                        every: 0.0,
                    },
                },
                BossPhase {
                    below: 0.25,
                    yell: "The Brotherhood shall prevail!",
                    action: BossAction::Bombs {
                        spell: BOMB,
                        count: 6,
                        radius: 4.0,
                        damage: 10.0,
                        every: 2.4,
                    },
                },
            ],
        )
    },
];

/// `Spell.dbc` Mother Fang's Venom Spit and Poison Cloud.
const VENOM_SPIT: u32 = 6917;
const POISON_CLOUD: u32 = 3815;

/// `Spell.dbc` Whirlwind, Princess's spin.
const WHIRLWIND: u32 = 1680;

/// `Spell.dbc` Hogger's Throw Rock.
const ROCK: u32 = 4164;
/// `Spell.dbc` Mother Fang's Rushing Charge and Hogger's Charge.
const RUSHING_CHARGE: u32 = 6268;
const CHARGE: u32 = 100;

/// Elite base stats; lair availability and kits are independent of enemy unlock times.
static GUARDIANS: [EnemyDef; 4] = [
    EnemyDef {
        from: 210.0,
        ..foe(
            (
                "Kobold Taskmaster",
                163,
                2.03,
                600.0,
                18.0,
                2.0,
                5.4,
                25,
                0.0,
            ),
            10,
        )
    },
    EnemyDef {
        from: 480.0,
        ..foe(
            ("Gruff Swiftbite", 246, 1.62, 800.0, 22.0, 1.6, 6.6, 28, 0.0),
            10,
        )
    },
    foe(
        ("Fedfennel", 204, 2.98, 17000.0, 38.0, 1.0, 5.4, 80, 9.0),
        11,
    ),
    EnemyDef {
        from: 780.0,
        ..foe(
            ("Bellygrub", 377, 2.97, 1000.0, 26.0, 2.4, 5.0, 30, 0.0),
            11,
        )
    },
];

/// Elwynn's roster, for its map; any other battleground keeps the mode's own.
pub(super) fn roster(map: &MapDef) -> Option<Roster> {
    (map.name == "Elwynn Forest").then(|| Roster {
        enemies: HORDE.to_vec(),
        bosses: BOSSES.to_vec(),
        timed: true,
        guardians: GUARDIANS.to_vec(),
    })
}

/// Whether a definition belongs to Elwynn's regular horde.
pub(super) fn is_regular(def: &EnemyDef) -> bool {
    regular_index(def).is_some()
}

pub(super) fn regular_index(def: &EnemyDef) -> Option<usize> {
    HORDE
        .iter()
        .position(|entry| entry.name == def.name && entry.display == def.display)
}

/// Whether a definition is one of the source's boss rows, Fedfennel included.
pub(super) fn is_boss(def: &EnemyDef) -> bool {
    BOSSES
        .iter()
        .chain(std::iter::once(&GUARDIANS[2]))
        .any(|entry| entry.name == def.name && entry.display == def.display)
}

/// The three elite enemy-table rows also spawn as roaming chasers, independently of lairs.
pub(super) fn is_roaming_elite(def: &EnemyDef) -> bool {
    [0, 1, 3].iter().any(|&i| {
        GUARDIANS[i].name == def.name && GUARDIANS[i].display == def.display
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hogger_is_the_opening_boss_and_morgan_ends_the_run() {
        assert_eq!(
            BOSSES.map(|d| d.name),
            ["Hogger", "Mother Fang", "Princess", "Morgan the Collector"]
        );
        assert_eq!(BOSSES.map(|d| d.from), [180.0, 360.0, 720.0, 960.0]);
        assert_eq!(BOSSES[0].hp, 3600.0);
        assert!(BOSSES[3].last);
        assert!(BOSSES[..3].iter().all(|d| !d.last));
    }

    #[test]
    fn four_elites_keep_entry_times_and_fedfennels_source_stats() {
        assert_eq!(
            GUARDIANS.map(|d| d.name),
            [
                "Kobold Taskmaster",
                "Gruff Swiftbite",
                "Fedfennel",
                "Bellygrub"
            ]
        );
        assert_eq!(GUARDIANS.map(|d| d.from), [210.0, 480.0, 540.0, 780.0]);
        assert_eq!(GUARDIANS[2].hp, 17000.0);
        assert_eq!(GUARDIANS[2].damage, 38.0);
        assert!(GUARDIANS.iter().all(|d| !d.last));
        assert!(HORDE.iter().all(is_regular));
        assert!(BOSSES.iter().all(|d| !is_regular(d)));
    }
}
