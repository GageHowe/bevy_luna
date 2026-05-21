use super::{
    RaytraceLightSelection, RaytraceView,
    shared::{MAX_DIRECTIONAL_LIGHTS, MAX_PUNCTUAL_LIGHTS},
};
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        system::{Commands, Query, Res},
    },
    image::ToExtents,
    math::Vec4,
    render::{
        camera::ExtractedCamera,
        render_resource::{
            ShaderType, Texture, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages, TextureView, TextureViewDescriptor, UniformBuffer,
        },
        renderer::RenderDevice,
    },
};
use bevy::render::renderer::RenderQueue;

#[derive(Clone, Copy, Debug, ShaderType, Default)]
pub struct GpuDirectionalLight {
    pub direction_to_light: Vec4,
    pub color_illuminance: Vec4,
}

#[derive(Clone, Copy, Debug, ShaderType, Default)]
pub struct GpuPunctualLight {
    pub position_range: Vec4,
    pub color_intensity: Vec4,
    pub direction_cos_outer: Vec4,
    pub params: Vec4,
}

#[derive(Clone, Copy, ShaderType)]
pub struct RaytraceLightsUniform {
    pub directional_lights: [GpuDirectionalLight; MAX_DIRECTIONAL_LIGHTS],
    pub directional_light_count: u32,
    pub _directional_padding: Vec4,
    pub punctual_lights: [GpuPunctualLight; MAX_PUNCTUAL_LIGHTS],
    pub punctual_light_count: u32,
    pub _padding: Vec4,
}

impl Default for RaytraceLightsUniform {
    fn default() -> Self {
        Self {
            directional_lights: [GpuDirectionalLight::default(); MAX_DIRECTIONAL_LIGHTS],
            directional_light_count: 0,
            _directional_padding: Vec4::ZERO,
            punctual_lights: [GpuPunctualLight::default(); MAX_PUNCTUAL_LIGHTS],
            punctual_light_count: 0,
            _padding: Vec4::ZERO,
        }
    }
}

#[allow(dead_code)]
#[derive(Component)]
pub struct RaytraceOutputTexture {
    #[allow(dead_code)]
    pub texture: Texture,
    pub view: TextureView,
    pub size: bevy::render::render_resource::Extent3d,
}

#[derive(Component, Default)]
pub struct RaytraceViewLights {
    pub uniform: UniformBuffer<RaytraceLightsUniform>,
}

pub fn prepare_raytrace_output_textures(
    query: Query<(Entity, &ExtractedCamera, Option<&RaytraceOutputTexture>), With<RaytraceView>>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    for (entity, camera, output_texture) in &query {
        let Some(viewport) = camera.physical_viewport_size else {
            continue;
        };
        if viewport.x == 0 || viewport.y == 0 {
            continue;
        }

        let size = viewport.to_extents();
        if let Some(output_texture) = output_texture
            && output_texture.size == size
        {
            continue;
        }

        let descriptor = TextureDescriptor {
            label: Some("raytrace_output_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };
        let texture = render_device.create_texture(&descriptor);
        let view = texture.create_view(&TextureViewDescriptor::default());

        commands.entity(entity).insert(RaytraceOutputTexture {
            texture,
            view,
            size,
        });
    }
}

pub fn prepare_raytrace_view_lights(
    views: Query<
        (
            Entity,
            Option<&RaytraceLightSelection>,
            Option<&mut RaytraceViewLights>,
        ),
        With<RaytraceView>,
    >,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut commands: Commands,
) {
    for (entity, selection, view_lights) in &views {
        let _ = view_lights;
        let selection = selection.cloned().unwrap_or_default();
        let mut uniform = UniformBuffer::default();
        uniform.set(RaytraceLightsUniform {
            directional_lights: selection.directional_lights,
            directional_light_count: selection.directional_light_count,
            _directional_padding: Vec4::ZERO,
            punctual_lights: selection.punctual_lights,
            punctual_light_count: selection.punctual_light_count,
            _padding: Vec4::ZERO,
        });
        uniform.write_buffer(&render_device, &render_queue);

        commands.entity(entity).insert(RaytraceViewLights { uniform });
    }
}
