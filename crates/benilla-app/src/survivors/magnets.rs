//! Elite magnet pickups collect all existing XP and gold; their mesh is authored here.
use super::{loot_ground::GroundedLoot, RunEntity};
use crate::net::Guid;
use bevy::prelude::*;

pub(super) const DROP_CHANCE: f32 = 0.20;

#[derive(Component)]
struct Magnet;

#[derive(Resource)]
struct Art {
    cube: Handle<Mesh>,
    red: Handle<StandardMaterial>,
    silver: Handle<StandardMaterial>,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup).add_systems(Update, attach);
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Art {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        red: materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.05, 0.08),
            unlit: true,
            ..default()
        }),
        silver: materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.9, 1.),
            unlit: true,
            ..default()
        }),
    });
}

pub(super) fn spawn(commands: &mut Commands, serial: u64, pos: Vec3) -> Entity {
    commands
        .spawn((
            Guid(serial),
            RunEntity,
            Magnet,
            GroundedLoot,
            Transform::from_translation(pos),
            Visibility::Hidden,
        ))
        .id()
}

fn attach(mut commands: Commands, art: Res<Art>, magnets: Query<Entity, Added<Magnet>>) {
    for entity in &magnets {
        commands.entity(entity).with_children(|root| {
            // A red U with silver poles, standing just above the ground.
            for (pos, size, silver) in [
                (Vec3::new(0., 0.18, 0.), Vec3::new(0.9, 0.22, 0.25), false),
                (
                    Vec3::new(-0.34, 0.48, 0.),
                    Vec3::new(0.22, 0.6, 0.25),
                    false,
                ),
                (Vec3::new(0.34, 0.48, 0.), Vec3::new(0.22, 0.6, 0.25), false),
                (
                    Vec3::new(-0.34, 0.86, 0.),
                    Vec3::new(0.22, 0.18, 0.25),
                    true,
                ),
                (Vec3::new(0.34, 0.86, 0.), Vec3::new(0.22, 0.18, 0.25), true),
            ] {
                root.spawn((
                    Mesh3d(art.cube.clone()),
                    MeshMaterial3d(if silver {
                        art.silver.clone()
                    } else {
                        art.red.clone()
                    }),
                    Transform::from_translation(pos).with_scale(size),
                ));
            }
        });
    }
}
