// The composited frame as limited-range BT.709 4:2:0 (NV12's two planes), for the encoder:
// the inverse of the NV12 conversion in composite.wgsl. Each pass draws one plane.

@group(0) @binding(0) var frame: texture_2d<f32>;

// One triangle over the whole target.
@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

fn luma(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn rgb_at(p: vec2<i32>) -> vec3<f32> {
    return textureLoad(frame, p, 0).rgb;
}

// Full size: the pixel under each target pixel.
@fragment
fn fs_luma(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let y = luma(rgb_at(vec2<i32>(at.xy)));
    return vec4<f32>((16.0 + 219.0 * y) / 255.0, 0.0, 0.0, 1.0);
}

// Half size: each 2x2 block's average colour (chroma sited at the block's centre, as the
// decode shader samples it).
@fragment
fn fs_chroma(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(at.xy) * 2;
    let rgb = (rgb_at(p) + rgb_at(p + vec2<i32>(1, 0)) + rgb_at(p + vec2<i32>(0, 1))
        + rgb_at(p + vec2<i32>(1, 1))) * 0.25;
    let y = luma(rgb);
    let cb = (rgb.b - y) / 1.8556;
    let cr = (rgb.r - y) / 1.5748;
    return vec4<f32>((128.0 + 224.0 * cb) / 255.0, (128.0 + 224.0 * cr) / 255.0, 0.0, 1.0);
}
