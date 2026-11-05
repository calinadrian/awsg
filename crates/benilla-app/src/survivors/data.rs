//! The mode's tables: classes, abilities, passives, enemies and the wave clock. Spell ids name
//! `Spell.dbc` rows only for their icon, name and visual chain; every number here is the mode's.

/// A weapon on the hero's virtual item slots: `(slot, display, class, subclass, material, inv,
/// sheath)`, the creature form of `UNIT_VIRTUAL_ITEM_SLOT_DISPLAY` and `_INFO`.
pub(crate) type Weapon = (u16, u32, u8, u8, u8, u8, u8);

pub(crate) struct ClassDef {
    pub id: u8,
    pub name: &'static str,
    /// `RAID_CLASS_COLORS`, 0xRRGGBB.
    pub color: u32,
    /// Outgoing damage tuning for the damage-only six-spell benchmark (L1 Common and L8 Legendary,
    /// one and ten targets); class talent procs are outside that baseline.
    pub damage_scale: f32,
    /// `UNIT_FIELD_BYTES_0` byte 3.
    pub power: u8,
    pub start: &'static str,
    pub pool: &'static [&'static str],
    /// Class-trainer `CreatureDisplayInfo` ids, deduped by race and sex at the menu.
    pub looks: &'static [u32],
    pub weapons: &'static [Weapon],
    /// The sheath state the hero fights in: 1 melee, 2 ranged.
    pub sheath: u8,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Kind {
    /// A missile at the nearest enemies (or instant if the spell has no speed).
    Bolt,
    /// Random missiles, with replacement; bosses and elites form the pool when in range.
    Missiles,
    /// A finishing strike, doubled below 30% health.
    Execute,
    /// A temporary damage or cooldown-recovery buff.
    Buff,
    /// A class-specific event-driven spell.
    Passive,
    /// An instant hit on the nearest enemies in range.
    Strike,
    /// A close hit on the nearest enemies in melee reach.
    Melee,
    /// Everything in a radius around the hero.
    Nova,
    /// Everything in a cone toward the nearest enemy.
    Cone,
    /// A ground area at the densest nearby enemy, ticking for `duration`.
    Ground,
    /// A ground area under the hero, ticking for `duration`.
    GroundSelf,
    /// Jumps from enemy to enemy.
    Chain,
    /// A damage-over-time on several enemies.
    Dot,
    /// Always on: pulses every `cooldown` around the hero, with the spell's state visual on him.
    Aura,
    /// Absorbs `damage` for `duration`.
    Shield,
    /// Immune for `duration`.
    Immune,
    /// Returns `damage` to every attacker for `duration`.
    Thorns,
    /// Heals `damage` over `duration`.
    Heal,
    /// Drops a totem that casts `sub` for `duration`.
    Totem,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Effect {
    None,
    /// Speed multiplier for seconds.
    Slow(f32, f32),
    Root(f32),
    Stun(f32),
    Fear(f32),
    /// Outgoing damage multiplier for seconds.
    Weaken(f32, f32),
    /// Heal the hero by this fraction of damage dealt.
    Drain(f32),
    /// Heal by a share of maximum health for each enemy hit.
    Heal(f32),
    /// Absorb a share of maximum health for each enemy hit.
    Shield(f32),
}

pub(crate) struct AbilityDef {
    pub key: &'static str,
    pub spell: u32,
    pub kind: Kind,
    /// Direct damage, total periodic damage, or a self effect's heal/absorb/buff coefficient.
    pub damage: f32,
    /// Damage over time, per second, for `duration`.
    pub dot: f32,
    pub cooldown: f32,
    pub range: f32,
    /// Splash for single hits, area for the area kinds, the jump for a chain.
    pub radius: f32,
    pub count: u32,
    pub duration: f32,
    pub effect: Effect,
    /// The spell whose state kit marks an affected enemy, 0 for none.
    pub mark: u32,
    /// A totem's display, and the spell it casts.
    pub totem: (u32, u32),
    /// The area's own visual hangs overhead and would hide the field from the camera, so the
    /// area plays the spell's impact on whatever it hits instead.
    pub overhead: bool,
    pub ticks: u32,
    pub speed: f32,
    pub school: u8,
    pub native_kind: u8,
    pub self_effect: u32,
}

const fn ab(key: &'static str, spell: u32, kind: Kind) -> AbilityDef {
    AbilityDef {
        key,
        spell,
        kind,
        damage: 0.0,
        dot: 0.0,
        cooldown: 2.0,
        range: 20.0,
        radius: 0.0,
        count: 1,
        duration: 0.0,
        effect: Effect::None,
        mark: 0,
        totem: (0, 0),
        overhead: false,
        ticks: 0,
        speed: 0.0,
        school: 0,
        native_kind: 255,
        self_effect: 0,
    }
}

macro_rules! ability {
    ($key:literal, $spell:literal, $kind:ident { $($f:ident: $v:expr),* $(,)? }) => {
        AbilityDef { $($f: $v,)* ..ab($key, $spell, Kind::$kind) }
    };
}

include!("spell_data.rs");

// Compatibility definitions for talent-granted spells outside the original class pools.
static LEGACY_ABILITIES: &[AbilityDef] = &[
    ability!(
        "heroic_strike",
        78,
        Melee {
            damage: 28.0,
            cooldown: 1.6,
            range: 5.5
        }
    ),
    ability!(
        "sweeping_strikes",
        12292,
        Aura {
            damage: 6.0,
            cooldown: 0.6,
            radius: 5.5
        }
    ),
    ability!(
        "retaliation",
        20230,
        Thorns {
            damage: 14.0,
            cooldown: 18.0,
            duration: 8.0
        }
    ),
    ability!(
        "shadowburn",
        17877,
        Strike {
            damage: 46.0,
            cooldown: 5.0,
            range: 20.0
        }
    ),
];

// Racial procs the original names in its race rows but has no spell row for.
static RACIAL_ABILITIES: &[AbilityDef] = &[ability!(
    "arcane_torrent",
    28730,
    Nova {
        damage: 6.0,
        cooldown: 20.0,
        range: 0.0,
        radius: 8.0,
        count: 0,
        school: 6,
        native_kind: 2,
        effect: Effect::Stun(2.0),
        mark: 20549
    }
)];

pub(crate) fn every_ability() -> impl Iterator<Item = &'static AbilityDef> {
    ABILITIES
        .iter()
        .chain(LEGACY_ABILITIES.iter())
        .chain(RACIAL_ABILITIES.iter())
}

pub(crate) fn ability(key: &str) -> &'static AbilityDef {
    every_ability()
        .find(|a| a.key == key)
        .unwrap_or_else(|| panic!("survivors: no ability {key:?}"))
}

pub(crate) const MAX_ABILITY_LEVEL: u8 = 8;
pub(crate) const MAX_ABILITIES: usize = 6;

pub(crate) static CLASSES: &[ClassDef] = &[
    ClassDef {
        id: 1,
        name: "Warrior",
        color: 0xC79C6E,
        damage_scale: 0.81,
        power: 1,
        start: "cleave",
        pool: &[
            "cleave",
            "whirlwind",
            "thunder_clap",
            "mortal_strike",
            "execute",
            "demoralizing_shout",
            "battle_shout",
            "bloodthirst",
            "rend",
            "shield_slam",
            "bloodrage",
        ],
        looks: &[
            1300, 1374, 1375, 1504, 1578, 1599, 1707, 1721, 1880, 2096, 2103, 2113, 2196, 2198,
            2614, 2620, 2658, 3053, 3054, 3055, 3280, 3287, 3343, 3399, 3431, 3743, 3793, 3794,
            4242, 4556, 6071, 11037,
        ],
        weapons: &[(0, 7483, 2, 7, 1, 13, 3), (1, 1706, 4, 6, 1, 14, 4)],
        sheath: 1,
    },
    ClassDef {
        id: 2,
        name: "Paladin",
        color: 0xF58CBA,
        damage_scale: 1.05,
        power: 0,
        start: "consecration",
        pool: &[
            "consecration",
            "exorcism",
            "hammer_of_wrath",
            "holy_shock",
            "holy_wrath",
            "judgement",
            "hammer_of_justice",
            "holy_light",
            "divine_shield",
            "holy_shield",
            "redemption",
        ],
        looks: &[
            1299, 1499, 1622, 3087, 3088, 3089, 3284, 3289, 3346, 3393, 7356,
        ],
        weapons: &[(0, 8011, 2, 4, 2, 13, 3), (1, 1705, 4, 6, 1, 14, 4)],
        sheath: 1,
    },
    ClassDef {
        id: 3,
        name: "Hunter",
        color: 0xABD473,
        damage_scale: 0.86,
        power: 0,
        start: "auto_shot",
        pool: &[
            "auto_shot",
            "arcane_shot",
            "multi_shot",
            "serpent_sting",
            "volley",
            "explosive_trap",
            "concussive_shot",
            "aimed_shot",
            "immolation_trap",
            "frost_trap",
            "hunters_mark",
        ],
        looks: &[
            1373, 1703, 1723, 1882, 2066, 2087, 2105, 2112, 2205, 2206, 2251, 3056, 3072, 3073,
            3299, 3309, 3310, 3312, 3395, 3558, 3744, 3810, 3811, 4239, 4241, 4372, 4560, 7538,
            10245,
        ],
        weapons: &[(0, 7480, 2, 6, 1, 17, 2), (2, 6235, 2, 2, 2, 15, 0)],
        sheath: 2,
    },
    ClassDef {
        id: 4,
        name: "Rogue",
        color: 0xFFF569,
        damage_scale: 1.11,
        power: 3,
        start: "sinister_strike",
        pool: &[
            "sinister_strike",
            "eviscerate",
            "blade_flurry",
            "rupture",
            "kidney_shot",
            "gouge",
            "ghostly_strike",
            "adrenaline_rush",
            "backstab",
            "slice_and_dice",
            "seal_fate",
        ],
        looks: &[
            1297, 1327, 1328, 1507, 1580, 1603, 1704, 1725, 1886, 2231, 2243, 2252, 2631, 2639,
            2659, 3100, 3101, 3113, 3351, 3407, 3436, 3749, 4360, 5146, 5528, 13171,
        ],
        weapons: &[(0, 7492, 2, 7, 1, 13, 3), (1, 6443, 2, 15, 1, 13, 3)],
        sheath: 1,
    },
    ClassDef {
        id: 5,
        name: "Priest",
        color: 0xFFFFFF,
        damage_scale: 1.11,
        power: 0,
        start: "smite",
        pool: &[
            "smite",
            "shadow_word_pain",
            "mind_blast",
            "holy_nova",
            "power_word_shield",
            "renew",
            "holy_fire",
            "mind_flay",
            "psychic_scream",
            "devouring_plague",
            "lightwell",
        ],
        looks: &[
            1295, 1495, 1579, 1602, 1708, 1733, 1897, 2137, 2138, 2139, 2200, 2201, 2202, 2618,
            2626, 3066, 3085, 3086, 3282, 3283, 3344, 3401, 3429, 4068, 4690, 4711, 10473, 10723,
            11044, 11048, 11053,
        ],
        weapons: &[(0, 1926, 2, 10, 2, 17, 2)],
        sheath: 1,
    },
    ClassDef {
        id: 7,
        name: "Shaman",
        color: 0x0070DE,
        damage_scale: 0.95,
        power: 0,
        start: "lightning_bolt",
        pool: &[
            "lightning_bolt",
            "chain_lightning",
            "earth_shock",
            "flame_shock",
            "frost_shock",
            "lightning_shield",
            "magma_totem",
            "searing_totem",
            "fire_nova",
            "healing_wave",
            "call_of_flame",
        ],
        looks: &[
            1360, 1878, 2082, 2102, 2123, 3746, 3816, 4231, 4552, 10180, 13341,
        ],
        weapons: &[(0, 7477, 2, 4, 2, 13, 3), (1, 1705, 4, 6, 1, 14, 4)],
        sheath: 1,
    },
    ClassDef {
        id: 8,
        name: "Mage",
        color: 0x69CCF0,
        damage_scale: 1.01,
        power: 0,
        start: "fireball",
        pool: &[
            "fireball",
            "frostbolt",
            "arcane_missiles",
            "frost_nova",
            "arcane_explosion",
            "blizzard",
            "flamestrike",
            "fire_blast",
            "cone_of_cold",
            "blast_wave",
            "pyroblast",
            "ice_barrier",
            "polymorph",
        ],
        looks: &[
            1294, 1470, 1484, 1592, 1600, 2134, 2135, 2644, 2657, 2810, 3108, 3109, 3292, 3293,
            4522, 4523, 4524, 4526, 4665, 5001, 6058, 6060, 6072, 7669, 10171, 10214, 10215, 10216,
            10548, 10733, 12849,
        ],
        weapons: &[(0, 1927, 2, 10, 2, 17, 2)],
        sheath: 1,
    },
    ClassDef {
        id: 9,
        name: "Warlock",
        color: 0x9482C9,
        damage_scale: 1.37,
        power: 0,
        start: "shadow_bolt",
        pool: &[
            "shadow_bolt",
            "corruption",
            "immolate",
            "curse_of_agony",
            "rain_of_fire",
            "hellfire",
            "death_coil",
            "searing_pain",
            "drain_life",
            "fear",
            "soul_fire",
            "demonic_empowerment",
        ],
        looks: &[
            1324, 1325, 1326, 1469, 1581, 1604, 1884, 1930, 2637, 2646, 2675, 3115, 3116, 3122,
            3271, 3286, 3291, 3345, 3607, 3745, 4567,
        ],
        weapons: &[(0, 5542, 2, 10, 2, 17, 2)],
        sheath: 1,
    },
    ClassDef {
        id: 11,
        name: "Druid",
        color: 0xFF7D0A,
        damage_scale: 0.93,
        power: 0,
        start: "wrath",
        pool: &[
            "wrath",
            "moonfire",
            "starfire",
            "hurricane",
            "insect_swarm",
            "entangling_roots",
            "rejuvenation",
            "regrowth",
            "thorns",
            "swipe",
            "faerie_fire",
            "shred",
        ],
        looks: &[
            1706, 1732, 2106, 2115, 2121, 2250, 2255, 2261, 3300, 3301, 3302, 3819, 3820, 7357,
            10738, 12053,
        ],
        weapons: &[(0, 22391, 2, 10, 2, 17, 2)],
        sheath: 1,
    },
];

#[derive(Clone, Copy)]
pub(crate) struct EnemyDef {
    pub name: &'static str,
    pub display: u32,
    pub hp: f32,
    pub speed: f32,
    pub damage: f32,
    pub scale: f32,
    pub xp: u32,
    pub level: u32,
    /// A great beast, which takes more room in the crowd.
    pub big: bool,
    /// An elite row that may join the horde independently of the lair.
    pub roaming_elite: bool,
    /// Seconds between swings; 0 for the mode's own pace.
    pub attack: f32,
    /// On a timed roster, the run second it first joins the horde (a boss: the second it comes).
    pub from: f32,
    /// A caster's bolt, which it throws from range instead of closing in.
    pub ranged: Option<Ranged>,
    /// A timed roster's last boss: slaying it wins the run.
    pub last: bool,
    /// What a boss does as its health falls.
    pub phases: &'static [BossPhase],
    /// A boss's entrance line, or a guardian's when it wakes; empty for the mode's own.
    pub intro: &'static str,
    /// A horde row's price in the spawner's budget; 0 for bosses and elites.
    pub cost: f32,
    /// The guardian kit it fights with as an elite (see `guardian_behavior`); 0 for none.
    pub kit: u8,
    /// A lair lord keeps a boss row's numbers and holds its lair once, from its own minute.
    pub lord: bool,
    /// Its battleground's tier, 1 (a starting zone) to 5 (where the roads meet).
    pub tier: u8,
    /// A boss's script from the mechanics library, past Elwynn's native kits.
    pub script: Option<&'static super::mechanics::Kit>,
    /// How many affixes it rolls as an elite; Elwynn's roll none.
    pub affixes: u8,
}

/// A caster's spell: the `Spell.dbc` row it uses, its reach and how often it casts.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ranged {
    pub spell: u32,
    pub range: f32,
    pub cooldown: f32,
}

/// A boss's turn at a share of its health: what it yells and what it does from then on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BossPhase {
    /// The share of its health it turns at.
    pub below: f32,
    pub yell: &'static str,
    pub action: BossAction,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum BossAction {
    /// Every `every` seconds, strikes everything within `radius` for `damage` (scaled with the run).
    Stomp {
        spell: u32,
        radius: f32,
        damage: f32,
        every: f32,
    },
    /// Calls `count` of the roster's `foe` to its side, then again every `every` seconds (0: once);
    /// an empty `foe` calls whatever the horde is sending now.
    Summon {
        foe: &'static str,
        count: u32,
        every: f32,
    },
    /// Every `every` seconds, stands a moment with a lane marked ahead of it, then rushes down it
    /// through where the hero stood, striking him once for `damage` if he is still in the way.
    Charge { spell: u32, damage: f32, every: f32 },
    /// Every `every` seconds, throws at the spot the hero stands on, marked while it flies;
    /// whatever is within `radius` when it lands takes `damage`.
    Throw {
        spell: u32,
        radius: f32,
        damage: f32,
        every: f32,
    },
    /// Every `every` seconds, `count` blasts fall around the hero, each `radius` across.
    Bombs {
        spell: u32,
        count: u32,
        radius: f32,
        damage: f32,
        every: f32,
    },
}

pub(super) const fn en(
    name: &'static str,
    display: u32,
    hp: f32,
    speed: f32,
    damage: f32,
    xp: u32,
    level: u32,
) -> EnemyDef {
    EnemyDef {
        name,
        display,
        hp,
        speed,
        damage,
        scale: 1.0,
        xp,
        level,
        big: false,
        roaming_elite: false,
        attack: 0.0,
        from: 0.0,
        ranged: None,
        last: false,
        phases: &[],
        intro: "",
        cost: 0.0,
        kit: 0,
        lord: false,
        tier: 1,
        script: None,
        affixes: 0,
    }
}

/// The Barrens, roughly in the order a character meets them.
pub(crate) static ENEMIES: &[EnemyDef] = &[
    en("Fleeting Plainstrider", 1284, 8.0, 3.6, 2.0, 1, 10),
    en("Zhevra Runner", 6087, 10.0, 4.4, 2.0, 1, 11),
    en("Razormane Quilboar", 1218, 16.0, 3.4, 3.0, 2, 12),
    en("Razormane Hunter", 6094, 18.0, 3.6, 4.0, 2, 12),
    en("Savannah Huntress", 1056, 22.0, 4.8, 8.0, 2, 13),
    en("Sunscale Lashtail", 1744, 26.0, 4.6, 8.0, 3, 13),
    en("Kolkar Wrangler", 9442, 36.0, 4.0, 10.0, 3, 14),
    en("Kolkar Stormer", 9443, 40.0, 4.0, 11.0, 3, 14),
    en("Witchwing Harpy", 3218, 34.0, 5.0, 10.0, 3, 15),
    en("Hecklefang Hyena", 2710, 44.0, 5.2, 12.0, 4, 15),
    en("Razormane Defender", 1253, 70.0, 3.3, 14.0, 5, 16),
    en("Razormane Mystic", 4643, 55.0, 3.8, 12.0, 4, 16),
    en("Sunscale Screecher", 1747, 60.0, 5.0, 13.0, 4, 17),
    en("Kolkar Bloodcharger", 9447, 85.0, 4.6, 16.0, 5, 18),
    en("Savannah Prowler", 1973, 75.0, 5.4, 15.0, 5, 18),
    en("Thunderhawk Hatchling", 1742, 90.0, 4.2, 16.0, 6, 19),
    en("Witchwing Slayer", 2163, 95.0, 5.0, 18.0, 6, 20),
    en("Bristleback Geomancer", 6091, 120.0, 3.6, 20.0, 7, 21),
    huge(en("Stormsnout", 1537, 150.0, 4.0, 22.0, 8, 22)),
    huge(en("Barrens Kodo", 1453, 220.0, 3.2, 28.0, 12, 23)),
    en("Sunscale Scytheclaw", 4442, 160.0, 5.6, 24.0, 8, 24),
    huge(en("Thunderhead", 1538, 180.0, 4.2, 26.0, 10, 25)),
    en("Silithid Swarmer", 2731, 90.0, 6.0, 18.0, 5, 25),
    huge(en("Greater Thunderhawk", 1974, 240.0, 4.4, 30.0, 12, 26)),
    en("Hecklefang Stalker", 2712, 200.0, 6.0, 30.0, 10, 27),
    en("Kolkar Destroyer", 9446, 280.0, 4.6, 34.0, 14, 28),
    en("Kolkar Mauler", 4874, 300.0, 4.6, 36.0, 14, 29),
    huge(en("Greater Barrens Kodo", 1232, 420.0, 3.4, 42.0, 24, 30)),
    huge(en("Elder Thunder Lizard", 2764, 380.0, 4.4, 44.0, 22, 31)),
    huge(en("Silithid Protector", 1307, 360.0, 5.0, 40.0, 20, 32)),
];

/// The great beasts at three quarters size, so a crowd of them still leaves the field readable.
const fn huge(def: EnemyDef) -> EnemyDef {
    EnemyDef {
        scale: 0.75,
        big: true,
        ..def
    }
}

pub(crate) static BOSSES: &[EnemyDef] = &[
    EnemyDef {
        scale: 1.6,
        ..en("Echeyakee", 1934, 700.0, 5.2, 20.0, 60, 16)
    },
    EnemyDef {
        scale: 1.6,
        ..en("Takk the Leaper", 1337, 1800.0, 5.4, 32.0, 120, 19)
    },
    EnemyDef {
        scale: 1.7,
        ..en("Lakota'mani", 1241, 4200.0, 4.0, 45.0, 200, 22)
    },
    EnemyDef {
        scale: 1.7,
        ..en("Humar the Pridelord", 4424, 7000.0, 5.6, 55.0, 300, 26)
    },
    EnemyDef {
        scale: 1.8,
        ..en("Agathelos the Raging", 2450, 12000.0, 5.0, 70.0, 400, 33)
    },
    EnemyDef {
        scale: 1.8,
        ..en("Hezrul Bloodmark", 9448, 20000.0, 4.8, 90.0, 500, 33)
    },
];

/// Original ASR stops awarding levels at 99 (RVA 0x5bfb6e0).
pub(crate) const MAX_LEVEL: u32 = 99;

/// A boss's scripted moves hit harder as the run goes on: +90% by fifteen minutes.
pub(crate) fn damage_scale(elapsed: f32) -> f32 {
    1.0 + (elapsed / 900.0).max(0.0) * 0.9
}

/// Original ASR's rounded quadratic threshold (RVA 0x5bfb69c..0x5bfb6c7).
pub(crate) fn xp_to_next(level: u32) -> f32 {
    let n = level.saturating_sub(1) as f32;
    (0.35 * n * n + (4.0 * n + 5.0)).round()
}

/// A creature on a battleground's roster: who it is, its pace and its size. Its health, damage
/// and experience come from its place on the roster, read off The Barrens' tuned ladder.
#[derive(Clone, Copy)]
pub(crate) struct Foe {
    name: &'static str,
    display: u32,
    level: u32,
    speed: f32,
    /// The drawn scale, chosen against the model's Stand height so the field reads as The
    /// Barrens does: about 2-3.5 yd for the horde, 4-5 for the great beasts, 5-9 for a boss.
    scale: f32,
    big: bool,
}

const fn foe(name: &'static str, display: u32, level: u32, speed: f32, scale: f32) -> Foe {
    Foe {
        name,
        display,
        level,
        speed,
        scale,
        big: false,
    }
}

const fn giant(name: &'static str, display: u32, level: u32, speed: f32, scale: f32) -> Foe {
    Foe {
        big: true,
        ..foe(name, display, level, speed, scale)
    }
}

/// Where a run is fought.
#[derive(Clone, Copy)]
pub(crate) struct MapDef {
    pub name: &'static str,
    /// The zone or continent under the name on the menu.
    pub zone: &'static str,
    /// The `Map.dbc` row.
    pub map: u32,
    /// Where the hero starts (WoW coordinates).
    pub center: [f32; 3],
    /// How far from the centre the hero may walk.
    pub leash: f32,
    /// The results screen's line on a win.
    pub victory: &'static str,
    /// Weakest first; empty for The Barrens, which keeps its hand-tuned [`ENEMIES`].
    foes: &'static [Foe],
    /// One every [`BOSS_EVERY_SECS`], in order; empty for The Barrens' [`BOSSES`].
    bosses: &'static [Foe],
}

impl MapDef {
    /// The boss line-up, by name, for a battleground that may not be playable yet.
    pub(crate) fn boss_names(&self) -> impl Iterator<Item = &'static str> {
        self.bosses.iter().map(|b| b.name)
    }
}

/// Every battleground defined, playable or not: the adventure atlas lists the locked ones too.
pub(crate) fn all_maps() -> &'static [MapDef] {
    &ALL_MAPS
}

static STORY_MAPS: &[MapDef] = include!("story_maps.rs");
static ALL_MAPS: std::sync::LazyLock<Vec<MapDef>> = std::sync::LazyLock::new(|| {
    ORIGINAL_MAPS.iter().chain(STORY_MAPS).copied().collect()
});

/// Stable menu/save/co-op indices: append new maps so existing picks keep their meaning.
pub(crate) static MAPS: std::sync::LazyLock<Vec<MapDef>> = std::sync::LazyLock::new(|| {
    asr_coop::story::ARENAS.iter().copied()
        .map(|name| {
            *ALL_MAPS
                .iter()
                .find(|map| map.name == name)
                .expect("arena definition")
        })
        .collect()
});

static ORIGINAL_MAPS: &[MapDef] = &[
    MapDef {
        name: "The Barrens",
        zone: "Kalimdor",
        map: 1,
        // The old ASR's circle for this zone, exactly as its story chapters placed it.
        center: [-800.0, -2600.0, 94.15],
        leash: 150.0,
        victory: "The Barrens are yours.",
        foes: &[],
        bosses: &[],
    },
    MapDef {
        name: "The Dark Portal",
        zone: "Blasted Lands",
        map: 0,
        // The basin before the Portal, ringed by cliffs.
        center: [-11840.0, -3197.0, -30.0],
        leash: 75.0,
        victory: "The Dark Portal stands silent.",
        foes: &[
            foe("Starving Snickerfang", 8050, 45, 4.6, 1.0),
            foe("Snickerfang Hyena", 2714, 49, 5.0, 1.15),
            foe("Black Slayer", 10824, 47, 5.0, 0.6),
            foe("Redstone Crystalhide", 798, 51, 3.6, 1.3),
            foe("Shadowsworn Cultist", 7838, 51, 3.8, 1.3),
            foe("Felbeast", 7949, 50, 4.8, 0.85),
            foe("Dreadmaul Mauler", 14401, 53, 3.6, 1.1),
            foe("Felhound", 1913, 54, 5.2, 1.0),
            foe("Felguard Sentry", 9017, 55, 4.2, 0.9),
            foe("Manahound", 6173, 60, 5.6, 1.15),
            giant("Lesser Infernal", 10906, 55, 3.4, 1.0),
            foe("Felguard Elite", 7970, 61, 4.4, 1.0),
            giant("Doomguard Commander", 4426, 61, 4.0, 1.1),
        ],
        bosses: &[
            foe("Akubar the Seer", 10920, 54, 4.4, 2.6),
            foe("Lord Azrethoc", 4426, 55, 4.6, 1.35),
            foe("Lord Hel'nurath", 14556, 62, 4.8, 1.65),
            giant("Teremus the Devourer", 6378, 60, 5.0, 0.55),
            foe("Lord Banehollow", 8611, 62, 4.8, 1.95),
            giant("Lord Kazzak", 12449, 63, 4.6, 0.75),
        ],
    },
    MapDef {
        name: "Gates of Ahn'Qiraj",
        zone: "Silithus",
        map: 1,
        // The sands before the Scarab Wall.
        center: [-8180.0, 1536.0, 5.0],
        leash: 80.0,
        victory: "The Qiraji retreat behind the Scarab Wall.",
        foes: &[
            foe("Sand Skitterer", 8014, 56, 5.0, 1.0),
            foe("Stonelash Scorpid", 15383, 55, 4.6, 1.6),
            foe("Dredge Striker", 15386, 56, 4.4, 1.0),
            foe("Twilight Avenger", 11811, 58, 4.0, 1.1),
            foe("Hive'Ashi Drone", 2303, 58, 5.4, 0.55),
            foe("Stonelash Flayer", 15385, 57, 4.8, 1.75),
            foe("Hive'Ashi Defender", 12153, 59, 4.2, 1.3),
            foe("Dust Stormer", 8715, 57, 4.6, 1.05),
            foe("Twilight Stonecaller", 11815, 59, 3.8, 1.1),
            foe("Hive'Zora Wasp", 11088, 59, 5.6, 0.6),
            foe("Hive'Zora Reaver", 11085, 60, 4.8, 1.0),
            foe("Hive'Regal Ambusher", 11106, 60, 5.2, 1.05),
            foe("Vekniss Wasp", 15335, 60, 5.6, 0.65),
            foe("Qiraji Gladiator", 15741, 61, 4.4, 0.75),
            foe("Vekniss Warrior", 15334, 61, 4.6, 1.15),
            giant("Obsidian Destroyer", 15343, 61, 3.8, 1.2),
            giant("Qiraji Champion", 15340, 63, 4.6, 0.95),
            giant("Anubisath Sentinel", 15347, 61, 4.0, 0.62),
        ],
        bosses: &[
            giant("Kurinnaxx", 15742, 62, 5.0, 1.15),
            foe("General Rajaxx", 15376, 63, 4.8, 1.15),
            foe("Moam", 15392, 63, 4.2, 1.6),
            giant("Buru the Gorger", 15654, 63, 4.4, 0.85),
            giant("Princess Huhuran", 15739, 63, 5.0, 0.55),
            foe("Ossirian the Unscarred", 15432, 63, 4.6, 0.82),
        ],
    },
    MapDef {
        name: "Un'Goro Crater",
        zone: "Un'Goro Crater",
        map: 1,
        // The summit plateau, its lava lake on the west rim; the crater floor's canopy would
        // hide the fight from above.
        center: [-7136.0, -1309.0, -184.0],
        leash: 32.0,
        victory: "Even King Mosh has fallen.",
        foes: &[
            foe("Bloodpetal Lasher", 11634, 49, 3.6, 0.9),
            foe("Young Diemetradon", 8510, 48, 4.4, 0.55),
            foe("Ravasaur", 5242, 50, 5.4, 0.75),
            foe("Muculent Ooze", 11140, 50, 3.2, 1.4),
            foe("Ravasaur Hunter", 5292, 51, 5.6, 0.85),
            foe("Tar Beast", 1549, 51, 3.4, 1.0),
            foe("Bloodpetal Flayer", 11635, 52, 3.8, 1.05),
            foe("Primal Ooze", 4754, 52, 3.4, 1.6),
            foe("Gorishi Wasp", 11090, 53, 5.4, 0.55),
            foe("Pterrordax", 8411, 53, 5.2, 0.5),
            foe("Diemetradon", 8511, 52, 4.6, 0.7),
            foe("Glutinous Ooze", 1146, 54, 3.6, 1.8),
            foe("Tar Lurker", 9010, 54, 3.6, 1.15),
            foe("Scorching Elemental", 1070, 54, 4.2, 1.05),
            foe("Frenzied Pterrordax", 8412, 55, 5.6, 0.55),
            foe("Elder Diemetradon", 8512, 55, 4.6, 0.8),
            giant("Stegodon", 5241, 52, 3.8, 0.72),
            giant("Stone Guardian", 8395, 52, 3.6, 1.45),
            giant("Plated Stegodon", 5287, 54, 3.8, 0.78),
            giant("Devilsaur", 5239, 56, 5.0, 0.72),
        ],
        bosses: &[
            foe("Ravasaur Matriarch", 11319, 56, 5.6, 1.55),
            foe("Uhk'loc", 8129, 59, 4.8, 2.4),
            foe("Blazerunner", 1204, 57, 4.6, 1.8),
            foe("Baron Charr", 14517, 58, 4.6, 1.95),
            giant("Tyrant Devilsaur", 5240, 60, 5.0, 1.0),
            giant("King Mosh", 5305, 61, 5.0, 1.2),
        ],
    },
    MapDef {
        name: "Gurubashi Arena",
        zone: "Stranglethorn Vale",
        map: 0,
        // The arena floor; the stands ring it.
        center: [-13205.0, 272.0, 22.0],
        leash: 30.0,
        victory: "The arena roars your name.",
        foes: &[
            foe("Shadowmaw Panther", 11452, 37, 5.6, 1.4),
            foe("Cold Eye Basilisk", 8797, 39, 3.8, 1.0),
            foe("Jungle Stalker", 11317, 40, 5.2, 0.8),
            foe("Elder Mistvale Gorilla", 838, 40, 4.0, 1.1),
            foe("Zanzil Zombie", 1065, 43, 3.4, 1.1),
            foe("Thrashtail Basilisk", 8802, 41, 4.0, 1.15),
            foe("Bloodsail Sea Dog", 796, 44, 4.0, 1.05),
            foe("Gurubashi Axe Thrower", 11074, 60, 4.2, 1.1),
            foe("Razzashi Serpent", 15182, 60, 4.8, 0.85),
            foe("Zulian Panther", 633, 60, 5.6, 1.6),
            foe("Gurubashi Headhunter", 11109, 60, 4.4, 1.15),
            foe("Bloodseeker Bat", 14562, 60, 5.6, 0.55),
            foe("Razzashi Raptor", 2571, 60, 5.6, 0.95),
            foe("Gurubashi Blood Drinker", 11080, 60, 4.2, 1.2),
            foe("Zulian Tiger", 11031, 61, 5.4, 1.8),
            foe("Hakkari Shadowcaster", 11229, 61, 4.0, 1.2),
            foe("Gurubashi Champion", 11100, 61, 4.4, 1.3),
            foe("Gurubashi Bat Rider", 15303, 62, 5.0, 0.6),
            giant("Gurubashi Berserker", 14832, 62, 4.0, 1.7),
        ],
        bosses: &[
            giant("King Bangalash", 616, 43, 5.8, 3.0),
            foe("Mokk the Savage", 840, 44, 4.8, 2.4),
            foe("Bloodlord Mandokir", 11288, 63, 5.0, 2.1),
            foe("High Priest Thekal", 15216, 63, 5.0, 2.3),
            foe("Jin'do the Hexxer", 11311, 63, 4.6, 2.5),
            giant("Hakkar", 15295, 63, 4.4, 2.7),
        ],
    },
    MapDef {
        name: "Elwynn Forest",
        zone: "Eastern Kingdoms",
        map: 0,
        // The old ASR's circle for this zone, exactly as its story chapters placed it.
        center: [-9396.0, -633.0, 68.0],
        leash: 120.0,
        victory: "Elwynn Forest is safe once more.",
        foes: &[
            foe("Young Wolf", 447, 1, 5.4, 1.0),
            foe("Timber Wolf", 604, 2, 5.4, 1.0),
            foe("Defias Thug", 5035, 4, 4.2, 1.15),
            foe("Mangy Wolf", 903, 6, 5.4, 1.0),
            foe("Stonetusk Boar", 503, 6, 4.6, 1.0),
            foe("Kobold Tunneler", 139, 6, 4.2, 1.0),
            foe("Defias Cutpurse", 2361, 6, 4.2, 1.15),
            foe("Forest Spider", 382, 6, 4.2, 1.0),
            foe("Murloc", 617, 7, 4.2, 1.0),
            foe("Kobold Miner", 373, 7, 4.2, 1.0),
            foe("Murloc Streamrunner", 527, 7, 4.2, 1.0),
            foe("Rockhide Boar", 389, 8, 4.6, 1.0),
            foe("Gray Forest Wolf", 380, 8, 5.4, 1.0),
            foe("Defias Bandit", 2357, 9, 4.2, 1.15),
            giant("Young Forest Bear", 1006, 9, 4.6, 1.0),
            foe("Defias Rogue Wizard", 2359, 10, 3.8, 1.1),
        ],
        bosses: &[
            foe("Mother Fang", 2541, 10, 4.6, 1.9),
            foe("Narg the Taskmaster", 774, 10, 4.2, 1.9),
            foe("Hogger", 384, 11, 4.2, 1.9),
            foe("Thuros Lightfingers", 3341, 11, 4.2, 2.18),
            foe("Fedfennel", 175, 12, 4.2, 1.9),
            foe("Gruff Swiftbite", 175, 12, 4.2, 1.9),
        ],
    },
    MapDef {
        name: "Durotar",
        zone: "Kalimdor",
        map: 1,
        // The old ASR's circle for this zone, exactly as its story chapters placed it.
        center: [587.5, -4311.4, 19.0],
        leash: 120.0,
        victory: "Durotar is quiet again.",
        foes: &[
            foe("Vile Familiar", 1185, 4, 4.2, 1.0),
            foe("Felstalker", 850, 4, 4.2, 1.0),
            foe("Clattering Scorpid", 2486, 6, 4.6, 1.5),
            foe("Pygmy Surf Crawler", 1307, 6, 4.6, 1.0),
            foe("Dire Mottled Boar", 381, 7, 4.6, 1.0),
            foe("Razormane Quilboar", 1218, 7, 4.6, 1.0),
            foe("Armored Scorpid", 2487, 8, 4.6, 1.5),
            foe("Bloodtalon Taillasher", 1960, 8, 5.4, 0.85),
            foe("Razormane Scout", 1252, 8, 4.6, 1.0),
            foe("Dustwind Harpy", 3012, 8, 4.2, 1.0),
            foe("Dustwind Pillager", 6814, 8, 4.2, 1.0),
            foe("Razormane Dustrunner", 6088, 9, 4.6, 1.0),
            foe("Elder Mottled Boar", 193, 9, 4.6, 1.0),
            foe("Razormane Battleguard", 1964, 10, 4.6, 1.0),
            foe("Burning Blade Neophyte", 4186, 10, 3.8, 1.0),
            foe("Burning Blade Cultist", 4194, 11, 3.8, 1.0),
        ],
        bosses: &[
            foe("Geolord Mottle", 6113, 9, 4.6, 1.9),
            foe("Razormane Dustrunner", 6088, 9, 4.6, 1.9),
            foe("Razormane Battleguard", 1964, 10, 4.6, 1.9),
            foe("Burning Blade Neophyte", 4186, 10, 3.8, 1.9),
            foe("Death Flayer", 2491, 11, 4.6, 2.85),
            foe("Burning Blade Cultist", 4194, 11, 3.8, 1.9),
        ],
    },
    MapDef {
        name: "Teldrassil",
        zone: "Kalimdor",
        map: 1,
        // Open, level ground among Teldrassil's own creatures: no building or big prop near.
        center: [9737.5, 649.8, 1298.5],
        leash: 150.0,
        victory: "Teldrassil blooms again.",
        foes: &[
            foe("Young Nightsaber", 11454, 1, 5.4, 1.5),
            foe("Young Thistle Boar", 8869, 2, 4.6, 1.0),
            foe("Grell", 6806, 3, 4.2, 1.0),
            foe("Grellkin", 3023, 4, 4.2, 1.0),
            foe("Nightsaber", 6805, 6, 5.4, 1.5),
            foe("Strigid Owl", 10832, 6, 5.2, 1.0),
            foe("Webwood Lurker", 760, 6, 4.2, 1.0),
            foe("Timberling", 3033, 6, 4.2, 1.15),
            foe("Gnarlpine Gardener", 6799, 6, 3.8, 1.0),
            foe("Gnarlpine Warrior", 6800, 7, 4.2, 1.0),
            foe("Gnarlpine Ambusher", 3024, 7, 4.2, 1.0),
            foe("Vicious Grell", 6811, 7, 4.2, 1.0),
            foe("Gnarlpine Defender", 6801, 8, 4.2, 1.0),
            foe("Gnarlpine Shaman", 897, 8, 3.8, 1.0),
            foe("Timberling Bark Ripper", 6817, 8, 4.2, 1.15),
            foe("Gnarlpine Augur", 3024, 9, 3.8, 1.0),
        ],
        bosses: &[
            foe("Threggil", 904, 6, 4.2, 1.9),
            foe("Uruson", 6818, 7, 4.2, 1.9),
            foe("Gnarlpine Defender", 6801, 8, 4.2, 1.9),
            foe("Duskstalker", 11453, 9, 5.4, 2.85),
            foe("Oakenscowl", 2567, 9, 4.2, 2.18),
            foe("Gnarlpine Augur", 3024, 9, 3.8, 1.9),
        ],
    },
    MapDef {
        name: "Dun Morogh",
        zone: "Eastern Kingdoms",
        map: 0,
        // Open, level ground among Dun Morogh's own creatures: no building or big prop near.
        center: [-5400.8, -739.9, 396.7],
        leash: 150.0,
        victory: "Dun Morogh thaws in your wake.",
        foes: &[
            foe("Crag Boar", 607, 6, 4.6, 1.0),
            giant("Young Black Bear", 8843, 6, 4.6, 1.0),
            foe("Juvenile Snow Leopard", 748, 6, 5.4, 1.5),
            foe("Young Wendigo", 929, 6, 4.2, 1.0),
            foe("Wendigo", 950, 7, 4.2, 1.0),
            foe("Large Crag Boar", 381, 7, 4.6, 1.0),
            foe("Snow Tracker Wolf", 604, 7, 5.4, 1.0),
            foe("Snow Leopard", 954, 8, 5.4, 1.5),
            foe("Elder Crag Boar", 744, 8, 4.6, 1.0),
            foe("Winter Wolf", 785, 8, 5.4, 1.0),
            giant("Ice Claw Bear", 8840, 8, 4.6, 1.0),
            foe("Frostmane Seer", 5610, 9, 3.8, 1.2),
            foe("Frostmane Snowstrider", 5607, 9, 4.2, 1.2),
        ],
        bosses: &[
            foe("Edan the Howler", 931, 9, 4.2, 1.9),
            foe("Timber", 11422, 10, 5.4, 1.9),
            foe("Great Father Arctikus", 5625, 11, 4.2, 2.28),
            foe("Old Icebeard", 1050, 11, 4.2, 1.9),
            foe("Vagash", 1050, 11, 4.2, 1.9),
            giant("Bjarn", 913, 12, 4.6, 1.9),
        ],
    },
    MapDef {
        name: "Mulgore",
        zone: "Kalimdor",
        map: 1,
        // Open, level ground among Mulgore's own creatures: no building or big prop near.
        center: [-1809.9, -220.6, -8.4],
        leash: 150.0,
        victory: "Mulgore's herds roam free.",
        foes: &[
            foe("Plainstrider", 1219, 2, 5.4, 1.0),
            foe("Battleboar", 8869, 4, 4.6, 1.0),
            foe("Prairie Wolf", 1100, 6, 5.4, 1.0),
            foe("Palemane Tanner", 550, 6, 4.2, 1.0),
            foe("Venture Co. Hireling", 7243, 6, 4.2, 1.3),
            foe("Adult Plainstrider", 1220, 7, 5.4, 1.0),
            foe("Wiry Swoop", 1228, 7, 5.2, 0.8),
            foe("Venture Co. Laborer", 7244, 7, 4.2, 1.3),
            foe("Palemane Skinner", 1216, 7, 4.2, 1.0),
            foe("Prairie Stalker", 643, 8, 5.4, 1.0),
            foe("Flatland Cougar", 1059, 8, 5.4, 1.5),
            foe("Palemane Poacher", 1217, 8, 4.2, 1.0),
            foe("Venture Co. Taskmaster", 7246, 8, 4.2, 1.3),
            foe("Swoop", 1229, 9, 5.2, 0.8),
            foe("Elder Plainstrider", 1221, 9, 5.4, 1.0),
        ],
        bosses: &[
            foe("Prairie Stalker", 643, 8, 5.4, 1.9),
            foe("Mazzranache", 1961, 9, 5.4, 1.9),
            foe("Snagglespear", 275, 9, 4.2, 1.9),
            foe("Swoop", 1229, 9, 5.2, 1.52),
            foe("Elder Plainstrider", 1221, 9, 5.4, 1.9),
            foe("Ghost Howl", 720, 12, 5.4, 1.9),
        ],
    },
    MapDef {
        name: "Tirisfal Glades",
        zone: "Eastern Kingdoms",
        map: 0,
        // Open, level ground among Tirisfal Glades's own creatures: no building or big prop near.
        center: [1993.0, -153.3, 35.4],
        leash: 150.0,
        victory: "Tirisfal is at peace.",
        foes: &[
            foe("Decrepit Darkhound", 9021, 6, 4.2, 1.4),
            foe("Rotting Dead", 1200, 6, 3.6, 1.0),
            foe("Greater Duskbat", 4734, 7, 5.2, 0.8),
            foe("Ravaged Corpse", 1201, 7, 3.6, 1.0),
            foe("Rot Hide Graverobber", 10853, 7, 4.2, 1.0),
            foe("Rot Hide Gnoll", 10852, 7, 4.2, 1.0),
            foe("Rattlecage Soldier", 200, 7, 4.2, 1.2),
            foe("Cursed Darkhound", 9020, 8, 4.2, 1.4),
            foe("Rot Hide Mongrel", 847, 8, 4.2, 1.0),
            foe("Hungering Dead", 1202, 8, 3.6, 1.0),
            foe("Scarlet Missionary", 2468, 8, 3.8, 1.15),
            foe("Darkeye Bonecaster", 11398, 8, 3.8, 1.0),
            foe("Vampiric Duskbat", 8808, 9, 5.2, 0.8),
            foe("Scarlet Zealot", 2483, 9, 4.2, 1.1),
            foe("Shambling Horror", 1197, 9, 3.6, 1.0),
        ],
        bosses: &[
            foe("Farmer Solliden", 3535, 8, 4.2, 2.18),
            foe("Tormented Spirit", 9534, 9, 4.2, 1.52),
            foe("Bayne", 7892, 10, 4.2, 2.66),
            foe("Ressan the Needler", 9750, 11, 5.2, 1.52),
            foe("Fellicent's Shade", 5430, 12, 4.2, 1.9),
            foe("Gorefang", 11413, 13, 5.4, 1.9),
        ],
    },
    MapDef {
        name: "Westfall",
        zone: "Eastern Kingdoms",
        map: 0,
        // The old ASR's circle for this zone, exactly as its story chapters placed it.
        center: [-10850.0, 900.0, 35.0],
        leash: 120.0,
        victory: "Westfall's fields are safe.",
        foes: &[
            foe("Young Fleshripper", 410, 11, 5.2, 0.8),
            foe("Coyote", 643, 11, 5.4, 1.0),
            foe("Defias Smuggler", 4418, 12, 4.2, 1.15),
            giant("Harvest Golem", 367, 12, 3.6, 1.0),
            foe("Young Goretusk", 8871, 13, 4.6, 1.0),
            foe("Defias Trapper", 2331, 13, 4.2, 1.1),
            foe("Fleshripper", 2305, 14, 5.2, 0.8),
            foe("Goretusk", 3027, 15, 4.6, 1.0),
            giant("Harvest Watcher", 378, 15, 3.6, 1.0),
            foe("Riverpaw Brute", 384, 16, 4.2, 1.0),
            foe("Riverpaw Bandit", 383, 17, 4.2, 1.0),
            foe("Great Goretusk", 3035, 17, 4.6, 1.0),
            foe("Riverpaw Taskmaster", 376, 18, 4.2, 1.0),
            foe("Defias Highwayman", 2342, 18, 4.2, 1.15),
            foe("Pygmy Venom Web Spider", 958, 19, 4.6, 1.0),
            foe("Riverpaw Mystic", 502, 19, 3.8, 1.0),
        ],
        bosses: &[
            foe("Brainwashed Noble", 3267, 18, 4.2, 2.18),
            foe("Marisa du'Paige", 2355, 18, 4.2, 2.09),
            foe("Sergeant Brashclaw", 383, 18, 4.2, 1.9),
            foe("Klaven Mortwake", 5805, 23, 4.2, 2.18),
            foe("Lupos", 11412, 23, 5.4, 1.9),
            foe("Vultros", 507, 26, 5.2, 1.52),
        ],
    },
    MapDef {
        name: "Duskwood",
        zone: "Eastern Kingdoms",
        map: 0,
        // The old ASR's circle for this zone, exactly as its story chapters placed it.
        center: [-10540.0, 180.0, 30.3],
        leash: 120.0,
        victory: "Duskwood's curse lifts.",
        foes: &[
            foe("Pygmy Venom Web Spider", 958, 19, 4.6, 1.0),
            foe("Venom Web Spider", 955, 20, 4.6, 1.0),
            foe("Skeletal Warrior", 200, 22, 4.2, 1.2),
            foe("Green Recluse", 2541, 22, 4.6, 1.0),
            foe("Skeletal Mage", 9783, 23, 4.2, 1.0),
            foe("Young Black Ravager", 246, 24, 5.4, 1.0),
            foe("Skeletal Horror", 9786, 24, 3.6, 1.0),
            foe("Black Widow Hatchling", 368, 25, 4.6, 1.0),
            foe("Black Ravager", 741, 25, 5.4, 1.0),
            foe("Black Ravager Mastiff", 9562, 26, 5.4, 1.0),
            foe("Nightbane Worgen", 657, 27, 5.4, 1.0),
            foe("Nightbane Shadow Weaver", 203, 28, 5.4, 1.0),
            foe("Nightbane Dark Runner", 736, 29, 5.4, 1.0),
            foe("Nightbane Vile Fang", 522, 30, 5.4, 1.0),
            foe("Fetid Corpse", 559, 30, 3.6, 1.3),
            foe("Nightbane Tainted One", 1098, 31, 5.4, 1.0),
        ],
        bosses: &[
            foe("Lupos", 11412, 23, 5.4, 1.9),
            foe("Naraxis", 963, 27, 4.6, 1.9),
            foe("Nightbane Vile Fang", 522, 30, 5.4, 1.9),
            foe("Nightbane Tainted One", 1098, 31, 5.4, 1.9),
            foe("Fenros", 11179, 32, 5.4, 1.9),
            foe("Nefaru", 11181, 34, 5.4, 1.9),
        ],
    },
    MapDef {
        name: "Stonetalon Mountains",
        zone: "Kalimdor",
        map: 1,
        // The old ASR's circle for this zone, exactly as its story chapters placed it.
        center: [1380.0, -350.0, 8.79],
        leash: 110.0,
        victory: "The Venture Company's saws fall silent.",
        foes: &[
            foe("Windshear Tunnel Rat", 373, 21, 4.2, 1.0),
            foe("Deepmoss Creeper", 760, 21, 4.6, 1.0),
            foe("Venture Co. Operator", 7230, 22, 4.2, 1.0),
            foe("Venture Co. Logger", 7228, 22, 4.2, 1.0),
            foe("Windshear Geomancer", 163, 23, 3.8, 1.0),
            foe("Deepmoss Webspinner", 1989, 23, 4.6, 1.0),
            foe("Venture Co. Cutter", 7225, 24, 4.2, 1.0),
            foe("Pridewing Wyvern", 295, 24, 5.2, 0.8),
            foe("Venture Co. Deforester", 7226, 25, 4.2, 1.0),
            foe("Venture Co. Engineer", 7227, 25, 3.8, 1.0),
            foe("Venture Co. Machine Smith", 7229, 26, 4.2, 1.0),
            giant("Venture Co. Shredder", 1269, 27, 3.6, 1.0),
        ],
        bosses: &[
            giant("XT:4", 1269, 25, 3.6, 1.6),
            giant("XT:9", 1269, 26, 3.6, 1.6),
            foe("Foreman Rigger", 4600, 27, 4.2, 2.09),
            foe("Taskmaster Whipfang", 487, 28, 4.2, 1.9),
            foe("Gerenzo Wrenchwhistle", 3885, 30, 4.2, 2.6),
        ],
    },
    MapDef {
        name: "Stranglethorn Vale",
        zone: "Eastern Kingdoms",
        map: 0,
        // The old ASR's circle for this zone, exactly as its story chapters placed it.
        center: [-11920.0, -180.0, 18.16],
        leash: 100.0,
        victory: "Stranglethorn's jungle bows to you.",
        foes: &[
            foe("Young Stranglethorn Tiger", 598, 30, 5.4, 0.8),
            foe("Young Lashtail Raptor", 2571, 31, 5.4, 0.8),
            foe("Panther", 599, 32, 5.4, 1.0),
            foe("Lashtail Raptor", 788, 33, 5.4, 1.0),
            foe("Skullsplitter Scout", 4625, 34, 4.2, 1.0),
            foe("Elder Stranglethorn Tiger", 698, 35, 5.4, 1.0),
            foe("Skullsplitter Mystic", 4616, 36, 3.8, 1.0),
            foe("Shadowmaw Panther", 633, 37, 5.4, 1.0),
            foe("Kurzen Elite", 4446, 38, 4.2, 1.0),
            foe("Skullsplitter Headhunter", 4635, 39, 4.2, 1.0),
            giant("Mosh'Ogg Shaman", 536, 40, 3.6, 1.0),
            giant("Mosh'Ogg Lord", 11569, 42, 3.6, 1.1),
        ],
        bosses: &[
            foe("Sin'Dall", 471, 37, 5.4, 1.9),
            foe("Bhag'thera", 613, 40, 5.4, 1.9),
            foe("Tethis", 8472, 43, 5.4, 1.9),
            foe("Colonel Kurzen", 637, 45, 4.2, 2.18),
            foe("King Bangalash", 616, 48, 5.4, 2.1),
        ],
    },
    MapDef {
        name: "The Deadmines",
        zone: "Dungeon",
        map: 36,
        // The old ASR's circle for this zone: the Mast Room, its exits barricaded
        // (`super::dungeon` holds its floor and outline).
        center: [-291.0, -515.0, 49.58],
        leash: 110.0,
        victory: "VanCleef's revenge sinks with his ship.",
        foes: &[
            foe("Defias Strip Miner", 2145, 16, 4.2, 1.0),
            foe("Defias Miner", 2311, 16, 4.2, 1.0),
            foe("Goblin Woodcarver", 7051, 17, 4.2, 1.0),
            foe("Defias Watchman", 2332, 17, 4.2, 1.0),
            foe("Goblin Craftsman", 7053, 18, 4.2, 1.0),
            foe("Defias Evoker", 2314, 18, 3.8, 1.0),
            foe("Defias Taskmaster", 2146, 19, 4.2, 1.0),
            foe("Goblin Engineer", 7031, 19, 3.8, 1.0),
            foe("Defias Henchman", 2347, 20, 4.2, 1.0),
            giant("Remote-Controlled Golem", 2280, 20, 3.6, 0.8),
            foe("Defias Pirate", 2350, 21, 4.2, 1.0),
            foe("Defias Squallshaper", 4418, 21, 3.8, 1.0),
        ],
        bosses: &[
            giant("Rhahk'Zor", 14403, 19, 3.6, 1.3),
            giant("Sneed's Shredder", 1269, 20, 3.6, 1.6),
            foe("Gilnid", 7056, 20, 4.2, 2.18),
            foe("Mr. Smite", 2026, 20, 4.2, 1.6),
            foe("Edwin VanCleef", 2029, 21, 4.6, 2.18),
        ],
    },
    MapDef {
        name: "Wailing Caverns",
        zone: "Dungeon",
        map: 43,
        // The old ASR's circle for this zone: the Pit of Fangs, its lower tunnel barricaded
        // (`super::dungeon` holds its floor and play area).
        center: [-31.0, 451.0, -101.85],
        leash: 190.0,
        victory: "Naralex wakes from the nightmare.",
        foes: &[
            foe("Deviate Coiler", 1742, 17, 5.2, 0.8),
            foe("Deviate Stinglash", 4091, 17, 5.2, 0.8),
            foe("Deviate Creeper", 1744, 18, 5.4, 0.9),
            foe("Deviate Slayer", 949, 18, 5.4, 1.0),
            foe("Deviate Stalker", 1746, 19, 5.4, 1.0),
            foe("Deviate Ravager", 1747, 19, 5.4, 1.1),
            foe("Deviate Guardian", 755, 20, 5.0, 1.2),
            foe("Devouring Ectoplasm", 4266, 20, 3.6, 1.0),
            foe("Evolving Ectoplasm", 1751, 21, 3.6, 1.0),
            giant("Deviate Lurker", 1752, 21, 3.6, 0.8),
            foe("Druid of the Fang", 4211, 22, 3.8, 1.0),
            foe("Nightmare Ectoplasm", 470, 22, 3.6, 1.1),
        ],
        bosses: &[
            foe("Lady Anacondra", 4313, 20, 4.2, 2.09),
            foe("Lord Cobrahn", 4213, 20, 4.2, 2.09),
            foe("Lord Pythas", 4214, 21, 4.2, 2.09),
            foe("Lord Serpentis", 4215, 21, 4.2, 2.09),
            foe("Mutanus the Devourer", 4088, 22, 4.6, 2.6),
        ],
    },
    MapDef {
        name: "Redridge Mountains",
        zone: "Eastern Kingdoms",
        map: 0,
        // Open, level ground among Redridge Mountains's own creatures: no building or big prop near.
        center: [-9199.8, -2767.5, 91.0],
        leash: 150.0,
        victory: "Redridge is safe once more.",
        foes: &[
            foe("Redridge Thrasher", 489, 15, 4.2, 1.0),
            foe("Redridge Mongrel", 487, 16, 4.2, 1.0),
            foe("Tarantula", 366, 16, 4.2, 1.0),
            foe("Great Goretusk", 3035, 17, 4.6, 1.0),
            foe("Redridge Poacher", 551, 17, 4.2, 1.0),
            foe("Murloc Shorestriker", 1305, 17, 4.2, 1.0),
            foe("Black Dragon Whelp", 387, 18, 4.2, 1.0),
            foe("Redridge Brute", 500, 18, 4.2, 1.0),
            foe("Redridge Mystic", 10789, 19, 3.8, 1.0),
            foe("Dire Condor", 490, 19, 5.2, 0.8),
            foe("Redridge Basher", 609, 20, 4.2, 1.0),
            foe("Greater Tarantula", 520, 20, 4.2, 1.0),
            foe("Redridge Drudger", 551, 21, 4.2, 1.0),
            foe("Blackrock Summoner", 6030, 23, 3.8, 1.0),
            foe("Blackrock Tracker", 6040, 24, 4.2, 1.0),
            foe("Blackrock Champion", 6044, 25, 4.2, 1.0),
        ],
        bosses: &[
            foe("Blackrock Hunter", 6031, 24, 4.2, 1.9),
            foe("Singe", 497, 24, 4.2, 1.9),
            foe("Tharil'zun", 6043, 24, 4.2, 1.9),
            foe("Blackrock Gladiator", 6042, 25, 4.2, 1.9),
            foe("Rohh the Silent", 10792, 26, 4.2, 1.9),
            foe("Kazon", 6041, 27, 4.2, 1.9),
        ],
    },
    MapDef {
        name: "Mount Hyjal",
        zone: "Kalimdor",
        map: 1,
        // A plateau below the World Tree.
        center: [5478.0, -3729.0, 1593.0],
        leash: 70.0,
        victory: "Nordrassil endures.",
        foes: &[
            foe("Cannibal Ghoul", 10626, 56, 4.6, 1.3),
            foe("Jadefire Rogue", 8575, 52, 5.2, 1.0),
            foe("Putrid Gargoyle", 7854, 56, 5.2, 0.85),
            foe("Death Cultist", 10391, 56, 3.8, 1.1),
            foe("Felbeast", 7949, 50, 4.8, 0.85),
            foe("Torn Screamer", 984, 57, 5.0, 0.7),
            foe("Necromancer", 10427, 57, 3.6, 1.15),
            foe("Jadefire Felsworn", 11334, 54, 4.8, 1.1),
            foe("Felhound", 1913, 54, 5.2, 1.0),
            foe("Nerubian Overseer", 14698, 58, 4.4, 0.95),
            foe("Felguard Sentry", 9017, 55, 4.2, 0.9),
            foe("Hate Shrieker", 10751, 58, 5.0, 0.8),
            foe("Hederine Manastalker", 6172, 58, 5.4, 1.15),
            giant("Lesser Infernal", 10906, 55, 3.4, 1.0),
            foe("Felguard Elite", 7970, 61, 4.4, 1.0),
            giant("Stitched Horror", 1693, 58, 3.6, 1.25),
            giant("Doomguard Commander", 4426, 61, 4.0, 1.1),
        ],
        bosses: &[
            foe("Prince Xavalis", 11335, 57, 5.0, 1.85),
            foe("Lady Hederine", 10925, 61, 4.8, 2.7),
            giant("Taerar", 15363, 63, 4.8, 0.5),
            giant("Lethon", 15365, 63, 4.8, 0.5),
            giant("Emeriss", 15366, 63, 4.8, 0.53),
            giant("Ysondre", 15364, 63, 4.8, 0.58),
        ],
    },
    MapDef {
        name: "Naxxramas",
        zone: "Eastern Plaguelands",
        map: 0,
        // Plaguewood, under the necropolis.
        center: [3010.0, -3790.0, 124.0],
        leash: 80.0,
        victory: "Kel'Thuzad's necropolis falls dark.",
        foes: &[
            foe("Scourge Soldier", 5231, 55, 4.4, 1.15),
            foe("Plagued Swine", 6121, 53, 5.0, 1.0),
            foe("Cannibal Ghoul", 10626, 56, 4.6, 1.3),
            foe("Cursed Mage", 11397, 55, 3.6, 0.95),
            foe("Putrid Gargoyle", 7854, 56, 5.2, 0.85),
            foe("Scourge Warder", 612, 56, 4.4, 1.2),
            foe("Death Cultist", 10391, 56, 3.8, 1.1),
            foe("Frenzied Plaguehound", 9022, 57, 5.6, 1.4),
            foe("Torn Screamer", 984, 57, 5.0, 0.7),
            foe("Necromancer", 10427, 57, 3.6, 1.15),
            foe("Carrion Devourer", 7900, 57, 4.0, 1.25),
            foe("Crypt Walker", 11178, 58, 4.6, 0.85),
            foe("Putrid Shrieker", 7855, 58, 5.4, 1.0),
            foe("Hate Shrieker", 10751, 58, 5.0, 0.8),
            foe("Ziggurat Protector", 10627, 59, 4.4, 1.6),
            foe("Deathknight", 16508, 60, 4.6, 1.75),
            giant("Stitched Horror", 1693, 58, 3.6, 1.25),
            giant("Slaughterhouse Protector", 7864, 60, 3.6, 1.3),
            giant("Death Lord", 10729, 61, 5.0, 1.05),
        ],
        bosses: &[
            giant("Anub'Rekhan", 15931, 63, 4.6, 0.55),
            foe("Grand Widow Faerlina", 15940, 63, 4.8, 2.9),
            giant("Patchwerk", 16174, 63, 4.0, 1.7),
            giant("Gluth", 16064, 63, 5.0, 0.62),
            giant("Thaddius", 16137, 63, 4.0, 0.98),
            foe("Kel'Thuzad", 15945, 63, 4.4, 2.1),
        ],
    },
    MapDef {
        name: "Hall of Thanes",
        zone: "Forever / Dungeon",
        map: 3065,
        // A safe floor cell in the first great burial hall, from the local import.
        center: [45.5, 0.5, 501.57],
        leash: 32.0,
        victory: "The ancient thanes rest once more.",
        // Survivors adaptation: existing classic displays stand in for Forever NPCs.
        foes: &[
            foe("Restless Dead", 200, 13, 3.6, 1.0),
            foe("Tormented Soul", 9534, 13, 3.8, 1.0),
            foe("Dark Iron Intruder", 825, 14, 4.2, 1.0),
            foe("Dark Iron Engineer", 870, 14, 3.8, 1.0),
            foe("Dark Iron Summoner", 2871, 15, 3.8, 1.0),
            foe("Tomb Guardian", 2280, 16, 3.6, 1.0),
            foe("Enraged Apparition", 9534, 16, 4.2, 1.3),
            foe("Dark Iron Veteran", 8807, 18, 4.2, 1.1),
        ],
        bosses: &[
            foe("Faldrim Anvilmar", 8807, 15, 4.0, 1.6),
            giant("Magmatus", 14517, 16, 3.8, 0.75),
            giant("Plunder", 2280, 17, 3.6, 1.5),
            foe("Durgen Dirgehammer", 8807, 18, 4.2, 1.8),
        ],
    },
];

/// A battleground's horde, bosses and elites with their numbers. Each foe joins at its `from`
/// second, each boss comes at its own, and the last boss's death wins the run.
pub(crate) struct Roster {
    pub enemies: Vec<EnemyDef>,
    pub bosses: Vec<EnemyDef>,
    /// The elites holding the battleground's four lairs, north, east, south and west.
    pub guardians: Vec<EnemyDef>,
    /// How many enemies the battleground holds at once, against open ground's 1.
    pub crowd: f32,
}

static ROSTERS: std::sync::LazyLock<Vec<Roster>> = std::sync::LazyLock::new(|| {
    MAPS.iter()
        .map(|m| super::elwynn::roster(m).unwrap_or_else(|| zone_roster(m)))
        .collect()
});

impl Foe {
    /// The Barrens' hand-tuned rows as plain creatures, for its roster on Elwynn's tables.
    fn of(def: &EnemyDef) -> Self {
        Foe {
            name: def.name,
            display: def.display,
            level: def.level,
            speed: def.speed,
            scale: def.scale,
            big: def.big,
        }
    }
}

/// A battleground's creatures on Elwynn's tables. Each horde row takes the numbers, price and
/// minute at its place along Elwynn's horde; each boss its place along Elwynn's boss clock, with
/// that Elwynn boss's moves; four local rows hold Elwynn's four elite slots. The zone's tier
/// scales them all at spawn.
fn zone_roster(map: &MapDef) -> Roster {
    let tier = super::difficulty::tier(map.name).unwrap_or(1);
    let (foes, bosses): (Vec<Foe>, Vec<Foe>) = if map.foes.is_empty() {
        (ENEMIES.iter().map(Foe::of).collect(), BOSSES.iter().map(Foe::of).collect())
    } else {
        (map.foes.to_vec(), map.bosses.to_vec())
    };
    let plain = |foe: &Foe| EnemyDef {
        scale: foe.scale,
        big: foe.big,
        tier,
        ..en(foe.name, foe.display, 0.0, foe.speed, 0.0, 0, foe.level)
    };

    let horde = super::elwynn::horde();
    let enemies = foes
        .iter()
        .enumerate()
        .map(|(i, foe)| {
            let (lo, hi, k) = rung(&horde, i, foes.len());
            let mix = |a: f32, b: f32| a + (b - a) * k;
            let ranged = bolt(foe.name);
            let swing = mix(lo.attack, hi.attack);
            EnemyDef {
                hp: mix(lo.hp, hi.hp).round(),
                damage: mix(lo.damage, hi.damage),
                speed: mix(lo.speed, hi.speed) * (foe.speed / 4.2).clamp(0.9, 1.15),
                // A caster throws on Elwynn's caster beat, whatever its place on the ladder.
                attack: if ranged.is_some() { swing.max(3.2) } else { swing },
                xp: mix(lo.xp as f32, hi.xp as f32).round() as u32,
                from: mix(lo.from, hi.from).round(),
                cost: mix(lo.cost, hi.cost),
                ranged,
                ..plain(foe)
            }
        })
        .collect();

    // Elwynn's boss clock, its lair lord included: (second, health, damage, experience).
    let mut clock: Vec<_> = super::elwynn::bosses()
        .iter()
        .chain(super::elwynn::guardians().iter().filter(|d| d.lord))
        .map(|d| (d.from, d.hp, d.damage, d.xp as f32, d.speed))
        .collect();
    clock.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (first, end) = (clock[0].0, clock[clock.len() - 1].0);
    let kits = super::elwynn::bosses();
    let count = bosses.len();
    let bosses = bosses
        .iter()
        .enumerate()
        .map(|(i, foe)| {
            let share = if bosses.len() > 1 { i as f32 / (bosses.len() - 1) as f32 } else { 1.0 };
            let at = first + (end - first) * share;
            let upper = clock.iter().position(|c| c.0 >= at).unwrap_or(clock.len() - 1).max(1);
            let (lo, hi) = (clock[upper - 1], clock[upper]);
            let k = ((at - lo.0) / (hi.0 - lo.0)).clamp(0.0, 1.0);
            let mix = |a: f32, b: f32| a + (b - a) * k;
            let kit = &kits[(share * (kits.len() - 1) as f32).round() as usize];
            EnemyDef {
                hp: mix(lo.1, hi.1).round(),
                damage: mix(lo.2, hi.2),
                xp: mix(lo.3, hi.3).round() as u32,
                speed: mix(lo.4, hi.4),
                attack: kit.attack,
                from: at.round(),
                last: i + 1 == count,
                // Its own moves from the mechanics library, not the Elwynn boss's at its place.
                script: Some(super::boss_scripts::kit(map.name, foe.name, i, count, tier)),
                ..plain(foe)
            }
        })
        .collect();

    // The Barrens keeps its first four great beasts; elsewhere the biggest, highest rows.
    let mut rows: Vec<Foe> = if map.foes.is_empty() {
        foes.iter().filter(|f| f.big).take(4).copied().collect()
    } else {
        let mut rows = foes.clone();
        rows.sort_by(|a, b| b.big.cmp(&a.big).then_with(|| b.level.cmp(&a.level)));
        rows.truncate(4);
        rows
    };
    rows.sort_by_key(|foe| foe.level);
    let guardians = rows
        .iter()
        .zip(super::elwynn::guardians())
        .map(|(foe, slot)| EnemyDef {
            name: foe.name,
            display: foe.display,
            level: foe.level,
            big: foe.big,
            // Elwynn's elites stand about twice their kin's size; a golem or giant is big already,
            // and twice it walls off the camera.
            scale: foe.scale * if foe.big { 1.25 } else { 1.8 },
            intro: "",
            tier,
            affixes: super::mechanics::affix_count(tier),
            ..*slot
        })
        .collect();

    Roster {
        enemies,
        bosses,
        guardians,
        crowd: super::difficulty::crowd(map.leash),
    }
}

/// The two rows of `ladder` around position `i` of `n`, and how far between them it falls.
fn rung(ladder: &[EnemyDef], i: usize, n: usize) -> (EnemyDef, EnemyDef, f32) {
    let top = (ladder.len() - 1) as f32;
    let at = if n > 1 { i as f32 / (n - 1) as f32 * top } else { 0.0 };
    (ladder[at.floor() as usize], ladder[at.ceil() as usize], at.fract())
}

/// A caster's bolt, read from its name; Elwynn's casters throw Fireball and Frostbolt.
fn bolt(name: &str) -> Option<Ranged> {
    const KINDS: [(&[&str], u32); 4] = [
        // Lightning Bolt.
        (&["Shaman", "Mystic", "Stormer", "Squallshaper", "Augur", "Druid of", "Stonecaller", "Geomancer"], 403),
        // Shadow Bolt.
        (&["Necromancer", "Bonecaster", "Shadow Weaver", "Shadowcaster", "Summoner", "Cultist", "Evoker", "Seer", "Neophyte"], 686),
        // Frostbolt.
        (&["Mage", "Wizard"], 116),
        // Fireball.
        (&["Missionary", "Zealot"], 133),
    ];
    KINDS
        .iter()
        .find(|(words, _)| words.iter().any(|w| name.contains(w)))
        .map(|&(_, spell)| Ranged {
            spell,
            range: 18.0,
            cooldown: 1.7,
        })
}

pub(crate) fn roster(map: usize) -> &'static Roster {
    &ROSTERS[map.min(ROSTERS.len() - 1)]
}

