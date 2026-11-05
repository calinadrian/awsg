//! An isolated character using the normal unit visuals and equipment dressing path.
use super::{
    Phase, RunEntity, data,
    inventory::{Equipment, Pane},
    inventory_ui::ItemIcon,
    menu_skin as skin,
    units::{self, UnitSpec},
};
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
};

// The menu hero owns the preceding layer. Neither preview enters the world camera.
const LAYER: usize =
    crate::portrait::UI_MODEL_CAM_LAYER_BASE + crate::portrait::UI_MODEL_CAM_LAYERS + 1;

#[derive(Component, PartialEq, Eq)]
pub(super) struct ViewerModel {
    display: u32,
    class: u8,
    race: u8,
    sex: u8,
}
#[derive(Resource)]
struct Target(Handle<Image>);
#[derive(Component)]
struct PreviewCamera;
#[derive(Component)]
struct PreviewImage;
#[derive(Component)]
struct Status;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, prepare.before(crate::entities::EntityVisualsSet))
        .add_systems(
            PostUpdate,
            sync.before(bevy::transform::TransformSystems::Propagate),
        );
}
fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = images.add(crate::portrait::new_target_image_sized(600, 800));
    commands.insert_resource(Target(image.clone()));
    commands.spawn((
        PreviewCamera,
        crate::portrait::booth_view_shape(),
        Camera {
            is_active: false,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            order: -1,
            ..default()
        },
        RenderTarget::Image(image.into()),
        benilla_world::ffx_glow::FfxGlow::UI_PANE,
        Projection::Perspective(PerspectiveProjection {
            aspect_ratio: 0.75,
            fov: 0.65,
            ..default()
        }),
        RenderLayers::layer(LAYER),
        Transform::default(),
    ));
}

pub(super) fn build(p: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    p.spawn(skin::panel(Node {
        width: Val::Percent(100.),
        min_height: Val::Px(270.),
        flex_grow: 1.,
        flex_basis: Val::Px(0.),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        overflow: Overflow::clip(),
        ..default()
    }))
    .insert((BackgroundColor(Color::BLACK), bevy::ui::FocusPolicy::Pass))
    .with_children(|p| {
        p.spawn((
            PreviewImage,
            ImageNode::default(),
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                min_width: Val::Px(0.),
                flex_shrink: 0.,
                ..default()
            },
            bevy::ui::FocusPolicy::Pass,
        ));
        p.spawn((
            Status,
            Text::new("Loading character…"),
            TextFont {
                font: font.clone(),
                font_size: 12.,
                ..default()
            },
            TextColor(skin::MUTED),
            TextLayout::new_with_justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.),
                right: Val::Px(10.),
                ..default()
            },
        ));
    });
}

fn active(gear: &Equipment, phase: &State<Phase>) -> bool {
    *phase.get() == Phase::Menu && gear.open && gear.pane == Pane::Items
}

fn prepare(
    mut commands: Commands,
    gear: Res<Equipment>,
    phase: Res<State<Phase>>,
    creatures: Option<Res<crate::entities::Creatures>>,
    models: Query<(Entity, &ViewerModel)>,
    attached: Query<(), With<crate::entities::VisualAttached>>,
    heroes: Query<&Transform, With<super::hero::Hero>>,
    mut statuses: Query<&mut Text, With<Status>>,
) {
    let class = data::CLASSES
        .iter()
        .find(|class| class.id == gear.viewer_class);
    let wanted = if active(&gear, &phase) {
        class
            .zip(creatures.as_deref())
            .and_then(|(class, creatures)| {
                super::races::looks_for(class, creatures)
                    .into_iter()
                    .find(|look| Some(look.1) == gear.viewer_race && look.2 == gear.viewer_sex)
                    .map(|(display, race, sex)| ViewerModel {
                        display,
                        class: class.id,
                        race,
                        sex,
                    })
            })
    } else {
        None
    };
    for mut text in &mut statuses {
        let message = if let Some(wanted) = &wanted {
            if models
                .iter()
                .any(|(entity, model)| model == wanted && attached.contains(entity))
            {
                ""
            } else {
                "Loading character model…"
            }
        } else if creatures.is_none() {
            "Loading character…"
        } else {
            "This race / gender model is unavailable in your game data."
        };
        if text.0 != message {
            text.0 = message.into();
        }
    }
    let mut exists = false;
    for (entity, model) in &models {
        if wanted.as_ref() == Some(model) {
            exists = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if !exists {
        if let (Some(wanted), Some(class)) = (wanted, class) {
            let entity = units::spawn_unit(
                &mut commands,
                &UnitSpec {
                    serial: 0x0000_FFFE_FFFF_FFFE,
                    display: wanted.display,
                    scale: 1.,
                    level: 60,
                    faction: units::FACTION_FRIENDLY,
                    bytes0: u32::from(wanted.race)
                        | u32::from(wanted.class) << 8
                        | u32::from(wanted.sex) << 16
                        | u32::from(class.power) << 24,
                    weapons: &[],
                    sheath: 1,
                },
                // Keep the model near the scene's lighting origin. Its exclusive render layer
                // keeps it out of the world and the other character preview.
                heroes
                    .iter()
                    .next()
                    .map_or(Vec3::ZERO, |pose| pose.translation),
                0.,
            );
            commands.entity(entity).remove::<RunEntity>().insert((
                wanted,
                RenderLayers::layer(LAYER),
                crate::portrait::StageRig,
            ));
        }
    }
}

#[allow(clippy::type_complexity)]
fn sync(
    mut commands: Commands,
    gear: Res<Equipment>,
    phase: Res<State<Phase>>,
    target: Res<Target>,
    mut images: ResMut<Assets<Image>>,
    models: Query<(Entity, &Transform), (With<ViewerModel>, Without<PreviewCamera>)>,
    children: Query<&Children>,
    layers: Query<&RenderLayers>,
    bounds: Query<(&bevy::camera::primitives::Aabb, &GlobalTransform)>,
    mut cameras: Query<
        (&mut Camera, &mut Transform, &mut Projection),
        (With<PreviewCamera>, Without<ViewerModel>),
    >,
    sizes: Query<&ComputedNode, With<PreviewImage>>,
    mut panes: Query<&mut ImageNode, (With<PreviewImage>, Without<ItemIcon>)>,
) {
    let active = active(&gear, &phase) && !models.is_empty();
    let size = sizes
        .iter()
        .find(|node| node.size().min_element() > 0.)
        .map(|node| node.size().clamp(Vec2::splat(1.), Vec2::splat(2048.)));
    let aspect = size.map_or(0.75, |size| size.x / size.y);
    if let Some(size) = size {
        if let Some(image) = images.get_mut(&target.0) {
            let wanted = bevy::render::render_resource::Extent3d {
                width: size.x.round().max(1.) as u32,
                height: size.y.round().max(1.) as u32,
                depth_or_array_layers: 1,
            };
            if image.texture_descriptor.size != wanted {
                image.resize(wanted);
            }
        }
    }
    let position = models
        .iter()
        .next()
        .map_or(Vec3::ZERO, |(_, pose)| pose.translation);
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for (model, _) in &models {
        for entity in children.iter_descendants(model) {
            let Ok((aabb, world)) = bounds.get(entity) else {
                continue;
            };
            let center = world.transform_point(Vec3::from(aabb.center));
            // Newly attached meshes have no propagated world pose until the next frame.
            if center.distance(position) > 20. {
                continue;
            }
            for corner in 0..8 {
                let signs = Vec3::new(
                    if corner & 1 == 0 { -1. } else { 1. },
                    if corner & 2 == 0 { -1. } else { 1. },
                    if corner & 4 == 0 { -1. } else { 1. },
                );
                let point = world.transform_point(
                    Vec3::from(aabb.center) + Vec3::from(aabb.half_extents) * signs,
                );
                low = low.min(point);
                high = high.max(point);
            }
        }
    }
    let (focus, distance) = if low.is_finite() && high.is_finite() {
        let half = (high - low) * 0.5;
        let horizontal = Vec2::new(half.x, half.z).length();
        let distance = (half.y / 0.325_f32.tan()).max(horizontal / (0.325_f32.tan() * aspect));
        ((low + high) * 0.5, (distance * 1.15 + horizontal).max(1.5))
    } else {
        (position + Vec3::Y * 1.25, 5.1)
    };
    for (mut camera, mut transform, mut projection) in &mut cameras {
        camera.is_active = active;
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.aspect_ratio = aspect;
        }
        let direction =
            Quat::from_rotation_y(gear.viewer_yaw) * Vec3::new(-0.12, 0.06, -1.).normalize();
        *transform = Transform::from_translation(focus + direction * distance * gear.viewer_zoom)
            .looking_at(focus, Vec3::Y);
    }
    for mut pane in &mut panes {
        pane.image = target.0.clone();
    }
    if active {
        let mask = RenderLayers::layer(LAYER);
        // Dressing creates new attachments asynchronously; include them as soon as they exist.
        for (model, _) in &models {
            for entity in std::iter::once(model).chain(children.iter_descendants(model)) {
                if layers.get(entity).ok() != Some(&mask) {
                    commands
                        .entity(entity)
                        .insert((mask.clone(), bevy::camera::visibility::NoFrustumCulling));
                }
            }
        }
    }
}
