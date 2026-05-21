#![allow(missing_docs, reason = "Example entrypoint")]

use bevy::{pbr::MeshMaterial3d, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(GlobalAmbientLight {
        brightness: 125.0,
        ..default()
    });

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.0, 9.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 18_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, -0.65, 0.0)),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.46, 0.50),
            perceptual_roughness: 0.92,
            ..default()
        })),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.4).mesh().uv(48, 32))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.82, 0.30, 0.18),
            perceptual_roughness: 0.18,
            metallic: 0.78,
            reflectance: 0.55,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.4, 0.0),
    ));
}
