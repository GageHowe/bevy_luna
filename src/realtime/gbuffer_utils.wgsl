#define_import_path bevy_raytrace::gbuffer_utils

#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::rgb9e5::rgb9e5_to_vec3_
#import bevy_pbr::utils::octahedral_decode
#import bevy_solari::scene_bindings::ResolvedMaterial

struct ResolvedGPixel {
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    material: ResolvedMaterial,
}

fn gpixel_resolve(
    gpixel: vec4<u32>,
    depth: f32,
    pixel_id: vec2<u32>,
    view_size: vec2<f32>,
    world_from_clip: mat4x4<f32>,
) -> ResolvedGPixel {
    let world_position = reconstruct_world_position(pixel_id, depth, view_size, world_from_clip);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));

    let base_rough = unpack4x8unorm(gpixel.r);
    let base_color = pow(base_rough.rgb, vec3(2.2));
    let perceptual_roughness = clamp(base_rough.a, 0.0316227766, 1.0);
    let roughness = perceptual_roughness * perceptual_roughness;
    let props = unpack4x8unorm(gpixel.b);
    let reflectance = vec3(props.r);
    let metallic = saturate(props.g);
    let emissive = rgb9e5_to_vec3_(gpixel.g);
    let material = ResolvedMaterial(
        base_color,
        emissive,
        reflectance,
        perceptual_roughness,
        roughness,
        metallic,
    );

    return ResolvedGPixel(world_position, world_normal, material);
}

fn reconstruct_world_position(
    pixel_id: vec2<u32>,
    depth: f32,
    view_size: vec2<f32>,
    world_from_clip: mat4x4<f32>,
) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view_size;
    let xy_ndc = (uv - vec2(0.5)) * vec2(2.0, -2.0);
    let world_pos = world_from_clip * vec4(xy_ndc, depth, 1.0);
    return world_pos.xyz / world_pos.w;
}
