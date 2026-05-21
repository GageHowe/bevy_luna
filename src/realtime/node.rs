use super::prepare::{RaytraceLightsUniform, RaytraceOutputTexture, RaytraceViewLights};
use super::{RaytraceDebugMode, RaytraceQuality, RaytraceView};
use bevy::{
    app::SubApp,
    asset::{AssetServer, embedded_asset, load_embedded_asset},
    core_pipeline::{
        FullscreenShader,
        core_3d::graph::{Core3d, Node3d},
        prepass::ViewPrepassTextures,
    },
    ecs::{
        prelude::*,
        query::QueryItem,
        resource::Resource,
        system::{Commands, lifetimeless::Read},
        world::World,
    },
    prelude::default,
    render::{
        RenderStartup,
        render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt, ViewNode, ViewNodeRunner},
        render_resource::{
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            CachedComputePipelineId, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            ComputePassDescriptor, ComputePipelineDescriptor, FragmentState, Operations,
            PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
            ShaderStages, StorageTextureAccess, TextureFormat, TextureSampleType,
            binding_types::{sampler, texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice},
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    },
};
use bevy_shader::load_shader_library;
use bevy_solari::scene::RaytracingSceneBindings;

pub mod graph {
    use bevy::render::render_graph::RenderLabel;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub struct RaytraceNode;
}

#[derive(Resource)]
pub struct RaytracePipelines {
    compute_layout: BindGroupLayoutDescriptor,
    compute_pipeline: CachedComputePipelineId,
    composite_layout: BindGroupLayoutDescriptor,
    composite_sampler: Sampler,
    composite_pipeline: CachedRenderPipelineId,
    composite_debug_pipeline: CachedRenderPipelineId,
    composite_shadow_mask_pipeline: CachedRenderPipelineId,
}

#[derive(Default)]
pub struct RaytraceNode;

pub fn load_internal_assets(app: &mut bevy::app::App) {
    load_shader_library!(app, "trace.wgsl");
    embedded_asset!(app, "composite.wgsl");
    embedded_asset!(app, "composite_debug.wgsl");
    embedded_asset!(app, "composite_shadow_mask.wgsl");
}

pub fn setup_render_app(render_app: &mut SubApp) {
    render_app
        .add_systems(RenderStartup, init_raytrace_pipelines)
        .add_render_graph_node::<ViewNodeRunner<RaytraceNode>>(Core3d, graph::RaytraceNode)
        .add_render_graph_edges(
            Core3d,
            (
                Node3d::StartMainPassPostProcessing,
                graph::RaytraceNode,
                Node3d::Tonemapping,
            ),
        );
}

fn init_raytrace_pipelines(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
) {
    let compute_layout = BindGroupLayoutDescriptor::new(
        "raytrace_compute_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                texture_2d(TextureSampleType::Uint),
                texture_depth_2d(),
                texture_2d(TextureSampleType::Float { filterable: false }),
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<RaytraceLightsUniform>(false),
            ),
        ),
    );

    let compute_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("raytrace_compute_pipeline".into()),
        layout: vec![
            RaytracingSceneBindings::new().bind_group_layout,
            compute_layout.clone(),
        ],
        shader: load_embedded_asset!(asset_server.as_ref(), "trace.wgsl"),
        ..default()
    });

    let composite_layout = BindGroupLayoutDescriptor::new(
        "raytrace_composite_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );
    let composite_sampler = render_device.create_sampler(&SamplerDescriptor::default());

    let composite_pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("raytrace_composite_pipeline".into()),
        layout: vec![composite_layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: load_embedded_asset!(asset_server.as_ref(), "composite.wgsl"),
            entry_point: Some("fragment".into()),
            targets: vec![Some(ColorTargetState {
                format: ViewTarget::TEXTURE_FORMAT_HDR,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });

    let composite_debug_pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("raytrace_composite_debug_pipeline".into()),
        layout: vec![composite_layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: load_embedded_asset!(asset_server.as_ref(), "composite_debug.wgsl"),
            entry_point: Some("fragment".into()),
            targets: vec![Some(ColorTargetState {
                format: ViewTarget::TEXTURE_FORMAT_HDR,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });

    let composite_shadow_mask_pipeline =
        pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("raytrace_composite_shadow_mask_pipeline".into()),
            layout: vec![composite_layout.clone()],
            vertex: fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: load_embedded_asset!(asset_server.as_ref(), "composite_shadow_mask.wgsl"),
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: ViewTarget::TEXTURE_FORMAT_HDR,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        });

    commands.insert_resource(RaytracePipelines {
        compute_layout,
        compute_pipeline,
        composite_layout,
        composite_sampler,
        composite_pipeline,
        composite_debug_pipeline,
        composite_shadow_mask_pipeline,
    });
}

impl ViewNode for RaytraceNode {
    type ViewQuery = (
        Read<RaytraceView>,
        Read<ViewTarget>,
        Read<ViewPrepassTextures>,
        Read<ViewUniformOffset>,
        Read<RaytraceOutputTexture>,
        Read<RaytraceViewLights>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            raytrace_view,
            view_target,
            prepass_textures,
            view_uniform_offset,
            output_texture,
            view_lights,
        ): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipelines = world.resource::<RaytracePipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let view_uniforms = world.resource::<ViewUniforms>();
        let scene_bindings = world.resource::<RaytracingSceneBindings>();

        let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.compute_pipeline)
        else {
            bevy::log::warn!(
                "bevy_raytrace: compute pipeline not ready: {:?}",
                pipeline_cache.get_compute_pipeline_state(pipelines.compute_pipeline)
            );
            return Ok(());
        };
        let composite_pipeline_id = match raytrace_view.debug {
            RaytraceDebugMode::DirectLighting => pipelines.composite_debug_pipeline,
            RaytraceDebugMode::ShadowMask => pipelines.composite_shadow_mask_pipeline,
            _ => pipelines.composite_pipeline,
        };
        let Some(composite_pipeline) = pipeline_cache.get_render_pipeline(composite_pipeline_id)
        else {
            bevy::log::warn!(
                "bevy_raytrace: composite pipeline not ready: {:?}",
                pipeline_cache.get_render_pipeline_state(composite_pipeline_id)
            );
            return Ok(());
        };
        let Some(scene_bind_group) = scene_bindings.bind_group.as_ref() else {
            bevy::log::warn!("bevy_raytrace: missing scene bind group");
            return Ok(());
        };
        let Some(deferred_view) = prepass_textures.deferred_view() else {
            bevy::log::warn!("bevy_raytrace: missing deferred prepass view");
            return Ok(());
        };
        let Some(depth_view) = prepass_textures.depth_view() else {
            bevy::log::warn!("bevy_raytrace: missing depth prepass view");
            return Ok(());
        };
        let Some(normal_view) = prepass_textures.normal_view() else {
            bevy::log::warn!("bevy_raytrace: missing normal prepass view");
            return Ok(());
        };
        let Some(view_uniform_binding) = view_uniforms.uniforms.binding() else {
            bevy::log::warn!("bevy_raytrace: missing view uniform binding");
            return Ok(());
        };
        let Some(light_binding) = view_lights.uniform.binding() else {
            bevy::log::warn!("bevy_raytrace: missing light uniform binding");
            return Ok(());
        };
        if output_texture.size.width == 0 || output_texture.size.height == 0 {
            bevy::log::warn!("bevy_raytrace: output texture has zero size");
            return Ok(());
        }

        let compute_bind_group = render_context.render_device().create_bind_group(
            Some("raytrace_compute_bind_group"),
            &pipeline_cache.get_bind_group_layout(&pipelines.compute_layout),
            &BindGroupEntries::sequential((
                &output_texture.view,
                deferred_view,
                depth_view,
                normal_view,
                view_uniform_binding,
                light_binding,
            )),
        );

        {
            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("raytrace_compute_pass"),
                        timestamp_writes: None,
                    });
            pass.set_pipeline(compute_pipeline);
            pass.set_bind_group(0, scene_bind_group, &[]);
            pass.set_bind_group(1, &compute_bind_group, &[view_uniform_offset.offset]);
            let (group_x, group_y) = dispatch_size(
                output_texture.size.width,
                output_texture.size.height,
                raytrace_view.quality,
            );
            pass.dispatch_workgroups(group_x, group_y, 1);
        }

        let post_process = view_target.post_process_write();
        let composite_bind_group = render_context.render_device().create_bind_group(
            Some("raytrace_composite_bind_group"),
            &pipeline_cache.get_bind_group_layout(&pipelines.composite_layout),
            &BindGroupEntries::sequential((
                post_process.source,
                &output_texture.view,
                &pipelines.composite_sampler,
            )),
        );

        let mut render_pass =
            render_context
                .command_encoder()
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some("raytrace_composite_pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: post_process.destination,
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations::default(),
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
        render_pass.set_pipeline(composite_pipeline);
        render_pass.set_bind_group(0, &composite_bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

pub fn dispatch_size(width: u32, height: u32, quality: RaytraceQuality) -> (u32, u32) {
    let divisor = match quality {
        RaytraceQuality::Performance => 16,
        RaytraceQuality::Balanced | RaytraceQuality::Quality => 8,
    };

    (width.div_ceil(divisor), height.div_ceil(divisor))
}
