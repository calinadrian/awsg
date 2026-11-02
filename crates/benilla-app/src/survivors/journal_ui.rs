//! The account window's Quests and Achievements tabs: a quest log you accept quests from and
//! turn them in to, and the achievements a hero has earned and is working toward.
use super::{
    front_end::{self as skin, gap, size},
    inventory::Equipment,
    inventory_ui::{choice, journal_zones, material_icon, pane_node, scroller, Action, ItemIcon},
    Selection,
};
use asr_coop::economy::Inventory;
use asr_coop::quests::{AchievementDef, Kind, Objective, QuestDef, MAX_ACTIVE};
use bevy::prelude::*;

/// The achievement categories, in the order the tab lists them after the summary.
const CATEGORIES: [&str; 6] = ["Zones", "Bosses", "Slayer", "Survival", "Co-op", "Mastery"];

/// Where a quest stands for this hero, as the log's marks show it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    /// Every objective done: a gold `?`.
    Ready,
    /// In the log: a grey `?`.
    Active,
    /// Offered: a gold `!`.
    Available,
    /// Waits on another quest: a grey `!`.
    Locked,
    Done,
}

fn status(gear: &Equipment, inventory: Option<&Inventory>, def: &QuestDef) -> Status {
    let Some(inventory) = inventory else {
        return if def.requires.is_empty() { Status::Available } else { Status::Locked };
    };
    if let Some(state) = inventory.quests.iter().find(|q| q.id == def.id) {
        return if gear.catalog.quests.complete(state) { Status::Ready } else { Status::Active };
    }
    if !def.repeatable && inventory.quests_done.contains_key(&def.id) {
        Status::Done
    } else if def.requires.iter().any(|r| !inventory.quests_done.contains_key(r)) {
        Status::Locked
    } else {
        Status::Available
    }
}

/// An objective's count as its kind reads: a clock for time survived.
fn amount(objective: &Objective, value: u32) -> String {
    if objective.kind == Kind::Survive {
        format!("{}:{:02} / {}:{:02}", value / 60, value % 60, objective.count / 60, objective.count % 60)
    } else {
        format!("{} / {}", value.min(objective.count), objective.count)
    }
}

/// A compact control sized by its text; primary is the screen's one main action.
fn action(p: &mut ChildSpawnerCommands, f: &Handle<Font>, title: &str, action: Action, primary: bool, enabled: bool) {
    p.spawn((
        Button,
        action,
        skin::Control { primary, selected: false, enabled },
        Node {
            min_height: Val::Px(34.),
            padding: UiRect::axes(Val::Px(gap::XL), Val::Px(6.)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(4.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(skin::SURFACE),
        BorderColor::all(skin::EDGE),
    ))
    .with_children(|p| {
        let color = if !enabled {
            skin::MUTED
        } else if primary {
            skin::GOLD
        } else {
            skin::TEXT
        };
        skin::label(p, f, title, if primary { size::LEAD } else { size::BODY }, color);
    });
}

/// A section's caption over a list, with a count after it.
fn section(p: &mut ChildSpawnerCommands, f: &Handle<Font>, title: &str, count: String) {
    p.spawn(Node {
        width: Val::Percent(100.),
        align_items: AlignItems::Center,
        column_gap: Val::Px(gap::S),
        margin: UiRect::top(Val::Px(gap::S)),
        flex_shrink: 0.,
        ..default()
    })
    .with_children(|p| {
        skin::label(p, f, title, size::SMALL, skin::GOLD);
        skin::label(p, f, count, size::SMALL, skin::MUTED);
        p.spawn((
            Node {
                flex_grow: 1.,
                height: Val::Px(1.),
                ..default()
            },
            BackgroundColor(skin::EDGE),
        ));
    });
}

/// A quest mark: `!` offered, `?` to turn in; gold when it can be acted on.
fn mark(p: &mut ChildSpawnerCommands, f: &Handle<Font>, status: Status) {
    let (glyph, color) = match status {
        Status::Ready => ("?", skin::GOLD),
        Status::Active => ("?", skin::MUTED),
        Status::Available => ("!", skin::GOLD),
        Status::Locked | Status::Done => ("!", skin::MUTED),
    };
    p.spawn((
        Node {
            width: Val::Px(24.),
            height: Val::Px(24.),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(12.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(skin::WELL),
        BorderColor::all(color.with_alpha(0.8)),
    ))
    .with_children(|p| {
        skin::label(p, f, glyph, size::LEAD, color);
    });
}

fn zone_name<'a>(gear: &'a Equipment, zone: &'a str) -> &'a str {
    gear.catalog.zones.get(zone).map_or(zone, |z| z.name.as_str())
}

fn tier(gear: &Equipment, zone: &str) -> String {
    super::difficulty::tier(zone_name(gear, zone))
        .map_or_else(String::new, |t| format!("Tier {}", super::difficulty::numeral(t)))
}

/// Objectives done of a quest in the log, and how far along it is overall.
fn done_of(inventory: Option<&Inventory>, def: &QuestDef) -> (usize, f32) {
    let progress = inventory
        .and_then(|inv| inv.quests.iter().find(|q| q.id == def.id))
        .map_or(&[][..], |q| &q.progress[..]);
    let value = |i: usize| progress.get(i).copied().unwrap_or(0);
    let done = def.objectives.iter().enumerate().filter(|(i, o)| value(*i) >= o.count).count();
    let fraction = def
        .objectives
        .iter()
        .enumerate()
        .map(|(i, o)| value(i).min(o.count) as f32 / o.count.max(1) as f32)
        .sum::<f32>()
        / def.objectives.len().max(1) as f32;
    (done, fraction)
}

fn entry(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    gear: &Equipment,
    inventory: Option<&Inventory>,
    def: &QuestDef,
    status: Status,
    selected: bool,
) {
    p.spawn((
        Button,
        Action::QuestPick(def.id.clone()),
        skin::Control { primary: false, selected, enabled: true },
        Node {
            width: Val::Percent(100.),
            padding: UiRect::axes(Val::Px(gap::M), Val::Px(gap::S)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(4.)),
            column_gap: Val::Px(gap::M),
            align_items: AlignItems::Center,
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(skin::SURFACE),
        BorderColor::all(skin::EDGE),
    ))
    .with_children(|p| {
        mark(p, f, status);
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.,
            min_width: Val::Px(0.),
            row_gap: Val::Px(3.),
            ..default()
        })
        .with_children(|p| {
            let title = if status == Status::Locked { skin::MUTED } else if selected { skin::GOLD } else { skin::TEXT };
            skin::label(p, f, &def.title, size::BODY, title);
            let zone = zone_name(gear, &def.zone);
            let zone = if def.repeatable { format!("{zone}  ·  Bounty") } else { zone.to_owned() };
            let sub = match status {
                Status::Locked => {
                    let first = def.requires.first().and_then(|r| gear.catalog.quests.quest(r));
                    format!("{zone}  ·  after \u{201c}{}\u{201d}", first.map_or("", |q| q.title.as_str()))
                }
                Status::Active | Status::Ready => {
                    let (done, _) = done_of(inventory, def);
                    format!("{zone}  ·  {done} of {} done", def.objectives.len())
                }
                _ => format!("{zone}  ·  {}", tier(gear, &def.zone)),
            };
            skin::label(p, f, sub, size::CAPTION, skin::MUTED);
            if status == Status::Active {
                let (_, fraction) = done_of(inventory, def);
                skin::progress_bar(p, fraction, skin::GOLD, 4.);
            }
        });
    });
}

/// The Quests tab: the log on the left, the open quest on the right.
pub(super) fn quests(
    body: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    gear: &Equipment,
    selection: &Selection,
    inventory: Option<&Inventory>,
) {
    let catalog = &gear.catalog.quests;
    let zones: Vec<_> = journal_zones(gear, selection).map(|(id, zone)| (id.clone(), zone.name.clone())).collect();
    let shown = |def: &QuestDef| {
        zones.iter().any(|(id, _)| *id == def.zone) && gear.quest_zone.as_ref().is_none_or(|zone| *zone == def.zone)
    };
    let (mut ready, mut active, mut offered, mut done) = (Vec::new(), Vec::new(), Vec::new(), 0);
    for def in &catalog.quests {
        match status(gear, inventory, def) {
            Status::Ready => ready.push(def),
            Status::Active => active.push(def),
            s @ (Status::Available | Status::Locked) if shown(def) => offered.push((def, s)),
            Status::Done => done += 1,
            _ => {}
        }
    }
    // Ready and available first, locked last.
    offered.sort_by_key(|(_, s)| *s == Status::Locked);
    let open = gear
        .quest
        .as_deref()
        .and_then(|id| {
            ready.iter().chain(&active).copied().chain(offered.iter().map(|(d, _)| *d)).find(|d| d.id == id)
        })
        .or_else(|| ready.first().or(active.first()).copied())
        .or_else(|| offered.first().map(|(d, _)| *d));
    let carried = inventory.map_or(0, |inv| inv.quests.len());

    body.spawn(pane_node(40.)).with_children(|p| {
        p.spawn(Node {
            align_items: AlignItems::Baseline,
            column_gap: Val::Px(gap::S),
            flex_shrink: 0.,
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, "Quest log", size::HEADING, skin::GOLD);
            skin::label(p, f, format!("{carried} / {MAX_ACTIVE}"), size::BODY, if carried >= MAX_ACTIVE { skin::RED } else { skin::MUTED });
        });
        p.spawn(Node {
            column_gap: Val::Px(6.),
            row_gap: Val::Px(5.),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            ..default()
        })
        .with_children(|p| {
            choice(p, f, "All zones", Action::QuestZone(None), gear.quest_zone.is_none());
            for (id, name) in &zones {
                choice(p, f, name, Action::QuestZone(Some(id.clone())), gear.quest_zone.as_ref() == Some(id));
            }
        });
        p.spawn(scroller(3)).with_children(|p| {
            let selected = |def: &QuestDef| open.is_some_and(|o| o.id == def.id);
            if !ready.is_empty() {
                section(p, f, "READY TO TURN IN", ready.len().to_string());
                for def in &ready {
                    entry(p, f, gear, inventory, def, Status::Ready, selected(def));
                }
            }
            if !active.is_empty() {
                section(p, f, "IN PROGRESS", active.len().to_string());
                for def in &active {
                    entry(p, f, gear, inventory, def, Status::Active, selected(def));
                }
            }
            let place = gear.quest_zone.as_deref().map_or("your zones", |zone| zone_name(gear, zone));
            section(p, f, "AVAILABLE", format!("in {place}"));
            if offered.is_empty() {
                skin::label(p, f, "Nothing more to take here. Try another zone.", size::SMALL, skin::MUTED);
            }
            for (def, s) in &offered {
                entry(p, f, gear, inventory, def, *s, selected(def));
            }
            if done > 0 {
                section(p, f, "COMPLETED", skin::plural(done as u32, "quest"));
            }
        });
    });

    body.spawn((
        Node {
            padding: UiRect::all(Val::Px(gap::XL)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(6.)),
            ..pane_node(60.)
        },
        // Parchment under candlelight: warmer than the window around it.
        BackgroundColor(Color::srgb(0.115, 0.09, 0.06)),
        BorderColor::all(skin::GOLD.with_alpha(0.45)),
    ))
    .with_children(|p| match open {
        Some(def) => detail(p, f, gear, selection, inventory, def, status(gear, inventory, def)),
        None => {
            skin::label(p, f, "No quests yet", size::HEADING, skin::GOLD);
            skin::label(p, f, "Pick a zone your hero can enter to see what its people need.", size::BODY, skin::MUTED);
        }
    });
}

fn detail(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    gear: &Equipment,
    selection: &Selection,
    inventory: Option<&Inventory>,
    def: &QuestDef,
    status: Status,
) {
    let zone = zone_name(gear, &def.zone);
    p.spawn(scroller(5)).with_children(|p| {
        p.spawn(Node {
            column_gap: Val::Px(gap::M),
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, &def.title, size::TITLE, skin::GOLD);
            let (text, color) = match status {
                Status::Ready => ("Ready to turn in", skin::GREEN),
                Status::Active => ("In progress", skin::TEXT),
                Status::Available => ("Available", skin::GOLD),
                Status::Locked => ("Locked", skin::MUTED),
                Status::Done => ("Completed", skin::GREEN),
            };
            skin::chip(p, f, text, color);
            if def.repeatable {
                skin::chip(p, f, "Bounty  ·  repeatable", skin::MUTED);
            }
        });
        skin::label(p, f, format!("{}  ·  {zone}  ·  {}", def.giver, tier(gear, &def.zone)), size::SMALL, skin::MUTED);
        skin::rule(p);
        skin::label(p, f, &def.text, size::LEAD, skin::TEXT);

        section(p, f, "OBJECTIVES", String::new());
        let progress = inventory
            .and_then(|inv| inv.quests.iter().find(|q| q.id == def.id))
            .map(|q| &q.progress[..]);
        for (i, objective) in def.objectives.iter().enumerate() {
            let value = progress.and_then(|v| v.get(i)).copied().unwrap_or(0);
            let met = value >= objective.count;
            p.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(gap::XS),
                flex_shrink: 0.,
                ..default()
            })
            .with_children(|p| {
                p.spawn(Node {
                    column_gap: Val::Px(gap::M),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|p| {
                    skin::label(p, f, &objective.text, size::BODY, if met { skin::GREEN } else { skin::TEXT });
                    if progress.is_some() {
                        skin::label(p, f, amount(objective, value), size::SMALL, if met { skin::GREEN } else { skin::MUTED });
                    }
                });
                if progress.is_some() {
                    skin::progress_bar(p, value as f32 / objective.count.max(1) as f32, if met { skin::GREEN } else { skin::GOLD }, 6.);
                }
            });
        }
        if progress.is_none() {
            skin::label(p, f, format!("Progress counts in {zone} runs once you accept."), size::SMALL, skin::MUTED);
        }

        section(p, f, "REWARDS", String::new());
        p.spawn(Node {
            column_gap: Val::Px(gap::S),
            row_gap: Val::Px(gap::S),
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            flex_shrink: 0.,
            ..default()
        })
        .with_children(|p| {
            let reward = &def.reward;
            if reward.gold > 0 {
                skin::chip(p, f, format!("{} gold", reward.gold), skin::GOLD);
            }
            if reward.materials > 0 {
                let pool = gear.catalog.zones.get(&def.zone).map_or(&[][..], |z| &z.material_pool[..]);
                for material in pool {
                    p.spawn(Node {
                        column_gap: Val::Px(gap::XS),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|p| {
                        p.spawn((
                            ItemIcon(format!("Interface\\Icons\\{}", material_icon(material))),
                            ImageNode::default(),
                            Node {
                                width: Val::Px(20.),
                                height: Val::Px(20.),
                                ..default()
                            },
                        ));
                        skin::label(p, f, format!("{} {material}", reward.materials), size::SMALL, skin::TEXT);
                    });
                }
            }
            if reward.item {
                let class = selection.class_def().map_or(0, |c| c.id);
                let best = gear
                    .catalog
                    .zones
                    .get(&def.zone)
                    .into_iter()
                    .flat_map(|z| z.equipment.iter())
                    .filter_map(|d| gear.catalog.definition(&d.item))
                    .filter(|d| d.can_use(class))
                    .map(|d| d.quality.clone())
                    .max_by_key(|q| match q.as_str() {
                        "epic" => 3,
                        "rare" => 2,
                        "uncommon" => 1,
                        _ => 0,
                    })
                    .unwrap_or_else(|| "common".into());
                let mut name = best.clone();
                if let Some(first) = name.get_mut(..1) {
                    first.make_ascii_uppercase();
                }
                skin::chip(p, f, format!("{name} item for your class"), skin::quality_color(&best));
            }
            if reward.points > 0 {
                skin::chip(p, f, skin::plural(reward.points, "achievement point"), skin::GREEN);
            }
        });
    });

    skin::rule(p);
    p.spawn(Node {
        column_gap: Val::Px(gap::M),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::FlexEnd,
        flex_shrink: 0.,
        ..default()
    })
    .with_children(|p| {
        let busy = gear.quest_busy(&def.id);
        let carried = inventory.map_or(0, |inv| inv.quests.len());
        let note = match status {
            Status::Available if carried >= MAX_ACTIVE => "Your log is full: abandon or turn in a quest to take this one.".into(),
            Status::Active => format!("Finish every objective during a run in {zone}, then turn it in here."),
            Status::Locked => {
                let first = def.requires.first().and_then(|r| gear.catalog.quests.quest(r));
                format!("Turn in \u{201c}{}\u{201d} first.", first.map_or("", |q| q.title.as_str()))
            }
            Status::Done => "You have finished this quest.".into(),
            _ if busy => "Saving…".into(),
            _ => String::new(),
        };
        p.spawn(Node {
            flex_grow: 1.,
            min_width: Val::Px(0.),
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, note, size::SMALL, skin::MUTED);
        });
        match status {
            Status::Available => {
                action(p, f, "Accept quest", Action::Accept(def.id.clone()), true, !busy && carried < MAX_ACTIVE);
            }
            Status::Active => action(p, f, "Abandon", Action::Abandon(def.id.clone()), false, !busy),
            Status::Ready => {
                action(p, f, "Abandon", Action::Abandon(def.id.clone()), false, !busy);
                action(p, f, "Turn in", Action::TurnIn(def.id.clone()), true, !busy);
            }
            Status::Locked | Status::Done => {}
        }
    });
}

/// A day as the achievements show it: "2 Nov 2026".
fn date(seconds: u64) -> String {
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let days = (seconds / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{day} {} {year}", names[(month - 1) as usize])
}

/// How far a hero is toward an achievement: (done, needed), in the unit its tile shows.
fn toward(inventory: Option<&Inventory>, def: &AchievementDef) -> (u32, u32) {
    let earned = |id: &str| inventory.is_some_and(|inv| inv.achievements.contains_key(id));
    if def.criteria.is_empty() {
        let done = def.requires.iter().filter(|r| earned(r)).count() as u32;
        return (done, def.requires.len() as u32);
    }
    let values = inventory.and_then(|inv| inv.achievement_progress.get(&def.id));
    let value = |i: usize| values.and_then(|v| v.get(i)).copied().unwrap_or(0);
    if let [single] = &def.criteria[..] {
        (value(0).min(single.count), single.count)
    } else {
        let done = def.criteria.iter().enumerate().filter(|(i, o)| value(*i) >= o.count).count() as u32;
        (done, def.criteria.len() as u32)
    }
}

fn tile(p: &mut ChildSpawnerCommands, f: &Handle<Font>, inventory: Option<&Inventory>, def: &AchievementDef) {
    let earned = inventory.and_then(|inv| inv.achievements.get(&def.id)).copied();
    p.spawn((
        Node {
            padding: UiRect::all(Val::Px(gap::M)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(6.)),
            column_gap: Val::Px(gap::M),
            min_width: Val::Px(0.),
            ..default()
        },
        BackgroundColor(if earned.is_some() { Color::srgb(0.12, 0.095, 0.055) } else { skin::SURFACE }),
        BorderColor::all(if earned.is_some() { skin::GOLD.with_alpha(0.6) } else { skin::EDGE }),
    ))
    .with_children(|p| {
        let tone = if earned.is_some() { skin::GOLD } else { skin::MUTED };
        p.spawn((
            Node {
                width: Val::Px(46.),
                height: Val::Px(46.),
                border: UiRect::all(Val::Px(1.)),
                border_radius: BorderRadius::bottom(Val::Px(18.)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_shrink: 0.,
                ..default()
            },
            BackgroundColor(skin::WELL),
            BorderColor::all(tone.with_alpha(0.8)),
        ))
        .with_children(|p| {
            skin::label(p, f, def.points.to_string(), size::HEADING, tone);
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.,
            min_width: Val::Px(0.),
            row_gap: Val::Px(3.),
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, &def.title, size::LEAD, if earned.is_some() { skin::GOLD } else { skin::TEXT });
            skin::label(p, f, &def.text, size::SMALL, skin::MUTED);
            match earned {
                Some(when) => {
                    skin::label(p, f, format!("Earned {}", date(when)), size::CAPTION, skin::GREEN);
                }
                None => {
                    let (done, needed) = toward(inventory, def);
                    p.spawn(Node {
                        column_gap: Val::Px(gap::S),
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(2.)),
                        ..default()
                    })
                    .with_children(|p| {
                        p.spawn(Node {
                            flex_grow: 1.,
                            ..default()
                        })
                        .with_children(|p| {
                            skin::progress_bar(p, done as f32 / needed.max(1) as f32, skin::GOLD, 5.);
                        });
                        let text = match &def.criteria[..] {
                            [single] => amount(single, done),
                            _ => format!("{done} / {needed}"),
                        };
                        skin::label(p, f, text, size::CAPTION, skin::MUTED);
                    });
                }
            }
        });
    });
}

fn grid(p: &mut ChildSpawnerCommands, columns: u16, children: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node {
        display: Display::Grid,
        grid_template_columns: RepeatedGridTrack::flex(columns, 1.),
        column_gap: Val::Px(gap::M),
        row_gap: Val::Px(gap::M),
        width: Val::Percent(100.),
        flex_shrink: 0.,
        ..default()
    })
    .with_children(children);
}

/// The Achievements tab: categories down the left with the score, tiles on the right.
pub(super) fn achievements(
    body: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    gear: &Equipment,
    inventory: Option<&Inventory>,
    width: f32,
) {
    let all = &gear.catalog.quests.achievements;
    let earned = |def: &AchievementDef| inventory.is_some_and(|inv| inv.achievements.contains_key(&def.id));
    let count = |category: Option<&str>| {
        let defs = all.iter().filter(|d| category.is_none_or(|c| d.category == c));
        let (mut have, mut total) = (0, 0);
        for def in defs {
            total += 1;
            have += u32::from(earned(def));
        }
        (have, total)
    };
    let category = gear.category.min(CATEGORIES.len());

    body.spawn(pane_node(24.)).with_children(|p| {
        p.spawn((
            Node {
                padding: UiRect::all(Val::Px(gap::L)),
                border: UiRect::all(Val::Px(1.)),
                border_radius: BorderRadius::all(Val::Px(6.)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(gap::XS),
                flex_shrink: 0.,
                ..default()
            },
            BackgroundColor(skin::WELL),
            BorderColor::all(skin::EDGE),
        ))
        .with_children(|p| {
            skin::label(p, f, "ACHIEVEMENT POINTS", size::CAPTION, skin::MUTED);
            skin::label(p, f, inventory.map_or(0, |inv| inv.achievement_total).to_string(), size::DISPLAY, skin::GOLD);
            let (have, total) = count(None);
            skin::label(p, f, format!("{have} of {total} earned"), size::SMALL, skin::TEXT);
            skin::progress_bar(p, have as f32 / total.max(1) as f32, skin::GOLD, 5.);
        });
        p.spawn(scroller(3)).with_children(|p| {
            let row = |p: &mut ChildSpawnerCommands, index: usize, title: &str, (have, total): (u32, u32)| {
                p.spawn((
                    Button,
                    Action::Category(index),
                    skin::Control { primary: false, selected: category == index, enabled: true },
                    Node {
                        width: Val::Percent(100.),
                        padding: UiRect::axes(Val::Px(gap::M), Val::Px(gap::S)),
                        border: UiRect::all(Val::Px(1.)),
                        border_radius: BorderRadius::all(Val::Px(4.)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(gap::XS),
                        flex_shrink: 0.,
                        ..default()
                    },
                    BackgroundColor(skin::SURFACE),
                    BorderColor::all(skin::EDGE),
                ))
                .with_children(|p| {
                    p.spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|p| {
                        skin::label(p, f, title, size::BODY, if category == index { skin::GOLD } else { skin::TEXT });
                        skin::label(p, f, format!("{have} / {total}"), size::CAPTION, skin::MUTED);
                    });
                    skin::progress_bar(p, have as f32 / total.max(1) as f32, if have == total { skin::GREEN } else { skin::GOLD }, 4.);
                });
            };
            row(p, 0, "Summary", count(None));
            for (i, name) in CATEGORIES.iter().enumerate() {
                row(p, i + 1, name, count(Some(name)));
            }
        });
    });

    let columns = if width * 0.76 > 980. { 3 } else { 2 };
    body.spawn(pane_node(76.)).with_children(|p| {
        p.spawn(scroller(6)).with_children(|p| {
            if category > 0 {
                let name = CATEGORIES[category - 1];
                let mut defs: Vec<_> = all.iter().filter(|d| d.category == name).collect();
                // Earned first, then those nearest done.
                defs.sort_by(|a, b| {
                    let near = |d: &AchievementDef| {
                        let (done, needed) = toward(inventory, d);
                        done as f32 / needed.max(1) as f32
                    };
                    earned(b).cmp(&earned(a)).then(near(b).total_cmp(&near(a)))
                });
                grid(p, columns, |p| {
                    for def in defs {
                        tile(p, f, inventory, def);
                    }
                });
                return;
            }
            let mut recent: Vec<_> = all
                .iter()
                .filter_map(|d| Some((d, *inventory?.achievements.get(&d.id)?)))
                .collect();
            recent.sort_by(|a, b| b.1.cmp(&a.1));
            section(p, f, "RECENTLY EARNED", String::new());
            if recent.is_empty() {
                skin::label(p, f, "Clear a zone, slay its bosses or survive its waves to earn your first.", size::BODY, skin::MUTED);
            } else {
                grid(p, columns, |p| {
                    for (def, _) in recent.iter().take(columns as usize) {
                        tile(p, f, inventory, def);
                    }
                });
            }
            let mut next: Vec<_> = all
                .iter()
                .filter(|d| !earned(d))
                .map(|d| {
                    let (done, needed) = toward(inventory, d);
                    (d, done as f32 / needed.max(1) as f32)
                })
                .collect();
            next.sort_by(|a, b| b.1.total_cmp(&a.1));
            section(p, f, "CLOSEST TO DONE", String::new());
            grid(p, columns, |p| {
                for (def, _) in next.iter().take(2 * columns as usize) {
                    tile(p, f, inventory, def);
                }
            });
            section(p, f, "SPEND POINTS", String::new());
            let points = inventory.map_or(0, |inv| inv.achievement_points);
            p.spawn(Node {
                column_gap: Val::Px(gap::M),
                row_gap: Val::Px(gap::S),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                flex_shrink: 0.,
                ..default()
            })
            .with_children(|p| {
                skin::label(p, f, format!("{} to spend", skin::plural(points, "point")), size::BODY, skin::GOLD);
                action(p, f, "20 of each material  ·  10", Action::Redeem(false), false, points >= 10 && gear.online());
                action(p, f, "Equipment from your best cleared zone  ·  30", Action::Redeem(true), false, points >= 30 && gear.online());
            });
            skin::label(p, f, "Points are shared by your linked classes. Spending them never lowers your score.", size::SMALL, skin::MUTED);
        });
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn dates_read_as_days() {
        assert_eq!(super::date(0), "1 Jan 1970");
        assert_eq!(super::date(1_793_577_600), "2 Nov 2026");
        assert_eq!(super::date(951_782_400), "29 Feb 2000");
    }
}
