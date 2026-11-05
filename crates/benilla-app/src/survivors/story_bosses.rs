// Encounter scripts for the story extension, using the shared telegraphed mechanics.
use super::*;

macro_rules! encounter {
    ($name:ident, $y1:literal, $y2:literal, $a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => {
        static $name: Kit = Kit {
            phases: &[
                Phase { below: 1.0, yell: "", ops: &[Add($a), Add($b)] },
                Phase { below: 0.65, yell: $y1, ops: &[Add($c)] },
                Phase { below: 0.3, yell: $y2, ops: &[Remove(2), Add($d), Add($e)] },
            ],
            enrage: 360.0,
        };
    };
}

encounter!(WHITEMANE, "Arise, champions of the Crusade!", "The Light will judge you!",
    mv(1, Throw { radius: 3.5 }, HAMMER, 7.0, 3.0, 1.0),
    mv(2, Shield { count: 2 }, 0, 32.0, 12.0, 0.0),
    mv(3, Mend { rate: 0.015, secs: 5.0, breaks: 0.04 }, TRANQUILITY, 25.0, 5.0, 0.0),
    mv(4, Beam { count: 3, length: 20.0, step: 60.0 }, CONSECRATION, 12.0, 3.0, 1.2),
    mv(5, Volley { count: 3, length: 18.0, spread: 25.0 }, HAMMER, 10.0, 6.0, 1.1));

encounter!(DESERT_COLOSSUS, "The sands rise around you!", "The desert will bury you!",
    mv(1, Stomp { radius: 8.0 }, STOMP, 11.0, 4.0, 1.0),
    mv(2, Throw { radius: 4.0 }, ROCK, 8.0, 2.0, 1.0),
    mv(3, Vortex { radius: 16.0, pull: 4.0 }, STOMP, 20.0, 7.0, 1.1),
    mv(4, Bombs { count: 4, radius: 3.5 }, ROCK, 10.0, 3.0, 1.1),
    mv(5, Closing { from: 24.0, to: 12.0, secs: 7.0 }, STOMP, 26.0, 9.0, 0.7));

encounter!(KING_MOSH, "The king's roar shakes the crater!", "The hunt becomes a frenzy!",
    mv(1, Cone { radius: 9.0, degrees: 90.0 }, CLEAVE, 8.0, 3.0, 1.1),
    mv(2, Charge { twice: false }, CHARGE, 14.0, 6.0, 1.0),
    mv(3, Summon { count: 3 }, 0, 24.0, 5.0, 0.0),
    mv(4, Fixate { secs: 5.0, haste: 1.25 }, ROAR, 18.0, 7.0, 0.0),
    mv(5, Stomp { radius: 11.0 }, STOMP, 12.0, 3.0, 1.2));

encounter!(BLACKROCK_FIRELORD, "Blackrock burns!", "You will never reach the Core!",
    mv(1, Pool { radius: 4.0, secs: 5.0 }, FLAMESTRIKE, 12.0, 4.0, 0.7),
    mv(2, Throw { radius: 4.0 }, FIREBALL, 7.0, 2.0, 1.0),
    mv(3, Summon { count: 3 }, 0, 24.0, 6.0, 0.0),
    mv(4, Nova { inner: 5.0, outer: 18.0 }, FLAMESTRIKE, 16.0, 3.0, 1.2),
    mv(5, Beam { count: 3, length: 24.0, step: 60.0 }, FIREBALL, 13.0, 8.0, 1.2));

encounter!(BROOD_GUARDIAN, "The trail ends here!", "The brood will endure!",
    mv(1, Pool { radius: 4.0, secs: 6.0 }, POISON, 12.0, 4.0, 0.7),
    mv(2, Stomp { radius: 8.0 }, STOMP, 10.0, 6.0, 1.0),
    mv(3, Shield { count: 3 }, 0, 30.0, 5.0, 0.0),
    mv(4, Volley { count: 4, length: 20.0, spread: 25.0 }, VENOM, 12.0, 3.0, 1.0),
    mv(5, Vortex { radius: 15.0, pull: 4.0 }, POISON, 22.0, 8.0, 1.1));

encounter!(UKORZ, "Sandfury, defend your chief!", "The sands will run red!",
    mv(1, Cone { radius: 8.0, degrees: 100.0 }, CLEAVE, 8.0, 3.0, 1.1),
    mv(2, Summon { count: 2 }, 0, 23.0, 10.0, 0.0),
    mv(3, Charge { twice: true }, CHARGE, 17.0, 6.0, 1.0),
    mv(4, Stomp { radius: 9.0 }, STOMP, 12.0, 4.0, 1.2),
    mv(5, Fixate { secs: 6.0, haste: 1.3 }, ROAR, 22.0, 8.0, 0.0));

encounter!(ONYXIA, "The brood answers its mother!", "This lair will be your grave!",
    mv(1, Cone { radius: 14.0, degrees: 80.0 }, FIREBALL, 10.0, 4.0, 1.3),
    mv(2, Stomp { radius: 10.0 }, STOMP, 15.0, 8.0, 1.0),
    mv(3, Summon { count: 4 }, 0, 22.0, 4.0, 0.0),
    mv(4, Beam { count: 3, length: 28.0, step: 60.0 }, FLAMESTRIKE, 16.0, 3.0, 1.5),
    mv(5, Nova { inner: 6.0, outer: 20.0 }, FIREBALL, 18.0, 9.0, 1.3));

encounter!(RAGNAROS, "Servants of flame, rise!", "The Core erupts around you!",
    mv(1, Pool { radius: 4.5, secs: 6.0 }, FLAMESTRIKE, 12.0, 4.0, 0.8),
    mv(2, Throw { radius: 4.0 }, FIREBALL, 8.0, 2.0, 1.1),
    mv(3, Shield { count: 4 }, 0, 35.0, 6.0, 0.0),
    mv(4, Nova { inner: 6.0, outer: 22.0 }, FLAMESTRIKE, 17.0, 3.0, 1.4),
    mv(5, SafeZone { radius: 24.0, gaps: 2 }, FIREBALL, 22.0, 10.0, 1.5));

pub(super) fn finale(boss: &str) -> Option<&'static Kit> {
    Some(match boss {
        "High Inquisitor Whitemane" => &WHITEMANE,
        "Desert Colossus" => &DESERT_COLOSSUS,
        "King Mosh" => &KING_MOSH,
        "Blackrock Firelord" => &BLACKROCK_FIRELORD,
        "Brood Trail Guardian" => &BROOD_GUARDIAN,
        "Chief Ukorz Sandscalp" => &UKORZ,
        "Onyxia" => &ONYXIA,
        "Ragnaros" => &RAGNAROS,
        _ => return None,
    })
}
