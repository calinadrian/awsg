//! The damage meter under the minimap: each hero's damage this run, largest first, with its DPS
//! and share. Every client counts only its own hits (health actually taken, so overkill and hits
//! on a replica the host already killed add nothing) and carries the total in its co-op pose;
//! the host relays every pose, so all players read the same table.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use super::coop::Coop;
use super::hud::{compact, HudFont};
use super::{data, Phase, Run};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Source {
    Spell(u32),
    Talent(&'static str),
    Jewelry(usize),
    Amulet(usize),
}

impl Source {
    fn info(self, class: u8, spells: Option<&crate::ui_action::Spells>) -> (String, String) {
        let unknown = "Interface\\Icons\\INV_Misc_QuestionMark";
        match self {
            Self::Spell(spell) => {
                let fallback = data::ABILITIES
                    .iter()
                    .find(|a| a.spell == spell)
                    .map(|a| a.key.to_owned())
                    .unwrap_or_else(|| {
                        if spell == 0 {
                            "Other damage".into()
                        } else {
                            format!("Spell {spell}")
                        }
                    });
                (
                    super::hud::spell_name(spells, spell, &fallback),
                    spells
                        .and_then(|s| s.catalog.get(spell))
                        .and_then(|d| d.icon.clone())
                        .unwrap_or_else(|| unknown.into()),
                )
            }
            Self::Talent(key) => {
                let talent = super::talents::trees(class)
                    .into_iter()
                    .flatten()
                    .flat_map(|tree| tree.talents.iter())
                    .find(|t| t.key == key);
                talent.map_or_else(
                    || (key.replace('_', " "), unknown.into()),
                    |t| {
                        (
                            format!("{} (Talent)", t.name),
                            format!("Interface\\Icons\\{}", t.icon),
                        )
                    },
                )
            }
            Self::Jewelry(i) => {
                let d = &super::jewelry::JEWELS[i];
                (format!("{} (Item)", d.name), d.icon.into())
            }
            Self::Amulet(i) => {
                let d = &super::amulets::AMULETS[i];
                (format!("{} (Item)", d.name), d.icon.into())
            }
        }
    }
}

/// Room for party members or the hero's spells, talents and item effects.
const ROWS: usize = 32;
const WIDTH: f32 = 280.0;
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
#[derive(Component)]
struct MeterIcon(usize);

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    Party,
    SelfDps,
}
#[derive(Component)]
struct ModeButton(Mode);
#[derive(Component)]
struct Summary;
#[derive(Component)]
struct MeterScroll;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Mode>()
        .add_systems(Update, (spawn, controls, paint, paint_icons).chain());
}

fn controls(
    mut mode: ResMut<Mode>,
    buttons: Query<(&ModeButton, &Interaction), Changed<Interaction>>,
    mut wheel: MessageReader<MouseWheel>,
    mut scrolls: Query<(&Interaction, &ComputedNode, &mut ScrollPosition), With<MeterScroll>>,
) {
    let mut switched = false;
    for (button, interaction) in &buttons {
        if *interaction == Interaction::Pressed && *mode != button.0 {
            *mode = button.0;
            switched = true;
        }
    }
    let delta: f32 = wheel
        .read()
        .map(|event| {
            event.y
                * if event.unit == MouseScrollUnit::Line {
                    24.0
                } else {
                    1.0
                }
        })
        .sum();
    for (interaction, node, mut scroll) in &mut scrolls {
        if switched {
            scroll.y = 0.0;
        }
        let max = ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.0);
        if *interaction != Interaction::None {
            scroll.y = (scroll.y - delta).clamp(0.0, max);
        } else {
            scroll.y = scroll.y.clamp(0.0, max);
        }
    }
}

fn spell_entries(run: &Run, spells: Option<&crate::ui_action::Spells>) -> Vec<Entry> {
    let mut rows: Vec<_> = run
        .damage_by_source
        .iter()
        .map(|(&source, &damage)| Entry {
            source: Some(source),
            name: source.info(run.class.id, spells).0,
            color: run.class.color,
            damage,
        })
        .collect();
    rows.sort_by(|a, b| {
        b.damage
            .total_cmp(&a.damage)
            .then_with(|| a.name.cmp(&b.name))
    });
    rows
}

#[derive(Debug, PartialEq)]
struct Entry {
    source: Option<Source>,
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
                source: None,
                name: if m.peer == coop.peer {
                    own(&m.name)
                } else {
                    m.name.clone()
                },
                color: data::CLASSES
                    .get(m.hero.class)
                    .map_or(0xFFFFFF, |c| c.color),
                // Our own total is live; a teammate's arrives with their pose.
                damage: if m.peer == coop.peer {
                    run.damage_done
                } else {
                    coop.players.get(&m.peer).map_or(0.0, |p| p.damage)
                },
            })
            .collect(),
        None => vec![Entry {
            source: None,
            name: own(&coop.name),
            color: run.class.color,
            damage: run.damage_done,
        }],
    };
    rows.sort_by(|a, b| {
        b.damage
            .total_cmp(&a.damage)
            .then_with(|| a.name.cmp(&b.name))
    });
    rows.truncate(ROWS);
    rows
}

fn rgb(hex: u32) -> Color {
    Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn text(font: &Handle<Font>, size: f32, color: Color, justify: Justify) -> impl Bundle {
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
        TextLayout::new(justify, LineBreak::NoWrap),
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
            panel
                .spawn(Node {
                    width: Val::Percent(100.0),
                    column_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|tabs| {
                    for (mode, label) in [(Mode::Party, "Party DPS"), (Mode::SelfDps, "Self DPS")] {
                        tabs.spawn((
                            Button,
                            ModeButton(mode),
                            Node {
                                flex_grow: 1.0,
                                justify_content: JustifyContent::Center,
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.12, 0.12, 0.12, 0.9)),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new(label),
                                TextFont {
                                    font: f.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    }
                });
            panel.spawn((
                Summary,
                Node {
                    margin: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                Text::new("Party DPS"),
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
            panel
                .spawn((
                    MeterScroll,
                    ScrollPosition::default(),
                    Interaction::None,
                    bevy::ui::FocusPolicy::Pass,
                    Node {
                        flex_direction: FlexDirection::Column,
                        max_height: Val::Px(300.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|panel| {
                    for i in 0..ROWS {
                        panel
                            .spawn((
                                MeterRow(i),
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(ROW_H),
                                    flex_shrink: 0.0,
                                    padding: UiRect::horizontal(Val::Px(4.0)),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::FlexStart,
                                    column_gap: Val::Px(4.0),
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
                                row.spawn((
                                    MeterIcon(i),
                                    ImageNode::default(),
                                    Node {
                                        width: Val::Px(15.0),
                                        height: Val::Px(15.0),
                                        flex_shrink: 0.0,
                                        margin: UiRect::right(Val::Px(3.0)),
                                        display: Display::None,
                                        ..default()
                                    },
                                ));
                                row.spawn((
                                    MeterName(i),
                                    text(f, 11.0, Color::WHITE, Justify::Left),
                                    Node {
                                        flex_grow: 1.0,
                                        flex_basis: Val::Px(0.0),
                                        min_width: Val::Px(0.0),
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                ));
                                row.spawn((
                                    MeterValue(i),
                                    text(f, 10.0, Color::WHITE, Justify::Right),
                                    Node {
                                        width: Val::Px(126.0),
                                        flex_shrink: 0.0,
                                        ..default()
                                    },
                                ));
                            });
                    }
                });
        });
}

#[allow(clippy::type_complexity)]
fn paint(
    run: Res<Run>,
    coop: Res<Coop>,
    phase: Res<State<Phase>>,
    mode: Res<Mode>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut summary: Query<&mut Text, (With<Summary>, Without<MeterName>, Without<MeterValue>)>,
    mut buttons: Query<(&ModeButton, &mut BackgroundColor), Without<MeterFill>>,
    mut meters: Query<&mut Visibility, With<Meter>>,
    mut rows: Query<(&MeterRow, &mut Node), Without<MeterFill>>,
    mut fills: Query<
        (&MeterFill, &mut Node, &mut BackgroundColor),
        (Without<MeterRow>, Without<ModeButton>),
    >,
    mut names: Query<(&MeterName, &mut Text), (Without<MeterValue>, Without<Summary>)>,
    mut values: Query<(&MeterValue, &mut Text), (Without<MeterName>, Without<Summary>)>,
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
    let table = if *mode == Mode::Party {
        entries(&run, &coop)
    } else {
        spell_entries(&run, spells.as_deref())
    };
    let top = table.first().map_or(0.0, |e| e.damage);
    let sum: f64 = table.iter().map(|e| e.damage).sum();
    let secs = f64::from(run.elapsed).max(1.0);
    for mut text in &mut summary {
        let total = if *mode == Mode::Party {
            sum
        } else {
            run.damage_done
        };
        text.0 = format!("{} DPS · {} damage", compact(total / secs), compact(total));
    }
    for (button, mut color) in &mut buttons {
        color.0 = if button.0 == *mode {
            Color::srgb(0.35, 0.27, 0.10)
        } else {
            Color::srgb(0.10, 0.10, 0.10)
        };
    }
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
        let percent = if sum > 0.0 {
            e.damage / sum * 100.0
        } else {
            0.0
        };
        let want = format!(
            "{} DPS · {} · {:.0}%",
            compact(e.damage / secs),
            compact(e.damage),
            percent
        );
        if t.0 != want {
            t.0 = want;
        }
    }
}

fn paint_icons(
    run: Res<Run>,
    mode: Res<Mode>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut icons: Query<(&MeterIcon, &mut Node, &mut ImageNode)>,
    mut cache: Local<std::collections::HashMap<String, Handle<Image>>>,
) {
    let table = if *mode == Mode::SelfDps {
        spell_entries(&run, spells.as_deref())
    } else {
        Vec::new()
    };
    for (slot, mut node, mut image) in &mut icons {
        let Some(source) = table.get(slot.0).and_then(|e| e.source) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        let path = source.info(run.class.id, spells.as_deref()).1;
        let handle = cache.get(&path).cloned().or_else(|| {
            let assets = assets.as_deref_mut()?;
            let handle = assets
                .sprite_texture(&format!("{path}.blp"), &mut images)
                .or_else(|| {
                    assets
                        .sprite_texture("Interface\\Icons\\INV_Misc_QuestionMark.blp", &mut images)
                })?;
            cache.insert(path, handle.clone());
            Some(handle)
        });
        image.image = handle.unwrap_or_default();
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
            members: vec![
                member(0, "Host", 0),
                member(1, "Me", 1),
                member(2, "Late", 2),
            ],
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
