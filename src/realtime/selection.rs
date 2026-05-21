use super::{
    RaytraceDirectionalLight, RaytraceLightSelection, RaytracePunctualLight,
    prepare::{GpuDirectionalLight, GpuPunctualLight},
    shared::{MAX_DIRECTIONAL_LIGHTS, MAX_PUNCTUAL_LIGHTS},
};
use bevy::{light::{DirectionalLight, PointLight, SpotLight}, math::Vec4, prelude::*};

pub fn pack_directional_light(
    light: &DirectionalLight,
    transform: &GlobalTransform,
    override_light: Option<&RaytraceDirectionalLight>,
) -> Option<GpuDirectionalLight> {
    let illuminance = override_light.map_or(light.illuminance, |value| value.illuminance);
    if illuminance <= 0.0 {
        return None;
    }

    let direction_to_light = -transform.forward().as_vec3();
    Some(GpuDirectionalLight {
        direction_to_light: direction_to_light.extend(0.0),
        color_illuminance: light.color.to_linear().to_vec3().extend(illuminance),
    })
}

pub fn score_punctual_light(intensity: f32, range: f32) -> f32 {
    intensity * range * range
}

pub fn pack_point_light(
    light: &PointLight,
    transform: &GlobalTransform,
    override_light: Option<&RaytracePunctualLight>,
) -> Option<(f32, GpuPunctualLight)> {
    let intensity = override_light.map_or(light.intensity, |value| value.intensity);
    if intensity <= 0.0 || light.range <= 0.0 {
        return None;
    }

    Some((
        score_punctual_light(intensity, light.range),
        GpuPunctualLight {
            position_range: transform.translation().extend(light.range),
            color_intensity: light.color.to_linear().to_vec3().extend(intensity),
            direction_cos_outer: Vec4::ZERO,
            params: Vec4::ZERO,
        },
    ))
}

pub fn pack_spot_light(
    light: &SpotLight,
    transform: &GlobalTransform,
    override_light: Option<&RaytracePunctualLight>,
) -> Option<(f32, GpuPunctualLight)> {
    let intensity = override_light.map_or(light.intensity, |value| value.intensity);
    if intensity <= 0.0 || light.range <= 0.0 {
        return None;
    }

    let direction = transform.forward().as_vec3();
    let cos_inner = light.inner_angle.cos();
    let cos_outer = light.outer_angle.cos();
    let inverse_angle_range = 1.0 / (cos_inner - cos_outer).max(1e-4);

    Some((
        score_punctual_light(intensity, light.range),
        GpuPunctualLight {
            position_range: transform.translation().extend(light.range),
            color_intensity: light.color.to_linear().to_vec3().extend(intensity),
            direction_cos_outer: direction.extend(cos_outer),
            params: Vec4::new(
                inverse_angle_range,
                -cos_outer * inverse_angle_range,
                1.0,
                0.0,
            ),
        },
    ))
}

pub fn push_directional_light(selection: &mut RaytraceLightSelection, light: GpuDirectionalLight) {
    if (selection.directional_light_count as usize) < MAX_DIRECTIONAL_LIGHTS {
        selection.directional_lights[selection.directional_light_count as usize] = light;
        selection.directional_light_count += 1;
    }
}

pub fn push_punctual_light(selection: &mut RaytraceLightSelection, light: GpuPunctualLight) {
    if (selection.punctual_light_count as usize) < MAX_PUNCTUAL_LIGHTS {
        selection.punctual_lights[selection.punctual_light_count as usize] = light;
        selection.punctual_light_count += 1;
    }
}
