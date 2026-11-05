//! ASR's compact encounter timers. Clocks come from the combat interpreters, never a second
//! simulation. The panel uses the HUD's font, palette and saved move/lock controls.
use bevy::prelude::*;
use bevy::text::LineBreak;
use serde::{Deserialize, Serialize};

use super::{
    data::BossAction,
    enemies::{Enemy, Script},
    hud::{HudFont, HudRoot},
    hud_layout::Movable,
    mechanics::Mechanics,
    right_column as col, Phase, Run,
};

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Timer {
    pub spell: u32,
    pub name: String,
    pub due: f32,
    pub period: f32,
    pub cast_until: f32,
    pub cast_duration: f32,
    pub ready: bool,
}

impl Timer {
    pub(super) fn native(action: BossAction, now: f32, first: f32) -> Self {
        let (spell, name, every) = match action {
            BossAction::Eggs { every, .. } => (6917, "Lay Eggs", every),
            BossAction::Stomp {
                spell,
                radius,
                every,
                ..
            } => (
                spell,
                if spell == 20549 && radius >= 8.0 {
                    "Enraged Stomp"
                } else if spell == 1680 {
                    "Whirlwind"
                } else {
                    "War Stomp"
                },
                every,
            ),
            BossAction::Throw { spell, every, .. } => (
                spell,
                if spell == 6917 {
                    "Venom Spit"
                } else {
                    "Throw Rock"
                },
                every,
            ),
            BossAction::Charge { spell, every, .. } => (
                spell,
                if spell == 6268 {
                    "Rushing Charge"
                } else {
                    "Charge"
                },
                every,
            ),
            BossAction::Bombs { spell, every, .. } => (
                spell,
                if spell == 3815 {
                    "Poison Pools"
                } else {
                    "Bombs"
                },
                every,
            ),
            BossAction::Summon { every, .. } => (0, "Call Minions", every),
        };
        Self {
            spell,
            name: name.to_string(),
            due: now + first,
            period: every.max(first).max(0.1),
            cast_until: 0.0,
            cast_duration: 0.0,
            ready: false,
        }
    }

    /// Encounter moves may share a visual spell or have no DBC icon. Use a distinct
    /// familiar ability icon for each move while retaining the actual spell for its FX.
    fn icon_spell(&self) -> u32 {
        match self.name.as_str() {
            "Boar Rush" | "Charge" | "Rushing Charge" => 100,
            "Boar Ambush" => 53,
            "Collector's Cut" => 1752,
            "Marked for Collection" => 19434,
            "Dirty Money" | "Dirty Money: Second Blast" => 13813,
            "Call the Brotherhood" => 6673,
            "Smoke and Daggers" => 1856,
            "Collector's Bombs" => 13795,
            "Tusk Sweep" | "Cleave" => 845,
            "Stampede" => 2643,
            "War Stomp" | "Stomp" => 20549,
            "Enraged Stomp" => 12328,
            "Whirlwind" => 1680,
            "Throw Rock" | "Hurl" => 2764,
            "Venom Spit" => 1978,
            "Lay Eggs" => 883,
            "Poison Pools" => 2818,
            "Call Minions" => 688,
            "Bombs" | "Bombard" => 19434,
            "Fire Pool" => 2120,
            "Meteor" => 11366,
            "Rain of Fire" => 5740,
            "Nova" => 122,
            "Closing Ring" => 1490,
            "Beam" => 15407,
            "Volley" => 1510,
            "Vortex" => 16914,
            "Fixate" => 355,
            "Shield" => 17,
            "Mend" => 136,
            "Blink" => 1953,
            _ => {
                if self.spell == 0 {
                    688
                } else {
                    self.spell
                }
            }
        }
    }

    pub(super) fn started(&mut self, now: f32, every: f32, windup: f32) {
        self.due = now + every;
        self.period = every.max(0.1);
        self.cast_until = now + windup;
        self.cast_duration = windup;
        self.ready = false;
    }

    fn casting(&self, now: f32) -> bool {
        self.cast_until > now
    }

    fn key(&self, now: f32) -> (u8, f32) {
        if self.casting(now) {
            (0, self.cast_until)
        } else if self.ready {
            (1, 0.0)
        } else {
            (2, self.due)
        }
    }

    fn valid(&self) -> bool {
        self.due.is_finite()
            && self.period.is_finite()
            && self.period > 0.0
            && self.cast_until.is_finite()
            && self.cast_duration.is_finite()
    }
}

/// Guests receive the host's absolute run-clock deadlines with each world snapshot.
#[derive(Component, Default)]
pub(super) struct ReplicaTimers(pub Vec<Timer>);

pub(super) fn snapshot(
    script: Option<&Script>,
    mechanics: Option<&Mechanics>,
    now: f32,
    enemy: &Enemy,
) -> Vec<Timer> {
    let mut timers = mechanics
        .map(|m| m.cooldowns(now))
        .unwrap_or_else(|| script.map(|s| s.cooldowns.clone()).unwrap_or_default());
    if enemy.boss && !enemy.dead && !enemy.timed_enraged {
        let warning = enemy.enrage_at - now <= 10.0;
        timers.push(Timer {
            spell: 8599,
            name: super::boss_enrage::NAME.into(),
            due: enemy.enrage_at,
            period: super::boss_enrage::SECONDS,
            cast_until: if warning { enemy.enrage_at } else { 0.0 },
            cast_duration: 10.0,
            ready: false,
        });
    }
    timers.into_iter().filter(Timer::valid).collect()
}

#[derive(Component, Default)]
struct Panel {
    signature: Vec<(Entity, usize)>,
    groups: Vec<Group>,
}
struct Group {
    rows: Vec<Row>,
}
struct Row {
    frame: Entity,
    fill: Entity,
    icon: Entity,
    name: Entity,
    seconds: Entity,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        paint
            .after(super::enemies::run_scripts)
            .after(super::mechanics::run_mechanics)
            .after(super::hud::show_panels),
    );
}

fn words(font: &Handle<Font>, value: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextLayout::new(Justify::Center, LineBreak::NoWrap),
        TextShadow {
            offset: Vec2::ONE,
            color: Color::BLACK,
        },
    )
}

fn build(
    commands: &mut Commands,
    panel: Entity,
    font: &Handle<Font>,
    fights: &[(Entity, &'static str, Vec<Timer>)],
) -> Vec<Group> {
    let mut groups = Vec::new();
    commands.entity(panel).despawn_children();
    commands.entity(panel).with_children(|p| {
        p.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(19.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(col::KNOB),
            BorderColor::all(col::STEEL_LO),
        ))
        .with_children(|h| {
            h.spawn(words(font, "Boss cooldowns", 10.0, col::STEEL_HI));
        });
        for (_, name, timers) in fights {
            let mut rows = Vec::new();
            p.spawn(Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            })
            .with_children(|g| {
                g.spawn(Node {
                    height: Val::Px(14.0),
                    width: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|h| {
                    h.spawn(words(font, name, 10.0, col::STEEL_HI));
                });
                for timer in timers {
                    let frame = g
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(17.0),
                                flex_shrink: 0.0,
                                border: UiRect::all(Val::Px(1.0)),
                                align_items: AlignItems::Center,
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BackgroundColor(col::WELL),
                            BorderColor::all(Color::BLACK),
                        ))
                        .id();
                    let mut commands = g.commands();
                    let fill = commands
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                bottom: Val::Px(0.0),
                                width: Val::Percent(0.0),
                                ..default()
                            },
                            BackgroundColor(col::WINE),
                        ))
                        .id();
                    let label = commands
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            padding: UiRect::left(Val::Px(1.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .id();
                    let icon = commands
                        .spawn((
                            Node {
                                width: Val::Px(15.0),
                                height: Val::Px(15.0),
                                flex_shrink: 0.0,
                                margin: UiRect::right(Val::Px(3.0)),
                                ..default()
                            },
                            ImageNode {
                                color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                                ..default()
                            },
                            super::hud::SpellIcon(timer.icon_spell()),
                        ))
                        .id();
                    let name = commands
                        .spawn((
                            words(font, "", 10.0, col::TEXT_HI),
                            Node {
                                flex_grow: 1.0,
                                flex_shrink: 1.0,
                                min_width: Val::Px(0.0),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                        ))
                        .insert(TextLayout::new(Justify::Left, LineBreak::NoWrap))
                        .id();
                    let seconds_seat = commands
                        .spawn(Node {
                            width: Val::Px(33.0),
                            height: Val::Percent(100.0),
                            flex_shrink: 0.0,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .id();
                    let seconds = commands.spawn(words(font, "", 10.0, col::TEXT_HI)).id();
                    commands.entity(seconds_seat).add_child(seconds);
                    commands
                        .entity(label)
                        .add_children(&[icon, name, seconds_seat]);
                    commands.entity(frame).add_children(&[fill, label]);
                    rows.push(Row {
                        frame,
                        fill,
                        icon,
                        name,
                        seconds,
                    });
                }
            });
            groups.push(Group { rows });
        }
    });
    groups
}

#[allow(clippy::type_complexity)]
fn paint(
    mut commands: Commands,
    phase: (Res<State<Phase>>, Res<super::ui_edit::UiEditor>),
    run: Res<Run>,
    font: Option<Res<HudFont>>,
    coop: Res<super::coop::Coop>,
    roots: Query<Entity, With<HudRoot>>,
    enemies: Query<(
        Entity,
        &Enemy,
        Option<&Script>,
        Option<&Mechanics>,
        Option<&ReplicaTimers>,
    )>,
    mut panels: Query<(Entity, &mut Panel, &mut Visibility)>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text>,
    mut edges: Query<&mut BorderColor>,
    mut fills: Query<&mut BackgroundColor>,
    mut colors: Query<&mut TextColor>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut icons: Query<(&mut super::hud::SpellIcon, &mut ImageNode)>,
    cvars: Res<crate::cvars::Cvars>,
) {
    let (phase, editor) = phase;
    let Some(font) = font else { return };
    let now = run.elapsed;
    let Ok((panel_id, mut panel, mut visible)) = panels.single_mut() else {
        if let Ok(root) = roots.single() {
            let id = commands
                .spawn((
                    Panel::default(),
                    Movable::new("boss_cooldowns", "Boss cooldowns"),
                    col::plate(Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(24.0),
                        top: Val::Px(180.0),
                        width: Val::Px(190.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    }),
                    Visibility::Hidden,
                ))
                .id();
            commands.entity(root).add_child(id);
        }
        return;
    };
    let mut fights: Vec<_> = enemies
        .iter()
        .filter(|(_, e, _, _, _)| e.boss && !e.dead)
        .filter_map(|(id, e, script, mech, replica)| {
            let mut timers = if coop.guest() {
                replica
                    .map(|r| r.0.iter().filter(|t| t.valid()).cloned().collect())
                    .unwrap_or_default()
            } else {
                snapshot(script, mech, now, e)
            };
            timers.sort_by(|a, b| {
                let a = a.key(now);
                let b = b.key(now);
                a.0.cmp(&b.0).then_with(|| a.1.total_cmp(&b.1))
            });
            (!timers.is_empty()).then_some((id, e.def.name, timers))
        })
        .collect();
    // Spawn order is stable; moving past another boss never exchanges their groups.
    if editor.active && fights.is_empty() {
        let timers = [(15496, "Cleave", 4.2, 8.), (20549, "War Stomp", 12., 18.)]
            .map(|(spell, name, left, period)| Timer {
                spell,
                name: name.into(),
                due: now + left,
                period,
                cast_until: 0.,
                cast_duration: 0.,
                ready: false,
            })
            .to_vec();
        fights.push((Entity::PLACEHOLDER, "Hogger", timers));
    }
    fights.sort_by_key(|(id, _, _)| id.to_bits());
    let signature: Vec<_> = fights
        .iter()
        .map(|(id, _, timers)| (*id, timers.len()))
        .collect();
    let shown = editor.active
        || (cvars.flag("SurvivorBossCooldowns").unwrap_or(true)
            && !matches!(phase.get(), Phase::Menu | Phase::GameOver)
            && !fights.is_empty());
    *visible = if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if signature != panel.signature {
        panel.groups = build(&mut commands, panel_id, &font.0, &fights);
        panel.signature = signature;
        return;
    }
    if !shown {
        return;
    }
    for (group, (_, _, timers)) in panel.groups.iter().zip(&fights) {
        for (i, (row, timer)) in group.rows.iter().zip(timers).enumerate() {
            // Rows reorder as deadlines change; update the icon together with its label.
            let spell = timer.icon_spell();
            if let Ok((mut icon, mut image)) = icons.get_mut(row.icon) {
                if icon.0 != spell {
                    icon.0 = spell;
                    image.color = Color::srgba(0.0, 0.0, 0.0, 0.6);
                    commands.entity(row.icon).remove::<super::hud::IconDone>();
                }
            }
            let casting = timer.casting(now);
            let left = if casting {
                timer.cast_until - now
            } else {
                (timer.due - now).max(0.0)
            };
            let progress = if casting {
                left / timer.cast_duration.max(0.1)
            } else if timer.ready {
                1.0
            } else {
                1.0 - left / timer.period
            };
            if let Ok(mut node) = nodes.get_mut(row.fill) {
                node.width = Val::Percent(progress.clamp(0.0, 1.0) * 100.0);
            }
            if let Ok(mut edge) = edges.get_mut(row.frame) {
                *edge = BorderColor::all(if casting {
                    col::CRIMSON
                } else if i == 0 {
                    col::STEEL
                } else {
                    Color::BLACK
                });
            }
            if let Ok(mut fill) = fills.get_mut(row.fill) {
                fill.0 = if casting { col::WINE_HI } else { col::WINE };
            }
            if let Ok(mut color) = colors.get_mut(row.seconds) {
                color.0 = if casting { col::CRIMSON } else { col::TEXT_HI };
            }
            let label = if casting {
                format!("{left:.1}!")
            } else if timer.ready {
                "Ready".to_string()
            } else {
                format!("{left:.1}")
            };
            if let Ok(mut text) = texts.get_mut(row.seconds) {
                if text.0 != label {
                    text.0 = label;
                }
            }
            // Native labels distinguish additional stomps and poison pools sharing a DBC row.
            let name = if matches!(
                timer.name.as_str(),
                "Enraged Stomp"
                    | "Poison Pools"
                    | "Lay Eggs"
                    | "Boar Rush"
                    | "Tusk Sweep"
                    | "Stampede"
                    | "Boar Ambush"
                    | "Collector's Cut"
                    | "Marked for Collection"
                    | "Dirty Money"
                    | "Call the Brotherhood"
                    | "Smoke and Daggers"
                    | "Dirty Money: Second Blast"
                    | "Collector's Bombs"
                    | "Berserk"
            ) {
                timer.name.clone()
            } else {
                spells
                    .as_ref()
                    .and_then(|s| s.catalog.get(timer.spell))
                    .map(|s| s.name.clone())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| timer.name.clone())
            };
            if let Ok(mut text) = texts.get_mut(row.name) {
                if text.0 != name {
                    text.0 = name;
                }
            }
        }
    }
}
