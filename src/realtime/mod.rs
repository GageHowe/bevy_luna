mod node;
mod prepare;
mod selection;
mod shared;

use crate::RaytracePlugins;
use bevy::{
    app::{App, Plugin, PostUpdate},
    camera::Camera3d,
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass, NormalPrepass},
    ecs::{component::Component, prelude::ReflectComponent},
    light::{
        DirectionalLight, PointLight, SimulationLightSystems, SpotLight,
        cluster::VisibleClusterableObjects,
    },
    pbr::DefaultOpaqueRendererMethod,
    prelude::*,
    reflect::{Reflect, std_traits::ReflectDefault},
    render::{
        Render, RenderApp, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        renderer::RenderDevice,
        view::{Hdr, Msaa},
    },
};
use node::{load_internal_assets, setup_render_app};
use prepare::{
    GpuDirectionalLight, GpuPunctualLight, prepare_raytrace_output_textures,
    prepare_raytrace_view_lights,
};
use selection::{
    pack_directional_light, pack_point_light, pack_spot_light, push_directional_light,
    push_punctual_light,
};
use shared::{MAX_DIRECTIONAL_LIGHTS, MAX_PUNCTUAL_LIGHTS};

/// Runtime settings used to enable or disable the raytraced path.
#[derive(Resource, Clone, Debug, Reflect, ExtractResource)]
#[reflect(Resource, Default, Clone)]
pub struct RaytraceSettings {
    /// Selects the active shadowing model for managed cameras.
    pub mode: RaytraceMode,
    /// Default quality preset used when a managed view is activated.
    pub quality: RaytraceQuality,
    /// Optional debug output mode.
    pub debug: RaytraceDebugMode,
}

impl Default for RaytraceSettings {
    fn default() -> Self {
        Self {
            mode: RaytraceMode::RaytracedShadows,
            quality: RaytraceQuality::Balanced,
            debug: RaytraceDebugMode::None,
        }
    }
}

/// Runtime rendering mode for managed views.
#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq)]
pub enum RaytraceMode {
    /// Use the raytraced shadowing path.
    RaytracedShadows,
    /// Use Bevy's normal rasterized shadowing path.
    Bevy,
}

impl Default for RaytraceMode {
    fn default() -> Self {
        Self::RaytracedShadows
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

/// Opts a camera out of automatic raytraced view management.
#[derive(Component, Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, PartialEq)]
#[require(Camera3d)]
pub struct DisableRaytraceView;

/// Opts a supported light out of automatic raytraced shadow ownership.
#[derive(Component, Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, PartialEq)]
pub struct DisableRaytraceLight;

/// Optional directional-light baseline override for the raytraced path.
///
/// By default, `bevy_luna` captures the authored Bevy light value once and uses
/// that as the baseline when switching between Bevy and raytraced modes.
/// Attach this component only if you need to pin a different directional-light
/// baseline for raytraced mode.
#[derive(Component, Clone, Copy, Debug, Default, Reflect, PartialEq)]
#[reflect(Component, Default, PartialEq)]
pub struct RaytraceDirectionalLight {
    /// Illuminance in lux used by the raytraced path.
    pub illuminance: f32,
}

/// Optional point/spot baseline override for the raytraced path.
///
/// By default, `bevy_luna` captures the authored Bevy light value once and uses
/// that as the baseline when switching between Bevy and raytraced modes.
/// Attach this component only if you need to pin a different point/spot light
/// baseline for raytraced mode.
#[derive(Component, Clone, Copy, Debug, Default, Reflect, PartialEq)]
#[reflect(Component, Default, PartialEq)]
pub struct RaytracePunctualLight {
    /// Luminous power/intensity scalar used by the raytraced path.
    pub intensity: f32,
}

/// Per-camera raytracing configuration.
#[derive(Component, Clone, Debug, Reflect, ExtractComponent)]
#[reflect(Component, Default, Clone)]
#[require(Hdr, DeferredPrepass, DepthPrepass, NormalPrepass)]
pub struct RaytraceView {
    /// Per-camera quality preset.
    pub quality: RaytraceQuality,
    /// Future debug visualization mode.
    pub debug: RaytraceDebugMode,
}

impl Default for RaytraceView {
    fn default() -> Self {
        Self {
            quality: RaytraceQuality::Balanced,
            debug: RaytraceDebugMode::None,
        }
    }
}

#[derive(Component, Clone, Debug, Default, ExtractComponent)]
pub struct RaytraceLightSelection {
    pub directional_lights: [GpuDirectionalLight; MAX_DIRECTIONAL_LIGHTS],
    pub directional_light_count: u32,
    pub punctual_lights: [GpuPunctualLight; MAX_PUNCTUAL_LIGHTS],
    pub punctual_light_count: u32,
}

/// Hardware capability summary used to decide whether the plugin can activate tracing.
#[derive(Resource, Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
#[reflect(Resource, Default, PartialEq)]
pub struct RaytraceCapabilities {
    /// Whether the active adapter supports the hardware ray-query path.
    pub hardware_ray_query: bool,
}

#[derive(Component, Clone, Copy, Debug, Default)]
struct CapturedDirectionalBaseline {
    illuminance: f32,
}

#[derive(Component, Clone, Copy, Debug, Default)]
struct CapturedPunctualBaseline {
    intensity: f32,
}

/// Realtime-side systems for managed views, extraction, and validation.
pub struct RaytraceViewPlugin;

impl Plugin for RaytraceViewPlugin {
    fn build(&self, app: &mut App) {
        load_internal_assets(app);
        app.insert_resource(DefaultOpaqueRendererMethod::deferred())
            .register_type::<RaytraceSettings>()
            .register_type::<RaytraceMode>()
            .register_type::<RaytraceQuality>()
            .register_type::<RaytraceDebugMode>()
            .register_type::<RaytraceCapabilities>()
            .register_type::<DisableRaytraceView>()
            .register_type::<DisableRaytraceLight>()
            .register_type::<RaytraceDirectionalLight>()
            .register_type::<RaytracePunctualLight>()
            .register_type::<RaytraceView>()
            .init_resource::<RaytraceSettings>()
            .init_resource::<RaytraceCapabilities>()
            .add_plugins((
                ExtractComponentPlugin::<RaytraceView>::default(),
                ExtractComponentPlugin::<RaytraceLightSelection>::default(),
                ExtractResourcePlugin::<RaytraceSettings>::default(),
            ))
            .add_systems(
                PostUpdate,
                (
                    sync_managed_views,
                    sync_supported_light_baselines
                        .after(sync_managed_views)
                        .before(restore_supported_lights_for_clustering),
                    restore_supported_lights_for_clustering
                        .after(sync_managed_views)
                        .before(SimulationLightSystems::AssignLightsToClusters),
                    apply_supported_light_render_mode
                        .after(SimulationLightSystems::AssignLightsToClusters),
                    sync_relevant_lights
                        .after(apply_supported_light_render_mode)
                        .after(SimulationLightSystems::AssignLightsToClusters),
                    validate_raytrace_views,
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
                "bevy_luna loaded on an adapter without hardware ray-query support. Managed raytraced views will stay disabled and Bevy rasterization will continue normally. Missing features: {:?}",
                RaytracePlugins::required_wgpu_features().difference(render_device.features())
            );
            return;
        }

        let render_app = app
            .get_sub_app_mut(RenderApp)
            .expect("render app should still be available");
        setup_render_app(render_app);
        render_app.add_systems(
            Render,
            (
                prepare_raytrace_output_textures,
                prepare_raytrace_view_lights,
            )
                .in_set(RenderSystems::PrepareResources),
        );
    }
}

fn sync_managed_views(
    mut commands: Commands,
    capabilities: Res<RaytraceCapabilities>,
    settings: Res<RaytraceSettings>,
    mut managed_views: Query<
        (Entity, Option<&mut RaytraceView>),
        (With<Camera3d>, Without<DisableRaytraceView>),
    >,
) {
    for (entity, view) in &mut managed_views {
        if settings.mode == RaytraceMode::RaytracedShadows && capabilities.hardware_ray_query {
            let next_view = RaytraceView {
                quality: settings.quality,
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

fn sync_supported_light_baselines(
    mut commands: Commands,
    directional_lights: Query<
        (
            Entity,
            &DirectionalLight,
            Option<&CapturedDirectionalBaseline>,
        ),
        Without<DisableRaytraceLight>,
    >,
    point_lights: Query<
        (
            Entity,
            &PointLight,
            Option<&CapturedPunctualBaseline>,
        ),
        (Without<SpotLight>, Without<DisableRaytraceLight>),
    >,
    spot_lights: Query<
        (
            Entity,
            &SpotLight,
            Option<&CapturedPunctualBaseline>,
        ),
        (Without<PointLight>, Without<DisableRaytraceLight>),
    >,
) {
    for (entity, light, baseline) in &directional_lights {
        if baseline.is_none() {
            commands.entity(entity).insert(CapturedDirectionalBaseline {
                illuminance: light.illuminance,
            });
        }
    }

    for (entity, light, baseline) in &point_lights {
        if baseline.is_none() {
            commands.entity(entity).insert(CapturedPunctualBaseline {
                intensity: light.intensity,
            });
        }
    }

    for (entity, light, baseline) in &spot_lights {
        if baseline.is_none() {
            commands.entity(entity).insert(CapturedPunctualBaseline {
                intensity: light.intensity,
            });
        }
    }
}

fn restore_supported_lights_for_clustering(
    settings: Res<RaytraceSettings>,
    mut directional_lights: Query<
        (
            &mut DirectionalLight,
            &CapturedDirectionalBaseline,
            Option<&RaytraceDirectionalLight>,
        ),
        Without<DisableRaytraceLight>,
    >,
    mut point_lights: Query<
        (
            &mut PointLight,
            &CapturedPunctualBaseline,
            Option<&RaytracePunctualLight>,
        ),
        (Without<SpotLight>, Without<DisableRaytraceLight>),
    >,
    mut spot_lights: Query<
        (
            &mut SpotLight,
            &CapturedPunctualBaseline,
            Option<&RaytracePunctualLight>,
        ),
        (Without<PointLight>, Without<DisableRaytraceLight>),
    >,
) {
    if settings.mode != RaytraceMode::RaytracedShadows {
        return;
    }

    for (mut light, baseline, override_light) in &mut directional_lights {
        light.illuminance = override_light.map_or(baseline.illuminance, |value| value.illuminance);
    }

    for (mut light, baseline, override_light) in &mut point_lights {
        light.intensity = override_light.map_or(baseline.intensity, |value| value.intensity);
    }

    for (mut light, baseline, override_light) in &mut spot_lights {
        light.intensity = override_light.map_or(baseline.intensity, |value| value.intensity);
    }
}

fn apply_supported_light_render_mode(
    settings: Res<RaytraceSettings>,
    mut directional_lights: Query<
        (
            &mut DirectionalLight,
            &CapturedDirectionalBaseline,
            Option<&RaytraceDirectionalLight>,
        ),
        Without<DisableRaytraceLight>,
    >,
    mut point_lights: Query<
        (
            &mut PointLight,
            &CapturedPunctualBaseline,
            Option<&RaytracePunctualLight>,
        ),
        (Without<SpotLight>, Without<DisableRaytraceLight>),
    >,
    mut spot_lights: Query<
        (
            &mut SpotLight,
            &CapturedPunctualBaseline,
            Option<&RaytracePunctualLight>,
        ),
        (Without<PointLight>, Without<DisableRaytraceLight>),
    >,
) {
    let raytraced = settings.mode == RaytraceMode::RaytracedShadows;

    for (mut light, baseline, override_light) in &mut directional_lights {
        light.illuminance = if raytraced {
            0.0
        } else {
            override_light.map_or(baseline.illuminance, |value| value.illuminance)
        };
        light.shadows_enabled = !raytraced;
    }

    for (mut light, baseline, override_light) in &mut point_lights {
        light.intensity = if raytraced {
            0.0
        } else {
            override_light.map_or(baseline.intensity, |value| value.intensity)
        };
        light.shadows_enabled = !raytraced;
    }

    for (mut light, baseline, override_light) in &mut spot_lights {
        light.intensity = if raytraced {
            0.0
        } else {
            override_light.map_or(baseline.intensity, |value| value.intensity)
        };
        light.shadows_enabled = !raytraced;
    }
}

fn sync_relevant_lights(
    mut commands: Commands,
    managed_views: Query<
        (
            Entity,
            Option<&RaytraceView>,
            Option<&VisibleClusterableObjects>,
        ),
        With<Camera3d>,
    >,
    directional_lights: Query<(
        &DirectionalLight,
        &GlobalTransform,
        Option<&CapturedDirectionalBaseline>,
        Option<&RaytraceDirectionalLight>,
    ), Without<DisableRaytraceLight>>,
    point_lights: Query<(
        &PointLight,
        &GlobalTransform,
        Option<&CapturedPunctualBaseline>,
        Option<&RaytracePunctualLight>,
    ), (Without<SpotLight>, Without<DisableRaytraceLight>)>,
    spot_lights: Query<
        (
            &SpotLight,
            &GlobalTransform,
            Option<&CapturedPunctualBaseline>,
            Option<&RaytracePunctualLight>,
        ),
        (Without<PointLight>, Without<DisableRaytraceLight>),
    >,
) {
    for (entity, raytrace_view, visible_lights) in &managed_views {
        if raytrace_view.is_none() {
            commands.entity(entity).remove::<RaytraceLightSelection>();
            continue;
        }

        let mut selection = RaytraceLightSelection::default();

        for (light, transform, captured, override_light) in &directional_lights {
            let effective_override = override_light.copied().or_else(|| {
                captured.map(|captured| RaytraceDirectionalLight {
                    illuminance: captured.illuminance,
                })
            });
            if let Some(light) = pack_directional_light(light, transform, effective_override.as_ref()) {
                push_directional_light(&mut selection, light);
            }
        }

        let mut scored = visible_lights
            .map(|visible_lights| {
                visible_lights
                    .entities
                    .iter()
                    .filter_map(|light_entity| {
                        if let Ok((light, transform, captured, override_light)) =
                            point_lights.get(*light_entity)
                        {
                            let effective_override = override_light.copied().or_else(|| {
                                captured.map(|captured| RaytracePunctualLight {
                                    intensity: captured.intensity,
                                })
                            });
                            return pack_point_light(light, transform, effective_override.as_ref());
                        }

                        let (light, transform, captured, override_light) =
                            spot_lights.get(*light_entity).ok()?;
                        let effective_override = override_light.copied().or_else(|| {
                            captured.map(|captured| RaytracePunctualLight {
                                intensity: captured.intensity,
                            })
                        });
                        pack_spot_light(light, transform, effective_override.as_ref())
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        scored.sort_by(|a, b| b.0.total_cmp(&a.0));

        for (_, light) in scored {
            push_punctual_light(&mut selection, light);
        }

        commands.entity(entity).insert(selection);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::realtime::shared::{
        TRACE_WGSL_MAX_DIRECTIONAL_LIGHTS, TRACE_WGSL_MAX_PUNCTUAL_LIGHTS,
    };

    #[test]
    fn settings_default_to_raytraced_mode() {
        let settings = RaytraceSettings::default();
        assert_eq!(settings.mode, RaytraceMode::RaytracedShadows);
        assert_eq!(settings.quality, RaytraceQuality::Balanced);
    }

    #[test]
    fn mode_default_matches_settings_default() {
        assert_eq!(RaytraceMode::default(), RaytraceMode::RaytracedShadows);
    }

    #[test]
    fn capabilities_default_to_disabled() {
        assert!(!RaytraceCapabilities::default().hardware_ray_query);
    }

    #[test]
    fn trace_shader_light_limits_match_rust_constants() {
        let shader = include_str!("trace.wgsl");
        assert!(shader.contains(TRACE_WGSL_MAX_DIRECTIONAL_LIGHTS));
        assert!(shader.contains(TRACE_WGSL_MAX_PUNCTUAL_LIGHTS));
    }
}
