// SMAA 1x (Jimenez, Echevarria, Sousa, Gutierrez; MIT, THIRD_PARTY_NOTICES.md), ported from
// SMAA.hlsl: luma edge detection, then the blending weights from the edges' shapes through the
// precomputed area and search textures, then each pixel blended with the neighbour across its
// edge. Three passes at the world's size (`gfx/smaa.rs`). The edge and blending passes do their
// colour maths in `hf` (gfx/half.rs); the weights pass works in texture coordinates, f32.
//
// The frame holds vanilla's gamma, which luma edge detection wants (docs/hdr-passes.md): light
// above 1 finds its edges as SDR's does, and the blend keeps it (no clamp, rule 1). The alpha is
// HDR's light code, not coverage: each pixel keeps its own.

// The preset (`SmaaQuality`): SMAA.hlsl's LOW, MEDIUM, HIGH and ULTRA.
override THRESHOLD: f32 = 0.1;
override MAX_SEARCH_STEPS: f32 = 16.0;
override MAX_SEARCH_STEPS_DIAG: f32 = 8.0;
override DIAG: bool = true;
override CORNER_ROUNDING: f32 = 25.0;
override CORNERS: bool = true;

const LOCAL_CONTRAST_ADAPTATION_FACTOR = 2.0;
const AREATEX_MAX_DISTANCE = 16.0;
const AREATEX_MAX_DISTANCE_DIAG = 20.0;
const AREATEX_PIXEL_SIZE = vec2(1.0 / 160.0, 1.0 / 560.0);
const AREATEX_SUBTEX_SIZE = 1.0 / 7.0;
const SEARCHTEX_SIZE = vec2(66.0, 33.0);
const SEARCHTEX_PACKED_SIZE = vec2(64.0, 16.0);

// The world's colour (edges, blending), the edges and the weights between passes, and SMAA's
// two lookup textures. Each pass binds what it reads.
@group(0) @binding(0) var color: texture_2d<f32>;
@group(0) @binding(1) var linear_sampler: sampler;
@group(0) @binding(2) var edges_tex: texture_2d<f32>;
@group(0) @binding(3) var area_tex: texture_2d<f32>;
@group(0) @binding(4) var search_tex: texture_2d<f32>;
@group(0) @binding(5) var blend_tex: texture_2d<f32>;

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex fn vs_smaa(@builtin(vertex_index) i: u32) -> Out {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: Out;
    out.clip = vec4(p * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2(p.x, 1.0 - p.y);
    return out;
}

// SMAA_RT_METRICS of a texture: one texel, and its size.
fn metrics(size: vec2<u32>) -> vec4<f32> {
    let s = vec2<f32>(size);
    return vec4(1.0 / s, s);
}

// ---------------------------------------------------------------------------------------------
// Pass 1: luma edge detection (SMAALumaEdgeDetectionPS).

// Luma at `p`, clamped to the texture. Held at 4096, far above any light with an edge to find,
// so that differences stay inside half float.
fn luma_at(p: vec2<i32>, size: vec2<i32>) -> hf {
    let c = hv3(textureLoad(color, clamp(p, vec2(0), size - 1), 0).rgb);
    return min(dot(c, hv3(0.2126, 0.7152, 0.0722)), hf(4096.0));
}

@fragment fn fs_edges(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(color));
    let p = vec2<i32>(position.xy);
    let l = luma_at(p, size);
    let left = luma_at(p + vec2(-1, 0), size);
    let top = luma_at(p + vec2(0, -1), size);
    let near = abs(hv2(l) - hv2(left, top));
    let edges = step(hv2(hf(THRESHOLD)), near);
    if dot(edges, hv2(1.0)) == 0.0 {
        discard;
    }
    let right = luma_at(p + vec2(1, 0), size);
    let bottom = luma_at(p + vec2(0, 1), size);
    var max_delta = max(near, abs(hv2(l) - hv2(right, bottom)));
    let far = abs(hv2(left, top) - hv2(luma_at(p + vec2(-2, 0), size), luma_at(p + vec2(0, -2), size)));
    max_delta = max(max_delta, far);
    let final_delta = max(max_delta.x, max_delta.y);
    // Local contrast adaptation: an edge much weaker than its neighbour's is no edge.
    let kept = edges * step(hv2(final_delta), hf(LOCAL_CONTRAST_ADAPTATION_FACTOR) * near);
    return vec4(vec2<f32>(kept), 0.0, 0.0);
}

// ---------------------------------------------------------------------------------------------
// Pass 2: blending weights (SMAABlendingWeightCalculationPS), in texture coordinates.

fn edge(uv: vec2<f32>) -> vec2<f32> {
    return textureSampleLevel(edges_tex, linear_sampler, uv, 0.0).rg;
}

// Two binary values from a bilinear access a quarter texel off (SMAADecodeDiagBilinearAccess).
fn decode_diag2(e: vec2<f32>) -> vec2<f32> {
    return round(vec2(e.x * abs(5.0 * e.x - 5.0 * 0.75), e.y));
}

fn decode_diag4(e: vec4<f32>) -> vec4<f32> {
    let rb = e.xz * abs(5.0 * e.xz - 5.0 * 0.75);
    return round(vec4(rb.x, e.y, rb.y, e.w));
}

fn search_diag1(uv: vec2<f32>, dir: vec2<f32>, rt: vec4<f32>, e: ptr<function, vec2<f32>>) -> vec2<f32> {
    var coord = vec4(uv, -1.0, 1.0);
    let t = vec3(rt.xy, 1.0);
    while coord.z < MAX_SEARCH_STEPS_DIAG - 1.0 && coord.w > 0.9 {
        let xyz = t * vec3(dir, 1.0) + coord.xyz;
        *e = edge(xyz.xy);
        coord = vec4(xyz, dot(*e, vec2(0.5)));
    }
    return coord.zw;
}

fn search_diag2(uv: vec2<f32>, dir: vec2<f32>, rt: vec4<f32>, e: ptr<function, vec2<f32>>) -> vec2<f32> {
    var coord = vec4(uv, -1.0, 1.0);
    coord.x += 0.25 * rt.x;
    let t = vec3(rt.xy, 1.0);
    while coord.z < MAX_SEARCH_STEPS_DIAG - 1.0 && coord.w > 0.9 {
        let xyz = t * vec3(dir, 1.0) + coord.xyz;
        *e = decode_diag2(edge(xyz.xy));
        coord = vec4(xyz, dot(*e, vec2(0.5)));
    }
    return coord.zw;
}

fn area_diag(dist: vec2<f32>, e: vec2<f32>, offset: f32) -> vec2<f32> {
    var texcoord = vec2(AREATEX_MAX_DISTANCE_DIAG) * e + dist;
    texcoord = AREATEX_PIXEL_SIZE * texcoord + 0.5 * AREATEX_PIXEL_SIZE;
    texcoord.x += 0.5;
    texcoord.y += AREATEX_SUBTEX_SIZE * offset;
    return textureSampleLevel(area_tex, linear_sampler, texcoord, 0.0).rg;
}

fn diag_weights(uv: vec2<f32>, e: vec2<f32>, rt: vec4<f32>) -> vec2<f32> {
    var weights = vec2(0.0);
    var d = vec4(0.0);
    var end = vec2(0.0);
    if e.r > 0.0 {
        let s = search_diag1(uv, vec2(-1.0, 1.0), rt, &end);
        d.x = s.x + f32(end.y > 0.9);
        d.z = s.y;
    }
    let s = search_diag1(uv, vec2(1.0, -1.0), rt, &end);
    d.y = s.x;
    d.w = s.y;
    if d.x + d.y > 2.0 {
        let coords = vec4(-d.x + 0.25, d.x, d.y, -d.y - 0.25) * rt.xyxy + uv.xyxy;
        let c = decode_diag4(vec4(edge(coords.xy + vec2(-1.0, 0.0) * rt.xy), edge(coords.zw + vec2(1.0, 0.0) * rt.xy))).yxwz;
        var cc = vec2(2.0) * c.xz + c.yw;
        cc = select(cc, vec2(0.0), d.zw >= vec2(0.9));
        weights += area_diag(d.xy, cc, 0.0);
    }
    let s2 = search_diag2(uv, vec2(-1.0, -1.0), rt, &end);
    d.x = s2.x;
    d.z = s2.y;
    if edge(uv + vec2(1.0, 0.0) * rt.xy).r > 0.0 {
        let s3 = search_diag2(uv, vec2(1.0, 1.0), rt, &end);
        d.y = s3.x + f32(end.y > 0.9);
        d.w = s3.y;
    } else {
        d.y = 0.0;
        d.w = 0.0;
    }
    if d.x + d.y > 2.0 {
        let coords = vec4(-d.x, -d.x, d.y, d.y) * rt.xyxy + uv.xyxy;
        let gr = edge(coords.zw + vec2(1.0, 0.0) * rt.xy).gr;
        let c = vec4(edge(coords.xy + vec2(-1.0, 0.0) * rt.xy).g, edge(coords.xy + vec2(0.0, -1.0) * rt.xy).r, gr);
        var cc = vec2(2.0) * c.xz + c.yw;
        cc = select(cc, vec2(0.0), d.zw >= vec2(0.9));
        weights += area_diag(d.xy, cc, 0.0).gr;
    }
    return weights;
}

// How much length the last step of a search adds: 0, 1 or 2 (SMAASearchLength).
fn search_length(e: vec2<f32>, offset: f32) -> f32 {
    let scale = (SEARCHTEX_SIZE * vec2(0.5, -1.0) + vec2(-1.0, 1.0)) / SEARCHTEX_PACKED_SIZE;
    let bias = (SEARCHTEX_SIZE * vec2(offset, 1.0) + vec2(0.5, -0.5)) / SEARCHTEX_PACKED_SIZE;
    return textureSampleLevel(search_tex, linear_sampler, scale * e + bias, 0.0).r;
}

fn search_x_left(start: vec2<f32>, end: f32, rt: vec4<f32>) -> f32 {
    var uv = start;
    var e = vec2(0.0, 1.0);
    while uv.x > end && e.g > 0.8281 && e.r == 0.0 {
        e = edge(uv);
        uv = -vec2(2.0, 0.0) * rt.xy + uv;
    }
    let offset = -(255.0 / 127.0) * search_length(e, 0.0) + 3.25;
    return rt.x * offset + uv.x;
}

fn search_x_right(start: vec2<f32>, end: f32, rt: vec4<f32>) -> f32 {
    var uv = start;
    var e = vec2(0.0, 1.0);
    while uv.x < end && e.g > 0.8281 && e.r == 0.0 {
        e = edge(uv);
        uv = vec2(2.0, 0.0) * rt.xy + uv;
    }
    let offset = -(255.0 / 127.0) * search_length(e, 0.5) + 3.25;
    return -rt.x * offset + uv.x;
}

fn search_y_up(start: vec2<f32>, end: f32, rt: vec4<f32>) -> f32 {
    var uv = start;
    var e = vec2(1.0, 0.0);
    while uv.y > end && e.r > 0.8281 && e.g == 0.0 {
        e = edge(uv);
        uv = -vec2(0.0, 2.0) * rt.xy + uv;
    }
    let offset = -(255.0 / 127.0) * search_length(e.gr, 0.0) + 3.25;
    return rt.y * offset + uv.y;
}

fn search_y_down(start: vec2<f32>, end: f32, rt: vec4<f32>) -> f32 {
    var uv = start;
    var e = vec2(1.0, 0.0);
    while uv.y < end && e.r > 0.8281 && e.g == 0.0 {
        e = edge(uv);
        uv = vec2(0.0, 2.0) * rt.xy + uv;
    }
    let offset = -(255.0 / 127.0) * search_length(e.gr, 0.5) + 3.25;
    return -rt.y * offset + uv.y;
}

// The areas on each side of the edge, for its distances and crossing edges (SMAAArea).
fn area(dist: vec2<f32>, e1: f32, e2: f32, offset: f32) -> vec2<f32> {
    var texcoord = vec2(AREATEX_MAX_DISTANCE) * round(4.0 * vec2(e1, e2)) + dist;
    texcoord = AREATEX_PIXEL_SIZE * texcoord + 0.5 * AREATEX_PIXEL_SIZE;
    texcoord.y = AREATEX_SUBTEX_SIZE * offset + texcoord.y;
    return textureSampleLevel(area_tex, linear_sampler, texcoord, 0.0).rg;
}

fn horizontal_corners(weights: vec2<f32>, texcoord: vec4<f32>, d: vec2<f32>, rt: vec4<f32>) -> vec2<f32> {
    if !CORNERS {
        return weights;
    }
    let left_right = step(d.xy, d.yx);
    let rounding = (1.0 - CORNER_ROUNDING / 100.0) * left_right / (left_right.x + left_right.y);
    var factor = vec2(1.0);
    factor.x -= rounding.x * edge(texcoord.xy + vec2(0.0, 1.0) * rt.xy).r;
    factor.x -= rounding.y * edge(texcoord.zw + vec2(1.0, 1.0) * rt.xy).r;
    factor.y -= rounding.x * edge(texcoord.xy + vec2(0.0, -2.0) * rt.xy).r;
    factor.y -= rounding.y * edge(texcoord.zw + vec2(1.0, -2.0) * rt.xy).r;
    return weights * saturate(factor);
}

fn vertical_corners(weights: vec2<f32>, texcoord: vec4<f32>, d: vec2<f32>, rt: vec4<f32>) -> vec2<f32> {
    if !CORNERS {
        return weights;
    }
    let left_right = step(d.xy, d.yx);
    let rounding = (1.0 - CORNER_ROUNDING / 100.0) * left_right / (left_right.x + left_right.y);
    var factor = vec2(1.0);
    factor.x -= rounding.x * edge(texcoord.xy + vec2(1.0, 0.0) * rt.xy).g;
    factor.x -= rounding.y * edge(texcoord.zw + vec2(1.0, 1.0) * rt.xy).g;
    factor.y -= rounding.x * edge(texcoord.xy + vec2(-2.0, 0.0) * rt.xy).g;
    factor.y -= rounding.y * edge(texcoord.zw + vec2(-2.0, 1.0) * rt.xy).g;
    return weights * saturate(factor);
}

@fragment fn fs_weights(in: Out) -> @location(0) vec4<f32> {
    let rt = metrics(textureDimensions(edges_tex));
    let uv = in.uv;
    let pixcoord = uv * rt.zw;
    // The searches' starts, a quarter texel off to read two edges at once, and their ends.
    let off0 = rt.xyxy * vec4(-0.25, -0.125, 1.25, -0.125) + uv.xyxy;
    let off1 = rt.xyxy * vec4(-0.125, -0.25, -0.125, 1.25) + uv.xyxy;
    let off2 = rt.xxyy * vec4(-2.0, 2.0, -2.0, 2.0) * MAX_SEARCH_STEPS + vec4(off0.xz, off1.yw);
    var weights = vec4(0.0);
    var e = edge(uv);
    if e.g > 0.0 {
        // Diagonals come first: one found, the horizontal and vertical searches are skipped.
        var diag = vec2(0.0);
        if DIAG {
            diag = diag_weights(uv, e, rt);
        }
        if diag.x == -diag.y {
            var coords = vec3(search_x_left(off0.xy, off2.x, rt), off1.y, 0.0);
            var d = vec2(coords.x, 0.0);
            let e1 = edge(coords.xy).r;
            coords.z = search_x_right(off0.zw, off2.y, rt);
            d.y = coords.z;
            d = abs(round(rt.zz * d - pixcoord.xx));
            let e2 = edge(coords.zy + vec2(1.0, 0.0) * rt.xy).r;
            var w = area(sqrt(d), e1, e2, 0.0);
            coords.y = uv.y;
            w = horizontal_corners(w, coords.xyzy, d, rt);
            weights = vec4(w, weights.zw);
        } else {
            weights = vec4(diag, weights.zw);
            e.r = 0.0;
        }
    }
    if e.r > 0.0 {
        var coords = vec3(off0.x, search_y_up(off1.xy, off2.z, rt), 0.0);
        var d = vec2(coords.y, 0.0);
        let e1 = edge(coords.xy).g;
        coords.z = search_y_down(off1.zw, off2.w, rt);
        d.y = coords.z;
        d = abs(round(rt.ww * d - pixcoord.yy));
        let e2 = edge(coords.xz + vec2(0.0, 1.0) * rt.xy).g;
        var w = area(sqrt(d), e1, e2, 0.0);
        coords.x = uv.x;
        w = vertical_corners(w, coords.xyxz, d, rt);
        weights = vec4(weights.xy, w);
    }
    return weights;
}

// ---------------------------------------------------------------------------------------------
// Pass 3: neighbourhood blending (SMAANeighborhoodBlendingPS).

@fragment fn fs_blend(in: Out) -> @location(0) vec4<f32> {
    let rt = metrics(textureDimensions(color));
    let uv = in.uv;
    let centre = textureSampleLevel(color, linear_sampler, uv, 0.0);
    let offset = rt.xyxy * vec4(1.0, 0.0, 0.0, 1.0) + uv.xyxy;
    // Right, top, and this pixel's bottom and left.
    let here = textureSampleLevel(blend_tex, linear_sampler, uv, 0.0);
    let a = vec4(
        textureSampleLevel(blend_tex, linear_sampler, offset.xy, 0.0).a,
        textureSampleLevel(blend_tex, linear_sampler, offset.zw, 0.0).g,
        here.z,
        here.x,
    );
    if dot(a, vec4(1.0)) < 1e-5 {
        return centre;
    }
    let horizontal = max(a.x, a.z) > max(a.y, a.w);
    var blending_offset = vec4(0.0, a.y, 0.0, a.w);
    var blending_weight = a.yw;
    if horizontal {
        blending_offset = vec4(a.x, 0.0, a.z, 0.0);
        blending_weight = a.xz;
    }
    blending_weight /= dot(blending_weight, vec2(1.0));
    let coord = blending_offset * vec4(rt.xy, -rt.xy) + uv.xyxy;
    // Bilinear filtering mixes this pixel with the chosen neighbour.
    let c = hf(blending_weight.x) * hv3(textureSampleLevel(color, linear_sampler, coord.xy, 0.0).rgb)
        + hf(blending_weight.y) * hv3(textureSampleLevel(color, linear_sampler, coord.zw, 0.0).rgb);
    return vec4(vec3<f32>(c), centre.a);
}
