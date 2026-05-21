#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var raytrace_texture: texture_2d<f32>;
@group(0) @binding(2) var linear_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(source_texture, linear_sampler, in.uv);
    let traced = textureSample(raytrace_texture, linear_sampler, in.uv);
    let strength = clamp(traced.a, 0.0, 1.0);
    let visibility = clamp(traced.r, 0.0, 1.0);
    let shadow_factor = mix(0.18, 1.0, visibility);
    let shaded = source.rgb * shadow_factor;
    let color = mix(source.rgb, shaded, strength);
    return vec4(color, source.a);
}
