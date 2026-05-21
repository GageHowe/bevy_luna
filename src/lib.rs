//! Experimental real-time raytraced shadow replacement for Bevy without temporal history.
//!
//! Add [`RaytracePlugins`] to enable the raytraced path on supported hardware.
//! Omit the plugin to keep plain Bevy rendering.
//!
//! By default, all `Camera3d` views and supported directional/point/spot
//! lights are managed automatically. Use [`DisableRaytraceView`] or
//! [`DisableRaytraceLight`] to opt specific entities out.

mod realtime;
mod scene;

pub use realtime::{
    DisableRaytraceLight, DisableRaytraceView, RaytraceCapabilities, RaytraceDebugMode,
    RaytraceDirectionalLight, RaytraceMode, RaytracePunctualLight, RaytraceQuality,
    RaytraceSettings, RaytraceView, RaytraceViewPlugin,
};
pub use scene::{DisableRaytracingMesh, RaytraceScenePlugin, RaytracingMesh3d};

use bevy::app::{PluginGroup, PluginGroupBuilder};
use bevy::render::settings::WgpuFeatures;
use bevy_solari::SolariPlugins;

/// Prelude exports for users of `bevy_luna`.
pub mod prelude {
    pub use crate::{
        DisableRaytraceLight, DisableRaytraceView, DisableRaytracingMesh, RaytraceCapabilities,
        RaytraceDebugMode, RaytraceDirectionalLight, RaytraceMode, RaytracePlugins,
        RaytracePunctualLight, RaytraceQuality, RaytraceScenePlugin, RaytraceSettings,
        RaytraceView, RaytraceViewPlugin, RaytracingMesh3d,
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
