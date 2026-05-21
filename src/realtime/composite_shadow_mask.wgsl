#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var raytrace_texture: texture_2d<f32>;
@group(0) @binding(2) var linear_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let traced = textureSample(raytrace_texture, linear_sampler, in.uv);
    let mask = clamp(traced.r, 0.0, 1.0);
    return vec4(vec3(mask), 1.0);
}
