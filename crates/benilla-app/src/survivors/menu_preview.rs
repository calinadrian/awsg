//! Render the selected, dressed hero in a plain black character panel.
//! The world and Adventure atlas retain their cameras and map selection.
use super::{hero::Hero, hud::MenuRoot, menu_skin as skin, Phase};
use bevy::{
    camera::{visibility::RenderLayers, RenderTarget},
    prelude::*,
};

// Past the engine's allocated model-pane layers; this camera sees only the menu hero.
const LAYER: usize =
    crate::portrait::UI_MODEL_CAM_LAYER_BASE + crate::portrait::UI_MODEL_CAM_LAYERS;

#[derive(Resource)]
struct PreviewImage(Handle<Image>);
#[derive(Component)]
struct PreviewCamera;
#[derive(Component)]
struct PreviewPane;
#[derive(Component)]
struct OriginalLayers(Option<RenderLayers>);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup).add_systems(
        PostUpdate,
        sync.before(bevy::transform::TransformSystems::Propagate),
    );
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = images.add(crate::portrait::new_target_image_sized(768, 1024));
    commands.insert_resource(PreviewImage(image.clone()));
    commands.spawn((
        PreviewCamera,
        crate::portrait::booth_view_shape(),
        Camera {
            is_active: false,
            // Composite only the model; an opaque clear becomes a visible rectangle
            // through the preview's separate rendering and UI colour paths.
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

pub(super) fn build(ui: &mut ChildSpawnerCommands) {
    ui.spawn(skin::panel(Node {
        width: Val::Percent(100.),
        flex_grow: 1.,
        flex_basis: Val::Px(0.),
        min_height: Val::Px(100.),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        overflow: Overflow::clip(),
        ..default()
    }))
    .insert((BackgroundColor(Color::BLACK), bevy::ui::FocusPolicy::Pass))
    .with_children(|p| {
        p.spawn((
            PreviewPane,
            ImageNode::default(),
            bevy::ui::FocusPolicy::Pass,
            Node {
                height: Val::Percent(100.),
                aspect_ratio: Some(0.75),
                max_width: Val::Percent(100.),
                min_width: Val::Px(0.),
                ..default()
            },
        ));
    });
}

#[allow(clippy::type_complexity)]
fn sync(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    target: Res<PreviewImage>,
    roots: Query<&Visibility, With<MenuRoot>>,
    heroes: Query<(Entity, &Transform), (With<Hero>, Without<PreviewCamera>)>,
    children: Query<&Children>,
    layers: Query<&RenderLayers>,
    originals: Query<(Entity, &OriginalLayers)>,
    mut cameras: Query<(&mut Camera, &mut Transform), (With<PreviewCamera>, Without<Hero>)>,
    mut panes: Query<&mut ImageNode, With<PreviewPane>>,
) {
    let active = *phase.get() == Phase::Menu && roots.iter().any(|v| *v != Visibility::Hidden);
    let hero = heroes.iter().next();
    for (mut camera, mut transform) in &mut cameras {
        camera.is_active = active && hero.is_some();
        if let Some((_, pose)) = hero {
            let focus = pose.translation + Vec3::Y * 1.25;
            *transform = Transform::from_translation(focus + Vec3::new(-0.6, 0.3, -5.1))
                .looking_at(focus, Vec3::Y);
        }
    }
    for mut pane in &mut panes {
        pane.image = target.0.clone();
    }
    if active {
        if let Some((hero, _)) = hero {
            for entity in std::iter::once(hero).chain(children.iter_descendants(hero)) {
                if originals.get(entity).is_err() {
                    commands.entity(entity).insert((
                        OriginalLayers(layers.get(entity).ok().cloned()),
                        RenderLayers::layer(LAYER),
                    ));
                }
            }
        }
    } else {
        // Restore exact prior masks before gameplay uses the hero again.
        for (entity, original) in &originals {
            let mut e = commands.entity(entity);
            if let Some(layers) = &original.0 {
                e.insert(layers.clone());
            } else {
                e.remove::<RenderLayers>();
            }
            e.remove::<OriginalLayers>();
        }
    }
}
