//! Personal completion records. Co-op advances the selected class on each player's copy.
use super::{Phase, Run, Selection};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Serialize, Deserialize)]
struct Record {
    runs: u32,
    wins: u32,
    coop_wins: u32,
    best_seconds: f32,
    best_kills: u32,
}
#[derive(Resource, Default, Serialize, Deserialize)]
pub(super) struct Progress {
    #[serde(default)]
    cleared_zones: BTreeSet<String>,
    #[serde(default)]
    records: BTreeMap<String, Record>,
    #[serde(skip)]
    paid: bool,
}
fn path() -> Option<std::path::PathBuf> {
    crate::local_state::home().map(|p| p.join("survivors-progress.toml"))
}
pub(super) fn plugin(app: &mut App) {
    let progress = path()
        .and_then(|p| crate::local_state::read_to_string(&p).ok())
        .and_then(|t| toml::from_str::<Progress>(&t).ok())
        .unwrap_or_default();
    app.init_resource::<Selection>();
    app.world_mut().resource_mut::<Selection>().unlocked_races = super::races::RACES.iter()
        .filter(|race| super::races::unlock_zone(race.id).is_some_and(|zone| progress.cleared_zones.contains(zone)))
        .map(|race| race.id).collect();
    app.insert_resource(progress)
        .add_systems(
            OnTransition {
                exited: Phase::Menu,
                entered: Phase::Playing,
            },
            begin,
        )
        .add_systems(OnEnter(Phase::GameOver), finish)
        .add_systems(Update, track.before(super::menu::menu_clicks));
}

#[allow(clippy::too_many_arguments)]
fn track(
    mut progress: ResMut<Progress>, mut selection: ResMut<Selection>, run: Res<Run>,
    phase: Res<State<Phase>>, gear: Res<super::inventory::Equipment>, coop: Res<super::coop::Coop>,
    creatures: Option<Res<crate::entities::Creatures>>,
) {
    let mut changed = false;
    if *phase.get() != Phase::Menu {
        if super::data::roster(selection.stage).bosses.last().is_some_and(|boss| run.boss_loot.iter().any(|name| name == boss.name)) {
            changed |= progress.cleared_zones.insert(selection.stage_def().name.into());
        }
    }
    if let Some(hero) = selection.class_def().and_then(|class| gear.hero(&coop.url, class.id)) {
        for id in &hero.inventory.cleared_zones {
            if let Some(zone) = gear.catalog.zones.get(id) {
                changed |= progress.cleared_zones.insert(zone.name.clone());
            }
        }
    }
    if changed { progress.save(); }
    let unlocked: Vec<_> = super::races::RACES.iter().filter(|race| {
        super::races::unlock_zone(race.id).is_some_and(|zone| progress.cleared_zones.contains(zone))
    }).map(|race| race.id).collect();
    if selection.unlocked_races != unlocked {
        selection.unlocked_races = unlocked;
        selection.version += 1;
    }
    if *phase.get() != Phase::Menu { return; }
    let race = selection.look.and_then(|i| selection.looks.get(i)).map(|look| look.1);
    if race.is_none_or(|race| !selection.race_unlocked(race)) {
        if let Some(creatures) = creatures.as_deref() {
            let all = super::menu::all_looks(creatures);
            let candidate = selection.class.into_iter().chain(0..all.len()).find_map(|class| {
                all[class].iter().position(|look| selection.race_unlocked(look.1)).map(|look| (class, look))
            });
            if let Some((class, look)) = candidate { super::menu::set(&mut selection, &all, class, look); }
        }
    }
    if !selection.can_visit(selection.map) {
        let race = selection.look.and_then(|i| selection.looks.get(i)).map_or(1, |look| look.1);
        let name = if super::races::RACES.iter().any(|r| r.id == race && r.faction == 1) { "Durotar" } else { "Elwynn Forest" };
        if let Some(map) = super::data::MAPS.iter().position(|map| map.name == name) { selection.select_map(map); }
    }
}
fn begin(mut progress: ResMut<Progress>) {
    progress.paid = false;
}
fn finish(mut progress: ResMut<Progress>, run: Res<Run>, selection: Res<Selection>) {
    progress.settle(&run, selection.stage_def().name, run.coop_run);
}
impl Progress {
    fn save(&self) {
        if let (Some(path), Ok(text)) = (path(), toml::to_string(self)) {
            let _ = crate::local_state::write_atomic(&path, &text);
        }
    }
    pub fn settle(&mut self, run: &Run, zone: &str, coop: bool) {
        if self.paid || (run.elapsed <= 0. && run.kills == 0) {
            return;
        }
        self.paid = true;
        let r = self
            .records
            .entry(format!("{}:{zone}", run.class.id))
            .or_default();
        r.runs = r.runs.saturating_add(1);
        r.best_seconds = r.best_seconds.max(run.elapsed);
        r.best_kills = r.best_kills.max(run.kills);
        if run.won {
            r.wins = r.wins.saturating_add(1);
            if coop {
                r.coop_wins = r.coop_wins.saturating_add(1);
            }
        }
        if let (Some(path), Ok(text)) = (path(), toml::to_string(self)) {
            let _ = crate::local_state::write_atomic(&path, &text);
        }
    }
    pub fn summary(&self, class: u8, zone: &str) -> String {
        self.records.get(&format!("{class}:{zone}")).map_or_else(
            || "No completed runs yet".into(),
            |r| {
                let secs = r.best_seconds.max(0.) as u32;
                format!(
                    "{} {}  ·  {} {} ({} co-op)  ·  Best {} kills in {}:{:02}",
                    r.runs,
                    if r.runs == 1 { "run" } else { "runs" },
                    r.wins,
                    if r.wins == 1 { "victory" } else { "victories" },
                    r.coop_wins,
                    r.best_kills,
                    secs / 60,
                    secs % 60
                )
            },
        )
    }
}
