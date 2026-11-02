//! Hero selection frames the live preview: a header with the account screens, the hero on the
//! left, the adventure on the right, the pick's name plate under the hero and one action bar.
//!
//! Laid out on the 768-unit-tall canvas; every panel is bounded so 4:3 (1024 wide) still fits.
use super::{
    data, front_end as skin,
    hud::{HudFont, MenuRoot},
    menu::{ClassSlot, MenuAction, MenuBuilt, PickDescription, PickName, RaceFace},
    menu_icons::icon_button,
    races::{RACES, race_tc},
};
use crate::glue::{
    art::{GlueArt, class_tc, tc_rect},
    widgets::{GlueDisabled, LockHighlight},
};
use bevy::prelude::*;

/// Canvas edge to the side panels, and the gap between panels.
const MARGIN: f32 = 20.;
const HEADER: f32 = 64.;
const HERO_WIDTH: f32 = 268.;
const ADVENTURE_WIDTH: f32 = 282.;
/// The panels' bottom edge, clear of the action bar and the key hint under it.
const PANELS_BOTTOM: f32 = 110.;
const BAR_BOTTOM: f32 = 34.;

pub(super) fn build_menu(
    mut commands: Commands,
    roots: Query<Entity, (With<MenuRoot>, Without<MenuBuilt>)>,
    assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut art: ResMut<GlueArt>,
    mut images: ResMut<Assets<Image>>,
    mut add_mats: ResMut<Assets<crate::glue::add_material::AddUiMaterial>>,
    font: Res<HudFont>,
) {
    let Ok(root) = roots.single() else { return };
    let Some(mut assets) = assets else { return };
    art.ensure_loaded(&mut assets, &mut images, &mut add_mats);
    let f = &font.0;
    commands.entity(root).insert(MenuBuilt).with_children(|ui| {
        header(ui, f);
        tower(ui, &art, f);
        // The hero's looks over the adventure, in the right column.
        super::features_ui::build(
            ui,
            f,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(MARGIN),
                top: Val::Px(HEADER + 16.),
                width: Val::Px(ADVENTURE_WIDTH),
                ..default()
            },
            heading,
        );
        adventure(ui, f);
        pick_plate(ui, f);
        action_bar(ui, f);
        ui.spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.),
            width: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|p| {
            skin::label(
                p,
                f,
                "ENTER  Play solo      F6  Co-op      ESC  Settings",
                11.,
                skin::MUTED,
            );
        });
    });
}

/// The title on the left; the screens that leave the pick on the right.
fn header(ui: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    ui.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.),
            width: Val::Percent(100.),
            height: Val::Px(HEADER),
            padding: UiRect::horizontal(Val::Px(MARGIN + 4.)),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            border: UiRect::bottom(Val::Px(1.)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.023, 0.02, 0.86)),
        BorderColor::all(skin::EDGE),
    ))
    .with_children(|p| {
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.),
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, "AZEROTH SURVIVORS", 24., skin::GOLD);
            skin::label(
                p,
                f,
                "Choose your hero. Begin your adventure.",
                11.,
                skin::MUTED,
            );
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.),
            ..default()
        })
        .with_children(|p| {
            skin::tab(p, f, "Inventory", super::inventory_ui::Action::Open(false));
            skin::tab(
                p,
                f,
                "Auction House",
                super::inventory_ui::Action::Open(true),
            );
            skin::tab(p, f, "Settings", MenuAction::Settings);
        });
    });
}

/// A panel's title, centred between two rules; the same on every panel.
fn heading(p: &mut ChildSpawnerCommands, f: &Handle<Font>, text: &str) {
    p.spawn(Node {
        width: Val::Percent(100.),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(10.),
        margin: UiRect::bottom(Val::Px(2.)),
        ..default()
    })
    .with_children(|row| {
        let line = || {
            (
                Node {
                    flex_grow: 1.,
                    height: Val::Px(1.),
                    ..default()
                },
                BackgroundColor(skin::EDGE),
            )
        };
        row.spawn(line());
        skin::label(row, f, text, 16., skin::GOLD);
        row.spawn(line());
    });
}

/// A small centred caption over a group of controls.
fn caption(p: &mut ChildSpawnerCommands, f: &Handle<Font>, text: &str, color: Color) {
    let e = skin::label(p, f, text, 10., color);
    p.commands().entity(e).insert((
        Node {
            width: Val::Percent(100.),
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
    ));
}

const ALLIANCE: Color = Color::srgb(0.42, 0.62, 0.95);
const HORDE: Color = Color::srgb(0.92, 0.36, 0.3);

/// The race shown beside the gender buttons: the hovered one, else the picked one.
#[derive(Component)]
pub(super) struct RaceCaption;

fn race_icons(grid: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>, faction: u8) {
    for def in RACES.iter().filter(|r| r.faction == faction) {
        let face = art.races.as_ref().and_then(|(sheet, size)| {
            race_tc(def.id, 0).map(|tc| (sheet.clone(), tc_rect(*size, tc)))
        });
        // The name shows in the [`RaceCaption`], not under the icon, where it would cover the
        // next row.
        icon_button(
            grid,
            f,
            MenuAction::Race(def.id),
            Some(RaceFace(def.id)),
            face,
            None::<()>,
            "",
            art,
            26. / 48.,
        );
    }
}

fn race_grid(width: Val) -> Node {
    Node {
        width,
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(4.),
        row_gap: Val::Px(4.),
        ..default()
    }
}

fn tower(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    ui.spawn(skin::panel(Node {
        position_type: PositionType::Absolute,
        left: Val::Px(MARGIN),
        top: Val::Px(HEADER + 16.),
        bottom: Val::Px(PANELS_BOTTOM),
        width: Val::Px(HERO_WIDTH),
        padding: UiRect::all(Val::Px(16.)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: Val::Px(7.),
        ..default()
    }))
    .with_children(|p| {
        heading(p, f, "YOUR HERO");
        // Alliance and Horde side by side, the races of neither under them.
        p.spawn(Node {
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|sides| {
            for (faction, name, color) in [(0, "ALLIANCE", ALLIANCE), (1, "HORDE", HORDE)] {
                sides
                    .spawn(Node {
                        width: Val::Px(116.),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(5.),
                        ..default()
                    })
                    .with_children(|col| {
                        caption(col, f, name, color);
                        let grid = race_grid(Val::Px(116.));
                        col.spawn(grid)
                            .with_children(|g| race_icons(g, art, f, faction));
                    });
            }
        });
        caption(p, f, "NEUTRAL", skin::MUTED);
        let grid = race_grid(Val::Percent(100.));
        p.spawn(grid).with_children(|g| race_icons(g, art, f, 2));
        // The picked (or pointed-at) race, then the gender.
        p.spawn(Node {
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.),
            margin: UiRect::top(Val::Px(2.)),
            ..default()
        })
        .with_children(|row| {
            let name = skin::label(row, f, "", 15., skin::TEXT);
            row.commands().entity(name).insert((
                RaceCaption,
                Node {
                    flex_grow: 1.,
                    ..default()
                },
            ));
            for (sex, label) in [(0, "Male"), (1, "Female")] {
                row.spawn(Node {
                    width: Val::Px(72.),
                    ..default()
                })
                .with_children(|p| {
                    let e = skin::button(p, f, label, "", MenuAction::Sex(sex), false, false, true);
                    p.commands().entity(e).insert(LockHighlight::default());
                });
            }
        });
        heading(p, f, "CLASS");
        p.spawn(Node {
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(6.),
            row_gap: Val::Px(8.),
            ..default()
        })
        .with_children(|grid| {
            for (i, class) in data::CLASSES.iter().enumerate() {
                grid.spawn((
                    ClassSlot(i),
                    Node {
                        width: Val::Px(74.),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(4.),
                        ..default()
                    },
                ))
                .with_children(|p| {
                    let face = art.classes.as_ref().and_then(|(sheet, size)| {
                        class_tc(class.id).map(|tc| (sheet.clone(), tc_rect(*size, tc)))
                    });
                    // Named by the label under it; no hover caption on top of that.
                    icon_button(
                        p,
                        f,
                        MenuAction::Class(i),
                        None::<()>,
                        face,
                        None::<()>,
                        "",
                        art,
                        40. / 48.,
                    );
                    skin::label(p, f, class.name, 11., super::hud::rgb(class.color));
                });
            }
        });
        p.spawn(Node {
            flex_grow: 1.,
            min_height: Val::Px(4.),
            ..default()
        });
        skin::button(
            p,
            f,
            "Randomize hero",
            "",
            MenuAction::Randomize,
            false,
            false,
            true,
        );
    });
}

/// A value box on the pick plate.
#[derive(Component, Clone, Copy)]
pub(super) enum PlateStat {
    Health,
    Reduction,
    Start,
    Zones,
}

/// The shown race's name over its bonuses.
#[derive(Component)]
pub(super) struct RaceTitle;

/// A spell-pool icon's frame, `.0` into the class's pool; hidden past the pool's end.
#[derive(Component)]
pub(super) struct PoolSlot(usize);

/// Enough frames for the longest class pool.
const POOL_SLOTS: usize = 16;
const POOL_ICON: f32 = 30.;

/// Under the hero: the pick's name, its race's bonuses, the class's numbers and its spell pool.
fn pick_plate(ui: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    ui.spawn(skin::panel(Node {
        position_type: PositionType::Absolute,
        left: Val::Px(MARGIN * 2. + HERO_WIDTH),
        right: Val::Px(MARGIN * 2. + ADVENTURE_WIDTH),
        bottom: Val::Px(PANELS_BOTTOM),
        padding: UiRect::axes(Val::Px(18.), Val::Px(14.)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.),
        ..default()
    }))
    .with_children(|p| {
        // Two columns side by side, stacked when the canvas is too narrow for both.
        p.spawn(Node {
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(18.),
            row_gap: Val::Px(10.),
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                flex_grow: 1.,
                flex_basis: Val::Px(170.),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.),
                ..default()
            })
            .with_children(|col| {
                let title = skin::label(col, f, "", 24., skin::GOLD);
                col.commands().entity(title).insert(PickName);
            });
            row.spawn(Node {
                flex_grow: 3.,
                flex_basis: Val::Px(340.),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.),
                ..default()
            })
            .with_children(|col| {
                col.spawn(Node {
                    width: Val::Percent(100.),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(6.),
                    row_gap: Val::Px(6.),
                    ..default()
                })
                .with_children(|boxes| {
                    stat_box(boxes, f, "HEALTH", PlateStat::Health);
                    stat_box(boxes, f, "REDUCTION", PlateStat::Reduction);
                    stat_box(boxes, f, "STARTS WITH", PlateStat::Start);
                    stat_box(boxes, f, "ZONES OPEN", PlateStat::Zones);
                });
                skin::label(col, f, "SPELL POOL", 10., skin::MUTED);
                col.spawn(Node {
                    width: Val::Percent(100.),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.),
                    row_gap: Val::Px(4.),
                    ..default()
                })
                .with_children(|icons| {
                    for i in 0..POOL_SLOTS {
                        icons
                            .spawn((
                                PoolSlot(i),
                                super::hud::TipFor::Pool(""),
                                Interaction::None,
                                Node {
                                    width: Val::Px(POOL_ICON),
                                    height: Val::Px(POOL_ICON),
                                    border: UiRect::all(Val::Px(1.)),
                                    border_radius: BorderRadius::all(Val::Px(3.)),
                                    display: Display::None,
                                    ..default()
                                },
                                BorderColor::all(skin::EDGE),
                            ))
                            .with_children(|slot| {
                                slot.spawn((
                                    Node {
                                        width: Val::Percent(100.),
                                        height: Val::Percent(100.),
                                        ..default()
                                    },
                                    ImageNode {
                                        color: Color::srgba(0., 0., 0., 0.6),
                                        ..default()
                                    },
                                    super::hud::SpellIcon(0),
                                ));
                            });
                    }
                });
            });
        });
        // The race's traits, across the whole plate so each fits on its line.
        p.spawn(Node {
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.),
            ..default()
        })
        .with_children(|col| {
            let race = skin::label(col, f, "", 14., skin::GOLD);
            col.commands().entity(race).insert(RaceTitle);
            let traits = skin::label(col, f, "", 12., skin::TEXT);
            col.commands().entity(traits).insert(PickDescription);
        });
        skin::rule(p);
        let progress = skin::label(p, f, "", 11., skin::MUTED);
        p.commands().entity(progress).insert((
            super::talents::MenuProgress,
            Node {
                width: Val::Percent(100.),
                ..default()
            },
            TextLayout::new_with_justify(Justify::Center),
        ));
    });
}

fn stat_box(p: &mut ChildSpawnerCommands, f: &Handle<Font>, caption: &str, stat: PlateStat) {
    p.spawn((
        Node {
            flex_grow: 1.,
            padding: UiRect::axes(Val::Px(8.), Val::Px(6.)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(5.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.),
            ..default()
        },
        BackgroundColor(skin::SURFACE),
        BorderColor::all(skin::EDGE),
    ))
    .with_children(|b| {
        skin::label(b, f, caption, 9., skin::MUTED);
        let value = skin::label(b, f, "", 15., skin::TEXT);
        b.commands().entity(value).insert(stat);
    });
}

/// The race's traits ([`super::races::RACIALS`]), one bullet each.
fn race_traits(race: u8) -> String {
    let lines = super::races::trait_lines(race);
    if lines.is_empty() {
        return "No racial traits".into();
    }
    lines
        .iter()
        .map(|l| format!("•  {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The race the plate describes: the one under the pointer, else the picked one.
fn shown_race(
    selection: &super::Selection,
    buttons: &Query<(&MenuAction, &Interaction)>,
) -> Option<u8> {
    buttons
        .iter()
        .find_map(|(a, i)| match a {
            MenuAction::Race(r) if *i != Interaction::None => Some(*r),
            _ => None,
        })
        .or_else(|| {
            selection
                .look
                .and_then(|i| selection.looks.get(i))
                .map(|l| l.1)
        })
}

pub(super) fn refresh_pick_plate(
    mut commands: Commands,
    selection: Res<super::Selection>,
    buttons: Query<(&MenuAction, &Interaction)>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut texts: ParamSet<(
        Query<(&PlateStat, &mut Text)>,
        Query<&mut Text, With<RaceTitle>>,
        Query<&mut Text, With<PickDescription>>,
    )>,
    mut slots: Query<(&PoolSlot, &mut Node, &Children, &mut super::hud::TipFor)>,
    mut icons: Query<&mut super::hud::SpellIcon>,
    mut shown: Local<Option<(Option<usize>, Option<u8>, bool, usize)>>,
) {
    let race = shown_race(&selection, &buttons);
    let key = (selection.class, race, spells.is_some(), slots.iter().len());
    if *shown == Some(key) {
        return;
    }
    *shown = Some(key);
    let class = selection.class_def();
    let (hp, reduction) = class.map_or((0., 0.), |c| super::class_base(c.id));
    let open = data::MAPS
        .iter()
        .filter(|m| super::dungeon::available(m))
        .count();
    for (stat, mut text) in &mut texts.p0() {
        text.0 = match (stat, class) {
            (_, None) => String::new(),
            (PlateStat::Health, _) => format!("{hp:.0}"),
            (PlateStat::Reduction, _) => format!("{:.0}%", reduction * 100.),
            (PlateStat::Start, Some(c)) => {
                super::hud::spell_name(spells.as_deref(), data::ability(c.start).spell, c.start)
            }
            (PlateStat::Zones, _) => format!("{open} of {}", data::MAPS.len()),
        };
    }
    let name = race.map_or("", super::races::race_name);
    for mut text in &mut texts.p1() {
        text.0 = name.into();
    }
    for mut text in &mut texts.p2() {
        text.0 = race.map_or_else(String::new, race_traits);
    }
    let pool = class.map_or(&[][..], |c| c.pool);
    for (slot, mut node, children, mut tip) in &mut slots {
        let key = pool.get(slot.0).copied();
        if let Some(key) = key {
            *tip = super::hud::TipFor::Pool(key);
        }
        let spell = key.map(|k| data::ability(k).spell);
        node.display = if spell.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        let Some(spell) = spell else { continue };
        for child in children {
            if let Ok(mut icon) = icons.get_mut(*child) {
                if icon.0 != spell {
                    icon.0 = spell;
                    commands.entity(*child).remove::<super::hud::IconDone>();
                }
            }
        }
    }
}

#[derive(Component)]
pub(super) struct AdventureDetails;

/// The detail line of the play button, naming the selected arena.
#[derive(Component)]
pub(super) struct PlayTarget;

fn adventure(ui: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    ui.spawn(skin::panel(Node {
        position_type: PositionType::Absolute,
        right: Val::Px(MARGIN),
        // Under the FEATURES panel.
        top: Val::Px(HEADER + 16. + super::features_ui::HEIGHT + 12.),
        bottom: Val::Px(PANELS_BOTTOM),
        width: Val::Px(ADVENTURE_WIDTH),
        padding: UiRect::all(Val::Px(16.)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.),
        overflow: Overflow::clip_y(),
        ..default()
    }))
    .with_children(|p| {
        heading(p, f, "YOUR ADVENTURE");
        skin::button(
            p,
            f,
            "Story · Dungeons · Raids",
            "The road so far",
            super::adventure_atlas::Action::Open,
            false,
            false,
            true,
        );
        for (i, map) in data::MAPS.iter().enumerate() {
            let ready = super::dungeon::available(map);
            // Selection and availability are painted by [`skin::hover`] from the [`skin::Control`].
            p.spawn((
                Button,
                MenuAction::Map(i),
                GlueDisabled(!ready),
                skin::Control {
                    primary: false,
                    selected: false,
                    enabled: ready,
                },
                Node {
                    width: Val::Percent(100.),
                    min_height: Val::Px(64.),
                    padding: UiRect::axes(Val::Px(14.), Val::Px(10.)),
                    border: UiRect::all(Val::Px(1.)),
                    border_radius: BorderRadius::all(Val::Px(5.)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(3.),
                    flex_shrink: 0.,
                    ..default()
                },
                BackgroundColor(skin::SURFACE),
                BorderColor::all(skin::EDGE),
            ))
            .with_children(|p| {
                skin::label(
                    p,
                    f,
                    map.name,
                    20.,
                    if ready { skin::GOLD } else { skin::MUTED },
                );
                skin::label(
                    p,
                    f,
                    if ready {
                        map.zone
                    } else {
                        "Dungeon pack required"
                    },
                    10.,
                    skin::MUTED,
                );
            });
        }
        skin::rule(p);
        let details = skin::label(p, f, "", 12., skin::TEXT);
        p.commands().entity(details).insert(AdventureDetails);
    });
}

/// Talents and Co-op flank the one primary action, centred under the preview.
fn action_bar(ui: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    ui.spawn(Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(BAR_BOTTOM),
        width: Val::Percent(100.),
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Stretch,
        column_gap: Val::Px(12.),
        ..default()
    })
    .with_children(|bar| {
        bar.spawn(Node {
            width: Val::Px(200.),
            ..default()
        })
        .with_children(|p| {
            skin::button(
                p,
                f,
                "Talents",
                "Spend your class points",
                MenuAction::Talents,
                false,
                false,
                true,
            );
        });
        bar.spawn((
            Button,
            MenuAction::Enter,
            GlueDisabled(false),
            skin::Control {
                primary: true,
                selected: false,
                enabled: true,
            },
            Node {
                width: Val::Px(360.),
                min_height: Val::Px(62.),
                padding: UiRect::axes(Val::Px(14.), Val::Px(8.)),
                border: UiRect::all(Val::Px(1.)),
                border_radius: BorderRadius::all(Val::Px(5.)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(3.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.29, 0.075, 0.043)),
            BorderColor::all(skin::GOLD),
        ))
        .with_children(|p| {
            skin::label(p, f, "Play solo", 22., skin::GOLD);
            let target = skin::label(p, f, "", 11., skin::TEXT);
            p.commands().entity(target).insert(PlayTarget);
        });
        bar.spawn(Node {
            width: Val::Px(200.),
            ..default()
        })
        .with_children(|p| {
            skin::button(
                p,
                f,
                "Co-op",
                "Host a game or join friends",
                super::coop_ui::OpenCoop,
                false,
                false,
                true,
            );
        });
    });
}

pub(super) fn refresh_adventure(
    selection: Res<super::Selection>,
    buttons: Query<(&MenuAction, &Interaction)>,
    mut labels: ParamSet<(
        Query<&mut Text, With<AdventureDetails>>,
        Query<&mut Text, With<PlayTarget>>,
        Query<&mut Text, With<RaceCaption>>,
    )>,
) {
    let race = shown_race(&selection, &buttons);
    let name = race.map_or("", super::races::race_name);
    for mut text in &mut labels.p2() {
        if text.0 != name {
            text.0 = name.into();
        }
    }
    let Some(map) = data::MAPS.get(selection.map) else {
        return;
    };
    let mut description = format!(
        "Hold your ground in {}. Defeat the final boss to claim victory.\n\nENCOUNTERS\n",
        map.name
    );
    for boss in &data::roster(selection.map).bosses {
        // A dungeon's bosses wait in their rooms rather than arriving on the clock.
        let at = boss.from as u32;
        if at == 0 {
            description.push_str(&format!("·  {}\n", boss.name));
        } else {
            description.push_str(&format!("{:02}:{:02}  {}\n", at / 60, at % 60, boss.name));
        }
    }
    for mut text in &mut labels.p0() {
        if text.0 != description {
            text.0.clone_from(&description);
        }
    }
    let target = format!("Enter {}", map.name);
    for mut text in &mut labels.p1() {
        if text.0 != target {
            text.0.clone_from(&target);
        }
    }
}
