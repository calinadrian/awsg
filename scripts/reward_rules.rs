//! Standalone original-reward arithmetic checks: rustc --test scripts/reward_rules.rs.
#[path = "../crates/benilla-app/src/survivors/amulets.rs"]
mod amulets;
#[path = "../crates/benilla-app/src/survivors/jewelry.rs"]
mod jewelry;
#[path = "../crates/benilla-app/src/survivors/rewards.rs"]
mod rewards;
#[test]
fn every_shared_item_matches_original_all_five_rarities() {
    let expected: [[f32; 5]; 23] = [
        [0.255, 0.35699999, 0.45899999, 0.61200005, 0.81600004],
        [0.13500001, 0.18900001, 0.243, 0.32400003, 0.43200004],
        [0.25261778, 0.33090505, 0.39859942, 0.48318818, 0.57019597],
        [0.06, 0.084, 0.108, 0.14399999, 0.192],
        [0.04420812, 0.05790838, 0.0697549, 0.08455794, 0.09978429],
        [2.25, 3.14999986, 4.04999971, 4.5, 4.5],
        [0.21000001, 0.294, 0.37800002, 0.50400007, 0.67200005],
        [0.15000001, 0.21000001, 0.26999998, 0.36000001, 0.40000001],
        [0.015, 0.021, 0.027, 0.03, 0.03],
        [0.19499999, 0.273, 0.35099998, 0.46799999, 0.62399995],
        [0.105, 0.147, 0.18900001, 0.25200003, 0.33600003],
        [0.20209423, 0.26472402, 0.31887954, 0.38655052, 0.45615676],
        [0.09, 0.126, 0.162, 0.21600002, 0.28799999],
        [0.29249999, 0.40949994, 0.52649999, 0.70200002, 0.93599999],
        [0.12, 0.168, 0.21600001, 0.28799999, 0.384],
        [0.52499998, 0.73499995, 0.94499999, 1.25999999, 1.68000007],
        [0.17999999, 0.252, 0.324, 0.43200004, 0.57599998],
        [0.375, 0.52499998, 0.67499995, 0.89999998, 1.20000005],
        [0.22500001, 0.315, 0.40500006, 0.54000002, 0.72000003],
        [3.0, 4.19999981, 5.39999962, 6.0, 6.0],
        [0.22500001, 0.315, 0.40500006, 0.54000002, 0.72000003],
        [1.0, 1.0, 1.0, 2.0, 2.0],
        [0.21051483, 0.27575418, 0.33216619, 0.40265679, 0.47516328],
    ];
    for (i, d) in jewelry::JEWELS.iter().enumerate() {
        for (r, quality) in rewards::Rarity::ALL.into_iter().enumerate() {
            let actual = rewards::jewel_gain(d.stat, d.base, quality, None);
            assert!(
                (actual - expected[i][r]).abs() < 0.00001,
                "{} / {:?}: {} != {}",
                d.name,
                quality,
                actual,
                expected[i][r]
            );
        }
    }
}
#[test]
fn amulets_start_at_base_strength_and_grow_towards_three() {
    for i in 0..amulets::AMULETS.len() {
        let base = amulets::Amulet { index: i, rank: 1 };
        assert_eq!(base.scale(), 1.);
        let mut previous = base.scale();
        for rank in 2..100 {
            let current = amulets::Amulet { index: i, rank }.scale();
            assert!(current >= previous && current <= 3.);
            previous = current;
        }
    }
}
#[test]
fn rarity_policy_can_reach_all_qualities_and_luck_cannot_downgrade() {
    let mut seen = [false; 5];
    for i in 0..10000 {
        let r = i as f32 / 10000.;
        let a = rewards::Rarity::roll(r, 0.);
        seen[a as usize] = true;
        assert!(rewards::Rarity::roll(r, 0.5) as usize >= a as usize);
    }
    assert_eq!(seen, [true; 5]);
}
#[test]
fn linked_health_defense_stats_match_original_caps() {
    let armor = jewelry::effects(4, 0.2);
    assert!((armor[0].1 - 0.105).abs() < 0.00001);
    assert!((armor[1].1 - 0.8).abs() < 0.00001);
    let health = jewelry::effects(22, 1.0);
    assert_eq!(health, [(6, 1.0), (5, 0.12)]);
    assert!((jewelry::stat(22, 0.25, 5) - 0.04).abs() < 0.00001);
}

#[path = "../crates/benilla-app/src/survivors/gold.rs"]
mod gold;
