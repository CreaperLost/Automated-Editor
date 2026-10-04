struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) local: vec2<f32>,
}

struct LayerParams {
    size_px: vec2<f32>,
    radius_px: f32,
    clip_mode: u32,
    shadow_blur_px: f32,
    shadow_opacity: f32,
    shadow_offset: vec2<f32>,
    pass_kind: u32,
    // 0: BGRA; 1: NV12, limited-range BT.709 (luma in layer_tex, chroma in layer_chroma).
    format: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var layer_tex: texture_2d<f32>;
@group(0) @binding(1) var layer_samp: sampler;
@group(0) @binding(2) var<uniform> params: LayerParams;
@group(0) @binding(3) var layer_chroma: texture_2d<f32>;

// The layer's colour at `uv`. NV12 is converted from limited-range BT.709 YCbCr; it has no
// alpha. (Sampled at level 0, which needs no derivatives, so it may run per pixel.)
fn layer_color(uv: vec2<f32>) -> vec4<f32> {
    if params.format == 1u {
        let y = (textureSampleLevel(layer_tex, layer_samp, uv, 0.0).r - 16.0 / 255.0) * (255.0 / 219.0);
        let c = (textureSampleLevel(layer_chroma, layer_samp, uv, 0.0).rg - vec2<f32>(128.0 / 255.0)) * (255.0 / 224.0);
        let rgb = vec3<f32>(y + 1.5748 * c.y, y - 0.1873 * c.x - 0.4681 * c.y, y + 1.8556 * c.x);
        return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
    }
    return textureSample(layer_tex, layer_samp, uv);
}

@vertex
fn vs_main(
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) local: vec2<f32>,
) -> VsOut {
    var out: VsOut;
    out.clip = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;
    out.local = local;
    return out;
}

fn sdf_rounded_rect(p: vec2<f32>, half: vec2<f32>, radius: f32) -> f32 {
    let r = max(min(radius, min(half.x, half.y)), 0.0);
    let q = abs(p) - (half - vec2<f32>(r, r));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sdf_circle(p: vec2<f32>, half: vec2<f32>) -> f32 {
    return length(p) - min(half.x, half.y);
}

fn sdf_squircle(p: vec2<f32>, half: vec2<f32>) -> f32 {
    let nx = select(0.0, abs(p.x) / half.x, half.x > 0.0);
    let ny = select(0.0, abs(p.y) / half.y, half.y > 0.0);
    let k = (nx * nx * nx * nx) + (ny * ny * ny * ny);
    let r = min(half.x, half.y);
    return pow(max(k, 0.0), 0.25) * r - r;
}

fn shape_sdf(p: vec2<f32>) -> f32 {
    let half = params.size_px * 0.5;
    switch params.clip_mode {
        case 2u: {
            return sdf_circle(p, half);
        }
        case 3u: {
            return sdf_squircle(p, half);
        }
        default: {
            return sdf_rounded_rect(p, half, params.radius_px);
        }
    }
}

fn shadow_coverage(sdf: f32, blur: f32) -> f32 {
    if blur <= 0.0 {
        return select(0.0, 1.0, sdf <= 0.0);
    }
    return 1.0 - clamp(sdf / blur, 0.0, 1.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if params.pass_kind == 1u {
        let p = (in.local - vec2<f32>(0.5)) * params.size_px - params.shadow_offset;
        let sdf = shape_sdf(p);
        let alpha = params.shadow_opacity * shadow_coverage(sdf, params.shadow_blur_px);
        return vec4<f32>(0.0, 0.0, 0.0, alpha);
    }
    let p = (in.local - vec2<f32>(0.5)) * params.size_px;
    let sdf = shape_sdf(p);
    if sdf > 0.0 {
        return vec4<f32>(0.0);
    }
    return layer_color(in.uv);
}
