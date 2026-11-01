//! Reward cards share the same values used by combat and chest application.
use super::hud::{self, Offer, SpellIcon, UiArt};
use super::{Phase, Rng, Run};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum RewardKey {
    Spell(&'static str),
    Jewel(usize),
    Amulet(usize),
    Gold,
}

pub(super) fn reward_key(choice: hud::Choice, run: &Run) -> RewardKey {
    match choice {
        hud::Choice::NewAbility(def, _) => RewardKey::Spell(def.key),
        hud::Choice::Upgrade(i, _, _) => RewardKey::Spell(run.abilities[i].def.key),
        hud::Choice::Jewelry(i, _) => RewardKey::Jewel(i),
        hud::Choice::Amulet(i) => RewardKey::Amulet(i),
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
}

/// Original RVA 0x5c627e4: a quarter of the current cache cost, at least 10.
pub(super) fn skip_gold(run: &Run) -> u32 {
    (run.chest_cost / 4).max(10)
}

#[derive(Component, Clone, Copy)]
pub(super) enum HandAction {
    Reroll,
    Banish(usize),
    Skip,
}
#[derive(Component)]
pub(super) struct ControlLabel;

pub(super) fn spawn_controls(ui: &mut ChildSpawnerCommands, art: &super::card_view::CardArt) {
    ui.spawn(Node {
        column_gap: Val::Px(12.),
        ..default()
    })
    .with_children(|row| {
        for action in [HandAction::Reroll, HandAction::Skip] {
            spawn_control(row, art, action, 218., "");
        }
    });
}

fn spawn_control(
    ui: &mut ChildSpawnerCommands,
    art: &super::card_view::CardArt,
    action: HandAction,
    width: f32,
    label: &str,
) {
    ui.spawn((
        Button,
        action,
        Node {
            width: Val::Px(width),
            height: Val::Px(38.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(4.)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.04, 0.06, 0.07)),
        BorderColor::all(Color::srgb(0.55, 0.45, 0.25)),
    ))
    .with_children(|button| {
        button.spawn((
            ControlLabel,
            super::card_view::text(&art.body, label, 17., super::card_view::TEXT),
        ));
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
    mut buttons: Query<(&HandAction, &Interaction, &Children, &mut BackgroundColor)>,
    mut labels: Query<(&mut Text, &mut TextColor), With<ControlLabel>>,
) {
    if *phase.get() != Phase::LevelUp || options.open || pending.0.is_some() {
        return;
    }
    let mut pressed = None;
    for (action, interaction, children, mut bg) in &mut buttons {
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
        let label = match action {
            HandAction::Reroll if run.hand.free_rerolls > 0 => {
                format!("Reroll ({} free) [R]", run.hand.free_rerolls)
            }
            HandAction::Reroll => format!("Reroll ({} gold) [R]", run.hand.reroll_cost()),
            HandAction::Banish(_) => format!("Banish ({} left)", run.hand.banishes),
            HandAction::Skip => format!("Skip & take {} gold", skip_gold(&run)),
        };
        for child in children {
            if let Ok((mut text, mut color)) = labels.get_mut(*child) {
                if text.0 != label {
                    text.0 = label.clone();
                }
                color.0 = if enabled {
                    super::card_view::TEXT
                } else {
                    Color::srgb(0.4, 0.4, 0.4)
                };
            }
        }
        bg.0 = if enabled && *interaction != Interaction::None {
            Color::srgb(0.16, 0.13, 0.08)
        } else {
            Color::srgb(0.04, 0.06, 0.07)
        };
        let key = match action {
            HandAction::Reroll => keys.just_pressed(KeyCode::KeyR),
            HandAction::Banish(_) => false,
            HandAction::Skip => false,
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
}

#[derive(Resource, Default)]
pub(super) struct PendingPick(Option<(usize, f32)>);
const DEAL_SECS: f32 = 0.22;
const PICK_SECS: f32 = 0.16;

pub(super) fn deal_cards(
    mut commands: Commands,
    offer: Res<Offer>,
    run: Res<Run>,
    spells: Option<Res<crate::ui_action::Spells>>,
    art: Res<super::card_view::CardArt>,
    mut rows: Query<(Entity, &mut Node), With<hud::OfferRow>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut last_size: Local<Option<(u32, u32)>>,
    mut sub: Query<&mut Text, With<hud::LevelUpSub>>,
) {
    let Ok(window) = windows.single() else { return };
    let size = (window.width().to_bits(), window.height().to_bits());
    let resized = *last_size != Some(size);
    if !offer.is_changed() && !resized {
        return;
    }
    *last_size = Some(size);
    let card_scale =
        super::card_view::readable_scale(window.width(), window.height(), offer.choices().len());
    if let Ok(mut t) = sub.single_mut() {
        **t = if offer.chest {
            "BATTLEGROUND CACHE  /  Choose one treasure".into()
        } else {
            format!("LEVEL {}  /  Choose your next spell upgrade", run.level)
        };
    }
    let Ok((row, mut row_node)) = rows.single_mut() else {
        return;
    };
    row_node.column_gap = px(32.0 * card_scale);
    commands.entity(row).despawn_related::<Children>();
    for (i, &choice) in offer.choices().iter().enumerate() {
        let card = hud::describe_card(choice, &run, spells.as_deref());
        let content = super::card_content::describe(choice, &run, &card);
        let nodes = super::card_view::spawn(
            &mut commands,
            &art,
            content,
            i,
            choice.rarity() as usize,
            card_scale,
        );
        commands.entity(nodes.root).insert((
            Card {
                index: i,
                t: -(i as f32) * 0.055,
                hover: 0.,
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
                row_gap: Val::Px(10.),
                ..default()
            })
            .id();
        commands.entity(row).add_child(slot);
        commands.entity(slot).add_child(nodes.root);
        commands.entity(slot).with_children(|column| {
            spawn_control(
                column,
                &art,
                HandAction::Banish(i),
                super::card_view::WIDTH * card_scale,
                &format!("Banish ({} left)", run.hand.banishes),
            );
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
    mut cards: Query<(&mut Card, &Interaction, &mut UiTransform, &mut Visibility)>,
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
    for (mut card, interaction, mut tf, mut vis) in &mut cards {
        card.t += dt;
        if card.t < 0. {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;
        let hot = *interaction != Interaction::None && pending.0.is_none();
        card.hover += ((if hot { 1. } else { 0. }) - card.hover) * (1. - (-dt * 18.).exp());
        let deal = (card.t / DEAL_SECS).clamp(0., 1.);
        let lift = (1. - deal).powi(3) * 20. - card.hover * 6.;
        let picked = pending.0.is_some_and(|(i, _)| i == card.index);
        tf.translation = Val2::px(0., lift);
        tf.scale = Vec2::splat(if picked {
            1.025
        } else {
            1. + card.hover * 0.012
        });
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
