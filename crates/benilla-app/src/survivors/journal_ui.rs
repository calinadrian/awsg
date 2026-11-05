//! The achievements a hero has earned and is working toward.
use super::{
    front_end::{self as skin, gap, size},
    inventory::Equipment,
    inventory_ui::{pane_node, scroller, Action},
};
use asr_coop::economy::Inventory;
use asr_coop::quests::{AchievementDef, Kind, Objective};
use bevy::prelude::*;

/// The achievement categories, in the order the tab lists them after the summary.
const CATEGORIES: [&str; 6] = ["Zones", "Bosses", "Slayer", "Survival", "Co-op", "Mastery"];

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
