//! Elwynn's numbers pinned before the zone tiers: every curve, roster row and spawner table as
//! the ported native rules give them. Tiers and rosters for other zones must leave this unchanged.
//!
//! `GOLDEN_UPDATE=1 cargo test -p benilla-app golden` rewrites the snapshot after a deliberate
//! change to Elwynn.

use std::fmt::Write;

use super::{data, enemies, horde_spawning};

const TIMES: [f32; 6] = [0.0, 60.0, 180.0, 540.0, 900.0, 1200.0];
const SNAPSHOT: &str = include_str!("golden_elwynn.txt");

fn row(out: &mut String, def: &data::EnemyDef) {
    let ranged = def.ranged.map_or(0, |r| r.spell);
    writeln!(
        out,
        "{} #{} hp {} dmg {} speed {} swing {} scale {} xp {} level {} from {} cost {} big {} roam {} last {} ranged {} phases {}",
        def.name, def.display, def.hp, def.damage, def.speed, def.attack, def.scale, def.xp,
        def.level, def.from, horde_spawning::cost(def), def.big, def.roaming_elite, def.last,
        ranged, def.phases.len(),
    )
    .unwrap();
}

fn snapshot() -> String {
    let roster = data::roster(0);
    let mut out = String::new();
    writeln!(out, "# roster").unwrap();
    for def in roster.enemies.iter().chain(&roster.bosses).chain(&roster.guardians) {
        row(&mut out, def);
    }
    writeln!(out, "# spawn stats (hp, damage, speed) by run second").unwrap();
    for &t in &TIMES {
        for def in &roster.enemies {
            let (hp, damage, speed) = enemies::spawn_stats(def, enemies::Role::Horde, t, 0);
            writeln!(out, "horde {} @{t}: {hp:.3} {damage:.4} {speed:.3}", def.name).unwrap();
        }
        for def in roster.guardians.iter().filter(|d| d.roaming_elite) {
            let (hp, damage, speed) = enemies::spawn_stats(def, enemies::Role::Roaming, t, 0);
            writeln!(out, "roaming {} @{t}: {hp:.3} {damage:.4} {speed:.3}", def.name).unwrap();
        }
        for def in &roster.guardians {
            for slain in [0, 2] {
                let (hp, damage, _) = enemies::spawn_stats(def, enemies::Role::Lair, t, slain);
                writeln!(out, "lair {} @{t} slain {slain}: {hp:.3} {damage:.4}", def.name).unwrap();
            }
        }
    }
    for def in &roster.bosses {
        let (hp, damage, speed) = enemies::spawn_stats(def, enemies::Role::Boss, def.from, 0);
        writeln!(out, "boss {}: {hp:.3} {damage:.4} {speed:.3}", def.name).unwrap();
    }
    writeln!(out, "# spawner").unwrap();
    for &t in &TIMES {
        writeln!(
            out,
            "@{t}: rate {:.4} heated {:.4} boss {:.4} floor {} {} elite every {:.2}",
            horde_spawning::rate(t, 0.0, 1.0),
            horde_spawning::rate(t, 30.0, 1.0),
            horde_spawning::rate(t, 0.0, 0.45),
            horde_spawning::population_floor(t, 1.0, 1.0),
            horde_spawning::population_floor(t, 1.0, horde_spawning::party_multiplier(3)),
            horde_spawning::elite_interval(t),
        )
        .unwrap();
    }
    for living in 1..=4 {
        let party = horde_spawning::party_multiplier(living);
        writeln!(out, "party {living}: x{party:.4} cap {}", (50.0 * party).round() as usize).unwrap();
    }
    out
}

/// Every other battleground runs on the same tables: a priced horde from the first second, a
/// boss clock ending in the last boss at 16:00, four elite slots, and its road's tier.
#[test]
fn every_zone_runs_on_elwynn_tables() {
    for (index, map) in data::MAPS.iter().enumerate() {
        let roster = data::roster(index);
        let tier = super::difficulty::tier(map.name).expect(map.name);
        assert!(roster.enemies.iter().all(|d| d.cost > 0.0 && d.tier == tier), "{}", map.name);
        assert_eq!(roster.enemies[0].from, 0.0, "{}", map.name);
        assert!(roster.enemies.windows(2).all(|w| w[0].from <= w[1].from), "{}", map.name);
        let last = roster.bosses.last().expect(map.name);
        assert!(last.last && last.from == 960.0, "{}", map.name);
        assert_eq!(roster.bosses.iter().filter(|b| b.last).count(), 1, "{}", map.name);
        assert!(roster.bosses.iter().all(|b| b.tier == tier && b.cost == 0.0), "{}", map.name);
        assert_eq!(roster.guardians.len(), 4, "{}", map.name);
        assert_eq!(roster.guardians.iter().map(|g| g.kit).collect::<Vec<_>>(), [1, 2, 3, 4]);
        assert_eq!(roster.guardians.iter().filter(|g| g.lord).count(), 1, "{}", map.name);
    }
}

/// `cargo test -p benilla-app zone_numbers -- --ignored --nocapture` prints each battleground's
/// spawn numbers at five and ten minutes.
#[test]
#[ignore]
fn zone_numbers() {
    for (index, map) in data::MAPS.iter().enumerate() {
        let roster = data::roster(index);
        let horde = |t: f32| {
            let rows: Vec<_> = roster.enemies.iter().filter(|d| d.from <= t).collect();
            let (hp, damage) = rows.iter().fold((0.0, 0.0), |(h, d), def| {
                let (hp, damage, _) = enemies::spawn_stats(def, enemies::Role::Horde, t, 0);
                (h + hp, d + damage)
            });
            (hp / rows.len() as f32, damage / rows.len() as f32)
        };
        let ((h5, d5), (h10, d10)) = (horde(300.0), horde(600.0));
        let elite = enemies::spawn_stats(&roster.guardians[0], enemies::Role::Roaming, 600.0, 0);
        let last = roster.bosses.last().unwrap();
        let boss = enemies::spawn_stats(last, enemies::Role::Boss, last.from, 0);
        println!(
            "{:<22} T{} horde 5:00 {h5:>6.0}/{d5:>4.1} 10:00 {h10:>6.0}/{d10:>4.1} | elite {} {:>6.0}/{:>4.1} | {} {:>7.0}/{:>4.1} | crowd {}",
            map.name, roster.enemies[0].tier, roster.guardians[0].name, elite.0, elite.1, last.name,
            boss.0, boss.1, roster.crowd,
        );
    }
}

#[test]
fn elwynn_numbers_are_unchanged() {
    let now = snapshot();
    if std::env::var_os("GOLDEN_UPDATE").is_some() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/survivors/golden_elwynn.txt");
        std::fs::write(path, &now).unwrap();
        return;
    }
    for (line, (want, got)) in SNAPSHOT.lines().zip(now.lines()).enumerate() {
        assert_eq!(want, got, "golden_elwynn.txt line {}", line + 1);
    }
    assert_eq!(SNAPSHOT.lines().count(), now.lines().count(), "golden_elwynn.txt length");
}
