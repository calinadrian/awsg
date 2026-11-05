//! Plain square selection buttons, with the portrait contained inside the border.
use bevy::prelude::*;

use super::menu_skin::GOLD;
use crate::glue::art::GlueArt;
use crate::glue::widgets::{GlueText, Hilight, HoverLabel, LockHighlight, outlined_text};

#[allow(clippy::too_many_arguments)]
pub(super) fn icon_button<A: Component, I: Bundle, L: Bundle>(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    action: A,
    dyn_icon: Option<I>,
    fixed: Option<(Handle<Image>, Rect)>,
    _label_dyn: Option<L>,
    label: &str,
    _art: &GlueArt,
    scale: f32,
) {
    let size = 48.0 * scale;
    parent
        .spawn((
            action,
            Button,
            LockHighlight::default(),
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.35, 0.38, 0.42)),
            BackgroundColor(Color::srgba(0.025, 0.03, 0.04, 0.92)),
        ))
        .with_children(|button| {
            let mut icon = button.spawn((
                fixed.map_or_else(
                    || ImageNode {
                        color: Color::NONE,
                        ..default()
                    },
                    |(image, rect)| ImageNode {
                        image,
                        rect: Some(rect),
                        ..default()
                    },
                ),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
            ));
            if let Some(marker) = dyn_icon {
                icon.insert(marker);
            }
            button.spawn((
                Hilight,
                Visibility::Hidden,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-1.0),
                    right: Val::Px(-1.0),
                    top: Val::Px(-1.0),
                    bottom: Val::Px(-1.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(GOLD),
            ));
            outlined_text(
                button,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    bottom: Val::Px(-17.0),
                    width: Val::Px(160.0),
                    ..default()
                },
                (HoverLabel, Visibility::Hidden, ZIndex(20)),
                (),
                GlueText {
                    text: label,
                    size: 11.0,
                    color: GOLD,
                    wrap: false,
                },
                font,
                1.0,
            );
        });
}
