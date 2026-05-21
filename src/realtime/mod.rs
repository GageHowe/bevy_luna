mod node;
mod prepare;

use crate::RaytracePlugins;
use bevy::{
    app::{App, Plugin, PostUpdate},
    camera::Camera3d,
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass, NormalPrepass},
    ecs::{component::Component, prelude::ReflectComponent},
    light::{cluster::VisibleClusterableObjects, PointLight, SimulationLightSystems, SpotLight},
    pbr::DefaultOpaqueRendererMethod,
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
use prepare::{
    GpuPunctualLight, MAX_PUNCTUAL_LIGHTS, prepare_raytrace_output_textures,
    prepare_raytrace_view_lights,
};

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
            mode: RaytraceMode::Bevy,
            quality: RaytraceQuality::Balanced,
            debug: RaytraceDebugMode::None,
        }
    }
}

/// Runtime rendering mode for managed views.
#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
pub enum RaytraceMode {
    /// Use Bevy's normal rasterized shadowing path.
    #[default]
    Bevy,
    /// Use the raytraced shadowing path.
    RaytracedShadows,
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
            .register_type::<RaytraceManagedView>()
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
                    sync_relevant_lights
                        .after(sync_managed_views)
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
            (prepare_raytrace_output_textures, prepare_raytrace_view_lights)
                .in_set(RenderSystems::PrepareResources),
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

fn sync_relevant_lights(
    mut commands: Commands,
    managed_views: Query<
        (
            Entity,
            Option<&RaytraceView>,
            Option<&VisibleClusterableObjects>,
            &GlobalTransform,
        ),
        With<RaytraceManagedView>,
    >,
    point_lights: Query<(&PointLight, &GlobalTransform)>,
    spot_lights: Query<(&SpotLight, &GlobalTransform)>,
) {
    for (entity, raytrace_view, visible_lights, camera_transform) in &managed_views {
        if raytrace_view.is_none() {
            commands.entity(entity).remove::<RaytraceLightSelection>();
            continue;
        }

        let camera_position = camera_transform.translation();
        let mut scored = visible_lights
            .map(|visible_lights| {
                visible_lights
                    .entities
                    .iter()
                    .filter_map(|light_entity| {
                        if let Ok((light, transform)) = point_lights.get(*light_entity) {
                            let distance_sq =
                                camera_position.distance_squared(transform.translation()).max(1.0);
                            let score = (light.intensity * light.range * light.range) / distance_sq;
                            return Some((
                                score,
                                GpuPunctualLight {
                                    position_range: transform.translation().extend(light.range),
                                    color_intensity: light
                                        .color
                                        .to_linear()
                                        .to_vec3()
                                        .extend(light.intensity),
                                    direction_cos_outer: Vec4::ZERO,
                                    params: Vec4::ZERO,
                                },
                            ));
                        }

                        let (light, transform) = spot_lights.get(*light_entity).ok()?;
                        let distance_sq =
                            camera_position.distance_squared(transform.translation()).max(1.0);
                        let score = (light.intensity * light.range * light.range) / distance_sq;
                        let direction = transform.forward().as_vec3();
                        let cos_inner = light.inner_angle.cos();
                        let cos_outer = light.outer_angle.cos();
                        let inverse_angle_range = 1.0 / (cos_inner - cos_outer).max(1e-4);
                        Some((
                            score,
                            GpuPunctualLight {
                                position_range: transform.translation().extend(light.range),
                                color_intensity: light
                                    .color
                                    .to_linear()
                                    .to_vec3()
                                    .extend(light.intensity),
                                direction_cos_outer: direction.extend(cos_outer),
                                params: Vec4::new(inverse_angle_range, -cos_outer * inverse_angle_range, 1.0, 0.0),
                            },
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        scored.sort_by(|a, b| b.0.total_cmp(&a.0));

        let mut selection = RaytraceLightSelection::default();
        for (_, light) in scored {
            if (selection.punctual_light_count as usize) < MAX_PUNCTUAL_LIGHTS {
                selection.punctual_lights[selection.punctual_light_count as usize] = light;
                selection.punctual_light_count += 1;
            }
        }

        commands.entity(entity).insert(selection);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_to_a_safe_runtime_toggle_off_state() {
        let settings = RaytraceSettings::default();
        assert_eq!(settings.mode, RaytraceMode::Bevy);
        assert_eq!(settings.quality, RaytraceQuality::Balanced);
    }

    #[test]
    fn capabilities_default_to_disabled() {
        assert!(!RaytraceCapabilities::default().hardware_ray_query);
    }
}
