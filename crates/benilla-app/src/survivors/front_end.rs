//! The mode's one skin: colours, the type scale, spacing, and the panels and controls every
//! survivors screen is built from.
use bevy::prelude::*;

pub(super) const INK: Color = Color::srgba(0.035, 0.032, 0.028, 0.96);
pub(super) const SURFACE: Color = Color::srgb(0.075, 0.065, 0.048);
pub(super) const EDGE: Color = Color::srgb(0.34, 0.27, 0.14);
pub(super) const GOLD: Color = Color::srgb(0.95, 0.78, 0.43);
pub(super) const TEXT: Color = Color::srgb(0.91, 0.87, 0.76);
pub(super) const MUTED: Color = Color::srgb(0.61, 0.59, 0.51);
pub(super) const GREEN: Color = Color::srgb(0.46, 0.78, 0.58);
pub(super) const RED: Color = Color::srgb(0.88, 0.38, 0.30);
pub(super) const ALLIANCE: Color = Color::srgb(0.42, 0.62, 0.95);
pub(super) const HORDE: Color = Color::srgb(0.92, 0.36, 0.3);
/// A recessed well: bag slots, search fields, empty bars.
pub(super) const WELL: Color = Color::srgb(0.035, 0.031, 0.025);
/// The fill of a selected, non-primary control.
pub(super) const SELECTED: Color = Color::srgb(0.15, 0.12, 0.075);

/// The type scale on the 768-unit canvas. Every survivors screen picks from these.
pub(super) mod size {
    pub(in super::super) const CAPTION: f32 = 10.;
    pub(in super::super) const SMALL: f32 = 12.;
    pub(in super::super) const BODY: f32 = 14.;
    pub(in super::super) const LEAD: f32 = 16.;
    pub(in super::super) const HEADING: f32 = 20.;
    pub(in super::super) const TITLE: f32 = 26.;
    pub(in super::super) const DISPLAY: f32 = 40.;
    pub(in super::super) const HERO: f32 = 56.;
}

/// Spacing steps on the 768-unit canvas.
pub(super) mod gap {
    pub(in super::super) const XS: f32 = 4.;
    pub(in super::super) const S: f32 = 8.;
    pub(in super::super) const M: f32 = 12.;
    pub(in super::super) const L: f32 = 16.;
    pub(in super::super) const XL: f32 = 24.;
}

/// The drop shadow under text drawn straight over the world.
pub(super) fn shadow() -> TextShadow {
    TextShadow {
        offset: Vec2::splat(1.5),
        color: Color::BLACK,
    }
}

/// An item quality's name colour, as the item tooltips colour it.
pub(super) fn quality_color(quality: &str) -> Color {
    let hex = match quality.to_ascii_lowercase().as_str() {
        "poor" | "0" => 0x9d9d9d,
        "uncommon" | "2" => 0x1eff00,
        "rare" | "3" => 0x0070dd,
        "epic" | "4" => 0xa335ee,
        "legendary" | "5" => 0xff8000,
        "artifact" | "heirloom" | "6" | "7" => 0xe6cc80,
        _ => 0xffffff,
    };
    Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// `n` with its noun, plural when it is not one: "1 talent point", "3 talent points".
pub(super) fn plural(n: impl Into<f64>, noun: &str) -> String {
    let n = n.into();
    let count = if n.fract() == 0. {
        format!("{n:.0}")
    } else {
        fmt_amount(n as f32)
    };
    if (n - 1.).abs() < f64::EPSILON {
        format!("{count} {noun}")
    } else if let Some(stem) = noun.strip_suffix('y').filter(|s| !s.ends_with(['a', 'e', 'o', 'u'])) {
        format!("{count} {stem}ies")
    } else {
        format!("{count} {noun}s")
    }
}

/// A number as a card or tooltip shows it: whole from 100 up, else one decimal, never a
/// trailing ".0".
pub(super) fn fmt_amount(x: f32) -> String {
    if x.abs() >= 100. {
        return format!("{:.0}", x.round());
    }
    let s = format!("{x:.1}");
    s.strip_suffix(".0").map_or(s.clone(), str::to_string)
}

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
            if primary { size::HEADING } else { size::LEAD },
            if enabled { GOLD } else { MUTED },
        );
        if !detail.is_empty() {
            label(p, font, detail, size::SMALL, if enabled { TEXT } else { MUTED });
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
    tab_state(p, font, title, action, false)
}
/// A [`tab`] that shows whether it is the open one: lit fill and a gold edge.
pub(super) fn tab_state<A: Component>(
    p: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    title: &str,
    action: A,
    selected: bool,
) -> Entity {
    p.spawn((
        Button,
        action,
        Control {
            primary: false,
            selected,
            enabled: true,
        },
        Node {
            height: Val::Px(30.),
            padding: UiRect::horizontal(Val::Px(gap::L)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(4.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(if selected { SELECTED } else { SURFACE }),
        BorderColor::all(if selected { GOLD } else { EDGE }),
    ))
    .with_children(|p| {
        label(p, font, title, size::BODY, if selected { GOLD } else { TEXT });
    })
    .id()
}
/// A panel's title, centred between two rules; the same on every panel.
pub(super) fn heading(p: &mut ChildSpawnerCommands, f: &Handle<Font>, text: &str) {
    p.spawn(Node {
        width: Val::Percent(100.),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(10.),
        flex_shrink: 0.,
        ..default()
    })
    .with_children(|row| {
        let line = || {
            (
                Node {
                    flex_grow: 1.,
                    height: Val::Px(1.),
                    ..default()
                },
                BackgroundColor(EDGE),
            )
        };
        row.spawn(line());
        label(row, f, text, size::LEAD, GOLD);
        row.spawn(line());
    });
}
/// A labelled value box; boxes in one row share its width equally. Returns the value's text,
/// for the caller to tag and fill.
pub(super) fn stat_tile(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    caption: &str,
    value: impl Into<String>,
    value_size: f32,
) -> Entity {
    let mut text = Entity::PLACEHOLDER;
    p.spawn((
        Node {
            flex_grow: 1.,
            flex_basis: Val::Px(0.),
            min_width: Val::Px(0.),
            padding: UiRect::axes(Val::Px(gap::S), Val::Px(6.)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(5.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(SURFACE),
        BorderColor::all(EDGE),
    ))
    .with_children(|b| {
        label(b, f, caption, size::CAPTION, MUTED);
        text = label(b, f, value, value_size, TEXT);
        b.commands()
            .entity(text)
            .insert(TextLayout::new_with_linebreak(LineBreak::NoWrap));
    });
    text
}
/// A thin bar filled to `fraction` (0..1) in `color`; returns the fill, whose width is a percent.
pub(super) fn progress_bar(p: &mut ChildSpawnerCommands, fraction: f32, color: Color, height: f32) -> Entity {
    let mut fill = Entity::PLACEHOLDER;
    p.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Px(height),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(height / 2.)),
            overflow: Overflow::clip(),
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(WELL),
        BorderColor::all(EDGE),
    ))
    .with_children(|bar| {
        fill = bar
            .spawn((
                Node {
                    width: Val::Percent(fraction.clamp(0., 1.) * 100.),
                    height: Val::Percent(100.),
                    ..default()
                },
                BackgroundColor(color),
            ))
            .id();
    });
    fill
}
/// A small rounded tag: a status, a count, a filter.
pub(super) fn chip(p: &mut ChildSpawnerCommands, f: &Handle<Font>, text: impl Into<String>, color: Color) -> Entity {
    p.spawn((
        Node {
            padding: UiRect::axes(Val::Px(gap::S), Val::Px(2.)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(9.)),
            flex_shrink: 0.,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.55)),
        BorderColor::all(color.with_alpha(0.7)),
    ))
    .with_children(|p| {
        label(p, f, text, size::SMALL, color);
    })
    .id()
}
/// A key and what it does, for the hint rows under a screen: `[ENTER] Play solo`.
pub(super) fn keycap(p: &mut ChildSpawnerCommands, f: &Handle<Font>, key: &str, what: &str) {
    p.spawn(Node {
        column_gap: Val::Px(6.),
        align_items: AlignItems::Center,
        flex_shrink: 0.,
        ..default()
    })
    .with_children(|p| {
        p.spawn((
            Node {
                padding: UiRect::axes(Val::Px(6.), Val::Px(1.)),
                border: UiRect::all(Val::Px(1.)),
                border_radius: BorderRadius::all(Val::Px(3.)),
                ..default()
            },
            BackgroundColor(SURFACE),
            BorderColor::all(EDGE),
        ))
        .with_children(|p| {
            label(p, f, key, size::CAPTION, GOLD);
        });
        label(p, f, what, size::SMALL, TEXT);
    });
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

#[cfg(test)]
mod tests {
    use super::{fmt_amount, plural};

    #[test]
    fn plurals_read_naturally() {
        assert_eq!(plural(1, "talent point"), "1 talent point");
        assert_eq!(plural(0, "talent point"), "0 talent points");
        assert_eq!(plural(3, "enemy"), "3 enemies");
        assert_eq!(plural(1, "enemy"), "1 enemy");
        assert_eq!(plural(2, "day"), "2 days");
    }

    #[test]
    fn amounts_drop_noise() {
        assert_eq!(fmt_amount(1480.2), "1480");
        assert_eq!(fmt_amount(156.8), "157");
        assert_eq!(fmt_amount(50.1), "50.1");
        assert_eq!(fmt_amount(6.0), "6");
        assert_eq!(fmt_amount(0.25), "0.2");
    }
}
