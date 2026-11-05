//! Quests and achievements: what the realm offers, and the rules that move a hero's progress
//! along. Pure data and arithmetic, so the realm service, a solo game and the menus share it;
//! the [`crate::ledger`] decides which progress it believes.
use crate::economy::Inventory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How many quests a hero may carry at once.
pub const MAX_ACTIVE: usize = 5;
/// The most progress lines one reward batch may carry.
pub const MAX_TICKS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Ordinary enemies slain; the target names one creature, or any when empty.
    Kill,
    /// Elites slain: lair guardians and roaming champions.
    Elite,
    /// Bosses slain. The ledger counts these itself, from the run's boss loot.
    Boss,
    /// A zone cleared: its final boss slain. The ledger counts it, once per run.
    Clear,
    /// Seconds survived in one run.
    Survive,
    /// The level reached in one run.
    Level,
    /// Gold carried out of one run.
    Gold,
    /// The party size of a run that cleared its zone.
    Party,
}
impl Kind {
    /// Whether a hero's runs add up toward the count, or only the best one counts.
    pub fn adds(self) -> bool {
        matches!(self, Kind::Kill | Kind::Elite | Kind::Boss | Kind::Clear)
    }
}

/// Progress a run reports: `n` more of `kind` (or, for a best-of kind, a run's best so far).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tick {
    pub kind: Kind,
    #[serde(default)]
    pub target: String,
    pub n: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Objective {
    pub kind: Kind,
    /// A creature's name; empty for any.
    #[serde(default)]
    pub target: String,
    /// An economy zone id; empty for any.
    #[serde(default)]
    pub zone: String,
    /// A class id; 0 for any.
    #[serde(default)]
    pub class: u8,
    pub count: u32,
    pub text: String,
}
impl Objective {
    /// Move this objective's count along by a tick from a run in `zone` as `class`; whether it
    /// moved.
    pub fn advance(&self, zone: &str, class: u8, tick: &Tick, value: &mut u32) -> bool {
        if self.kind != tick.kind
            || (!self.target.is_empty() && self.target != tick.target)
            || (!self.zone.is_empty() && self.zone != zone)
            || (self.class != 0 && self.class != class)
        {
            return false;
        }
        let next = if tick.kind.adds() {
            value.saturating_add(tick.n)
        } else {
            (*value).max(tick.n)
        }
        .min(self.count);
        std::mem::replace(value, next) != next
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QuestReward {
    #[serde(default)]
    pub gold: u64,
    /// This many of each material the quest's zone drops.
    #[serde(default)]
    pub materials: u32,
    /// Achievement points.
    #[serde(default)]
    pub points: u32,
    /// One of the best items the quest's zone drops that the hero's class can use.
    #[serde(default)]
    pub item: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuestDef {
    /// Never changes once shipped: heroes' logs refer to it.
    pub id: String,
    /// The economy zone the quest is given in; its objectives count there.
    pub zone: String,
    pub title: String,
    pub giver: String,
    pub text: String,
    /// Quests that must be turned in before this one is offered.
    #[serde(default)]
    pub requires: Vec<String>,
    /// A bounty: offered again once turned in.
    #[serde(default)]
    pub repeatable: bool,
    pub objectives: Vec<Objective>,
    pub reward: QuestReward,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AchievementDef {
    /// Never changes once shipped.
    pub id: String,
    pub category: String,
    pub title: String,
    pub text: String,
    pub points: u32,
    #[serde(default)]
    pub criteria: Vec<Objective>,
    /// Achievements that must all be earned; for a meta-achievement with no criteria.
    #[serde(default)]
    pub requires: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QuestCatalog {
    /// Raised whenever quests or achievements change; heroes saved under an older one are
    /// brought forward on their next transaction.
    pub revision: u32,
    pub quests: Vec<QuestDef>,
    pub achievements: Vec<AchievementDef>,
}

/// A quest in a hero's log, with one count per objective.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestState {
    pub id: String,
    pub progress: Vec<u32>,
}

impl QuestCatalog {
    pub fn load() -> Self {
        serde_json::from_str(include_str!("../assets/quest_catalog.json"))
            .expect("valid quest catalog")
    }
    pub fn quest(&self, id: &str) -> Option<&QuestDef> {
        self.quests.iter().find(|q| q.id == id)
    }
    pub fn achievement(&self, id: &str) -> Option<&AchievementDef> {
        self.achievements.iter().find(|a| a.id == id)
    }
    /// The achievement a zone's first clear earns.
    pub fn conquest(&self, zone: &str) -> Option<&AchievementDef> {
        self.achievement(&format!("conquer.{zone}"))
    }
    /// Whether every objective of a quest in the log is done.
    pub fn complete(&self, state: &QuestState) -> bool {
        self.quest(&state.id).is_some_and(|def| {
            def.objectives
                .iter()
                .enumerate()
                .all(|(i, o)| state.progress.get(i).copied().unwrap_or(0) >= o.count)
        })
    }
    /// Whether a hero may take this quest now: `Err` says why not.
    pub fn offer(&self, inventory: &Inventory, id: &str) -> Result<&QuestDef, String> {
        let def = self.quest(id).ok_or("That quest is no longer offered")?;
        if inventory.quests.iter().any(|q| q.id == id) {
            return Err("That quest is already in your log".into());
        }
        if !def.repeatable && inventory.quests_done.contains_key(id) {
            return Err("You have already completed that quest".into());
        }
        if let Some(missing) = def.requires.iter().find(|r| !inventory.quests_done.contains_key(*r)) {
            let title = self.quest(missing).map_or(missing.as_str(), |q| q.title.as_str());
            return Err(format!("Complete \u{201c}{title}\u{201d} first"));
        }
        if inventory.quests.len() >= MAX_ACTIVE {
            return Err(format!("Your quest log is full ({MAX_ACTIVE} quests)"));
        }
        Ok(def)
    }

    /// Apply a run's ticks to the hero's active quests and unearned achievements. Earned
    /// achievements pay their points; returns their ids.
    pub fn progress(&self, inventory: &mut Inventory, zone: &str, class: u8, ticks: &[Tick], now: u64) -> Vec<String> {
        for state in &mut inventory.quests {
            let Some(def) = self.quest(&state.id) else { continue };
            state.progress.resize(def.objectives.len(), 0);
            for tick in ticks {
                for (objective, value) in def.objectives.iter().zip(state.progress.iter_mut()) {
                    objective.advance(zone, class, tick, value);
                }
            }
        }
        for def in &self.achievements {
            if def.criteria.is_empty() || inventory.achievements.contains_key(&def.id) {
                continue;
            }
            let mut values = inventory.achievement_progress.remove(&def.id).unwrap_or_default();
            values.resize(def.criteria.len(), 0);
            for tick in ticks {
                for (objective, value) in def.criteria.iter().zip(values.iter_mut()) {
                    objective.advance(zone, class, tick, value);
                }
            }
            if values.iter().any(|&v| v > 0) {
                inventory.achievement_progress.insert(def.id.clone(), values);
            }
        }
        self.earn(inventory, now)
    }

    /// Earn every achievement whose criteria or prerequisites are met, paying its points.
    pub(crate) fn earn(&self, inventory: &mut Inventory, now: u64) -> Vec<String> {
        let mut earned = Vec::new();
        loop {
            let ready = self.achievements.iter().find(|def| {
                !inventory.achievements.contains_key(&def.id)
                    && (!def.criteria.is_empty() || !def.requires.is_empty())
                    && def.requires.iter().all(|r| inventory.achievements.contains_key(r))
                    && def.criteria.iter().enumerate().all(|(i, o)| {
                        inventory
                            .achievement_progress
                            .get(&def.id)
                            .and_then(|v| v.get(i))
                            .is_some_and(|&v| v >= o.count)
                    })
            });
            let Some(def) = ready else { break };
            inventory.achievement_progress.remove(&def.id);
            inventory.achievements.insert(def.id.clone(), now);
            inventory.achievement_points = inventory.achievement_points.saturating_add(def.points);
            inventory.achievement_total = inventory.achievement_total.saturating_add(def.points);
            earned.push(def.id.clone());
        }
        earned
    }

    /// Bring a hero saved under an older catalog forward: zones cleared before achievements
    /// existed become their conquests (their points were paid at the clear), retired quests
    /// leave the log. Whether anything changed.
    pub fn migrate(&self, inventory: &mut Inventory, now: u64) -> bool {
        if inventory.quest_rev >= self.revision {
            return false;
        }
        inventory.quest_rev = self.revision;
        for zone in &inventory.cleared_zones {
            if let Some(def) = self.conquest(zone) {
                if !inventory.achievements.contains_key(&def.id) {
                    inventory.achievements.insert(def.id.clone(), now);
                    inventory.achievement_progress.remove(&def.id);
                    inventory.achievement_total = inventory.achievement_total.saturating_add(def.points);
                }
            }
        }
        inventory.quests.retain(|state| self.quest(&state.id).is_some());
        inventory.achievement_progress.retain(|id, _| self.achievement(id).is_some());
        // Meta-achievements over migrated conquests; paid like any other.
        self.earn(inventory, now);
        true
    }

    /// Points a hero holds for achievements in both of two inventories being combined.
    pub fn overlap(&self, a: &BTreeMap<String, u64>, b: &BTreeMap<String, u64>) -> u32 {
        a.keys()
            .filter(|id| b.contains_key(*id))
            .filter_map(|id| self.achievement(id))
            .map(|def| def.points)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(kind: Kind, target: &str, n: u32) -> Tick {
        Tick { kind, target: target.into(), n }
    }

    #[test]
    fn the_catalog_is_consistent() {
        let catalog = QuestCatalog::load();
        let zones = crate::economy::Catalog::load().zones;
        let mut ids = std::collections::BTreeSet::new();
        for quest in &catalog.quests {
            assert!(ids.insert(&quest.id), "duplicate quest {}", quest.id);
            assert!(zones.contains_key(&quest.zone), "{} has unknown zone {}", quest.id, quest.zone);
            assert!(!quest.objectives.is_empty(), "{}", quest.id);
            for objective in &quest.objectives {
                assert_eq!(objective.zone, quest.zone, "{} counts outside its zone", quest.id);
                assert!(objective.count > 0);
            }
            for r in &quest.requires {
                assert!(catalog.quest(r).is_some(), "{} requires unknown {r}", quest.id);
            }
        }
        for def in &catalog.achievements {
            assert!(ids.insert(&def.id), "duplicate achievement {}", def.id);
            assert!(!def.criteria.is_empty() || !def.requires.is_empty(), "{} can never be earned", def.id);
            for r in &def.requires {
                assert!(catalog.achievement(r).is_some(), "{} requires unknown {r}", def.id);
            }
            for objective in &def.criteria {
                assert!(objective.zone.is_empty() || zones.contains_key(&objective.zone), "{}", def.id);
            }
        }
        for zone in zones.keys() {
            let conquest = catalog.conquest(zone).unwrap_or_else(|| panic!("no conquest for {zone}"));
            // The first clear paid 10 points before achievements; migrated heroes keep that.
            assert_eq!(conquest.points, 10, "{zone}");
        }
    }

    #[test]
    fn ticks_count_in_their_zone_and_best_ofs_keep_the_best() {
        let objective = |kind, target: &str| Objective {
            kind,
            target: target.into(),
            zone: "elwynn".into(),
            class: 0,
            count: 10,
            text: String::new(),
        };
        let mut v = 0;
        let kill = objective(Kind::Kill, "Kobold Vermin");
        assert!(kill.advance("elwynn", 1, &tick(Kind::Kill, "Kobold Vermin", 4), &mut v));
        assert!(!kill.advance("westfall", 1, &tick(Kind::Kill, "Kobold Vermin", 4), &mut v));
        assert!(!kill.advance("elwynn", 1, &tick(Kind::Kill, "Young Wolf", 4), &mut v));
        assert!(!kill.advance("elwynn", 1, &tick(Kind::Elite, "Kobold Vermin", 4), &mut v));
        kill.advance("elwynn", 1, &tick(Kind::Kill, "Kobold Vermin", 40), &mut v);
        assert_eq!(v, 10, "capped at the count");
        let mut best = 0;
        let level = objective(Kind::Level, "");
        level.advance("elwynn", 1, &tick(Kind::Level, "", 7), &mut best);
        level.advance("elwynn", 1, &tick(Kind::Level, "", 5), &mut best);
        assert_eq!(best, 7);
    }

    #[test]
    fn achievements_pay_once_and_meta_achievements_follow() {
        let catalog = QuestCatalog::load();
        let mut hero = Inventory::default();
        let earned = catalog.progress(&mut hero, "elwynn", 1, &[tick(Kind::Clear, "", 1)], 5);
        assert!(earned.contains(&"conquer.elwynn".to_string()), "{earned:?}");
        let points = hero.achievement_points;
        assert!(points >= 10 && hero.achievement_total == points);
        assert!(catalog.progress(&mut hero, "elwynn", 1, &[tick(Kind::Clear, "", 1)], 6).iter().all(|id| id != "conquer.elwynn"));
        assert!(hero.achievement_points >= points);
    }

    #[test]
    fn migration_credits_old_clears_without_paying_twice() {
        let catalog = QuestCatalog::load();
        let mut hero = Inventory { achievement_points: 20, ..Inventory::default() };
        hero.cleared_zones.insert("elwynn".into());
        hero.cleared_zones.insert("westfall".into());
        hero.quests.push(QuestState { id: "retired.quest".into(), progress: vec![1] });
        assert!(catalog.migrate(&mut hero, 9));
        assert!(hero.achievements.contains_key("conquer.elwynn"));
        assert!(hero.achievements.contains_key("conquer.westfall"));
        assert_eq!(hero.achievement_points, 20, "conquests were paid at the clear");
        assert_eq!(hero.achievement_total, 20);
        assert!(hero.quests.is_empty());
        assert!(!catalog.migrate(&mut hero, 10), "once per revision");
    }
}
