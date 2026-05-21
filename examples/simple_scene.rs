#![allow(missing_docs, reason = "Example entrypoint")]

use bevy::{
    pbr::MeshMaterial3d, prelude::*,
};
use bevy_raytrace::prelude::*;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
enum ShadowRenderMode {
    #[default]
    Bevy,
    Raytraced,
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
        brightness: 18.0,
        ..default()
    });

    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.03, 0.04, 0.06)),
            ..default()
        },
        Transform::from_xyz(0.0, 4.0, 9.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        RaytraceManagedView,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 35_000.0,
            color: Color::srgb(1.0, 0.96, 0.90),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.72, -0.92, 0.0)),
        RaytraceDirectionalLight {
            illuminance: 35_000.0,
        },
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.24, 0.26, 0.30),
            perceptual_roughness: 0.98,
            ..default()
        })),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.4).mesh().uv(48, 32))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.86, 0.17, 0.10),
            perceptual_roughness: 0.42,
            metallic: 0.0,
            reflectance: 0.35,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.4, 0.0),
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
