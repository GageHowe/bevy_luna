#![allow(missing_docs, reason = "Example entrypoint")]

use bevy::{
    pbr::MeshMaterial3d, prelude::*,
};
use bevy_raytrace::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RaytracePlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_raytracing)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(GlobalAmbientLight {
        brightness: 16.0,
        ..default()
    });

    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Transform::from_xyz(0.0, 4.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        RaytraceManagedView,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 28_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, -0.7, 0.0)),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.4, 0.4, 0.45))),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(2.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.2, 0.15),
            perceptual_roughness: 0.2,
            metallic: 0.8,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));
}

fn toggle_raytracing(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<RaytraceSettings>) {
    if keys.just_pressed(KeyCode::KeyR) {
        settings.enabled = !settings.enabled;
        info!("raytracing enabled: {}", settings.enabled);
    }
}
