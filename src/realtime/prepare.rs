use super::RaytraceView;
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        system::{Commands, Query, Res},
    },
    image::ToExtents,
    render::{
        camera::ExtractedCamera,
        render_resource::{
            Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
            TextureViewDescriptor,
        },
        renderer::RenderDevice,
    },
};

#[allow(dead_code)]
#[derive(Component)]
pub struct RaytraceOutputTexture {
    #[allow(dead_code)]
    pub texture: Texture,
    pub view: TextureView,
    pub size: bevy::render::render_resource::Extent3d,
}

#[allow(dead_code)]
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
