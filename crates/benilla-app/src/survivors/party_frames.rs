//! Co-op party frames and overhead nameplates, driven by the directory's player IDs.
use super::{
    coop::{Ally, Coop},
    hud::HudFont,
};
use bevy::prelude::*;

#[derive(Component)]
struct PartyFrame {
    peer: u32,
    overhead: bool,
}
#[derive(Component)]
struct PartyLabel {
    peer: u32,
    overhead: bool,
}
#[derive(Component)]
struct PartyFill(u32);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (sync, position).chain());
}

fn sync(
    mut commands: Commands,
    coop: Res<Coop>,
    font: Option<Res<HudFont>>,
    frames: Query<(Entity, &PartyFrame)>,
) {
    let Some(font) = font else {
        return;
    };
    for (entity, frame) in &frames {
        if !coop.running
            || coop
                .room
                .as_ref()
                .is_none_or(|r| !r.members.iter().any(|m| m.peer == frame.peer))
        {
            commands.entity(entity).despawn();
        }
    }
    let Some(room) = coop.room.as_ref().filter(|_| coop.running) else {
        return;
    };
    for member in &room.members {
        for overhead in [false, true] {
            if overhead && member.peer == coop.peer {
                continue;
            }
            if frames
                .iter()
                .any(|(_, f)| f.peer == member.peer && f.overhead == overhead)
            {
                continue;
            }
            commands
                .spawn((
                    PartyFrame {
                        peer: member.peer,
                        overhead,
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(if overhead { 128. } else { 182. }),
                        height: Val::Px(if overhead { 30. } else { 46. }),
                        border: UiRect::all(Val::Px(1.)),
                        padding: UiRect::all(Val::Px(4.)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(
                        0.02,
                        0.025,
                        0.02,
                        if overhead { 0.6 } else { 0.9 },
                    )),
                    BorderColor::all(Color::srgb(0.42, 0.38, 0.23)),
                    GlobalZIndex(if overhead { 80 } else { 90 }),
                    Visibility::Hidden,
                ))
                .with_children(|root| {
                    root.spawn((
                        PartyLabel {
                            peer: member.peer,
                            overhead,
                        },
                        Text::new(&member.name),
                        TextFont {
                            font: font.0.clone(),
                            font_size: if overhead { 11. } else { 12. },
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 1., 0.65)),
                        TextShadow {
                            offset: Vec2::ONE,
                            color: Color::BLACK,
                        },
                        TextLayout::new(Justify::Center, LineBreak::NoWrap),
                    ));
                    root.spawn((
                        Node {
                            width: Val::Percent(100.),
                            height: Val::Px(if overhead { 4. } else { 8. }),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.025, 0.02)),
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            PartyFill(member.peer),
                            Node {
                                width: Val::Percent(100.),
                                height: Val::Percent(100.),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.12, 0.65, 0.18)),
                        ));
                    });
                });
        }
    }
}

fn position(
    coop: Res<Coop>,
    nameplates: Res<super::plates::NameplateSettings>,
    ui_scale: Res<UiScale>,
    cameras: Query<(&Camera, &GlobalTransform), With<benilla_world::view::WorldCamera>>,
    allies: Query<(&Ally, &Transform)>,
    mut frames: Query<(&PartyFrame, &mut Node, &mut Visibility), Without<PartyFill>>,
    mut labels: Query<(&PartyLabel, &mut Text)>,
    mut fills: Query<(&PartyFill, &mut Node), Without<PartyFrame>>,
) {
    for (frame, mut node, mut visibility) in &mut frames {
        *visibility = Visibility::Hidden;
        let Some(room) = coop.room.as_ref().filter(|_| coop.running) else {
            continue;
        };
        if frame.overhead && !nameplates.friends {
            continue;
        }
        let Some(index) = room.members.iter().position(|m| m.peer == frame.peer) else {
            continue;
        };
        if frame.overhead {
            let Some(pose) = coop
                .players
                .get(&frame.peer)
                .filter(|p| p.loaded && p.hp > 0.)
            else {
                continue;
            };
            let pos = allies
                .iter()
                .find(|(ally, _)| ally.0 == frame.peer)
                .map_or(Vec3::from_array(pose.pos), |(_, tf)| tf.translation);
            let Ok((camera, transform)) = cameras.single() else {
                continue;
            };
            let Ok(at) = camera.world_to_viewport(transform, pos + Vec3::Y * 2.6) else {
                continue;
            };
            let Some(size) = camera.logical_viewport_size() else {
                continue;
            };
            if at.x < 0. || at.y < 0. || at.x > size.x || at.y > size.y {
                continue;
            }
            node.left = Val::Px(at.x / ui_scale.0 - 64.);
            node.top = Val::Px(at.y / ui_scale.0 - 32.);
        } else {
            node.left = Val::Px(18.);
            // Leave the player frame's buff row (106..142) unobstructed.
            node.top = Val::Px(152. + index as f32 * 50.);
        }
        *visibility = Visibility::Inherited;
    }
    for (label, mut text) in &mut labels {
        let Some(member) = coop
            .room
            .as_ref()
            .and_then(|r| r.members.iter().find(|m| m.peer == label.peer))
        else {
            continue;
        };
        let value = match coop.players.get(&label.peer) {
            Some(p) if label.overhead => format!(
                "{}{}",
                member.name,
                if p.paused { " · Choosing" } else { "" }
            ),
            Some(p) => format!(
                "{} · {}\n{:.0}/{:.0}{}",
                member.name,
                p.level,
                p.hp.max(0.),
                p.maximum,
                if !p.loaded {
                    " · Loading"
                } else if p.hp <= 0. {
                    " · Fallen"
                } else if p.paused {
                    " · Choosing"
                } else {
                    ""
                }
            ),
            None => format!("{} · Loading", member.name),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    for (fill, mut node) in &mut fills {
        node.width = Val::Percent(
            coop.players
                .get(&fill.0)
                .map_or(0., |p| (p.hp / p.maximum.max(1.)).clamp(0., 1.) * 100.),
        );
    }
}
