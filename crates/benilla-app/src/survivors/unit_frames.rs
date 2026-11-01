//! Classic unit frames using art from the installed patch chain.
mod classic;
use super::{
    Run,
    enemies::Enemy,
    frame_settings::{self, FrameSettings},
    hud::{HudFont, HudRoot, UiArt},
};
use crate::portrait::{PortraitImages, PortraitSource};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui_render::ui_material::{MaterialNode, UiMaterial};

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct PortraitMaterial {
    #[texture(0)]
    #[sampler(1)]
    image: Handle<Image>,
}

impl UiMaterial for PortraitMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://benilla_app/survivors/unit_portrait.wgsl".into()
    }
}

pub(super) fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "unit_portrait.wgsl");
    app.add_plugins(bevy::ui_render::UiMaterialPlugin::<PortraitMaterial>::default())
        .add_plugins(frame_settings::plugin)
        .add_systems(
            Update,
            (
                spawn,
                refresh,
                refresh_portraits,
                classic::refresh_portraits,
                refresh_power,
            )
                .chain(),
        );
    #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
    if std::env::var_os("WOW_SURVIVORS_FRAME_PREVIEW").is_some() {
        app.add_systems(Update, preview_subjects.before(refresh));
    }
}

#[derive(Component)]
struct UnitFrame(bool);
#[derive(Component)]
struct FrameName(bool);
#[derive(Component)]
struct FrameHealth(bool);
#[derive(Component)]
struct FrameFill(bool);
#[derive(Component)]
struct FramePortrait(bool, Handle<PortraitMaterial>);
#[derive(Component)]
struct FrameLevel(bool);
#[derive(Component)]
struct MoveOutline(bool);
#[derive(Component)]
struct DesignRoot(bool, u8);
#[derive(Component)]
struct PowerNode(bool, u8, bool);
#[derive(Component)]
struct FramePowerText(bool);

fn seat(x: f32, y: f32, w: f32, h: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(x),
        top: Val::Px(y),
        width: Val::Px(w),
        height: Val::Px(h),
        ..default()
    }
}

fn label(font: &Handle<Font>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::ONE,
            color: Color::BLACK,
        },
        TextLayout::new(Justify::Center, LineBreak::NoWrap),
    )
}

fn spawn(
    mut commands: Commands,
    roots: Query<Entity, Added<HudRoot>>,
    font: Option<Res<HudFont>>,
    mut materials: ResMut<Assets<PortraitMaterial>>,
    settings: Res<FrameSettings>,
) {
    let Some(font) = font else { return };
    let gold = Color::srgb(1., 0.82, 0.);
    for root in &roots {
        for target in [false, true] {
            commands.entity(root).with_children(|parent| {
                parent
                    .spawn((
                        UnitFrame(target),
                        seat(
                            settings.items[usize::from(target)].pos.x,
                            settings.items[usize::from(target)].pos.y,
                            0.,
                            0.,
                        ),
                        UiTransform::from_scale(Vec2::splat(
                            settings.items[usize::from(target)].scale,
                        )),
                    ))
                    .with_children(|frame| {
                        for design in 0..3u8 {
                            frame
                                .spawn((
                                    DesignRoot(target, design),
                                    seat(0., 0., 0., 0.),
                                    UiTransform::from_scale(Vec2::splat(if design == 2 {
                                        0.85
                                    } else {
                                        1.
                                    })),
                                ))
                                .with_children(|skin| {
                                    spawn_design(skin, &font.0, target, design, &mut materials)
                                });
                        }
                        frame.spawn((
                            MoveOutline(target),
                            Node {
                                border: UiRect::all(Val::Px(1.)),
                                display: Display::None,
                                ..seat(0., 0., 240., 76.)
                            },
                            BorderColor::all(gold),
                            ZIndex(8),
                        ));
                    });
            });
        }
    }
}

fn spawn_design(
    frame: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    target: bool,
    design: u8,
    materials: &mut Assets<PortraitMaterial>,
) {
    classic::spawn(frame, font, target, design, materials);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn refresh(
    run: Res<Run>,
    coop: Res<super::coop::Coop>,
    selection: Res<crate::target::Selection>,
    enemies: Query<&Enemy>,
    mut frames: Query<
        (&UnitFrame, &mut Node, &mut UiTransform),
        (
            Without<FrameFill>,
            Without<MoveOutline>,
            Without<DesignRoot>,
        ),
    >,
    mut fills: Query<
        (&FrameFill, &mut Node),
        (
            Without<UnitFrame>,
            Without<MoveOutline>,
            Without<DesignRoot>,
        ),
    >,
    mut outlines: Query<
        (&MoveOutline, &mut Node),
        (Without<UnitFrame>, Without<FrameFill>, Without<DesignRoot>),
    >,
    mut designs: Query<
        (&DesignRoot, &mut Node),
        (Without<UnitFrame>, Without<FrameFill>, Without<MoveOutline>),
    >,
    mut names: Query<(&FrameName, &mut Text), (Without<FrameHealth>, Without<FrameLevel>)>,
    mut health: Query<
        (&FrameHealth, &mut Text, &mut Visibility),
        (Without<FrameName>, Without<FrameLevel>),
    >,
    mut levels: Query<
        (&FrameLevel, &mut Text, &mut Visibility),
        (Without<FrameName>, Without<FrameHealth>),
    >,
    mut faces: Query<
        (&FramePortrait, &mut Visibility),
        (Without<FrameHealth>, Without<FrameLevel>),
    >,
    settings: Res<FrameSettings>,
) {
    let target = selection
        .target
        .and_then(|e| enemies.get(e).ok())
        .filter(|e| !e.dead && e.hp > 0.);
    let max = run.stats().max_hp;
    for (frame, mut node, mut transform) in &mut frames {
        let placement = settings.items[usize::from(frame.0)];
        node.left = Val::Px(placement.pos.x);
        node.top = Val::Px(placement.pos.y);
        transform.scale = Vec2::splat(placement.scale);
        node.display = if frame.0 && target.is_none() && placement.locked {
            Display::None
        } else {
            Display::Flex
        };
    }
    for (design, mut node) in &mut designs {
        node.display = if settings.items[usize::from(design.0)].design == design.1 {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (outline, mut node) in &mut outlines {
        node.display = if settings.items[usize::from(outline.0)].locked {
            Display::None
        } else {
            Display::Flex
        };
    }
    for (fill, mut node) in &mut fills {
        let (hp, max) = if fill.0 {
            target.map_or((0., 1.), |e| (e.hp, e.max_hp))
        } else {
            (run.hp, max)
        };
        node.width = Val::Percent((hp / max.max(1.)).clamp(0., 1.) * 100.);
    }
    for (name, mut text) in &mut names {
        let value = if name.0 {
            target.map_or("Select a target", |e| e.def.name)
        } else {
            if coop.running && !coop.name.trim().is_empty() {
                coop.name.trim()
            } else {
                run.class.name
            }
        };
        if text.0 != value {
            text.0 = value.to_string();
        }
    }
    for (value, mut text, mut visible) in &mut health {
        *visible = if settings.items[usize::from(value.0)].health {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let (hp, max) = if value.0 {
            target.map_or((0., 1.), |e| (e.hp, e.max_hp))
        } else {
            (run.hp, max)
        };
        text.0 = format!("{:.0} / {:.0}", hp.max(0.).ceil(), max.ceil());
    }
    for (level, mut text, mut visible) in &mut levels {
        *visible = if settings.items[usize::from(level.0)].level {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        text.0 = if level.0 {
            target.map_or(String::new(), |e| e.def.level.to_string())
        } else {
            run.level.to_string()
        };
    }
    for (face, mut visibility) in &mut faces {
        *visibility = if face.0 && target.is_none() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

/// Resolve both live model bakes and the portrait system's loading stand-ins.
fn refresh_portraits(
    portraits: Res<PortraitImages>,
    faces: Query<&FramePortrait>,
    mut materials: ResMut<Assets<PortraitMaterial>>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut files: Local<std::collections::HashMap<String, Handle<Image>>>,
) {
    for face in &faces {
        let handle = match portraits.0.get(if face.0 { "target" } else { "player" }) {
            Some(PortraitSource::Live(handle)) => Some(handle.clone()),
            Some(PortraitSource::File(path)) => {
                if let Some(handle) = files.get(path) {
                    Some(handle.clone())
                } else {
                    let handle = assets
                        .as_deref_mut()
                        .and_then(|assets| assets.sprite_texture(path, &mut images));
                    if let Some(handle) = &handle {
                        files.insert(path.clone(), handle.clone());
                    }
                    handle
                }
            }
            None => None,
        };
        if let (Some(handle), Some(material)) = (handle, materials.get_mut(&face.1)) {
            if material.image != handle {
                material.image = handle;
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn refresh_power(
    selection: Res<crate::target::Selection>,
    enemies: Query<&Enemy>,
    settings: Res<FrameSettings>,
    hero: Query<&crate::net::ObjectStore, With<super::hero::Hero>>,
    stores: Query<&crate::net::ObjectStore>,
    mut power_nodes: Query<
        (&PowerNode, &mut Node, Option<&mut BackgroundColor>),
        (
            Without<UnitFrame>,
            Without<FrameFill>,
            Without<MoveOutline>,
            Without<DesignRoot>,
        ),
    >,
    mut power_text: Query<
        (&FramePowerText, &mut Text, &mut Visibility),
        (
            Without<FrameName>,
            Without<FrameHealth>,
            Without<FrameLevel>,
            Without<FramePortrait>,
        ),
    >,
) {
    let target = selection
        .target
        .and_then(|e| enemies.get(e).ok())
        .filter(|e| !e.dead && e.hp > 0.);
    let resource = |store: Option<&crate::net::ObjectStore>| {
        let Some(store) = store else {
            return (0, 0, Color::NONE);
        };
        let kind = store.0.unit_power_type();
        let current = store.0.unit_shown_power(kind).unwrap_or(0);
        let maximum = store.0.unit_shown_max_power(kind).unwrap_or(0);
        let color = match kind {
            1 => Color::srgb(0.85, 0.06, 0.04),
            2 => Color::srgb(1., 0.5, 0.15),
            3 => Color::srgb(1., 0.85, 0.08),
            _ => Color::srgb(0.02, 0.3, 0.95),
        };
        (current, maximum, color)
    };
    let player_power = resource(hero.single().ok());
    let target_power = resource(
        selection
            .target
            .filter(|_| target.is_some())
            .and_then(|e| stores.get(e).ok()),
    );
    for (part, mut node, color) in &mut power_nodes {
        let (value, maximum, tint) = if part.0 { target_power } else { player_power };
        match part.1 {
            0 => {
                node.height = Val::Px(if maximum > 0 {
                    if part.2 { 49. } else { 56. }
                } else if part.2 {
                    39.
                } else {
                    45.
                })
            }
            1 => {
                node.display = if maximum > 0 {
                    Display::Flex
                } else {
                    Display::None
                }
            }
            _ => {
                node.width = Val::Percent(if maximum > 0 {
                    (value as f32 / maximum as f32).clamp(0., 1.) * 100.
                } else {
                    0.
                });
                if let Some(mut color) = color {
                    color.0 = tint;
                }
            }
        }
    }
    for (part, mut text, mut visibility) in &mut power_text {
        let (value, maximum, _) = if part.0 { target_power } else { player_power };
        *visibility = if maximum > 0 && settings.items[usize::from(part.0)].health {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        text.0 = format!("{value} / {maximum}");
    }
}

/// Opt-in visual QA with real local units, usable with the existing screenshot runner.
#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
fn preview_subjects(
    mut commands: Commands,
    mut selection: ResMut<crate::target::Selection>,
    hero: Query<Entity, (With<super::hero::Hero>, Without<crate::net::SelfPlayer>)>,
    enemies: Query<(Entity, &Enemy, &crate::net::Guid)>,
) {
    for body in &hero {
        commands.entity(body).insert(crate::net::SelfPlayer);
    }
    if let Some((entity, _, guid)) = enemies
        .iter()
        .filter(|(_, e, _)| !e.dead && e.hp > 0.)
        .max_by(|(_, a, _), (_, b, _)| a.max_hp.total_cmp(&b.max_hp))
    {
        selection.target = Some(entity);
        selection.guid = Some(guid.0);
    }
}
