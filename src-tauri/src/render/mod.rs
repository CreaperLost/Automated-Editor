//! Offscreen WGPU compositor. One device, CPU upload + readback (bounded-copy fallback).
use crate::media::{validate_dim, ColorInfo, PixelFormat, VideoFrame, MAX_FRAME_DIM};
use crate::project::layout::{validate_wallpaper_relative, MAX_WALLPAPER_BYTES};
use crate::project::reader::{open_regular, safe_path};
use crate::project::EditLayout;
use bytemuck::{Pod, Zeroable};
use std::io::Read;
use std::path::Path;
use wgpu::util::DeviceExt;

pub const COPIES_COMPOSITE: u32 = 2;
/// Background, screen, webcam border, webcam, pointer and captions, plus an overlay for every
/// other video track a sequence may have (16 in all), with room to spare. A valid sequence
/// never runs out of layers.
pub const MAX_LAYERS: usize = 24;
const SHADER: &str = include_str!("composite.wgsl");
const TO_YUV: &str = include_str!("to_yuv.wgsl");
const WEBCAM_SHADOW_BLUR_PX: f32 = 16.0;
const WEBCAM_SHADOW_OPACITY: f32 = 0.55;
const SHADOW_OFFSET_FACTOR: f32 = 0.35;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    local: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LayerParams {
    size_px: [f32; 2],
    radius_px: f32,
    clip_mode: u32,
    shadow_blur_px: f32,
    shadow_opacity: f32,
    shadow_offset: [f32; 2],
    pass_kind: u32,
    /// 0: BGRA; 1: NV12 (limited-range BT.709), converted to RGB in the shader.
    format: u32,
    _pad: [u32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LayerRole {
    Background,
    #[default]
    Screen,
    WebcamBorder,
    Webcam,
    /// A clip on a video track above the main sequence; straight alpha, so a logo with a
    /// transparent background shows what is below.
    Overlay,
    /// Straight-alpha text over everything else.
    Caption,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ClipMode {
    #[default]
    None = 0,
    RoundedRect = 1,
    Circle = 2,
    Squircle = 3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub frame: VideoFrame,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub uv_x: f32,
    pub uv_y: f32,
    pub uv_w: f32,
    pub uv_h: f32,
    pub role: LayerRole,
    pub clip: ClipMode,
    pub radius_px: f32,
    pub shadow_blur_px: f32,
    pub shadow_opacity: f32,
    pub shadow_offset: [f32; 2],
    /// Set when the pixels are the same on every frame (the background); the compositor then
    /// keeps the uploaded texture instead of uploading it again.
    pub cache_key: Option<u64>,
}

impl Layer {
    pub fn placed(frame: VideoFrame, x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            frame,
            x,
            y,
            width,
            height,
            uv_x: 0.0,
            uv_y: 0.0,
            uv_w: 1.0,
            uv_h: 1.0,
            role: LayerRole::Screen,
            clip: ClipMode::None,
            radius_px: 0.0,
            shadow_blur_px: 0.0,
            shadow_opacity: 0.0,
            shadow_offset: [0.0, 0.0],
            cache_key: None,
        }
    }

    pub fn with_role(mut self, role: LayerRole) -> Self {
        self.role = role;
        self
    }

    pub fn with_clip(mut self, clip: ClipMode, radius_px: f32) -> Self {
        self.clip = clip;
        self.radius_px = radius_px.max(0.0);
        self
    }

    pub fn with_shadow(mut self, blur_px: f32, opacity: f32) -> Self {
        self.shadow_blur_px = blur_px.max(0.0);
        self.shadow_opacity = opacity.clamp(0.0, 1.0);
        self.shadow_offset = [0.0, self.shadow_blur_px * SHADOW_OFFSET_FACTOR];
        self
    }

    pub fn mirrored(mut self) -> Self {
        self.uv_x += self.uv_w;
        self.uv_w = -self.uv_w;
        self
    }

    pub fn cover_uv(mut self, dest_w: u32, dest_h: u32) -> Self {
        let (uv_x, uv_y, uv_w, uv_h) =
            cover_uv(self.frame.width, self.frame.height, dest_w, dest_h);
        self.uv_x = uv_x;
        self.uv_y = uv_y;
        self.uv_w = uv_w;
        self.uv_h = uv_h;
        self
    }

    fn has_shadow(&self) -> bool {
        self.shadow_blur_px > 0.0 && self.shadow_opacity > 0.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub width: u32,
    pub height: u32,
    pub background: [f32; 4],
    pub layers: Vec<Layer>,
}

/// The webcam bubble for `layout`: its border ring (if any), then the camera picture.
fn webcam_layers(
    width: u32,
    height: u32,
    webcam: VideoFrame,
    layout: &EditLayout,
    unit: f32,
) -> Result<Vec<Layer>, String> {
    let px = |value: u32| (value as f32 * unit).round() as u32;
    let mut layers = Vec::new();
    let (bw, bh) = webcam_bubble_size(width, height, webcam.width, webcam.height, layout);
    let (x, y) = webcam_origin(width, height, bw, bh, layout, unit);
    let (cam_clip, cam_radius) = webcam_clip(layout, bw, bh);
    let cam_shadow = if layout.webcam_shadow {
        Some((WEBCAM_SHADOW_BLUR_PX * unit, WEBCAM_SHADOW_OPACITY))
    } else {
        None
    };
    let inset = if layout.webcam_border_width > 0 {
        px(layout.webcam_border_width).max(1)
    } else {
        0
    };
    if inset > 0 {
        let bx = x.saturating_sub(inset);
        let by = y.saturating_sub(inset);
        let bwidth = (bw + inset * 2).min(width.saturating_sub(bx)).max(1);
        let bheight = (bh + inset * 2).min(height.saturating_sub(by)).max(1);
        let [r, g, b] = crate::project::layout::parse_hex_rgb(&layout.webcam_border_color)?;
        let border = VideoFrame::solid(8, 8, b, g, r, 0)?;
        // The ring stays concentric with the rounded bubble inside it.
        let border_radius = if cam_radius > 0.0 {
            cam_radius + inset as f32
        } else {
            0.0
        };
        let mut border_layer = Layer::placed(border, bx, by, bwidth, bheight)
            .with_role(LayerRole::WebcamBorder)
            .with_clip(cam_clip, border_radius);
        if let Some((blur, opacity)) = cam_shadow {
            border_layer = border_layer.with_shadow(blur, opacity);
        }
        layers.push(border_layer);
    }
    let mut bubble = Layer::placed(webcam, x, y, bw, bh)
        .with_role(LayerRole::Webcam)
        .with_clip(cam_clip, cam_radius);
    if matches!(layout.webcam_shape.as_str(), "circle" | "squircle") {
        bubble = bubble.cover_uv(bw, bh);
    }
    if inset == 0 {
        if let Some((blur, opacity)) = cam_shadow {
            bubble = bubble.with_shadow(blur, opacity);
        }
    }
    if layout.webcam_mirror {
        bubble = bubble.mirrored();
    }
    layers.push(bubble);
    Ok(layers)
}

impl Scene {
    pub fn styled_preview(screen: VideoFrame, webcam: Option<VideoFrame>) -> Result<Self, String> {
        validate_dim(64, 64)?;
        let mut layers = vec![Layer::placed(screen, 12, 16, 40, 32)];
        if let Some(webcam) = webcam {
            layers.push(Layer::placed(webcam, 46, 4, 12, 12));
        }
        Ok(Self {
            width: 64,
            height: 64,
            background: [0.05, 0.12, 0.28, 1.0],
            layers,
        })
    }

    pub fn from_layout(
        width: u32,
        height: u32,
        layout: &EditLayout,
        screen: Option<VideoFrame>,
        webcam: Option<VideoFrame>,
    ) -> Result<Self, String> {
        Self::from_layout_with_wallpaper(width, height, layout, screen, webcam, None)
    }

    pub fn from_layout_with_wallpaper(
        width: u32,
        height: u32,
        layout: &EditLayout,
        screen: Option<VideoFrame>,
        webcam: Option<VideoFrame>,
        wallpaper: Option<VideoFrame>,
    ) -> Result<Self, String> {
        Self::from_layout_scaled(width, height, layout, screen, webcam, wallpaper, 1.0)
    }

    /// Build the scene with layout pixel values multiplied by `unit`; see [`layout_px_unit`].
    pub fn from_layout_scaled(
        width: u32,
        height: u32,
        layout: &EditLayout,
        screen: Option<VideoFrame>,
        webcam: Option<VideoFrame>,
        wallpaper: Option<VideoFrame>,
        unit: f32,
    ) -> Result<Self, String> {
        validate_dim(width, height)?;
        layout.validate()?;
        let unit = if unit.is_finite() && unit > 0.0 {
            unit
        } else {
            1.0
        };
        let px = |value: u32| (value as f32 * unit).round() as u32;
        let padding = px(layout.padding_px);
        if padding.saturating_mul(2) >= width || padding.saturating_mul(2) >= height {
            return Err("Export padding leaves no content rectangle".into());
        }
        let (background, background_end) = layout.background_rgba()?;
        let mut layers = Vec::new();
        let generated = match (&wallpaper, layout.background_type.as_str()) {
            (None, "preset") => Some(preset_frame(width, height, &layout.background_preset)?),
            _ => None,
        };
        if let Some(paper) = wallpaper.or(generated) {
            layers.push(
                Layer::placed(paper, 0, 0, width, height)
                    .with_role(LayerRole::Background)
                    .cover_uv(width, height),
            );
        } else if let Some(end) = background_end {
            let gradient = gradient_frame(width, height, background, end)?;
            layers.push(
                Layer::placed(gradient, 0, 0, width, height).with_role(LayerRole::Background),
            );
        }
        if let Some(screen) = screen {
            let (crop_x, crop_y, crop_w, crop_h) = layout.screen_crop_uv();
            let scale = layout.screen_scale_pct.clamp(1.0, 100.0) / 100.0;
            let area_w = width - padding * 2;
            let area_h = height - padding * 2;
            let box_w = ((area_w as f32 * scale).round() as u32).clamp(1, area_w);
            let box_h = ((area_h as f32 * scale).round() as u32).clamp(1, area_h);
            // The kept region keeps the scale of the uncropped screen (re-fitting it would
            // enlarge it, so cropping one side looked like cropping both) and is centred
            // in the content area at its new size.
            let (_, _, full_w, full_h) =
                fit_inside(screen.width, screen.height, 0, 0, box_w, box_h);
            let span = |size: u32, start: f32, length: f32| {
                let a = (size as f32 * start).round() as u32;
                let b = (size as f32 * (start + length)).round() as u32;
                b.saturating_sub(a).clamp(1, size.max(1))
            };
            let w = span(full_w, crop_x, crop_w);
            let h = span(full_h, crop_y, crop_h);
            let x = padding + (area_w - w) / 2;
            let y = padding + (area_h - h) / 2;
            let radius = layout.corner_radius_px as f32 * unit;
            let clip = if radius > 0.0 {
                ClipMode::RoundedRect
            } else {
                ClipMode::None
            };
            let mut layer = Layer::placed(screen, x, y, w, h)
                .with_role(LayerRole::Screen)
                .with_clip(clip, radius);
            layer.uv_x = crop_x;
            layer.uv_y = crop_y;
            layer.uv_w = crop_w;
            layer.uv_h = crop_h;
            if layout.shadow_blur_px > 0 && layout.shadow_opacity > 0.0 {
                layer =
                    layer.with_shadow(layout.shadow_blur_px as f32 * unit, layout.shadow_opacity);
            }
            layers.push(layer);
        }
        if layout.webcam_enabled {
            if let Some(webcam) = webcam {
                layers.extend(webcam_layers(width, height, webcam, layout, unit)?);
            }
        }
        if layers.len() > MAX_LAYERS {
            return Err("Compositor layer count exceeds the F2 bound".into());
        }
        Ok(Self {
            width,
            height,
            background,
            layers,
        })
    }

    /// Adds a caption on top of everything else, if the scene has room for one more layer.
    pub fn push_caption(&mut self, frame: VideoFrame, x: u32, y: u32) {
        if self.layers.len() >= MAX_LAYERS || x >= self.width || y >= self.height {
            return;
        }
        let width = frame.width.min(self.width - x);
        let height = frame.height.min(self.height - y);
        let mut layer = Layer::placed(frame, x, y, width, height).with_role(LayerRole::Caption);
        layer.uv_w = width as f32 / layer.frame.width as f32;
        layer.uv_h = height as f32 / layer.frame.height as f32;
        self.layers.push(layer);
    }

    /// A track clip over the whole canvas, under the captions: fitted inside it (`cover` false)
    /// or filling it with the picture's edges cropped.
    /// A camera clip from a track above V1, drawn in the webcam bubble of `layout`, under
    /// the captions.
    pub fn push_webcam_bubble(
        &mut self,
        frame: VideoFrame,
        layout: &EditLayout,
        unit: f32,
    ) -> Result<(), String> {
        if frame.width == 0 || frame.height == 0 {
            return Ok(());
        }
        let layers = webcam_layers(self.width, self.height, frame, layout, unit)?;
        if self.layers.len() + layers.len() > MAX_LAYERS {
            return Ok(());
        }
        let at = self
            .layers
            .iter()
            .position(|l| l.role == LayerRole::Caption)
            .unwrap_or(self.layers.len());
        self.layers.splice(at..at, layers);
        Ok(())
    }

    /// Moves the layers added since `from` (a track below V1) to just above the background.
    pub fn move_under_main(&mut self, from: usize) {
        let added: Vec<Layer> = self.layers.drain(from..).collect();
        let at = self
            .layers
            .iter()
            .position(|l| l.role != LayerRole::Background)
            .unwrap_or(self.layers.len());
        // Captions stay on top: they were behind the added layers, so keep them at the end.
        let captions: Vec<Layer> = {
            let mut kept = Vec::new();
            let mut i = 0;
            while i < self.layers.len() {
                if self.layers[i].role == LayerRole::Caption {
                    kept.push(self.layers.remove(i));
                } else {
                    i += 1;
                }
            }
            kept
        };
        let at = at.min(self.layers.len());
        self.layers.splice(at..at, added);
        self.layers.extend(captions);
    }

    /// Draws the pointer `frame` over the screen picture: its tip (`hotspot`, source pixels
    /// into the picture) at recorded position `at` (0..1 of the recorded screen), sized
    /// `size` source pixels, scaled with the screen and its zoom, cut to the screen's area.
    pub fn push_cursor(
        &mut self,
        frame: VideoFrame,
        at: (f64, f64),
        hotspot: (f64, f64),
        size: (f64, f64),
        source: (f64, f64),
        cache_key: u64,
    ) {
        let Some(index) = self.layers.iter().position(|l| l.role == LayerRole::Screen) else {
            return;
        };
        if self.layers.len() >= MAX_LAYERS || frame.width == 0 || frame.height == 0 {
            return;
        }
        let screen = &self.layers[index];
        let (sx, sy, sw, sh) = (
            screen.x as f64,
            screen.y as f64,
            screen.width as f64,
            screen.height as f64,
        );
        let (uv_x, uv_y, uv_w, uv_h) = (
            screen.uv_x as f64,
            screen.uv_y as f64,
            screen.uv_w.max(1e-6) as f64,
            screen.uv_h.max(1e-6) as f64,
        );
        // Canvas pixels per recorded pixel, zoom included.
        let per_x = sw / (uv_w * source.0.max(1.0));
        let per_y = sh / (uv_h * source.1.max(1.0));
        let tip_x = sx + (at.0 - uv_x) / uv_w * sw;
        let tip_y = sy + (at.1 - uv_y) / uv_h * sh;
        if tip_x < sx || tip_y < sy || tip_x > sx + sw || tip_y > sy + sh {
            return;
        }
        let left = tip_x - hotspot.0 * per_x;
        let top = tip_y - hotspot.1 * per_y;
        let (w, h) = (size.0 * per_x, size.1 * per_y);
        // Cut to the screen's area, cropping the picture to match.
        let x0 = left.max(sx);
        let y0 = top.max(sy);
        let x1 = (left + w).min(sx + sw).min(self.width as f64);
        let y1 = (top + h).min(sy + sh).min(self.height as f64);
        if x1 - x0 < 1.0 || y1 - y0 < 1.0 {
            return;
        }
        let mut layer = Layer::placed(
            frame,
            x0.round() as u32,
            y0.round() as u32,
            ((x1 - x0).round() as u32).max(1),
            ((y1 - y0).round() as u32).max(1),
        )
        .with_role(LayerRole::Overlay);
        layer.uv_x = ((x0 - left) / w) as f32;
        layer.uv_y = ((y0 - top) / h) as f32;
        layer.uv_w = ((x1 - x0) / w) as f32;
        layer.uv_h = ((y1 - y0) / h) as f32;
        layer.cache_key = Some(cache_key);
        // Right over the screen: under the camera, the tracks above and the captions.
        self.layers.insert(index + 1, layer);
    }

    /// `cache_key` names a picture that is the same on every frame (a still), so the GPU can
    /// keep it rather than upload it again.
    pub fn push_overlay(&mut self, frame: VideoFrame, cover: bool, cache_key: Option<u64>) {
        if self.layers.len() >= MAX_LAYERS || frame.width == 0 || frame.height == 0 {
            return;
        }
        let (canvas_w, canvas_h) = (self.width, self.height);
        let layer = if cover {
            Layer::placed(frame, 0, 0, canvas_w, canvas_h).cover_uv(canvas_w, canvas_h)
        } else {
            let scale =
                (canvas_w as f64 / frame.width as f64).min(canvas_h as f64 / frame.height as f64);
            let w = ((frame.width as f64 * scale).round() as u32).clamp(1, canvas_w);
            let h = ((frame.height as f64 * scale).round() as u32).clamp(1, canvas_h);
            Layer::placed(frame, (canvas_w - w) / 2, (canvas_h - h) / 2, w, h)
        }
        .with_role(LayerRole::Overlay);
        let mut layer = layer;
        layer.cache_key = cache_key;
        let at = self
            .layers
            .iter()
            .position(|l| l.role == LayerRole::Caption)
            .unwrap_or(self.layers.len());
        self.layers.insert(at, layer);
    }

    pub fn apply_screen_uv(&mut self, uv_x: f32, uv_y: f32, uv_w: f32, uv_h: f32) {
        if let Some(layer) = self
            .layers
            .iter_mut()
            .find(|layer| layer.role == LayerRole::Screen)
        {
            layer.uv_x = uv_x;
            layer.uv_y = uv_y;
            layer.uv_w = uv_w;
            layer.uv_h = uv_h;
        }
    }

    /// Grows the webcam from its bubble towards a centered rectangle covering `size_pct` of
    /// the canvas. `weight` is the eased progress: 0 leaves the bubble untouched, 1 is the
    /// enlarged webcam. Position, size, corner radius and the source crop all interpolate, so
    /// the move is continuous; the border thins out and the shadow fades as it grows.
    pub fn apply_webcam_focus(
        &mut self,
        layout: &EditLayout,
        size_pct: f32,
        weight: f32,
        unit: f32,
    ) {
        let t = if weight.is_finite() {
            weight.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if t <= 0.0 {
            return;
        }
        let Some(index) = self.layers.iter().position(|l| l.role == LayerRole::Webcam) else {
            return;
        };
        let (canvas_w, canvas_h) = (self.width as f32, self.height as f32);
        let frac = size_pct.clamp(40.0, 100.0) / 100.0;
        let (target_w, target_h) = (
            (canvas_w * frac).round().max(1.0),
            (canvas_h * frac).round().max(1.0),
        );
        let target_x = ((canvas_w - target_w) / 2.0).round();
        let target_y = ((canvas_h - target_h) / 2.0).round();
        // Full bleed has square corners; a smaller focus matches the screen's rounding.
        let target_radius = if frac < 1.0 {
            layout.corner_radius_px as f32 * unit
        } else {
            0.0
        };
        let lerp = |a: f32, b: f32| a + (b - a) * t;

        let cam = &self.layers[index];
        let (bx, by, bw, bh) = (
            cam.x as f32,
            cam.y as f32,
            cam.width as f32,
            cam.height as f32,
        );
        let bubble_radius = match cam.clip {
            ClipMode::None => 0.0,
            ClipMode::RoundedRect => cam.radius_px,
            ClipMode::Circle => bw.min(bh) / 2.0,
            // The closest rounded rectangle; the shapes differ by a hair at the first frame.
            ClipMode::Squircle => bw.min(bh) * 0.3,
        };
        let (mut tu_x, tu_y, mut tu_w, tu_h) = cover_uv(
            cam.frame.width,
            cam.frame.height,
            target_w as u32,
            target_h as u32,
        );
        if layout.webcam_mirror {
            tu_x += tu_w;
            tu_w = -tu_w;
        }
        let x = lerp(bx, target_x).round().clamp(0.0, canvas_w - 1.0);
        let y = lerp(by, target_y).round().clamp(0.0, canvas_h - 1.0);
        let w = lerp(bw, target_w).round().clamp(1.0, canvas_w - x);
        let h = lerp(bh, target_h).round().clamp(1.0, canvas_h - y);
        let radius = lerp(bubble_radius, target_radius).min(w.min(h) / 2.0);
        let clip = if radius > 0.0 {
            ClipMode::RoundedRect
        } else {
            ClipMode::None
        };
        let fade = 1.0 - t;

        let cam = &mut self.layers[index];
        cam.uv_x = lerp(cam.uv_x, tu_x);
        cam.uv_y = lerp(cam.uv_y, tu_y);
        cam.uv_w = lerp(cam.uv_w, tu_w);
        cam.uv_h = lerp(cam.uv_h, tu_h);
        cam.x = x as u32;
        cam.y = y as u32;
        cam.width = w as u32;
        cam.height = h as u32;
        cam.clip = clip;
        cam.radius_px = radius;
        cam.shadow_opacity *= fade;

        if let Some(border) = self
            .layers
            .iter_mut()
            .find(|l| l.role == LayerRole::WebcamBorder)
        {
            let inset = (border.width as f32 - bw).max(0.0) / 2.0 * fade;
            let bx = (x - inset).round().max(0.0);
            let by = (y - inset).round().max(0.0);
            border.x = bx as u32;
            border.y = by as u32;
            border.width = (w + inset * 2.0).round().clamp(1.0, canvas_w - bx) as u32;
            border.height = (h + inset * 2.0).round().clamp(1.0, canvas_h - by) as u32;
            border.clip = clip;
            border.radius_px = if radius > 0.0 { radius + inset } else { 0.0 };
            border.shadow_opacity *= fade;
        }
    }
}

struct LayerDraw {
    _content_uniform: wgpu::Buffer,
    bind_content: wgpu::BindGroup,
    v_content: wgpu::Buffer,
    shadow: Option<(wgpu::BindGroup, wgpu::Buffer, wgpu::Buffer)>,
}

/// The render target and its readback buffer, kept while the canvas size stays the same.
struct TargetCache {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    staging: wgpu::Buffer,
    /// The NV12 planes for the encoder, made the first time they are asked for.
    yuv: Option<YuvTarget>,
}

/// The composited frame converted to NV12's two planes, and their readback (luma rows, then
/// chroma rows, each padded to the copy alignment).
struct YuvTarget {
    luma_view: wgpu::TextureView,
    luma: wgpu::Texture,
    chroma_view: wgpu::TextureView,
    chroma: wgpu::Texture,
    bind: wgpu::BindGroup,
    staging: wgpu::Buffer,
}

/// A layer texture kept for the layer in the same position on the next frame.
struct LayerSlot {
    width: u32,
    height: u32,
    format: PixelFormat,
    /// BGRA pixels, or NV12's luma plane.
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    /// NV12's half-size chroma plane.
    chroma: Option<(wgpu::Texture, wgpu::TextureView)>,
    /// The `cache_key` of the pixels last uploaded, if they were static.
    key: Option<u64>,
}

pub struct Compositor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    bind_layout: wgpu::BindGroupLayout,
    adapter_name: String,
    copies: u32,
    /// Creating textures and buffers for every frame cost more than drawing it.
    target: std::sync::Mutex<Option<TargetCache>>,
    slots: std::sync::Mutex<Vec<LayerSlot>>,
    /// Bound as the chroma plane of layers that have none (BGRA).
    no_chroma: wgpu::TextureView,
    /// Converting the composited frame to NV12 (see `to_yuv.wgsl`).
    yuv_layout: wgpu::BindGroupLayout,
    luma_pipeline: wgpu::RenderPipeline,
    chroma_pipeline: wgpu::RenderPipeline,
}

impl Compositor {
    pub fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("No GPU adapter for the F2 compositor: {e}"))?;
        let adapter_name = adapter.get_info().name;
        let mut desc = wgpu::DeviceDescriptor::default();
        desc.label = Some("aeroedits-compositor");
        let (device, queue) = pollster::block_on(adapter.request_device(&desc))
            .map_err(|e| format!("Failed to open the compositor device: {e}"))?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("aeroedits-composite"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("aeroedits-layer"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("aeroedits-composite-layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("aeroedits-composite-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x2
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let yuv_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("aeroedits-to-yuv"),
            source: wgpu::ShaderSource::Wgsl(TO_YUV.into()),
        });
        let yuv_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("aeroedits-to-yuv"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let yuv_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("aeroedits-to-yuv-layout"),
            bind_group_layouts: &[&yuv_layout],
            push_constant_ranges: &[],
        });
        let plane_pipeline = |entry: &str, format: wgpu::TextureFormat| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&yuv_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &yuv_shader,
                    entry_point: Some("vs_full"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &yuv_shader,
                    entry_point: Some(entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };
        let luma_pipeline = plane_pipeline("fs_luma", wgpu::TextureFormat::R8Unorm);
        let chroma_pipeline = plane_pipeline("fs_chroma", wgpu::TextureFormat::Rg8Unorm);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            // Bilinear: layers are almost never drawn 1:1 (padding, screen size, zoom), and
            // nearest sampling dropped or doubled whole pixel columns of screen text.
            label: Some("aeroedits-bilinear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let no_chroma = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("aeroedits-no-chroma"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rg8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self {
            device,
            queue,
            pipeline,
            sampler,
            bind_layout,
            adapter_name,
            copies: COPIES_COMPOSITE,
            target: std::sync::Mutex::new(None),
            slots: std::sync::Mutex::new(Vec::new()),
            no_chroma,
            yuv_layout,
            luma_pipeline,
            chroma_pipeline,
        })
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn copies(&self) -> u32 {
        self.copies
    }

    pub fn composite_cpu(scene: &Scene) -> Result<VideoFrame, String> {
        validate_dim(scene.width, scene.height)?;
        let r = (scene.background[0].clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (scene.background[1].clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (scene.background[2].clamp(0.0, 1.0) * 255.0).round() as u8;
        let mut frame = VideoFrame::solid(scene.width, scene.height, b, g, r, 0)?;
        for layer in &scene.layers {
            if layer.has_shadow() {
                blit_shadow(&mut frame, layer)?;
            }
            if layer.frame.format == PixelFormat::Bgra8888 {
                blit_bilinear(&mut frame, layer)?;
            } else {
                let bgra = Layer {
                    frame: layer.frame.to_bgra(),
                    ..layer.clone()
                };
                blit_bilinear(&mut frame, &bgra)?;
            }
        }
        if let Some(first) = scene.layers.first() {
            frame.pts_us = first.frame.pts_us;
        }
        Ok(frame)
    }

    /// The scene as BGRA.
    pub fn composite(&self, scene: &Scene) -> Result<VideoFrame, String> {
        self.render(scene, false)
    }

    /// The scene as NV12 in limited-range BT.709, as an H.264 encoder takes it: the GPU
    /// converts it, and reading it back moves 1.5 bytes a pixel instead of 4. Even sizes only.
    pub fn composite_nv12(&self, scene: &Scene) -> Result<VideoFrame, String> {
        if scene.width % 2 != 0 || scene.height % 2 != 0 {
            return Err("NV12 needs an even width and height".into());
        }
        self.render(scene, true)
    }

    /// The NV12 planes for a target of `target`'s size.
    fn yuv_target(&self, target: &TargetCache) -> YuvTarget {
        let plane = |label: &str, width: u32, height: u32, format: wgpu::TextureFormat| {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            (texture, view)
        };
        let (w, h) = (target.width, target.height);
        let (luma, luma_view) = plane("aeroedits-luma", w, h, wgpu::TextureFormat::R8Unorm);
        let (chroma, chroma_view) = plane(
            "aeroedits-chroma",
            w / 2,
            h / 2,
            wgpu::TextureFormat::Rg8Unorm,
        );
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("aeroedits-to-yuv-bind"),
            layout: &self.yuv_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&target.view),
            }],
        });
        // Both planes have `w` bytes a row (chroma: w/2 pairs).
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aeroedits-yuv-readback"),
            size: u64::from(aligned_row(w)) * u64::from(h + h / 2),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        YuvTarget {
            luma_view,
            luma,
            chroma_view,
            chroma,
            bind,
            staging,
        }
    }

    fn render(&self, scene: &Scene, nv12: bool) -> Result<VideoFrame, String> {
        validate_dim(scene.width, scene.height)?;
        if scene.layers.len() > MAX_LAYERS {
            return Err("Compositor layer count exceeds the F2 bound".into());
        }
        let padded = padded_bytes_per_row(scene.width);
        let mut target_cache = self.target.lock().unwrap_or_else(|e| e.into_inner());
        if !target_cache
            .as_ref()
            .is_some_and(|t| (t.width, t.height) == (scene.width, scene.height))
        {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("aeroedits-target"),
                size: wgpu::Extent3d {
                    width: scene.width,
                    height: scene.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                // Read by the NV12 conversion.
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("aeroedits-readback"),
                size: u64::from(padded * scene.height),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            *target_cache = Some(TargetCache {
                width: scene.width,
                height: scene.height,
                texture,
                view,
                staging,
                yuv: None,
            });
        }
        if nv12 {
            let cached = target_cache.as_mut().unwrap();
            if cached.yuv.is_none() {
                cached.yuv = Some(self.yuv_target(cached));
            }
        }
        let cached = target_cache.as_ref().unwrap();
        let (target, view) = (&cached.texture, &cached.view);
        let yuv = cached.yuv.as_ref().filter(|_| nv12);
        let mut slots = self.slots.lock().unwrap_or_else(|e| e.into_inner());
        slots.truncate(scene.layers.len());
        let mut draws = Vec::new();
        for (index, layer) in scene.layers.iter().enumerate() {
            validate_dim(layer.frame.width, layer.frame.height)?;
            if layer.frame.width > MAX_FRAME_DIM || layer.frame.height > MAX_FRAME_DIM {
                return Err("Layer exceeds the compositor working-set limit".into());
            }
            // BGRA layers upload as they are (the target is BGRA too, so nothing is swapped);
            // NV12 uploads its two planes, and the shader converts them.
            let format = layer.frame.format;
            let nv12 = format == PixelFormat::Nv12;
            let row_bytes = layer.frame.width as usize * if nv12 { 1 } else { 4 };
            let needed = layer.frame.byte_len();
            if (layer.frame.stride as usize) < row_bytes
                || layer.frame.data.len() < needed
                || (nv12 && (layer.frame.width % 2 != 0 || layer.frame.height % 2 != 0))
            {
                return Err("Layer frame buffer is truncated".into());
            }
            let size = (layer.frame.width, layer.frame.height);
            let new_texture =
                |label: &str, width: u32, height: u32, format: wgpu::TextureFormat| {
                    let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some(label),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    (texture, view)
                };
            if slots.get(index).is_none_or(|slot| {
                (slot.width, slot.height, slot.format) != (size.0, size.1, format)
            }) {
                let (texture, view) = if nv12 {
                    new_texture(
                        "aeroedits-layer-luma",
                        size.0,
                        size.1,
                        wgpu::TextureFormat::R8Unorm,
                    )
                } else {
                    new_texture(
                        "aeroedits-layer",
                        size.0,
                        size.1,
                        wgpu::TextureFormat::Bgra8Unorm,
                    )
                };
                let chroma = nv12.then(|| {
                    new_texture(
                        "aeroedits-layer-chroma",
                        size.0 / 2,
                        size.1 / 2,
                        wgpu::TextureFormat::Rg8Unorm,
                    )
                });
                let slot = LayerSlot {
                    width: size.0,
                    height: size.1,
                    format,
                    texture,
                    view,
                    chroma,
                    key: None,
                };
                if index < slots.len() {
                    slots[index] = slot;
                } else {
                    slots.push(slot);
                }
            }
            let slot = &mut slots[index];
            let unchanged = layer.cache_key.is_some() && slot.key == layer.cache_key;
            slot.key = layer.cache_key;
            if !unchanged {
                let stride = layer.frame.stride;
                let plane = |texture: &wgpu::Texture, bytes: &[u8], width: u32, height: u32| {
                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        bytes,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(stride),
                            rows_per_image: Some(height),
                        },
                        wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );
                };
                let (w, h) = size;
                if let Some((chroma, _)) = &slot.chroma {
                    let luma_len = stride as usize * h as usize;
                    plane(&slot.texture, &layer.frame.data[..luma_len], w, h);
                    plane(chroma, &layer.frame.data[luma_len..needed], w / 2, h / 2);
                } else {
                    plane(&slot.texture, &layer.frame.data[..needed], w, h);
                }
            }
            let layer_view = slot.view.clone();
            let chroma_view = slot
                .chroma
                .as_ref()
                .map_or_else(|| self.no_chroma.clone(), |(_, view)| view.clone());
            let content_params = layer_params(layer, 0);
            let content_uniform =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("aeroedits-layer-params"),
                        contents: bytemuck::bytes_of(&content_params),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
            let bind_content = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("aeroedits-layer-bind"),
                layout: &self.bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&layer_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: content_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&chroma_view),
                    },
                ],
            });
            let v_content = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("aeroedits-quad"),
                    contents: bytemuck::cast_slice(&quad_vertices(
                        scene.width,
                        scene.height,
                        layer,
                        0.0,
                    )),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let shadow = if layer.has_shadow() {
                let pad = shadow_pad(layer);
                let shadow_params = layer_params(layer, 1);
                let shadow_uniform =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("aeroedits-shadow-params"),
                            contents: bytemuck::bytes_of(&shadow_params),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });
                let bind_shadow = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("aeroedits-shadow-bind"),
                    layout: &self.bind_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&layer_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: shadow_uniform.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&chroma_view),
                        },
                    ],
                });
                let v_shadow = self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("aeroedits-shadow-quad"),
                        contents: bytemuck::cast_slice(&quad_vertices(
                            scene.width,
                            scene.height,
                            layer,
                            pad,
                        )),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                Some((bind_shadow, v_shadow, shadow_uniform))
            } else {
                None
            };
            draws.push(LayerDraw {
                _content_uniform: content_uniform,
                bind_content,
                v_content,
                shadow,
            });
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("aeroedits-composite-enc"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("aeroedits-layers"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: scene.background[0] as f64,
                            g: scene.background[1] as f64,
                            b: scene.background[2] as f64,
                            a: scene.background[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            for draw in &draws {
                if let Some((bind, vbuf, _)) = &draw.shadow {
                    pass.set_bind_group(0, bind, &[]);
                    pass.set_vertex_buffer(0, vbuf.slice(..));
                    pass.draw(0..6, 0..1);
                }
                pass.set_bind_group(0, &draw.bind_content, &[]);
                pass.set_vertex_buffer(0, draw.v_content.slice(..));
                pass.draw(0..6, 0..1);
            }
        }

        let copy = |encoder: &mut wgpu::CommandEncoder,
                    texture: &wgpu::Texture,
                    buffer: &wgpu::Buffer,
                    offset: u64,
                    row: u32,
                    (width, height): (u32, u32)| {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset,
                        bytes_per_row: Some(row),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        };
        let (w, h) = (scene.width, scene.height);
        // What to read back: the buffer, its padded row, the bytes of each row and the rows.
        let (staging, padded, row_bytes, rows) = match yuv {
            Some(yuv) => {
                for (pipeline, plane) in [
                    (&self.luma_pipeline, &yuv.luma_view),
                    (&self.chroma_pipeline, &yuv.chroma_view),
                ] {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("aeroedits-to-yuv"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: plane,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &yuv.bind, &[]);
                    pass.draw(0..3, 0..1);
                }
                let row = aligned_row(w);
                copy(&mut encoder, &yuv.luma, &yuv.staging, 0, row, (w, h));
                let chroma_at = u64::from(row) * u64::from(h);
                copy(
                    &mut encoder,
                    &yuv.chroma,
                    &yuv.staging,
                    chroma_at,
                    row,
                    (w / 2, h / 2),
                );
                (&yuv.staging, row, w, h + h / 2)
            }
            None => {
                copy(&mut encoder, target, &cached.staging, 0, padded, (w, h));
                (&cached.staging, padded, w * 4, h)
            }
        };
        self.queue.submit(Some(encoder.finish()));
        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| ());
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("Compositor readback poll failed: {e}"))?;
        let data = slice.get_mapped_range();
        let mut packed = Vec::with_capacity(row_bytes as usize * rows as usize);
        for row in 0..rows {
            let start = (row * padded) as usize;
            packed.extend_from_slice(&data[start..start + row_bytes as usize]);
        }
        drop(data);
        staging.unmap();
        let _keep = draws;
        Ok(VideoFrame {
            pts_us: scene.layers.first().map(|l| l.frame.pts_us).unwrap_or(0),
            width: w,
            height: h,
            stride: row_bytes,
            format: if nv12 {
                PixelFormat::Nv12
            } else {
                PixelFormat::Bgra8888
            },
            color: if nv12 {
                ColorInfo::rec709_limited()
            } else {
                ColorInfo::rec709_full()
            },
            data: packed.into(),
        })
    }
}

fn fit_inside(
    src_w: u32,
    src_h: u32,
    dest_x: u32,
    dest_y: u32,
    dest_w: u32,
    dest_h: u32,
) -> (u32, u32, u32, u32) {
    if src_w == 0 || src_h == 0 || dest_w == 0 || dest_h == 0 {
        return (dest_x, dest_y, dest_w.max(1), dest_h.max(1));
    }
    let src_a = src_w as f64 / src_h as f64;
    let dest_a = dest_w as f64 / dest_h as f64;
    let (w, h) = if src_a > dest_a {
        let w = dest_w;
        let h = ((dest_w as f64 / src_a).round() as u32).clamp(1, dest_h);
        (w, h)
    } else {
        let h = dest_h;
        let w = ((dest_h as f64 * src_a).round() as u32).clamp(1, dest_w);
        (w, h)
    };
    let x = dest_x + dest_w.saturating_sub(w) / 2;
    let y = dest_y + dest_h.saturating_sub(h) / 2;
    (x, y, w, h)
}

fn webcam_bubble_size(
    canvas_w: u32,
    canvas_h: u32,
    src_w: u32,
    src_h: u32,
    layout: &EditLayout,
) -> (u32, u32) {
    let frac = layout.webcam_size_fraction();
    let target = ((canvas_w.min(canvas_h) as f32) * frac).round() as u32;
    let target = target.max(8);
    if src_w == 0 || src_h == 0 {
        return (target, target);
    }
    let (w, h) = if src_w >= src_h {
        let h = ((target as f64 * src_h as f64 / src_w as f64).round() as u32).max(8);
        (target.min(canvas_w).max(1), h.min(canvas_h).max(1))
    } else {
        let w = ((target as f64 * src_w as f64 / src_h as f64).round() as u32).max(8);
        (w.min(canvas_w).max(1), target.min(canvas_h).max(1))
    };
    if matches!(layout.webcam_shape.as_str(), "circle" | "squircle") {
        let side = w.min(h).max(8);
        return (side.min(canvas_w).max(1), side.min(canvas_h).max(1));
    }
    (w, h)
}

fn webcam_origin(
    canvas_w: u32,
    canvas_h: u32,
    bubble_w: u32,
    bubble_h: u32,
    layout: &EditLayout,
    unit: f32,
) -> (u32, u32) {
    let max_x = canvas_w.saturating_sub(bubble_w);
    let max_y = canvas_h.saturating_sub(bubble_h);
    let margin = ((8.0 * unit).round() as u32).min(max_x / 2).min(max_y / 2);
    match layout.webcam_position.as_str() {
        "top-left" => (margin.min(max_x), margin.min(max_y)),
        "top-right" => (max_x.saturating_sub(margin), margin.min(max_y)),
        "bottom-left" => (margin.min(max_x), max_y.saturating_sub(margin)),
        "custom" => {
            let x =
                (layout.webcam_custom_x.clamp(0.0, 100.0) / 100.0 * max_x as f32).round() as u32;
            let y =
                (layout.webcam_custom_y.clamp(0.0, 100.0) / 100.0 * max_y as f32).round() as u32;
            (x.min(max_x), y.min(max_y))
        }
        _ => (max_x.saturating_sub(margin), max_y.saturating_sub(margin)),
    }
}

fn gradient_frame(
    width: u32,
    height: u32,
    start: [f32; 4],
    end: [f32; 4],
) -> Result<VideoFrame, String> {
    let mut frame = VideoFrame::solid(
        width,
        height,
        (start[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (start[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (start[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        0,
    )?;
    let denom_x = (width.saturating_sub(1)).max(1) as f32;
    let denom_y = (height.saturating_sub(1)).max(1) as f32;
    for y in 0..height {
        for x in 0..width {
            let t = 0.5 * (x as f32 / denom_x + y as f32 / denom_y);
            let lerp = |a: f32, b: f32| a + (b - a) * t;
            let i = (y * frame.stride + x * 4) as usize;
            frame.data[i] = (lerp(start[2], end[2]).clamp(0.0, 1.0) * 255.0).round() as u8;
            frame.data[i + 1] = (lerp(start[1], end[1]).clamp(0.0, 1.0) * 255.0).round() as u8;
            frame.data[i + 2] = (lerp(start[0], end[0]).clamp(0.0, 1.0) * 255.0).round() as u8;
            frame.data[i + 3] = 255;
        }
    }
    Ok(frame)
}

fn webcam_clip(layout: &EditLayout, bubble_w: u32, bubble_h: u32) -> (ClipMode, f32) {
    match layout.webcam_shape.as_str() {
        "circle" => (ClipMode::Circle, 0.0),
        "squircle" => (ClipMode::Squircle, 0.0),
        _ => {
            let radius = layout.webcam_roundness_pct.clamp(0.0, 50.0) / 100.0
                * bubble_w.min(bubble_h) as f32;
            if radius > 0.0 {
                (ClipMode::RoundedRect, radius)
            } else {
                (ClipMode::None, 0.0)
            }
        }
    }
}

/// Multiplier for layout pixel values on a canvas of this size, so padding, radii, shadows and
/// borders keep their proportions between the downscaled preview and any export resolution.
pub fn layout_px_unit(canvas_w: u32, canvas_h: u32) -> f32 {
    (canvas_w.min(canvas_h) as f32 / crate::project::layout::REFERENCE_SHORT_SIDE_PX).max(0.01)
}

/// The screen UV for a zoom camera inside a crop. The zoom keeps its target point in full-source
/// coordinates and its size relative to the crop, and never shows anything the crop removed.
pub fn zoom_within_crop(
    crop: (f32, f32, f32, f32),
    zoom: (f32, f32, f32, f32),
) -> (f32, f32, f32, f32) {
    let (cx, cy, cw, ch) = crop;
    let (zx, zy, zw, zh) = zoom;
    let w = (zw * cw).clamp(0.0001, cw);
    let h = (zh * ch).clamp(0.0001, ch);
    let center_x = zx + zw * 0.5;
    let center_y = zy + zh * 0.5;
    // max/min rather than clamp: rounding can put the upper bound a hair below the lower one.
    let x = (center_x - w * 0.5).min(cx + cw - w).max(cx);
    let y = (center_y - h * 0.5).min(cy + ch - h).max(cy);
    (x, y, w, h)
}

struct Blob {
    x: f32,
    y: f32,
    radius: f32,
    color: [u8; 3],
}

const fn blob(x: f32, y: f32, radius: f32, color: u32) -> Blob {
    Blob {
        x,
        y,
        radius,
        color: [(color >> 16) as u8, (color >> 8) as u8, color as u8],
    }
}

/// Built-in backgrounds: a base color with soft color blobs. Positions are fractions of the
/// canvas; radii are fractions of its long side. The inspector swatches mirror these values.
fn preset_recipe(name: &str) -> Option<(u32, [Blob; 3])> {
    Some(match name {
        "aurora" => (
            0x0b1026,
            [
                blob(0.15, 0.2, 0.7, 0x3b82f6),
                blob(0.85, 0.3, 0.6, 0xa855f7),
                blob(0.5, 1.0, 0.7, 0x14b8a6),
            ],
        ),
        "sunset" => (
            0x1e1033,
            [
                blob(0.2, 0.9, 0.8, 0xf97316),
                blob(0.8, 0.2, 0.7, 0xdb2777),
                blob(0.55, 0.55, 0.4, 0xfacc15),
            ],
        ),
        "ocean" => (
            0x031b34,
            [
                blob(0.1, 0.1, 0.8, 0x0ea5e9),
                blob(0.9, 0.9, 0.8, 0x1d4ed8),
                blob(0.6, 0.4, 0.45, 0x22d3ee),
            ],
        ),
        "forest" => (
            0x052e16,
            [
                blob(0.2, 0.8, 0.8, 0x15803d),
                blob(0.85, 0.15, 0.6, 0x65a30d),
                blob(0.6, 0.6, 0.45, 0x0f766e),
            ],
        ),
        "candy" => (
            0x3b0764,
            [
                blob(0.1, 0.3, 0.7, 0xec4899),
                blob(0.9, 0.7, 0.7, 0x8b5cf6),
                blob(0.5, 0.0, 0.5, 0xf472b6),
            ],
        ),
        "graphite" => (
            0x111113,
            [
                blob(0.2, 0.15, 0.8, 0x3f3f46),
                blob(0.85, 0.85, 0.7, 0x27272a),
                blob(0.6, 0.4, 0.4, 0x52525b),
            ],
        ),
        _ => return None,
    })
}

pub fn preset_frame(width: u32, height: u32, name: &str) -> Result<VideoFrame, String> {
    let (base, blobs) =
        preset_recipe(name).ok_or_else(|| format!("Unknown background preset: {name}"))?;
    let base = [(base >> 16) as u8, (base >> 8) as u8, base as u8];
    let mut frame = VideoFrame::solid(width, height, base[2], base[1], base[0], 0)?;
    let long = width.max(height).max(1) as f32;
    for y in 0..height {
        for x in 0..width {
            let mut rgb = [base[0] as f32, base[1] as f32, base[2] as f32];
            for b in &blobs {
                let dx = (x as f32 + 0.5 - b.x * width as f32) / long;
                let dy = (y as f32 + 0.5 - b.y * height as f32) / long;
                let d = (dx * dx + dy * dy) / (b.radius * b.radius);
                if d >= 1.0 {
                    continue;
                }
                let t = (1.0 - d) * (1.0 - d) * 0.85;
                for c in 0..3 {
                    rgb[c] += (b.color[c] as f32 - rgb[c]) * t;
                }
            }
            let i = (y * frame.stride + x * 4) as usize;
            frame.data[i] = rgb[2].round() as u8;
            frame.data[i + 1] = rgb[1].round() as u8;
            frame.data[i + 2] = rgb[0].round() as u8;
            frame.data[i + 3] = 255;
        }
    }
    Ok(frame)
}

fn cover_uv(src_w: u32, src_h: u32, dest_w: u32, dest_h: u32) -> (f32, f32, f32, f32) {
    if src_w == 0 || src_h == 0 || dest_w == 0 || dest_h == 0 {
        return (0.0, 0.0, 1.0, 1.0);
    }
    let src_a = src_w as f32 / src_h as f32;
    let dest_a = dest_w as f32 / dest_h as f32;
    if src_a > dest_a {
        let vis_w = dest_a / src_a;
        ((1.0 - vis_w) * 0.5, 0.0, vis_w, 1.0)
    } else {
        let vis_h = src_a / dest_a;
        (0.0, (1.0 - vis_h) * 0.5, 1.0, vis_h)
    }
}

fn sdf_rounded_rect(px: f32, py: f32, half_w: f32, half_h: f32, radius: f32) -> f32 {
    let r = radius.min(half_w).min(half_h).max(0.0);
    let qx = px.abs() - (half_w - r);
    let qy = py.abs() - (half_h - r);
    let outside_x = qx.max(0.0);
    let outside_y = qy.max(0.0);
    (outside_x * outside_x + outside_y * outside_y).sqrt() + qx.max(qy).min(0.0) - r
}

fn sdf_circle(px: f32, py: f32, half_w: f32, half_h: f32) -> f32 {
    (px * px + py * py).sqrt() - half_w.min(half_h)
}

fn sdf_squircle(px: f32, py: f32, half_w: f32, half_h: f32) -> f32 {
    let nx = if half_w > 0.0 { px.abs() / half_w } else { 0.0 };
    let ny = if half_h > 0.0 { py.abs() / half_h } else { 0.0 };
    let k = nx * nx * nx * nx + ny * ny * ny * ny;
    let r = half_w.min(half_h);
    k.max(0.0).powf(0.25) * r - r
}

fn layer_sdf(layer: &Layer, local_x: f32, local_y: f32, shadow: bool) -> f32 {
    let mut px = (local_x - 0.5) * layer.width as f32;
    let mut py = (local_y - 0.5) * layer.height as f32;
    if shadow {
        px -= layer.shadow_offset[0];
        py -= layer.shadow_offset[1];
    }
    let half_w = layer.width as f32 * 0.5;
    let half_h = layer.height as f32 * 0.5;
    match layer.clip {
        ClipMode::Circle => sdf_circle(px, py, half_w, half_h),
        ClipMode::Squircle => sdf_squircle(px, py, half_w, half_h),
        ClipMode::RoundedRect | ClipMode::None => {
            sdf_rounded_rect(px, py, half_w, half_h, layer.radius_px)
        }
    }
}

fn shadow_coverage(sdf: f32, blur: f32) -> f32 {
    if blur <= 0.0 {
        if sdf <= 0.0 {
            1.0
        } else {
            0.0
        }
    } else {
        1.0 - (sdf / blur).clamp(0.0, 1.0)
    }
}

fn shadow_pad(layer: &Layer) -> f32 {
    layer.shadow_blur_px + layer.shadow_offset[0].abs() + layer.shadow_offset[1].abs()
}

fn layer_params(layer: &Layer, pass_kind: u32) -> LayerParams {
    LayerParams {
        size_px: [layer.width as f32, layer.height as f32],
        radius_px: layer.radius_px,
        clip_mode: layer.clip as u32,
        shadow_blur_px: layer.shadow_blur_px,
        shadow_opacity: layer.shadow_opacity,
        shadow_offset: layer.shadow_offset,
        pass_kind,
        format: match layer.frame.format {
            PixelFormat::Bgra8888 => 0,
            PixelFormat::Nv12 => 1,
        },
        _pad: [0, 0],
    }
}

/// Decode a project-relative wallpaper asset. Never follows URLs or symlinks.
/// The scene background as one canvas-ready frame: the wallpaper, or the gradient. Callers
/// that render many frames of one layout compute it once and pass it as the wallpaper.
pub fn background_frame(
    root: &Path,
    layout: &EditLayout,
    canvas_w: u32,
    canvas_h: u32,
) -> Result<Option<VideoFrame>, String> {
    if let Some(paper) = load_wallpaper_frame(root, layout, canvas_w, canvas_h)? {
        return Ok(Some(paper));
    }
    if layout.background_type == "preset" {
        return preset_frame(canvas_w, canvas_h, &layout.background_preset).map(Some);
    }
    match layout.background_rgba()? {
        (start, Some(end)) => gradient_frame(canvas_w, canvas_h, start, end).map(Some),
        _ => Ok(None),
    }
}

pub fn load_wallpaper_frame(
    root: &Path,
    layout: &EditLayout,
    canvas_w: u32,
    canvas_h: u32,
) -> Result<Option<VideoFrame>, String> {
    if layout.background_type != "wallpaper" {
        return Ok(None);
    }
    let relative = layout
        .wallpaper_asset
        .as_deref()
        .ok_or_else(|| "Wallpaper background requires a project asset".to_string())?;
    if relative.contains("://") {
        return Err("Wallpaper cannot be an external URL".into());
    }
    validate_wallpaper_relative(relative)?;
    let path = safe_path(root, relative)?;
    let file = open_regular(&path)?;
    let mut bytes = Vec::new();
    file.take(MAX_WALLPAPER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_WALLPAPER_BYTES {
        return Err("Wallpaper exceeds size limit".into());
    }
    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("Wallpaper decode failed: {e}"))?
        .to_rgba8();
    if img.width() > MAX_FRAME_DIM
        || img.height() > MAX_FRAME_DIM
        || img.width() == 0
        || img.height() == 0
    {
        return Err("Wallpaper exceeds compositor working-set limit".into());
    }
    validate_dim(img.width(), img.height())?;
    // Shrink once to just cover the canvas, so each frame uploads and samples a small image.
    let cover = (canvas_w as f64 / img.width() as f64).max(canvas_h as f64 / img.height() as f64);
    let img = if cover < 1.0 && canvas_w > 0 && canvas_h > 0 {
        let w = ((img.width() as f64 * cover).ceil() as u32).clamp(canvas_w, img.width());
        let h = ((img.height() as f64 * cover).ceil() as u32).clamp(canvas_h, img.height());
        image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let mut frame = VideoFrame::solid(img.width(), img.height(), 0, 0, 0, 0)?;
    for y in 0..img.height() {
        for x in 0..img.width() {
            let p = img[(x, y)].0;
            let i = (y * frame.stride + x * 4) as usize;
            frame.data[i] = p[2];
            frame.data[i + 1] = p[1];
            frame.data[i + 2] = p[0];
            frame.data[i + 3] = 255;
        }
    }
    Ok(Some(frame))
}

fn blit_shadow(dest: &mut VideoFrame, layer: &Layer) -> Result<(), String> {
    if layer.width == 0 || layer.height == 0 {
        return Err("Layer size is empty".into());
    }
    let pad = shadow_pad(layer).ceil() as i32;
    let x0 = layer.x as i32 - pad;
    let y0 = layer.y as i32 - pad;
    let x1 = layer.x as i32 + layer.width as i32 + pad;
    let y1 = layer.y as i32 + layer.height as i32 + pad;
    for y in y0.max(0)..y1.min(dest.height as i32) {
        for x in x0.max(0)..x1.min(dest.width as i32) {
            let local_x = (x as f32 + 0.5 - layer.x as f32) / layer.width as f32;
            let local_y = (y as f32 + 0.5 - layer.y as f32) / layer.height as f32;
            let sdf = layer_sdf(layer, local_x, local_y, true);
            let alpha = layer.shadow_opacity * shadow_coverage(sdf, layer.shadow_blur_px);
            if alpha <= 0.0 {
                continue;
            }
            let di = (y as u32 * dest.stride + x as u32 * 4) as usize;
            let keep = 1.0 - alpha.clamp(0.0, 1.0);
            dest.data[di] = (dest.data[di] as f32 * keep).round() as u8;
            dest.data[di + 1] = (dest.data[di + 1] as f32 * keep).round() as u8;
            dest.data[di + 2] = (dest.data[di + 2] as f32 * keep).round() as u8;
        }
    }
    Ok(())
}

/// Samples `frame` at normalized `(u, v)` like a clamp-to-edge bilinear GPU sampler.
fn sample_bilinear(frame: &VideoFrame, u: f32, v: f32) -> [u8; 4] {
    let x = u * frame.width as f32 - 0.5;
    let y = v * frame.height as f32 - 0.5;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let clamp_x = |value: i64| value.clamp(0, frame.width as i64 - 1) as u32;
    let clamp_y = |value: i64| value.clamp(0, frame.height as i64 - 1) as u32;
    let (xa, xb) = (clamp_x(x0 as i64), clamp_x(x0 as i64 + 1));
    let (ya, yb) = (clamp_y(y0 as i64), clamp_y(y0 as i64 + 1));
    let texel =
        |sx: u32, sy: u32, c: usize| frame.data[(sy * frame.stride + sx * 4) as usize + c] as f32;
    let mut out = [0u8; 4];
    for (c, slot) in out.iter_mut().enumerate() {
        let top = texel(xa, ya, c) * (1.0 - fx) + texel(xb, ya, c) * fx;
        let bottom = texel(xa, yb, c) * (1.0 - fx) + texel(xb, yb, c) * fx;
        *slot = (top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn blit_bilinear(dest: &mut VideoFrame, layer: &Layer) -> Result<(), String> {
    if layer.width == 0 || layer.height == 0 {
        return Err("Layer size is empty".into());
    }
    for dy in 0..layer.height {
        let y = layer.y + dy;
        if y >= dest.height {
            continue;
        }
        let local_y = (dy as f32 + 0.5) / layer.height as f32;
        let v = layer.uv_y + layer.uv_h * local_y;
        for dx in 0..layer.width {
            let x = layer.x + dx;
            if x >= dest.width {
                continue;
            }
            let local_x = (dx as f32 + 0.5) / layer.width as f32;
            if layer_sdf(layer, local_x, local_y, false) > 0.0 {
                continue;
            }
            let u = layer.uv_x + layer.uv_w * local_x;
            let src = sample_bilinear(&layer.frame, u, v);
            let di = (y * dest.stride + x * 4) as usize;
            if matches!(layer.role, LayerRole::Caption | LayerRole::Overlay) {
                // Same blend as the GPU pipeline: source over, straight alpha.
                let alpha = src[3] as u32;
                for c in 0..3 {
                    let dst = dest.data[di + c] as u32;
                    dest.data[di + c] =
                        ((src[c] as u32 * alpha + dst * (255 - alpha) + 127) / 255) as u8;
                }
                continue;
            }
            dest.data[di..di + 4].copy_from_slice(&src);
        }
    }
    Ok(())
}

fn padded_bytes_per_row(width: u32) -> u32 {
    aligned_row(width * 4)
}

/// `bytes` rounded up to the row alignment of a texture-to-buffer copy.
fn aligned_row(bytes: u32) -> u32 {
    bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
}

fn quad_vertices(canvas_w: u32, canvas_h: u32, layer: &Layer, pad: f32) -> [Vertex; 6] {
    let x0 = (layer.x as f32 - pad) / canvas_w as f32 * 2.0 - 1.0;
    let x1 = (layer.x as f32 + layer.width as f32 + pad) / canvas_w as f32 * 2.0 - 1.0;
    let y0 = 1.0 - (layer.y as f32 - pad) / canvas_h as f32 * 2.0;
    let y1 = 1.0 - (layer.y as f32 + layer.height as f32 + pad) / canvas_h as f32 * 2.0;
    let u0 = layer.uv_x;
    let v0 = layer.uv_y;
    let u1 = layer.uv_x + layer.uv_w;
    let v1 = layer.uv_y + layer.uv_h;
    let lx0 = -pad / layer.width as f32;
    let lx1 = 1.0 + pad / layer.width as f32;
    let ly0 = -pad / layer.height as f32;
    let ly1 = 1.0 + pad / layer.height as f32;
    [
        Vertex {
            pos: [x0, y0],
            uv: [u0, v0],
            local: [lx0, ly0],
        },
        Vertex {
            pos: [x1, y0],
            uv: [u1, v0],
            local: [lx1, ly0],
        },
        Vertex {
            pos: [x0, y1],
            uv: [u0, v1],
            local: [lx0, ly1],
        },
        Vertex {
            pos: [x1, y0],
            uv: [u1, v0],
            local: [lx1, ly0],
        },
        Vertex {
            pos: [x1, y1],
            uv: [u1, v1],
            local: [lx1, ly1],
        },
        Vertex {
            pos: [x0, y1],
            uv: [u0, v1],
            local: [lx0, ly1],
        },
    ]
}

pub fn run_parity(
    dir: &std::path::Path,
    gate: &crate::media::EncoderGate,
) -> Result<crate::media::MediaParityReport, String> {
    use crate::fixtures::generate_pcm16_wav;
    use crate::media::{
        compare_frames, decode_h264_frame, decode_pcm, encode_h264_frames, region_mean_delta,
        write_solid_h264, COMPOSITOR_BACKEND, COMPOSITOR_MAX_TOLERANCE, COMPOSITOR_MEAN_TOLERANCE,
        CONCURRENT_ENCODER_LIMIT, COPIES_DECODE, COPIES_ENCODE, PARITY_MEAN_TOLERANCE,
        PARITY_REGION_MEAN_TOLERANCE,
    };
    use crate::project::pcm::channel_peak_rms;
    use std::fs;

    let compositor = Compositor::new()?;
    let screen_path = dir.join("f2-screen.mp4");
    write_solid_h264(&screen_path, 64, 64, 0.92, 0.12, 0.10)?;
    let screen = decode_h264_frame(&screen_path, 0)?;
    let webcam = VideoFrame::solid(16, 16, 32, 200, 48, 0)?;
    let scene = Scene::styled_preview(screen, Some(webcam))?;
    let preview = compositor.composite(&scene)?;
    let cpu = Compositor::composite_cpu(&scene)?;
    let (compositor_max_delta, compositor_mean_delta) = compare_frames(&preview, &cpu)?;
    let _slot = gate.try_acquire()?;
    if gate.try_acquire().is_ok() {
        return Err("Encoder gate allowed a second concurrent encode".into());
    }
    let export_path = dir.join("f2-export.mp4");
    encode_h264_frames(&export_path, &[preview.clone(), preview.clone()], 30)?;
    drop(_slot);
    let exported = decode_h264_frame(&export_path, 0)?;
    let (max_abs_delta, mean_abs_delta) = compare_frames(&preview, &exported)?;
    let region_mean_delta = region_mean_delta(&preview, &exported);
    let wav_path = dir.join("f2-mic.wav");
    let pcm_bytes = generate_pcm16_wav(48_000, 1, &[16384i16; 480]);
    fs::write(&wav_path, pcm_bytes).map_err(|e| e.to_string())?;
    let audio = decode_pcm(&wav_path, 64)?;
    let (pcm_peak, pcm_rms) = channel_peak_rms(&audio.samples);
    let matched = compositor_mean_delta <= COMPOSITOR_MEAN_TOLERANCE
        && compositor_max_delta <= COMPOSITOR_MAX_TOLERANCE
        && mean_abs_delta <= PARITY_MEAN_TOLERANCE
        && region_mean_delta <= PARITY_REGION_MEAN_TOLERANCE;
    let mut diagnostics = vec![
        format!("adapter={}", compositor.adapter_name()),
        format!("ffmpeg_pinned=false"),
        format!("software_fallback=none"),
    ];
    if !matched {
        diagnostics.push(format!(
            "parity missed tolerance compositor_mean={compositor_mean_delta:.2} compositor_max={compositor_max_delta} encode_mean={mean_abs_delta:.2} region={region_mean_delta:.2}"
        ));
    }
    Ok(crate::media::MediaParityReport {
        matched,
        preview_width: preview.width,
        preview_height: preview.height,
        export_width: exported.width,
        export_height: exported.height,
        preview_pts_us: preview.pts_us,
        export_pts_us: exported.pts_us,
        max_abs_delta,
        mean_abs_delta,
        region_mean_delta,
        compositor_mean_delta,
        compositor_max_delta,
        copies_decode: COPIES_DECODE,
        copies_composite: compositor.copies(),
        copies_encode: COPIES_ENCODE,
        concurrent_encoder_limit: CONCURRENT_ENCODER_LIMIT,
        decoder_backend: crate::media::decoder_backend().into(),
        compositor_backend: COMPOSITOR_BACKEND.into(),
        encoder_backend: crate::media::encoder_backend(),
        ffmpeg_pinned: false,
        color_space: "rec709_full".into(),
        pcm_peak,
        pcm_rms,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::VideoFrame;

    /// Nearest sampling at 60% scale skipped source column 3 entirely; bilinear keeps it.
    #[test]
    fn bilinear_sampling_keeps_thin_lines_when_scaling_down() {
        let mut frame = VideoFrame::solid(100, 4, 0, 0, 0, 255).unwrap();
        for y in 0..4u32 {
            let i = (y * frame.stride + 3 * 4) as usize;
            frame.data[i..i + 3].copy_from_slice(&[255, 255, 255]);
        }
        // Texel centres return the texel exactly.
        assert_eq!(sample_bilinear(&frame, 3.5 / 100.0, 0.5)[0], 255);
        assert_eq!(sample_bilinear(&frame, 2.5 / 100.0, 0.5)[0], 0);
        // Halfway between two texels is their average.
        assert_eq!(sample_bilinear(&frame, 3.0 / 100.0, 0.5)[0], 128);
        let brightest = (0..60)
            .map(|dx| sample_bilinear(&frame, (dx as f32 + 0.5) / 60.0, 0.5)[0])
            .max()
            .unwrap();
        assert!(brightest >= 64, "thin line vanished: {brightest}");
    }

    #[test]
    fn cpu_composite_crops_screen_uv_and_leaves_webcam() {
        let mut screen = VideoFrame::solid(8, 8, 0, 0, 255, 0).unwrap();
        for y in 0..8u32 {
            for x in 4..8u32 {
                let i = ((y * 8 + x) * 4) as usize;
                screen.data[i] = 255;
                screen.data[i + 1] = 0;
                screen.data[i + 2] = 0;
            }
        }
        let webcam = VideoFrame::solid(8, 8, 0, 255, 0, 0).unwrap();
        let mut scene = Scene {
            width: 8,
            height: 8,
            background: [0.0, 0.0, 0.0, 1.0],
            layers: vec![
                Layer::placed(screen, 0, 0, 8, 8),
                Layer::placed(webcam, 0, 0, 2, 2),
            ],
        };
        scene.layers[0].uv_x = 0.5;
        scene.layers[0].uv_w = 0.5;
        let out = Compositor::composite_cpu(&scene).unwrap();
        let center = ((4 * 8 + 4) * 4) as usize;
        assert!(
            out.data[center] > 200 && out.data[center + 2] < 40,
            "screen UV crop should sample the blue half"
        );
        assert!(
            out.data[1] > 200 && out.data[2] < 40,
            "webcam layer must keep identity UV"
        );
    }

    fn split_frame(width: u32, height: u32, left: [u8; 3], right: [u8; 3]) -> VideoFrame {
        let mut frame = VideoFrame::solid(width, height, left[0], left[1], left[2], 0).unwrap();
        for y in 0..height {
            for x in width / 2..width {
                let i = (y * frame.stride + x * 4) as usize;
                frame.data[i] = right[0];
                frame.data[i + 1] = right[1];
                frame.data[i + 2] = right[2];
            }
        }
        frame
    }

    #[test]
    fn layout_letterboxes_without_stretching() {
        let screen = VideoFrame::solid(16, 8, 0, 0, 255, 0).unwrap();
        let mut landscape = EditLayout::default();
        landscape.background_type = "solid".into();
        landscape.color_start = "#000000".into();
        landscape.webcam_enabled = false;
        landscape.padding_px = 0;
        let wide = Scene::from_layout(16, 8, &landscape, Some(screen.clone()), None).unwrap();
        let screen_layer = wide
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Screen)
            .unwrap();
        assert_eq!((screen_layer.width, screen_layer.height), (16, 8));

        let mut portrait = landscape.clone();
        portrait.aspect_ratio = "9:16".into();
        let tall = Scene::from_layout(8, 16, &portrait, Some(screen), None).unwrap();
        let placed = tall
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Screen)
            .unwrap();
        let placed_aspect = placed.width as f32 / placed.height as f32;
        assert!(
            (placed_aspect - 2.0).abs() < 0.05,
            "9:16 canvas must letterbox a 16:9 source, got {placed_aspect}"
        );
        assert_eq!(placed.width, 8);
        assert_eq!(placed.height, 4);
    }

    #[test]
    fn webcam_mirror_flips_only_webcam_pixels() {
        let screen = split_frame(16, 16, [0, 0, 255], [255, 0, 0]);
        let webcam = split_frame(16, 8, [0, 255, 0], [255, 255, 255]);
        let mut layout = EditLayout::default();
        layout.background_type = "solid".into();
        layout.color_start = "#000000".into();
        layout.padding_px = 0;
        layout.webcam_enabled = true;
        layout.webcam_mirror = true;
        layout.webcam_position = "top-left".into();
        layout.webcam_size = "xl".into();
        layout.webcam_border_width = 0;
        let scene = Scene::from_layout(16, 16, &layout, Some(screen), Some(webcam)).unwrap();
        let cam = scene
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Webcam)
            .unwrap();
        assert_eq!(cam.uv_x, 1.0);
        assert_eq!(cam.uv_w, -1.0);
        let screen_layer = scene
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Screen)
            .unwrap();
        assert_eq!(screen_layer.uv_w, 1.0);
        let out = Compositor::composite_cpu(&scene).unwrap();
        let screen_left = out.data[0];
        assert!(
            out.data[2] > 200 && screen_left < 40,
            "screen left must stay red (unmirrored)"
        );
        let cx = cam.x;
        let cy = cam.y;
        let left = (cy * out.stride + cx * 4) as usize;
        let right = (cy * out.stride + (cx + cam.width - 1) * 4) as usize;
        assert!(
            out.data[left] > 200 && out.data[left + 1] > 200 && out.data[left + 2] > 200,
            "mirrored webcam should put the white half on the left"
        );
        assert!(
            out.data[right + 1] > 200 && out.data[right + 2] < 40,
            "mirrored webcam should put the green half on the right"
        );
    }

    #[test]
    fn styled_solid_background_differs_from_identity() {
        let screen = VideoFrame::solid(8, 8, 0, 0, 255, 0).unwrap();
        let mut identity = EditLayout::default();
        identity.background_type = "solid".into();
        identity.color_start = "#000000".into();
        identity.padding_px = 2;
        identity.webcam_enabled = false;
        let mut styled = identity.clone();
        styled.color_start = "#ff0000".into();
        let a = Compositor::composite_cpu(
            &Scene::from_layout(16, 16, &identity, Some(screen.clone()), None).unwrap(),
        )
        .unwrap();
        let b = Compositor::composite_cpu(
            &Scene::from_layout(16, 16, &styled, Some(screen), None).unwrap(),
        )
        .unwrap();
        assert_ne!(a.data[0..4], b.data[0..4], "padding should show background");
        assert!(b.data[2] > 200 && a.data[2] < 40);
    }

    fn pixel(frame: &VideoFrame, x: u32, y: u32) -> [u8; 4] {
        let i = (y * frame.stride + x * 4) as usize;
        [
            frame.data[i],
            frame.data[i + 1],
            frame.data[i + 2],
            frame.data[i + 3],
        ]
    }

    fn full_frame_delta(a: &VideoFrame, b: &VideoFrame) -> (u8, f32) {
        let mut max = 0u8;
        let mut sum = 0u64;
        let mut count = 0u64;
        for y in 0..a.height {
            for x in 0..a.width {
                let i = (y * a.stride + x * 4) as usize;
                for c in 0..3 {
                    let d = a.data[i + c].abs_diff(b.data[i + c]);
                    max = max.max(d);
                    sum += u64::from(d);
                    count += 1;
                }
            }
        }
        (max, sum as f32 / count as f32)
    }

    fn solid_layout() -> EditLayout {
        let mut layout = EditLayout::default();
        layout.background_type = "solid".into();
        layout.color_start = "#00ff00".into();
        layout.padding_px = 4;
        layout.webcam_enabled = false;
        layout.corner_radius_px = 0;
        layout.shadow_blur_px = 0;
        layout
    }

    #[test]
    fn screen_crop_keeps_scale_and_recentres() {
        // Left half red, right half blue; cropping 45% from the left leaves mostly blue.
        let screen = split_frame(32, 16, [0, 0, 255], [255, 0, 0]);
        let mut layout = solid_layout();
        layout.padding_px = 0;
        let full = Scene::from_layout(32, 32, &layout, Some(screen.clone()), None).unwrap();
        let full = full
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Screen)
            .unwrap();
        assert_eq!((full.x, full.y, full.width, full.height), (0, 8, 32, 16));

        layout.screen_crop_left = 45.0;
        let scene = Scene::from_layout(32, 32, &layout, Some(screen.clone()), None).unwrap();
        let layer = scene
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Screen)
            .unwrap();
        assert!((layer.uv_x - 0.45).abs() < 1e-6 && (layer.uv_w - 0.55).abs() < 1e-6);
        // Same scale as uncropped (18 of 32 px kept), centred horizontally.
        assert_eq!(
            (layer.x, layer.y, layer.width, layer.height),
            (7, 8, 18, 16)
        );
        let out = Compositor::composite_cpu(&scene).unwrap();
        let [b, _, r, _] = pixel(&out, 23, 16);
        assert!(b > 200 && r < 40, "right side shows the blue half");
        let [_, _, r, _] = pixel(&out, 7, 16);
        assert!(r > 200, "new left edge shows the red sliver the crop kept");
        assert_eq!(
            pixel(&out, 3, 16)[1],
            255,
            "background shows either side of the centred crop"
        );

        // Cropping right and bottom shrinks the region and centres it both ways.
        layout.screen_crop_left = 0.0;
        layout.screen_crop_right = 25.0;
        layout.screen_crop_bottom = 25.0;
        let scene = Scene::from_layout(32, 32, &layout, Some(screen), None).unwrap();
        let layer = scene
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Screen)
            .unwrap();
        assert_eq!(
            (layer.x, layer.y, layer.width, layer.height),
            (4, 10, 24, 12)
        );
    }

    #[test]
    fn screen_scale_shrinks_and_centers_the_screen() {
        let screen = VideoFrame::solid(32, 32, 0, 0, 255, 0).unwrap();
        let mut layout = solid_layout();
        layout.padding_px = 0;
        layout.screen_scale_pct = 50.0;
        let scene = Scene::from_layout(32, 32, &layout, Some(screen), None).unwrap();
        let layer = scene
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Screen)
            .unwrap();
        assert_eq!(
            (layer.x, layer.y, layer.width, layer.height),
            (8, 8, 16, 16)
        );
        let out = Compositor::composite_cpu(&scene).unwrap();
        assert_eq!(
            pixel(&out, 2, 2)[1],
            255,
            "background shows around the scaled screen"
        );
        assert_eq!(pixel(&out, 16, 16)[2], 255);
    }

    #[test]
    fn webcam_size_percent_and_roundness_apply() {
        let screen = VideoFrame::solid(32, 32, 0, 0, 255, 0).unwrap();
        let webcam = VideoFrame::solid(16, 16, 255, 0, 0, 0).unwrap();
        let mut layout = solid_layout();
        layout.webcam_enabled = true;
        layout.webcam_mirror = false;
        layout.webcam_size_pct = Some(50.0);
        layout.webcam_roundness_pct = 50.0;
        layout.webcam_border_width = 2;
        let scene = Scene::from_layout(64, 64, &layout, Some(screen), Some(webcam)).unwrap();
        let cam = scene
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Webcam)
            .unwrap();
        assert_eq!((cam.width, cam.height), (32, 32));
        assert_eq!(cam.clip, ClipMode::RoundedRect);
        assert!((cam.radius_px - 16.0).abs() < 1e-6);
        let ring = scene
            .layers
            .iter()
            .find(|l| l.role == LayerRole::WebcamBorder)
            .unwrap();
        assert!(
            (ring.radius_px - 18.0).abs() < 1e-6,
            "ring stays concentric"
        );
        let out = Compositor::composite_cpu(&scene).unwrap();
        let (cx, cy) = (cam.x, cam.y);
        assert_ne!(
            pixel(&out, cx, cy)[0],
            255,
            "rounded corner hides the webcam"
        );
        assert_eq!(pixel(&out, cx + 16, cy + 16)[0], 255);
    }

    fn focus_scene(layout: &EditLayout) -> Scene {
        let screen = VideoFrame::solid(32, 32, 0, 0, 255, 0).unwrap();
        let webcam = VideoFrame::solid(16, 12, 255, 0, 0, 0).unwrap();
        Scene::from_layout(64, 48, layout, Some(screen), Some(webcam)).unwrap()
    }

    fn focus_layout() -> EditLayout {
        let mut layout = solid_layout();
        layout.webcam_enabled = true;
        layout.webcam_shape = "circle".into();
        layout.webcam_size_pct = Some(30.0);
        layout.webcam_border_width = 2;
        layout.webcam_shadow = true;
        layout
    }

    #[test]
    fn webcam_focus_leaves_bubble_at_zero_and_fills_canvas_at_one() {
        let layout = focus_layout();
        let bubble = focus_scene(&layout);
        let mut untouched = bubble.clone();
        untouched.apply_webcam_focus(&layout, 100.0, 0.0, 1.0);
        assert_eq!(untouched, bubble);

        let mut full = bubble.clone();
        full.apply_webcam_focus(&layout, 100.0, 1.0, 1.0);
        let cam = full
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Webcam)
            .unwrap();
        assert_eq!((cam.x, cam.y, cam.width, cam.height), (0, 0, 64, 48));
        assert_eq!(cam.clip, ClipMode::None);
        assert_eq!(cam.shadow_opacity, 0.0);
        // Cover crop of a 4:3 source into a 4:3 canvas uses the whole frame, mirrored.
        assert!((cam.uv_x - 1.0).abs() < 1e-6 && (cam.uv_w + 1.0).abs() < 1e-6);
        let ring = full
            .layers
            .iter()
            .find(|l| l.role == LayerRole::WebcamBorder)
            .unwrap();
        assert_eq!((ring.width, ring.height), (64, 48), "the border thins away");
        let out = Compositor::composite_cpu(&full).unwrap();
        assert_eq!(pixel(&out, 0, 0)[0], 255, "webcam covers the corner");
        assert_eq!(pixel(&out, 32, 24)[0], 255, "webcam covers the screen");
    }

    #[test]
    fn webcam_focus_interpolates_size_position_and_radius() {
        let layout = focus_layout();
        let bubble = focus_scene(&layout);
        let start = bubble
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Webcam)
            .unwrap()
            .clone();
        let mut previous = (start.width, start.x);
        for step in 1..=10 {
            let mut scene = bubble.clone();
            scene.apply_webcam_focus(&layout, 80.0, step as f32 / 10.0, 1.0);
            let cam = scene
                .layers
                .iter()
                .find(|l| l.role == LayerRole::Webcam)
                .unwrap();
            assert!(cam.width >= previous.0, "grows monotonically");
            assert!(cam.x <= previous.1, "slides towards the center");
            assert!(cam.radius_px <= cam.width.min(cam.height) as f32 / 2.0 + 1e-3);
            previous = (cam.width, cam.x);
        }
        let mut focused = bubble.clone();
        focused.apply_webcam_focus(&layout, 80.0, 1.0, 1.0);
        let cam = focused
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Webcam)
            .unwrap();
        // 80% of 64x48, centered.
        assert_eq!((cam.x, cam.y, cam.width, cam.height), (7, 5, 51, 38));
        // The circle opens up as a rounded rectangle on the first frame.
        let mut first = bubble.clone();
        first.apply_webcam_focus(&layout, 80.0, 0.01, 1.0);
        let cam = first
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Webcam)
            .unwrap();
        assert_eq!(cam.clip, ClipMode::RoundedRect);
        assert!((cam.radius_px - cam.width.min(cam.height) as f32 / 2.0).abs() < 1.0);
    }

    #[test]
    fn preset_background_renders_without_an_asset() {
        let mut layout = solid_layout();
        layout.background_type = "preset".into();
        layout.background_preset = "sunset".into();
        let scene = Scene::from_layout(48, 27, &layout, None, None).unwrap();
        assert_eq!(scene.layers[0].role, LayerRole::Background);
        let out = Compositor::composite_cpu(&scene).unwrap();
        assert_ne!(
            pixel(&out, 2, 2),
            pixel(&out, 45, 24),
            "preset is not a flat fill"
        );
        assert!(preset_frame(8, 8, "plaid").is_err());
    }

    #[test]
    fn pixel_values_scale_with_canvas_size() {
        let screen = VideoFrame::solid(16, 9, 0, 0, 255, 0).unwrap();
        let mut layout = solid_layout();
        layout.padding_px = 54;
        let small = Scene::from_layout_scaled(
            640,
            360,
            &layout,
            Some(screen.clone()),
            None,
            None,
            layout_px_unit(640, 360),
        )
        .unwrap();
        let large = Scene::from_layout_scaled(
            1920,
            1080,
            &layout,
            Some(screen),
            None,
            None,
            layout_px_unit(1920, 1080),
        )
        .unwrap();
        let s = small
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Screen)
            .unwrap();
        let l = large
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Screen)
            .unwrap();
        assert_eq!(s.y, 18);
        assert_eq!(l.y, 54);
        assert_eq!(s.width * 3, l.width);
    }

    #[test]
    fn zoom_stays_inside_the_crop() {
        let crop = (0.2, 0.1, 0.6, 0.8);
        assert_eq!(zoom_within_crop(crop, (0.0, 0.0, 1.0, 1.0)), crop);
        // A 2x zoom near the top-left corner of the full frame clamps to the crop's corner.
        let (x, y, w, h) = zoom_within_crop(crop, (0.0, 0.0, 0.5, 0.5));
        assert!((x - 0.2).abs() < 1e-6 && (y - 0.1).abs() < 1e-6);
        assert!((w - 0.3).abs() < 1e-6 && (h - 0.4).abs() < 1e-6);
        // A zoom centered inside the crop keeps its center.
        let (x, _, w, _) = zoom_within_crop(crop, (0.25, 0.25, 0.5, 0.5));
        assert!((x + w * 0.5 - 0.5).abs() < 1e-6);
    }

    #[test]
    fn rounded_rect_reveals_background_in_screen_corners() {
        let screen = VideoFrame::solid(24, 24, 0, 0, 255, 0).unwrap();
        let mut layout = solid_layout();
        layout.corner_radius_px = 12;
        let scene = Scene::from_layout(32, 32, &layout, Some(screen), None).unwrap();
        let out = Compositor::composite_cpu(&scene).unwrap();
        let corner = pixel(&out, 4, 4);
        assert!(
            corner[1] > 200 && corner[2] < 40,
            "rounded clip must show green background in the screen AABB corner, got {corner:?}"
        );
        let center = pixel(&out, 16, 16);
        assert!(
            center[2] > 200 && center[1] < 40,
            "screen interior stays red"
        );
    }

    #[test]
    fn circle_and_squircle_clip_webcam_only() {
        let screen = VideoFrame::solid(32, 32, 0, 0, 255, 0).unwrap();
        let webcam = VideoFrame::solid(16, 16, 0, 255, 0, 0).unwrap();
        let mut layout = solid_layout();
        layout.padding_px = 0;
        layout.webcam_enabled = true;
        layout.webcam_shape = "circle".into();
        layout.webcam_position = "top-left".into();
        layout.webcam_size = "xl".into();
        layout.webcam_border_width = 0;
        layout.webcam_mirror = false;
        let scene = Scene::from_layout(32, 32, &layout, Some(screen.clone()), Some(webcam.clone()))
            .unwrap();
        let cam = scene
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Webcam)
            .unwrap();
        assert_eq!(cam.clip, ClipMode::Circle);
        assert_eq!(cam.width, cam.height);
        let out = Compositor::composite_cpu(&scene).unwrap();
        let cam_corner = pixel(&out, cam.x, cam.y);
        assert!(
            cam_corner[2] > 200 && cam_corner[1] < 40,
            "circle clip must not cover the webcam AABB corner, got {cam_corner:?}"
        );
        let cx = cam.x + cam.width / 2;
        let cy = cam.y + cam.height / 2;
        let cam_center = pixel(&out, cx, cy);
        assert!(
            cam_center[1] > 200 && cam_center[2] < 40,
            "circle interior is webcam"
        );
        let screen_far = pixel(&out, 30, 30);
        assert!(screen_far[2] > 200, "screen layer is not circle-clipped");

        layout.webcam_shape = "squircle".into();
        let squircle = Scene::from_layout(32, 32, &layout, Some(screen), Some(webcam)).unwrap();
        let cam = squircle
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Webcam)
            .unwrap();
        assert_eq!(cam.clip, ClipMode::Squircle);
        let out = Compositor::composite_cpu(&squircle).unwrap();
        let cam_corner = pixel(&out, cam.x, cam.y);
        assert!(
            cam_corner[2] > 200 && cam_corner[1] < 40,
            "squircle clip must drop the webcam AABB corner"
        );
    }

    #[test]
    fn circle_webcam_cover_crops_nonsquare_source() {
        let screen = VideoFrame::solid(32, 32, 0, 0, 255, 0).unwrap();
        let webcam = VideoFrame::solid(32, 16, 0, 255, 0, 0).unwrap();
        let mut layout = solid_layout();
        layout.padding_px = 0;
        layout.webcam_enabled = true;
        layout.webcam_shape = "circle".into();
        layout.webcam_position = "top-left".into();
        layout.webcam_size = "xl".into();
        layout.webcam_border_width = 0;
        layout.webcam_mirror = false;
        let scene = Scene::from_layout(32, 32, &layout, Some(screen), Some(webcam)).unwrap();
        let cam = scene
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Webcam)
            .unwrap();
        assert_eq!(cam.width, cam.height);
        assert!(
            (cam.uv_w - 0.5).abs() < 0.05 && (cam.uv_x - 0.25).abs() < 0.05 && cam.uv_h == 1.0,
            "16:9 source in a square bubble must cover-crop, got uv=({}, {}, {}, {})",
            cam.uv_x,
            cam.uv_y,
            cam.uv_w,
            cam.uv_h
        );
    }

    #[test]
    fn shadow_does_not_punch_a_hole_in_background() {
        let screen = VideoFrame::solid(16, 16, 0, 0, 255, 0).unwrap();
        let mut layout = solid_layout();
        layout.padding_px = 8;
        layout.corner_radius_px = 6;
        layout.shadow_blur_px = 8;
        layout.shadow_opacity = 0.8;
        let scene = Scene::from_layout(32, 32, &layout, Some(screen), None).unwrap();
        let screen_layer = scene
            .layers
            .iter()
            .find(|l| l.role == LayerRole::Screen)
            .unwrap();
        let out = Compositor::composite_cpu(&scene).unwrap();
        let interior = pixel(
            &out,
            screen_layer.x + screen_layer.width / 2,
            screen_layer.y + screen_layer.height / 2,
        );
        assert!(
            interior[2] > 200 && interior[1] < 40,
            "opaque screen must cover its own shadow"
        );
        let below = pixel(
            &out,
            screen_layer.x + screen_layer.width / 2,
            (screen_layer.y + screen_layer.height).min(31),
        );
        assert!(
            below[1] > 20 && below[1] < 240 && below[2] < 80,
            "drop shadow should darken green padding, not replace it with a hole, got {below:?}"
        );
        let far = pixel(&out, 1, 1);
        assert!(
            far[1] > 200 && far[2] < 40,
            "background far from the screen must stay green"
        );
        let clip_corner = pixel(&out, screen_layer.x, screen_layer.y);
        assert!(
            clip_corner[1] > 0,
            "rounded corner must keep background (possibly shadowed), not a punched hole"
        );
    }

    #[test]
    fn wallpaper_decodes_project_asset_and_rejects_url() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        let relative = "assets/wallpaper-test.png";
        let mut img = image::RgbaImage::new(8, 8);
        for p in img.pixels_mut() {
            *p = image::Rgba([255, 0, 255, 255]);
        }
        img.save(dir.path().join(relative)).unwrap();
        let mut layout = solid_layout();
        layout.background_type = "wallpaper".into();
        layout.wallpaper_asset = Some(relative.into());
        layout.color_start = "#00ff00".into();
        layout.padding_px = 4;
        let paper = load_wallpaper_frame(dir.path(), &layout, 16, 16)
            .unwrap()
            .expect("decoded wallpaper");
        let screen = VideoFrame::solid(8, 8, 0, 0, 255, 0).unwrap();
        let scene =
            Scene::from_layout_with_wallpaper(16, 16, &layout, Some(screen), None, Some(paper))
                .unwrap();
        let out = Compositor::composite_cpu(&scene).unwrap();
        let pad = pixel(&out, 0, 0);
        assert!(
            pad[0] > 200 && pad[2] > 200 && pad[1] < 40,
            "padding must blit the magenta wallpaper, not the green clear color, got {pad:?}"
        );

        let mut url = layout.clone();
        url.wallpaper_asset = Some("https://example.com/bg.png".into());
        assert!(load_wallpaper_frame(dir.path(), &url, 16, 16)
            .unwrap_err()
            .contains("URL"));
    }

    /// The in-app media check: compositor vs CPU, and an encode/decode round trip.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_media_parity_report_matches() {
        let dir = tempfile::tempdir().unwrap();
        let report = run_parity(dir.path(), &crate::media::EncoderGate::new()).unwrap();
        assert!(report.matched, "parity failed: {:?}", report.diagnostics);
        crate::media::release_decoders();
    }

    /// Textures are kept between frames: a new screen picture, a new canvas size and a new
    /// background key must all show up, and an unchanged key may skip the upload.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter; run with --ignored on a machine that has one"
    )]
    fn gpu_reused_textures_never_show_a_stale_frame() {
        let compositor = Compositor::new().unwrap();
        let scene = |size: u32, background: u8, key: u64, screen: u8| {
            let paper =
                VideoFrame::solid(size, size, background, background, background, 0).unwrap();
            let mut paper = Layer::placed(paper, 0, 0, size, size).with_role(LayerRole::Background);
            paper.cache_key = Some(key);
            let shot = VideoFrame::solid(8, 8, screen, screen, screen, 0).unwrap();
            Scene {
                width: size,
                height: size,
                background: [0.0, 0.0, 0.0, 1.0],
                layers: vec![paper, Layer::placed(shot, 0, 0, size / 2, size / 2)],
            }
        };
        let level =
            |frame: &VideoFrame, x: u32, y: u32| frame.data[(y * frame.stride + x * 4) as usize];
        let first = compositor.composite(&scene(32, 40, 1, 200)).unwrap();
        assert!(level(&first, 4, 4).abs_diff(200) <= 2);
        assert!(level(&first, 28, 28).abs_diff(40) <= 2);
        // Same key: the background is not uploaded again, the screen is.
        let second = compositor.composite(&scene(32, 40, 1, 90)).unwrap();
        assert!(level(&second, 4, 4).abs_diff(90) <= 2);
        assert!(level(&second, 28, 28).abs_diff(40) <= 2);
        // A new key uploads the new background.
        let third = compositor.composite(&scene(32, 160, 2, 90)).unwrap();
        assert!(level(&third, 28, 28).abs_diff(160) <= 2);
        // A new canvas size gets a new target and readback buffer.
        let fourth = compositor.composite(&scene(48, 160, 2, 10)).unwrap();
        assert_eq!((fourth.width, fourth.height), (48, 48));
        assert!(level(&fourth, 4, 4).abs_diff(10) <= 2);
        assert!(level(&fourth, 44, 44).abs_diff(160) <= 2);
    }

    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter; run with --ignored on a machine that has one"
    )]
    fn gpu_cpu_clip_shadow_wallpaper_within_tolerance() {
        let compositor = match Compositor::new() {
            Ok(c) => c,
            Err(err) => panic!("A1 GPU compositor required for contract tests: {err}"),
        };
        let screen = VideoFrame::solid(24, 24, 0, 0, 255, 0).unwrap();
        let webcam = VideoFrame::solid(16, 16, 32, 200, 48, 0).unwrap();
        let mut layout = solid_layout();
        layout.corner_radius_px = 8;
        layout.shadow_blur_px = 6;
        layout.shadow_opacity = 0.6;
        layout.webcam_enabled = true;
        layout.webcam_shape = "circle".into();
        layout.webcam_position = "bottom-right".into();
        layout.webcam_size = "lg".into();
        layout.webcam_border_width = 0;
        layout.webcam_shadow = true;
        layout.webcam_mirror = false;
        let paper = VideoFrame::solid(16, 8, 180, 40, 40, 0).unwrap();
        layout.background_type = "wallpaper".into();
        layout.wallpaper_asset = Some("assets/unused.png".into());
        let scene = Scene::from_layout_with_wallpaper(
            32,
            32,
            &layout,
            Some(screen),
            Some(webcam),
            Some(paper),
        )
        .unwrap();
        let cpu = Compositor::composite_cpu(&scene).unwrap();
        let gpu = compositor.composite(&scene).unwrap();
        let (max, mean) = crate::media::compare_frames(&gpu, &cpu).unwrap();
        assert!(
            mean <= crate::media::COMPOSITOR_MEAN_TOLERANCE,
            "GPU vs CPU mean {mean} max {max}"
        );
        assert!(
            max <= crate::media::COMPOSITOR_MAX_TOLERANCE,
            "GPU vs CPU max {max} mean {mean}"
        );
        let (full_max, full_mean) = full_frame_delta(&gpu, &cpu);
        assert!(
            full_mean <= crate::media::COMPOSITOR_MEAN_TOLERANCE + 2.0,
            "full-frame GPU vs CPU mean {full_mean} max {full_max}"
        );
    }

    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter; run with --ignored on a machine that has one"
    )]
    fn gpu_cpu_crop_scale_roundness_preset_within_tolerance() {
        let compositor = Compositor::new().expect("GPU compositor required for contract tests");
        let screen = split_frame(48, 24, [0, 0, 255], [255, 0, 0]);
        let webcam = VideoFrame::solid(16, 12, 32, 200, 48, 0).unwrap();
        let mut layout = solid_layout();
        layout.background_type = "preset".into();
        layout.background_preset = "ocean".into();
        layout.screen_crop_left = 20.0;
        layout.screen_crop_top = 10.0;
        layout.screen_scale_pct = 80.0;
        layout.corner_radius_px = 6;
        layout.webcam_enabled = true;
        layout.webcam_mirror = false;
        layout.webcam_size_pct = Some(40.0);
        layout.webcam_roundness_pct = 30.0;
        layout.webcam_border_width = 2;
        let scene = Scene::from_layout(48, 32, &layout, Some(screen), Some(webcam)).unwrap();
        let cpu = Compositor::composite_cpu(&scene).unwrap();
        let gpu = compositor.composite(&scene).unwrap();
        let (max, mean) = crate::media::compare_frames(&gpu, &cpu).unwrap();
        assert!(
            mean <= crate::media::COMPOSITOR_MEAN_TOLERANCE
                && max <= crate::media::COMPOSITOR_MAX_TOLERANCE,
            "GPU vs CPU mean {mean} max {max}"
        );
    }

    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter; run with --ignored on a machine that has one"
    )]
    fn gpu_cpu_webcam_focus_mid_transition_within_tolerance() {
        let compositor = Compositor::new().expect("GPU compositor required for contract tests");
        let mut layout = focus_layout();
        layout.webcam_mirror = false;
        let mut scene = focus_scene(&layout);
        scene.apply_webcam_focus(&layout, 90.0, 0.5, 1.0);
        let cpu = Compositor::composite_cpu(&scene).unwrap();
        let gpu = compositor.composite(&scene).unwrap();
        let (max, mean) = crate::media::compare_frames(&gpu, &cpu).unwrap();
        assert!(
            mean <= crate::media::COMPOSITOR_MEAN_TOLERANCE
                && max <= crate::media::COMPOSITOR_MAX_TOLERANCE,
            "GPU vs CPU mean {mean} max {max}"
        );
    }
}
