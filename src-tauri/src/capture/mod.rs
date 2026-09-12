pub mod preview;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureSourceType {
    Display,
    Window,
    Application,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSource {
    pub id: String,
    pub name: String,
    pub source_type: CaptureSourceType,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitMode {
    Fit,
    Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl SourceRect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_dimensions(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceGeometry {
    pub source_rect: SourceRect,
    pub dest_rect: SourceRect,
    pub fit_mode: FitMode,
    pub preserves_aspect_ratio: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl NormalizedRect {
    pub const FULL: Self = Self {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };
}

pub fn compute_source_geometry(
    source: &CaptureSource,
    dest_width: u32,
    dest_height: u32,
    fit_mode: FitMode,
) -> SourceGeometry {
    let source_rect = SourceRect::from_dimensions(source.width, source.height);
    let dest_rect = match fit_mode {
        FitMode::Fit => fit_letterbox(source.width, source.height, dest_width, dest_height),
        FitMode::Fill => fit_crop(source.width, source.height, dest_width, dest_height),
    };
    SourceGeometry {
        source_rect,
        dest_rect,
        fit_mode,
        preserves_aspect_ratio: matches!(fit_mode, FitMode::Fit),
    }
}

fn fit_letterbox(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> SourceRect {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return SourceRect::from_dimensions(dst_w, dst_h);
    }
    let src_ratio = src_w as f64 / src_h as f64;
    let dst_ratio = dst_w as f64 / dst_h as f64;
    if src_ratio > dst_ratio {
        let w = dst_w;
        let h = ((dst_w as f64) / src_ratio).round() as u32;
        let h = h.min(dst_h).max(1);
        let y = ((dst_h as i32) - (h as i32)) / 2;
        SourceRect::new(0, y, w, h)
    } else {
        let h = dst_h;
        let w = ((dst_h as f64) * src_ratio).round() as u32;
        let w = w.min(dst_w).max(1);
        let x = ((dst_w as i32) - (w as i32)) / 2;
        SourceRect::new(x, 0, w, h)
    }
}

fn fit_crop(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> SourceRect {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return SourceRect::from_dimensions(dst_w, dst_h);
    }
    let src_ratio = src_w as f64 / src_h as f64;
    let dst_ratio = dst_w as f64 / dst_h as f64;
    if src_ratio > dst_ratio {
        let h = dst_h;
        let w = ((dst_h as f64) * src_ratio).round() as u32;
        let x = ((dst_w as i32) - (w as i32)) / 2;
        SourceRect::new(x, 0, w, h)
    } else {
        let w = dst_w;
        let h = ((dst_w as f64) / src_ratio).round() as u32;
        let y = ((dst_h as i32) - (h as i32)) / 2;
        SourceRect::new(0, y, w, h)
    }
}
