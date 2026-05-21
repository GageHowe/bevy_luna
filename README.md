# bevy_luna

`bevy_luna` is an experimental real-time raytraced shadow-replacement crate for Bevy.

If the plugin is added, the raytraced path is available and enabled by default
for 3D cameras on supported hardware. If the plugin is not added, the game runs
with plain Bevy rendering.

Current scope:
- directional-light shadow replacement
- point-light shadow replacement
- spot-light shadow replacement
- runtime switch between Bevy shadows and raytraced shadows

Ownership model:
- all `Camera3d` views are managed automatically unless they have
  `DisableRaytraceView`
- all supported directional, point, and spot lights are managed automatically
  unless they have `DisableRaytraceLight`
- `RaytraceDirectionalLight` / `RaytracePunctualLight` are optional baseline
  overrides only; they are not required for normal use
- `RaytraceSettings` can be inserted if you want to switch between `Bevy` and
  `RaytracedShadows` at runtime; otherwise the plugin defaults to
  `RaytracedShadows` on supported hardware

Non-goals right now:
- temporal denoising or temporal accumulation
- custom relighting
