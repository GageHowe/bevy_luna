//! Experimental real-time raytraced shadow replacement for Bevy without temporal history.
//!
//! Add [`RaytracePlugins`] to enable the raytraced path on supported hardware.
//! Omit the plugin to keep plain Bevy rendering.

mod realtime;
mod scene;

pub use realtime::{
    RaytraceCapabilities, RaytraceDebugMode, RaytraceDirectionalLight, RaytraceManagedView,
    RaytraceMode, RaytracePunctualLight, RaytraceQuality, RaytraceSettings, RaytraceView,
    RaytraceViewPlugin,
};
pub use scene::{DisableRaytracingMesh, RaytraceScenePlugin, RaytracingMesh3d};

use bevy::app::{PluginGroup, PluginGroupBuilder};
use bevy::render::settings::WgpuFeatures;
use bevy_solari::SolariPlugins;

/// Prelude exports for users of `bevy_raytrace`.
pub mod prelude {
    pub use crate::{
        DisableRaytracingMesh, RaytraceDebugMode, RaytraceDirectionalLight, RaytraceManagedView,
        RaytracePlugins, RaytraceCapabilities, RaytraceMode, RaytracePunctualLight,
        RaytraceQuality, RaytraceScenePlugin, RaytraceSettings, RaytraceView,
        RaytraceViewPlugin, RaytracingMesh3d,
    };
}

/// Plugin group for the raytracing scene and view systems.
pub struct RaytracePlugins;

impl PluginGroup for RaytracePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(RaytraceScenePlugin)
            .add(RaytraceViewPlugin)
    }
}

impl RaytracePlugins {
    /// Required wgpu features for the current hardware ray-query backend.
    pub fn required_wgpu_features() -> WgpuFeatures {
        SolariPlugins::required_wgpu_features()
    }
}
