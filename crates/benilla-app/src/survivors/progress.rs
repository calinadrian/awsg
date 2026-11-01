//! Personal completion records. Co-op advances the selected class on each player's copy.
use super::{Phase, Run, Selection};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    app.insert_resource(progress)
        .add_systems(
            OnTransition {
                exited: Phase::Menu,
                entered: Phase::Playing,
            },
            begin,
        )
        .add_systems(OnEnter(Phase::GameOver), finish);
}
fn begin(mut progress: ResMut<Progress>) {
    progress.paid = false;
}
fn finish(mut progress: ResMut<Progress>, run: Res<Run>, selection: Res<Selection>) {
    progress.settle(&run, selection.stage_def().name, run.coop_run);
}
impl Progress {
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
