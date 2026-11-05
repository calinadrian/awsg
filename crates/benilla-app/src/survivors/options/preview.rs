//! The settings page's preview strip: a patch of ground under the tabs with the open tab's pieces
//! drawn from the live settings, so a change shows before the run does. Only the unit frames' and
//! the nameplates' tabs have one; sizes and sound have nothing to draw.
use super::super::hud::{HudFont, SpellIcon};
use super::super::plates::{NameplateSettings, PLATE_HEALTH_TEXT};
use super::super::right_column as col;
use super::super::unit_frames::{health_height, AuraLayout, AuraPos, HealthText, HEALTH_TEXT};
use super::Options;
use bevy::prelude::*;

/// The tabs with a preview, as the (topic, tab) pairs of [`super::TOPICS`].
const UNIT_FRAMES: (u8, u8) = (3, 0);
const FRAME_AURAS: (u8, u8) = (3, 1);
const PLATES: (u8, u8) = (4, 0);
const PLATE_AURAS: (u8, u8) = (4, 1);
const NAMES: (u8, u8) = (4, 2);
/// The tabs whose samples wear auras, laid out again when the Auras tabs change.
const WORN: [(u8, u8); 4] = [UNIT_FRAMES, FRAME_AURAS, PLATES, PLATE_AURAS];

/// Sample auras' spells: Fortitude, Mark of the Wild and Battle Shout; Shadow Word: Pain,
/// Corruption, Curse of Agony and Immolate.
const BUFF_ICONS: [u32; 3] = [1243, 1126, 6673];
const DEBUFF_ICONS: [u32; 4] = [589, 172, 980, 348];

/// The world's grass under the samples, so they read as they do in a run.
const GROUND: Color = Color::srgb(0.173, 0.227, 0.086);
const VEIL: Color = Color::srgba(0.173, 0.227, 0.086, 0.75);
const CAPTION: Color = Color::srgba(1., 1., 1., 0.45);
const NAME_BLUE: Color = Color::srgb(0.25, 0.55, 1.);

#[derive(Component)]
pub(super) struct Strip;
/// One tab's samples.
#[derive(Component)]
pub(super) struct Group((u8, u8));
/// A sample frame's health well, as tall as its setting.
#[derive(Component)]
pub(super) struct HealthWell;
/// A sample health text: a nameplate's or a unit frame's format, over these numbers.
#[derive(Component)]
pub(super) struct Health {
    plate: bool,
    cur: f32,
    max: f32,
}
/// A sample plate in one of the two nameplate styles.
#[derive(Component)]
pub(super) struct Style {
    compact: bool,
}
/// Fades a sample whose switch is off: it stays in view, so the switch shows what it hides.
#[derive(Component)]
pub(super) enum Veil {
    Enemies,
    Friends,
    Cvar(&'static str),
}
/// The hero's bar, or the line that stands in for it when it is off.
#[derive(Component)]
pub(super) struct HeroBar(bool);
#[derive(Component)]
pub(super) struct GuildLine;

fn label(f: &Handle<Font>, value: &str, size: f32, color: Color) -> impl Bundle {
    col::label(f, value, size, color)
}

fn rgb([r, g, b, _]: [f32; 4], k: f32) -> Color {
    Color::srgb(r * k, g * k, b * k)
}

fn span(width: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.),
        top: Val::Px(0.),
        width: Val::Percent(width),
        height: Val::Percent(100.),
        ..default()
    }
}

fn veil(which: Veil) -> impl Bundle {
    (
        which,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(-4.),
            right: Val::Px(-4.),
            top: Val::Px(-4.),
            bottom: Val::Px(-4.),
            display: Display::None,
            ..default()
        },
        BackgroundColor(VEIL),
        ZIndex(1),
    )
}

/// The strip, hidden until a tab with a preview is open.
pub(super) fn spawn(page: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    page.spawn((
        Strip,
        Node {
            height: Val::Px(104.),
            flex_shrink: 0.,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(GROUND),
        BorderColor::all(Color::BLACK),
        Outline::new(Val::Px(1.), Val::ZERO, super::EDGE_DIM),
    ))
    .with_children(|strip| {
        strip.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.),
                top: Val::Px(6.),
                ..default()
            },
            label(f, "PREVIEW", 9., CAPTION),
        ));
        let mut group = |page: (u8, u8), fill: &dyn Fn(&mut ChildSpawnerCommands)| {
            strip
                .spawn((
                    Group(page),
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(22.),
                        display: Display::None,
                        ..default()
                    },
                ))
                .with_children(|g| fill(g));
        };
        for page in WORN {
            group(page, &|_| {});
        }
        group(NAMES, &|g| names(g, f));
    });
}

/// The samples of a tab that wears auras, in the current layouts.
fn fill(
    g: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    page: (u8, u8),
    frames: AuraLayout,
    plates: AuraLayout,
) {
    if page == UNIT_FRAMES || page == FRAME_AURAS {
        frame(g, f, false, frames);
        frame(g, f, true, frames);
    } else {
        plate(g, f, false, plates);
        plate(g, f, true, plates);
        if page == PLATES {
            hero(g, f);
        }
    }
}

/// An aura's icon in its kind's edge, grey for a buff and crimson for a debuff.
fn aura(p: &mut ChildSpawnerCommands, spell: u32, size: f32, debuff: bool) {
    p.spawn((
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.,
            border: UiRect::all(Val::Px(1.)),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        BorderColor::all(if debuff { col::WINE_HI } else { col::STEEL_LO }),
        Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
    ))
    .with_child((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        },
        ImageNode {
            color: Color::NONE,
            ..default()
        },
        SpellIcon(spell),
    ));
}

/// A row of `count` sample auras of one kind, from `reverse`'s end.
fn auras(
    p: &mut ChildSpawnerCommands,
    count: usize,
    size: f32,
    debuff: bool,
    gap: f32,
    reverse: bool,
) {
    let icons: &[u32] = if debuff { &DEBUFF_ICONS } else { &BUFF_ICONS };
    p.spawn(Node {
        flex_direction: if reverse {
            FlexDirection::RowReverse
        } else {
            FlexDirection::Row
        },
        column_gap: Val::Px(gap),
        flex_shrink: 0.,
        ..default()
    })
    .with_children(|row| {
        for i in 0..count {
            aura(row, icons[i % icons.len()], size, debuff);
        }
    });
}

/// A frame's rows on one edge, outermost first: on a shared edge the debuffs sit nearest the frame.
fn frame_edge(p: &mut ChildSpawnerCommands, layout: AuraLayout, at: AuraPos, target: bool) {
    let (buffs, debuffs) = if target { (1, 4) } else { (3, 1) };
    let mut kinds = vec![
        (false, buffs, layout.buffs, layout.buff_size),
        (true, debuffs, layout.debuffs, layout.debuff_size),
    ];
    if at == AuraPos::Below {
        kinds.reverse();
    }
    for (debuff, count, pos, size) in kinds {
        if pos == at {
            auras(p, count, size, debuff, 2., target);
        }
    }
}

/// A unit frame as `unit_frames::classic` draws it at 100%, with a sample unit in it and its
/// auras on the edges the layout names.
fn frame(p: &mut ChildSpawnerCommands, f: &Handle<Font>, target: bool, layout: AuraLayout) {
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        align_items: if target {
            AlignItems::FlexEnd
        } else {
            AlignItems::FlexStart
        },
        row_gap: Val::Px(4.),
        ..default()
    })
    .with_children(|worn| {
        frame_edge(worn, layout, AuraPos::Above, target);
        frame_bar(worn, f, target);
        frame_edge(worn, layout, AuraPos::Below, target);
    });
}

fn frame_bar(p: &mut ChildSpawnerCommands, f: &Handle<Font>, target: bool) {
    let (name, cur, max) = if target {
        ("Kobold Vermin", 132., 176.)
    } else {
        ("Warrior", 230., 257.)
    };
    let (fill, fill_hi, well) = if target {
        (col::WINE, col::WINE_HI, col::WELL)
    } else {
        (
            Color::srgb(0.18, 0.55, 0.2),
            Color::srgb(0.36, 0.74, 0.36),
            Color::srgb(0.027, 0.07, 0.04),
        )
    };
    let lined = || Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK);
    p.spawn(col::plate(Node {
        width: Val::Px(170.),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.),
        padding: UiRect::all(Val::Px(2.)),
        ..default()
    }))
    .with_children(|plate| {
        plate
            .spawn((
                HealthWell,
                Node {
                    height: Val::Px(13.),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(well),
                lined(),
            ))
            .with_children(|w| {
                w.spawn((span(cur / max * 100.), BackgroundColor(fill)))
                    .with_child((
                        Node {
                            width: Val::Percent(100.),
                            height: Val::Px(1.),
                            ..default()
                        },
                        BackgroundColor(fill_hi),
                    ));
                w.spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(5.)),
                    column_gap: Val::Px(6.),
                    ..span(100.)
                })
                .with_children(|t| {
                    t.spawn(Node {
                        align_items: AlignItems::Center,
                        // A long name gives way, clipped, so the health text stays whole.
                        flex_shrink: 1.,
                        min_width: Val::Px(0.),
                        overflow: Overflow::clip(),
                        column_gap: Val::Px(4.),
                        ..default()
                    })
                    .with_children(|who| {
                        who.spawn(label(f, "6", 8., col::STEEL));
                        who.spawn(label(f, name, 10., col::TEXT_HI));
                    });
                    t.spawn((
                        Health {
                            plate: false,
                            cur,
                            max,
                        },
                        Node {
                            flex_shrink: 0.,
                            ..default()
                        },
                        label(f, "", 9., col::TEXT_HI),
                    ));
                });
            });
        plate
            .spawn((
                Node {
                    height: Val::Px(4.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.02, 0.024, 0.043)),
                lined(),
            ))
            .with_child((
                span(if target { 0. } else { 60. }),
                BackgroundColor(Color::srgb(0.70, 0.14, 0.10)),
            ));
    });
}

/// A compact plate's row on one edge: the debuffs from the left, the buffs from the right.
fn plate_edge(p: &mut ChildSpawnerCommands, layout: AuraLayout, at: AuraPos, friend: bool) {
    let (debuffs, buffs) = if friend { (0, 2) } else { (3, 1) };
    let here = |pos: AuraPos, count: usize| pos == at && count > 0;
    if !here(layout.debuffs, debuffs) && !here(layout.buffs, buffs) {
        return;
    }
    p.spawn(Node {
        width: Val::Px(100.),
        justify_content: JustifyContent::SpaceBetween,
        align_items: if at == AuraPos::Above {
            AlignItems::FlexEnd
        } else {
            AlignItems::FlexStart
        },
        ..default()
    })
    .with_children(|row| {
        if here(layout.debuffs, debuffs) {
            auras(row, debuffs, layout.debuff_size, true, 1., false);
        } else {
            row.spawn(Node::default());
        }
        if here(layout.buffs, buffs) {
            auras(row, buffs, layout.buff_size, false, 1., true);
        }
    });
}

/// An enemy's or an ally's plate in both styles, the setting's one shown; the compact one wears
/// its auras, as it does in a run.
fn plate(p: &mut ChildSpawnerCommands, f: &Handle<Font>, friend: bool, layout: AuraLayout) {
    let (name, cur, max) = if friend {
        ("Thrall", 410., 520.)
    } else {
        ("Kobold Miner", 98., 152.)
    };
    let fill = if friend {
        rgb(crate::vplates::PLATE_FRIENDLY, 0.6)
    } else {
        col::WINE
    };
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|seat| {
        // The compact plate: as `plates::spawn_compact_plate`, the name giving way to the health.
        seat.spawn((
            Style { compact: true },
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(2.),
                ..default()
            },
        ))
        .with_children(|worn| {
            plate_edge(worn, layout, AuraPos::Above, friend);
            worn.spawn((
                Node {
                    width: Val::Px(100.),
                    height: Val::Px(11.),
                    border: UiRect::all(Val::Px(1.)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(col::WELL),
                BorderColor::all(col::STEEL_LO),
                Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
            ))
            .with_children(|bar| {
                bar.spawn((span(cur / max * 100.), BackgroundColor(fill)));
                bar.spawn(Node {
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(2.)),
                    column_gap: Val::Px(2.),
                    ..span(100.)
                })
                .with_children(|words| {
                    words.spawn(label(f, "5", 5.5, col::STEEL));
                    words
                        .spawn(Node {
                            flex_grow: 1.,
                            flex_shrink: 1.,
                            min_width: Val::Px(0.),
                            align_items: AlignItems::Center,
                            overflow: Overflow::clip(),
                            ..default()
                        })
                        .with_child(label(f, name, 7., col::TEXT_HI));
                    words.spawn((
                        Health {
                            plate: true,
                            cur,
                            max,
                        },
                        label(f, "", 5.5, col::TEXT_HI),
                    ));
                });
            });
            plate_edge(worn, layout, AuraPos::Below, friend);
        });
        // The client's plate: the name over a bar in its grey frame, the level past its end.
        seat.spawn((
            Style { compact: false },
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(2.),
                ..default()
            },
        ))
        .with_children(|c| {
            c.spawn(label(f, name, 10., Color::WHITE));
            c.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(5.),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Node {
                        width: Val::Px(96.),
                        height: Val::Px(8.),
                        border: UiRect::all(Val::Px(1.)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.13, 0., 0.)),
                    BorderColor::all(Color::srgb(0.54, 0.54, 0.54)),
                    Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
                ))
                .with_child((
                    span(cur / max * 100.),
                    BackgroundColor(if friend {
                        rgb(crate::vplates::PLATE_FRIENDLY, 0.8)
                    } else {
                        Color::srgb(0.88, 0., 0.)
                    }),
                ));
                row.spawn(label(f, "5", 9., rgb(crate::vplates::CON_YELLOW, 1.)));
            });
        });
        seat.spawn(veil(if friend { Veil::Friends } else { Veil::Enemies }));
    });
}

/// The hero's plate as `hud::hero_plate` draws it, or a line saying it is off.
fn hero(p: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    p.spawn(Node {
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|seat| {
        seat.spawn((
            HeroBar(true),
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.),
                ..default()
            },
        ))
        .with_children(|h| {
            h.spawn((
                Node {
                    width: Val::Px(84.),
                    height: Val::Px(8.),
                    border: UiRect::all(Val::Px(1.)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.035, 0.03, 0.85)),
                BorderColor::all(col::STEEL_LO),
                Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
            ))
            .with_child((
                span(89.),
                BackgroundColor(rgb(crate::vplates::PLATE_FRIENDLY, 0.8)),
            ));
            h.spawn(label(f, "6", 9., rgb(crate::vplates::CON_YELLOW, 1.)));
        });
        seat.spawn((
            HeroBar(false),
            Node {
                padding: UiRect::axes(Val::Px(8.), Val::Px(3.)),
                border: UiRect::all(Val::Px(1.)),
                display: Display::None,
                ..default()
            },
            BorderColor::all(CAPTION),
        ))
        .with_child(label(f, "No bar under your hero", 10., CAPTION));
    });
}

/// A player with a guild, a creature and the hero, each under its own switch.
fn names(p: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    let yellow = rgb(crate::vplates::CON_YELLOW, 0.85);
    let mut name = |key: &'static str, fill: &dyn Fn(&mut ChildSpawnerCommands)| {
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(1.),
            ..default()
        })
        .with_children(|n| {
            fill(n);
            n.spawn(veil(Veil::Cvar(key)));
        });
    };
    name("UnitNamePlayer", &|n| {
        n.spawn(label(f, "Jaina", 11., NAME_BLUE));
        n.spawn((
            GuildLine,
            Node::default(),
            label(f, "<Kirin Tor>", 11., NAME_BLUE),
        ));
    });
    name("UnitNameNPC", &|n| {
        n.spawn(label(f, "Marshal Dughan", 11., yellow));
    });
    name("UnitNameOwn", &|n| {
        n.spawn(label(f, "Warrior", 11., NAME_BLUE));
    });
}

/// Show the open tab's samples and dress them in the current settings.
#[allow(clippy::type_complexity)]
pub(super) fn paint(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    groups: Query<(Entity, &Group)>,
    mut worn: Local<Option<(AuraLayout, AuraLayout)>>,
    options: Res<Options>,
    cvars: Res<crate::cvars::Cvars>,
    nameplates: Res<NameplateSettings>,
    mut nodes: Query<
        (
            &mut Node,
            Has<Strip>,
            Option<&Group>,
            Has<HealthWell>,
            Option<&Style>,
            Option<&Veil>,
            Option<&HeroBar>,
            Has<GuildLine>,
        ),
        Or<(
            With<Strip>,
            With<Group>,
            With<HealthWell>,
            With<Style>,
            With<Veil>,
            With<HeroBar>,
            With<GuildLine>,
        )>,
    >,
    mut texts: Query<(&Health, &mut Text)>,
) {
    if !options.settings {
        return;
    }
    let layouts = (AuraLayout::frames(&cvars), nameplates.auras);
    if let Some(font) = font.filter(|_| *worn != Some(layouts)) {
        *worn = Some(layouts);
        for (group, page) in &groups {
            if WORN.contains(&page.0) {
                let page = page.0;
                commands
                    .entity(group)
                    .despawn_related::<Children>()
                    .with_children(|g| fill(g, &font.0, page, layouts.0, layouts.1));
            }
        }
    }
    let page = (options.topic, options.tab);
    let shown = options.search.trim().is_empty() && (WORN.contains(&page) || page == NAMES);
    let flag = |key: &str| cvars.flag(key).unwrap_or(false);
    let well = health_height(&cvars);
    for (mut node, strip, group, health_well, style, veil, hero, guild) in &mut nodes {
        let display = if strip {
            Some(shown)
        } else if let Some(group) = group {
            Some(group.0 == page)
        } else if health_well {
            let height = Val::Px(well);
            if node.height != height {
                node.height = height;
            }
            None
        } else if let Some(style) = style {
            Some(style.compact == nameplates.compact)
        } else if let Some(veil) = veil {
            Some(!match veil {
                Veil::Enemies => nameplates.enemies,
                Veil::Friends => nameplates.friends,
                Veil::Cvar(key) => flag(key),
            })
        } else if let Some(hero) = hero {
            Some(hero.0 == nameplates.hero)
        } else if guild {
            Some(flag("UnitNamePlayerGuild"))
        } else {
            None
        };
        if let Some(on) = display {
            let want = if on { Display::Flex } else { Display::None };
            if node.display != want {
                node.display = want;
            }
        }
    }
    let frames = HealthText::from_setting(cvars.get(HEALTH_TEXT), HealthText::All);
    let plates = HealthText::from_setting(cvars.get(PLATE_HEALTH_TEXT), HealthText::CurrentPercent);
    for (health, mut text) in &mut texts {
        let format = if health.plate { plates } else { frames };
        let want = format.format(health.cur, health.max);
        if text.0 != want {
            text.0 = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// The nameplates' tab shows its strip, the compact plates with the plates' own format, and
    /// fades the enemy plate while V has them off.
    #[test]
    fn the_plates_tab_previews_the_live_settings() {
        let mut world = World::new();
        world.insert_resource(super::super::super::hud::HudFont(Handle::default()));
        world.init_resource::<Assets<Image>>();
        world.run_system_once(super::super::spawn).unwrap();
        let (topic, tab) = PLATES;
        world.insert_resource(Options {
            open: true,
            settings: true,
            topic,
            tab,
            ..default()
        });
        world.insert_resource(crate::cvars::Cvars::with_value(HEALTH_TEXT, "4"));
        world.insert_resource(NameplateSettings {
            enemies: false,
            friends: true,
            compact: true,
            hero: true,
            auras: AuraLayout::default(),
        });
        // The first pass lays the samples out; the second dresses them. One system keeps its cache
        // of the layouts between the passes, as it does in a run.
        let paint = world.register_system(paint);
        world.run_system(paint).unwrap();
        world.run_system(paint).unwrap();
        let shown = |world: &mut World, pick: &dyn Fn(&Group) -> bool| {
            world
                .query::<(&Group, &Node)>()
                .iter(world)
                .filter(|(g, _)| pick(g))
                .all(|(_, n)| n.display == Display::Flex)
        };
        assert!(shown(&mut world, &|g| g.0 == PLATES));
        assert!(!shown(&mut world, &|g| g.0 == NAMES));
        let mut texts: Vec<(bool, String)> = world
            .query::<(&Health, &Text)>()
            .iter(&world)
            .map(|(h, t)| (h.plate, t.0.clone()))
            .collect();
        texts.sort();
        assert!(texts.contains(&(true, "98 · 65%".into())), "{texts:?}");
        assert!(texts.contains(&(false, "90%".into())), "{texts:?}");
        // Every enemy plate fades, on each tab that shows one, and no ally plate does.
        let veiled: Vec<(bool, bool)> = world
            .query::<(&Veil, &Node)>()
            .iter(&world)
            .filter_map(|(v, n)| match v {
                Veil::Enemies => Some((true, n.display == Display::Flex)),
                Veil::Friends => Some((false, n.display == Display::Flex)),
                Veil::Cvar(_) => None,
            })
            .collect();
        assert!(!veiled.is_empty());
        assert!(
            veiled.iter().all(|(enemy, faded)| enemy == faded),
            "only enemy plates fade"
        );
    }
}
