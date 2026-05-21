#![allow(missing_docs, reason = "Example entrypoint")]

use bevy::{
    pbr::MeshMaterial3d, prelude::*,
};
use bevy_raytrace::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RaytracePlugins)
        .insert_resource(RaytraceSettings {
            enabled: true,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_raytracing, toggle_debug_view, animate_lights))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(GlobalAmbientLight {
        brightness: 6.0,
        ..default()
    });

    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.015, 0.02, 0.03)),
            ..default()
        },
        Transform::from_xyz(6.4, 4.0, 7.2).looking_at(Vec3::new(0.0, 1.1, 0.0), Vec3::Y),
        RaytraceManagedView,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(18.0, 18.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.16, 0.18, 0.21),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            ..default()
        })),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.35).mesh().uv(48, 32))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.90, 0.28, 0.18),
            perceptual_roughness: 0.62,
            metallic: 0.0,
            reflectance: 0.25,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.35, 0.0),
    ));

    commands.spawn((
        PointLight {
            intensity: 420_000.0,
            color: Color::srgb(1.0, 0.72, 0.54),
            range: 18.0,
            radius: 0.08,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(2.6, 2.8, 1.3),
        OrbitingPointLight,
    ));

    commands.spawn((
        SpotLight {
            intensity: 520_000.0,
            color: Color::srgb(0.60, 0.78, 1.0),
            range: 18.0,
            inner_angle: 0.24,
            outer_angle: 0.42,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-3.4, 4.6, 3.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        SweepingSpotLight,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 24_000.0,
            color: Color::srgb(1.0, 0.96, 0.92),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.62, -0.92, 0.0)),
    ));
}

#[derive(Component)]
struct OrbitingPointLight;

#[derive(Component)]
struct SweepingSpotLight;

fn animate_lights(
    time: Res<Time>,
    mut point_lights: Query<
        &mut Transform,
        (
            With<PointLight>,
            With<OrbitingPointLight>,
            Without<SpotLight>,
            Without<SweepingSpotLight>,
        ),
    >,
    mut spot_lights: Query<
        &mut Transform,
        (
            With<SpotLight>,
            With<SweepingSpotLight>,
            Without<PointLight>,
            Without<OrbitingPointLight>,
        ),
    >,
) {
    let orbit_angle = time.elapsed_secs() * 0.75;
    for mut transform in &mut point_lights {
        transform.translation = Vec3::new(orbit_angle.cos() * 2.9, 2.8, orbit_angle.sin() * 2.1);
    }

    let sweep = time.elapsed_secs() * 0.9;
    for mut transform in &mut spot_lights {
        let position = Vec3::new(-3.4, 4.6, 3.0);
        let target = Vec3::new(sweep.sin() * 1.4, 1.0, sweep.cos() * 1.2);
        *transform = Transform::from_translation(position).looking_at(target, Vec3::Y);
    }
}

fn toggle_raytracing(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<RaytraceSettings>) {
    if keys.just_pressed(KeyCode::KeyR) {
        settings.enabled = !settings.enabled;
        info!("raytracing enabled: {}", settings.enabled);
    }
}

fn toggle_debug_view(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<RaytraceSettings>) {
    if keys.just_pressed(KeyCode::KeyG) {
        settings.debug = match settings.debug {
            RaytraceDebugMode::None => RaytraceDebugMode::DirectLighting,
            RaytraceDebugMode::DirectLighting => RaytraceDebugMode::ShadowMask,
            _ => RaytraceDebugMode::None,
        };
        info!("raytrace debug mode: {:?}", settings.debug);
    }
}
