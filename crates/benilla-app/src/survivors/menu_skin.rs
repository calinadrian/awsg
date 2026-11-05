//! Main-menu colours and compact controls, independent of the Adventure atlas skin.
pub(super) use super::front_end::{ALLIANCE, Control, GREEN, HORDE, chip, gap, label, plural, size};
use bevy::prelude::*;

pub(super) const INK: Color = Color::srgb(0.025, 0.03, 0.035);
pub(super) const SURFACE: Color = Color::srgb(0.055, 0.065, 0.075);
pub(super) const EDGE: Color = Color::srgb(0.29, 0.33, 0.38);
pub(super) const GOLD: Color = Color::srgb(0.72, 0.23, 0.31);
pub(super) const TEXT: Color = Color::srgb(0.9, 0.92, 0.94);
pub(super) const MUTED: Color = Color::srgb(0.52, 0.58, 0.64);
pub(super) const WELL: Color = Color::srgb(0.012, 0.016, 0.02);

#[derive(Component)]
pub(super) struct MenuControl;

pub(super) fn panel(node: Node) -> impl Bundle {
    (
        Node {
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(3.)),
            ..node
        },
        BackgroundColor(INK),
        BorderColor::all(EDGE),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn button<A: Component>(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    title: &str,
    detail: &str,
    action: A,
    primary: bool,
    selected: bool,
    enabled: bool,
) -> Entity {
    p.spawn((
        Button,
        action,
        MenuControl,
        Control {
            primary,
            selected,
            enabled,
        },
        Node {
            width: Val::Percent(100.),
            min_height: Val::Px(30.),
            padding: UiRect::axes(Val::Px(10.), Val::Px(6.)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(3.)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(2.),
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(if primary {
            Color::srgb(0.54, 0.12, 0.2)
        } else {
            SURFACE
        }),
        BorderColor::all(if primary || selected { GOLD } else { EDGE }),
    ))
    .with_children(|p| {
        if !title.is_empty() {
            label(p, f, title, size::BODY, TEXT);
        }
        if !detail.is_empty() {
            label(p, f, detail, size::CAPTION, MUTED);
        }
    })
    .id()
}

pub(super) fn tab<A: Component>(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    title: &str,
    action: A,
) -> Entity {
    tab_state(p, f, title, action, false)
}

pub(super) fn tab_state<A: Component>(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    title: &str,
    action: A,
    selected: bool,
) -> Entity {
    let e = button(p, f, title, "", action, false, selected, true);
    p.commands().entity(e).insert(Node {
        height: Val::Px(30.),
        padding: UiRect::horizontal(Val::Px(10.)),
        border: UiRect::all(Val::Px(1.)),
        border_radius: BorderRadius::all(Val::Px(3.)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        flex_shrink: 0.,
        ..default()
    });
    e
}

pub(super) fn heading(p: &mut ChildSpawnerCommands, f: &Handle<Font>, text: &str) {
    label(p, f, text, size::BODY, TEXT);
}

pub(super) fn rule(p: &mut ChildSpawnerCommands) {
    p.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Px(1.),
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(EDGE),
    ));
}

pub(super) fn stat_tile(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    caption: &str,
    value: &str,
    font_size: f32,
) -> Entity {
    let mut e = Entity::PLACEHOLDER;
    p.spawn((
        Node {
            flex_grow: 1.,
            flex_basis: Val::Px(0.),
            min_width: Val::Px(0.),
            padding: UiRect::all(Val::Px(6.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.),
            border: UiRect::all(Val::Px(1.)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.035, 0.055, 0.065)),
        BorderColor::all(EDGE),
    ))
    .with_children(|p| {
        label(p, f, caption, 9., MUTED);
        e = label(p, f, value, font_size, TEXT);
    });
    e
}

pub(super) fn hover(
    mut buttons: Query<
        (
            &Interaction,
            &Control,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<MenuControl>,
    >,
) {
    for (interaction, control, mut bg, mut edge) in &mut buttons {
        let hot = control.enabled && *interaction != Interaction::None;
        bg.0 = if !control.enabled {
            Color::srgb(0.045, 0.06, 0.07)
        } else if control.primary {
            if hot {
                Color::srgb(0.63, 0.17, 0.26)
            } else {
                Color::srgb(0.54, 0.12, 0.2)
            }
        } else if control.selected {
            Color::srgb(0.16, 0.08, 0.12)
        } else if hot {
            Color::srgb(0.12, 0.17, 0.2)
        } else {
            SURFACE
        };
        *edge = BorderColor::all(
            if control.enabled && (hot || control.selected || control.primary) {
                GOLD
            } else {
                EDGE
            },
        );
    }
}
