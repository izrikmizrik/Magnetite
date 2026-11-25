//! Half-precision arithmetic for the world's post passes (FXAA, TAA, sharpening, 4x MSAA's
//! resolve, and the passes added beside them). A phone's GPU runs `f16` at twice `f32`'s rate and
//! with half the registers, and these passes touch every pixel. Their shaders name their colour
//! maths with the aliases below: `f16` where the device has `SHADER_F16` and the Anti-aliasing
//! Precision setting asks for it (its default), `f32` otherwise (OpenGL, the Vulkan and Metal
//! devices without it, or the setting at f32), so one source serves both.
//!
//! Only colour and luma go to half precision. Texture coordinates, depth and reprojection stay
//! `f32`: half's 11 bits of mantissa hold a 2048-pixel coordinate to half a texel, and depth
//! not at all. Light reaches half float's largest value, 65504, on the HDR frame
//! (docs/hdr-passes.md): a pass sums scaled values (`0.5 * a + 0.5 * b`, never `(a + b) * 0.5`)
//! and clamps what could overflow, so no pass writes infinity or NaN (rule 5).

/// The aliases in half precision: `enable f16` must open the module, so this goes first.
const F16: &str = "enable f16;
alias hf = f16;
alias hv2 = vec2<f16>;
alias hv3 = vec3<f16>;
alias hv4 = vec4<f16>;
";

/// The same aliases in single precision.
pub(crate) const F32: &str = "alias hf = f32;
alias hv2 = vec2<f32>;
alias hv3 = vec3<f32>;
alias hv4 = vec4<f32>;
";

/// The `f16` prefix, for tests that check the half-precision form of a shader.
#[cfg(test)]
pub(crate) const F16_FOR_TESTS: &str = F16;

/// Whether `device` can run the passes in half precision: it has `SHADER_F16`.
pub(crate) fn available(device: &wgpu::Device) -> bool {
    device.features().contains(wgpu::Features::SHADER_F16)
}

/// `source` with the aliases in front of it: half precision where `wanted` (the setting) and
/// `device` has it, else single.
pub(crate) fn shader(device: &wgpu::Device, wanted: bool, source: &str) -> String {
    let prefix = if wanted && available(device) {
        F16
    } else {
        F32
    };
    let mut text = String::with_capacity(prefix.len() + source.len());
    text.push_str(prefix);
    text.push_str(source);
    text
}
