//! Compact encounter instructions. Each mechanic owns an explicit severity; live interpreter
//! clocks (or their host snapshots) drive countdowns. Visibility filters are saved CVars.
use super::{
    boss_cooldowns::{self, ReplicaTimers},
    enemies::{Enemy, Script},
    hud::{HudFont, HudRoot},
    hud_layout::Movable,
    mechanics::Mechanics,
    mother_fang::Eggs,
    right_column as col, Phase, Run,
};
use bevy::{prelude::*, text::LineBreak};
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Severity {
    Standard,
    Dangerous,
    Lethal,
}
impl Severity {
    fn key(self) -> &'static str {
        match self {
            Self::Lethal => "SurvivorWarningLethal",
            Self::Dangerous => "SurvivorWarningDangerous",
            Self::Standard => "SurvivorWarningStandard",
        }
    }
    fn color(self) -> Color {
        match self {
            Self::Lethal => Color::srgb(1.0, 0.22, 0.26),
            Self::Dangerous => Color::srgb(0.91, 0.88, 0.79),
            Self::Standard => col::STEEL,
        }
    }
}

/// Consequence-based catalog, separate from the instruction wording and urgency ordering.
/// Avoid treating ordinary adds as an unavoidable wipe: eggs are Dangerous, not Lethal.
fn mechanic(name: &str, spell: u32) -> Option<(Severity, &'static str)> {
    use Severity::*;
    Some(match (name, spell) {
        ("Collector's Cut", _) => (Dangerous, "Move behind"),
        ("Marked for Collection", _) => (Dangerous, "Sidestep locked lane"),
        ("Berserk", _) => (Lethal, "Berserk imminent: finish boss"),
        ("Collector's Bombs", _) => (Lethal, "Avoid persistent bombs"),
        ("Dirty Money", _) => (Dangerous, "Leave coin circles"),
        ("Dirty Money: Second Blast", _) => (Dangerous, "Second blast: stay out"),
        ("Call the Brotherhood", _) => (Dangerous, "Kill marksman first"),
        ("Smoke and Daggers", _) => (Dangerous, "Dodge ambush cone"),
        ("Boar Ambush", _) => (Dangerous, "Dodge boar lane"),
        ("Stampede", _) => (Dangerous, "Dodge boar lanes"),
        ("Boar Rush", _) => (Dangerous, "Sidestep"),
        ("Tusk Sweep", _) => (Dangerous, "Move behind"),
        ("Lay Eggs", _) => return None,
        ("Meteor", _) => (Lethal, "Stack together"),
        ("Rain of Fire", _) => (Lethal, "Find safe ground"),
        ("Closing Ring", _) => (Lethal, "Stay inside"),
        ("Nova", _) => (Lethal, "Find safe zone"),
        (_, 15496) | ("Cleave", _) => (Dangerous, "Move behind"),
        (_, 20549) | ("Stomp" | "War Stomp" | "Enraged Stomp", _) => (Dangerous, "Move out"),
        (_, 100 | 6268) | ("Charge", _) => (Dangerous, "Sidestep"),
        ("Whirlwind", _) => (Dangerous, "Move out"),
        ("Beam" | "Volley", _) => (Dangerous, "Dodge lanes"),
        ("Vortex", _) => (Dangerous, "Move out"),
        ("Fixate", _) => (Dangerous, "Keep running"),
        ("Shield", _) => (Dangerous, "Kill guards"),
        ("Mend", _) => (Dangerous, "Break the heal"),
        ("Call Minions", _) => (Dangerous, "Focus minions"),
        ("Poison Pools" | "Bombs" | "Bombard" | "Fire Pool", _) => (Dangerous, "Avoid pools"),
        (_, 4164 | 6917) | ("Hurl" | "Throw Rock", _) => (Standard, "Leave circle"),
        _ => (Standard, "Dodge attack"),
    })
}

#[derive(Component)]
struct Panel {
    rows: Vec<Row>,
}
struct Row {
    seat: Entity,
    icon: Entity,
    diamond: Entity,
    boss: Entity,
    action: Entity,
    timer: Entity,
    fill: Entity,
}
#[derive(Clone)]
struct Alert {
    boss: &'static str,
    action: String,
    until: f32,
    duration: f32,
    severity: Severity,
}
#[derive(Default)]
struct Notices {
    enraged: HashSet<Entity>,
    berserk: HashSet<Entity>,
    adds: HashSet<Entity>,
    poisoned: bool,
    events: Vec<Alert>,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        announce
            .after(super::enemies::run_scripts)
            .after(super::mechanics::run_mechanics)
            .after(super::mother_fang::tick)
            .after(super::hud::show_panels),
    );
}
fn words(font: &Handle<Font>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
        TextColor(color),
        TextShadow {
            offset: Vec2::ONE,
            color: Color::BLACK,
        },
    )
}
fn create(commands: &mut Commands, root: Entity, font: &Handle<Font>) {
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                // Leave the center announcement frame clear beneath these mechanic rows.
                top: Val::Percent(12.0),
                left: Val::Percent(50.0),
                width: Val::Px(224.0),
                margin: UiRect::left(Val::Px(-112.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            Movable::new("encounter_announcer", "Encounter warnings"),
            Visibility::Hidden,
        ))
        .id();
    let mut rows = Vec::new();
    for _ in 0..3 {
        let seat = commands
            .spawn((
                Node {
                    height: Val::Px(20.0),
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    padding: UiRect::axes(Val::Px(2.0), Val::Px(1.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(col::INK),
                BorderColor::all(col::STEEL_LO),
            ))
            .id();
        let symbol = commands
            .spawn(Node {
                width: Val::Px(15.0),
                height: Val::Px(15.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .id();
        let icon = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(15.0),
                    height: Val::Px(15.0),
                    ..default()
                },
                ImageNode::default(),
                super::plates::BarGlyph::Skull,
            ))
            .id();
        let diamond = commands
            .spawn((
                Node {
                    width: Val::Px(6.0),
                    height: Val::Px(6.0),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::degrees(45.0)),
                BackgroundColor(col::STEEL),
            ))
            .id();
        commands.entity(symbol).add_children(&[icon, diamond]);
        let boss = commands
            .spawn((
                words(font, 8.0, col::TEXT_DIM),
                Node {
                    max_width: Val::Px(69.0),
                    flex_shrink: 0.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
            ))
            .id();
        let action = commands
            .spawn((
                words(font, 9.0, col::TEXT_HI),
                Node {
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
            ))
            .id();
        let timer = commands
            .spawn((
                words(font, 9.0, col::TEXT_HI),
                Node {
                    width: Val::Px(29.0),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .insert(TextLayout::new(Justify::Center, LineBreak::NoWrap))
            .id();
        let track = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(20.0),
                    right: Val::Px(2.0),
                    bottom: Val::Px(0.0),
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ))
            .id();
        let fill = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(col::STEEL),
            ))
            .id();
        commands.entity(track).add_child(fill);
        commands
            .entity(seat)
            .add_children(&[symbol, boss, action, timer, track]);
        commands.entity(panel).add_child(seat);
        rows.push(Row {
            seat,
            icon,
            diamond,
            boss,
            action,
            timer,
            fill,
        });
    }
    commands.entity(panel).insert(Panel { rows });
    commands.entity(root).add_child(panel);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn announce(
    mut commands: Commands,
    run: Res<Run>,
    phase: Res<State<Phase>>,
    eggs: Res<Eggs>,
    coop: Res<super::coop::Coop>,
    font: Option<Res<HudFont>>,
    cvars: Res<crate::cvars::Cvars>,
    roots: Query<Entity, With<HudRoot>>,
    enemies: Query<(
        Entity,
        &Enemy,
        Option<&Script>,
        Option<&Mechanics>,
        Option<&ReplicaTimers>,
    )>,
    mut panels: Query<(&Panel, &mut Visibility)>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<(&mut Text, &mut TextColor)>,
    mut fills: Query<&mut BackgroundColor>,
    mut icons: Query<&mut ImageNode>,
    mut edges: Query<&mut BorderColor>,
    mut notices: Local<Notices>,
) {
    let Some(font) = font else {
        return;
    };
    let Ok((panel, mut shown)) = panels.single_mut() else {
        if let Ok(root) = roots.single() {
            create(&mut commands, root, &font.0);
        }
        return;
    };
    if matches!(phase.get(), Phase::Menu | Phase::GameOver) {
        *shown = Visibility::Hidden;
        *notices = Notices::default();
        return;
    }
    let now = run.elapsed;
    notices.events.retain(|e| e.until > now);
    notices.enraged.retain(|id| enemies.get(*id).is_ok());
    notices.berserk.retain(|id| enemies.get(*id).is_ok());
    notices.adds.retain(|id| enemies.get(*id).is_ok());
    let mut alerts = Vec::new();
    let mut new_adds = false;
    for (id, enemy, script, mechanics, replica) in &enemies {
        if enemy.dead {
            continue;
        }
        if enemy.timed_enraged && notices.berserk.insert(id) {
            notices.events.push(Alert {
                boss: enemy.def.name, action: "Berserk: +500% damage, +50% speed".into(),
                until: now + 5.0, duration: 5.0, severity: Severity::Lethal,
            });
        }
        if enemy.hogger_enraged && notices.enraged.insert(id) {
            notices.events.push(Alert {
                boss: enemy.def.name,
                action: format!("{}: +50% damage", super::hogger::FRENZY_NAME),
                until: now + 3.0,
                duration: 3.0,
                severity: Severity::Dangerous,
            });
        }
        if std::ptr::eq(enemy.def, &super::mother_fang::BROODLING) && notices.adds.insert(id) {
            new_adds = true;
        }
        if !enemy.boss {
            continue;
        }
        let timers = if coop.guest() {
            replica.map(|r| r.0.clone()).unwrap_or_default()
        } else {
            boss_cooldowns::snapshot(script, mechanics, now, enemy)
        };
        for timer in timers
            .iter()
            .filter(|t| t.cast_until.is_finite() && t.cast_until > now)
        {
            if let Some((severity, action)) = mechanic(&timer.name, timer.spell) {
                alerts.push(Alert {
                    boss: enemy.def.name,
                    action: action.into(),
                    until: timer.cast_until,
                    duration: timer.cast_duration.max(0.1),
                    severity,
                });
            }
        }
    }
    if new_adds {
        notices.events.push(Alert {
            boss: "Mother Fang",
            action: "Focus spiders".into(),
            until: now + 3.0,
            duration: 3.0,
            severity: Severity::Dangerous,
        });
    }
    let poisoned = run.brood_poison.until > now;
    if poisoned && !notices.poisoned {
        notices.events.push(Alert {
            boss: "Mother Fang",
            action: "Venom: slowed".into(),
            until: now + 2.5,
            duration: 2.5,
            severity: Severity::Dangerous,
        });
    }
    notices.poisoned = poisoned;
    alerts.extend(notices.events.iter().cloned());
    if let Some((count, left, duration)) = eggs.objective(now) {
        alerts.push(Alert {
            boss: "Mother Fang",
            action: format!("Crush {count} eggs"),
            until: now + left,
            duration,
            severity: Severity::Dangerous,
        });
    }
    alerts.retain(|a| cvars.flag(a.severity.key()).unwrap_or(true));
    // Severity first, then soonest consequence. Keep one instruction per boss so three
    // simultaneous encounters fit in the compact stack without one boss taking every seat.
    alerts.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.until.total_cmp(&b.until))
    });
    let mut bosses = HashSet::new();
    alerts.retain(|a| bosses.insert(a.boss));
    let enabled = cvars.flag("SurvivorEncounterWarnings").unwrap_or(true);
    *shown = if enabled && !alerts.is_empty() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (i, row) in panel.rows.iter().enumerate() {
        let alert = alerts.get(i);
        if let Ok(mut node) = nodes.get_mut(row.seat) {
            node.display = if alert.is_some() {
                Display::Flex
            } else {
                Display::None
            };
        }
        let Some(alert) = alert else {
            continue;
        };
        let color = alert.severity.color();
        let skull = alert.severity != Severity::Standard;
        if let Ok(mut node) = nodes.get_mut(row.icon) {
            node.display = if skull { Display::Flex } else { Display::None };
        }
        if let Ok(mut node) = nodes.get_mut(row.diamond) {
            node.display = if skull { Display::None } else { Display::Flex };
        }
        if let Ok(mut image) = icons.get_mut(row.icon) {
            // Only Lethal pulses, briefly as the warning starts; the icon remains red.
            let age = alert.duration - (alert.until - now);
            let alpha = if alert.severity == Severity::Lethal && age < 1.0 {
                0.65 + 0.35 * (now * 16.0).sin().abs()
            } else {
                1.0
            };
            image.color = color.with_alpha(alpha);
        }
        if let Ok(mut edge) = edges.get_mut(row.seat) {
            *edge = BorderColor::all(if alert.severity == Severity::Lethal {
                color
            } else {
                col::STEEL_LO
            });
        }
        if let Ok(mut fill) = fills.get_mut(row.fill) {
            fill.0 = color;
        }
        if let Ok(mut node) = nodes.get_mut(row.fill) {
            node.width = Val::Percent(
                ((alert.until - now) / alert.duration.max(0.1)).clamp(0.0, 1.0) * 100.0,
            );
        }
        for (entity, value, tint) in [
            (row.boss, alert.boss.to_string(), col::TEXT_DIM),
            (row.action, alert.action.clone(), col::TEXT_HI),
            (
                row.timer,
                format!("{:.1}", (alert.until - now).max(0.0)),
                color,
            ),
        ] {
            if let Ok((mut text, mut text_color)) = texts.get_mut(entity) {
                if text.0 != value {
                    text.0 = value;
                }
                text_color.0 = tint;
            }
        }
    }
}
