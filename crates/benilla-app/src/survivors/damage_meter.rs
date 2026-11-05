//! The damage meter in the right column, under the run tracker: each hero's damage this run,
//! largest first, with its DPS, as small crimson bars in the boss bars' steel frame. Every client counts only its own hits (health actually taken, so overkill and hits
//! on a replica the host already killed add nothing) and carries the total in its co-op pose;
//! the host relays every pose, so all players read the same table.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use super::coop::Coop;
use super::hud::{compact, HudFont};
use super::right_column::{self as col, HudSizes, Slot};
use super::{data, Phase, Run};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Source {
    Spell(u32),
    Talent(&'static str),
    Jewelry(usize),
    Amulet(usize),
    Food(super::food::FoodId),
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
            Self::Food(id) => (format!("{} (Food)", id.food().name), id.food().icon.into()),
        }
    }
}

/// Room for party members or the hero's spells, talents and item effects.
const ROWS: usize = 32;
const WIDTH: f32 = 210.0;
const ROW_H: f32 = 14.0;
const TOP: f32 = 8.0;

/// The meter, built at this scale.
#[derive(Component)]
struct Meter(f32);
/// The column down the right edge: the minimap, the run tracker, this meter, then the quest
/// tracker ([`super::right_column`]).
#[derive(Component)]
pub(super) struct RightStack;
#[derive(Component)]
struct MeterRow(usize);
#[derive(Component)]
struct MeterFill(usize);
#[derive(Component)]
struct MeterIcon(usize);
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MeterText {
    Rank(usize),
    Name(usize),
    Value(usize),
    Dps(usize),
    Total,
    TotalDps,
    Clock,
}

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    Party,
    SelfDps,
}
#[derive(Component)]
struct ModeButton(Mode);
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
            damage,
            own: true,
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
    damage: f64,
    /// The local hero's own row, drawn in crimson; teammates' are slate.
    own: bool,
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
                // Our own total is live; a teammate's arrives with their pose.
                damage: if m.peer == coop.peer {
                    run.damage_done
                } else {
                    coop.players.get(&m.peer).map_or(0.0, |p| p.damage)
                },
                own: m.peer == coop.peer,
            })
            .collect(),
        None => vec![Entry {
            source: None,
            name: own(&coop.name),
            damage: run.damage_done,
            own: true,
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

fn spawn(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    sizes: Res<HudSizes>,
    stacks: Query<Entity, With<RightStack>>,
    meters: Query<(Entity, &Meter)>,
) {
    let Some(font) = font else {
        return;
    };
    let stack = match stacks.single() {
        Ok(stack) => stack,
        Err(_) => commands
            .spawn((
                RightStack,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(8.0),
                    top: Val::Px(TOP),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                GlobalZIndex(85),
            ))
            .id(),
    };
    let k = sizes.meter;
    if let Some((e, meter)) = meters.iter().next() {
        if meter.0 == k {
            return;
        }
        commands.entity(e).despawn();
    }
    let f = &font.0;
    let px = |v: f32| Val::Px(v * k);
    let text = |which: MeterText, size: f32, color: Color| (which, col::label(f, "", size * k, color));
    commands
        .spawn((
            Meter(k),
            Slot(2),
            ChildOf(stack),
            super::hud_layout::Movable::new("meter", "Damage meter"),
            col::plate(Node {
                width: px(WIDTH),
                flex_direction: FlexDirection::Column,
                row_gap: px(1.0),
                padding: UiRect {
                    left: px(5.0),
                    right: px(5.0),
                    top: px(4.0),
                    bottom: px(5.0),
                },
                ..default()
            }),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: px(8.0),
                    padding: UiRect::horizontal(px(2.0)),
                    margin: UiRect::bottom(px(2.0)),
                    ..default()
                })
                .with_children(|tabs| {
                    for (mode, label) in [(Mode::Party, "Party"), (Mode::SelfDps, "Self")] {
                        tabs.spawn((
                            Button,
                            ModeButton(mode),
                            Node {
                                padding: UiRect::bottom(px(1.0)),
                                border: UiRect::bottom(px(2.0)),
                                ..default()
                            },
                            BorderColor::all(Color::NONE),
                        ))
                        .with_children(|b| {
                            b.spawn((ModeButton(mode), col::label(f, label, 10.0 * k, col::STEEL_LO)));
                        });
                    }
                    tabs.spawn(Node {
                        flex_grow: 1.0,
                        ..default()
                    });
                    tabs.spawn(text(MeterText::Clock, 10.0, col::STEEL));
                });
            panel
                .spawn((
                    MeterScroll,
                    ScrollPosition::default(),
                    Interaction::None,
                    bevy::ui::FocusPolicy::Pass,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(1.0),
                        max_height: px(240.0),
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
                                    height: px(ROW_H),
                                    flex_shrink: 0.0,
                                    padding: UiRect::horizontal(px(4.0)),
                                    align_items: AlignItems::Center,
                                    column_gap: px(4.0),
                                    display: Display::None,
                                    ..default()
                                },
                                BackgroundColor(col::WELL),
                                Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
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
                                    BackgroundColor(col::WINE),
                                ));
                                row.spawn((text(MeterText::Rank(i), 9.0, col::STEEL), Node {
                                    min_width: px(9.0),
                                    ..default()
                                }));
                                row.spawn((
                                    MeterIcon(i),
                                    ImageNode::default(),
                                    Node {
                                        width: px(12.0),
                                        height: px(12.0),
                                        flex_shrink: 0.0,
                                        display: Display::None,
                                        ..default()
                                    },
                                ));
                                row.spawn((
                                    text(MeterText::Name(i), 10.0, col::TEXT_HI),
                                    Node {
                                        flex_grow: 1.0,
                                        flex_basis: Val::Px(0.0),
                                        min_width: Val::Px(0.0),
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                ));
                                row.spawn(text(MeterText::Value(i), 10.0, col::TEXT_HI));
                                row.spawn((
                                    text(MeterText::Dps(i), 8.0, col::TEXT_DIM),
                                    Node {
                                        min_width: px(20.0),
                                        justify_content: JustifyContent::FlexEnd,
                                        ..default()
                                    },
                                ));
                            });
                    }
                });
            panel.spawn((
                Node {
                    height: Val::Px(1.0),
                    margin: UiRect::top(px(3.0)),
                    ..default()
                },
                BackgroundColor(col::STEEL_LO.with_alpha(0.45)),
            ));
            panel
                .spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: px(4.0),
                    padding: UiRect::horizontal(px(4.0)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        col::label(f, "Total", 10.0 * k, col::STEEL),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                    ));
                    row.spawn(text(MeterText::Total, 10.0, col::TEXT_HI));
                    row.spawn(text(MeterText::TotalDps, 8.0, col::TEXT_DIM));
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
    mut buttons: Query<(&ModeButton, Option<&mut BorderColor>, Option<&mut TextColor>)>,
    mut meters: Query<&mut Visibility, With<Meter>>,
    mut rows: Query<(&MeterRow, &mut Node), Without<MeterFill>>,
    mut fills: Query<(&MeterFill, &mut Node, &mut BackgroundColor), Without<MeterRow>>,
    mut texts: Query<(&MeterText, &mut Text)>,
) {
    let Ok(mut visibility) = meters.single_mut() else {
        return;
    };
    // Not behind the pause menu or the results, which have the screen to themselves.
    let shown = matches!(*phase.get(), Phase::Playing | Phase::LevelUp);
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
    for (button, border, color) in &mut buttons {
        let on = button.0 == *mode;
        if let Some(mut border) = border {
            let want = BorderColor::all(if on { col::WINE_HI } else { Color::NONE });
            if *border != want {
                *border = want;
            }
        }
        if let Some(mut color) = color {
            color.set_if_neq(TextColor(if on { col::TEXT_HI } else { col::STEEL_LO }));
        }
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
        color.set_if_neq(BackgroundColor(if e.own { col::WINE } else { col::SLATE }));
    }
    let total = if *mode == Mode::Party {
        sum
    } else {
        run.damage_done
    };
    for (which, mut t) in &mut texts {
        let want = match *which {
            MeterText::Rank(i) => match table.get(i) {
                Some(_) => (i + 1).to_string(),
                None => continue,
            },
            MeterText::Name(i) => match table.get(i) {
                Some(e) => e.name.clone(),
                None => continue,
            },
            MeterText::Value(i) => match table.get(i) {
                Some(e) => compact(e.damage),
                None => continue,
            },
            MeterText::Dps(i) => match table.get(i) {
                Some(e) => compact(e.damage / secs),
                None => continue,
            },
            MeterText::Total => compact(total),
            MeterText::TotalDps => format!("{} DPS", compact(total / secs)),
            MeterText::Clock => {
                let s = run.elapsed as u32;
                format!("{:02}:{:02}", s / 60, s % 60)
            }
        };
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
