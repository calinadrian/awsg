//! Hero selection frames the live preview: a header with the account screens, the hero on the
//! left, the looks and the adventure on the right, the pick's name plate under the hero and one
//! action bar.
//!
//! Laid out on the 768-unit-tall canvas; every panel is bounded so 4:3 (1024 wide) still fits, and
//! panels in one column are stacked by flexbox, never by summed heights.
use super::{
    data,
    front_end::{self as skin, gap, size},
    hud::{HudFont, MenuRoot},
    inventory::Pane,
    menu::{ClassSlot, MenuAction, MenuBuilt, PickName, RaceFace},
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
/// The pick plate stops here, so it never climbs over the hero's chest.
const PLATE_MAX_HEIGHT: f32 = 236.;

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
        right_column(ui, f);
        pick_plate(ui, f);
        action_bar(ui, f);
        ui.spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.),
            width: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(gap::XL),
            ..default()
        })
        .with_children(|p| {
            skin::keycap(p, f, "ENTER", "Play solo");
            skin::keycap(p, f, "F6", "Co-op");
            skin::keycap(p, f, "ESC", "Settings");
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
            skin::label(p, f, "AZEROTH SURVIVORS", size::TITLE, skin::GOLD);
            skin::label(
                p,
                f,
                "Choose your hero. Begin your adventure.",
                size::SMALL,
                skin::MUTED,
            );
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(gap::S),
            ..default()
        })
        .with_children(|p| {
            use super::inventory_ui::Action;
            skin::tab(p, f, "Inventory", Action::Open(Pane::Inventory));
            skin::tab(p, f, "Quests", Action::Open(Pane::Quests));
            skin::tab(p, f, "Achievements", Action::Open(Pane::Achievements));
            skin::tab(p, f, "Auction House", Action::Open(Pane::Auction));
            skin::tab(p, f, "Settings", MenuAction::Settings);
        });
    });
}

/// A small centred caption over a group of controls.
fn caption(p: &mut ChildSpawnerCommands, f: &Handle<Font>, text: &str, color: Color) {
    let e = skin::label(p, f, text, size::CAPTION, color);
    p.commands().entity(e).insert((
        Node {
            width: Val::Percent(100.),
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
    ));
}

/// The race shown over the gender buttons: the hovered one, else the picked one.
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
        column_gap: Val::Px(gap::XS),
        row_gap: Val::Px(gap::XS),
        ..default()
    }
}

/// A group of the tower's controls; the tower spreads its groups evenly down its height.
fn group(p: &mut ChildSpawnerCommands, children: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node {
        width: Val::Percent(100.),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: Val::Px(6.),
        flex_shrink: 0.,
        ..default()
    })
    .with_children(children);
}

fn tower(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    ui.spawn(skin::panel(Node {
        position_type: PositionType::Absolute,
        left: Val::Px(MARGIN),
        top: Val::Px(HEADER + 16.),
        bottom: Val::Px(PANELS_BOTTOM),
        width: Val::Px(HERO_WIDTH),
        padding: UiRect::all(Val::Px(gap::L)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        row_gap: Val::Px(gap::S),
        overflow: Overflow::clip(),
        ..default()
    }))
    .with_children(|p| {
        group(p, |p| {
            skin::heading(p, f, "YOUR HERO");
            // Alliance and Horde side by side with a rule between, the races of neither under.
            p.spawn(Node {
                width: Val::Percent(100.),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|sides| {
                for (faction, name, color) in [(0, "ALLIANCE", skin::ALLIANCE), (1, "HORDE", skin::HORDE)] {
                    if faction == 1 {
                        sides.spawn((
                            Node {
                                width: Val::Px(1.),
                                margin: UiRect::vertical(Val::Px(2.)),
                                ..default()
                            },
                            BackgroundColor(skin::EDGE),
                        ));
                    }
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
        });
        // The picked (or pointed-at) race on its own line, then the gender, half and half.
        group(p, |p| {
            let name = skin::label(p, f, "", size::LEAD, skin::TEXT);
            p.commands().entity(name).insert((
                RaceCaption,
                Node {
                    width: Val::Percent(100.),
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Center),
            ));
            p.spawn(Node {
                width: Val::Percent(100.),
                column_gap: Val::Px(gap::S),
                ..default()
            })
            .with_children(|row| {
                for (sex, label) in [(0, "Male"), (1, "Female")] {
                    row.spawn(Node {
                        flex_grow: 1.,
                        flex_basis: Val::Px(0.),
                        ..default()
                    })
                    .with_children(|p| {
                        let e = skin::button(p, f, label, "", MenuAction::Sex(sex), false, false, true);
                        p.commands().entity(e).insert(LockHighlight::default());
                    });
                }
            });
        });
        group(p, |p| {
            skin::heading(p, f, "CLASS");
            p.spawn(Node {
                width: Val::Percent(100.),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(6.),
                row_gap: Val::Px(gap::S),
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
                            row_gap: Val::Px(gap::XS),
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
                        skin::label(p, f, class.name, size::SMALL, super::hud::rgb(class.color));
                    });
                }
            });
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
    GearScore,
}

/// The shown race's name over its bonuses.
#[derive(Component)]
pub(super) struct RaceTitle;

/// One of the shown race's traits, `.0` into [`super::races::trait_lines`].
#[derive(Component)]
pub(super) struct TraitLine(usize);
const TRAIT_SLOTS: usize = 6;

/// A spell-pool icon's frame, `.0` into the class's pool; hidden past the pool's end.
#[derive(Component)]
pub(super) struct PoolSlot(usize);

/// Enough frames for the longest class pool.
const POOL_SLOTS: usize = 16;
const POOL_ICON: f32 = 28.;

/// Under the hero: the pick's name and numbers, its spell pool, its race's traits, its progress.
fn pick_plate(ui: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    ui.spawn(skin::panel(Node {
        position_type: PositionType::Absolute,
        left: Val::Px(MARGIN * 2. + HERO_WIDTH),
        right: Val::Px(MARGIN * 2. + ADVENTURE_WIDTH),
        bottom: Val::Px(PANELS_BOTTOM),
        max_height: Val::Px(PLATE_MAX_HEIGHT),
        padding: UiRect::axes(Val::Px(18.), Val::Px(gap::M)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(gap::S),
        overflow: Overflow::clip(),
        ..default()
    }))
    .with_children(|p| {
        // The name, and the numbers in equal boxes beside it; stacked when too narrow.
        p.spawn(Node {
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            column_gap: Val::Px(gap::L),
            row_gap: Val::Px(gap::S),
            flex_shrink: 0.,
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                flex_grow: 1.,
                flex_basis: Val::Px(160.),
                min_width: Val::Px(0.),
                ..default()
            })
            .with_children(|col| {
                let title = skin::label(col, f, "", size::TITLE, skin::GOLD);
                col.commands().entity(title).insert(PickName);
            });
            row.spawn(Node {
                flex_grow: 3.,
                flex_basis: Val::Px(320.),
                column_gap: Val::Px(6.),
                ..default()
            })
            .with_children(|boxes| {
                for (caption, stat) in [
                    ("HEALTH", PlateStat::Health),
                    ("REDUCTION", PlateStat::Reduction),
                    ("STARTS WITH", PlateStat::Start),
                    ("GEAR SCORE", PlateStat::GearScore),
                ] {
                    let value = skin::stat_tile(boxes, f, caption, "", size::LEAD);
                    boxes.commands().entity(value).insert(stat);
                }
            });
        });
        // The class's spells, one row.
        p.spawn(Node {
            width: Val::Percent(100.),
            align_items: AlignItems::Center,
            column_gap: Val::Px(gap::M),
            flex_shrink: 0.,
            ..default()
        })
        .with_children(|row| {
            skin::label(row, f, "SPELL POOL", size::CAPTION, skin::MUTED);
            row.spawn(Node {
                flex_grow: 1.,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(gap::XS),
                row_gap: Val::Px(gap::XS),
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
        // The race's traits in two columns.
        p.spawn(Node {
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(gap::XS),
            flex_shrink: 0.,
            ..default()
        })
        .with_children(|col| {
            let race = skin::label(col, f, "", size::BODY, skin::GOLD);
            col.commands().entity(race).insert(RaceTitle);
            col.spawn(Node {
                width: Val::Percent(100.),
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(2.),
                ..default()
            })
            .with_children(|lines| {
                for i in 0..TRAIT_SLOTS {
                    let line = skin::label(lines, f, "", size::SMALL, skin::TEXT);
                    lines.commands().entity(line).insert((
                        TraitLine(i),
                        Node {
                            width: Val::Percent(50.),
                            padding: UiRect::right(Val::Px(gap::M)),
                            display: Display::None,
                            ..default()
                        },
                    ));
                }
            });
        });
        skin::rule(p);
        let progress = skin::label(p, f, "", size::SMALL, skin::MUTED);
        p.commands().entity(progress).insert((
            super::talents::MenuProgress,
            Node {
                width: Val::Percent(100.),
                flex_shrink: 0.,
                ..default()
            },
            TextLayout::new_with_justify(Justify::Center),
        ));
    });
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

#[allow(clippy::type_complexity)]
pub(super) fn refresh_pick_plate(
    mut commands: Commands,
    selection: Res<super::Selection>,
    buttons: Query<(&MenuAction, &Interaction)>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut texts: ParamSet<(
        Query<(&PlateStat, &mut Text)>,
        Query<&mut Text, With<RaceTitle>>,
        Query<(&TraitLine, &mut Text, &mut Node)>,
    )>,
    mut slots: Query<(&PoolSlot, &mut Node, &Children, &mut super::hud::TipFor), Without<TraitLine>>,
    mut icons: Query<&mut super::hud::SpellIcon>,
    gear: Res<super::inventory::Equipment>,
    coop: Res<super::coop::Coop>,
    mut shown: Local<Option<(Option<usize>, Option<u8>, bool, usize, bool, u64)>>,
) {
    let race = shown_race(&selection, &buttons);
    let key = (selection.class, race, spells.is_some(), slots.iter().len(), race.is_some_and(|race| selection.race_unlocked(race)), gear.revision);
    if *shown == Some(key) {
        return;
    }
    *shown = Some(key);
    let class = selection.class_def();
    let (hp, reduction) = class.map_or((0., 0.), |c| super::class_base(c.id));
    let score = class
        .and_then(|c| gear.view(&gear.realm(&coop), c.id))
        .map_or(0, |hero| hero.inventory.gear_score(&gear.catalog, hero.inventory.class));
    for (stat, mut text) in &mut texts.p0() {
        text.0 = match (stat, class) {
            (_, None) => String::new(),
            (PlateStat::Health, _) => format!("{hp:.0}"),
            (PlateStat::Reduction, _) => format!("{:.0}%", reduction * 100.),
            (PlateStat::Start, Some(c)) => {
                super::hud::spell_name(spells.as_deref(), data::ability(c.start).spell, c.start)
            }
            (PlateStat::GearScore, _) => score.to_string(),
        };
    }
    let name = race.map_or(String::new(), |race| {
        let name = super::races::race_name(race);
        if selection.race_unlocked(race) { name.into() }
        else if let Some(zone) = super::races::unlock_zone(race) { format!("{name}  ·  Locked: defeat the final boss in {zone}") }
        else { format!("{name}  ·  Locked: future expansion") }
    });
    for mut text in &mut texts.p1() {
        text.0 = name.clone();
    }
    let traits = race.map(super::races::trait_lines).unwrap_or_default();
    for (line, mut text, mut node) in &mut texts.p2() {
        let value = traits.get(line.0);
        text.0 = value.map_or_else(String::new, |t| format!("•  {t}"));
        node.display = if value.is_some() { Display::Flex } else { Display::None };
        // A proc's sentence takes the row; the short bonuses pair up.
        let wide = value.is_some_and(|t| t.chars().count() > 38);
        node.width = Val::Percent(if wide { 100. } else { 50. });
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

/// The detail line of the play button, naming the selected arena.
#[derive(Component)]
pub(super) struct PlayTarget;

/// The adventure panel's selected zone: its name, and where it is and whether it is cleared.
#[derive(Component)]
pub(super) struct ZoneName;
#[derive(Component)]
pub(super) struct ZoneLine;

/// FEATURES over YOUR ADVENTURE, stacked by flexbox so neither can run into the other.
fn right_column(ui: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    ui.spawn(Node {
        position_type: PositionType::Absolute,
        right: Val::Px(MARGIN),
        top: Val::Px(HEADER + 16.),
        bottom: Val::Px(PANELS_BOTTOM),
        width: Val::Px(ADVENTURE_WIDTH),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(gap::M),
        ..default()
    })
    .with_children(|col| {
        super::features_ui::build(
            col,
            f,
            Node {
                width: Val::Percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        );
        adventure(col, f);
    });
}

fn adventure(col: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    col.spawn(skin::panel(Node {
        width: Val::Percent(100.),
        padding: UiRect::all(Val::Px(gap::L)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(gap::S),
        flex_shrink: 0.,
        ..default()
    }))
    .with_children(|p| {
        skin::heading(p, f, "YOUR ADVENTURE");
        p.spawn((
            Node {
                width: Val::Percent(100.),
                padding: UiRect::axes(Val::Px(gap::M), Val::Px(gap::S)),
                border: UiRect::left(Val::Px(3.)),
                border_radius: BorderRadius::all(Val::Px(4.)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.),
                ..default()
            },
            BackgroundColor(skin::SURFACE),
            BorderColor::all(skin::GOLD),
        ))
        .with_children(|card| {
            let name = skin::label(card, f, "", size::HEADING, skin::GOLD);
            card.commands().entity(name).insert(ZoneName);
            let line = skin::label(card, f, "", size::SMALL, skin::MUTED);
            card.commands().entity(line).insert(ZoneLine);
        });
        skin::button(
            p,
            f,
            "Choose zone",
            "",
            super::adventure_atlas::Action::Open(0),
            false,
            false,
            true,
        );
        p.spawn(Node {
            width: Val::Percent(100.),
            column_gap: Val::Px(gap::S),
            ..default()
        })
        .with_children(|row| {
            for (title, tab) in [("Dungeons", 1), ("Raids", 2)] {
                row.spawn(Node {
                    flex_grow: 1.,
                    flex_basis: Val::Px(0.),
                    ..default()
                })
                .with_children(|p| {
                    skin::button(
                        p,
                        f,
                        title,
                        "",
                        super::adventure_atlas::Action::Open(tab),
                        false,
                        false,
                        true,
                    );
                });
            }
        });
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
        column_gap: Val::Px(gap::M),
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
                padding: UiRect::axes(Val::Px(14.), Val::Px(gap::S)),
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
            skin::label(p, f, "Play solo", size::HEADING + 2., skin::GOLD);
            let target = skin::label(p, f, "", size::SMALL, skin::TEXT);
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
        bar.spawn(Node {
            width: Val::Px(200.),
            ..default()
        })
        .with_children(|p| {
            skin::button(
                p,
                f,
                "Test Grounds",
                "Spawn anything; nothing is saved",
                MenuAction::TestGrounds,
                false,
                false,
                true,
            );
        });
    });
}

#[allow(clippy::type_complexity)]
pub(super) fn refresh_adventure(
    selection: Res<super::Selection>,
    buttons: Query<(&MenuAction, &Interaction)>,
    mut labels: ParamSet<(
        Query<&mut Text, With<PlayTarget>>,
        Query<&mut Text, With<RaceCaption>>,
        Query<&mut Text, With<ZoneName>>,
        Query<(&mut Text, &mut TextColor), With<ZoneLine>>,
    )>,
) {
    let race = shown_race(&selection, &buttons);
    let name = race.map_or(String::new(), |race| {
        let name = super::races::race_name(race);
        if selection.race_unlocked(race) { name.into() }
        else if let Some(zone) = super::races::unlock_zone(race) { format!("{name}  ·  clear {zone}") }
        else { format!("{name}  ·  future race") }
    });
    for mut text in &mut labels.p1() {
        if text.0 != name {
            text.0 = name.clone();
        }
    }
    let map = selection.map_def();
    let target = format!("Enter {}", map.name);
    for mut text in &mut labels.p0() {
        if text.0 != target {
            text.0.clone_from(&target);
        }
    }
    for mut text in &mut labels.p2() {
        if text.0 != map.name {
            text.0 = map.name.into();
        }
    }
    let cleared = selection.cleared.contains(map.name);
    let line = format!("{}  ·  {}", map.zone, if cleared { "Cleared" } else { "Not yet cleared" });
    for (mut text, mut color) in &mut labels.p3() {
        if text.0 != line {
            text.0.clone_from(&line);
            color.0 = if cleared { skin::GREEN } else { skin::MUTED };
        }
    }
}
