//! The reward screen: its heading, the hand, the food bar on chests and the actions. Cards share
//! the same values used by combat and reward application.
use super::card_view::{self, card_stop, hex, CardArt, CAPTION, HAND_GAP, LEAD, SMALL, TEXT, TITLE, WIDTH};
use super::food::{self, FoodId};
use super::hud::{self, Offer, SpellIcon, UiArt};
use super::{Phase, Rng, Run};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum RewardKey {
    Spell(&'static str),
    Food(FoodId),
    Gold,
}

pub(super) fn reward_key(choice: hud::Choice, run: &Run) -> RewardKey {
    match choice {
        hud::Choice::NewAbility(def, _) => RewardKey::Spell(def.key),
        hud::Choice::Upgrade(i, _, _) => RewardKey::Spell(run.abilities[i].def.key),
        hud::Choice::Food(id) => RewardKey::Food(id),
        hud::Choice::Gold => RewardKey::Gold,
    }
}

/// Per-run controls. Account perk unlocks are not yet part of the rebuild; this mode
/// grants three free rerolls and one banish so all hand controls are available.
pub(super) struct Hand {
    pub banned: std::collections::HashSet<RewardKey>,
    free_rerolls: u32,
    paid_rerolls: u32,
    banishes: u32,
}

impl Default for Hand {
    fn default() -> Self {
        Self {
            banned: Default::default(),
            free_rerolls: 3,
            paid_rerolls: 0,
            banishes: 1,
        }
    }
}

impl Hand {
    /// Original 0.124.5 RVA 0x5c660e0: free charges, then 100 * 3^paid gold.
    fn reroll_cost(&self) -> u32 {
        if self.free_rerolls > 0 {
            0
        } else {
            3u32.saturating_pow(self.paid_rerolls).saturating_mul(100)
        }
    }

    /// A banished food lasts for its chest only: a slot's next rank is the only way that slot can
    /// climb, so banishing it for the run would strand the slot below Legendary.
    pub(super) fn forget_banished_food(&mut self) {
        self.banned.retain(|key| !matches!(key, RewardKey::Food(_)));
    }
}

/// Original RVA 0x5c627e4: a quarter of the current cache cost, at least 10.
pub(super) fn skip_gold(run: &Run) -> u32 {
    (run.chest_cost / 4).max(10)
}

const ACTION_FILL: Color = Color::srgb(0.078, 0.098, 0.137);
const BANISH_FILL: Color = Color::srgb(0.051, 0.082, 0.09);
const HOT_FILL: Color = Color::srgb(0.188, 0.075, 0.125);
const HOT_EDGE: Color = Color::srgb(0.816, 0.337, 0.416);
const DISABLED_INK: Color = Color::srgb(0.365, 0.4, 0.459);
const DISABLED_EDGE: Color = Color::srgb(0.169, 0.196, 0.239);
const REWARD_GOLD: Color = Color::srgb(0.949, 0.78, 0.431);

#[derive(Component, Clone, Copy)]
pub(super) enum HandAction {
    Reroll,
    Banish(usize),
    Skip,
}

/// The parts of an action's text that change with the run.
#[derive(Component)]
pub(super) enum ControlLabel {
    Title,
    Detail,
}

#[derive(Component)]
pub(super) struct BanishCount;

fn spawn_action(
    bar: &mut ChildSpawnerCommands,
    art: &CardArt,
    action: HandAction,
    key: &str,
    title: &str,
    scale: f32,
) {
    let u = |v: f32| px(v * scale);
    bar.spawn((
        Button,
        action,
        Node {
            height: u(44.0),
            padding: UiRect::horizontal(u(18.0)),
            column_gap: u(9.0),
            align_items: AlignItems::Center,
            border: UiRect::all(px(1.0)),
            border_radius: BorderRadius::all(px(2.0)),
            ..default()
        },
        BackgroundColor(ACTION_FILL),
        BorderColor::all(hex(0x9aa3b8)),
    ))
    .with_children(|button| {
        button
            .spawn((
                Node {
                    padding: UiRect::axes(u(6.0), u(2.0)),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(hex(0x080b0f)),
                BorderColor::all(hex(0x5d6778)),
            ))
            .with_children(|kbd| {
                kbd.spawn(card_view::text(
                    &art.label,
                    key,
                    CAPTION * scale,
                    hex(0xd0d4dd),
                ));
            });
        button.spawn((
            ControlLabel::Title,
            card_view::text(&art.body, title, card_view::BODY * scale, TEXT),
        ));
        button.spawn((
            ControlLabel::Detail,
            card_view::text(&art.label, "", SMALL * scale, REWARD_GOLD),
        ));
    });
}

fn spawn_banish(column: &mut ChildSpawnerCommands, art: &CardArt, index: usize, scale: f32) {
    column
        .spawn((
            Button,
            HandAction::Banish(index),
            Node {
                width: px(WIDTH * scale),
                height: px(34.0 * scale),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(px(2.0)),
                ..default()
            },
            BackgroundColor(BANISH_FILL),
            BorderColor::all(card_view::STEEL_LO),
        ))
        .with_children(|button| {
            button.spawn((
                ControlLabel::Title,
                card_view::text(&art.body, "Banish", SMALL * scale, card_view::MUTED),
            ));
        });
}

/// What the screen says the offer is.
fn heading(offer: &Offer, run: &Run) -> (String, &'static str) {
    if matches!(offer.choices().first(), Some(hud::Choice::Gold)) {
        let kicker = if offer.chest {
            "BATTLEGROUND CACHE · ALL FOOD AT ITS LIMIT".to_owned()
        } else {
            format!("LEVEL {} · SPELL REWARD", run.level)
        };
        return (kicker, "Take the gold");
    }
    if !offer.chest {
        return (
            format!("LEVEL {} · SPELL REWARD", run.level),
            "Choose your reward",
        );
    }
    if run.food.iter().all(Option::is_some) {
        (
            format!("BATTLEGROUND CACHE · ALL {} FOOD SLOTS FULL", food::SLOTS),
            "Improve a meal",
        )
    } else {
        (
            "BATTLEGROUND CACHE · PERMANENT FOOD".to_owned(),
            "Choose one meal",
        )
    }
}

fn spawn_heading(screen: &mut ChildSpawnerCommands, art: &CardArt, offer: &Offer, run: &Run, scale: f32) {
    let u = |v: f32| px(v * scale);
    let (kicker, title) = heading(offer, run);
    let shadow = super::front_end::shadow();
    screen
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: u(8.0),
            margin: UiRect::bottom(u(30.0)),
            ..default()
        })
        .with_children(|head| {
            head.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: u(10.0),
                ..default()
            })
            .with_children(|line| {
                let mark = || {
                    card_view::diamond(
                        Node {
                            width: u(7.0),
                            height: u(7.0),
                            ..default()
                        },
                        HOT_EDGE,
                        hex(0x090b10),
                    )
                };
                line.spawn(mark());
                line.spawn((
                    card_view::text(&art.label, kicker, CAPTION * scale, hex(0xd4d9e4)),
                    shadow,
                ));
                line.spawn(mark());
            });
            head.spawn((
                card_view::text(&art.title, title.to_uppercase(), TITLE * scale, TEXT),
                shadow,
            ));
        });
}

/// The five food slots, so a chest shows what its choice does to them.
fn spawn_food_bar(screen: &mut ChildSpawnerCommands, art: &CardArt, run: &Run, scale: f32) {
    let u = |v: f32| px(v * scale);
    screen
        .spawn(Node {
            margin: UiRect::top(u(18.0)),
            column_gap: u(7.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|bar| {
            bar.spawn(Node {
                align_items: AlignItems::Baseline,
                column_gap: u(4.0),
                margin: UiRect::right(u(7.0)),
                ..default()
            })
            .with_children(|count| {
                let held = run.food.iter().flatten().count();
                count.spawn(card_view::text(&art.label, "FOOD", CAPTION * scale, card_view::MUTED));
                count.spawn(card_view::text(&art.label, held.to_string(), LEAD * scale, TEXT));
                count.spawn(card_view::text(
                    &art.label,
                    format!("/ {}", food::SLOTS),
                    CAPTION * scale,
                    card_view::MUTED,
                ));
            });
            for (kind, held) in run.food.iter().enumerate() {
                bar.spawn(Node {
                    width: u(56.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: u(4.0),
                    ..default()
                })
                .with_children(|column| {
                    spawn_food_box(column, art, *held, scale);
                    column.spawn(card_view::text(
                        &art.label,
                        food::KINDS[kind].name,
                        CAPTION * scale,
                        card_view::MUTED,
                    ));
                });
            }
        });
}

fn spawn_food_box(column: &mut ChildSpawnerCommands, art: &CardArt, held: Option<FoodId>, scale: f32) {
    let u = |v: f32| px(v * scale);
    let edge = held.map_or(hex(0x3c4552), |id| card_view::quality(id.rarity() as usize));
    column
        .spawn((
            Node {
                width: u(42.0),
                height: u(42.0),
                padding: UiRect::all(u(2.0)),
                border: UiRect::all(px(1.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(hex(0x05080b)),
            BorderColor::all(edge),
        ))
        .with_children(|slot| {
            let Some(id) = held else {
                slot.spawn(card_view::diamond(
                    Node {
                        width: u(7.0),
                        height: u(7.0),
                        ..default()
                    },
                    hex(0x232a35),
                    hex(0x232a35),
                ));
                return;
            };
            slot.spawn((
                Node {
                    width: percent(100.0),
                    height: percent(100.0),
                    ..default()
                },
                ImageNode::default(),
                UiArt::crop(id.food().icon, [0., 1., 0., 1.]),
            ));
            slot.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: u(-5.0),
                    bottom: u(-5.0),
                    min_width: u(18.0),
                    height: u(17.0),
                    padding: UiRect::horizontal(u(3.0)),
                    border: UiRect::all(px(1.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(hex(0x10141d)),
                BorderColor::all(hex(0x778297)),
            ))
            .with_children(|badge| {
                let rank = if id.is_last() {
                    "MAX".to_owned()
                } else {
                    (id.rank + 1).to_string()
                };
                badge.spawn(card_view::text(&art.label, rank, CAPTION * scale, TEXT));
            });
        });
}

fn spawn_action_bar(screen: &mut ChildSpawnerCommands, art: &CardArt, run: &Run, scale: f32) {
    let u = |v: f32| px(v * scale);
    screen
        .spawn(Node {
            margin: UiRect::top(u(22.0)),
            column_gap: u(10.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|bar| {
            spawn_action(bar, art, HandAction::Reroll, "R", "Reroll", scale);
            bar.spawn((
                Node {
                    height: u(44.0),
                    padding: UiRect::horizontal(u(14.0)),
                    column_gap: u(8.0),
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1.0)),
                    border_radius: BorderRadius::all(px(2.0)),
                    ..default()
                },
                BackgroundColor(BANISH_FILL),
                BorderColor::all(card_view::STEEL_LO),
            ))
            .with_children(|count| {
                count.spawn(card_view::text(
                    &art.label,
                    "Banishes left",
                    SMALL * scale,
                    card_view::MUTED,
                ));
                count.spawn((
                    BanishCount,
                    card_view::text(&art.label, run.hand.banishes.to_string(), LEAD * scale, TEXT),
                ));
            });
            spawn_action(bar, art, HandAction::Skip, "S", "Skip", scale);
        });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn hand_controls(
    phase: Res<State<Phase>>,
    options: Res<super::options::Options>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pending: Res<PendingPick>,
    mut run: ResMut<Run>,
    mut offer: ResMut<Offer>,
    mut rng: ResMut<Rng>,
    mut next: ResMut<NextState<Phase>>,
    cards: Query<&Card>,
    mut buttons: Query<(
        &HandAction,
        &Interaction,
        &Children,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut labels: Query<(&ControlLabel, &mut Text, &mut TextColor)>,
    mut counts: Query<&mut Text, (With<BanishCount>, Without<ControlLabel>)>,
) {
    if *phase.get() != Phase::LevelUp || options.open || pending.0.is_some() {
        return;
    }
    for mut count in &mut counts {
        let banishes = run.hand.banishes.to_string();
        if count.0 != banishes {
            count.0 = banishes;
        }
    }
    // S is also a movement key, so a tap as the hand arrives must not throw the reward away.
    let settled = cards.iter().all(|card| card.t >= DEAL_SECS);
    let mut pressed = None;
    for (action, interaction, children, mut fill, mut edge) in &mut buttons {
        let enabled = match action {
            HandAction::Reroll => run.gold >= run.hand.reroll_cost(),
            HandAction::Banish(i) => {
                run.hand.banishes > 0
                    && offer
                        .choices()
                        .get(*i)
                        .is_some_and(|c| !matches!(c, hud::Choice::Gold))
            }
            HandAction::Skip => true,
        };
        let banish = matches!(action, HandAction::Banish(_));
        let detail = match action {
            HandAction::Reroll if run.hand.free_rerolls > 0 => {
                format!("{} free", run.hand.free_rerolls)
            }
            HandAction::Reroll => format!("{} gold", run.hand.reroll_cost()),
            HandAction::Banish(_) => String::new(),
            HandAction::Skip => format!("+{} gold", skip_gold(&run)),
        };
        let hot = enabled && *interaction != Interaction::None;
        for child in children {
            if let Ok((label, mut text, mut color)) = labels.get_mut(*child) {
                let ink = match label {
                    _ if !enabled => DISABLED_INK,
                    ControlLabel::Detail => REWARD_GOLD,
                    ControlLabel::Title if banish && hot => Color::WHITE,
                    ControlLabel::Title if banish => card_view::MUTED,
                    ControlLabel::Title => TEXT,
                };
                color.set_if_neq(TextColor(ink));
                if matches!(label, ControlLabel::Detail) && text.0 != detail {
                    text.0 = detail.clone();
                }
            }
        }
        let (rest_fill, rest_edge) = if banish {
            (BANISH_FILL, card_view::STEEL_LO)
        } else {
            (ACTION_FILL, hex(0x9aa3b8))
        };
        fill.set_if_neq(BackgroundColor(if hot { HOT_FILL } else { rest_fill }));
        edge.set_if_neq(BorderColor::all(if !enabled {
            DISABLED_EDGE
        } else if hot {
            HOT_EDGE
        } else {
            rest_edge
        }));
        let key = settled
            && match action {
                HandAction::Reroll => keys.just_pressed(KeyCode::KeyR),
                HandAction::Banish(_) => false,
                HandAction::Skip => keys.just_pressed(KeyCode::KeyS),
            };
        if enabled
            && (key
                || (*interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left)))
        {
            pressed = Some(*action);
        }
    }
    match pressed {
        Some(HandAction::Reroll) => {
            let cost = run.hand.reroll_cost();
            run.gold -= cost;
            if run.hand.free_rerolls > 0 {
                run.hand.free_rerolls -= 1;
            } else {
                run.hand.paid_rerolls = run.hand.paid_rerolls.saturating_add(1);
            }
            hud::redeal(&run, &mut rng, &mut offer);
        }
        Some(HandAction::Banish(i)) => {
            let key = reward_key(offer.choices()[i], &run);
            run.hand.banned.insert(key);
            run.hand.banishes -= 1;
            hud::redeal(&run, &mut rng, &mut offer);
        }
        Some(HandAction::Skip) => {
            hud::skip_offer(&mut run, &mut offer, &mut rng, &mut next);
        }
        None => {}
    }
}

#[derive(Component)]
pub(super) struct Card {
    index: usize,
    t: f32,
    hover: f32,
    hover_wash: Entity,
    dim: Entity,
    frame: (Color, Color),
}

#[derive(Resource, Default)]
pub(super) struct PendingPick(Option<(usize, f32)>);

impl PendingPick {
    /// Choose a card as a click does, for the reward preview.
    pub(super) fn begin(&mut self, index: usize) {
        self.0.get_or_insert((index, 0.));
    }
}
const DEAL_SECS: f32 = 0.22;
const PICK_SECS: f32 = 0.3;

#[allow(clippy::too_many_arguments)]
pub(super) fn deal_cards(
    mut commands: Commands,
    offer: Res<Offer>,
    run: Res<Run>,
    spells: Option<Res<crate::ui_action::Spells>>,
    art: Res<CardArt>,
    roots: Query<Entity, With<hud::LevelUpRoot>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut last_size: Local<Option<(u32, u32)>>,
) {
    let Ok(window) = windows.single() else { return };
    let size = (window.width().to_bits(), window.height().to_bits());
    let resized = *last_size != Some(size);
    if !offer.is_changed() && !resized {
        return;
    }
    *last_size = Some(size);
    let Ok(root) = roots.single() else { return };
    let scale = card_view::readable_scale(window.width(), window.height(), offer.choices().len());
    commands.entity(root).despawn_related::<Children>();
    let top = offer
        .choices()
        .iter()
        .map(|choice| choice.rarity() as usize)
        .max()
        .unwrap_or(0);
    let mut hand = Entity::PLACEHOLDER;
    commands.entity(root).with_children(|screen| {
        // The best card in the hand colours the room it is dealt in.
        if top >= 3 {
            let (color, alpha, reach) = if top == 3 {
                (hex(0xa335ee), 0.07, 58.0)
            } else {
                (hex(0xff8000), 0.086, 60.0)
            };
            screen.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100.0),
                    height: percent(100.0),
                    ..default()
                },
                BackgroundGradient::from(RadialGradient::new(
                    UiPosition::CENTER,
                    RadialGradientShape::FarthestCorner,
                    vec![
                        card_stop(color.with_alpha(alpha), 0.0),
                        card_stop(color.with_alpha(0.0), reach),
                    ],
                )),
            ));
        }
        spawn_heading(screen, &art, &offer, &run, scale);
        hand = screen
            .spawn(Node {
                column_gap: px(HAND_GAP * scale),
                ..default()
            })
            .id();
        if offer.chest {
            spawn_food_bar(screen, &art, &run, scale);
        }
        spawn_action_bar(screen, &art, &run, scale);
    });
    for (i, &choice) in offer.choices().iter().enumerate() {
        let card = hud::describe_card(choice, &run, spells.as_deref());
        let content = super::card_content::describe(choice, &run, &card);
        let nodes = card_view::spawn(
            &mut commands,
            &art,
            content,
            i,
            choice.rarity() as usize,
            scale,
        );
        commands.entity(nodes.root).insert((
            Card {
                index: i,
                t: -(i as f32) * 0.055,
                hover: 0.,
                hover_wash: nodes.hover,
                dim: nodes.dim,
                frame: nodes.frame,
            },
            RelativeCursorPosition::default(),
            UiTransform::default(),
            Visibility::Hidden,
        ));
        // The banish control is a sibling of the clickable card, so banishing
        // cannot also select that card's reward.
        let slot = commands
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(8.0 * scale),
                ..default()
            })
            .id();
        commands.entity(hand).add_child(slot);
        commands.entity(slot).add_child(nodes.root);
        commands.entity(slot).with_children(|column| {
            spawn_banish(column, &art, i, scale);
        });
        if let Some(path) = card.icon {
            commands
                .entity(nodes.icon)
                .insert(UiArt::crop(path, [0., 1., 0., 1.]));
        } else {
            commands.entity(nodes.icon).insert(SpellIcon(card.spell));
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn animate_cards(
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    mut cards: Query<(
        &mut Card,
        &Interaction,
        &mut UiTransform,
        &mut Visibility,
        &mut BorderColor,
    )>,
    mut actions: Query<(&HandAction, &mut Visibility), Without<Card>>,
    mut overlays: Query<&mut Visibility, (Without<Card>, Without<HandAction>)>,
    mut pending: ResMut<PendingPick>,
    mut run: ResMut<Run>,
    mut offer: ResMut<Offer>,
    mut rng: ResMut<Rng>,
    mut next: ResMut<NextState<Phase>>,
) {
    if *phase.get() != Phase::LevelUp {
        pending.0 = None;
        return;
    }
    let dt = time.delta_secs().min(0.05);
    let lift = 6.0 * card_view::GAME_SCALE;
    for (mut card, interaction, mut tf, mut vis, mut edge) in &mut cards {
        card.t += dt;
        if card.t < 0. {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;
        let hot = *interaction != Interaction::None && pending.0.is_none();
        card.hover += ((if hot { 1. } else { 0. }) - card.hover) * (1. - (-dt * 18.).exp());
        let deal = (card.t / DEAL_SECS).clamp(0., 1.);
        let picked = pending.0.is_some_and(|(i, _)| i == card.index);
        tf.translation = Val2::px(0., (1. - deal).powi(3) * 20. - card.hover * lift);
        tf.scale = Vec2::splat(if picked { 1.025 } else { 1. });
        edge.set_if_neq(BorderColor::all(if pending.0.is_some() && !picked {
            card.frame.0.mix(&Color::BLACK, 0.72)
        } else if hot || picked {
            card.frame.1
        } else {
            card.frame.0
        }));
        let shown = |on: bool| {
            if on {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        };
        if let Ok(mut wash) = overlays.get_mut(card.hover_wash) {
            wash.set_if_neq(shown(hot || picked));
        }
        if let Ok(mut dim) = overlays.get_mut(card.dim) {
            dim.set_if_neq(shown(pending.0.is_some() && !picked));
        }
    }
    // Banishing is over once a card is chosen.
    if pending.0.is_some() {
        for (action, mut visibility) in &mut actions {
            if matches!(action, HandAction::Banish(_)) {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }
    if let Some((i, t)) = pending.0.as_mut() {
        *t += dt;
        if *t >= PICK_SECS {
            let i = *i;
            pending.0 = None;
            hud::pick(i, &mut run, &mut offer, &mut rng, &mut next);
        }
    }
}

pub(super) fn choose_card(
    phase: Res<State<Phase>>,
    options: Res<super::options::Options>,
    keys: Res<ButtonInput<KeyCode>>,
    cards: Query<(&Card, &Interaction), Changed<Interaction>>,
    offer: Res<Offer>,
    mut pending: ResMut<PendingPick>,
) {
    if options.open || *phase.get() != Phase::LevelUp || pending.0.is_some() {
        return;
    }
    let mut pick = cards
        .iter()
        .find(|(c, i)| **i == Interaction::Pressed && c.t >= DEAL_SECS)
        .map(|(c, _)| c.index);
    for (i, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
        .into_iter()
        .enumerate()
    {
        if keys.just_pressed(key) {
            pick = Some(i)
        }
    }
    if let Some(i) = pick.filter(|&i| i < offer.len()) {
        pending.0 = Some((i, 0.));
    }
}
