// Snapdragon Game Super Resolution 2 (Qualcomm, BSD-3-Clause; THIRD_PARTY_NOTICES.md), the
// two-pass fragment variant ported from sgsr2_convert.fs and sgsr2_upscale.fs: a temporal
// upscaler (or, at Native, temporal anti-aliasing) that the world's jitter feeds (`gfx/sgsr.rs`).
//
// Horizon's side of SGSR 2's contract, as its README asks of an engine like it: the depth is
// reversed (1 near, 0 far), read here as standard depth (`1 - d`) so that the nearest is the
// smallest, as the shaders take it; NDC's y points up and the texture's down (`REQUEST_NDC_Y_UP`);
// the motion is the camera's (`clip_to_prev_clip`, TAA's reprojection). For HDR's light
// (docs/hdr-passes.md): the frame holds gamma light, which blends as stored (rule 3); the
// neighbourhood's sums run in f32, where squares of bright light would pass half float; no
// clamp at 1; and the history keeps each pixel's alpha, HDR's light code.

struct Params {
    // This frame's unjittered clip space to the previous frame's, for the world's reversed depth.
    clip_to_prev_clip: mat4x4<f32>,
    render_size: vec2<f32>,
    output_size: vec2<f32>,
    render_size_rcp: vec2<f32>,
    output_size_rcp: vec2<f32>,
    // This frame's jitter in rendered pixels, y down.
    jitter_offset: vec2<f32>,
    // The kernel's widest bias (the output's scale along x, below 2), and how far the box
    // widens for the scale (its area's cube, at most 20).
    scale_ratio: vec2<f32>,
    // tan(vertical fov / 2) * width / height.
    camera_fov_angle_hor: f32,
    // The history's share where it falls outside the box, while the camera stands still.
    min_lerp_contribution: f32,
    // 1 when the history is unusable.
    reset: f32,
    pad: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
// The world's depth through an unfilterable float binding (as TAA reads it).
@group(0) @binding(1) var input_depth: texture_2d<f32>;
@group(0) @binding(2) var point_sampler: sampler;
@group(0) @binding(3) var input_color: texture_2d<f32>;
@group(0) @binding(4) var motion_depth_clip: texture_2d<f32>;
@group(0) @binding(5) var prev_output: texture_2d<f32>;
@group(0) @binding(6) var linear_sampler: sampler;

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex fn vs_sgsr2(@builtin(vertex_index) i: u32) -> Out {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: Out;
    out.clip = vec4(p * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2(p.x, 1.0 - p.y);
    return out;
}

// Four depths around `uv`'s corner, as standard depth.
fn standard_depths(uv: vec2<f32>) -> vec4<f32> {
    return vec4(1.0) - textureGather(0, input_depth, point_sampler, uv);
}

fn min4(v: vec4<f32>) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}

// Pass 1, at the world's size: the camera's motion from the nearest depth around each pixel,
// and how much its depth stands apart from the neighbours' (the history's distrust).
@fragment fn fs_convert(in: Out) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let gather_coord = uv - vec2(0.5) * params.render_size_rcp;
    let btm_left = standard_depths(gather_coord);
    let btm_right = standard_depths(gather_coord + vec2(params.render_size_rcp.x * 2.0, 0.0));
    let top_left = standard_depths(gather_coord + vec2(0.0, params.render_size_rcp.y * 2.0));
    let top_right = standard_depths(gather_coord + params.render_size_rcp * 2.0);
    let max_c = min(min(min(btm_left.z, btm_right.w), top_left.y), top_right.x);
    let btm_left4 = min4(btm_left);
    let btm_left_max9 = min(top_left.x, min(min(max_c, btm_left4), btm_right.x));
    var depthclip = 0.0;
    if max_c < 1.0 - 1.0e-05 {
        let btm_right4 = min4(btm_right);
        let top_left4 = min4(top_left);
        let top_right4 = min4(top_right);
        let ksep = 1.37e-05;
        let kfov = params.camera_fov_angle_hor;
        let diagonal_length = length(params.render_size);
        let depthsep = ksep * kfov * diagonal_length * (1.0 - max_c);
        let epsilon = 1.19e-07;
        var wdepth = 0.0;
        wdepth += clamp(depthsep / (abs(max_c - btm_left4) + epsilon), 0.0, 1.0);
        wdepth += clamp(depthsep / (abs(max_c - btm_right4) + epsilon), 0.0, 1.0);
        wdepth += clamp(depthsep / (abs(max_c - top_left4) + epsilon), 0.0, 1.0);
        wdepth += clamp(depthsep / (abs(max_c - top_right4) + epsilon), 0.0, 1.0);
        depthclip = clamp(1.0 - wdepth * 0.25, 0.0, 1.0);
    }
    let screen = vec2(2.0 * uv.x - 1.0, 1.0 - 2.0 * uv.y);
    // The reprojection takes the world's own, reversed, depth.
    let prev_clip = params.clip_to_prev_clip * vec4(screen, 1.0 - btm_left_max9, 1.0);
    let motion = screen - prev_clip.xy / prev_clip.w;
    return vec4(motion, depthclip, 0.0);
}

fn fast_lanczos(base: f32) -> f32 {
    let y = base - 1.0;
    let y2 = y * y;
    return (0.75 * y + y2) * y2;
}

// One of the five taps around the jittered sample: its Lanczos share of the upsampled colour,
// and its Gaussian share of the box the history is clipped to.
struct Taps {
    upsampled: vec4<f32>,
    box_min: vec3<f32>,
    box_max: vec3<f32>,
    box_center: vec3<f32>,
    box_var: vec3<f32>,
    box_weight: f32,
};

fn tap(t: ptr<function, Taps>, color: vec3<f32>, offset: vec2<f32>, kernelbias2: f32, curvebias: f32) {
    let offset_dot = dot(offset, offset);
    let weight = fast_lanczos(clamp(offset_dot * kernelbias2, 0.0, 1.0));
    (*t).upsampled += vec4(color * weight, weight);
    let box_weight = exp(offset_dot * curvebias);
    (*t).box_min = min((*t).box_min, color);
    (*t).box_max = max((*t).box_max, color);
    let weighted = color * box_weight;
    (*t).box_center += weighted;
    (*t).box_var += color * weighted;
    (*t).box_weight += box_weight;
}

// Pass 2, at the output's size: the jittered frame upsampled and blended into the history.
@fragment fn fs_upscale(in: Out) -> @location(0) vec4<f32> {
    let biasmax = params.scale_ratio.x;
    let scalefactor = params.scale_ratio.y;
    let hr_uv = in.uv;
    let jitter_uv = clamp(hr_uv + params.jitter_offset * params.render_size_rcp, vec2(0.0), vec2(1.0));
    let last = vec2<i32>(params.render_size) - 1;
    let input_pos = min(vec2<i32>(jitter_uv * params.render_size), last);
    let mda = textureSampleLevel(motion_depth_clip, linear_sampler, jitter_uv, 0.0).xyz;
    let motion = mda.xy;
    let prev_uv = vec2(clamp(-0.5 * motion.x + hr_uv.x, 0.0, 1.0), clamp(0.5 * motion.y + hr_uv.y, 0.0, 1.0));
    let depthfactor = mda.z;
    let history = textureSampleLevel(prev_output, linear_sampler, prev_uv, 0.0).rgb;

    let biasmin = max(1.0, 0.3 + 0.3 * biasmax);
    let biasfactor = 0.25 * depthfactor;
    let kernelbias = mix(biasmax, biasmin, biasfactor) * 0.5;
    let kernelbias2 = kernelbias * kernelbias;
    let motion_viewport_len = length(motion * params.output_size);
    let curvebias = mix(-2.0, -3.0, clamp(motion_viewport_len * 0.02, 0.0, 1.0));
    let srcpos = vec2<f32>(input_pos) + vec2(0.5) - params.jitter_offset;
    let srcpos_output = srcpos - hr_uv * params.render_size;

    let centre = textureLoad(input_color, input_pos, 0);
    var t: Taps;
    t.upsampled = vec4(0.0);
    t.box_min = centre.rgb;
    t.box_max = centre.rgb;
    t.box_center = vec3(0.0);
    t.box_var = vec3(0.0);
    t.box_weight = 0.0;
    tap(&t, textureLoad(input_color, clamp(input_pos + vec2(0, 1), vec2(0), last), 0).rgb, srcpos_output + vec2(0.0, 1.0), kernelbias2, curvebias);
    tap(&t, textureLoad(input_color, clamp(input_pos + vec2(1, 0), vec2(0), last), 0).rgb, srcpos_output + vec2(1.0, 0.0), kernelbias2, curvebias);
    tap(&t, textureLoad(input_color, clamp(input_pos + vec2(-1, 0), vec2(0), last), 0).rgb, srcpos_output + vec2(-1.0, 0.0), kernelbias2, curvebias);
    tap(&t, centre.rgb, srcpos_output, kernelbias2, curvebias);
    tap(&t, textureLoad(input_color, clamp(input_pos + vec2(0, -1), vec2(0), last), 0).rgb, srcpos_output + vec2(0.0, -1.0), kernelbias2, curvebias);

    let box_weight = 1.0 / t.box_weight;
    let box_center = t.box_center * box_weight;
    let box_var = sqrt(abs(t.box_var * box_weight - box_center * box_center));
    var upsampled = clamp(t.upsampled.xyz / t.upsampled.w, t.box_min - vec3(0.075), t.box_max + vec3(0.075));
    upsampled = clamp(upsampled, vec3(0.0), vec3(65504.0));
    let upsampled_w = t.upsampled.w * (1.0 / 3.0);

    var baseupdate = 1.0 - depthfactor;
    baseupdate = min(baseupdate, mix(baseupdate, upsampled_w * 10.0, clamp(10.0 * motion_viewport_len, 0.0, 1.0)));
    baseupdate = min(baseupdate, mix(baseupdate, upsampled_w, clamp(motion_viewport_len * 0.05, 0.0, 1.0)));
    var basealpha = baseupdate;
    let boxscale = max(depthfactor, clamp(motion_viewport_len * 0.05, 0.0, 1.0));
    let boxsize = mix(scalefactor, 1.0, boxscale);
    let sboxvar = box_var * boxsize;
    let box_max = min(t.box_max, box_center + sboxvar);
    let box_min = max(t.box_min, box_center - sboxvar);

    // The history clipped to the box, and blended in the passes' precision (gfx/half.rs).
    let past = hv3(min(history, vec3(65504.0)));
    let lo = hv3(clamp(box_min, vec3(-65504.0), vec3(65504.0)));
    let hi = hv3(clamp(box_max, vec3(-65504.0), vec3(65504.0)));
    let clamped = clamp(past, lo, hi);
    var start_lerp = params.min_lerp_contribution;
    if abs(mda.x) + abs(mda.y) > 0.000001 {
        start_lerp = 0.0;
    }
    let outside = any(lo > past) || any(past > hi);
    let lerpcontribution = select(1.0, start_lerp, outside);
    let kept = mix(clamped, past, hf(clamp(lerpcontribution, 0.0, 1.0)));
    let basemin = min(basealpha, 0.1);
    basealpha = mix(basemin, basealpha, clamp(lerpcontribution, 0.0, 1.0));
    let alphasum = max(1.192e-07, basealpha + upsampled_w);
    let alpha = clamp(upsampled_w / alphasum + params.reset, 0.0, 1.0);
    let blended = mix(kept, hv3(upsampled), hf(alpha));
    return vec4(max(vec3<f32>(blended), vec3(0.0)), centre.a);
}
