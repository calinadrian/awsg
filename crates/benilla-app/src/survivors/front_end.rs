//! Shared dark-fantasy panels and controls for hero selection and the party lobby.
use bevy::prelude::*;

pub(super) const INK: Color = Color::srgba(0.035, 0.032, 0.028, 0.96);
pub(super) const SURFACE: Color = Color::srgb(0.075, 0.065, 0.048);
pub(super) const EDGE: Color = Color::srgb(0.34, 0.27, 0.14);
pub(super) const GOLD: Color = Color::srgb(0.95, 0.78, 0.43);
pub(super) const TEXT: Color = Color::srgb(0.91, 0.87, 0.76);
pub(super) const MUTED: Color = Color::srgb(0.61, 0.59, 0.51);
pub(super) const GREEN: Color = Color::srgb(0.46, 0.78, 0.58);

#[derive(Component)]
pub(super) struct Control {
    pub primary: bool,
    pub selected: bool,
    pub enabled: bool,
}

pub(super) fn panel(node: Node) -> impl Bundle {
    (
        Node {
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(9.)),
            ..node
        },
        BackgroundColor(INK),
        BorderColor::all(EDGE),
        BoxShadow::new(
            Color::srgba(0., 0., 0., 0.55),
            Val::Px(0.),
            Val::Px(8.),
            Val::Px(3.),
            Val::Px(18.),
        ),
    )
}
pub(super) fn label(
    p: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    text: impl Into<String>,
    size: f32,
    color: Color,
) -> Entity {
    p.spawn((
        Text::new(text),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
    ))
    .id()
}
pub(super) fn rule(p: &mut ChildSpawnerCommands) {
    p.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Px(1.),
            flex_shrink: 0.,
            margin: UiRect::vertical(Val::Px(5.)),
            ..default()
        },
        BackgroundColor(EDGE),
    ));
}
pub(super) fn button<A: Component>(
    p: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    title: &str,
    detail: &str,
    action: A,
    primary: bool,
    selected: bool,
    enabled: bool,
) -> Entity {
    let background = if primary {
        Color::srgb(0.29, 0.075, 0.043)
    } else {
        SURFACE
    };
    p.spawn((
        Button,
        action,
        Control {
            primary,
            selected,
            enabled,
        },
        Node {
            min_height: Val::Px(if detail.is_empty() { 34. } else { 62. }),
            width: Val::Percent(100.),
            padding: UiRect::axes(Val::Px(14.), Val::Px(9.)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(5.)),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.),
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(if selected || primary { GOLD } else { EDGE }),
    ))
    .with_children(|p| {
        label(
            p,
            font,
            title,
            if primary { 21. } else { 17. },
            if enabled { GOLD } else { MUTED },
        );
        if !detail.is_empty() {
            label(p, font, detail, 11., if enabled { TEXT } else { MUTED });
        }
    })
    .id()
}
/// A compact single-line control for a header's navigation row, sized by its text.
pub(super) fn tab<A: Component>(
    p: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    title: &str,
    action: A,
) -> Entity {
    p.spawn((
        Button,
        action,
        Control {
            primary: false,
            selected: false,
            enabled: true,
        },
        Node {
            height: Val::Px(30.),
            padding: UiRect::horizontal(Val::Px(16.)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(4.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(SURFACE),
        BorderColor::all(EDGE),
    ))
    .with_children(|p| {
        label(p, font, title, 14., GOLD);
    })
    .id()
}
pub(super) fn hover(
    mut controls: Query<(
        &Interaction,
        &Control,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (interaction, style, mut background, mut border) in &mut controls {
        let hot = style.enabled && *interaction != Interaction::None;
        let color = if !style.enabled {
            Color::srgb(0.055, 0.05, 0.043)
        } else if hot && style.primary {
            Color::srgb(0.41, 0.105, 0.055)
        } else if style.primary {
            Color::srgb(0.29, 0.075, 0.043)
        } else if hot || style.selected {
            Color::srgb(0.15, 0.12, 0.075)
        } else {
            SURFACE
        };
        if background.0 != color {
            background.0 = color;
        }
        let edge = if hot || style.selected || style.primary {
            GOLD
        } else {
            EDGE
        };
        if *border != BorderColor::all(edge) {
            *border = BorderColor::all(edge);
        }
    }
}
