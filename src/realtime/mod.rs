mod node;
mod prepare;

use crate::RaytracePlugins;
use bevy::{
    app::{App, Plugin, PostUpdate},
    camera::Camera3d,
    core_pipeline::prepass::{DepthPrepass, NormalPrepass},
    ecs::{component::Component, prelude::ReflectComponent},
    prelude::*,
    reflect::{Reflect, std_traits::ReflectDefault},
    render::{
        RenderApp,
        Render, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        renderer::RenderDevice,
        view::{Hdr, Msaa},
    },
};
use node::{load_internal_assets, setup_render_app};
use prepare::{RaytraceOutputTexture, prepare_raytrace_output_textures};

/// Runtime settings used to enable or disable the raytraced path.
#[derive(Resource, Clone, Debug, Reflect, ExtractResource)]
#[reflect(Resource, Default, Clone)]
pub struct RaytraceSettings {
    /// Enables the crate for managed cameras.
    pub enabled: bool,
    /// Enables raytraced direct lighting.
    pub direct_lighting: bool,
    /// Default quality preset used when a managed view is activated.
    pub quality: RaytraceQuality,
    /// The maximum ray distance in world units.
    pub max_ray_distance: f32,
    /// Optional debug output mode.
    pub debug: RaytraceDebugMode,
}

impl Default for RaytraceSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            direct_lighting: true,
            quality: RaytraceQuality::Balanced,
            max_ray_distance: 200.0,
            debug: RaytraceDebugMode::None,
        }
    }
}

/// Simple quality presets for the eventual real-time renderer.
#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
pub enum RaytraceQuality {
    /// Lowest-cost settings.
    Performance,
    /// Middle-ground settings intended for gameplay.
    #[default]
    Balanced,
    /// Highest-cost settings intended for screenshots or slow scenes.
    Quality,
}

/// Debug visualizations for future lighting passes.
#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
pub enum RaytraceDebugMode {
    /// Final shaded output.
    #[default]
    None,
    /// Visualize raw binary shadow visibility for the first active light.
    ShadowMask,
    /// Visualize future direct-light results.
    DirectLighting,
}

/// Marker for cameras whose `RaytraceView` should be managed by `RaytraceSettings`.
#[derive(Component, Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, PartialEq)]
#[require(Camera3d)]
pub struct RaytraceManagedView;

/// Per-camera raytracing configuration.
#[derive(Component, Clone, Debug, Reflect, ExtractComponent)]
#[reflect(Component, Default, Clone)]
#[require(Hdr, DepthPrepass, NormalPrepass)]
pub struct RaytraceView {
    /// Enables direct lighting for this camera.
    pub direct_lighting: bool,
    /// Per-camera quality preset.
    pub quality: RaytraceQuality,
    /// Maximum ray distance in world units.
    pub max_ray_distance: f32,
    /// Future debug visualization mode.
    pub debug: RaytraceDebugMode,
}

impl Default for RaytraceView {
    fn default() -> Self {
        Self {
            direct_lighting: true,
            quality: RaytraceQuality::Balanced,
            max_ray_distance: 200.0,
            debug: RaytraceDebugMode::None,
        }
    }
}

/// Render-world diagnostics gathered by the placeholder MVP pipeline.
#[derive(Resource, Clone, Debug, Default, Reflect, PartialEq, Eq)]
#[reflect(Resource, Default, PartialEq)]
pub struct RenderRaytraceDiagnostics {
    /// Number of active extracted raytraced views in the render world.
    pub active_views: usize,
    /// Number of per-view output textures allocated this frame.
    pub prepared_outputs: usize,
}

/// Hardware capability summary used to decide whether the plugin can activate tracing.
#[derive(Resource, Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
#[reflect(Resource, Default, PartialEq)]
pub struct RaytraceCapabilities {
    /// Whether the active adapter supports the hardware ray-query path.
    pub hardware_ray_query: bool,
}

/// Realtime-side systems for managed views, extraction, and validation.
pub struct RaytraceViewPlugin;

impl Plugin for RaytraceViewPlugin {
    fn build(&self, app: &mut App) {
        load_internal_assets(app);
        app.register_type::<RaytraceSettings>()
            .register_type::<RaytraceQuality>()
            .register_type::<RaytraceDebugMode>()
            .register_type::<RaytraceCapabilities>()
            .register_type::<RaytraceManagedView>()
            .register_type::<RaytraceView>()
            .init_resource::<RaytraceSettings>()
            .init_resource::<RaytraceCapabilities>()
            .init_resource::<RenderRaytraceDiagnostics>()
            .add_plugins((
                ExtractComponentPlugin::<RaytraceView>::default(),
                ExtractResourcePlugin::<RaytraceSettings>::default(),
            ))
            .add_systems(
                PostUpdate,
                (
                    sync_managed_views,
                    validate_raytrace_views,
                    prepare_raytrace_diagnostics,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(hardware_ray_query) = ({
            let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
                return;
            };
            let render_device = render_app.world().resource::<RenderDevice>();
            let features = render_device.features();
            Some(features.contains(RaytracePlugins::required_wgpu_features()))
        }) else {
            return;
        };
        app.insert_resource(RaytraceCapabilities { hardware_ray_query });

        if !hardware_ray_query {
            let render_app = app
                .get_sub_app(RenderApp)
                .expect("render app should still be available");
            let render_device = render_app.world().resource::<RenderDevice>();
            warn!(
                "bevy_raytrace loaded on an adapter without hardware ray-query support. Managed raytraced views will stay disabled and Bevy rasterization will continue normally. Missing features: {:?}",
                RaytracePlugins::required_wgpu_features()
                    .difference(render_device.features())
            );
            return;
        }

        let render_app = app
            .get_sub_app_mut(RenderApp)
            .expect("render app should still be available");
        setup_render_app(render_app);
        render_app.add_systems(
            Render,
            prepare_raytrace_output_textures.in_set(RenderSystems::PrepareResources),
        );
    }
}

fn sync_managed_views(
    mut commands: Commands,
    capabilities: Res<RaytraceCapabilities>,
    settings: Res<RaytraceSettings>,
    mut managed_views: Query<(Entity, Option<&mut RaytraceView>), With<RaytraceManagedView>>,
) {
    for (entity, view) in &mut managed_views {
        if settings.enabled && capabilities.hardware_ray_query {
            let next_view = RaytraceView {
                direct_lighting: settings.direct_lighting,
                quality: settings.quality,
                max_ray_distance: settings.max_ray_distance,
                debug: settings.debug,
            };

            if let Some(mut current_view) = view {
                *current_view = next_view;
            } else {
                commands.entity(entity).insert(next_view);
            }
        } else if view.is_some() {
            commands.entity(entity).remove::<RaytraceView>();
        }
    }
}

fn validate_raytrace_views(raytrace_views: Query<(Entity, &Msaa), With<RaytraceView>>) {
    for (entity, msaa) in &raytrace_views {
        if *msaa != Msaa::Off {
            warn!("RaytraceView on entity {entity} requires Msaa::Off");
        }
    }
}

fn prepare_raytrace_diagnostics(
    mut diagnostics: ResMut<RenderRaytraceDiagnostics>,
    raytrace_views: Query<&RaytraceView>,
    output_textures: Query<&RaytraceOutputTexture>,
) {
    diagnostics.active_views = raytrace_views.iter().count();
    diagnostics.prepared_outputs = output_textures.iter().count();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_to_a_safe_runtime_toggle_off_state() {
        let settings = RaytraceSettings::default();
        assert!(!settings.enabled);
        assert!(settings.direct_lighting);
    }

    #[test]
    fn capabilities_default_to_disabled() {
        assert!(!RaytraceCapabilities::default().hardware_ray_query);
    }
}
