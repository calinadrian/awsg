//! Hero selection frames the live preview: a header with the account screens, the hero on the
//! left, the looks and the adventure on the right, and racial traits under the hero.
//!
//! Laid out on the 768-unit-tall canvas; every panel is bounded so 4:3 (1024 wide) still fits, and
//! panels in one column are stacked by flexbox, never by summed heights.
use super::{
    data,
    hud::{HudFont, MenuRoot},
    inventory::Pane,
    menu::{ClassSlot, MenuAction, MenuBuilt, PickName, RaceFace},
    menu_icons::icon_button,
    menu_skin::{self as skin, gap, size},
    races::{race_tc, RACES},
};
use crate::glue::{
    art::{class_tc, tc_rect, GlueArt},
    widgets::{GlueDisabled, LockHighlight},
};
use bevy::prelude::*;

/// Canvas edge to the side panels, and the gap between panels.
const MARGIN: f32 = 20.;
const HEADER: f32 = 64.;
const HERO_WIDTH: f32 = 284.;
const ADVENTURE_WIDTH: f32 = 258.;
/// Panels leave a small margin at the bottom of the canvas.
const PANELS_BOTTOM: f32 = 24.;

/// Reserve enough space for the racial traits and class progression.
const PLATE_HEIGHT: f32 = 240.;

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
    commands
        .entity(root)
        .insert((MenuBuilt, BackgroundColor(skin::INK)))
        .with_children(|ui| {
            header(ui, f);
            tower(ui, &art, f);
            right_column(ui, f);
            ui.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MARGIN * 2. + HERO_WIDTH),
                right: Val::Px(MARGIN * 2. + ADVENTURE_WIDTH),
                top: Val::Px(HEADER + 16.),
                bottom: Val::Px(PANELS_BOTTOM),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(gap::S),
                ..default()
            })
            .with_children(|center| {
                super::menu_preview::build(center);
                pick_plate(center, f);
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
        BackgroundColor(skin::INK),
        BorderColor::all(skin::EDGE),
    ))
    .with_children(|p| {
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.),
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, "AZEROTH SURVIVORS", size::HEADING, skin::TEXT);
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
            column_gap: Val::Px(gap::XS),
            row_gap: Val::Px(2.),
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::End,
            min_width: Val::Px(0.),
            max_width: Val::Percent(70.),
            ..default()
        })
        .with_children(|p| {
            use super::inventory_ui::Action;
            skin::tab(p, f, "Inventory", Action::Open(Pane::Inventory));
            skin::tab(p, f, "Items", Action::Open(Pane::Items));
            skin::tab(p, f, "Quests", Action::Open(Pane::Quests));
            skin::tab(p, f, "Achievements", Action::Open(Pane::Achievements));
            skin::tab(p, f, "Auction House", Action::Open(Pane::Auction));
            skin::tab(p, f, "Test Grounds", MenuAction::TestGrounds);
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

#[derive(Component)]
pub(super) struct RacePopup;
#[derive(Component)]
pub(super) struct ToggleRaces;
#[derive(Component)]
pub(super) struct ClassName;
#[derive(Component)]
pub(super) struct ClassDescription;

fn tower(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    ui.spawn(skin::panel(Node {
        position_type: PositionType::Absolute,
        left: Val::Px(MARGIN),
        top: Val::Px(HEADER + 16.),
        bottom: Val::Px(PANELS_BOTTOM),
        width: Val::Px(HERO_WIDTH),
        padding: UiRect::all(Val::Px(14.)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.),
        ..default()
    }))
    .with_children(|p| {
        let toggle = skin::button(p, f, "", "", ToggleRaces, false, false, true);
        p.commands().entity(toggle).with_children(|button| {
            let label = skin::label(button, f, "Choose race", size::SMALL, skin::TEXT);
            button.commands().entity(label).insert(RaceCaption);
            skin::label(button, f, "Race & gender / change", 9., skin::MUTED);
        });
        p.spawn((
            RacePopup,
            // The preview is a sibling stacking context; a local ZIndex cannot clear it.
            GlobalZIndex(415),
            skin::panel(Node {
                position_type: PositionType::Absolute,
                // Open beside the selector, keeping the class grid and its details visible.
                left: Val::Px(HERO_WIDTH + gap::S),
                width: Val::Px(HERO_WIDTH),
                top: Val::Px(0.),
                padding: UiRect::all(Val::Px(10.)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.),
                display: Display::None,
                ..default()
            }),
        ))
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
                    for (faction, name, color) in
                        [(0, "ALLIANCE", skin::ALLIANCE), (1, "HORDE", skin::HORDE)]
                    {
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
                            let e = skin::button(
                                p,
                                f,
                                label,
                                "",
                                MenuAction::Sex(sex),
                                false,
                                false,
                                true,
                            );
                            p.commands().entity(e).insert(LockHighlight::default());
                        });
                    }
                });
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
                            width: Val::Px(76.),
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
                            36. / 48.,
                        );
                        skin::label(p, f, class.name, size::SMALL, super::hud::rgb(class.color));
                    });
                }
            });
        });

        skin::rule(p);
        let name = skin::label(p, f, "", size::HEADING, skin::TEXT);
        p.commands().entity(name).insert(ClassName);
        let description = skin::label(p, f, "", size::SMALL, skin::MUTED);
        p.commands().entity(description).insert(ClassDescription);
        p.spawn(Node {
            width: Val::Percent(100.),
            column_gap: Val::Px(5.),
            ..default()
        })
        .with_children(|boxes| {
            for (caption, stat) in [
                ("HEALTH", PlateStat::Health),
                ("STARTS WITH", PlateStat::Start),
                ("GEAR", PlateStat::GearScore),
            ] {
                let value = skin::stat_tile(boxes, f, caption, "", size::BODY);
                boxes.commands().entity(value).insert(stat);
            }
        });
        skin::label(p, f, "SPELL POOL", size::CAPTION, skin::MUTED);
        pool(p);
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
fn pool(p: &mut ChildSpawnerCommands) {
    p.spawn(Node {
        width: Val::Percent(100.),
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
                            color: Color::NONE,
                            ..default()
                        },
                        super::hud::SpellIcon(0),
                    ));
                });
        }
    });
}
pub(super) fn refresh_class(
    clicks: Res<crate::glue::GlueClicks>,
    selection: Res<super::Selection>,
    toggle: Query<Entity, With<ToggleRaces>>,
    actions: Query<(Entity, &MenuAction)>,
    mut popup: Query<&mut Node, With<RacePopup>>,
    mut labels: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<ClassName>>,
        Query<&mut Text, With<ClassDescription>>,
    )>,
    mut open: Local<bool>,
) {
    if toggle.iter().any(|e| clicks.hit(e)) {
        *open = !*open;
    }
    if actions
        .iter()
        .any(|(e, a)| matches!(a, MenuAction::Sex(_) | MenuAction::Class(_)) && clicks.hit(e))
    {
        *open = false;
    }
    for mut node in &mut popup {
        node.display = if *open { Display::Flex } else { Display::None };
    }
    let class = selection.class_def();
    for (mut text, mut color) in &mut labels.p0() {
        text.0 = class.map_or("Choose a class", |c| c.name).into();
        color.0 = class.map_or(skin::TEXT, |c| super::hud::rgb(c.color));
    }
    for mut text in &mut labels.p1() {
        text.0 = class.map_or("", |c| match c.id {
            1 => "Steel and fury. Cleave through packs and hold your ground.",
            2 => "Holy ground and righteous hammers. Healing and divine protection keep you fighting.",
            3 => "Ranged shots and traps. Control the pack before it reaches you.",
            4 => "Quick blades and relentless strikes. Turn an opening into a deadly finish.",
            5 => "Holy light and shadow. Balance damage, healing and protective shields.",
            7 => "Lightning, elemental shocks and totems. Shape the battlefield.",
            8 => "Fire, frost and arcane. Control crowds and unleash area spells.",
            9 => "Curses, shadow and demonic power. Drain enemies as damage over time takes hold.",
            _ => "Nature magic and feral strikes. Blend damage, roots and restoration.",
        }).into();
    }
}

/// A value box on the pick plate.
#[derive(Component, Clone, Copy)]
pub(super) enum PlateStat {
    Health,
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

/// Under the hero: the pick's name, racial traits and class progression.
fn pick_plate(ui: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    ui.spawn(skin::panel(Node {
        width: Val::Percent(100.),
        flex_shrink: 0.,
        height: Val::Px(PLATE_HEIGHT),
        padding: UiRect::axes(Val::Px(18.), Val::Px(gap::M)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(gap::S),
        overflow: Overflow::clip(),
        ..default()
    }))
    .with_children(|p| {
        let title = skin::label(p, f, "", size::HEADING, skin::TEXT);
        p.commands().entity(title).insert((
            PickName,
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ));
        // Keep every racial trait on its own row, with a common bullet alignment.
        p.spawn(Node {
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(gap::XS),
            flex_grow: 1.,
            flex_basis: Val::Px(0.),
            min_height: Val::Px(0.),
            overflow: Overflow::clip(),
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
                            width: Val::Percent(100.),
                            padding: UiRect::right(Val::Px(gap::M)),
                            display: Display::None,
                            ..default()
                        },
                    ));
                }
            });
        });
        p.spawn(Node {
            width: Val::Percent(100.),
            justify_content: JustifyContent::FlexStart,
            flex_shrink: 0.,
            ..default()
        })
        .with_children(|row| {
            skin::tab(row, f, "Talents", MenuAction::Talents);
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
    mut slots: Query<
        (&PoolSlot, &mut Node, &Children, &mut super::hud::TipFor),
        Without<TraitLine>,
    >,
    mut icons: Query<&mut super::hud::SpellIcon>,
    gear: Res<super::inventory::Equipment>,
    coop: Res<super::coop::Coop>,
    mut shown: Local<Option<(Option<usize>, Option<u8>, bool, usize, bool, u64)>>,
) {
    let race = shown_race(&selection, &buttons);
    let key = (
        selection.class,
        race,
        spells.is_some(),
        slots.iter().len(),
        race.is_some_and(|race| selection.race_unlocked(race)),
        gear.revision,
    );
    if *shown == Some(key) {
        return;
    }
    *shown = Some(key);
    let class = selection.class_def();
    let (hp, _) = class.map_or((0., 0.), |c| super::class_base(c.id));
    let score = class
        .and_then(|c| gear.view(&gear.realm(&coop), c.id))
        .map_or(0, |hero| {
            hero.inventory
                .gear_score(&gear.catalog, hero.inventory.class)
        });
    for (stat, mut text) in &mut texts.p0() {
        text.0 = match (stat, class) {
            (_, None) => String::new(),
            (PlateStat::Health, _) => format!("{hp:.0}"),
            (PlateStat::Start, Some(c)) => {
                super::hud::spell_name(spells.as_deref(), data::ability(c.start).spell, c.start)
            }
            (PlateStat::GearScore, _) => score.to_string(),
        };
    }
    let name = race.map_or(String::new(), |race| {
        let name = super::races::race_name(race);
        if selection.race_unlocked(race) {
            name.into()
        } else if let Some(zone) = super::races::unlock_zone(race) {
            format!("{name}  ·  Locked: defeat the final boss in {zone}")
        } else {
            format!("{name}  ·  Locked: future expansion")
        }
    });
    for mut text in &mut texts.p1() {
        text.0 = name.clone();
    }
    let traits = race.map(super::races::trait_lines).unwrap_or_default();
    for (line, mut text, mut node) in &mut texts.p2() {
        let value = traits.get(line.0);
        text.0 = value.map_or_else(String::new, |t| format!("•  {t}"));
        node.display = if value.is_some() {
            Display::Flex
        } else {
            Display::None
        };
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
        skin::rule(p);
        let play = skin::button(p, f, "Play solo", "", MenuAction::Enter, true, false, true);
        p.commands().entity(play).insert(GlueDisabled(false));
        let target = skin::label(p, f, "", size::CAPTION, skin::MUTED);
        p.commands().entity(target).insert(PlayTarget);
        skin::button(
            p,
            f,
            "Co-op",
            "",
            super::coop_ui::OpenCoop,
            false,
            false,
            true,
        );
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
        if selection.race_unlocked(race) {
            name.into()
        } else if let Some(zone) = super::races::unlock_zone(race) {
            format!("{name}  ·  clear {zone}")
        } else {
            format!("{name}  ·  future race")
        }
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
    let line = format!(
        "{}  ·  {}",
        map.zone,
        if cleared {
            "Cleared"
        } else {
            "Not yet cleared"
        }
    );
    for (mut text, mut color) in &mut labels.p3() {
        if text.0 != line {
            text.0.clone_from(&line);
            color.0 = if cleared { skin::GREEN } else { skin::MUTED };
        }
    }
}
