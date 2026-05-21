#![allow(missing_docs, reason = "Example entrypoint")]

use bevy::{
    pbr::MeshMaterial3d, prelude::*,
};
use bevy_luna::prelude::*;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
enum ShadowRenderMode {
    #[default]
    Raytraced,
    Bevy,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RaytracePlugins)
        .init_resource::<ShadowRenderMode>()
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_shadow_mode, apply_shadow_mode).chain())
        .add_systems(Update, toggle_debug_view)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(RaytraceSettings::default());
    commands.insert_resource(GlobalAmbientLight {
        brightness: 5.0,
        ..default()
    });

    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.025, 0.03)),
            ..default()
        },
        Transform::from_xyz(8.0, 5.0, 8.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(24.0, 24.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.19, 0.22),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.4, 3.2, 1.4))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.84, 0.25, 0.14),
            perceptual_roughness: 0.72,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.6, 0.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.9, 0.9, 0.9))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.86, 0.82, 0.76),
            perceptual_roughness: 0.68,
            ..default()
        })),
        Transform::from_xyz(-2.2, 0.45, -1.3),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 45_000.0,
            color: Color::srgb(1.0, 0.97, 0.92),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.55, -1.05, 0.0)),
    ));
}

fn toggle_shadow_mode(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<ShadowRenderMode>) {
    if keys.just_pressed(KeyCode::KeyR) {
        *mode = match *mode {
            ShadowRenderMode::Bevy => ShadowRenderMode::Raytraced,
            ShadowRenderMode::Raytraced => ShadowRenderMode::Bevy,
        };
        info!("shadow mode: {:?}", *mode);
    }
}

fn apply_shadow_mode(
    mode: Res<ShadowRenderMode>,
    mut settings: ResMut<RaytraceSettings>,
) {
    let raytraced = *mode == ShadowRenderMode::Raytraced;
    settings.mode = if raytraced {
        RaytraceMode::RaytracedShadows
    } else {
        RaytraceMode::Bevy
    };
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
