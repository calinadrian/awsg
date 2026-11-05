//! The bosses' scripts past Elwynn: each battleground's last boss fights in three phases of its
//! own, and the bosses before it each take a pair of signature moves no other boss of their
//! battleground shares. Starting zones deal from the simpler pairs.

use super::mechanics::{
    mv, Kit,
    Op::{Add, Remove},
    Phase,
    Verb::*,
};

// `Spell.dbc` rows the moves play.
const STOMP: u32 = 20549;
const ROCK: u32 = 4164;
const MOONFIRE: u32 = 8921;
const STARFIRE: u32 = 2912;
const SHADOW_BOLT: u32 = 686;
const RAIN_OF_FIRE: u32 = 5740;
const FIREBALL: u32 = 133;
const FLAMESTRIKE: u32 = 2120;
const FROST_NOVA: u32 = 122;
const BLIZZARD: u32 = 10;
const BLINK: u32 = 1953;
const CLEAVE: u32 = 845;
const THUNDERCLAP: u32 = 6343;
const CHARGE: u32 = 100;
const POISON: u32 = 3815;
const ARCANE: u32 = 1449;
const MULTI_SHOT: u32 = 2643;
const TRANQUILITY: u32 = 740;
const HOWL: u32 = 5484;
const BOMB: u32 = 13812;
const ROAR: u32 = 99;
const LIGHTNING: u32 = 403;
const HAMMER: u32 = 853;
const VENOM: u32 = 6917;
const SWOOP: u32 = 5708;
const CONSECRATION: u32 = 26573;

mod story {
    include!("story_bosses.rs");
}

/// A mid-boss's two moves, both from the start of the fight.
macro_rules! pair {
    ($a:expr, $b:expr) => {
        Kit {
            phases: &[Phase {
                below: 1.0,
                yell: "",
                ops: &[Add($a), Add($b)],
            }],
            enrage: 0.0,
        }
    };
}

/// The signature pairs; the first six are the starting zones'.
static PAIRS: [Kit; 15] = [
    pair!(
        mv(1, Throw { radius: 4.0 }, ROCK, 7.0, 3.0, 0.9),
        mv(2, Charge { twice: false }, CHARGE, 11.0, 6.0, 1.0)
    ),
    pair!(
        mv(1, Stomp { radius: 7.0 }, STOMP, 10.0, 4.0, 1.0),
        mv(2, Bombs { count: 3, radius: 3.5 }, BOMB, 9.0, 7.0, 0.9)
    ),
    pair!(
        mv(1, Cone { radius: 8.0, degrees: 90.0 }, CLEAVE, 7.0, 3.0, 1.1),
        mv(2, Pool { radius: 4.5, secs: 5.0 }, POISON, 12.0, 6.0, 0.7)
    ),
    pair!(
        mv(1, Volley { count: 3, length: 18.0, spread: 25.0 }, MULTI_SHOT, 9.0, 4.0, 0.9),
        mv(2, Stomp { radius: 7.0 }, STOMP, 12.0, 8.0, 1.0)
    ),
    pair!(
        mv(1, Charge { twice: true }, CHARGE, 13.0, 5.0, 0.9),
        mv(2, Cone { radius: 7.0, degrees: 80.0 }, CLEAVE, 7.0, 3.0, 1.0)
    ),
    pair!(
        mv(1, Throw { radius: 3.5 }, VENOM, 5.0, 2.0, 0.8),
        mv(2, Pool { radius: 4.0, secs: 6.0 }, POISON, 13.0, 7.0, 0.6)
    ),
    pair!(
        mv(1, Soak { radius: 4.0, need: 2 }, ARCANE, 16.0, 6.0, 1.6),
        mv(2, Bombs { count: 4, radius: 3.5 }, FIREBALL, 9.0, 3.0, 0.9)
    ),
    pair!(
        mv(1, Blink, BLINK, 11.0, 5.0, 0.0),
        mv(2, Cone { radius: 8.0, degrees: 90.0 }, CLEAVE, 6.0, 2.0, 1.1)
    ),
    pair!(
        mv(1, Nova { inner: 5.0, outer: 14.0 }, FROST_NOVA, 13.0, 6.0, 1.3),
        mv(2, Throw { radius: 4.0 }, ROCK, 6.0, 2.0, 0.9)
    ),
    pair!(
        mv(1, Vortex { radius: 7.0, pull: 6.0 }, STOMP, 15.0, 6.0, 1.2),
        mv(2, Pool { radius: 4.5, secs: 5.0 }, FLAMESTRIKE, 10.0, 3.0, 0.6)
    ),
    pair!(
        mv(1, Fixate { secs: 6.0, haste: 1.3 }, ROAR, 18.0, 5.0, 0.0),
        mv(2, Stomp { radius: 7.0 }, STOMP, 9.0, 3.0, 1.0)
    ),
    pair!(
        mv(1, Beam { count: 2, length: 20.0, step: 90.0 }, ARCANE, 14.0, 6.0, 1.2),
        mv(2, Throw { radius: 4.0 }, ROCK, 7.0, 2.0, 0.9)
    ),
    pair!(
        mv(1, SafeZone { radius: 16.0, gaps: 3 }, BLIZZARD, 17.0, 8.0, 1.4),
        mv(2, Charge { twice: false }, CHARGE, 10.0, 3.0, 1.0)
    ),
    pair!(
        mv(1, Mend { rate: 0.02, secs: 6.0, breaks: 0.05 }, TRANQUILITY, 24.0, 10.0, 0.0),
        mv(2, Bombs { count: 3, radius: 3.5 }, BOMB, 8.0, 3.0, 0.9)
    ),
    pair!(
        mv(1, Summon { count: 3 }, 0, 18.0, 4.0, 0.0),
        mv(2, Volley { count: 4, length: 18.0, spread: 20.0 }, MULTI_SHOT, 9.0, 6.0, 0.9)
    ),
];

/// Teldrassil: moonfire and owlbeasts, then starfall with two gaps, then a rain of moonfire.
static GNARLPINE_AUGUR: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Throw { radius: 4.0 }, MOONFIRE, 6.0, 2.0, 1.0)),
                Add(mv(2, Summon { count: 3 }, 0, 18.0, 8.0, 0.0)),
            ],
        },
        Phase {
            below: 0.6,
            yell: "The stars answer my call!",
            ops: &[Add(mv(3, SafeZone { radius: 16.0, gaps: 2 }, STARFIRE, 16.0, 0.0, 1.4))],
        },
        Phase {
            below: 0.25,
            yell: "Teldrassil will be ours!",
            ops: &[Remove(2), Add(mv(4, Bombs { count: 4, radius: 3.5 }, MOONFIRE, 7.0, 1.0, 0.9))],
        },
    ],
    enrage: 0.0,
};

/// Dun Morogh: stomps and charges, then avalanche lanes, then a wider fan of them.
static BJARN: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Stomp { radius: 7.0 }, STOMP, 10.0, 3.0, 1.0)),
                Add(mv(2, Charge { twice: false }, CHARGE, 12.0, 6.0, 1.0)),
            ],
        },
        Phase {
            below: 0.6,
            yell: "Rrraaargh! The snow above shudders!",
            ops: &[Add(mv(3, Volley { count: 3, length: 20.0, spread: 25.0 }, BLIZZARD, 11.0, 0.0, 1.0))],
        },
        Phase {
            below: 0.25,
            yell: "Graaaah!",
            ops: &[
                Remove(2),
                Remove(3),
                Add(mv(4, Volley { count: 5, length: 22.0, spread: 18.0 }, BLIZZARD, 8.0, 0.0, 1.0)),
            ],
        },
    ],
    enrage: 0.0,
};

/// Mulgore: a bite and the pack, then the hunt, then he is everywhere at once.
static GHOST_HOWL: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Cone { radius: 7.0, degrees: 90.0 }, CLEAVE, 6.0, 2.0, 1.1)),
                Add(mv(2, Summon { count: 3 }, 0, 16.0, 6.0, 0.0)),
            ],
        },
        Phase {
            below: 0.6,
            yell: "Aroooooo!",
            ops: &[Add(mv(3, Fixate { secs: 6.0, haste: 1.3 }, HOWL, 18.0, 0.0, 0.0))],
        },
        Phase {
            below: 0.25,
            yell: "Aroooooo! The pack closes in!",
            ops: &[
                Remove(2),
                Add(mv(4, Blink, BLINK, 10.0, 0.0, 0.0)),
                Add(mv(5, Charge { twice: false }, CHARGE, 9.0, 4.0, 1.0)),
                Add(mv(6, Summon { count: 5 }, 0, 20.0, 2.0, 0.0)),
            ],
        },
    ],
    enrage: 0.0,
};

/// Tirisfal: charges and bile, then he gorges unless struck hard, then the bile never stops.
static GOREFANG: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Charge { twice: false }, CHARGE, 9.0, 3.0, 1.0)),
                Add(mv(2, Pool { radius: 4.5, secs: 5.0 }, POISON, 11.0, 6.0, 0.6)),
            ],
        },
        Phase {
            below: 0.5,
            yell: "Grrr... flesh!",
            ops: &[Add(mv(3, Mend { rate: 0.03, secs: 6.0, breaks: 0.06 }, TRANQUILITY, 25.0, 0.0, 0.0))],
        },
        Phase {
            below: 0.2,
            yell: "Graaaah!",
            ops: &[Remove(2), Add(mv(4, Pool { radius: 4.5, secs: 5.0 }, POISON, 6.0, 1.0, 0.6))],
        },
    ],
    enrage: 0.0,
};

/// Durotar: shadow bolts and imps, then a fel shield behind two guards, then rain of fire.
static BURNING_BLADE_CULTIST: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Throw { radius: 3.5 }, SHADOW_BOLT, 5.0, 2.0, 0.9)),
                Add(mv(2, Summon { count: 2 }, 0, 15.0, 6.0, 0.0)),
            ],
        },
        Phase {
            below: 0.6,
            yell: "The Burning Blade protects its own!",
            ops: &[Add(mv(3, Shield { count: 2 }, 0, 40.0, 0.0, 0.0))],
        },
        Phase {
            below: 0.25,
            yell: "Burn in the Legion's fire!",
            ops: &[
                Remove(3),
                Add(mv(4, Bombs { count: 5, radius: 3.5 }, RAIN_OF_FIRE, 6.0, 1.0, 0.8)),
                Add(mv(5, Shield { count: 2 }, 0, 40.0, 3.0, 0.0)),
            ],
        },
    ],
    enrage: 0.0,
};

/// Westfall: two swoops and carrion, then the wind drags you in, then dives from every side.
static VULTROS: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Charge { twice: true }, SWOOP, 12.0, 4.0, 0.9)),
                Add(mv(2, Throw { radius: 4.0 }, VENOM, 7.0, 2.0, 0.9)),
            ],
        },
        Phase {
            below: 0.66,
            yell: "Skreeee!",
            ops: &[Add(mv(3, Vortex { radius: 8.0, pull: 6.0 }, STOMP, 15.0, 0.0, 1.2))],
        },
        Phase {
            below: 0.33,
            yell: "Skreeee! Skreeee!",
            ops: &[
                Add(mv(4, Blink, BLINK, 9.0, 0.0, 0.0)),
                Add(mv(5, Volley { count: 4, length: 20.0, spread: 20.0 }, SWOOP, 10.0, 3.0, 1.0)),
            ],
        },
    ],
    enrage: 300.0,
};

/// The Barrens: cleave and thunder, then a nova to run into, then lightning sweeps and Kolkar.
static HEZRUL_BLOODMARK: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Cone { radius: 8.0, degrees: 90.0 }, CLEAVE, 6.0, 2.0, 1.1)),
                Add(mv(2, Stomp { radius: 7.0 }, THUNDERCLAP, 11.0, 6.0, 1.0)),
            ],
        },
        Phase {
            below: 0.66,
            yell: "You will not leave the Barrens alive!",
            ops: &[Remove(2), Add(mv(3, Nova { inner: 4.0, outer: 14.0 }, THUNDERCLAP, 14.0, 0.0, 1.3))],
        },
        Phase {
            below: 0.33,
            yell: "Kolkar, to me!",
            ops: &[
                Add(mv(4, Beam { count: 3, length: 20.0, step: 60.0 }, LIGHTNING, 16.0, 2.0, 1.2)),
                Add(mv(5, Summon { count: 3 }, 0, 18.0, 0.0, 0.0)),
            ],
        },
    ],
    enrage: 300.0,
};

/// Hall of Thanes: hammers, then a dirge rune to stand in together, then two runes and beams.
static DURGEN_DIRGEHAMMER: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Throw { radius: 4.0 }, HAMMER, 7.0, 2.0, 1.0)),
                Add(mv(2, Charge { twice: false }, CHARGE, 11.0, 5.0, 1.0)),
            ],
        },
        Phase {
            below: 0.6,
            yell: "Hear the dirge of the Thanes!",
            ops: &[Add(mv(3, Soak { radius: 4.0, need: 2 }, CONSECRATION, 15.0, 0.0, 1.6))],
        },
        Phase {
            below: 0.3,
            yell: "Into the earth with you!",
            ops: &[
                Add(mv(4, Beam { count: 2, length: 22.0, step: 90.0 }, HAMMER, 14.0, 3.0, 1.2)),
                Add(mv(5, Soak { radius: 4.0, need: 2 }, CONSECRATION, 15.0, 7.0, 1.6)),
            ],
        },
    ],
    enrage: 300.0,
};

/// The Deadmines: blades and pirates, then he is behind you, then the cannons and his guard.
static EDWIN_VANCLEEF: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Cone { radius: 7.0, degrees: 80.0 }, CLEAVE, 6.0, 2.0, 1.1)),
                Add(mv(2, Summon { count: 3 }, 0, 15.0, 5.0, 0.0)),
            ],
        },
        Phase {
            below: 0.66,
            yell: "Lapdogs, all of you!",
            ops: &[
                Add(mv(3, Blink, BLINK, 11.0, 0.0, 0.0)),
                Add(mv(4, Fixate { secs: 5.0, haste: 1.25 }, ROAR, 20.0, 6.0, 0.0)),
            ],
        },
        Phase {
            below: 0.33,
            yell: "None may challenge the Brotherhood!",
            ops: &[
                Add(mv(5, SafeZone { radius: 18.0, gaps: 2 }, FIREBALL, 15.0, 0.0, 1.5)),
                Add(mv(6, Shield { count: 2 }, 0, 45.0, 3.0, 0.0)),
            ],
        },
    ],
    enrage: 300.0,
};

/// Wailing Caverns: stomps and rocks, then the cavern closes in, then the pull and the runes.
static MUTANUS: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Stomp { radius: 8.0 }, STOMP, 9.0, 3.0, 1.0)),
                Add(mv(2, Throw { radius: 4.0 }, ROCK, 7.0, 5.0, 0.9)),
            ],
        },
        Phase {
            below: 0.66,
            yell: "The nightmare closes around you!",
            ops: &[Add(mv(3, Closing { from: 24.0, to: 10.0, secs: 10.0 }, FROST_NOVA, 30.0, 0.0, 1.0))],
        },
        Phase {
            below: 0.33,
            yell: "The Devourer feeds!",
            ops: &[
                Add(mv(4, Vortex { radius: 8.0, pull: 7.0 }, STOMP, 13.0, 2.0, 1.2)),
                Add(mv(5, Soak { radius: 4.0, need: 2 }, ARCANE, 17.0, 6.0, 1.6)),
            ],
        },
    ],
    enrage: 330.0,
};

/// Stonetalon: bombs and the shredder, then saw beams and turrets, then all of it behind a ward.
static GERENZO_WRENCHWHISTLE: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Bombs { count: 4, radius: 3.5 }, BOMB, 8.0, 2.0, 0.9)),
                Add(mv(2, Charge { twice: false }, CHARGE, 12.0, 5.0, 1.0)),
            ],
        },
        Phase {
            below: 0.66,
            yell: "Start the saws!",
            ops: &[
                Add(mv(3, Beam { count: 2, length: 22.0, step: 90.0 }, FLAMESTRIKE, 13.0, 0.0, 1.2)),
                Add(mv(4, Volley { count: 3, length: 20.0, spread: 25.0 }, MULTI_SHOT, 9.0, 4.0, 0.9)),
            ],
        },
        Phase {
            below: 0.33,
            yell: "Full power! Protect me, you fools!",
            ops: &[
                Remove(3),
                Remove(4),
                Add(mv(5, Beam { count: 3, length: 24.0, step: 60.0 }, FLAMESTRIKE, 10.0, 2.0, 1.3)),
                Add(mv(6, Shield { count: 3 }, 0, 50.0, 0.0, 0.0)),
                Add(mv(7, Volley { count: 6, length: 22.0, spread: 15.0 }, MULTI_SHOT, 9.0, 5.0, 0.9)),
            ],
        },
    ],
    enrage: 300.0,
};

/// Duskwood: bites and howls, then the pack while he feeds, then he hunts you down.
static NEFARU: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Cone { radius: 8.0, degrees: 90.0 }, CLEAVE, 6.0, 2.0, 1.1)),
                Add(mv(2, Nova { inner: 5.0, outer: 14.0 }, HOWL, 13.0, 6.0, 1.2)),
            ],
        },
        Phase {
            below: 0.66,
            yell: "The pack answers my call!",
            ops: &[
                Add(mv(3, Summon { count: 4 }, 0, 16.0, 0.0, 0.0)),
                Add(mv(4, Mend { rate: 0.025, secs: 6.0, breaks: 0.08 }, TRANQUILITY, 22.0, 6.0, 0.0)),
            ],
        },
        Phase {
            below: 0.33,
            yell: "Your blood is mine!",
            ops: &[
                Remove(4),
                Add(mv(5, Blink, BLINK, 10.0, 0.0, 0.0)),
                Add(mv(6, Charge { twice: true }, CHARGE, 12.0, 3.0, 1.0)),
            ],
        },
    ],
    enrage: 300.0,
};

/// Stranglethorn: pounces from nowhere, then charges, roars and the pull, then the jungle burns.
static KING_BANGALASH: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Blink, BLINK, 10.0, 4.0, 0.0)),
                Add(mv(2, Cone { radius: 8.0, degrees: 90.0 }, CLEAVE, 6.0, 2.0, 1.2)),
                Add(mv(3, Summon { count: 2 }, 0, 18.0, 8.0, 0.0)),
            ],
        },
        Phase {
            below: 0.66,
            yell: "Rrrrrrr...",
            ops: &[
                Add(mv(4, Charge { twice: true }, CHARGE, 11.0, 0.0, 1.0)),
                Add(mv(5, Nova { inner: 5.0, outer: 15.0 }, ROAR, 14.0, 4.0, 1.3)),
                Add(mv(6, Vortex { radius: 8.0, pull: 7.0 }, STOMP, 17.0, 8.0, 1.2)),
            ],
        },
        Phase {
            below: 0.33,
            yell: "RRRAAAWR! The jungle burns!",
            ops: &[
                Remove(4),
                Remove(5),
                Add(mv(7, SafeZone { radius: 20.0, gaps: 2 }, FLAMESTRIKE, 14.0, 0.0, 1.6)),
                Add(mv(8, Beam { count: 2, length: 24.0, step: 90.0 }, FIREBALL, 12.0, 4.0, 1.3)),
                Add(mv(9, Fixate { secs: 6.0, haste: 1.3 }, ROAR, 20.0, 7.0, 0.0)),
            ],
        },
    ],
    enrage: 360.0,
};

/// Any other last boss: rocks and stomps, then a nova, then fire with gaps and the horde.
static FINALE: Kit = Kit {
    phases: &[
        Phase {
            below: 1.0,
            yell: "",
            ops: &[
                Add(mv(1, Throw { radius: 4.0 }, ROCK, 7.0, 2.0, 1.0)),
                Add(mv(2, Stomp { radius: 7.0 }, STOMP, 11.0, 5.0, 1.0)),
            ],
        },
        Phase {
            below: 0.6,
            yell: "Enough!",
            ops: &[Add(mv(3, Nova { inner: 5.0, outer: 14.0 }, FROST_NOVA, 14.0, 0.0, 1.3))],
        },
        Phase {
            below: 0.3,
            yell: "You will fall here!",
            ops: &[
                Add(mv(4, SafeZone { radius: 18.0, gaps: 2 }, FLAMESTRIKE, 16.0, 0.0, 1.5)),
                Add(mv(5, Summon { count: 3 }, 0, 18.0, 4.0, 0.0)),
            ],
        },
    ],
    enrage: 300.0,
};

/// A last boss's own three phases.
fn finale(boss: &str) -> &'static Kit {
    if let Some(kit) = story::finale(boss) { return kit; }
    match boss {
        "Gnarlpine Augur" => &GNARLPINE_AUGUR,
        "Bjarn" => &BJARN,
        "Ghost Howl" => &GHOST_HOWL,
        "Gorefang" => &GOREFANG,
        "Burning Blade Cultist" => &BURNING_BLADE_CULTIST,
        "Vultros" => &VULTROS,
        "Hezrul Bloodmark" => &HEZRUL_BLOODMARK,
        "Durgen Dirgehammer" => &DURGEN_DIRGEHAMMER,
        "Edwin VanCleef" => &EDWIN_VANCLEEF,
        "Mutanus the Devourer" => &MUTANUS,
        "Gerenzo Wrenchwhistle" => &GERENZO_WRENCHWHISTLE,
        "Nefaru" => &NEFARU,
        "King Bangalash" => &KING_BANGALASH,
        _ => &FINALE,
    }
}

/// The script of a battleground's boss `index` of `count`: the last boss's finale, or a
/// signature pair dealt by the zone so its bosses never share one.
pub(super) fn kit(zone: &str, boss: &str, index: usize, count: usize, tier: u8) -> &'static Kit {
    if index + 1 >= count {
        return finale(boss);
    }
    let pool = if tier <= 1 { &PAIRS[..6] } else { &PAIRS[..] };
    let seed = zone
        .bytes()
        .fold(7usize, |h, b| h.wrapping_mul(31).wrapping_add(usize::from(b)));
    &pool[(seed + index) % pool.len()]
}

#[cfg(test)]
mod tests {
    use super::super::{data, difficulty, mechanics::Op};
    use super::*;

    fn moves(kit: &Kit) -> impl Iterator<Item = super::super::mechanics::Move> + '_ {
        kit.phases.iter().flat_map(|p| p.ops).filter_map(|op| match op {
            Op::Add(m) => Some(*m),
            Op::Remove(_) => None,
        })
    }

    #[test]
    fn every_last_boss_has_its_own_three_phases() {
        for (index, map) in data::MAPS.iter().enumerate() {
            if map.name == "Elwynn Forest" {
                continue;
            }
            let roster = data::roster(index);
            let last = roster.bosses.last().unwrap();
            let kit = last.script.expect(map.name);
            assert!(!std::ptr::eq(kit, &FINALE), "{} has no finale of its own", last.name);
            assert!(kit.phases.len() >= 3, "{}", last.name);
            assert!(kit.phases.windows(2).all(|w| w[0].below > w[1].below), "{}", last.name);
            assert!(kit.phases[1..].iter().all(|p| !p.yell.is_empty()), "{}", last.name);
        }
    }

    #[test]
    fn no_two_bosses_of_a_battleground_fight_alike() {
        for (index, map) in data::MAPS.iter().enumerate() {
            if map.name == "Elwynn Forest" {
                continue;
            }
            let roster = data::roster(index);
            let kits: Vec<_> = roster.bosses.iter().map(|b| b.script.expect(b.name)).collect();
            for (i, a) in kits.iter().enumerate() {
                for b in &kits[i + 1..] {
                    assert!(!std::ptr::eq(*a, *b), "{}", map.name);
                }
            }
            // Bosses carry no Elwynn phases alongside their script.
            assert!(roster.bosses.iter().all(|b| b.phases.is_empty()), "{}", map.name);
            // A starting zone's bosses deal from the simpler pairs.
            if difficulty::tier(map.name) == Some(1) {
                for kit in &kits[..kits.len() - 1] {
                    let at = PAIRS.iter().position(|p| std::ptr::eq(p, *kit)).unwrap();
                    assert!(at < 6, "{}", map.name);
                }
            }
        }
    }

    #[test]
    fn every_move_is_drawable_and_lands_somewhere() {
        let all = PAIRS.iter().chain([
            &GNARLPINE_AUGUR, &BJARN, &GHOST_HOWL, &GOREFANG, &BURNING_BLADE_CULTIST, &VULTROS,
            &HEZRUL_BLOODMARK, &DURGEN_DIRGEHAMMER, &EDWIN_VANCLEEF, &MUTANUS,
            &GERENZO_WRENCHWHISTLE, &NEFARU, &KING_BANGALASH, &FINALE,
        ]);
        for kit in all {
            assert!(kit.phases[0].below >= 1.0 && kit.phases[0].ops.len() >= 2);
            for m in moves(kit) {
                assert!(m.every > 0.0 && m.first >= 0.0, "{m:?}");
                match m.verb {
                    // The telegraph art draws 90- and 80-degree cones.
                    Cone { degrees, .. } => assert!(degrees == 90.0 || degrees == 80.0),
                    SafeZone { gaps, .. } => assert!((1..=4).contains(&gaps)),
                    Beam { count, .. } | Volley { count, .. } => assert!(count > 0),
                    Nova { inner, outer } => assert!(inner < outer),
                    Closing { from, to, .. } => assert!(to < from),
                    _ => {}
                }
                let hits = !matches!(
                    m.verb,
                    Summon { .. } | Shield { .. } | Fixate { .. } | Mend { .. } | Blink
                );
                assert_eq!(hits, m.power > 0.0, "{m:?}");
            }
        }
    }
}
