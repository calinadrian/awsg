//! The damage meter under the minimap: each hero's damage this run, largest first, with its DPS
//! and share. Every client counts only its own hits (health actually taken, so overkill and hits
//! on a replica the host already killed add nothing) and carries the total in its co-op pose;
//! the host relays every pose, so all players read the same table.

use bevy::prelude::*;

use super::coop::Coop;
use super::hud::{compact, HudFont};
use super::{data, Phase, Run};

/// Rows the panel holds; a room never seats more.
const ROWS: usize = 8;
const WIDTH: f32 = 196.0;
const ROW_H: f32 = 17.0;
/// Below the 192-unit minimap cluster.
const TOP: f32 = 198.0;

#[derive(Component)]
struct Meter;
#[derive(Component)]
struct MeterRow(usize);
#[derive(Component)]
struct MeterFill(usize);
#[derive(Component)]
struct MeterName(usize);
#[derive(Component)]
struct MeterValue(usize);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (spawn, paint).chain());
}

#[derive(Debug, PartialEq)]
struct Entry {
    name: String,
    color: u32,
    damage: f64,
}

/// The table, largest first: the room's members in co-op, else the lone hero.
fn entries(run: &Run, coop: &Coop) -> Vec<Entry> {
    let own = |name: &str| {
        let name = name.trim();
        if name.is_empty() { "You" } else { name }.to_owned()
    };
    let mut rows: Vec<Entry> = match coop.room.as_ref().filter(|_| coop.running) {
        Some(room) => room
            .members
            .iter()
            .map(|m| Entry {
                name: if m.peer == coop.peer { own(&m.name) } else { m.name.clone() },
                color: data::CLASSES.get(m.hero.class).map_or(0xFFFFFF, |c| c.color),
                // Our own total is live; a teammate's arrives with their pose.
                damage: if m.peer == coop.peer {
                    run.damage_done
                } else {
                    coop.players.get(&m.peer).map_or(0.0, |p| p.damage)
                },
            })
            .collect(),
        None => vec![Entry {
            name: own(&coop.name),
            color: run.class.color,
            damage: run.damage_done,
        }],
    };
    rows.sort_by(|a, b| b.damage.total_cmp(&a.damage).then_with(|| a.name.cmp(&b.name)));
    rows.truncate(ROWS);
    rows
}

fn rgb(hex: u32) -> Color {
    Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn text(font: &Handle<Font>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::ONE,
            color: Color::BLACK,
        },
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
    )
}

fn spawn(mut commands: Commands, font: Option<Res<HudFont>>, meters: Query<(), With<Meter>>) {
    let Some(font) = font else {
        return;
    };
    if !meters.is_empty() {
        return;
    }
    let f = &font.0;
    commands
        .spawn((
            Meter,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Px(TOP),
                width: Val::Px(WIDTH),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(1.0),
                padding: UiRect::all(Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.025, 0.02, 0.72)),
            BorderColor::all(Color::srgb(0.42, 0.38, 0.23)),
            GlobalZIndex(85),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    margin: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                Text::new("Damage Done"),
                TextFont {
                    font: f.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.82, 0.0)),
                TextShadow {
                    offset: Vec2::ONE,
                    color: Color::BLACK,
                },
            ));
            for i in 0..ROWS {
                panel
                    .spawn((
                        MeterRow(i),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(ROW_H),
                            padding: UiRect::horizontal(Val::Px(4.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            display: Display::None,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                    ))
                    .with_children(|row| {
                        row.spawn((
                            MeterFill(i),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                height: Val::Percent(100.0),
                                width: Val::Percent(0.0),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                        ));
                        row.spawn((MeterName(i), text(f, 11.0, Color::WHITE)));
                        row.spawn((MeterValue(i), text(f, 11.0, Color::WHITE)));
                    });
            }
        });
}

#[allow(clippy::type_complexity)]
fn paint(
    run: Res<Run>,
    coop: Res<Coop>,
    phase: Res<State<Phase>>,
    mut meters: Query<&mut Visibility, With<Meter>>,
    mut rows: Query<(&MeterRow, &mut Node), Without<MeterFill>>,
    mut fills: Query<(&MeterFill, &mut Node, &mut BackgroundColor), Without<MeterRow>>,
    mut names: Query<(&MeterName, &mut Text), Without<MeterValue>>,
    mut values: Query<(&MeterValue, &mut Text), Without<MeterName>>,
) {
    let Ok(mut visibility) = meters.single_mut() else {
        return;
    };
    let shown = *phase.get() != Phase::Menu;
    let want = if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    visibility.set_if_neq(want);
    if !shown {
        return;
    }
    let table = entries(&run, &coop);
    let top = table.first().map_or(0.0, |e| e.damage);
    let sum: f64 = table.iter().map(|e| e.damage).sum();
    let secs = f64::from(run.elapsed).max(1.0);
    for (row, mut node) in &mut rows {
        let display = if row.0 < table.len() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (fill, mut node, mut color) in &mut fills {
        let Some(e) = table.get(fill.0) else {
            continue;
        };
        let share = if top > 0.0 { e.damage / top } else { 0.0 };
        node.width = Val::Percent((share * 100.0) as f32);
        color.set_if_neq(BackgroundColor(rgb(e.color).with_alpha(0.55)));
    }
    for (name, mut t) in &mut names {
        let Some(e) = table.get(name.0) else {
            continue;
        };
        let want = format!("{}. {}", name.0 + 1, e.name);
        if t.0 != want {
            t.0 = want;
        }
    }
    for (value, mut t) in &mut values {
        let Some(e) = table.get(value.0) else {
            continue;
        };
        let percent = if sum > 0.0 { e.damage / sum * 100.0 } else { 0.0 };
        let want = format!(
            "{} ({}, {:.0}%)",
            compact(e.damage),
            compact(e.damage / secs),
            percent
        );
        if t.0 != want {
            t.0 = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(peer: u32, name: &str, class: usize) -> asr_coop::Member {
        asr_coop::Member {
            peer,
            name: name.into(),
            hero: asr_coop::Hero {
                class,
                race: 1,
                sex: 0,
                display: 0,
                features: None,
            },
            ready: true,
        }
    }

    #[test]
    fn solo_shows_the_lone_hero() {
        let run = Run {
            damage_done: 1234.0,
            ..default()
        };
        let mut coop = Coop::default();
        coop.name = "Thrall".into();
        let table = entries(&run, &coop);
        assert_eq!(table.len(), 1);
        assert_eq!(table[0].name, "Thrall");
        coop.name = "  ".into();
        assert_eq!(entries(&run, &coop)[0].name, "You");
        assert_eq!(table[0].damage, 1234.0);
    }

    #[test]
    fn coop_ranks_every_member_by_the_damage_their_pose_carries() {
        let run = Run {
            damage_done: 500.0,
            ..default()
        };
        let mut coop = Coop::default();
        coop.running = true;
        coop.peer = 1;
        coop.room = Some(asr_coop::Room {
            code: String::new(),
            name: String::new(),
            map: 0,
            private: false,
            members: vec![member(0, "Host", 0), member(1, "Me", 1), member(2, "Late", 2)],
        });
        // The host's pose of us lags; our own live total wins.
        for (peer, damage) in [(0, 900.0), (1, 10.0)] {
            coop.players.insert(
                peer,
                super::super::coop::Pose {
                    peer,
                    damage,
                    ..default()
                },
            );
        }
        let table = entries(&run, &coop);
        let ranked: Vec<_> = table.iter().map(|e| (e.name.as_str(), e.damage)).collect();
        assert_eq!(ranked, [("Host", 900.0), ("Me", 500.0), ("Late", 0.0)]);
    }
}
