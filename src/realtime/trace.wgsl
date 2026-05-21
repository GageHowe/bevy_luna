#import bevy_render::maths::PI
#import bevy_pbr::pbr_functions::calculate_diffuse_color
#import bevy_render::view::View
#import bevy_raytrace::gbuffer_utils::gpixel_resolve
#import bevy_solari::sampling::{trace_light_visibility, trace_point_visibility}

struct GpuDirectionalLight {
    direction_to_light: vec4<f32>,
    color_illuminance: vec4<f32>,
}

struct GpuPunctualLight {
    position_range: vec4<f32>,
    color_intensity: vec4<f32>,
    direction_cos_outer: vec4<f32>,
    params: vec4<f32>,
}

struct RaytraceLights {
    directional_lights: array<GpuDirectionalLight, 4u>,
    directional_light_count: u32,
    _directional_padding: vec4<f32>,
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
    let resolved = gpixel_resolve(gbuffer, depth, gid.xy, vec2<f32>(dims), view.world_from_clip);
    let diffuse_color =
        calculate_diffuse_color(resolved.material.base_color, resolved.material.metallic, 0.0, 0.0) / PI;
    let world_normal = normalize(resolved.world_normal);
    let world_position = resolved.world_position;
    let ray_origin = world_position + world_normal * 0.03;

    var total_shadowed_light = vec3(0.0);
    var total_unshadowed = 0.0;
    var total_shadowed = 0.0;

    for (var i = 0u; i < lights.directional_light_count; i += 1u) {
        let light = lights.directional_lights[i];
        let direction_to_light = normalize(light.direction_to_light.xyz);
        let ndotl = saturate(dot(world_normal, direction_to_light));
        if ndotl <= 0.0 {
            continue;
        }

        let unshadowed = diffuse_color * light.color_illuminance.rgb * light.color_illuminance.w * ndotl;
        let visibility = trace_light_visibility(ray_origin, vec4(direction_to_light, 0.0));
        let shadowed = unshadowed * visibility;
        total_shadowed_light += shadowed;
        total_unshadowed += length(unshadowed);
        total_shadowed += length(shadowed);
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
        let unshadowed = diffuse_color * luminous_intensity * attenuation * ndotl * cone_factor;
        let visibility = trace_point_visibility(ray_origin, light.position_range.xyz);
        let shadowed = unshadowed * visibility;
        total_shadowed_light += shadowed;
        total_unshadowed += length(unshadowed);
        total_shadowed += length(shadowed);
    }

    if total_unshadowed <= 0.0 {
        textureStore(output_texture, pixel, vec4(0.0));
        return;
    }

    let shadow_mask = 1.0 - saturate(total_shadowed / total_unshadowed);
    textureStore(output_texture, pixel, vec4(total_shadowed_light * view.exposure, shadow_mask));
}
