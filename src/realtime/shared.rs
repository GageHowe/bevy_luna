pub const MAX_DIRECTIONAL_LIGHTS: usize = 4;
pub const MAX_PUNCTUAL_LIGHTS: usize = 32;

#[cfg(test)]
pub const TRACE_WGSL_MAX_DIRECTIONAL_LIGHTS: &str = "const MAX_DIRECTIONAL_LIGHTS: u32 = 4u;";
#[cfg(test)]
pub const TRACE_WGSL_MAX_PUNCTUAL_LIGHTS: &str = "const MAX_PUNCTUAL_LIGHTS: u32 = 32u;";
