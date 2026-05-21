use super::{RaytraceLightSelection, RaytraceView};
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

pub const MAX_POINT_LIGHTS: usize = 16;

#[derive(Clone, Copy, Debug, ShaderType, Default)]
pub struct GpuPointLight {
    pub position_range: Vec4,
    pub color_intensity: Vec4,
}

#[derive(Clone, Copy, ShaderType)]
pub struct RaytraceLightsUniform {
    pub point_lights: [GpuPointLight; MAX_POINT_LIGHTS],
    pub point_light_count: u32,
    pub _padding: Vec4,
}

impl Default for RaytraceLightsUniform {
    fn default() -> Self {
        Self {
            point_lights: [GpuPointLight::default(); MAX_POINT_LIGHTS],
            point_light_count: 0,
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
            point_lights: selection.point_lights,
            point_light_count: selection.point_light_count,
            _padding: Vec4::ZERO,
        });
        uniform.write_buffer(&render_device, &render_queue);

        commands.entity(entity).insert(RaytraceViewLights { uniform });
    }
}
