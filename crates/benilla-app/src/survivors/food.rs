//! Chest food: five kinds, one slot each, three builds per kind, and five ranks per build.
//! An empty slot takes the Common of any build, and that pick sets the slot's build. A filled slot
//! only takes the next rank of its build, so a food's numbers only climb, up to Legendary. When
//! every slot holds its Legendary the chest offers Full Belly. Food never gives move speed.
use super::rewards::Rarity;
use super::Rng;

pub(super) struct Food {
    pub name: &'static str,
    pub icon: &'static str,
}

pub(super) struct FoodBonus {
    pub label: &'static str,
    /// The run stat it feeds (see [`super::jewelry::label`]), or [`TARGETS`].
    stat: u8,
    /// What each rank adds to the ranks below it, in the units the card prints.
    steps: [f32; RANKS],
    percent: bool,
    /// The rank's own value rather than a running total.
    flat: bool,
}

pub(super) struct FoodBuild {
    pub desc: &'static str,
    pub bonuses: &'static [FoodBonus],
    foods: [Food; RANKS],
}

pub(super) struct FoodKind {
    pub name: &'static str,
    pub one: &'static str,
    pub slot: &'static str,
    builds: [FoodBuild; 3],
}

include!("food_data.rs");

pub(super) const RANKS: usize = 5;
pub const SLOTS: usize = 5;
pub(super) const FULL_BELLY_DAMAGE_PERCENT: u32 = 5;
/// A bonus to every spell's targets, which is no run stat.
const TARGETS: u8 = 255;

/// One food: its slot, its build and its rank, Common being 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FoodId {
    pub kind: u8,
    pub build: u8,
    pub rank: u8,
}

/// What the hero carries, by kind.
pub type Slots = [Option<FoodId>; SLOTS];

impl FoodId {
    pub(super) fn kind(self) -> &'static FoodKind {
        &KINDS[self.kind as usize]
    }
    pub(super) fn build(self) -> &'static FoodBuild {
        &self.kind().builds[self.build as usize]
    }
    pub(super) fn food(self) -> &'static Food {
        &self.build().foods[self.rank as usize]
    }
    pub(super) fn rarity(self) -> Rarity {
        [
            Rarity::Common,
            Rarity::Uncommon,
            Rarity::Rare,
            Rarity::Epic,
            Rarity::Legendary,
        ][self.rank as usize]
    }
    pub(super) fn is_last(self) -> bool {
        self.rank as usize == RANKS - 1
    }
}

impl FoodBonus {
    /// The total this bonus gives at a rank, in the units the card prints.
    fn total(&self, rank: u8) -> f32 {
        let rank = rank as usize;
        if self.flat {
            self.steps[rank]
        } else {
            self.steps[..=rank].iter().sum()
        }
    }

    /// As the game prints bonuses: percents with at least one decimal, health rates with two.
    pub(super) fn amount(&self, rank: u8) -> String {
        let value = (self.total(rank) * 100.0).round() / 100.0;
        if self.percent {
            if (value * 10.0 - (value * 10.0).round()).abs() < 0.001 {
                format!("+{value:.1}%")
            } else {
                format!("+{value:.2}%")
            }
        } else if self.stat == TARGETS {
            format!("+{value:.0}")
        } else {
            format!("+{value:.2}")
        }
    }

    /// The value the run's stats add up.
    fn value(&self, rank: u8) -> f32 {
        self.total(rank) * if self.percent { 0.01 } else { 1.0 }
    }
}

/// What one held food adds to a run stat.
pub(super) fn food_stat(food: FoodId, stat: u8) -> f32 {
    food.build()
        .bonuses
        .iter()
        .filter(|bonus| bonus.stat == stat)
        .map(|bonus| bonus.value(food.rank))
        .sum()
}

/// What all held food adds to a run stat.
pub(super) fn stat(slots: &Slots, stat: u8) -> f32 {
    slots.iter().flatten().map(|food| food_stat(*food, stat)).sum()
}

/// The extra targets every spell gets from held food.
pub(super) fn targets(slots: &Slots) -> u32 {
    stat(slots, TARGETS).round() as u32
}

/// Every food a chest may offer now.
pub(super) fn eligible(slots: &Slots) -> Vec<FoodId> {
    let mut out = Vec::new();
    for (kind, held) in slots.iter().enumerate() {
        match held {
            None => out.extend((0..3).map(|build| FoodId {
                kind: kind as u8,
                build,
                rank: 0,
            })),
            Some(food) if !food.is_last() => out.push(FoodId {
                rank: food.rank + 1,
                ..*food
            }),
            Some(_) => {}
        }
    }
    out
}

/// Up to `count` foods, one per kind while kinds last, each kind as likely as any other.
pub(super) fn draw(mut pool: Vec<FoodId>, count: usize, rng: &mut Rng) -> Vec<FoodId> {
    let mut out: Vec<FoodId> = Vec::new();
    while out.len() < count && !pool.is_empty() {
        let mut kinds: Vec<u8> = pool.iter().map(|food| food.kind).collect();
        kinds.sort_unstable();
        kinds.dedup();
        let fresh: Vec<u8> = kinds
            .iter()
            .copied()
            .filter(|kind| !out.iter().any(|food| food.kind == *kind))
            .collect();
        let from = if fresh.is_empty() { &kinds } else { &fresh };
        let kind = from[rng.index(from.len())];
        let options: Vec<usize> = (0..pool.len()).filter(|&i| pool[i].kind == kind).collect();
        out.push(pool.swap_remove(options[rng.index(options.len())]));
    }
    out
}

/// A Common in the slot, for the preview of a chest with every slot full.
pub(super) fn all_common() -> Slots {
    std::array::from_fn(|kind| {
        Some(FoodId {
            kind: kind as u8,
            build: 0,
            rank: 0,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_food() -> impl Iterator<Item = FoodId> {
        (0..SLOTS as u8).flat_map(|kind| {
            (0..3).flat_map(move |build| {
                (0..RANKS as u8).map(move |rank| FoodId { kind, build, rank })
            })
        })
    }

    #[test]
    fn the_table_has_75_foods_and_no_move_speed() {
        assert_eq!(every_food().count(), 75);
        for food in every_food() {
            assert!(!food.food().name.is_empty());
            assert_eq!(food_stat(food, 10), 0.0, "{}", food.food().name);
        }
    }

    #[test]
    fn a_slot_climbs_from_common_to_legendary_without_skipping_or_changing_build() {
        let mut rng = Rng::default();
        for _ in 0..200 {
            let mut slots: Slots = [None; SLOTS];
            let mut picks = 0;
            loop {
                let offer = draw(eligible(&slots), 3, &mut rng);
                if offer.is_empty() {
                    break;
                }
                assert!(offer.len() <= 3);
                let chosen = offer[rng.index(offer.len())];
                match slots[chosen.kind as usize] {
                    None => assert_eq!(chosen.rank, 0),
                    Some(held) => {
                        assert_eq!(chosen.build, held.build);
                        assert_eq!(chosen.rank, held.rank + 1);
                    }
                }
                slots[chosen.kind as usize] = Some(chosen);
                picks += 1;
                assert!(slots.iter().flatten().count() <= SLOTS);
            }
            assert_eq!(picks, SLOTS * RANKS);
            assert!(slots.iter().flatten().all(|food| food.is_last()));
        }
    }

    #[test]
    fn an_offer_shows_different_kinds_while_it_can() {
        let mut rng = Rng::default();
        for _ in 0..200 {
            let offer = draw(eligible(&[None; SLOTS]), 3, &mut rng);
            assert_eq!(offer.len(), 3);
            assert!(offer[0].kind != offer[1].kind && offer[1].kind != offer[2].kind);
            assert!(offer[0].kind != offer[2].kind);
        }
    }

    #[test]
    fn numbers_only_climb_and_print_as_the_game_prints_them() {
        for food in every_food().filter(|food| food.rank > 0) {
            let below = FoodId {
                rank: food.rank - 1,
                ..food
            };
            for bonus in food.build().bonuses {
                assert!(bonus.total(food.rank) >= bonus.total(below.rank));
            }
        }
        let sugar = FoodId { kind: 0, build: 0, rank: 0 };
        assert_eq!(sugar.build().bonuses[0].amount(0), "+1.0%");
        assert_eq!(sugar.build().bonuses[0].amount(1), "+2.5%");
        assert_eq!(food_stat(sugar, 0), 0.01);
        let treats = FoodId { kind: 0, build: 1, rank: 3 };
        assert_eq!(treats.build().bonuses[1].amount(3), "+2");
        let mut slots: Slots = [None; SLOTS];
        slots[0] = Some(treats);
        assert_eq!(targets(&slots), 2);
    }

    #[test]
    fn every_food_icon_is_in_the_game_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the install's MPQ chain");
        for food in every_food() {
            let path = format!("{}.blp", food.food().icon);
            assert!(chain.read_file(&path).is_ok(), "{path}");
        }
    }
}
