#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_pbr::pbr_deferred_types::unpack_unorm4x8_
#import bevy_solari::scene_bindings::{DirectionalLight, directional_lights}
#import bevy_solari::sampling::{trace_light_visibility, trace_point_visibility}

struct GpuPunctualLight {
    position_range: vec4<f32>,
    color_intensity: vec4<f32>,
    direction_cos_outer: vec4<f32>,
    params: vec4<f32>,
}

struct RaytraceLights {
    punctual_lights: array<GpuPunctualLight, 16u>,
    punctual_light_count: u32,
    _padding: vec4<f32>,
}

@group(1) @binding(0) var output_texture: texture_storage_2d<rgba16float, write>;
@group(1) @binding(1) var deferred_texture: texture_2d<u32>;
@group(1) @binding(2) var depth_texture: texture_depth_2d;
@group(1) @binding(3) var normal_texture: texture_2d<f32>;
@group(1) @binding(4) var<uniform> view: View;
@group(1) @binding(5) var<uniform> lights: RaytraceLights;

fn reconstruct_world_position(pixel_id: vec2<u32>, depth: f32, view_size: vec2<f32>) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view_size;
    let xy_ndc = (uv - vec2(0.5)) * vec2(2.0, -2.0);
    let world_pos = view.world_from_clip * vec4(xy_ndc, depth, 1.0);
    return world_pos.xyz / world_pos.w;
}

fn inverse_square_range_attenuation(distance_sq: f32, range: f32) -> f32 {
    let inverse_range_sq = 1.0 / max(range * range, 0.0001);
    let factor = distance_sq * inverse_range_sq;
    let smooth_factor = saturate(1.0 - factor * factor);
    return (smooth_factor * smooth_factor) / max(distance_sq, 0.0001);
}

fn punctual_cone_attenuation(light: GpuPunctualLight, wi: vec3<f32>) -> f32 {
    if light.params.z < 0.5 {
        return 1.0;
    }

    let cd = dot(-light.direction_cos_outer.xyz, wi);
    let spot_attenuation = saturate(cd * light.params.x + light.params.y);
    return spot_attenuation * spot_attenuation;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(output_texture);
    if any(gid.xy >= dims) {
        return;
    }

    let pixel = vec2<i32>(gid.xy);
    let depth = textureLoad(depth_texture, pixel, 0);
    if depth <= 0.0 {
        textureStore(output_texture, pixel, vec4(0.0));
        return;
    }

    let gbuffer = textureLoad(deferred_texture, pixel, 0);
    let base_rough = unpack_unorm4x8_(gbuffer.r);
    let base_color = pow(base_rough.rgb, vec3(2.2)) / PI;
    let world_normal = normalize(textureLoad(normal_texture, pixel, 0).xyz * 2.0 - vec3(1.0));
    let world_position = reconstruct_world_position(gid.xy, depth, vec2<f32>(dims));
    let ray_origin = world_position + world_normal * 0.03;

    var total_occlusion = vec3(0.0);
    var total_unshadowed = 0.0;
    var total_occluded = 0.0;

    for (var i = 0u; i < arrayLength(&directional_lights); i += 1u) {
        let light: DirectionalLight = directional_lights[i];
        let ndotl = saturate(dot(world_normal, light.direction_to_light));
        if ndotl <= 0.0 {
            continue;
        }

        let unshadowed = base_color * light.luminance * ndotl;
        let visibility = trace_light_visibility(ray_origin, vec4(light.direction_to_light, 0.0));
        let occluded = unshadowed * (1.0 - visibility);
        total_occlusion += occluded;
        total_unshadowed += length(unshadowed);
        total_occluded += length(occluded);
    }

    for (var i = 0u; i < lights.punctual_light_count; i += 1u) {
        let light = lights.punctual_lights[i];
        let to_light = light.position_range.xyz - world_position;
        let distance_sq = dot(to_light, to_light);
        let distance = sqrt(max(distance_sq, 0.0001));
        let wi = to_light / distance;
        let ndotl = saturate(dot(world_normal, wi));
        if ndotl <= 0.0 || distance >= light.position_range.w {
            continue;
        }

        let attenuation = inverse_square_range_attenuation(distance_sq, light.position_range.w);
        let cone_factor = punctual_cone_attenuation(light, wi);
        if cone_factor <= 0.0 {
            continue;
        }
        let luminous_intensity = light.color_intensity.rgb * (light.color_intensity.w / (4.0 * PI));
        let unshadowed = base_color * luminous_intensity * attenuation * ndotl * cone_factor;
        let visibility = trace_point_visibility(ray_origin, light.position_range.xyz);
        let occluded = unshadowed * (1.0 - visibility);
        total_occlusion += occluded;
        total_unshadowed += length(unshadowed);
        total_occluded += length(occluded);
    }

    if total_unshadowed <= 0.0 {
        textureStore(output_texture, pixel, vec4(0.0));
        return;
    }

    let shadow_mask = saturate(total_occluded / total_unshadowed);
    textureStore(output_texture, pixel, vec4(total_occlusion, shadow_mask));
}
