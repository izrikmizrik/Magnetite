// Snapdragon Game Super Resolution 1 (`gfx/sgsr.rs`): the world, drawn smaller, onto the screen.
// Its colour maths runs in `hf`/`hv3` (gfx/half.rs); coordinates stay f32.
@group(0) @binding(0) var world: texture_2d<f32>;
@group(0) @binding(1) var world_sampler: sampler;

struct PostOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex fn vs_post(@builtin(vertex_index) i: u32) -> PostOut {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: PostOut;
    out.clip = vec4(p * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2(p.x, 1.0 - p.y);
    return out;
}

// Snapdragon Game Super Resolution 1 (Qualcomm, BSD-3-Clause; THIRD_PARTY_NOTICES.md), ported
// from sgsr1_shader_mobile_edge_direction.frag in its RGBA mode: the world drawn smaller, rebuilt at the
// screen's size in one pass by a 12-tap Lanczos-like filter on the green channel, with the edges
// sharpened. Changed for HDR's light (docs/hdr-passes.md): no clamp at 1 (rule 1), and the
// green held at 4096 with the weights scaled by 1/64, so that half float's sums stay finite.
const SGSR_EDGE_THRESHOLD = 8.0 / 255.0;
const SGSR_EDGE_SHARPNESS = 2.0;

fn sgsr_lanczos2(x: hf) -> hf {
    let a = x - 4.0;
    let b = x * a - a;
    // fastLanczos2 reaches 64 at 0: a 64th keeps twelve weights times light inside half float.
    return b * (a * a) * (1.0 / 64.0);
}

fn sgsr_weight(dx: hf, dy: hf, c: hf, spread: hf) -> hv2 {
    let x = (dx * dx + dy * dy) * 0.55 + clamp(abs(c) * spread, hf(0.0), hf(1.0));
    let w = sgsr_lanczos2(x);
    return hv2(w, w * c);
}

fn sgsr_greens(uv: vec2<f32>) -> hv4 {
    return min(hv4(textureGather(1, world, world_sampler, uv)), hv4(4096.0));
}

@fragment fn fs_sgsr1(in: PostOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(world));
    let texel = 1.0 / size;
    let sampled = textureSampleLevel(world, world_sampler, in.uv, 0.0);
    var color = hv3(sampled.rgb);
    let img = in.uv * size + vec2(-0.5, 0.5);
    let img_pixel = floor(img);
    var coord = img_pixel * texel;
    let pl = hv2(img - img_pixel);
    var left = sgsr_greens(coord);
    let green = min(color.g, hf(4096.0));
    let edge_vote = abs(left.z - left.y) + abs(green - left.y) + abs(green - left.z);
    if edge_vote > SGSR_EDGE_THRESHOLD {
        coord.x += texel.x;
        var right = sgsr_greens(coord + vec2(texel.x, 0.0));
        var up_down = hv4(sgsr_greens(coord + vec2(0.0, -texel.y)).wz, sgsr_greens(coord + vec2(0.0, texel.y)).yx);
        let mean = 0.25 * left.y + 0.25 * left.z + 0.25 * right.x + 0.25 * right.w;
        left -= hv4(mean);
        right -= hv4(mean);
        up_down -= hv4(mean);
        let w = green - mean;
        let sum = abs(left.x) + abs(left.y) + abs(left.z) + abs(left.w) + abs(right.x) + abs(right.y)
            + abs(right.z) + abs(right.w) + abs(up_down.x) + abs(up_down.y) + abs(up_down.z) + abs(up_down.w);
        let sum_mean = 10.14185 / sum;
        let spread = sum_mean * sum_mean;
        var awy = sgsr_weight(pl.x, pl.y + 1.0, up_down.x, spread);
        awy += sgsr_weight(pl.x - 1.0, pl.y + 1.0, up_down.y, spread);
        awy += sgsr_weight(pl.x - 1.0, pl.y - 2.0, up_down.z, spread);
        awy += sgsr_weight(pl.x, pl.y - 2.0, up_down.w, spread);
        awy += sgsr_weight(pl.x + 1.0, pl.y - 1.0, left.x, spread);
        awy += sgsr_weight(pl.x, pl.y - 1.0, left.y, spread);
        awy += sgsr_weight(pl.x, pl.y, left.z, spread);
        awy += sgsr_weight(pl.x + 1.0, pl.y, left.w, spread);
        awy += sgsr_weight(pl.x - 1.0, pl.y - 1.0, right.x, spread);
        awy += sgsr_weight(pl.x - 2.0, pl.y - 1.0, right.y, spread);
        awy += sgsr_weight(pl.x - 2.0, pl.y, right.z, spread);
        awy += sgsr_weight(pl.x - 1.0, pl.y, right.w, spread);
        let final_y = select(hf(0.0), awy.y / awy.x, awy.x != 0.0);
        let max_y = max(max(left.y, left.z), max(right.x, right.w));
        let min_y = min(min(left.y, left.z), min(right.x, right.w));
        var delta_y = clamp(hf(SGSR_EDGE_SHARPNESS) * final_y, min_y, max_y) - w;
        // Smooths high-contrast input, as SGSR does: the same step for light above 1.
        delta_y = clamp(delta_y, hf(-23.0 / 255.0), hf(23.0 / 255.0));
        color = max(color + delta_y, hv3(0.0));
    }
    return vec4(vec3<f32>(color), sampled.a);
}
