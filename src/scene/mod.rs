mod proxy;
mod types;

pub use types::{DisableRaytracingMesh, RaytracingMesh3d};

use bevy::app::{App, Plugin, Update};
use bevy_solari::scene::RaytracingScenePlugin as SolariRaytracingScenePlugin;
use proxy::{RaytraceProxyMeshes, tag_raytracing_meshes};

/// Scene-side systems for raytracing mesh tagging and validation.
pub struct RaytraceScenePlugin;

impl Plugin for RaytraceScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SolariRaytracingScenePlugin)
            .init_resource::<RaytraceProxyMeshes>()
            .add_systems(Update, tag_raytracing_meshes);
    }
}
