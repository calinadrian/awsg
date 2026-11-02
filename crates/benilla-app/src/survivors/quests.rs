//! A run's quest and achievement progress: the kill tally the ledger is sent, the in-run quest
//! tracker under the damage meter, and the quest lines on the results screen.
use super::{
    coop::Coop,
    damage_meter::RightStack,
    front_end::{self as skin, gap, size},
    hud::HudFont,
    inventory::Equipment,
    Phase, Run, Selection,
};
use asr_coop::economy::{Catalog, Inventory};
use asr_coop::quests::{Kind, Tick};
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// The most creature names a run tallies; a roster has about thirty.
pub(super) const MAX_TALLY: usize = 256;
/// How long a tracker notice stays up, in seconds.
const NOTICE_SECS: f32 = 5.;

/// The kills since `counted`, as ticks for the ledger; `counted` catches up.
pub(super) fn kill_ticks(run: &Run, counted: &mut BTreeMap<String, [u32; 2]>) -> Vec<Tick> {
    let mut ticks = Vec::new();
    for (name, now) in &run.tally {
        let before = counted.get(name).copied().unwrap_or_default();
        for (kind, i) in [(Kind::Kill, 0), (Kind::Elite, 1)] {
            let n = now[i].saturating_sub(before[i]);
            if n > 0 {
                ticks.push(Tick { kind, target: name.clone(), n });
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
        Tick { kind: Kind::Survive, target: String::new(), n: run.elapsed.max(0.) as u32 },
        Tick { kind: Kind::Level, target: String::new(), n: run.level },
    ]
}

/// One objective as the tracker shows it.
#[derive(Clone, PartialEq)]
pub(super) struct Goal {
    pub text: String,
    pub value: u32,
    pub count: u32,
    /// Seconds, shown as a clock.
    pub clock: bool,
}
impl Goal {
    fn met(&self) -> bool {
        self.value >= self.count
    }
    fn amount(&self) -> String {
        let value = self.value.min(self.count);
        if self.clock {
            format!("{}:{:02}", value / 60, value % 60)
        } else {
            format!("{value}/{}", self.count)
        }
    }
}
/// One quest as the tracker shows it.
#[derive(Clone, PartialEq)]
pub(super) struct Line {
    pub id: String,
    pub title: String,
    pub objectives: Vec<Goal>,
    pub complete: bool,
}

/// The hero's active quests in `zone` with this run's unsent kills folded in, so the tracker
/// moves the moment an enemy dies rather than when the ledger next answers.
pub(super) fn live(
    catalog: &Catalog,
    inventory: &Inventory,
    zone: &str,
    class: u8,
    run: &Run,
    counted: &BTreeMap<String, [u32; 2]>,
    bosses_counted: usize,
) -> Vec<Line> {
    let mut pending = kill_ticks(run, &mut counted.clone());
    pending.extend(run_ticks(run));
    pending.extend(run.boss_loot.iter().skip(bosses_counted).map(|name| Tick {
        kind: Kind::Boss,
        target: name.clone(),
        n: 1,
    }));
    inventory
        .quests
        .iter()
        .filter_map(|state| {
            let def = catalog.quests.quest(&state.id).filter(|def| def.zone == zone)?;
            let objectives: Vec<_> = def
                .objectives
                .iter()
                .enumerate()
                .map(|(i, objective)| {
                    let mut value = state.progress.get(i).copied().unwrap_or(0);
                    for tick in &pending {
                        objective.advance(zone, class, tick, &mut value);
                    }
                    Goal {
                        text: objective.text.clone(),
                        value,
                        count: objective.count,
                        clock: objective.kind == Kind::Survive,
                    }
                })
                .collect();
            let complete = objectives.iter().all(Goal::met);
            Some(Line { id: def.id.clone(), title: def.title.clone(), objectives, complete })
        })
        .collect()
}

/// The economy zone id a battleground's runs count in.
pub(super) fn zone_id<'a>(catalog: &'a Catalog, map_name: &str) -> Option<&'a str> {
    catalog.zones.iter().find(|(_, z)| z.name == map_name).map(|(id, _)| id.as_str())
}

/// The tracker's box in the right-hand column.
#[derive(Component)]
struct Tracker;
/// The results screen's quest lines; the HUD builds the box, this module fills it.
#[derive(Component)]
pub(super) struct ResultsQuests;

/// What this run has done for the hero's quests and achievements.
#[derive(Resource, Default)]
struct Journal {
    /// The achievements held when the run began, and the objectives already met.
    held: Option<(BTreeSet<String>, BTreeSet<(String, usize)>)>,
    /// Achievements earned during this run, by title.
    earned: Vec<String>,
    /// A notice over the tracker and how long it has left.
    notice: Option<(String, f32)>,
    lines: Vec<Line>,
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
        .add_systems(Update, (follow, paint_tracker, paint_results).chain());
}

/// Keep the journal current with the run, and raise a notice when an objective is met or an
/// achievement comes in.
fn follow(
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    selection: Res<Selection>,
    coop: Res<Coop>,
    gear: Res<Equipment>,
    mut journal: ResMut<Journal>,
) {
    if *phase.get() == Phase::Menu {
        return;
    }
    if let Some((_, t)) = journal.notice.as_mut() {
        *t -= time.delta_secs();
        if *t <= 0. {
            journal.notice = None;
        }
    }
    let Some(zone) = zone_id(&gear.catalog, selection.stage_def().name) else {
        journal.lines.clear();
        return;
    };
    let Some((lines, achievements)) = gear.journal(&coop, &run, zone) else {
        journal.lines.clear();
        return;
    };
    let met: BTreeSet<_> = lines
        .iter()
        .flat_map(|line| {
            line.objectives
                .iter()
                .enumerate()
                .filter(|(_, goal)| goal.met())
                .map(|(i, _)| (line.id.clone(), i))
        })
        .collect();
    if journal.held.is_none() {
        journal.held = Some((achievements.keys().cloned().collect(), met));
        journal.lines = lines;
        return;
    }
    let journal = &mut *journal;
    let Some((held, done)) = journal.held.as_mut() else { return };
    let mut notice = None;
    for (quest, i) in met.difference(done) {
        if let Some(line) = lines.iter().find(|l| l.id == *quest) {
            notice = Some(if line.complete {
                format!("{} complete", line.title)
            } else {
                format!("{}  ·  objective met", line.objectives[*i].text)
            });
        }
    }
    done.extend(met);
    let mut earned = Vec::new();
    for id in achievements.keys().filter(|id| !held.contains(*id)) {
        if let Some(def) = gear.catalog.quests.achievement(id) {
            earned.push((def.title.clone(), def.points));
        }
    }
    held.extend(achievements.keys().cloned());
    for (title, points) in earned {
        notice = Some(format!("Achievement earned: {title}  (+{points})"));
        journal.earned.push(title);
    }
    if let Some(text) = notice {
        journal.notice = Some((text, NOTICE_SECS));
    }
    if journal.lines != lines {
        journal.lines = lines;
    }
}

fn text(f: &Handle<Font>, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: f.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        skin::shadow(),
    )
}

fn paint_tracker(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    phase: Res<State<Phase>>,
    journal: Res<Journal>,
    stacks: Query<Entity, With<RightStack>>,
    mut trackers: Query<(Entity, &mut Node), With<Tracker>>,
    mut last: Local<Option<String>>,
) {
    let Some(font) = font else { return };
    let Ok(stack) = stacks.single() else { return };
    let shown = matches!(*phase.get(), Phase::Playing | Phase::LevelUp) && !journal.lines.is_empty();
    let tracker = match trackers.single_mut() {
        Ok((entity, mut node)) => {
            let display = if shown { Display::Flex } else { Display::None };
            if node.display != display {
                node.display = display;
            }
            entity
        }
        Err(_) => {
            *last = None;
            commands
                .spawn((
                    Tracker,
                    ChildOf(stack),
                    Node {
                        width: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(gap::XS),
                        padding: UiRect::all(Val::Px(gap::S)),
                        border: UiRect::all(Val::Px(1.)),
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.02, 0.025, 0.02, 0.55)),
                    BorderColor::all(Color::srgba(0.42, 0.38, 0.23, 0.6)),
                ))
                .id()
        }
    };
    if !shown {
        return;
    }
    let key = format!(
        "{:?}|{}",
        journal.notice.as_ref().map(|n| &n.0),
        journal
            .lines
            .iter()
            .map(|l| format!("{}:{:?}", l.title, l.objectives.iter().map(|o| o.value).collect::<Vec<_>>()))
            .collect::<Vec<_>>()
            .join(";")
    );
    if last.as_ref() == Some(&key) {
        return;
    }
    *last = Some(key);
    let f = &font.0;
    commands.entity(tracker).despawn_related::<Children>().with_children(|p| {
        if let Some((notice, _)) = &journal.notice {
            p.spawn(text(f, notice, size::SMALL, skin::GREEN));
        }
        p.spawn(text(f, "QUESTS", size::CAPTION, skin::MUTED));
        for line in &journal.lines {
            p.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(1.),
                margin: UiRect::top(Val::Px(2.)),
                ..default()
            })
            .with_children(|p| {
                let title = if line.complete {
                    format!("{}  ·  ready to turn in", line.title)
                } else {
                    line.title.clone()
                };
                p.spawn(text(f, title, size::BODY, if line.complete { skin::GREEN } else { skin::GOLD }));
                if !line.complete {
                    for goal in &line.objectives {
                        let met = goal.met();
                        p.spawn(Node {
                            padding: UiRect::left(Val::Px(gap::S)),
                            column_gap: Val::Px(gap::S),
                            justify_content: JustifyContent::SpaceBetween,
                            ..default()
                        })
                        .with_children(|p| {
                            p.spawn(text(f, &goal.text, size::SMALL, if met { skin::GREEN } else { skin::TEXT }));
                            p.spawn(text(f, goal.amount(), size::SMALL, if met { skin::GREEN } else { skin::MUTED }));
                        });
                    }
                }
            });
        }
    });
}

fn paint_results(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    phase: Res<State<Phase>>,
    journal: Res<Journal>,
    boxes: Query<Entity, With<ResultsQuests>>,
    mut last: Local<Option<String>>,
) {
    if *phase.get() != Phase::GameOver {
        *last = None;
        return;
    }
    let (Some(font), Ok(target)) = (font, boxes.single()) else { return };
    let key = format!(
        "{}|{}",
        journal.earned.join(","),
        journal.lines.iter().map(|l| format!("{}{}", l.title, l.complete)).collect::<Vec<_>>().join(",")
    );
    if last.as_ref() == Some(&key) {
        return;
    }
    *last = Some(key);
    let f = &font.0;
    commands.entity(target).despawn_related::<Children>().with_children(|p| {
        if journal.lines.is_empty() && journal.earned.is_empty() {
            p.spawn(text(f, "Accept quests in the Quests tab and every run here pays more.", size::SMALL, skin::MUTED));
            return;
        }
        for line in &journal.lines {
            p.spawn(Node {
                column_gap: Val::Px(gap::M),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|p| {
                p.spawn(text(f, &line.title, size::BODY, skin::TEXT));
                let done = line.objectives.iter().filter(|goal| goal.met()).count();
                let (state, color) = if line.complete {
                    ("Ready to turn in".to_string(), skin::GREEN)
                } else {
                    (format!("{done} of {} done", line.objectives.len()), skin::MUTED)
                };
                p.spawn(text(f, state, size::SMALL, color));
            });
        }
        if !journal.earned.is_empty() {
            p.spawn(text(f, format!("Achievements earned: {}", journal.earned.join(", ")), size::SMALL, skin::GREEN));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::super::data::{roster, MAPS};
    use asr_coop::economy::Catalog;

    /// A quest or achievement naming a creature no roster spawns could never be finished.
    #[test]
    fn catalog_targets_are_roster_names() {
        let catalog = Catalog::load();
        let objectives = catalog
            .quests
            .quests
            .iter()
            .flat_map(|q| q.objectives.iter())
            .chain(catalog.quests.achievements.iter().flat_map(|a| a.criteria.iter()));
        for objective in objectives.filter(|o| !o.target.is_empty()) {
            let zone = &catalog.zones[&objective.zone];
            let map = MAPS.iter().position(|m| m.name == zone.name).expect("a battleground per zone");
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
