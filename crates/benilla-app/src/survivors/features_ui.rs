//! The menu's FEATURES panel: one row per appearance dial (skin colour, face, hair style, hair
//! colour, facial hair), each `label  <  n / N  >`, and a Randomize under them. The dials are kept
//! per class on [`super::Selection::features`]; the clicks are [`MenuAction::Feature`] and
//! [`MenuAction::RandomFeatures`] in [`super::menu::menu_clicks`].
use bevy::prelude::*;

use super::appearance::{self, DIALS};
use super::front_end as skin;
use super::menu::MenuAction;

/// The panel's height on the 768-unit canvas: heading, five rows and the Randomize.
pub(super) const HEIGHT: f32 = 208.;
const ROW: f32 = 22.;

#[derive(Component)]
pub(super) struct FeatureLabel(usize);

#[derive(Component)]
pub(super) struct FeatureCount(usize);

/// Spawn the panel at `node`'s place.
pub(super) fn build(
    ui: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    node: Node,
    heading: impl Fn(&mut ChildSpawnerCommands, &Handle<Font>, &str),
) {
    ui.spawn(skin::panel(Node {
        height: Val::Px(HEIGHT),
        padding: UiRect::axes(Val::Px(16.), Val::Px(12.)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: Val::Px(4.),
        ..node
    }))
    .with_children(|p| {
        heading(p, f, "FEATURES");
        for dial in 0..DIALS {
            p.spawn(Node {
                width: Val::Percent(100.),
                height: Val::Px(ROW),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.),
                flex_shrink: 0.,
                ..default()
            })
            .with_children(|row| {
                let label = skin::label(row, f, "", 13., skin::TEXT);
                row.commands().entity(label).insert((
                    FeatureLabel(dial),
                    Node {
                        flex_grow: 1.,
                        ..default()
                    },
                ));
                arrow(row, f, "<", MenuAction::Feature(dial, false));
                let count = skin::label(row, f, "", 12., skin::MUTED);
                row.commands().entity(count).insert((
                    FeatureCount(dial),
                    Node {
                        width: Val::Px(48.),
                        ..default()
                    },
                    TextLayout::new_with_justify(Justify::Center),
                ));
                arrow(row, f, ">", MenuAction::Feature(dial, true));
            });
        }
        p.spawn(Node {
            flex_grow: 1.,
            ..default()
        });
        skin::tab(p, f, "Randomize features", MenuAction::RandomFeatures);
    });
}

/// A small square step button, painted by [`skin::hover`].
fn arrow(row: &mut ChildSpawnerCommands, f: &Handle<Font>, glyph: &str, action: MenuAction) {
    row.spawn((
        Button,
        action,
        skin::Control {
            primary: false,
            selected: false,
            enabled: true,
        },
        Node {
            width: Val::Px(24.),
            height: Val::Px(ROW),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(4.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(skin::SURFACE),
        BorderColor::all(skin::EDGE),
    ))
    .with_children(|p| {
        skin::label(p, f, glyph, 14., skin::GOLD);
    });
}

/// The rows' labels (the race's own word for the dial) and `n / N` counts, and which arrows can
/// step: a dial with one value, or a race the tables do not cover, is greyed.
#[allow(clippy::type_complexity)]
pub(super) fn refresh(
    selection: Res<super::Selection>,
    creatures: Option<Res<crate::entities::Creatures>>,
    appearances: Option<Res<appearance::Appearances>>,
    catalog: Option<Res<crate::entities::CharCreate>>,
    mut labels: Query<(&FeatureLabel, &mut Text), Without<FeatureCount>>,
    mut counts: Query<(&FeatureCount, &mut Text), Without<FeatureLabel>>,
    mut arrows: Query<(&MenuAction, &mut skin::Control)>,
    mut seen: Local<Option<(u32, bool, bool)>>,
) {
    let key = (selection.version, creatures.is_some(), appearances.is_some());
    if *seen == Some(key) && labels.iter().all(|(_, t)| !t.0.is_empty()) {
        return;
    }
    *seen = Some(key);
    let cur = creatures
        .as_deref()
        .and_then(|c| appearance::current(&selection, c, appearances.as_deref()));
    let race_sex = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map(|&(_, r, s)| (r, s));
    let mut steps = [0usize; DIALS];
    let mut text = [String::new(), String::new(), String::new(), String::new(), String::new()];
    for dial in 0..DIALS {
        let (n, total) = cur.as_ref().map_or((0, 0), |(_, f, ranges)| {
            let opts = ranges.options(dial, f);
            let at = opts.iter().position(|&v| v == f[dial]).unwrap_or(0);
            (at + 1, opts.len())
        });
        steps[dial] = total;
        text[dial] = if total == 0 {
            "-".into()
        } else {
            format!("{n} / {total}")
        };
    }
    for (label, mut t) in &mut labels {
        let want = race_sex.map_or("", |(r, s)| {
            appearance::dial_label(catalog.as_deref(), label.0, r, s)
        });
        let want = if want.is_empty() {
            ["Skin Color", "Face", "Hair Style", "Hair Color", "Facial Hair"][label.0]
        } else {
            want
        };
        if t.0 != want {
            t.0 = want.into();
        }
    }
    for (count, mut t) in &mut counts {
        if t.0 != text[count.0] {
            t.0.clone_from(&text[count.0]);
        }
    }
    for (action, mut control) in &mut arrows {
        let enabled = match *action {
            MenuAction::Feature(dial, _) => steps[dial] > 1,
            MenuAction::RandomFeatures => steps.iter().any(|&n| n > 1),
            _ => continue,
        };
        if control.enabled != enabled {
            control.enabled = enabled;
        }
    }
}
