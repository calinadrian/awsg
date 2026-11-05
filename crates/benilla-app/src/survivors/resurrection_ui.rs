//! Both players see the host's resurrection channel and remaining time.
use super::{
    coop::{Coop, REVIVE_SECONDS},
    front_end as skin,
    hud::HudFont,
    Phase,
};
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct ReviveBar(u32);
#[derive(Component)]
pub(super) struct ReviveFill(u32);
#[derive(Component)]
pub(super) struct ReviveLabel(u32);

pub(super) fn refresh(
    mut commands: Commands,
    coop: Res<Coop>,
    phase: Res<State<Phase>>,
    font: Option<Res<HudFont>>,
    mut bars: Query<(Entity, &ReviveBar, &mut Node), Without<ReviveFill>>,
    mut fills: Query<(&ReviveFill, &mut Node), Without<ReviveBar>>,
    mut labels: Query<(&ReviveLabel, &mut Text)>,
) {
    let mut peers: Vec<_> = coop
        .revive_progress
        .keys()
        .copied()
        .filter(|peer| {
            coop.running
                && matches!(phase.get(), Phase::Playing | Phase::Paused | Phase::LevelUp)
                && coop
                    .room
                    .as_ref()
                    .is_some_and(|r| r.members.iter().any(|m| m.peer == *peer))
                && coop
                    .players
                    .get(peer)
                    .is_some_and(|p| p.loaded && p.hp <= 0.)
        })
        .collect();
    peers.sort_unstable();
    for (entity, bar, mut node) in &mut bars {
        if let Some(index) = peers.iter().position(|peer| *peer == bar.0) {
            node.bottom = Val::Px(180. + index as f32 * 62.);
        } else {
            commands.entity(entity).despawn();
        }
    }
    let label = |peer| {
        let name = coop
            .room
            .as_ref()
            .and_then(|r| r.members.iter().find(|m| m.peer == peer))
            .map_or("Teammate", |m| m.name.as_str());
        let remaining = (REVIVE_SECONDS - coop.revive_progress[&peer]).max(0.);
        if peer == coop.peer {
            format!("Being resurrected · {remaining:.1}s")
        } else {
            format!("Resurrecting {name} · {remaining:.1}s")
        }
    };
    for (fill, mut node) in &mut fills {
        if let Some(progress) = coop.revive_progress.get(&fill.0) {
            node.width = Val::Percent((progress / REVIVE_SECONDS).clamp(0., 1.) * 100.);
        }
    }
    for (text, mut value) in &mut labels {
        if peers.contains(&text.0) {
            value.0 = label(text.0);
        }
    }
    let Some(font) = font else { return };
    for (index, peer) in peers.into_iter().enumerate() {
        if bars.iter().any(|(_, bar, _)| bar.0 == peer) {
            continue;
        }
        commands
            .spawn((
                ReviveBar(peer),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.),
                    margin: UiRect::left(Val::Px(-170.)),
                    bottom: Val::Px(180. + index as f32 * 62.),
                    width: Val::Px(340.),
                    padding: UiRect::axes(Val::Px(12.), Val::Px(8.)),
                    border: UiRect::all(Val::Px(1.)),
                    border_radius: BorderRadius::all(Val::Px(6.)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.),
                    ..default()
                },
                BackgroundColor(skin::INK),
                BorderColor::all(skin::EDGE),
                GlobalZIndex(95),
            ))
            .with_children(|root| {
                root.spawn((
                    ReviveLabel(peer),
                    Text::new(label(peer)),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 16.,
                        ..default()
                    },
                    TextColor(skin::GOLD),
                    TextLayout::new(Justify::Center, LineBreak::NoWrap),
                ));
                root.spawn((
                    Node {
                        width: Val::Percent(100.),
                        height: Val::Px(10.),
                        border_radius: BorderRadius::all(Val::Px(3.)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(skin::SURFACE),
                ))
                .with_children(|track| {
                    track.spawn((
                        ReviveFill(peer),
                        Node {
                            width: Val::Percent(
                                (coop.revive_progress[&peer] / REVIVE_SECONDS).clamp(0., 1.) * 100.,
                            ),
                            height: Val::Percent(100.),
                            ..default()
                        },
                        BackgroundColor(skin::GOLD),
                    ));
                });
            });
    }
}
