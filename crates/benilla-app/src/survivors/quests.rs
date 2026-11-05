//! Run progress sent to the achievement ledger and achievements shown on the results screen.
use super::{
    coop::Coop,
    front_end::{self as skin, size},
    hud::HudFont,
    inventory::Equipment,
    Phase, Run,
};
use asr_coop::quests::{Kind, Tick};
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// The most creature names a run tallies; a roster has about thirty.
pub(super) const MAX_TALLY: usize = 256;

/// The kills since `counted`, as ticks for the ledger; `counted` catches up.
pub(super) fn kill_ticks(run: &Run, counted: &mut BTreeMap<String, [u32; 2]>) -> Vec<Tick> {
    let mut ticks = Vec::new();
    for (name, now) in &run.tally {
        let before = counted.get(name).copied().unwrap_or_default();
        for (kind, i) in [(Kind::Kill, 0), (Kind::Elite, 1)] {
            let n = now[i].saturating_sub(before[i]);
            if n > 0 {
                ticks.push(Tick {
                    kind,
                    target: name.clone(),
                    n,
                });
            }
        }
    }
    counted.clone_from(&run.tally);
    ticks.truncate(asr_coop::quests::MAX_TICKS - 8);
    ticks
}

/// This run's best-of progress: how long it has lasted and how far the hero has grown.
pub(super) fn run_ticks(run: &Run) -> [Tick; 2] {
    [
        Tick {
            kind: Kind::Survive,
            target: String::new(),
            n: run.elapsed.max(0.) as u32,
        },
        Tick {
            kind: Kind::Level,
            target: String::new(),
            n: run.level,
        },
    ]
}

/// The results screen slot for achievements earned during the run.
#[derive(Component)]
pub(super) struct ResultsAchievements;

#[derive(Resource, Default)]
struct Journal {
    held: Option<BTreeSet<String>>,
    earned: Vec<String>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Journal>()
        .add_systems(
            OnTransition {
                exited: Phase::Menu,
                entered: Phase::Playing,
            },
            |mut journal: ResMut<Journal>| *journal = Journal::default(),
        )
        .add_systems(Update, (follow, paint_results).chain());
}

fn follow(
    phase: Res<State<Phase>>,
    run: Res<Run>,
    coop: Res<Coop>,
    gear: Res<Equipment>,
    mut journal: ResMut<Journal>,
) {
    if *phase.get() == Phase::Menu || run.test {
        return;
    }
    let Some(hero) = gear.hero(&gear.realm(&coop), run.class.id) else {
        return;
    };
    let achievements = &hero.inventory.achievements;
    let Journal { held, earned } = &mut *journal;
    let Some(held) = held.as_mut() else {
        *held = Some(achievements.keys().cloned().collect());
        return;
    };
    for id in achievements.keys() {
        if held.insert(id.clone()) {
            if let Some(def) = gear.catalog.quests.achievement(id) {
                earned.push(def.title.clone());
            }
        }
    }
}

fn paint_results(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    phase: Res<State<Phase>>,
    journal: Res<Journal>,
    mut boxes: Query<(Entity, &mut Node), With<ResultsAchievements>>,
    mut shown: Local<usize>,
) {
    if *phase.get() != Phase::GameOver {
        *shown = 0;
        return;
    }
    if *shown == journal.earned.len() {
        return;
    }
    let (Some(font), Ok((target, mut node))) = (font, boxes.single_mut()) else {
        return;
    };
    *shown = journal.earned.len();
    node.display = Display::Flex;
    commands
        .entity(target)
        .despawn_related::<Children>()
        .with_children(|p| {
            skin::label(
                p,
                &font.0,
                format!("Achievements earned: {}", journal.earned.join(", ")),
                size::SMALL,
                skin::GREEN,
            );
        });
}

#[cfg(test)]
mod tests {
    use super::super::data::{roster, MAPS};
    use asr_coop::economy::Catalog;

    /// An achievement naming a creature no roster spawns could never be finished.
    #[test]
    fn catalog_targets_are_roster_names() {
        let catalog = Catalog::load();
        let objectives = catalog.quests.achievements.iter().flat_map(|a| &a.criteria);
        for objective in objectives.filter(|o| !o.target.is_empty()) {
            let zone = &catalog.zones[&objective.zone];
            let map = MAPS
                .iter()
                .position(|m| m.name == zone.name)
                .expect("a battleground per zone");
            let r = roster(map);
            let names: Vec<_> = match objective.kind {
                asr_coop::quests::Kind::Boss => r.bosses.iter().map(|e| e.name).collect(),
                asr_coop::quests::Kind::Elite => r.guardians.iter().map(|e| e.name).collect(),
                _ => r.enemies.iter().map(|e| e.name).collect(),
            };
            assert!(
                names.contains(&objective.target.as_str()),
                "{} in {} is not on its roster: {names:?}",
                objective.target,
                zone.name
            );
        }
    }
}
