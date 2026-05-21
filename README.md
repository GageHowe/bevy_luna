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

Usage note:
- supported directional, point, and spot lights are managed automatically
- `RaytraceDirectionalLight` / `RaytracePunctualLight` are optional overrides
  that pin the baseline light intensity used by the raytraced path
- `RaytraceSettings` can still be inserted if you want to toggle back to Bevy
  shadows at runtime

Non-goals right now:
- temporal denoising or temporal accumulation
- custom relighting
