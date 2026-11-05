//! The survivors grade: a light ReShade-style polish (CAS sharpening, vibrance, a soft S-curve, a
//! touch of warmth) on the world camera, after the FFXGlow filter passes and before the HUD, which
//! composites over it untouched. The Graphics menu and `G` share a saved setting;
//! `?grade=0` (`WOW_GRADE=0` native) overrides it for the session.
//!
//! It is Bevy's [`FullscreenMaterial`]: one uniform, one fragment shader
//! (`shaders/survivors_grade.wgsl`), and a node the plugin places between FFXGlow and the end of
//! the world's post-processing.

use bevy::anti_alias::fxaa::{Fxaa, FxaaPlugin};
use bevy::core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy::core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_graph::{
    InternedRenderLabel, InternedRenderSubGraph, RenderGraphExt, RenderLabel, RenderSubGraph,
};
use bevy::render::render_resource::ShaderType;
use bevy::render::RenderApp;
use bevy::shader::ShaderRef;

use benilla_world::view::WorldCamera;

use crate::cvars::{CvarChanged, Cvars};

/// The grade's uniform, on the world camera. `amount` 0 is the identity, and the toggle eases it.
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub(super) struct Grade {
    pub amount: f32,
    /// CAS sharpness, 0..1.
    pub sharpness: f32,
    /// Extra saturation for the least saturated pixels.
    pub vibrance: f32,
    /// How much of the S-curve to blend in.
    pub contrast: f32,
}

impl Default for Grade {
    fn default() -> Self {
        Self {
            amount: 0.0,
            sharpness: 0.4,
            vibrance: 0.18,
            contrast: 0.12,
        }
    }
}

impl FullscreenMaterial for Grade {
    fn fragment_shader() -> ShaderRef {
        "embedded://benilla_app/shaders/survivors_grade.wgsl".into()
    }

    fn sub_graph() -> Option<InternedRenderSubGraph> {
        Some(Core3d.intern())
    }

    /// After the glow's filter passes, before the world's post-processing ends.
    fn node_edges() -> Vec<InternedRenderLabel> {
        vec![
            Node3d::Tonemapping.intern(),
            Self::node_label().intern(),
            Node3d::EndMainPassPostProcessing.intern(),
        ]
    }
}

/// Whether the grade is wanted; the camera's `amount` eases toward it.
#[derive(Resource)]
struct GradeOn(bool);

pub(super) fn plugin(app: &mut App) {
    let on = crate::webenv::var("WOW_GRADE").is_none_or(|v| v != "0");
    app.add_plugins((FullscreenMaterialPlugin::<Grade>::default(), FxaaPlugin))
        .insert_resource(GradeOn(on))
        .add_observer(on_cvar)
        .add_systems(Startup, apply_grade_override.before(crate::cvars::CvarLoad))
        .add_systems(Update, (attach, toggle, drive, sync_fxaa).chain());
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render.add_render_graph_edges(Core3d, (Grade::node_label(), Node3d::Fxaa));
    }
}

fn apply_grade_override(mut cvars: ResMut<Cvars>) {
    if let Some(value) = crate::webenv::var("WOW_GRADE") {
        cvars.own_for_session("SurvivorWorldGrade", Some(if value == "0" { "0" } else { "1" }));
    }
}

fn on_cvar(event: On<CvarChanged>, mut on: ResMut<GradeOn>) {
    if event.is("SurvivorWorldGrade") {
        on.0 = event.flag();
    }
}

fn attach(
    mut commands: Commands,
    cams: Query<Entity, (With<WorldCamera>, With<Camera3d>, Without<Grade>)>,
) {
    for e in &cams {
        commands.entity(e).insert((
            Grade::default(),
            Fxaa {
                enabled: false,
                ..default()
            },
        ));
    }
}

fn sync_fxaa(cvars: Res<Cvars>, mut cameras: Query<&mut Fxaa, With<WorldCamera>>) {
    let enabled = cvars.flag("SurvivorFxaa").unwrap_or(false);
    for mut fxaa in &mut cameras {
        if fxaa.enabled != enabled {
            fxaa.enabled = enabled;
        }
    }
}

fn toggle(keys: Res<ButtonInput<KeyCode>>, mut on: ResMut<GradeOn>, mut cvars: ResMut<Cvars>) {
    if keys.just_pressed(KeyCode::KeyG) {
        on.0 = !on.0;
        cvars.set("SurvivorWorldGrade", if on.0 { "1" } else { "0" });
    }
}

/// Eases `amount` over a quarter second.
fn drive(time: Res<Time<Real>>, on: Res<GradeOn>, mut grades: Query<&mut Grade>) {
    let target = if on.0 { 1.0 } else { 0.0 };
    let k = 1.0 - (-time.delta_secs() * 12.0).exp();
    for mut g in &mut grades {
        g.amount += (target - g.amount) * k;
        if (g.amount - target).abs() < 1e-3 {
            g.amount = target;
        }
    }
}
