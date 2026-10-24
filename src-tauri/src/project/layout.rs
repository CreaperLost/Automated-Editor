//! A timeline's style: the canvas, its background, how the screen and the camera are framed and
//! dressed. Where they go is the layout's business (`crate::layouts`).
use super::reader::{open_regular, safe_path};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

pub const MAX_PADDING_PX: u32 = 80;
pub const MAX_CORNER_RADIUS_PX: u32 = 32;
pub const MAX_SHADOW_BLUR_PX: u32 = 40;
pub const MAX_BORDER_WIDTH: u32 = 8;
pub const MAX_WALLPAPER_BYTES: u64 = 8 * 1024 * 1024;
pub const SUPPORTED_ASPECTS: [&str; 4] = ["16:9", "9:16", "4:3", "1:1"];
/// Pixel values (padding, radius, shadow, border) are authored against this canvas short side
/// and scale with the real canvas, so a 1280px preview matches a 1080p or 4K export.
pub const REFERENCE_SHORT_SIDE_PX: f32 = 1080.0;
/// Each screen crop edge, as a percent of the source width or height.
pub const MAX_SCREEN_CROP_PCT: f32 = 45.0;
pub const MIN_SCREEN_SCALE_PCT: f32 = 40.0;
/// Built-in backgrounds rendered by the compositor, so they need no project asset.
pub const BACKGROUND_PRESETS: [&str; 6] =
    ["aurora", "sunset", "ocean", "forest", "candy", "graphite"];

/// Fields missing from a stored style take the values of [`EditLayout::default`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct EditLayout {
    pub aspect_ratio: String,
    pub padding_px: u32,
    /// The project's own background image; shown instead of the style's background.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallpaper_asset: Option<String>,
    // How the frame is dressed: never stored, put on from the project's style as the document
    // is drawn (`style::dress`). `wallpaper` when the project has a wallpaper.
    #[serde(skip)]
    pub background_type: String,
    #[serde(skip)]
    pub color_start: String,
    #[serde(skip)]
    pub color_end: String,
    #[serde(skip)]
    pub background_preset: String,
    #[serde(skip)]
    pub corner_radius_px: u32,
    #[serde(skip)]
    pub shadow_blur_px: u32,
    #[serde(skip)]
    pub shadow_opacity: f32,
    #[serde(skip)]
    pub webcam_border_color: String,
    #[serde(skip)]
    pub webcam_border_width: u32,
    #[serde(skip)]
    pub webcam_shadow: bool,
    /// Percent of the source trimmed from each screen edge.
    pub screen_crop_left: f32,
    pub screen_crop_top: f32,
    pub screen_crop_right: f32,
    pub screen_crop_bottom: f32,
    /// Screen size as a percent of the padded content area.
    pub screen_scale_pct: f32,
    pub webcam_mirror: bool,
    /// Line every camera clip up with the face in it (from its face track), wherever the
    /// camera shows; off keeps the centre crop.
    pub face_framing: bool,
    /// Camera clips (by clip id) framed by hand instead.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub camera_clips: std::collections::BTreeMap<String, crate::shorts::ClipFraming>,
    /// Draw the recorded mouse pointer (recordings that left it out of the video).
    pub cursor_visible: bool,
    /// The pointer's size, percent of its recorded size (a little larger reads better).
    pub cursor_size_pct: f32,
    /// Show keyboard shortcuts pressed in the recording ("Ctrl + S") as a badge, when the
    /// recorder recorded the keyboard.
    pub shortcut_badges: bool,
    /// Fade the webcam bubble out while you click, drag, scroll, rest the pointer or type
    /// under it (it never moves to another corner).
    pub webcam_auto_hide: bool,
    /// Crop the screen to the window in front when it is clearly smaller than the recorded
    /// display, when the recorder recorded the windows.
    pub window_crop: bool,
    /// Room around the window, as a percent of the source height.
    pub window_crop_padding_pct: f32,
    /// Stretches of a recording where the window crop is off.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub window_crop_off: Vec<crate::window_crop::CropOffRange>,
}

/// A new project's framing: the screen a little smaller, the bottom of the screen (the taskbar)
/// cropped off, the camera lined up with the face. The dressing is the Light style's.
impl Default for EditLayout {
    fn default() -> Self {
        Self {
            aspect_ratio: "16:9".into(),
            padding_px: 0,
            background_type: "preset".into(),
            color_start: "#312e81".into(),
            color_end: "#0f172a".into(),
            corner_radius_px: 17,
            shadow_blur_px: 0,
            shadow_opacity: 0.5,
            wallpaper_asset: None,
            background_preset: "forest".into(),
            screen_crop_left: 0.0,
            screen_crop_top: 0.0,
            screen_crop_right: 0.0,
            screen_crop_bottom: 5.5,
            screen_scale_pct: 84.0,
            webcam_border_color: "#6366f1".into(),
            webcam_border_width: MAX_BORDER_WIDTH,
            webcam_mirror: true,
            webcam_shadow: true,
            face_framing: true,
            camera_clips: Default::default(),
            cursor_visible: true,
            cursor_size_pct: 150.0,
            shortcut_badges: true,
            webcam_auto_hide: true,
            window_crop: true,
            window_crop_padding_pct: crate::window_crop::DEFAULT_PADDING_PCT,
            window_crop_off: Vec::new(),
        }
    }
}

impl EditLayout {
    pub fn preview_dimensions(&self) -> Result<(u32, u32), String> {
        match self.aspect_ratio.as_str() {
            "16:9" => Ok((1920, 1080)),
            "9:16" => Ok((1080, 1920)),
            "4:3" => Ok((1440, 1080)),
            "1:1" => Ok((1080, 1080)),
            other => Err(format!("Unknown aspect ratio: {other}")),
        }
    }

    /// Keep synthetic/custom export sizes; remap standard landscape presets
    /// so 9:16 / 1:1 / 4:3 do not stretch inside a 16:9 encoder frame.
    pub fn fit_export_size(&self, width: u32, height: u32) -> Result<(u32, u32), String> {
        self.validate()?;
        let standard = matches!(
            (width, height),
            (1280, 720) | (1920, 1080) | (2560, 1440) | (3840, 2160)
        );
        if !standard {
            return Ok((width, height));
        }
        let short = height;
        let (w, h) = match self.aspect_ratio.as_str() {
            "9:16" => (short, short.saturating_mul(16) / 9),
            "4:3" => (short.saturating_mul(4) / 3, short),
            "1:1" => (short, short),
            _ => (width, height),
        };
        Ok((w & !1, h & !1))
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.cursor_size_pct.is_finite() || !(25.0..=400.0).contains(&self.cursor_size_pct) {
            return Err("Cursor size is out of range".into());
        }
        if !SUPPORTED_ASPECTS.contains(&self.aspect_ratio.as_str()) {
            return Err(format!("Unknown aspect ratio: {}", self.aspect_ratio));
        }
        if self.padding_px > MAX_PADDING_PX {
            return Err("Canvas padding is out of range".into());
        }
        let in_range =
            |value: f32, min: f32, max: f32| value.is_finite() && (min..=max).contains(&value);
        for (edge, value) in [
            ("left", self.screen_crop_left),
            ("top", self.screen_crop_top),
            ("right", self.screen_crop_right),
            ("bottom", self.screen_crop_bottom),
        ] {
            if !in_range(value, 0.0, MAX_SCREEN_CROP_PCT) {
                return Err(format!("Screen crop {edge} is out of range"));
            }
        }
        if !in_range(self.screen_scale_pct, MIN_SCREEN_SCALE_PCT, 100.0) {
            return Err("Screen scale is out of range".into());
        }
        crate::shorts::validate_framing(&self.camera_clips)?;
        if let Some(asset) = self.wallpaper_asset.as_deref() {
            validate_wallpaper_relative(asset)?;
        }
        if !in_range(
            self.window_crop_padding_pct,
            0.0,
            crate::window_crop::MAX_PADDING_PCT,
        ) {
            return Err("Window crop padding is out of range".into());
        }
        if self.window_crop_off.len() > crate::window_crop::MAX_OFF_RANGES {
            return Err("Too many stretches without the window crop".into());
        }
        if self
            .window_crop_off
            .iter()
            .any(|r| r.source_end_us <= r.source_start_us)
        {
            return Err("A stretch without the window crop must end after it starts".into());
        }
        Ok(())
    }

    /// The visible screen source rectangle after cropping, as normalized (x, y, w, h).
    pub fn screen_crop_uv(&self) -> (f32, f32, f32, f32) {
        let pct = |v: f32| v.clamp(0.0, MAX_SCREEN_CROP_PCT) / 100.0;
        let (l, t, r, b) = (
            pct(self.screen_crop_left),
            pct(self.screen_crop_top),
            pct(self.screen_crop_right),
            pct(self.screen_crop_bottom),
        );
        (l, t, 1.0 - l - r, 1.0 - t - b)
    }

    pub fn background_rgba(&self) -> Result<([f32; 4], Option<[f32; 4]>), String> {
        let start = hex_to_rgba(&self.color_start)?;
        match self.background_type.as_str() {
            "gradient" => Ok((start, Some(hex_to_rgba(&self.color_end)?))),
            // Wallpaper decode is a project-asset blit; start color remains the clear color.
            _ => Ok((start, None)),
        }
    }
}

pub fn validate_layout(layout: &EditLayout) -> Result<(), String> {
    layout.validate()
}

pub fn parse_hex_rgb(s: &str) -> Result<[u8; 3], String> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Color must be #RRGGBB".into());
    }
    let n = u32::from_str_radix(hex, 16).map_err(|_| "Invalid color".to_string())?;
    Ok([
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ])
}

pub fn hex_to_rgba(s: &str) -> Result<[f32; 4], String> {
    let [r, g, b] = parse_hex_rgb(s)?;
    Ok([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
}

pub fn validate_wallpaper_relative(relative: &str) -> Result<(), String> {
    if relative.contains("://") || relative.contains('\\') || relative.contains(':') {
        return Err("Wallpaper cannot be an external URL".into());
    }
    let path = Path::new(relative);
    let mut parts = path.components();
    let Some(std::path::Component::Normal(root)) = parts.next() else {
        return Err("Wallpaper asset must live under assets/".into());
    };
    if root != "assets" {
        return Err("Wallpaper asset must live under assets/".into());
    }
    let Some(std::path::Component::Normal(name)) = parts.next() else {
        return Err("Wallpaper asset name is missing".into());
    };
    if parts.next().is_some() {
        return Err("Wallpaper asset path is too deep".into());
    }
    let name = name.to_string_lossy();
    if name.is_empty() || name.len() > 128 || name.contains('\0') {
        return Err("Invalid wallpaper asset name".into());
    }
    let ext = Path::new(name.as_ref())
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg") {
        return Err("Wallpaper must be PNG or JPEG".into());
    }
    Ok(())
}

pub fn ingest_wallpaper(root: &Path, source: &Path) -> Result<String, String> {
    let source_str = source.to_string_lossy();
    if source_str.is_empty() || source_str.contains("://") {
        return Err("Wallpaper cannot be an external URL".into());
    }
    let meta = fs::symlink_metadata(source).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("Wallpaper cannot be a symlink".into());
    }
    if !meta.is_file() {
        return Err("Wallpaper must be a regular file".into());
    }
    if meta.len() == 0 || meta.len() > MAX_WALLPAPER_BYTES {
        return Err("Wallpaper exceeds size limit".into());
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg") {
        return Err("Wallpaper must be PNG or JPEG".into());
    }
    let assets = safe_path(root, "assets")?;
    if let Ok(meta) = fs::symlink_metadata(&assets) {
        if meta.file_type().is_symlink() {
            return Err("Project assets directory cannot be a symlink".into());
        }
        if !meta.is_dir() {
            return Err("Project assets path is not a directory".into());
        }
    }
    fs::create_dir_all(&assets).map_err(|e| e.to_string())?;
    if fs::symlink_metadata(&assets)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Project assets directory cannot be a symlink".into());
    }
    let input = open_regular(source)?;
    let mut bytes = Vec::new();
    input
        .take(MAX_WALLPAPER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_WALLPAPER_BYTES {
        return Err("Wallpaper exceeds size limit".into());
    }
    let relative = format!("assets/wallpaper-{}.{ext}", uuid::Uuid::new_v4());
    validate_wallpaper_relative(&relative)?;
    let dest = safe_path(root, &relative)?;
    if dest.exists() {
        return Err("Wallpaper asset already exists".into());
    }
    let mut temp = tempfile::NamedTempFile::new_in(&assets).map_err(|e| e.to_string())?;
    temp.write_all(&bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(&dest).map_err(|e| e.to_string())?;
    if let Ok(directory) = File::open(&assets) {
        let _ = directory.sync_all();
    }
    Ok(relative)
}

#[cfg(test)]
impl EditLayout {
    /// A neutral style for tests: the screen fills the canvas, nothing cropped or rounded, the
    /// camera without border, shadow or face framing, a gradient behind.
    pub fn plain() -> Self {
        Self {
            background_type: "gradient".into(),
            corner_radius_px: 0,
            screen_crop_bottom: 0.0,
            screen_scale_pct: 100.0,
            webcam_border_width: 0,
            webcam_shadow: false,
            face_framing: false,
            ..Self::default()
        }
    }
}

#[cfg(test)]
impl crate::layouts::Bubble {
    /// The bubble tests use: a plain rectangle bottom right at the old "medium" size.
    pub fn plain() -> Self {
        Self {
            spot: crate::layouts::BubbleSpot::BottomRight,
            size_pct: 20.0,
            shape: crate::layouts::CameraShape::Rect,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_unknown_aspect_padding_and_colors() {
        let mut layout = EditLayout::default();
        layout.aspect_ratio = "21:9".into();
        assert!(layout.validate().unwrap_err().contains("aspect"));
        layout = EditLayout::default();
        layout.padding_px = 81;
        assert!(layout.validate().unwrap_err().contains("padding"));
        // How the frame is dressed is the style's, never stored with the layout.
        let stored = serde_json::to_value(EditLayout::default()).unwrap();
        assert!(stored.get("colorStart").is_none() && stored.get("backgroundType").is_none());
        layout = EditLayout::default();
        layout.wallpaper_asset = Some("../outside.png".into());
        assert!(layout.validate().is_err());
    }

    #[test]
    fn crop_scale_and_camera_style_validate_and_default() {
        // A new project: the bottom 5.5 % cropped off, the screen at 84 %, faces framed.
        let fresh = EditLayout::default();
        fresh.validate().unwrap();
        let (x, y, w, h) = fresh.screen_crop_uv();
        assert!(x == 0.0 && y == 0.0 && w == 1.0 && (h - 0.945).abs() < 1e-6);
        assert_eq!(fresh.screen_scale_pct, 84.0);
        assert!(fresh.face_framing);
        // Fields missing from a stored style take those defaults.
        let sparse: EditLayout =
            serde_json::from_str(r#"{"aspectRatio":"9:16","faceFraming":false}"#).unwrap();
        assert_eq!(
            sparse,
            EditLayout {
                aspect_ratio: "9:16".into(),
                face_framing: false,
                ..EditLayout::default()
            }
        );

        let mut layout = EditLayout::default();
        layout.screen_crop_left = 10.0;
        layout.screen_crop_bottom = 20.0;
        let (x, y, w, h) = layout.screen_crop_uv();
        assert!(
            (x - 0.1).abs() < 1e-6 && y == 0.0 && (w - 0.9).abs() < 1e-6 && (h - 0.8).abs() < 1e-6
        );
        layout.screen_crop_right = 46.0;
        assert!(layout.validate().unwrap_err().contains("crop right"));
        layout = EditLayout::default();
        layout.screen_scale_pct = 30.0;
        assert!(layout.validate().unwrap_err().contains("scale"));
        layout = EditLayout::default();
        layout.camera_clips.insert(
            "c1".into(),
            crate::shorts::ClipFraming {
                offset_x: 2.0,
                offset_y: 0.0,
            },
        );
        assert!(layout.validate().unwrap_err().contains("framing"));
    }

    #[test]
    fn wallpaper_ingest_rejects_url_symlink_and_oversize() {
        let dir = tempdir().unwrap();
        assert!(
            ingest_wallpaper(dir.path(), Path::new("https://example.com/bg.png"))
                .unwrap_err()
                .contains("URL")
        );
        let file = dir.path().join("ok.png");
        fs::write(&file, b"\x89PNG").unwrap();
        let rel = ingest_wallpaper(dir.path(), &file).unwrap();
        assert!(rel.starts_with("assets/wallpaper-"));
        assert!(rel.ends_with(".png"));
        assert!(dir.path().join(&rel).is_file());

        let link = dir.path().join("link.png");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert!(ingest_wallpaper(dir.path(), &link)
                .unwrap_err()
                .contains("symlink"));
        }
    }

    #[test]
    fn export_size_keeps_synthetic_and_remaps_standard_portrait() {
        let mut layout = EditLayout::default();
        assert_eq!(layout.fit_export_size(64, 64).unwrap(), (64, 64));
        assert_eq!(layout.fit_export_size(1920, 1080).unwrap(), (1920, 1080));
        layout.aspect_ratio = "9:16".into();
        assert_eq!(layout.fit_export_size(1920, 1080).unwrap(), (1080, 1920));
        layout.aspect_ratio = "1:1".into();
        assert_eq!(layout.fit_export_size(1920, 1080).unwrap(), (1080, 1080));
    }
}
