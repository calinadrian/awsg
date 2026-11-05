//! Brief, non-modal announcements and confirmed equipment rewards.
use super::{
    Phase, Run, Selection, hud::HudFont, hud_layout::Movable, inventory::Equipment,
    inventory_ui::ItemIcon, right_column as col,
};
use bevy::prelude::*;
use std::collections::VecDeque;

const DURATION: f32 = 3.;
const ANNOUNCEMENT: usize = usize::MAX;

#[derive(Component)]
enum Root {
    Announcement,
    Loot,
}
#[derive(Component)]
struct Progress(usize);

#[derive(Default)]
struct Signal(Option<(String, f32)>);
impl Signal {
    fn observe(&mut self, value: &Option<(String, f32)>) -> Option<String> {
        let fresh = value.as_ref().is_some_and(|(text, left)| {
            *left > 0.
                && self
                    .0
                    .as_ref()
                    .is_none_or(|(old, previous)| text != old || *left > *previous + 0.05)
        });
        self.0 = value.clone();
        fresh.then(|| value.as_ref().unwrap().0.clone())
    }
}
struct Announcement {
    heading: String,
    text: String,
    subtitle: String,
    urgent: bool,
    left: f32,
}
struct Drop {
    definition: String,
    source: String,
    count: u32,
    left: f32,
}
#[derive(Resource, Default)]
struct Notices {
    banner: Signal,
    yell: Signal,
    announcement: Option<Announcement>,
    visible: Vec<Drop>,
    waiting: VecDeque<Drop>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Notices>().add_systems(
        Update,
        update
            .after(super::hud::refresh_hud)
            .after(super::hud::show_panels)
            .after(super::plates::boss_bar),
    );
}

fn describe(text: String, arrival: bool, selection: &Selection) -> Announcement {
    let urgent = text.to_lowercase().contains("berserk");
    let mut subtitle = String::new();
    let (heading, text) = if urgent {
        let (title, detail) = text.split_once("! ").unwrap_or((&text, ""));
        subtitle = detail.into();
        (
            "BERSERK · 3:00".into(),
            title.trim_end_matches('!').to_string() + "!",
        )
    } else if let Some((boss, line)) = text.split_once(" yells: ") {
        let phase = if boss == "Hogger" && line.contains("no run") {
            "BLOOD FRENZY"
        } else if boss == "Morgan" && line.contains("never take") {
            "LAST STAND"
        } else {
            "PHASE"
        };
        (
            format!("{} · {phase}", boss.to_uppercase()),
            line.to_string(),
        )
    } else if arrival {
        let boss = super::data::roster(selection.map)
            .bosses
            .iter()
            .find(|boss| boss.intro == text || text == format!("{} approaches!", boss.name));
        (
            boss.map_or_else(
                || format!("{} · EVENT", selection.map_def().name.to_uppercase()),
                |boss| format!("{} · ARRIVAL", boss.name.to_uppercase()),
            ),
            text,
        )
    } else {
        ("ENCOUNTER · UPDATE".into(), text)
    };
    Announcement {
        heading,
        text,
        subtitle,
        urgent,
        left: DURATION,
    }
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    time: Res<Time>,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    selection: Res<Selection>,
    font: Option<Res<HudFont>>,
    mut gear: ResMut<Equipment>,
    mut state: ResMut<Notices>,
    mut roots: Query<(Entity, &Root, &mut Visibility)>,
    mut progress: Query<(&Progress, &mut Node)>,
) {
    let Some(font) = font else { return };
    let menu = *phase.get() == Phase::Menu;
    let show = matches!(*phase.get(), Phase::Playing | Phase::GameOver);
    let mut dirty = false;
    if menu {
        dirty = state.announcement.is_some() || !state.visible.is_empty();
        *state = Notices::default();
        gear.loot_notices.clear();
    } else {
        // Pauses and reward pickers preserve the full readable lifetime.
        if show {
            if let Some(notice) = &mut state.announcement {
                notice.left -= time.delta_secs();
                if notice.left <= 0. {
                    state.announcement = None;
                    dirty = true;
                }
            }
            for drop in &mut state.visible {
                drop.left -= time.delta_secs();
            }
            let before = state.visible.len();
            state.visible.retain(|drop| drop.left > 0.);
            dirty |= state.visible.len() != before;
        }
        let banner = state.banner.observe(&run.banner);
        let yell = state.yell.observe(&run.yell);
        if let Some((text, arrival)) = yell
            .map(|text| (text, false))
            .or_else(|| banner.map(|text| (text, true)))
        {
            state.announcement = Some(describe(text, arrival, &selection));
            dirty = true;
        }
        for drop in gear.loot_notices.drain(..) {
            if let Some(existing) = state
                .visible
                .iter_mut()
                .find(|shown| shown.definition == drop.definition)
            {
                existing.count += 1;
                existing.left = DURATION;
                if existing.source != drop.source {
                    existing.source = "Multiple drops".into();
                }
            } else if let Some(existing) = state
                .waiting
                .iter_mut()
                .find(|shown| shown.definition == drop.definition)
            {
                existing.count += 1;
                if existing.source != drop.source {
                    existing.source = "Multiple drops".into();
                }
            } else {
                state.waiting.push_back(Drop {
                    definition: drop.definition,
                    source: drop.source,
                    count: 1,
                    left: DURATION,
                });
            }
            dirty = true;
        }
        while state.visible.len() < 3 {
            let Some(drop) = state.waiting.pop_front() else {
                break;
            };
            state.visible.push(drop);
            dirty = true;
        }
    }
    if roots.is_empty() {
        for kind in [Root::Announcement, Root::Loot] {
            let announcement = matches!(kind, Root::Announcement);
            let entity = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Percent(if announcement { 22. } else { 56. }),
                        left: if announcement {
                            Val::Percent(50.)
                        } else {
                            Val::Auto
                        },
                        right: if announcement {
                            Val::Auto
                        } else {
                            Val::Px(12.)
                        },
                        margin: if announcement {
                            UiRect::left(Val::Px(-170.))
                        } else {
                            UiRect::ZERO
                        },
                        width: Val::Px(if announcement { 340. } else { 260. }),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.),
                        ..default()
                    },
                    Movable::new(
                        if announcement {
                            "announcements"
                        } else {
                            "item_drops"
                        },
                        if announcement {
                            "Announcements"
                        } else {
                            "Item drops"
                        },
                    ),
                    GlobalZIndex(405),
                    Visibility::Hidden,
                    bevy::ui::FocusPolicy::Pass,
                ))
                .id();
            render(&mut commands, entity, &kind, &state, &gear, &font.0);
            commands.entity(entity).insert(kind);
        }
    }
    for (entity, kind, mut visibility) in &mut roots {
        let populated = match kind {
            Root::Announcement => state.announcement.is_some(),
            Root::Loot => !state.visible.is_empty(),
        };
        let allowed = show
            && populated
            && (!matches!(kind, Root::Announcement) || *phase.get() == Phase::Playing);
        *visibility = if allowed {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if dirty {
            commands.entity(entity).despawn_related::<Children>();
            render(&mut commands, entity, kind, &state, &gear, &font.0);
        }
    }
    for (bar, mut node) in &mut progress {
        let left = if bar.0 == ANNOUNCEMENT {
            state.announcement.as_ref().map_or(0., |notice| notice.left)
        } else {
            state.visible.get(bar.0).map_or(0., |drop| drop.left)
        };
        node.width = Val::Percent((left / DURATION * 100.).clamp(0., 100.));
    }
}

fn label(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    text: impl Into<String>,
    size: f32,
    color: Color,
    centered: bool,
) {
    p.spawn((
        Text::new(text),
        TextFont {
            font: f.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::ONE,
            color: Color::BLACK,
        },
        TextLayout::new_with_justify(if centered {
            Justify::Center
        } else {
            Justify::Left
        }),
        Node {
            min_width: Val::Px(0.),
            flex_shrink: 0.,
            ..default()
        },
    ));
}
fn frame() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.),
            padding: UiRect::all(Val::Px(2.)),
            border: UiRect::all(Val::Px(1.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.),
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(Color::srgb(0.025, 0.035, 0.045)),
        BorderColor::all(col::STEEL_LO),
    )
}
fn inner() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.),
            padding: UiRect::axes(Val::Px(9.), Val::Px(7.)),
            border: UiRect::all(Val::Px(1.)),
            row_gap: Val::Px(4.),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(col::INK),
        BorderColor::all(col::SLATE),
    )
}
fn bar(p: &mut ChildSpawnerCommands, id: usize, left: f32, color: Color) {
    p.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Px(2.),
            ..default()
        },
        BackgroundColor(col::KNOB),
    ))
    .with_children(|p| {
        p.spawn((
            Progress(id),
            Node {
                width: Val::Percent(left / DURATION * 100.),
                height: Val::Percent(100.),
                ..default()
            },
            BackgroundColor(color),
        ));
    });
}
fn rarity(quality: &str) -> Color {
    match quality.to_ascii_lowercase().as_str() {
        "poor" | "0" => Color::srgb_u8(157, 157, 157),
        "uncommon" | "2" => Color::srgb_u8(100, 189, 118),
        "rare" | "3" => Color::srgb_u8(92, 157, 233),
        "epic" | "4" => Color::srgb_u8(181, 133, 225),
        "legendary" | "5" => Color::srgb_u8(255, 128, 0),
        "artifact" | "heirloom" | "6" | "7" => Color::srgb_u8(230, 204, 128),
        _ => col::TEXT_HI,
    }
}
fn render(
    commands: &mut Commands,
    entity: Entity,
    kind: &Root,
    state: &Notices,
    gear: &Equipment,
    font: &Handle<Font>,
) {
    commands.entity(entity).with_children(|p| match kind {
        Root::Announcement => {
            let Some(notice) = &state.announcement else {
                return;
            };
            p.spawn(frame()).with_children(|p| {
                p.spawn(inner()).with_children(|p| {
                    p.spawn(Node {
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(7.),
                        ..default()
                    })
                    .with_children(|p| {
                        diamond(p, col::CRIMSON);
                        label(
                            p,
                            font,
                            &notice.heading,
                            9.,
                            if notice.urgent {
                                col::CRIMSON
                            } else {
                                col::STEEL_HI
                            },
                            true,
                        );
                        diamond(p, col::CRIMSON);
                    });
                    label(
                        p,
                        font,
                        &notice.text,
                        17.,
                        if notice.urgent {
                            Color::srgb_u8(240, 154, 167)
                        } else {
                            col::TEXT_HI
                        },
                        true,
                    );
                    if !notice.subtitle.is_empty() {
                        label(p, font, &notice.subtitle, 10., col::STEEL, true);
                    }
                });
                bar(p, ANNOUNCEMENT, notice.left, col::WINE);
            });
        }
        Root::Loot => {
            for (index, drop) in state.visible.iter().enumerate() {
                let Some(item) = gear.catalog.definition(&drop.definition) else {
                    continue;
                };
                let color = rarity(&item.quality);
                p.spawn((frame(), GlobalZIndex(445))).with_children(|p| {
                    p.spawn(inner()).with_children(|p| {
                        p.spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(9.),
                            ..default()
                        })
                        .with_children(|p| {
                            p.spawn((
                                Node {
                                    width: Val::Px(36.),
                                    height: Val::Px(36.),
                                    flex_shrink: 0.,
                                    padding: UiRect::all(Val::Px(2.)),
                                    border: UiRect::all(Val::Px(1.)),
                                    ..default()
                                },
                                BackgroundColor(col::KNOB),
                                BorderColor::all(color),
                            ))
                            .with_children(|p| {
                                p.spawn((
                                    ItemIcon(item.icon.clone()),
                                    ImageNode::default(),
                                    Node {
                                        width: Val::Percent(100.),
                                        height: Val::Percent(100.),
                                        ..default()
                                    },
                                ));
                                if drop.count > 1 {
                                    p.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            right: Val::Px(-3.),
                                            bottom: Val::Px(-3.),
                                            padding: UiRect::horizontal(Val::Px(2.)),
                                            ..default()
                                        },
                                        BackgroundColor(col::KNOB),
                                    ))
                                    .with_children(|p| {
                                        label(
                                            p,
                                            font,
                                            format!("×{}", drop.count),
                                            10.,
                                            col::TEXT_HI,
                                            false,
                                        )
                                    });
                                }
                            });
                            p.spawn(Node {
                                flex_grow: 1.,
                                flex_basis: Val::Px(0.),
                                min_width: Val::Px(0.),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(4.),
                                ..default()
                            })
                            .with_children(|p| {
                                label(p, font, &item.name, 13., color, false);
                                label(
                                    p,
                                    font,
                                    format!(
                                        "iLv {} · {} · {}",
                                        item.item_level(&gear.catalog, 0),
                                        item.slot.replace('_', " "),
                                        item.quality
                                    ),
                                    9.,
                                    col::STEEL,
                                    false,
                                );
                                p.spawn(Node {
                                    column_gap: Val::Px(6.),
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..default()
                                })
                                .with_children(|p| {
                                    label(
                                        p,
                                        font,
                                        "✓ Inventory",
                                        9.,
                                        Color::srgb_u8(165, 182, 159),
                                        false,
                                    );
                                    p.spawn(Node {
                                        min_width: Val::Px(0.),
                                        max_width: Val::Px(100.),
                                        overflow: Overflow::clip(),
                                        ..default()
                                    })
                                    .with_children(|p| {
                                        label(p, font, &drop.source, 9., col::TEXT_DIM, false)
                                    });
                                });
                            });
                        });
                    });
                    bar(p, index, drop.left, color);
                });
            }
        }
    });
}
fn diamond(p: &mut ChildSpawnerCommands, color: Color) {
    p.spawn((
        Node {
            width: Val::Px(5.),
            height: Val::Px(5.),
            flex_shrink: 0.,
            ..default()
        },
        UiTransform::from_rotation(Rot2::degrees(45.)),
        BackgroundColor(color),
    ));
}
