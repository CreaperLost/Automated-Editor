//! Preview surface session. Geometry and lifetime are owned here. On macOS pixels go to a
//! native child view; elsewhere frames are JPEG-encoded and fetched by the webview.
use crate::media::ffmpeg::DecodeLimit;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const ARRANGEMENT: &str = "child_overlay";
pub const WEBVIEW_ARRANGEMENT: &str = "webview";
const WEBVIEW_JPEG_QUALITY: u8 = 82;
pub const MAX_VIEWPORT: f64 = 8_192.0;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewViewport {
    pub window_label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub backing_scale: f64,
    pub visible: bool,
    pub occluded: bool,
    pub revision: u64,
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub clip: Option<[f64; 4]>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewHitMode {
    Consume,
    Circle,
    PassThrough,
    CirclePassThrough,
    SquirclePassThrough,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewStatus {
    pub attached: bool,
    pub window_label: Option<String>,
    pub generation: u64,
    pub layout_revision: u64,
    pub arrangement: String,
    pub supported: bool,
    pub presented_kind: String,
    pub copies_per_present: u32,
    pub copies: u64,
    pub presented_bytes: u64,
    pub backing_scale: f64,
    pub physical: Option<PhysicalRect>,
    pub visible: bool,
    pub occluded: bool,
    pub hit_mode: PreviewHitMode,
    /// "native" (Swift child view) or "webview" (frames fetched with `preview_frame`).
    pub surface: String,
    pub diagnostics: Vec<String>,
}

/// How the editor preview is drawn, chosen in the stage toolbar. Playback and scrubbing only;
/// exports always use the export settings.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewQuality {
    /// The canvas's shorter side in pixels (720 draws a 16:9 canvas at 1280x720); 0 draws
    /// it at the project's full size.
    pub resolution: u32,
    /// Frames drawn per second while playing; 0 follows the source.
    pub fps: u32,
}

impl PreviewQuality {
    /// The webview preview copies every frame through a JPEG, so it starts at 720p and 30 fps;
    /// the macOS native view draws at full size and the source rate.
    pub fn default_for(webview: bool) -> Self {
        if webview {
            Self {
                resolution: 720,
                fps: 30,
            }
        } else {
            Self {
                resolution: 0,
                fps: 0,
            }
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.resolution != 0 && !(144..=4320).contains(&self.resolution) {
            return Err("Preview resolution must be between 144p and 4320p".into());
        }
        if self.fps > 120 {
            return Err("Preview frame rate must be at most 120 fps".into());
        }
        Ok(())
    }

    /// The preview canvas for a project canvas of `width` x `height`: scaled down so its
    /// shorter side is at most `resolution`, kept even.
    pub fn canvas(&self, width: u32, height: u32) -> (u32, u32) {
        let short = width.min(height).max(1);
        if self.resolution == 0 || short <= self.resolution {
            return (width, height);
        }
        let fit = |value: u32| {
            let scaled = (value as u64 * self.resolution as u64 / short as u64) as u32;
            (scaled & !1).max(16)
        };
        (fit(width), fit(height))
    }

    /// Sources are decoded no larger than the canvas and no faster than the preview rate. At
    /// full resolution the native view keeps source-size frames, so zooms stay sharp.
    pub fn decode_limit(&self, canvas: (u32, u32), webview: bool) -> DecodeLimit {
        let capped = webview || self.resolution != 0;
        DecodeLimit {
            max_width: if capped { canvas.0 } else { 0 },
            max_height: if capped { canvas.1 } else { 0 },
            max_rate: self.fps,
        }
    }

    /// The time to draw while playing: the start of the frame the clock is in, so a frame is
    /// drawn once rather than on every tick.
    pub fn frame_time(&self, position_us: u64) -> u64 {
        if self.fps == 0 {
            position_us
        } else {
            frame_start_us(position_us, self.fps)
        }
    }
}

/// Start of the `rate`-per-second frame that contains `position_us`.
pub fn frame_start_us(position_us: u64, rate: u32) -> u64 {
    let rate = rate.max(1) as u128;
    let index = position_us as u128 * rate / 1_000_000;
    (index * 1_000_000 / rate) as u64
}

/// JPEG bytes of a BGRA frame, for the webview preview.
pub fn encode_webview_frame(frame: &crate::media::VideoFrame) -> Result<Vec<u8>, String> {
    let row = frame.width as usize * 4;
    let tight;
    let pixels = if frame.stride as usize == row {
        frame
            .data
            .get(..row * frame.height as usize)
            .ok_or("Preview frame buffer is truncated")?
    } else {
        let mut packed = Vec::with_capacity(row * frame.height as usize);
        for y in 0..frame.height as usize {
            let start = y * frame.stride as usize;
            packed.extend_from_slice(
                frame
                    .data
                    .get(start..start + row)
                    .ok_or("Preview frame buffer is truncated")?,
            );
        }
        tight = packed;
        &tight
    };
    let width = u16::try_from(frame.width).map_err(|_| "Preview frame is too wide")?;
    let height = u16::try_from(frame.height).map_err(|_| "Preview frame is too tall")?;
    let mut jpeg = Vec::with_capacity(pixels.len() / 10);
    jpeg_encoder::Encoder::new(&mut jpeg, WEBVIEW_JPEG_QUALITY)
        .encode(pixels, width, height, jpeg_encoder::ColorType::Bgra)
        .map_err(|e| format!("Preview JPEG encode failed: {e}"))?;
    Ok(jpeg)
}

impl PreviewViewport {
    pub fn physical(&self) -> Result<PhysicalRect, String> {
        validate_viewport(self)?;
        let scale = self.backing_scale;
        Ok(PhysicalRect {
            x: (self.x * scale).round() as i32,
            y: (self.y * scale).round() as i32,
            width: (self.width * scale).round() as u32,
            height: (self.height * scale).round() as u32,
        })
    }
}

pub fn validate_viewport(viewport: &PreviewViewport) -> Result<(), String> {
    if viewport.window_label.trim().is_empty() {
        return Err("Preview window label is required".into());
    }
    if [
        viewport.x,
        viewport.y,
        viewport.width,
        viewport.height,
        viewport.backing_scale,
    ]
    .iter()
    .any(|v| !v.is_finite())
        || viewport
            .clip
            .is_some_and(|c| c.iter().any(|v| !v.is_finite()) || c[2] < 0.0 || c[3] < 0.0)
        || viewport.width <= 0.0
        || viewport.height <= 0.0
        || viewport.backing_scale <= 0.0
        || viewport.width > MAX_VIEWPORT
        || viewport.height > MAX_VIEWPORT
        || viewport.backing_scale > 8.0
    {
        return Err("Invalid preview viewport".into());
    }
    Ok(())
}

/// Circle hit region used by the HUD spike: corners pass through to whatever is below.
pub fn circle_consumes(width: f64, height: f64, x: f64, y: f64) -> bool {
    let rx = width / 2.0;
    let ry = height / 2.0;
    if rx <= 0.0 || ry <= 0.0 {
        return false;
    }
    let dx = (x - rx) / rx;
    let dy = (y - ry) / ry;
    dx * dx + dy * dy <= 1.0
}

pub struct PreviewOwner {
    generation: u64,
    window_label: Option<String>,
    viewport: Option<PreviewViewport>,
    attached: bool,
    native: Option<std::ptr::NonNull<std::ffi::c_void>>,
    presented_kind: String,
    copies: u64,
    presented_bytes: u64,
    hit_mode: PreviewHitMode,
    supported: bool,
    /// Latest webview frame: an 8-byte little-endian sequence number, then JPEG bytes.
    web_frame: Option<Arc<Vec<u8>>>,
    web_seq: u64,
}

impl PreviewOwner {
    pub fn new() -> Self {
        Self {
            generation: 0,
            window_label: None,
            viewport: None,
            attached: false,
            native: None,
            presented_kind: "none".into(),
            copies: 0,
            presented_bytes: 0,
            hit_mode: PreviewHitMode::Consume,
            supported: true,
            web_frame: None,
            web_seq: 0,
        }
    }

    pub fn status(&self) -> PreviewStatus {
        PreviewStatus {
            attached: self.attached,
            window_label: self.window_label.clone(),
            generation: self.generation,
            layout_revision: self.viewport.as_ref().map(|v| v.revision).unwrap_or(0),
            arrangement: if self.attached && self.native.is_none() {
                WEBVIEW_ARRANGEMENT.into()
            } else {
                ARRANGEMENT.into()
            },
            supported: self.supported,
            presented_kind: self.presented_kind.clone(),
            copies_per_present: 1,
            copies: self.copies,
            presented_bytes: self.presented_bytes,
            backing_scale: self
                .viewport
                .as_ref()
                .map(|v| v.backing_scale)
                .unwrap_or(1.0),
            physical: self.viewport.as_ref().and_then(|v| v.physical().ok()),
            visible: self.viewport.as_ref().map(|v| v.visible).unwrap_or(false),
            occluded: self.viewport.as_ref().map(|v| v.occluded).unwrap_or(false),
            hit_mode: self.hit_mode,
            surface: if self.native.is_some() {
                "native".into()
            } else {
                "webview".into()
            },
            diagnostics: Vec::new(),
        }
    }

    pub fn attach(
        &mut self,
        window_label: String,
        hit_mode: PreviewHitMode,
        native_window: Option<*mut std::ffi::c_void>,
    ) -> Result<PreviewStatus, String> {
        if !self.supported {
            return Err("Native preview is not implemented on this platform".into());
        }
        self.detach();
        self.generation = self.generation.saturating_add(1).max(1);
        self.window_label = Some(window_label);
        self.hit_mode = hit_mode;
        self.presented_kind = "none".into();
        self.copies = 0;
        self.presented_bytes = 0;
        if let Some(window) = native_window {
            let handle = super::native::attach(window, self.generation)?;
            super::native::set_hit_mode(handle, hit_mode)?;
            self.native = Some(handle);
            self.attached = true;
        } else {
            self.attached = true;
        }
        Ok(self.status())
    }

    pub fn layout(&mut self, viewport: PreviewViewport) -> Result<PreviewStatus, String> {
        self.ensure_attached(&viewport.window_label)?;
        self.ensure_generation(viewport.generation)?;
        validate_viewport(&viewport)?;
        if let Some(current) = &self.viewport {
            if viewport.revision < current.revision {
                return Err("Stale preview layout revision".into());
            }
        }
        if let Some(handle) = self.native {
            super::native::set_geometry(handle, &viewport, self.generation)?;
        }
        self.viewport = Some(viewport);
        Ok(self.status())
    }

    pub fn present_fixed(
        &mut self,
        r: f32,
        g: f32,
        b: f32,
        generation: u64,
    ) -> Result<PreviewStatus, String> {
        self.ensure_open()?;
        self.ensure_generation(generation)?;
        if let Some(handle) = self.native {
            super::native::present_fixed(handle, r, g, b, self.generation)?;
        } else {
            let to_u8 = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
            let frame = crate::media::VideoFrame::solid(16, 16, to_u8(b), to_u8(g), to_u8(r), 0)?;
            self.store_web_frame(encode_webview_frame(&frame)?);
        }
        self.presented_kind = "fixed".into();
        self.copies = self.copies.saturating_add(1);
        self.presented_bytes = 0;
        Ok(self.status())
    }

    pub fn present_fixture(
        &mut self,
        path: &str,
        generation: u64,
    ) -> Result<PreviewStatus, String> {
        self.ensure_open()?;
        self.ensure_generation(generation)?;
        if path.trim().is_empty() {
            return Err("Decode fixture path is required".into());
        }
        let handle = self
            .native
            .ok_or("Native preview view is required to decode a fixture")?;
        super::native::decode_present(handle, path, self.generation)?;
        self.presented_kind = "decoded".into();
        self.copies = self.copies.saturating_add(1);
        Ok(self.status())
    }

    pub fn present_frame(
        &mut self,
        frame: &crate::media::VideoFrame,
        generation: u64,
    ) -> Result<(), String> {
        self.ensure_open()?;
        self.ensure_generation(generation)?;
        match self.native {
            Some(native) => super::native::present_frame(native, frame, generation)?,
            None => self.store_web_frame(encode_webview_frame(frame)?),
        }
        self.presented_kind = "project".into();
        self.copies += 1;
        self.presented_bytes = frame.data.len() as u64;
        Ok(())
    }

    /// Presents a frame already encoded by [`encode_webview_frame`], so the caller can keep
    /// encoding off the UI thread.
    pub fn present_encoded(&mut self, jpeg: Vec<u8>, generation: u64) -> Result<(), String> {
        self.ensure_open()?;
        self.ensure_generation(generation)?;
        if self.native.is_some() {
            return Err("Encoded frames are only for the webview preview".into());
        }
        self.presented_bytes = jpeg.len() as u64;
        self.store_web_frame(jpeg);
        self.presented_kind = "project".into();
        self.copies += 1;
        Ok(())
    }

    fn store_web_frame(&mut self, jpeg: Vec<u8>) {
        self.web_seq += 1;
        let mut payload = Vec::with_capacity(8 + jpeg.len());
        payload.extend_from_slice(&self.web_seq.to_le_bytes());
        payload.extend_from_slice(&jpeg);
        self.web_frame = Some(Arc::new(payload));
    }

    /// The latest webview frame if it is newer than `after_seq`.
    pub fn web_frame_after(&self, after_seq: u64) -> Option<Arc<Vec<u8>>> {
        if !self.attached || self.web_seq <= after_seq {
            return None;
        }
        self.web_frame.clone()
    }

    pub fn hit_test(&self, x: f64, y: f64) -> bool {
        let Some(viewport) = &self.viewport else {
            return false;
        };
        if !viewport.visible || viewport.occluded {
            return false;
        }
        if let Some([cx, cy, w, h]) = viewport.clip {
            if x < cx || y < cy || x >= cx + w || y >= cy + h {
                return false;
            }
        }
        match self.hit_mode {
            PreviewHitMode::Consume => {
                x >= 0.0 && y >= 0.0 && x < viewport.width && y < viewport.height
            }
            PreviewHitMode::Circle => circle_consumes(viewport.width, viewport.height, x, y),
            PreviewHitMode::PassThrough
            | PreviewHitMode::CirclePassThrough
            | PreviewHitMode::SquirclePassThrough => false,
        }
    }

    pub fn detach(&mut self) {
        if let Some(handle) = self.native.take() {
            super::native::detach(handle);
        }
        self.attached = false;
        self.window_label = None;
        self.viewport = None;
        self.web_frame = None;
        self.presented_kind = "none".into();
        self.generation = self.generation.saturating_add(1);
    }

    fn ensure_open(&self) -> Result<(), String> {
        if !self.attached {
            return Err("Preview is not attached".into());
        }
        Ok(())
    }

    fn ensure_attached(&self, window_label: &str) -> Result<(), String> {
        self.ensure_open()?;
        if self.window_label.as_deref() != Some(window_label) {
            return Err("Stale preview window label".into());
        }
        Ok(())
    }

    fn ensure_generation(&self, generation: u64) -> Result<(), String> {
        if generation != 0 && generation != self.generation {
            return Err("Stale preview generation".into());
        }
        Ok(())
    }
}

impl Default for PreviewOwner {
    fn default() -> Self {
        Self::new()
    }
}

// The adapter pointer is only used through Swift `onMain`; Mutex serializes Rust access.
unsafe impl Send for PreviewOwner {}
unsafe impl Sync for PreviewOwner {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_positions_snap_to_frame_starts() {
        assert_eq!(frame_start_us(0, 30), 0);
        assert_eq!(frame_start_us(33_333, 30), 0);
        assert_eq!(frame_start_us(33_334, 30), 33_333);
        assert_eq!(frame_start_us(1_000_000, 30), 1_000_000);
        // Every position inside one frame maps to the same start, so it renders once.
        let starts: std::collections::BTreeSet<_> = (0..1_000_000)
            .step_by(997)
            .map(|us| frame_start_us(us, 30))
            .collect();
        assert_eq!(starts.len(), 30);
    }

    fn viewport(revision: u64) -> PreviewViewport {
        PreviewViewport {
            window_label: "main".into(),
            x: 10.0,
            y: 20.0,
            width: 320.0,
            height: 180.0,
            backing_scale: 2.0,
            visible: true,
            occluded: false,
            revision,
            generation: 0,
            clip: None,
        }
    }

    #[test]
    fn preview_quality_sizes_the_canvas_and_decode() {
        let p720 = PreviewQuality::default_for(true);
        assert_eq!(p720.canvas(1920, 1080), (1280, 720));
        assert_eq!(p720.canvas(1080, 1920), (720, 1280));
        assert_eq!(p720.canvas(1440, 1080), (960, 720));
        assert_eq!(p720.canvas(640, 360), (640, 360));
        let p360 = PreviewQuality {
            resolution: 360,
            fps: 60,
        };
        assert_eq!(p360.canvas(1920, 1080), (640, 360));
        assert_eq!(
            p360.decode_limit((640, 360), false),
            DecodeLimit {
                max_width: 640,
                max_height: 360,
                max_rate: 60
            }
        );
        let full = PreviewQuality::default_for(false);
        assert_eq!(full.canvas(1920, 1080), (1920, 1080));
        assert_eq!(full.decode_limit((1920, 1080), false), DecodeLimit::NONE);
        assert_eq!(full.decode_limit((1920, 1080), true).max_width, 1920);
        assert_eq!(full.frame_time(1_234_567), 1_234_567);
        assert_eq!(p360.frame_time(1_020_000), 1_016_666);
        assert!(PreviewQuality {
            resolution: 100,
            fps: 30
        }
        .validate()
        .is_err());
        assert!(PreviewQuality {
            resolution: 0,
            fps: 240
        }
        .validate()
        .is_err());
        assert!(p360.validate().is_ok() && full.validate().is_ok());
    }

    #[test]
    fn webview_frame_round_trips_through_jpeg() {
        let frame = crate::media::VideoFrame::solid(64, 32, 20, 40, 220, 0).unwrap();
        let jpeg = encode_webview_frame(&frame).unwrap();
        let decoded = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_eq!(decoded.dimensions(), (64, 32));
        let px = decoded.get_pixel(32, 16).0;
        for (got, want) in px.iter().zip([220u8, 40, 20]) {
            assert!(got.abs_diff(want) <= 8, "decoded {px:?}");
        }
    }

    #[test]
    fn logical_points_map_to_physical_pixels() {
        let physical = viewport(1).physical().unwrap();
        assert_eq!(physical.x, 20);
        assert_eq!(physical.y, 40);
        assert_eq!(physical.width, 640);
        assert_eq!(physical.height, 360);
    }

    #[test]
    fn stale_layout_and_generation_are_rejected() {
        let mut owner = PreviewOwner::new();
        owner
            .attach("main".into(), PreviewHitMode::Consume, None)
            .unwrap();
        owner.layout(viewport(2)).unwrap();
        assert!(owner.layout(viewport(1)).unwrap_err().contains("Stale"));
        let generation = owner.status().generation;
        owner.present_fixed(0.1, 0.2, 0.3, generation).unwrap();
        assert!(owner
            .present_fixed(1.0, 0.0, 0.0, generation + 9)
            .unwrap_err()
            .contains("Stale"));
        let json = serde_json::to_value(owner.status()).unwrap();
        assert!(json.get("pixels").is_none());
        assert!(json.get("samples").is_none());
        // No native window was given, so frames go to the webview.
        assert_eq!(json["arrangement"], "webview");
        assert_eq!(json["surface"], "webview");
        assert_eq!(json["copiesPerPresent"], 1);
        assert_eq!(owner.status().presented_kind, "fixed");
        assert!(owner
            .present_fixture("/tmp/missing-f1.mp4", generation)
            .unwrap_err()
            .contains("Native preview view"));
        let fixed = owner.web_frame_after(0).expect("fixed colour frame");
        assert_eq!(&fixed[..8], &1u64.to_le_bytes());
        assert_eq!(&fixed[8..10], &[0xFF, 0xD8], "payload is a JPEG");
        assert!(owner.web_frame_after(1).is_none());
        owner.detach();
        assert!(owner.web_frame_after(0).is_none());
        owner
            .attach("main".into(), PreviewHitMode::Consume, None)
            .unwrap();
        assert_ne!(owner.status().generation, generation);
        owner.detach();
        assert!(!owner.status().attached);
    }

    #[test]
    fn circle_hit_mode_passes_transparent_corners() {
        let mut owner = PreviewOwner::new();
        owner
            .attach("camera_overlay".into(), PreviewHitMode::Circle, None)
            .unwrap();
        owner
            .layout(PreviewViewport {
                window_label: "camera_overlay".into(),
                x: 0.0,
                y: 0.0,
                width: 120.0,
                height: 120.0,
                backing_scale: 1.0,
                visible: true,
                occluded: false,
                revision: 1,
                generation: 0,
                clip: None,
            })
            .unwrap();
        assert!(owner.hit_test(60.0, 60.0));
        assert!(!owner.hit_test(1.0, 1.0));
        owner.detach();
        owner
            .attach("camera_overlay".into(), PreviewHitMode::Circle, None)
            .unwrap();
        owner.detach();
        assert!(!owner.status().attached);
    }
}
