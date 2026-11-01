//! `rustc --edition 2021 --test scripts/race_rules.rs -o target/race-rules.exe`
mod data {
    pub struct ClassDef {
        pub id: u8,
        pub looks: &'static [u32],
    }
}
mod entities {
    use std::collections::HashMap;
    pub struct Creatures(pub HashMap<u32, (u8, u8)>);
    impl Creatures {
        pub fn display_race_sex(&self, display: u32) -> Option<(u8, u8)> {
            self.0.get(&display).copied()
        }
    }
}
#[path = "../crates/benilla-app/src/survivors/races.rs"]
mod races;

#[test]
fn roster_keeps_sparse_race_ids_and_both_portrait_rows() {
    let mut ids = races::RACES.iter().map(|r| r.id).collect::<Vec<_>>();
    ids.sort_unstable();
    assert_eq!(
        ids,
        [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 22, 24, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 52,
            84, 86
        ]
    );
    let mut cells = std::collections::HashSet::new();
    for race in races::RACES {
        assert_ne!(races::race_name(race.id), "Unknown");
        assert!(race.faction <= 2);
        for sex in 0..=1 {
            let tc = races::race_tc(race.id, sex).unwrap();
            assert!(tc.iter().all(|n| (0.0..=1.0).contains(n)));
            assert_eq!(tc[1] - tc[0], 1.0 / 16.0);
            assert_eq!(tc[3] - tc[2], 0.25);
            assert!(cells.insert(((tc[0] * 1024.0) as u32, (tc[2] * 256.0) as u32)));
        }
    }
    assert!(races::race_tc(12, 0).is_none());
}

#[test]
fn each_class_exposes_exactly_its_supported_races_and_both_body_types() {
    let mut displays = std::collections::HashMap::new();
    for class in [1, 2, 3, 4, 5, 7, 8, 9, 11] {
        for race in races::RACES {
            for sex in 0..=1 {
                displays.insert(
                    300000 + u32::from(class) * 200 + u32::from(race.id) * 2 + u32::from(sex),
                    (race.id, sex),
                );
            }
        }
        let looks = races::looks_for(
            &data::ClassDef {
                id: class,
                looks: &[],
            },
            &entities::Creatures(displays.clone()),
        );
        for race in races::RACES {
            let body_types: Vec<_> = looks
                .iter()
                .filter(|l| l.1 == race.id)
                .map(|l| l.2)
                .collect();
            assert_eq!(
                body_types,
                if race.classes.contains(&class) {
                    vec![0, 1]
                } else {
                    vec![]
                }
            );
        }
    }
}

#[test]
fn missing_or_mismatched_imports_do_not_offer_invisible_characters() {
    let class = data::ClassDef {
        id: 8,
        looks: &[123, 124],
    };
    let catalog = entities::Creatures([(123, (1, 0)), (124, (1, 0)), (301620, (1, 0))].into());
    assert_eq!(races::looks_for(&class, &catalog), vec![(123, 1, 0)]);
    assert!(races::looks_for(&class, &entities::Creatures(Default::default())).is_empty());
}
