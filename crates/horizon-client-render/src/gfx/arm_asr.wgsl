// Arm Accuracy Super Resolution (Arm; MIT, after AMD FidelityFX Super Resolution 2;
// THIRD_PARTY_NOTICES.md), ported from Arm ASR's generic library (`include/gpu/fsr2`) with its
// Ultra Performance shader options (`FFXM_FSR2_OPTION_SHADER_OPT_ULTRA_PERFORMANCE`, Arm's own
// preset for the weakest GPUs: no luminance pyramid, no luma history, a 5-tap upsample). A
// temporal upscaler in TAA's place (`gfx/arm_asr.rs`), four passes:
//
// 1. `fs_reconstruct` (the world's size): the nearest depth around each pixel and its motion,
//    the luma that finds thin features, and the previous frame's depth scattered where the
//    motion says it was (`reconstructed_depth`, an atomic max: Horizon's depth is reversed).
// 2. `fs_depth_clip` (the world's size): where the scene was hidden last frame (depth clip),
//    and where the motion diverges.
// 3. `cs_lock` (the world's size): thin features locked into the output (`new_locks`), and
//    the reconstructed depth cleared for the next frame.
// 4. `fs_accumulate` (the output's size): the jittered frame upsampled and blended into the
//    history, the locks kept, and the picture written for the screen.
//
// Horizon's side of ASR's contract (docs/hdr-passes.md, rule 4): depth inverted and finite;
// the motion is the camera's, from the depth and TAA's reprojection, without the jitter;
// exposure 1; the frame's gamma light decoded to linear before (HDR input, 1.0 at paper white)
// and encoded again for the screen; each pixel keeps its alpha, HDR's light code. Its maths
// stays f32: ASR's own half path assumes light below half float's largest value.

struct Params {
    // This frame's unjittered clip space to the previous frame's.
    clip_to_prev_clip: mat4x4<f32>,
    // Device depth to view-space depth (ASR's `DeviceToViewSpaceTransformFactors`).
    device_to_view_depth: vec4<f32>,
    render_size: vec2<f32>,
    display_size: vec2<f32>,
    // This frame's jitter in rendered pixels, y down.
    jitter: vec2<f32>,
    // The render size over the display size.
    downscale_factor: vec2<f32>,
    jitter_sequence_length: f32,
    // 1 when the history is unusable (ASR's `FrameIndex() == 0`).
    reset: f32,
    pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var input_color: texture_2d<f32>;
// The world's depth through an unfilterable float binding (as TAA reads it).
@group(0) @binding(2) var input_depth: texture_2d<f32>;
@group(0) @binding(3) var linear_sampler: sampler;
@group(0) @binding(4) var<storage, read_write> reconstructed_depth: array<atomic<u32>>;
@group(0) @binding(5) var dilated_depth: texture_2d<f32>;
@group(0) @binding(6) var dilated_motion: texture_2d<f32>;
@group(0) @binding(7) var previous_dilated_motion: texture_2d<f32>;
@group(0) @binding(8) var lock_luma: texture_2d<f32>;
@group(0) @binding(9) var<storage, read_write> new_locks: array<u32>;
@group(0) @binding(10) var reactive_masks: texture_2d<f32>;
@group(0) @binding(11) var history: texture_2d<f32>;
@group(0) @binding(12) var lock_status: texture_2d<f32>;

const EPSILON = 1e-03;
const FP16_MAX = 65504.0;
const TONEMAP_EPSILON = 1.0 / 65504.0;
const RECONSTRUCTED_DEPTH_BILINEAR_WEIGHT_THRESHOLD = 0.01;
// ASR keeps 1/9 with the Ultra Performance 5-tap kernel (its `fUpsampleLanczosWeightScale`).
const UPSAMPLE_LANCZOS_WEIGHT_SCALE = 1.0 / 9.0;
const MAX_ACCUMULATION_LANCZOS_WEIGHT = 1.0;
const AVERAGE_LANCZOS_WEIGHT_PER_FRAME = 0.74 * UPSAMPLE_LANCZOS_WEIGHT_SCALE;

@vertex fn vs_asr(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4(p * 2.0 - 1.0, 0.0, 1.0);
}

// ---------------------------------------------------------------------------------------------
// Helpers (ffxm_fsr2_common.h, ffxm_fsr2_sample.h).

fn render_size() -> vec2<i32> {
    return vec2<i32>(params.render_size);
}

fn display_size() -> vec2<i32> {
    return vec2<i32>(params.display_size);
}

fn is_on_screen(p: vec2<i32>, size: vec2<i32>) -> bool {
    return all(vec2<u32>(p) < vec2<u32>(size));
}

fn clamp_uv(uv: vec2<f32>, size: vec2<f32>) -> vec2<f32> {
    return clamp(uv * size, vec2(0.5), size - vec2(0.5)) / size;
}

fn render_index(p: vec2<i32>) -> u32 {
    let q = clamp(p, vec2(0), render_size() - 1);
    return u32(q.y * render_size().x + q.x);
}

fn view_depth(device_depth: f32) -> f32 {
    return params.device_to_view_depth.y / (device_depth - params.device_to_view_depth.x);
}

fn view_position(p: vec2<i32>, size: vec2<f32>, device_depth: f32) -> vec3<f32> {
    let z = view_depth(device_depth);
    let ndc = vec2<f32>(p) / size * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    return vec3(params.device_to_view_depth.z * ndc.x * z, params.device_to_view_depth.w * ndc.y * z, z);
}

// The view's furthest depth: device depth 0, reversed. A block is a metre.
fn max_distance_in_meters() -> f32 {
    return view_depth(0.0);
}

fn hr_from_lr(p: vec2<i32>) -> vec2<i32> {
    let jittered = vec2<f32>(p) + vec2(0.5) - params.jitter;
    return vec2<i32>(floor(jittered / params.render_size * params.display_size));
}

// The frame's gamma light as the linear light ASR takes, and back.
fn decode(c: vec3<f32>) -> vec3<f32> {
    return min(pow(max(c, vec3(0.0)), vec3(2.2)), vec3(FP16_MAX));
}

fn encode(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3(0.0)), vec3(1.0 / 2.2));
}

fn input_rgb(p: vec2<i32>) -> vec3<f32> {
    return decode(textureLoad(input_color, clamp(p, vec2(0), render_size() - 1), 0).rgb);
}

fn input_depth_at(p: vec2<i32>) -> f32 {
    return textureLoad(input_depth, clamp(p, vec2(0), render_size() - 1), 0).r;
}

fn tonemap(c: vec3<f32>) -> vec3<f32> {
    return c / (max(max(0.0, c.r), max(c.g, c.b)) + 1.0);
}

fn inverse_tonemap(c: vec3<f32>) -> vec3<f32> {
    return c / max(TONEMAP_EPSILON, 1.0 - max(c.r, max(c.g, c.b)));
}

fn rgb_to_ycocg(c: vec3<f32>) -> vec3<f32> {
    return vec3(0.25 * c.r + 0.5 * c.g + 0.25 * c.b, 0.5 * c.r - 0.5 * c.b, -0.25 * c.r + 0.5 * c.g - 0.25 * c.b);
}

fn ycocg_to_rgb(c: vec3<f32>) -> vec3<f32> {
    return vec3(c.x + c.y - c.z, c.x + c.z, c.x - c.y - c.z);
}

fn perceived_luma(rgb: vec3<f32>) -> f32 {
    let luminance = dot(rgb, vec3(0.2126, 0.7152, 0.0722));
    var perceived = 0.0;
    if luminance <= 216.0 / 24389.0 {
        perceived = luminance * (24389.0 / 27.0);
    } else {
        perceived = pow(luminance, 1.0 / 3.0) * 116.0 - 16.0;
    }
    return perceived * 0.01;
}

fn min_divided_by_max(a: f32, b: f32) -> f32 {
    let m = max(a, b);
    return select(0.0, min(a, b) / m, m != 0.0);
}

fn lanczos2_approx_sq(x2_in: f32) -> f32 {
    let x2 = min(x2_in, 4.0);
    let a = (2.0 / 5.0) * x2 - 1.0;
    let b = (1.0 / 4.0) * x2 - 1.0;
    return ((25.0 / 16.0) * a * a - (25.0 / 16.0 - 1.0)) * (b * b);
}

// The camera's motion at a rendered pixel, ASR's way: the UV offset from where its unjittered
// point is now to where it was last frame.
fn input_motion(p_in: vec2<i32>) -> vec2<f32> {
    let p = clamp(p_in, vec2(0), render_size() - 1);
    let uv = (vec2<f32>(p) + vec2(0.5)) / params.render_size - params.jitter / params.render_size;
    let ndc = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let previous = params.clip_to_prev_clip * vec4(ndc, input_depth_at(p), 1.0);
    let previous_ndc = previous.xy / previous.w;
    let previous_uv = vec2(previous_ndc.x * 0.5 + 0.5, 0.5 - previous_ndc.y * 0.5);
    return previous_uv - uv;
}

struct Bilinear {
    base: vec2<i32>,
    weights: vec4<f32>,
};

fn bilinear(uv: vec2<f32>, size: vec2<f32>) -> Bilinear {
    let px = uv * size - vec2(0.5);
    let f = fract(px);
    return Bilinear(
        vec2<i32>(floor(px)),
        vec4((1.0 - f.x) * (1.0 - f.y), f.x * (1.0 - f.y), (1.0 - f.x) * f.y, f.x * f.y),
    );
}

fn bilinear_offset(i: i32) -> vec2<i32> {
    return vec2(i & 1, i >> 1);
}

// ---------------------------------------------------------------------------------------------
// 1. Reconstruct and dilate (ffxm_fsr2_reconstruct_dilated_velocity_and_previous_depth.h).

struct ReconstructOut {
    @location(0) depth: vec4<f32>,
    @location(1) motion: vec4<f32>,
    @location(2) luma: vec4<f32>,
};


@fragment fn fs_reconstruct(@builtin(position) position: vec4<f32>) -> ReconstructOut {
    let p = vec2<i32>(position.xy);
    let size = render_size();
    // The nearest depth of the 3x3: reversed, the largest.
    var nearest = input_depth_at(p);
    var nearest_p = p;
    var neighbours = array<vec2<i32>, 8>(
        vec2(1, 0), vec2(0, 1), vec2(0, -1), vec2(-1, 0),
        vec2(-1, 1), vec2(1, 1), vec2(-1, -1), vec2(1, -1),
    );
    for (var i = 0; i < 8; i++) {
        let q = p + neighbours[i];
        if is_on_screen(q, size) {
            let d = input_depth_at(q);
            if d > nearest {
                nearest = d;
                nearest_p = q;
            }
        }
    }
    let motion = input_motion(nearest_p);
    // Where this depth was last frame, scattered into the four pixels around it.
    let moving = motion * f32(length(motion * params.display_size) > 0.1);
    let uv = (vec2<f32>(p) + vec2(0.5)) / params.render_size;
    let b = bilinear(uv + moving, params.render_size);
    for (var i = 0; i < 4; i++) {
        if b.weights[i] > RECONSTRUCTED_DEPTH_BILINEAR_WEIGHT_THRESHOLD {
            let q = b.base + bilinear_offset(i);
            if is_on_screen(q, size) {
                atomicMax(&reconstructed_depth[render_index(q)], bitcast<u32>(nearest));
            }
        }
    }
    let luma = pow(perceived_luma(tonemap(input_rgb(p))), 1.0 / 6.0);
    return ReconstructOut(vec4(nearest, 0.0, 0.0, 0.0), vec4(motion, 0.0, 0.0), vec4(luma, 0.0, 0.0, 0.0));
}

// ---------------------------------------------------------------------------------------------
// 2. Depth clip (ffxm_fsr2_depth_clip.h).

fn previous_depth(p: vec2<i32>) -> f32 {
    return bitcast<f32>(atomicLoad(&reconstructed_depth[render_index(p)]));
}

fn compute_depth_clip(uv: vec2<f32>, current_depth: f32) -> f32 {
    let size = params.render_size;
    let current_view = view_depth(current_depth);
    let b = bilinear(uv, size);
    let half_viewport_width = length(size);
    let ndc_center = vec2<f32>(vec2<i32>(size * 0.5)) / size * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    let kfov = length(vec3(-params.device_to_view_depth.z, params.device_to_view_depth.w, 1.0))
        / length(vec3(
            params.device_to_view_depth.z * ndc_center.x,
            params.device_to_view_depth.w * ndc_center.y,
            1.0,
        ));
    let required_scale = 1.37e-05 * kfov * half_viewport_width;
    let resolution_factor = saturate(half_viewport_width / length(vec2(1920.0, 1080.0)));
    let power = mix(1.0, 3.0, resolution_factor);
    var depth = 0.0;
    var weight_sum = 0.0;
    for (var i = 0; i < 4; i++) {
        let q = b.base + bilinear_offset(i);
        if is_on_screen(q, render_size()) {
            let weight = b.weights[i];
            if weight > RECONSTRUCTED_DEPTH_BILINEAR_WEIGHT_THRESHOLD {
                let previous = previous_depth(q);
                let previous_view = view_depth(previous);
                let diff = current_view - previous_view;
                if diff > 0.0 {
                    let threshold = max(current_view, previous_view);
                    let required = required_scale * threshold;
                    depth += pow(saturate(required / diff), power) * weight;
                    weight_sum += weight;
                }
            }
        }
    }
    return select(0.0, saturate(1.0 - depth / weight_sum), weight_sum > 0.0);
}

fn motion_divergence(p: vec2<i32>) -> f32 {
    var min_convergence = 1.0;
    let nucleus = input_motion(p);
    var max_velocity_uv = length(nucleus);
    if length(nucleus * params.render_size) > 1e-02 {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                if x == 0 && y == 0 {
                    continue;
                }
                let motion = input_motion(p + vec2(x, y));
                var velocity = length(motion);
                max_velocity_uv = max(velocity, max_velocity_uv);
                velocity = max(velocity, max_velocity_uv);
                min_convergence = min(min_convergence, dot(motion / velocity, nucleus / velocity));
            }
        }
    }
    return saturate(1.0 - min_convergence) * saturate(max_velocity_uv / 0.01);
}

fn depth_divergence(p: vec2<i32>) -> f32 {
    let max_distance = max_distance_in_meters();
    var depth_max = 0.0;
    var depth_min = max_distance;
    var max_found = false;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let q = p + vec2(x, y);
            let on_screen = f32(is_on_screen(q, render_size()));
            let q_clamped = clamp(q, vec2(0), render_size() - 1);
            let depth = view_depth(textureLoad(dilated_depth, q_clamped, 0).r) * on_screen;
            max_found = max_found || max_distance == depth;
            depth_min = min(depth_min, depth);
            depth_max = max(depth_max, depth);
        }
    }
    return (1.0 - depth_min / depth_max) * select(1.0, 0.0, max_found);
}

fn temporal_motion_divergence(p: vec2<i32>) -> f32 {
    let uv = (vec2<f32>(p) + vec2(0.5)) / params.render_size;
    let motion = textureLoad(dilated_motion, p, 0).xy;
    let reprojected = clamp_uv(uv + motion, params.render_size);
    let previous_motion = textureSampleLevel(previous_dilated_motion, linear_sampler, reprojected, 0.0).xy;
    let px_distance = length(motion * params.display_size);
    if px_distance <= 1.0 {
        return 0.0;
    }
    return mix(0.0, 1.0 - saturate(length(previous_motion) / length(motion)), saturate(pow(px_distance / 20.0, 3.0)));
}

fn evaluate_surface(p: vec2<i32>) -> f32 {
    let d0 = view_depth(previous_depth(p + vec2(0, -1)));
    let d1 = view_depth(previous_depth(p));
    let d2 = view_depth(previous_depth(p + vec2(0, 1)));
    return 1.0 - f32((d0 - d1) > d1 * 0.01 && (d1 - d2) > d2 * 0.01);
}

@fragment fn fs_depth_clip(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(position.xy);
    let depth_uv = (vec2<f32>(p) + vec2(0.5)) / params.render_size;
    var motion = textureLoad(dilated_motion, p, 0).xy;
    motion *= f32(length(motion * params.display_size) > 0.01);
    let dilated = textureLoad(dilated_depth, p, 0).r;
    let depth_clip = compute_depth_clip(depth_uv + motion, dilated) * evaluate_surface(p);
    let temporal = saturate(temporal_motion_divergence(p) - depth_divergence(p));
    // Ultra Performance: the depth clip in x, the motion's divergence (the accumulation mask) in y.
    return vec4(depth_clip, max(temporal, motion_divergence(p)), 0.0, 0.0);
}

// ---------------------------------------------------------------------------------------------
// 3. Locks (ffxm_fsr2_lock.h).

fn lock_luma_at(p: vec2<i32>) -> f32 {
    return textureLoad(lock_luma, clamp(p, vec2(0), render_size() - 1), 0).r;
}

// A thin feature: a ridge in luma that no 2x2 corner of the 3x3 shares.
fn thin_feature(p: vec2<i32>) -> bool {
    let nucleus = lock_luma_at(p);
    var mask = 1u << 4u;
    var dissimilar_min = FP16_MAX;
    var dissimilar_max = 0.0;
    var index = 0u;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            if !(x == 0 && y == 0) {
                let neighbour = lock_luma_at(p + vec2(x, y));
                let difference = max(neighbour, nucleus) / min(neighbour, nucleus);
                if difference > 0.0 && difference < 1.05 {
                    mask |= 1u << index;
                } else {
                    dissimilar_min = min(dissimilar_min, neighbour);
                    dissimilar_max = max(dissimilar_max, neighbour);
                }
            }
            index += 1u;
        }
    }
    if !(nucleus > dissimilar_max || nucleus < dissimilar_min) {
        return false;
    }
    var rejections = array<u32, 4>(
        (1u << 0u) | (1u << 1u) | (1u << 3u) | (1u << 4u),
        (1u << 1u) | (1u << 2u) | (1u << 4u) | (1u << 5u),
        (1u << 3u) | (1u << 4u) | (1u << 6u) | (1u << 7u),
        (1u << 4u) | (1u << 5u) | (1u << 7u) | (1u << 8u),
    );
    for (var i = 0; i < 4; i++) {
        if (mask & rejections[i]) == rejections[i] {
            return false;
        }
    }
    return true;
}

@compute @workgroup_size(8, 8) fn cs_lock(@builtin(global_invocation_id) id: vec3<u32>) {
    let p = vec2<i32>(id.xy);
    if !is_on_screen(p, render_size()) {
        return;
    }
    if thin_feature(p) {
        let hr = hr_from_lr(p);
        if is_on_screen(hr, display_size()) {
            new_locks[u32(hr.y * display_size().x + hr.x)] = 1u;
        }
    }
    // The next frame scatters into an empty buffer: reversed depth's far is 0.
    atomicStore(&reconstructed_depth[render_index(p)], 0u);
}

// ---------------------------------------------------------------------------------------------
// 4. Accumulate (ffxm_fsr2_accumulate.h, _upsample.h, _reproject.h, _postprocess_lock_status.h).

// The history at `uv`: Catmull-Rom over 9 bilinear taps, held inside the 4 nearest texels.
fn history_sample(uv: vec2<f32>) -> vec4<f32> {
    let size = params.display_size;
    let inv = 1.0 / size;
    let sample_pos = uv * size;
    let tex_pos1 = floor(sample_pos - 0.5) + 0.5;
    let f = sample_pos - tex_pos1;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let wa = w0;
    let wb = w1 + w2;
    let wc = w3;
    let uv0 = (tex_pos1 - 1.0) * inv;
    let uv1 = (tex_pos1 + 2.0) * inv;
    let uv2 = (tex_pos1 + w2 / (w1 + w2)) * inv;
    let c00 = textureSampleLevel(history, linear_sampler, uv0, 0.0);
    let c20 = textureSampleLevel(history, linear_sampler, vec2(uv2.x, uv0.y), 0.0);
    let c02 = textureSampleLevel(history, linear_sampler, vec2(uv0.x, uv2.y), 0.0);
    let c22 = textureSampleLevel(history, linear_sampler, uv2, 0.0);
    var color = c00 * wa.x * wa.y + c20 * wb.x * wa.y
        + textureSampleLevel(history, linear_sampler, vec2(uv1.x, uv0.y), 0.0) * wc.x * wa.y
        + c02 * wa.x * wb.y + c22 * wb.x * wb.y
        + textureSampleLevel(history, linear_sampler, vec2(uv1.x, uv2.y), 0.0) * wc.x * wb.y
        + textureSampleLevel(history, linear_sampler, vec2(uv0.x, uv1.y), 0.0) * wa.x * wc.y
        + textureSampleLevel(history, linear_sampler, vec2(uv2.x, uv1.y), 0.0) * wb.x * wc.y
        + textureSampleLevel(history, linear_sampler, uv1, 0.0) * wc.x * wc.y;
    let lo = min(min(c00, c20), min(c02, c22));
    let hi = max(max(c00, c20), max(c02, c22));
    return clamp(color, lo, hi);
}

struct Rect {
    center: vec3<f32>,
    spread: vec3<f32>,
    aabb_min: vec3<f32>,
    aabb_max: vec3<f32>,
    weight: f32,
};

fn prepared_color(p: vec2<i32>) -> vec3<f32> {
    return rgb_to_ycocg(input_rgb(p));
}

struct AccumulateOut {
    @location(0) history: vec4<f32>,
    @location(1) lock_status: vec4<f32>,
    @location(2) color: vec4<f32>,
};


@fragment fn fs_accumulate(@builtin(position) position: vec4<f32>) -> AccumulateOut {
    let hr = vec2<i32>(position.xy);
    let display = params.display_size;
    let render = params.render_size;
    let hr_uv = (vec2<f32>(hr) + vec2(0.5)) / display;
    let lr_uv = clamp_uv(hr_uv + params.jitter / render, render);
    let lr = clamp(vec2<i32>(hr_uv * render), vec2(0), render_size() - 1);
    let motion = textureLoad(dilated_motion, lr, 0).xy;
    let hr_velocity = length(motion * display);
    let reprojected_uv = hr_uv + motion;
    let existing = all(reprojected_uv >= vec2(0.0)) && all(reprojected_uv <= vec2(1.0));
    let reset = params.reset > 0.5;
    let lock_index = u32(hr.y * display_size().x + hr.x);

    var history_color = vec3(0.0);
    var lock = vec2(0.0);
    var in_motion_last_frame = false;
    var new_lock = false;
    if existing && !reset {
        let h = history_sample(reprojected_uv);
        history_color = rgb_to_ycocg(h.rgb);
        in_motion_last_frame = h.w < 0.0;
        new_lock = f32(new_locks[lock_index]) > 127.0 / 255.0;
        lock = textureSampleLevel(lock_status, linear_sampler, reprojected_uv, 0.0).xy;
    }
    // Ultra Performance: no reactive mask, the accumulation mask and the depth clip from pass 2.
    let masks = textureSampleLevel(reactive_masks, linear_sampler, lr_uv, 0.0).xy;
    let accumulation_mask = masks.y;
    let depth_clip = masks.x;
    var reactive = 0.0;

    // UpdateLockStatus: Ultra Performance's shading change luma is 1.
    let shading = 1.0;
    if lock.y == 0.0 {
        lock.y = shading;
    }
    let luminance_diff = 1.0 - min_divided_by_max(lock.y, shading);
    if new_lock {
        lock.y = shading;
        lock.x = select(1.0, 2.0, lock.x != 0.0);
    } else if lock.x <= 1.0 {
        lock.y = mix(lock.y, shading, 0.5);
    } else if luminance_diff > 0.1 {
        lock.x = 0.0;
    }
    reactive = max(reactive, saturate((luminance_diff - 0.1) * 10.0));
    lock.x *= 1.0 - reactive;
    lock.x *= saturate(1.0 - accumulation_mask);
    lock.x *= f32(depth_clip < 0.1);
    let lifetime_contribution = saturate(lock.x - 1.0);
    let shading_contribution = saturate(min_divided_by_max(lock.y, shading));
    let lock_contribution = saturate(saturate(lifetime_contribution * 4.0) * shading_contribution);

    let is_new_sample = !existing || reset;

    // ComputeUpsampledColorAndWeight: the 5-tap Lanczos kernel and the rectification box.
    let src_output = (vec2<f32>(hr) + vec2(0.5)) * params.downscale_factor;
    let src_input = vec2<i32>(floor(src_output));
    let base_offset = (vec2<f32>(src_input) + vec2(0.5) - params.jitter) - src_output;
    let kernel_reactive = max(reactive, f32(is_new_sample));
    let max_kernel = min(1.99, 1.0 + (1.0 / params.downscale_factor.x - 1.0));
    let kernel_bias_max = max_kernel * (1.0 - kernel_reactive);
    let kernel_bias_min = max(1.0, (1.0 + kernel_bias_max) * 0.3);
    let kernel_bias_factor = max(0.0, max(0.25 * depth_clip, kernel_reactive));
    let kernel_bias = mix(kernel_bias_max, kernel_bias_min, kernel_bias_factor);
    let curve_bias = mix(-2.0, -3.0, saturate(hr_velocity / 50.0));
    var upsampled = vec4(0.0);
    var rect: Rect;
    var taps = array<vec2<i32>, 5>(vec2(0, -1), vec2(-1, 0), vec2(0, 0), vec2(1, 0), vec2(0, 1));
    for (var i = 0; i < 5; i++) {
        let tap_color = prepared_color(src_input + taps[i]);
        let offset = base_offset + vec2<f32>(taps[i]);
        let biased = offset * kernel_bias;
        let weight = lanczos2_approx_sq(dot(biased, biased));
        upsampled += vec4(tap_color * weight, weight);
        let box_weight = exp(curve_bias * dot(offset, offset));
        let weighted = tap_color * box_weight;
        if i == 0 {
            rect = Rect(weighted, tap_color * weighted, tap_color, tap_color, box_weight);
        } else {
            rect.aabb_min = min(rect.aabb_min, tap_color);
            rect.aabb_max = max(rect.aabb_max, tap_color);
            rect.center += weighted;
            rect.spread += tap_color * weighted;
            rect.weight += box_weight;
        }
    }
    rect.weight = select(1.0, rect.weight, abs(rect.weight) > EPSILON);
    rect.center /= rect.weight;
    rect.spread = sqrt(abs(rect.spread / rect.weight - rect.center * rect.center));
    upsampled.w *= f32(upsampled.w > EPSILON);
    if upsampled.w > EPSILON {
        upsampled = vec4(clamp(upsampled.xyz / upsampled.w, rect.aabb_min, rect.aabb_max), upsampled.w * UPSAMPLE_LANCZOS_WEIGHT_SCALE);
    }

    // FinalizeLockStatus.
    let next_uv = hr_uv - motion;
    if !(all(next_uv >= vec2(0.0)) && all(next_uv <= vec2(1.0))) {
        lock.x = 0.0;
    } else {
        let decrease = upsampled.w / (params.jitter_sequence_length * AVERAGE_LANCZOS_WEIGHT_PER_FRAME);
        lock.x = max(0.0, lock.x - decrease);
    }

    // ComputeBaseAccumulationWeight.
    var base = MAX_ACCUMULATION_LANCZOS_WEIGHT * f32(existing) * (1.0 - reactive) * (1.0 - depth_clip);
    base = min(base, mix(base, upsampled.w * 10.0, max(f32(in_motion_last_frame), saturate(hr_velocity * 10.0))));
    base = min(base, mix(base, upsampled.w, saturate(hr_velocity / 20.0)));
    var accumulation = vec3(base);

    var color = vec3(0.0);
    if is_new_sample {
        color = ycocg_to_rgb(upsampled.xyz);
    } else {
        // RectifyHistory.
        let df = params.downscale_factor;
        let scale_factor_influence = min(20.0, pow(1.0 / abs(df.x * df.y), 3.0));
        let box_scale_t = max(depth_clip, max(accumulation_mask, saturate(hr_velocity / 20.0)));
        let box_scale = mix(scale_factor_influence, 1.0, box_scale_t);
        let scaled = rect.spread * box_scale;
        let box_min = max(rect.aabb_min, rect.center - scaled);
        let box_max = min(rect.aabb_max, rect.center + scaled);
        if any(box_min > history_color) || any(history_color > box_max) {
            let clamped = clamp(history_color, box_min, box_max);
            // No luma instability nor reactive mask under Ultra Performance.
            let contribution = vec3(saturate(lock_contribution));
            history_color = mix(clamped, history_color, contribution);
            accumulation = mix(min(accumulation, vec3(0.1)), accumulation, contribution);
        }
        // Accumulate, tone-mapped as ASR does for HDR input.
        accumulation = max(vec3(EPSILON), accumulation + upsampled.www);
        let up = rgb_to_ycocg(tonemap(ycocg_to_rgb(upsampled.xyz)));
        let past = rgb_to_ycocg(tonemap(ycocg_to_rgb(history_color)));
        color = inverse_tonemap(ycocg_to_rgb(mix(past, up, upsampled.www / accumulation)));
    }
    color = clamp(color, vec3(0.0), vec3(FP16_MAX));

    // ComputeTemporalReactiveFactor.
    var factor = min(0.99, reactive);
    factor = max(factor, mix(factor, 0.4, saturate(hr_velocity)));
    factor = max(factor * factor, depth_clip * 0.1);
    factor = select(factor, 1.0, is_new_sample);
    if saturate(hr_velocity * 10.0) >= 1.0 {
        factor = max(EPSILON, factor) * -1.0;
    }

    // StoreNewLocks(.., 0): this frame's locks are spent.
    new_locks[lock_index] = 0u;
    let alpha = textureLoad(input_color, lr, 0).a;
    return AccumulateOut(vec4(color, factor), vec4(lock, 0.0, 0.0), vec4(encode(color), alpha));
}
