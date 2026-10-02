//! Shorts: vertical clips cut from the long video. A short is anchored in source time at the
//! first and last word it covers, so it follows later edits; it exports as a 9:16 video made of
//! that stretch of the current edit, with the project's zooms, webcam focus and captions.
use crate::project::revision::EditDocument;
use crate::project::RetainedInterval;
use crate::timeline::TimelineMapper;
use serde::{Deserialize, Serialize};

pub const MAX_SHORTS: usize = 50;
pub const MIN_SHORT_US: u64 = 3_000_000;
pub const MAX_SHORT_US: u64 = 180_000_000;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CameraPosition {
    #[default]
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptionSpot {
    /// Centred on the line between camera and screen.
    #[default]
    Seam,
    /// Near the bottom of the screen part.
    Screen,
    /// Near the bottom of the camera part.
    Camera,
}

pub const MIN_CAMERA_PCT: f32 = 20.0;
pub const MAX_CAMERA_PCT: f32 = 70.0;
pub const MAX_SCREEN_ZOOM: f32 = 3.0;

/// The split-screen look of a short: the camera across the top or bottom, the screen in the
/// rest, cut to that shape and following the project's zooms.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortLayout {
    pub camera_position: CameraPosition,
    /// Share of the frame height the camera takes.
    pub camera_pct: f32,
    /// Extra zoom on the screen part, on top of the project's zooms. 1 shows as much as fits.
    pub screen_zoom: f32,
    /// Move the screen view to where the project's zooms point.
    pub follow_zooms: bool,
    pub captions: bool,
    pub caption_spot: CaptionSpot,
}

impl Default for ShortLayout {
    fn default() -> Self {
        Self {
            camera_position: CameraPosition::Top,
            camera_pct: 35.0,
            screen_zoom: 1.0,
            follow_zooms: true,
            captions: true,
            caption_spot: CaptionSpot::Seam,
        }
    }
}

impl ShortLayout {
    pub fn validate(&self) -> Result<(), String> {
        if !(MIN_CAMERA_PCT..=MAX_CAMERA_PCT).contains(&self.camera_pct) {
            return Err(format!(
                "The camera takes {MIN_CAMERA_PCT}% to {MAX_CAMERA_PCT}% of a short's height"
            ));
        }
        if !(1.0..=MAX_SCREEN_ZOOM).contains(&self.screen_zoom) {
            return Err(format!("Screen zoom is 1x to {MAX_SCREEN_ZOOM}x"));
        }
        Ok(())
    }
}

/// Pixel rectangles (x, y, width, height) of a split frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitRects {
    pub camera: Option<(u32, u32, u32, u32)>,
    pub screen: (u32, u32, u32, u32),
}

/// Where camera and screen go. Without a camera the screen takes the whole frame.
pub fn split_rects(width: u32, height: u32, layout: &ShortLayout, has_camera: bool) -> SplitRects {
    if !has_camera {
        return SplitRects {
            camera: None,
            screen: (0, 0, width, height),
        };
    }
    let camera_h =
        ((height as f32 * layout.camera_pct / 100.0).round() as u32).clamp(1, height - 1);
    let screen_h = height - camera_h;
    match layout.camera_position {
        CameraPosition::Top => SplitRects {
            camera: Some((0, 0, width, camera_h)),
            screen: (0, camera_h, width, screen_h),
        },
        CameraPosition::Bottom => SplitRects {
            camera: Some((0, screen_h, width, camera_h)),
            screen: (0, 0, width, screen_h),
        },
    }
}

/// The part of the screen recording (as UV) shown in a `region_w` x `region_h` area: the
/// region's shape cut from inside the crop, `zoom` times closer, centred on `center` (UV in the
/// full frame) and kept inside the crop.
#[allow(clippy::too_many_arguments)]
pub fn screen_window(
    frame_w: u32,
    frame_h: u32,
    crop: (f32, f32, f32, f32),
    region_w: u32,
    region_h: u32,
    zoom: f32,
    center: (f32, f32),
) -> (f32, f32, f32, f32) {
    let (cx, cy, cw, ch) = crop;
    let crop_w = cw * frame_w.max(1) as f32;
    let crop_h = ch * frame_h.max(1) as f32;
    let aspect = region_w.max(1) as f32 / region_h.max(1) as f32;
    // The largest window of the region's shape that fits in the crop, then zoomed.
    let (mut w, mut h) = if crop_w / crop_h > aspect {
        (crop_h * aspect, crop_h)
    } else {
        (crop_w, crop_w / aspect)
    };
    let zoom = zoom.max(1.0);
    w /= zoom;
    h /= zoom;
    let uv_w = w / frame_w.max(1) as f32;
    let uv_h = h / frame_h.max(1) as f32;
    let x = (center.0 - uv_w / 2.0).clamp(cx, cx + cw - uv_w);
    let y = (center.1 - uv_h / 2.0).clamp(cy, cy + ch - uv_h);
    (x, y, uv_w, uv_h)
}

/// The caption's top edge in a split frame.
pub fn caption_y(spot: CaptionSpot, rects: &SplitRects, caption_h: u32, canvas_h: u32) -> u32 {
    let margin = canvas_h / 40;
    let bottom_of = |(_, y, _, h): (u32, u32, u32, u32)| (y + h).saturating_sub(caption_h + margin);
    let y = match (spot, rects.camera) {
        (CaptionSpot::Seam, Some((_, cam_y, _, cam_h))) => {
            let seam = if cam_y == 0 { cam_h } else { cam_y };
            seam.saturating_sub(caption_h / 2)
        }
        (CaptionSpot::Camera, Some(camera)) => bottom_of(camera),
        _ => bottom_of(rects.screen),
    };
    y.min(canvas_h.saturating_sub(caption_h))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Short {
    pub id: String,
    pub title: String,
    pub source_start_us: u64,
    pub source_end_us: u64,
    /// Why the AI picked it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    /// How the vertical frame is split between camera and screen.
    #[serde(default)]
    pub layout: ShortLayout,
    /// Where the short lies on the edited timeline; absent when an end was cut.
    /// Filled in for the UI and cleared before the document is stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_start_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_end_us: Option<u64>,
}

pub fn validate(shorts: &[Short]) -> Result<(), String> {
    if shorts.len() > MAX_SHORTS {
        return Err(format!("Keep at most {MAX_SHORTS} shorts"));
    }
    let mut ids = std::collections::HashSet::new();
    for short in shorts {
        if short.id.is_empty() || short.id.len() > 64 || !ids.insert(short.id.as_str()) {
            return Err("Short ids must be unique".into());
        }
        let title = short.title.trim();
        if title.is_empty() || title.chars().count() > 100 || title.chars().any(char::is_control) {
            return Err("A short needs a one-line title of at most 100 characters".into());
        }
        if short.source_end_us <= short.source_start_us {
            return Err("A short must end after it starts".into());
        }
        if short.reason.chars().count() > 300 {
            return Err("A short's note is too long".into());
        }
        short.layout.validate()?;
    }
    Ok(())
}

pub fn normalized(mut shorts: Vec<Short>) -> Vec<Short> {
    for short in &mut shorts {
        short.title = short.title.trim().to_string();
        short.edited_start_us = None;
        short.edited_end_us = None;
    }
    shorts.sort_by(|a, b| {
        a.source_start_us
            .cmp(&b.source_start_us)
            .then(a.id.cmp(&b.id))
    });
    shorts
}

/// The edited range a short covers now, if both ends are still on the timeline in order.
pub fn edited_range(short: &Short, mapper: &TimelineMapper) -> Option<(u64, u64)> {
    let start = mapper.source_to_edited_us(short.source_start_us)?;
    let end = mapper
        .source_to_edited_us(short.source_end_us.saturating_sub(1))?
        .saturating_add(1);
    (end > start).then_some((start, end))
}

pub fn attach_edited(shorts: &mut [Short], mapper: &TimelineMapper) {
    for short in shorts {
        let range = edited_range(short, mapper);
        short.edited_start_us = range.map(|r| r.0);
        short.edited_end_us = range.map(|r| r.1);
    }
}

/// The retained entries covering edited `[start, end)`, in playback order, imported media
/// included.
pub fn slice_retained(
    retained: &[RetainedInterval],
    start: u64,
    end: u64,
) -> Vec<RetainedInterval> {
    let mut out = Vec::new();
    let mut cursor = 0u64;
    for entry in retained {
        let length = entry.end_us - entry.start_us;
        let (a, b) = (start.max(cursor), end.min(cursor + length));
        if a < b {
            out.push(RetainedInterval {
                start_us: entry.start_us + (a - cursor),
                end_us: entry.start_us + (b - cursor),
                media: entry.media.clone(),
            });
        }
        cursor += length;
    }
    out
}

/// The document a short exports from: the short's stretch of the edit on a 9:16 canvas, with
/// captions on when `captions` is set.
pub fn short_document(
    base: &EditDocument,
    short: &Short,
    captions: bool,
) -> Result<EditDocument, String> {
    let mapper = base.mapper()?;
    let (start, end) = edited_range(short, &mapper)
        .ok_or("Part of this short was cut from the video; adjust it first")?;
    let length = end - start;
    if !(MIN_SHORT_US..=MAX_SHORT_US).contains(&length) {
        return Err("A short must be between 3 seconds and 3 minutes long".into());
    }
    let mut document = base.clone();
    document.retained_intervals = slice_retained(&base.retained_intervals, start, end);
    document.layout.aspect_ratio = "9:16".into();
    document.captions.enabled = captions && short.layout.captions;
    document.short_layout = Some(short.layout.clone());
    document.chapters.clear();
    document.shorts.clear();
    document.split_points_us.clear();
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u64 = 1_000_000;

    fn ri(start_us: u64, end_us: u64) -> RetainedInterval {
        RetainedInterval {
            start_us,
            end_us,
            media: None,
        }
    }

    fn short(start: u64, end: u64) -> Short {
        Short {
            id: "s1".into(),
            title: "Best bit".into(),
            source_start_us: start,
            source_end_us: end,
            reason: String::new(),
            layout: ShortLayout::default(),
            edited_start_us: None,
            edited_end_us: None,
        }
    }

    #[test]
    fn shorts_slice_the_current_edit() {
        // 0-10 s, imported media 0-4 s, then 20-40 s.
        let mut base =
            EditDocument::from_retained(vec![ri(0, 10 * S), ri(20 * S, 40 * S)]).unwrap();
        base.retained_intervals.insert(
            1,
            RetainedInterval {
                start_us: 0,
                end_us: 4 * S,
                media: Some("m1".into()),
            },
        );
        // From 6 s into the recording to 25 s: crosses the media clip.
        let s = short(6 * S, 25 * S);
        let mapper = base.mapper().unwrap();
        assert_eq!(edited_range(&s, &mapper), Some((6 * S, 19 * S)));
        let doc = short_document(&base, &s, true).unwrap();
        assert_eq!(
            doc.retained_intervals,
            vec![
                ri(6 * S, 10 * S),
                RetainedInterval {
                    start_us: 0,
                    end_us: 4 * S,
                    media: Some("m1".into())
                },
                ri(20 * S, 25 * S)
            ]
        );
        assert_eq!(doc.layout.aspect_ratio, "9:16");
        assert!(doc.captions.enabled);
        assert_eq!(doc.edited_duration_us().unwrap(), 13 * S);

        // An end that was cut makes the short unusable until it is adjusted.
        let cut = EditDocument::from_retained(vec![ri(0, 10 * S), ri(30 * S, 40 * S)]).unwrap();
        assert!(edited_range(&s, &cut.mapper().unwrap()).is_none());
        assert!(short_document(&cut, &s, false).is_err());
        assert!(
            short_document(&base, &short(0, S), false).is_err(),
            "too short"
        );
    }

    #[test]
    fn validation_and_ui_fields() {
        let mut shorts = vec![short(5 * S, 30 * S)];
        validate(&shorts).unwrap();
        let mapper = EditDocument::from_retained(vec![ri(0, 60 * S)])
            .unwrap()
            .mapper()
            .unwrap();
        attach_edited(&mut shorts, &mapper);
        let json = serde_json::to_value(&shorts).unwrap();
        assert_eq!(json[0]["editedStartUs"], 5 * S);
        let stored = serde_json::to_value(normalized(shorts)).unwrap();
        assert!(stored[0].get("editedStartUs").is_none());
        assert!(validate(&[short(5, 5)]).is_err());
        let mut untitled = short(0, S);
        untitled.title = " ".into();
        assert!(validate(&[untitled]).is_err());
    }

    #[test]
    fn split_frames_and_screen_windows() {
        let layout = ShortLayout::default();
        let rects = split_rects(1080, 1920, &layout, true);
        assert_eq!(rects.camera, Some((0, 0, 1080, 672)));
        assert_eq!(rects.screen, (0, 672, 1080, 1248));
        let bottom = ShortLayout {
            camera_position: CameraPosition::Bottom,
            camera_pct: 50.0,
            ..ShortLayout::default()
        };
        let rects_b = split_rects(1080, 1920, &bottom, true);
        assert_eq!(rects_b.camera, Some((0, 960, 1080, 960)));
        assert_eq!(rects_b.screen, (0, 0, 1080, 960));
        assert_eq!(
            split_rects(1080, 1920, &layout, false).screen,
            (0, 0, 1080, 1920)
        );

        // A 1920x1080 screen in a 1080x1248 region: full height, a centred slice of the width.
        let (x, y, w, h) = screen_window(
            1920,
            1080,
            (0.0, 0.0, 1.0, 1.0),
            1080,
            1248,
            1.0,
            (0.5, 0.5),
        );
        assert!((h - 1.0).abs() < 1e-6 && y == 0.0);
        assert!(
            (w - (1080.0 * 1080.0 / 1248.0) / 1920.0).abs() < 1e-4,
            "{w}"
        );
        assert!((x - (0.5 - w / 2.0)).abs() < 1e-6);
        // Zoomed 2x towards the top-right corner: half the size, kept inside the frame.
        let (x2, y2, w2, h2) = screen_window(
            1920,
            1080,
            (0.0, 0.0, 1.0, 1.0),
            1080,
            1248,
            2.0,
            (0.95, 0.05),
        );
        assert!((w2 - w / 2.0).abs() < 1e-6 && (h2 - 0.5).abs() < 1e-6);
        assert!((x2 + w2 - 1.0).abs() < 1e-6 && y2 == 0.0);
        // A crop limits where the window can go.
        let (x3, _, _, _) = screen_window(
            1920,
            1080,
            (0.1, 0.0, 0.8, 1.0),
            1080,
            1248,
            1.0,
            (0.0, 0.5),
        );
        assert!((x3 - 0.1).abs() < 1e-6);

        // Captions: on the seam, in the screen part, or in the camera part.
        assert_eq!(caption_y(CaptionSpot::Seam, &rects, 100, 1920), 622);
        assert_eq!(
            caption_y(CaptionSpot::Screen, &rects, 100, 1920),
            1920 - 100 - 48
        );
        assert_eq!(
            caption_y(CaptionSpot::Camera, &rects, 100, 1920),
            672 - 100 - 48
        );
        assert_eq!(caption_y(CaptionSpot::Seam, &rects_b, 100, 1920), 910);

        assert!(ShortLayout {
            camera_pct: 10.0,
            ..ShortLayout::default()
        }
        .validate()
        .is_err());
        assert!(ShortLayout {
            screen_zoom: 4.0,
            ..ShortLayout::default()
        }
        .validate()
        .is_err());
    }
}
