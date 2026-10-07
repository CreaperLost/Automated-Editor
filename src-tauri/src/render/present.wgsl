// Draws the preview into the window, under the transparent webview: the backgrounds the page
// gave up over the preview (filled rounded rectangles), then the composited frame.

struct Item {
    // x, y, width, height in window pixels.
    rect: vec4<f32>,
    // Premultiplied fill colour (ignored for the frame).
    color: vec4<f32>,
    // The window's size in pixels.
    surface: vec2<f32>,
    radius: f32,
    // 1 draws the frame, 0 the fill colour.
    textured: f32,
};

@group(0) @binding(0) var frame: texture_2d<f32>;
@group(0) @binding(1) var frame_sampler: sampler;
@group(0) @binding(2) var<uniform> item: Item;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) px: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    let corner = corners[index];
    let px = item.rect.xy + corner * item.rect.zw;
    var out: VsOut;
    out.position = vec4<f32>(
        px.x / item.surface.x * 2.0 - 1.0,
        1.0 - px.y / item.surface.y * 2.0,
        0.0,
        1.0,
    );
    out.px = px;
    out.uv = corner;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let half = item.rect.zw * 0.5;
    let radius = min(item.radius, min(half.x, half.y));
    let q = abs(in.px - (item.rect.xy + half)) - half + vec2<f32>(radius);
    let distance = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
    let coverage = clamp(0.5 - distance, 0.0, 1.0);
    // Four taps across the window pixel: the frame is usually drawn smaller than it was
    // composited, and one bilinear tap would skip texels of screen text.
    let step = 0.25 / item.rect.zw;
    let sampled = 0.25 * (
        textureSample(frame, frame_sampler, in.uv + vec2<f32>(-step.x, -step.y)) +
        textureSample(frame, frame_sampler, in.uv + vec2<f32>(step.x, -step.y)) +
        textureSample(frame, frame_sampler, in.uv + vec2<f32>(-step.x, step.y)) +
        textureSample(frame, frame_sampler, in.uv + vec2<f32>(step.x, step.y))
    );
    let color = select(item.color, vec4<f32>(sampled.rgb, 1.0), item.textured > 0.5);
    return color * coverage;
}
