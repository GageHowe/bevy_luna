pub use bevy_solari::scene::RaytracingMesh3d;
use bevy::{
    ecs::{component::Component, prelude::ReflectComponent},
    reflect::{Reflect, prelude::ReflectDefault},
};

/// Opts a mesh entity out of automatic raytracing proxy generation.
#[derive(Component, Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, PartialEq)]
pub struct DisableRaytracingMesh;
