//! Test Grounds: an Elwynn run where nothing spawns by itself and nothing earned is saved, with a
//! panel to call in any mob, silver, elite or boss, make the hero or the enemies unkillable, and
//! hand out levels, upgrades and gold. F8 hides and shows the panel.

use bevy::prelude::*;

use super::abilities::Hit;
use super::enemies::{self, Enemy};
use super::front_end as skin;
use super::hero::Hero;
use super::hud::HudFont;
use super::right_column as col;
use super::{data, Phase, Run, RunEntity, Selection};

/// The panel's root.
#[derive(Component)]
struct Panel;

/// A tab's page; only the open tab's page shows.
#[derive(Component)]
struct Page(u8);

/// The open tab, and whether the panel is folded to its header.
#[derive(Component, Default)]
struct View {
    tab: u8,
    folded: bool,
}

/// What a panel button does.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum TestAction {
    /// A horde row; Shift spawns five.
    Mob(usize),
    /// A silver from a random melee horde row.
    Silver,
    /// A lair elite, asleep where it lands until the hero comes near.
    Elite(usize),
    Boss(usize),
    God,
    Immortal,
    Levels(u32),
    Upgrade,
    Gold,
    Heal,
    KillAll,
    Clear,
    Tab(u8),
    Fold,
}

/// A toggle's On/Off text.
#[derive(Component)]
struct ToggleLabel(TestAction);

const WIDTH: f32 = 292.;
/// A button in the two-column grid.
const CELL: f32 = 134.;
// The HUD's frame and palette ([`super::right_column`]): steel edges, crimson for what is on.
const IDLE: Color = col::KNOB;
const HOVER: Color = Color::srgb(0.114, 0.145, 0.188);
const EDGE: Color = col::STEEL_LO;
const SILVER: Color = Color::srgb(0.72, 0.76, 0.84);
const ELITE: Color = Color::srgb(0.78, 0.6, 0.24);
const TABS: [&str; 2] = ["Spawn", "Hero"];

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (build, clicks, paint, hide_key).run_if(in_state(Phase::Playing)),
    );
}

fn text(font: &Handle<Font>, value: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
    )
}

/// A small-caps heading with a hint on the right and a rule under both.
fn section(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, title: &str, hint: &str) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.),
                margin: UiRect::top(Val::Px(4.)),
                padding: UiRect::bottom(Val::Px(2.)),
                border: UiRect::bottom(Val::Px(1.)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            BorderColor::all(col::STEEL_LO.with_alpha(0.5)),
        ))
        .with_children(|row| {
            row.spawn(text(font, &title.to_uppercase(), 10., col::STEEL));
            row.spawn(text(font, hint, 9., col::TEXT_DIM));
        });
}

/// One button; `accent` frames it by what it spawns.
fn button(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    action: TestAction,
    accent: Color,
    width: f32,
) {
    parent
        .spawn((
            Button,
            action,
            Node {
                width: Val::Px(width),
                height: Val::Px(24.),
                padding: UiRect::horizontal(Val::Px(6.)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                border: UiRect::all(Val::Px(1.)),
                border_radius: BorderRadius::all(Val::Px(1.)),
                ..default()
            },
            BackgroundColor(IDLE),
            BorderColor::all(accent),
            Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
        ))
        .with_children(|b| {
            let mut label = b.spawn((
                text(font, label, 11., col::TEXT_HI),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
            ));
            if matches!(action, TestAction::God | TestAction::Immortal) {
                label.insert(ToggleLabel(action));
            }
        });
}

/// A wrapped two-column grid of buttons.
fn grid(parent: &mut ChildSpawnerCommands, fill: impl FnOnce(&mut ChildSpawnerCommands)) {
    parent
        .spawn(Node {
            width: Val::Percent(100.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(6.),
            row_gap: Val::Px(4.),
            ..default()
        })
        .with_children(fill);
}

/// A tab's page.
fn page(parent: &mut ChildSpawnerCommands, tab: u8, fill: impl FnOnce(&mut ChildSpawnerCommands)) {
    parent
        .spawn((
            Page(tab),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.),
                display: if tab == 0 {
                    Display::Flex
                } else {
                    Display::None
                },
                ..default()
            },
        ))
        .with_children(fill);
}

/// Raise the panel once a test run is under way: on the left, under the unit frames, where the
/// minimap and the damage meter leave room.
fn build(
    mut commands: Commands,
    run: Res<Run>,
    selection: Res<Selection>,
    font: Option<Res<HudFont>>,
    panels: Query<(), With<Panel>>,
) {
    if !run.test || !panels.is_empty() {
        return;
    }
    let Some(font) = font else { return };
    let f = &font.0;
    let roster = data::roster(selection.map);
    commands
        .spawn((
            Panel,
            View::default(),
            RunEntity,
            super::hud_layout::Movable::window("test_grounds", "Test Grounds"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.),
                top: Val::Px(210.),
                width: Val::Px(WIDTH),
                padding: UiRect::all(Val::Px(8.)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.),
                border: UiRect::all(Val::Px(1.)),
                border_radius: BorderRadius::all(Val::Px(2.)),
                ..default()
            },
            BackgroundColor(col::INK),
            BorderColor::all(EDGE),
            Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
            BoxShadow::new(
                Color::srgba(0., 0., 0., 0.55),
                Val::ZERO,
                Val::Px(3.),
                Val::ZERO,
                Val::Px(10.),
            ),
            GlobalZIndex(900),
        ))
        .with_children(|p| {
            // The header: title, the save warning and the fold button.
            p.spawn(Node {
                width: Val::Percent(100.),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|h| {
                h.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|t| {
                    t.spawn(text(f, "Test Grounds", 15., col::TEXT_HI));
                    t.spawn(text(f, "Nothing is saved  ·  F8 hides", 9., col::TEXT_DIM));
                });
                button(h, f, "-", TestAction::Fold, EDGE, 26.);
            });
            p.spawn(Node {
                width: Val::Percent(100.),
                column_gap: Val::Px(6.),
                ..default()
            })
            .with_children(|row| {
                for (i, name) in TABS.iter().enumerate() {
                    button(row, f, name, TestAction::Tab(i as u8), EDGE, CELL);
                }
            });
            page(p, 0, |pg| {
                section(pg, f, "Mobs", "Shift-click: x5");
                grid(pg, |g| {
                    for (i, def) in roster.enemies.iter().enumerate() {
                        button(g, f, def.name, TestAction::Mob(i), EDGE, CELL);
                    }
                    button(g, f, "Silver (random)", TestAction::Silver, SILVER, CELL);
                });
                section(pg, f, "Elites", "sleep until pulled");
                grid(pg, |g| {
                    for (i, def) in roster.guardians.iter().enumerate() {
                        button(g, f, def.name, TestAction::Elite(i), ELITE, CELL);
                    }
                });
                section(pg, f, "Bosses", "");
                grid(pg, |g| {
                    for (i, def) in roster.bosses.iter().enumerate() {
                        button(g, f, def.name, TestAction::Boss(i), col::WINE_HI, CELL);
                    }
                });
            });
            page(p, 1, |pg| {
                section(pg, f, "Toggles", "");
                grid(pg, |g| {
                    button(g, f, "God mode", TestAction::God, EDGE, CELL);
                    button(g, f, "Immortal enemies", TestAction::Immortal, EDGE, CELL);
                });
                section(pg, f, "Hero", "");
                grid(pg, |g| {
                    button(g, f, "+1 level", TestAction::Levels(1), EDGE, CELL);
                    button(g, f, "+5 levels", TestAction::Levels(5), EDGE, CELL);
                    button(g, f, "Upgrade (chest)", TestAction::Upgrade, EDGE, CELL);
                    button(g, f, "+1000 gold", TestAction::Gold, EDGE, CELL);
                    button(g, f, "Full heal", TestAction::Heal, skin::GREEN, CELL);
                });
                section(pg, f, "Enemies", "");
                grid(pg, |g| {
                    button(g, f, "Kill all", TestAction::KillAll, col::WINE_HI, CELL);
                    button(g, f, "Clear all", TestAction::Clear, EDGE, CELL);
                });
            });
        });
}

/// A spot on the ground `distance` yards from the hero, in a random direction.
fn near(hero: Vec3, distance: f32, rng: &mut super::Rng, ground: &super::dungeon::Ground) -> Vec3 {
    let a = rng.range(0.0, std::f32::consts::TAU);
    ground.spawn_spot(hero + Vec3::new(a.cos(), 0.0, a.sin()) * distance, hero.y)
}

#[allow(clippy::too_many_arguments)]
fn clicks(
    mut commands: Commands,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    mut next: ResMut<NextState<Phase>>,
    mut rng: ResMut<super::Rng>,
    ground: super::dungeon::Ground,
    keys: Res<ButtonInput<KeyCode>>,
    hero: Query<&Transform, With<Hero>>,
    enemies: Query<(Entity, &Enemy)>,
    mut hits: MessageWriter<Hit>,
    mut buttons: Query<(&Interaction, &TestAction, &mut BackgroundColor), Changed<Interaction>>,
    mut views: Query<&mut View>,
) {
    if !run.test {
        return;
    }
    let Ok(htf) = hero.single() else { return };
    let hpos = htf.translation;
    let roster = data::roster(selection.map);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    for (interaction, action, mut bg) in &mut buttons {
        bg.0 = if *interaction == Interaction::None {
            IDLE
        } else {
            HOVER
        };
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            TestAction::Tab(tab) => {
                for mut view in &mut views {
                    view.tab = tab;
                    view.folded = false;
                }
            }
            TestAction::Fold => {
                for mut view in &mut views {
                    view.folded = !view.folded;
                }
            }
            TestAction::Mob(i) => {
                let def = &roster.enemies[i];
                for _ in 0..if shift { 5 } else { 1 } {
                    let at = near(hpos, 14.0, &mut rng, &ground);
                    enemies::spawn_enemy(&mut commands, &mut run, def, at, hpos, false);
                }
            }
            TestAction::Silver => {
                let melee: Vec<_> = roster
                    .enemies
                    .iter()
                    .filter(|d| d.ranged.is_none())
                    .collect();
                if !melee.is_empty() {
                    let def = melee[rng.index(melee.len())];
                    let at = near(hpos, 16.0, &mut rng, &ground);
                    enemies::spawn_silver(&mut commands, &mut run, def, at, hpos);
                    run.banner = Some((format!("A silver {} approaches!", def.name), 3.0));
                }
            }
            TestAction::Elite(i) => {
                let at = near(hpos, 20.0, &mut rng, &ground);
                enemies::spawn_guardian(&mut commands, &mut run, &roster.guardians[i], at, hpos, 0);
            }
            TestAction::Boss(i) => {
                let def = &roster.bosses[i];
                let at = near(hpos, 24.0, &mut rng, &ground);
                enemies::spawn_enemy(&mut commands, &mut run, def, at, hpos, true);
                run.banner = Some(if def.intro.is_empty() {
                    (format!("{} approaches!", def.name), 4.0)
                } else {
                    (def.intro.to_string(), 4.0)
                });
            }
            TestAction::God => run.test_god = !run.test_god,
            TestAction::Immortal => run.test_immortal = !run.test_immortal,
            TestAction::Levels(n) => {
                let to = (run.level + n).min(data::MAX_LEVEL);
                run.pending_levels += to - run.level;
                run.level = to;
                if run.pending_levels > 0 {
                    next.set(Phase::LevelUp);
                }
            }
            TestAction::Upgrade => {
                run.pending_chests += 1;
                next.set(Phase::LevelUp);
            }
            TestAction::Gold => run.gold = run.gold.saturating_add(1000),
            TestAction::Heal => run.hp = run.stats().max_hp,
            TestAction::KillAll => {
                // Through the normal hit path, so loot, chests and death scripts all run.
                for (e, en) in &enemies {
                    if !en.dead {
                        hits.write(Hit::direct(e, en.hp + en.max_hp, false));
                    }
                }
            }
            TestAction::Clear => {
                for (e, _) in &enemies {
                    commands.entity(e).despawn();
                }
            }
        }
    }
}

/// The toggles' On/Off, the open page, and the frames of the open tab and the toggles that are on.
#[allow(clippy::type_complexity)]
fn paint(
    run: Res<Run>,
    views: Query<&View>,
    mut labels: Query<(&ToggleLabel, &mut Text, &mut TextColor)>,
    mut pages: Query<(&Page, &mut Node)>,
    mut frames: Query<(&TestAction, &mut BorderColor)>,
) {
    let Ok(view) = views.single() else { return };
    for (label, mut text, mut color) in &mut labels {
        let (name, on) = match label.0 {
            TestAction::God => ("God mode", run.test_god),
            _ => ("Immortal enemies", run.test_immortal),
        };
        let want = format!("{name}: {}", if on { "On" } else { "Off" });
        if text.0 != want {
            text.0 = want;
            color.0 = if on { col::CRIMSON } else { col::TEXT_DIM };
        }
    }
    for (page, mut node) in &mut pages {
        let want = if !view.folded && page.0 == view.tab {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != want {
            node.display = want;
        }
    }
    for (action, mut border) in &mut frames {
        let lit = |on: bool| BorderColor::all(if on { col::WINE_HI } else { EDGE });
        let want = match action {
            TestAction::Tab(t) => lit(*t == view.tab && !view.folded),
            TestAction::God => lit(run.test_god),
            TestAction::Immortal => lit(run.test_immortal),
            _ => continue,
        };
        if *border != want {
            *border = want;
        }
    }
}

fn hide_key(keys: Res<ButtonInput<KeyCode>>, mut panels: Query<&mut Visibility, With<Panel>>) {
    if !keys.just_pressed(KeyCode::F8) {
        return;
    }
    for mut v in &mut panels {
        *v = if *v == Visibility::Hidden {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
