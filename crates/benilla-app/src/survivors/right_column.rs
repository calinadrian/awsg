//! The column down the right edge, in the boss bars' steel and crimson: the square minimap with
//! the zone and the local time, the run tracker (the clock, the boss schedule and the run's
//! counts), the damage meter, then the quest tracker. Each panel is built at the size the
//! Interface settings give it, and rebuilt when that changes.

use bevy::prelude::*;

use super::damage_meter::RightStack;
use super::enemies::Loot;
use super::guardians::Lairs;
use super::hud::HudFont;
use super::{data, difficulty, Phase, Run, Selection};

pub(super) const STEEL: Color = Color::srgb(0.604, 0.639, 0.722);
pub(super) const STEEL_HI: Color = Color::srgb(0.831, 0.851, 0.894);
pub(super) const STEEL_LO: Color = Color::srgb(0.365, 0.4, 0.475);
pub(super) const INK: Color = Color::srgba(0.051, 0.082, 0.09, 0.92);
pub(super) const KNOB: Color = Color::srgb(0.078, 0.098, 0.137);
pub(super) const WELL: Color = Color::srgb(0.106, 0.039, 0.067);
pub(super) const WINE: Color = Color::srgb(0.541, 0.122, 0.2);
pub(super) const WINE_HI: Color = Color::srgb(0.722, 0.227, 0.314);
pub(super) const SLATE: Color = Color::srgb(0.275, 0.318, 0.42);
pub(super) const TEXT_HI: Color = Color::srgb(0.91, 0.918, 0.941);
pub(super) const TEXT_DIM: Color = Color::srgb(0.545, 0.576, 0.651);
pub(super) const TEXT_DONE: Color = Color::srgb(0.435, 0.467, 0.537);
pub(super) const CRIMSON: Color = Color::srgb(0.816, 0.337, 0.416);

/// The saved CVars behind the Interface tab's size rows.
pub(super) const MAP_SIZE: &str = "SurvivorHudMap";
pub(super) const TRACKER_SIZE: &str = "SurvivorHudTracker";
pub(super) const METER_SIZE: &str = "SurvivorHudMeter";
pub(super) const BAR_SIZE: &str = "SurvivorHudBar";

/// Each panel's scale against the 768-unit canvas, from the Interface settings.
#[derive(Resource, Clone, Copy, PartialEq)]
pub(super) struct HudSizes {
    pub map: f32,
    pub tracker: f32,
    pub meter: f32,
    /// The action bar's, which is not in the column but sizes the same way.
    pub bar: f32,
}

impl Default for HudSizes {
    fn default() -> Self {
        Self {
            map: 1.0,
            tracker: 1.0,
            meter: 1.0,
            bar: 1.0,
        }
    }
}

/// A panel's place in the column, top first; the quest tracker, which has none, goes last.
#[derive(Component)]
pub(super) struct Slot(pub u8);

/// What the tracker has seen of this run: when each boss fell, and the silvers slain.
#[derive(Resource, Default)]
struct RunLog {
    kills: Vec<(String, f32)>,
    silvers: u32,
    seen: f32,
}

/// The most bosses a roster schedules; their rows are built once.
const MAX_BOSSES: usize = 6;

#[derive(Component)]
struct Tracker(f32);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum TrackText {
    Map,
    Tier,
    Clock,
    Name(usize),
    Time(usize),
    Elites,
    Silvers,
    Level,
    Kills,
    Gold,
}

#[derive(Component)]
struct TimerFill;
#[derive(Component)]
struct TimerMark(usize);
#[derive(Component)]
struct TimerBar;
#[derive(Component)]
struct BossRow(usize);
#[derive(Component)]
struct BossMark(usize);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<HudSizes>()
        .init_resource::<RunLog>()
        .add_systems(
            Update,
            (
                load_sizes,
                super::minimap::build,
                build_tracker,
                log_run,
                paint_tracker,
                order,
            )
                .chain(),
        );
}

fn load_sizes(cvars: Res<crate::cvars::Cvars>, mut sizes: ResMut<HudSizes>) {
    if !cvars.is_changed() {
        return;
    }
    let read = |key: &str| {
        cvars
            .get(key)
            .and_then(|v| v.parse::<f32>().ok())
            .map_or(1.0, |v| v.clamp(0.7, 1.6))
    };
    sizes.set_if_neq(HudSizes {
        map: read(MAP_SIZE),
        tracker: read(TRACKER_SIZE),
        meter: read(METER_SIZE),
        bar: read(BAR_SIZE),
    });
}

/// Keep the column's panels in their slots: each is spawned or rebuilt on its own schedule.
fn order(
    mut commands: Commands,
    stacks: Query<(Entity, &Children), With<RightStack>>,
    slots: Query<&Slot>,
) {
    for (stack, children) in &stacks {
        let mut sorted: Vec<Entity> = children.to_vec();
        sorted.sort_by_key(|e| slots.get(*e).map_or(9, |s| s.0));
        if sorted[..] != children[..] {
            commands.entity(stack).replace_children(&sorted);
        }
    }
}

/// A steel-edged plate, as the boss bars' frame.
pub(super) fn plate(node: Node) -> impl Bundle {
    (
        Node {
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(2.0)),
            ..node
        },
        BackgroundColor(INK),
        BorderColor::all(STEEL_LO),
        Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
        BoxShadow::new(
            Color::srgba(0.0, 0.0, 0.0, 0.55),
            Val::ZERO,
            Val::Px(2.0),
            Val::ZERO,
            Val::Px(6.0),
        ),
    )
}

pub(super) fn label(f: &Handle<Font>, value: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: f.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::new(0.0, 1.0),
            color: Color::srgba(0.0, 0.0, 0.0, 0.95),
        },
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
    )
}

/// A steel diamond `size` across, centred in a box of that size.
pub(super) fn diamond(node: Node, size: f32, color: Color) -> impl Bundle {
    (
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            ..node
        },
        BackgroundColor(color),
        Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
        UiTransform {
            rotation: Rot2::degrees(45.0),
            ..default()
        },
    )
}

fn mmss(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn build_tracker(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    sizes: Res<HudSizes>,
    stacks: Query<Entity, With<RightStack>>,
    trackers: Query<(Entity, &Tracker)>,
) {
    let Some(font) = font else { return };
    let Ok(stack) = stacks.single() else { return };
    let k = sizes.tracker;
    if let Some((e, t)) = trackers.iter().next() {
        if t.0 == k {
            return;
        }
        commands.entity(e).despawn();
    }
    let f = &font.0;
    let px = |v: f32| Val::Px(v * k);
    let pair = |row: &mut ChildSpawnerCommands, name: &str, text: TrackText| {
        row.spawn(Node {
            column_gap: px(3.0),
            ..default()
        })
        .with_children(|p| {
            p.spawn(label(f, name, 9.0 * k, STEEL));
            p.spawn((text, label(f, "", 9.0 * k, TEXT_HI)));
        });
    };
    commands
        .spawn((
            Tracker(k),
            Slot(1),
            ChildOf(stack),
            super::hud_layout::Movable::new("tracker", "Run tracker"),
            plate(Node {
                width: px(210.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(7.0), px(5.0)),
                display: Display::None,
                ..default()
            }),
        ))
        .with_children(|t| {
            t.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|h| {
                h.spawn((TrackText::Map, label(f, "", 12.0 * k, TEXT_HI)));
                h.spawn((TrackText::Tier, label(f, "", 9.0 * k, STEEL)));
            });
            t.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: px(7.0),
                margin: UiRect::vertical(px(4.0)),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    TimerBar,
                    Node {
                        flex_grow: 1.0,
                        height: px(6.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(WELL),
                    BorderColor::all(STEEL_LO),
                    Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        TimerFill,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            height: Val::Percent(100.0),
                            width: Val::Percent(0.0),
                            ..default()
                        },
                        BackgroundColor(WINE),
                    ));
                    for i in 0..MAX_BOSSES {
                        bar.spawn((
                            TimerMark(i),
                            diamond(
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Percent(50.0),
                                    margin: UiRect {
                                        left: px(-2.5),
                                        top: px(-2.5),
                                        ..default()
                                    },
                                    display: Display::None,
                                    ..default()
                                },
                                5.0 * k,
                                STEEL,
                            ),
                        ));
                    }
                });
                row.spawn((
                    TrackText::Clock,
                    label(f, "00:00", 15.0 * k, TEXT_HI),
                    Node {
                        min_width: px(40.0),
                        justify_content: JustifyContent::FlexEnd,
                        ..default()
                    },
                ));
            });
            for i in 0..MAX_BOSSES {
                t.spawn((
                    BossRow(i),
                    Node {
                        height: px(15.0),
                        align_items: AlignItems::Center,
                        margin: UiRect::horizontal(px(-7.0)),
                        padding: UiRect::horizontal(px(7.0)),
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|row| {
                    row.spawn(Node {
                        width: px(13.0),
                        height: px(15.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|m| {
                        m.spawn((
                            BossMark(i),
                            diamond(Node::default(), 5.0 * k, STEEL_LO),
                            BoxShadow::new(Color::NONE, Val::ZERO, Val::ZERO, Val::ZERO, Val::Px(4.0)),
                        ));
                    });
                    row.spawn((
                        TrackText::Name(i),
                        label(f, "", 11.0 * k, TEXT_DIM),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                    ));
                    row.spawn((TrackText::Time(i), label(f, "", 10.0 * k, TEXT_DIM)));
                });
            }
            t.spawn((
                Node {
                    height: Val::Px(1.0),
                    margin: UiRect {
                        top: px(4.0),
                        bottom: px(3.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(STEEL_LO.with_alpha(0.45)),
            ));
            t.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|row| {
                pair(row, "Elites", TrackText::Elites);
                pair(row, "Silvers", TrackText::Silvers);
                pair(row, "Lv", TrackText::Level);
                pair(row, "Kills", TrackText::Kills);
                pair(row, "Gold", TrackText::Gold);
            });
        });
}

/// Note each boss's kill time and each silver slain; a new run starts the log over.
fn log_run(run: Res<Run>, mut log: ResMut<RunLog>, mut loot: MessageReader<Loot>) {
    if run.elapsed + 0.5 < log.seen {
        *log = RunLog::default();
    }
    log.seen = run.elapsed;
    for name in &run.boss_loot {
        if !log.kills.iter().any(|(n, _)| n == name) {
            log.kills.push((name.clone(), run.elapsed));
        }
    }
    log.silvers += loot.read().filter(|l| l.silver).count() as u32;
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn paint_tracker(
    run: Res<Run>,
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    lairs: Res<Lairs>,
    log: Res<RunLog>,
    mut trackers: Query<&mut Node, With<Tracker>>,
    mut texts: Query<(&TrackText, &mut Text, &mut TextColor)>,
    mut fills: Query<&mut Node, (With<TimerFill>, Without<Tracker>)>,
    mut marks: Query<(&TimerMark, &mut Node), (Without<TimerFill>, Without<Tracker>)>,
    mut bars: Query<&mut Node, (With<TimerBar>, Without<TimerMark>, Without<TimerFill>, Without<Tracker>)>,
    mut rows: Query<
        (&BossRow, &mut Node, &mut BackgroundColor),
        (Without<TimerBar>, Without<TimerMark>, Without<TimerFill>, Without<Tracker>),
    >,
    mut boss_marks: Query<(&BossMark, &mut BackgroundColor, &mut BoxShadow), Without<BossRow>>,
) {
    let p = *phase.get();
    let shown = p != Phase::Menu && p != Phase::GameOver;
    for mut node in &mut trackers {
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    if !shown {
        return;
    }
    let map = &data::MAPS[selection.stage.min(data::MAPS.len() - 1)];
    let roster = data::roster(selection.stage);
    let bosses = &roster.bosses[..roster.bosses.len().min(MAX_BOSSES)];
    let end = bosses.iter().map(|b| b.from).fold(0.0f32, f32::max);
    let killed = |name: &str| log.kills.iter().find(|(n, _)| n == name).map(|&(_, t)| t);
    // The boss the run is on: the first still standing.
    let current = bosses.iter().position(|b| killed(b.name).is_none());

    for mut node in &mut bars {
        let display = if end > 0.0 { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    for mut node in &mut fills {
        let share = if end > 0.0 { run.elapsed / end } else { 0.0 };
        node.width = Val::Percent((share * 100.0).clamp(0.0, 100.0));
    }
    for (mark, mut node) in &mut marks {
        match bosses.get(mark.0).filter(|_| end > 0.0) {
            Some(b) => {
                node.display = Display::Flex;
                node.left = Val::Percent(b.from / end * 100.0);
            }
            None => node.display = Display::None,
        }
    }
    for (row, mut node, mut bg) in &mut rows {
        let display = if row.0 < bosses.len() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let want = if Some(row.0) == current {
            WINE_HI.with_alpha(0.28)
        } else {
            Color::NONE
        };
        bg.set_if_neq(BackgroundColor(want));
    }
    for (mark, mut bg, mut shadow) in &mut boss_marks {
        let Some(b) = bosses.get(mark.0) else { continue };
        let (color, glow) = if killed(b.name).is_some() {
            (TEXT_DONE, Color::NONE)
        } else if Some(mark.0) == current {
            (CRIMSON, CRIMSON.with_alpha(0.8))
        } else {
            (STEEL_LO, Color::NONE)
        };
        bg.set_if_neq(BackgroundColor(color));
        let want = BoxShadow::new(glow, Val::ZERO, Val::ZERO, Val::ZERO, Val::Px(4.0));
        if *shadow != want {
            *shadow = want;
        }
    }

    for (which, mut text, mut color) in &mut texts {
        let (want, tint) = match *which {
            TrackText::Map => (map.name.to_string(), TEXT_HI),
            TrackText::Tier => (
                difficulty::tier(map.name).map_or(String::new(), |t| format!("TIER {t}")),
                STEEL,
            ),
            TrackText::Clock => (mmss(run.elapsed), TEXT_HI),
            TrackText::Name(i) => match bosses.get(i) {
                Some(b) if killed(b.name).is_some() => (b.name.to_string(), TEXT_DONE),
                Some(b) if Some(i) == current => (b.name.to_string(), TEXT_HI),
                Some(b) => (b.name.to_string(), TEXT_DIM),
                None => continue,
            },
            TrackText::Time(i) => match bosses.get(i) {
                Some(b) => match killed(b.name) {
                    Some(t) => (mmss(t), TEXT_DONE),
                    None if Some(i) == current && b.from > run.elapsed => {
                        (format!("in {}", mmss(b.from - run.elapsed)), STEEL_HI)
                    }
                    None if Some(i) == current => ("now".into(), CRIMSON),
                    None => (mmss(b.from), TEXT_DIM),
                },
                None => continue,
            },
            TrackText::Elites => (
                format!("{}/{}", lairs.slain, roster.guardians.len()),
                TEXT_HI,
            ),
            TrackText::Silvers => (log.silvers.to_string(), TEXT_HI),
            TrackText::Level => (run.level.to_string(), TEXT_HI),
            TrackText::Kills => (run.kills.to_string(), TEXT_HI),
            TrackText::Gold => (run.gold.to_string(), TEXT_HI),
        };
        if text.0 != want {
            text.0 = want;
        }
        color.set_if_neq(TextColor(tint));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tracker_clock_reads_minutes_and_seconds() {
        assert_eq!(mmss(0.0), "00:00");
        assert_eq!(mmss(314.9), "05:14");
        assert_eq!(mmss(-3.0), "00:00");
    }
}
