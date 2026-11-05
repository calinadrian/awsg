//! Edit the actual HUD widgets; only empty widgets substitute non-persistent sample data.
use super::{
    frame_settings::{self, FrameSettings, Placement},
    hud::HudFont,
    hud_layout::{HudLayout, Movable, Stacked, MENU_Z},
    options::Options,
    right_column as col,
};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const PANELS: [&str; 10] = [
    "minimap",
    "tracker",
    "meter",
    "action_bar",
    "boss_bars",
    "boss_cooldowns",
    "encounter_announcer",
    "announcements",
    "item_drops",
    "test_grounds",
];

#[derive(Resource, Default)]
pub(super) struct UiEditor {
    pub active: bool,
    pub requested: bool,
    pub closing: bool,
    layout_before: Option<HudLayout>,
    frames_before: Option<[Placement; 2]>,
    return_to_settings: bool,
}
#[derive(Component)]
struct EditorRoot;
#[derive(Component)]
struct EditorLayer(Option<GlobalZIndex>);
#[derive(Component)]
pub(super) struct EditorButton;
#[derive(Component, Clone, Copy)]
enum Action {
    Save,
    Cancel,
    Reset,
}

pub(super) fn inactive(editor: Res<UiEditor>) -> bool {
    !editor.active
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<UiEditor>()
        .add_systems(Update, session.before(frame_settings::input))
        .add_systems(
            Update,
            contain.after(session).before(super::hud_layout::input),
        )
        .add_systems(
            Update,
            block_world
                .after(super::hud_layout::input)
                .before(benilla_world::schedule::WorldStage::Input),
        )
        .add_systems(
            PostUpdate,
            layers
                .after(super::hud_layout::apply)
                .before(bevy::ui::UiSystems::Layout),
        );
}

fn contain(
    editor: Res<UiEditor>,
    mut layout: ResMut<HudLayout>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scale: Res<UiScale>,
    panels: Query<(
        &Movable,
        &ComputedNode,
        &UiGlobalTransform,
        &UiTransform,
        &InheritedVisibility,
    )>,
) {
    if !editor.active {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let canvas = Vec2::new(window.width(), window.height()) / scale.0.max(0.01);
    if canvas.min_element() <= 0. {
        return;
    }
    for (panel, node, at, transform, visible) in &panels {
        if panel.is_settings() || !visible.get() || node.size().min_element() <= 0. {
            continue;
        }
        let inv = node.inverse_scale_factor();
        let size = node.size() * inv;
        let px = |v| if let Val::Px(n) = v { n } else { 0. };
        let translation = Vec2::new(px(transform.translation.x), px(transform.translation.y));
        let drawn = size * at.matrix2.x_axis.length();
        let shift = (size - drawn) / 2. + translation;
        let position = at.translation * inv - translation - size / 2.;
        layout.constrain_preview(panel.id, position, drawn, shift, canvas);
    }
}

fn button(p: &mut ChildSpawnerCommands, font: &Handle<Font>, title: &str, action: Action) {
    p.spawn((
        Button,
        EditorButton,
        action,
        Node {
            padding: UiRect::axes(Val::Px(14.), Val::Px(8.)),
            border: UiRect::all(Val::Px(1.)),
            ..default()
        },
        BackgroundColor(col::KNOB),
        BorderColor::all(col::STEEL),
    ))
    .with_child(col::label(font, title, 12., col::TEXT_HI));
}

fn session(
    mut commands: Commands,
    mut editor: ResMut<UiEditor>,
    mut options: ResMut<Options>,
    mut layout: ResMut<HudLayout>,
    mut frames: ResMut<FrameSettings>,
    font: Option<Res<HudFont>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scale: Res<UiScale>,
    roots: Query<Entity, With<EditorRoot>>,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    editor.closing = false;
    if editor.active {
        let action = if keys.just_pressed(KeyCode::Escape) {
            Some(Action::Cancel)
        } else {
            buttons
                .iter()
                .find_map(|(i, a)| (*i == Interaction::Pressed).then_some(*a))
        };
        if let Some(action) = action {
            if matches!(action, Action::Reset) {
                layout.reset_positions(PANELS.into_iter());
                frames.reset_edit();
            } else {
                if matches!(action, Action::Cancel) {
                    if let Some(before) = editor.layout_before.take() {
                        *layout = before;
                    }
                    if let Some(before) = editor.frames_before.take() {
                        frames.cancel_edit(before);
                    }
                } else {
                    frames.finish_edit();
                }
                layout.finish_edit();
                editor.active = false;
                editor.closing = true;
                editor.layout_before = None;
                editor.frames_before = None;
                options.open = true;
                options.settings = editor.return_to_settings;
                for entity in &roots {
                    commands.entity(entity).despawn();
                }
            }
        }
        return;
    }
    if !editor.requested {
        return;
    }
    let (Some(font), Ok(window)) = (font, windows.single()) else {
        return;
    };
    let canvas = Vec2::new(window.width(), window.height()) / scale.0.max(0.01);
    if canvas.min_element() <= 0. {
        return;
    }
    editor.layout_before = Some(layout.clone());
    editor.frames_before = Some(frames.items);
    editor.return_to_settings = options.settings;
    editor.active = true;
    editor.requested = false;
    layout.edit(PANELS.into_iter());
    frames.begin_edit();
    options.settings = false;
    let f = &font.0;
    commands
        .spawn((
            EditorRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.025, 0.035, 0.92)),
            GlobalZIndex(MENU_Z + 5),
            bevy::ui::FocusPolicy::Block,
            Interaction::None,
        ))
        .with_children(|root| {
            for x in (0..canvas.x as u32).step_by(32) {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(x as f32),
                        width: Val::Px(1.),
                        height: Val::Percent(100.),
                        ..default()
                    },
                    BackgroundColor(Color::WHITE.with_alpha(0.035)),
                    Pickable::IGNORE,
                ));
            }
            for y in (0..canvas.y as u32).step_by(32) {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(y as f32),
                        height: Val::Px(1.),
                        width: Val::Percent(100.),
                        ..default()
                    },
                    BackgroundColor(Color::WHITE.with_alpha(0.035)),
                    Pickable::IGNORE,
                ));
            }
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(12.),
                    bottom: Val::Px(40.),
                    max_width: Val::Percent(90.),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.)),
                    row_gap: Val::Px(8.),
                    border: UiRect::all(Val::Px(1.)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.025, 0.035, 0.045)),
                BorderColor::all(col::STEEL),
                GlobalZIndex(MENU_Z + 40),
            ))
            .with_children(|toolbar| {
                toolbar.spawn(col::label(f, "EDIT UI", 14., col::TEXT_HI));
                toolbar.spawn(col::label(
                    f,
                    "Drag the real panels · Empty panels show sample data",
                    11.,
                    col::TEXT_DIM,
                ));
                toolbar.spawn(col::label(
                    f,
                    "Right-click for options · Escape cancels",
                    11.,
                    col::TEXT_DIM,
                ));
                toolbar
                    .spawn(Node {
                        column_gap: Val::Px(8.),
                        ..default()
                    })
                    .with_children(|row| {
                        button(row, f, "Save layout", Action::Save);
                        button(row, f, "Cancel", Action::Cancel);
                        button(row, f, "Reset positions", Action::Reset);
                    });
            });
        });
}

/// Lift real roots and explicitly layered descendants over the editor backdrop.
fn layers(
    mut commands: Commands,
    editor: Res<UiEditor>,
    layout: Res<HudLayout>,
    roots: Query<(Entity, Option<&Movable>, Option<&Stacked>), Or<(With<Movable>, With<Stacked>)>>,
    children: Query<&Children>,
    zs: Query<&GlobalZIndex>,
    lifted: Query<(Entity, &EditorLayer)>,
) {
    if !editor.active {
        for (entity, layer) in &lifted {
            let mut e = commands.entity(entity);
            e.remove::<EditorLayer>();
            if let Some(z) = layer.0 {
                e.insert(z);
            } else {
                e.remove::<GlobalZIndex>();
            }
        }
        return;
    }
    for (root, movable, stacked) in &roots {
        if movable.is_some_and(Movable::is_settings) {
            continue;
        }
        let id = movable
            .map(|m| m.id)
            .or_else(|| stacked.map(|s| s.0))
            .unwrap_or("");
        let z = GlobalZIndex(layout.editor_z(id));
        for entity in std::iter::once(root).chain(children.iter_descendants(root)) {
            let old = zs.get(entity).ok().copied();
            if entity != root && old.is_none() {
                continue;
            }
            if lifted.get(entity).is_err() {
                commands.entity(entity).insert(EditorLayer(old));
            }
            if old != Some(z) {
                commands.entity(entity).insert(z);
            }
        }
    }
}

fn block_world(
    editor: Res<UiEditor>,
    mut hover: ResMut<crate::ui_script::PlayerUiHover>,
    mut pointer: ResMut<crate::ui_script::PointerOverUi>,
    mut panel: ResMut<crate::ui_script::PointerOverUiPanel>,
    mut consumed: ResMut<crate::ui_script::PlayerUiClickConsumed>,
) {
    const TOKEN: u32 = u32::MAX - 66;
    if editor.active {
        hover.0 = Some(TOKEN);
        pointer.0 = true;
        panel.0 = true;
        consumed.0 = true;
    } else if hover.0 == Some(TOKEN) {
        hover.0 = None;
        pointer.0 = false;
        panel.0 = false;
    }
}
