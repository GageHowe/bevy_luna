#import bevy_render::view::View
#import bevy_solari::scene_bindings::{directional_lights, trace_ray, RAY_T_MAX}

@group(1) @binding(0) var output_texture: texture_storage_2d<rgba16float, write>;
@group(1) @binding(1) var depth_texture: texture_depth_2d;
@group(1) @binding(2) var normal_texture: texture_2d<f32>;
@group(1) @binding(3) var<uniform> view: View;

fn reconstruct_view_position(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2(0.5, 0.5)) / vec2<f32>(dims);
    let clip_xy = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - 2.0 * uv.y);
    let view_position_h = view.view_from_clip * vec4<f32>(clip_xy, depth, 1.0);
    return view_position_h.xyz / view_position_h.w;
}

fn reconstruct_world_position(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let view_position = reconstruct_view_position(pixel, dims, depth);
    return (view.world_from_view * vec4<f32>(view_position, 1.0)).xyz;
}

fn decode_normal(encoded: vec3<f32>) -> vec3<f32> {
    return normalize(encoded * 2.0 - vec3<f32>(1.0));
}

fn is_valid_depth(depth: f32) -> bool {
    return depth > 0.0 && depth < 1.0;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(output_texture);
    if global_id.x >= dims.x || global_id.y >= dims.y {
        return;
    }

    let pixel = vec2<i32>(global_id.xy);
    let depth = textureLoad(depth_texture, pixel, 0);
    if !is_valid_depth(depth) {
        textureStore(output_texture, pixel, vec4(1.0, 1.0, 1.0, 0.0));
        return;
    }

    let encoded_normal = textureLoad(normal_texture, pixel, 0).xyz;
    let normal = decode_normal(encoded_normal);
    if any(normal != normal) {
        textureStore(output_texture, pixel, vec4(1.0, 1.0, 1.0, 0.0));
        return;
    }

    let directional_light_count = arrayLength(&directional_lights);
    if directional_light_count == 0u {
        textureStore(output_texture, pixel, vec4(1.0, 1.0, 1.0, 0.0));
        return;
    }

    let world_position = reconstruct_world_position(pixel, dims, depth);
    let light = directional_lights[0u];
    let direction_to_light = normalize(light.direction_to_light);
    let ndotl = max(dot(normal, direction_to_light), 0.0);
    if ndotl <= 0.0 {
        textureStore(output_texture, pixel, vec4(1.0, 1.0, 1.0, 0.0));
        return;
    }

    let ray_origin = world_position + normal * 0.02;
    let ray_hit = trace_ray(ray_origin, direction_to_light, 0.001, RAY_T_MAX, RAY_FLAG_TERMINATE_ON_FIRST_HIT);
    let visible = f32(ray_hit.kind == RAY_QUERY_INTERSECTION_NONE);
    textureStore(output_texture, pixel, vec4(vec3<f32>(visible), ndotl));
}
