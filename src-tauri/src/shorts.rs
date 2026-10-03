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
/// Below 1 the screen zooms out: smaller than its part of the frame, background around it.
pub const MIN_SCREEN_ZOOM: f32 = 0.3;

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
    /// Moves the screen view sideways and up or down: -1 to 1, as a share of how far it can
    /// go inside the crop. Added to where the zooms point when following them.
    pub screen_pan_x: f32,
    pub screen_pan_y: f32,
    pub captions: bool,
    pub caption_spot: CaptionSpot,
    /// Words per caption in this short; 0 uses the editor's setting.
    pub caption_max_words: u32,
    /// Lines a caption may take in this short (1 to 3).
    pub caption_lines: u32,
    /// Caption text size in this short, as a percentage of its height; 0 uses the editor's.
    pub caption_size_pct: f32,
    /// The background behind the screen and camera: `project` (the video's own), `solid`,
    /// `gradient`, `preset` or `wallpaper` (the project's image).
    pub background_type: String,
    pub background_color_start: String,
    pub background_color_end: String,
    pub background_preset: String,
}

impl Default for ShortLayout {
    fn default() -> Self {
        Self {
            camera_position: CameraPosition::Top,
            camera_pct: 35.0,
            screen_zoom: 1.0,
            follow_zooms: true,
            screen_pan_x: 0.0,
            screen_pan_y: 0.0,
            captions: true,
            caption_spot: CaptionSpot::Seam,
            caption_max_words: 0,
            caption_lines: 2,
            caption_size_pct: 0.0,
            background_type: "project".into(),
            background_color_start: "#000000".into(),
            background_color_end: "#1e1b4b".into(),
            background_preset: "aurora".into(),
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
        if !(MIN_SCREEN_ZOOM..=MAX_SCREEN_ZOOM).contains(&self.screen_zoom) {
            return Err(format!(
                "Screen zoom is {MIN_SCREEN_ZOOM}x to {MAX_SCREEN_ZOOM}x"
            ));
        }
        if !matches!(
            self.background_type.as_str(),
            "project" | "solid" | "gradient" | "preset" | "wallpaper"
        ) {
            return Err("Unknown short background".into());
        }
        crate::project::layout::parse_hex_rgb(&self.background_color_start)?;
        crate::project::layout::parse_hex_rgb(&self.background_color_end)?;
        if self.background_preset.is_empty() || self.background_preset.len() > 32 {
            return Err("Unknown background preset".into());
        }
        if !(-1.0..=1.0).contains(&self.screen_pan_x) || !(-1.0..=1.0).contains(&self.screen_pan_y)
        {
            return Err("The screen pans -1 to 1 each way".into());
        }
        if self.caption_max_words > crate::captions::MAX_WORDS_RANGE.1 {
            return Err("Too many words per caption".into());
        }
        if !(1..=3).contains(&self.caption_lines) {
            return Err("A short's captions take 1 to 3 lines".into());
        }
        if self.caption_size_pct != 0.0
            && !(crate::captions::FONT_SIZE_PCT_RANGE.0..=crate::captions::FONT_SIZE_PCT_RANGE.1)
                .contains(&self.caption_size_pct)
        {
            return Err("Caption size is out of range".into());
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

/// The short's background as the project layout it is drawn from: the video's own, or the
/// short's choice of colour, gradient, built-in or the project's image.
pub fn background_layout(
    project: &crate::project::EditLayout,
    short: &ShortLayout,
) -> crate::project::EditLayout {
    let mut layout = project.clone();
    if short.background_type != "project" {
        layout.background_type = short.background_type.clone();
        layout.color_start = short.background_color_start.clone();
        layout.color_end = short.background_color_end.clone();
        layout.background_preset = short.background_preset.clone();
    }
    layout
}

/// Where the screen goes in its `region` (x, y, w, h) and which part of it (UV) shows.
/// `zoom` 1 fills the region (cutting what does not fit); above 1 it is closer; below 1 the
/// whole picture shrinks inside the region with background around it. `center` (UV in the
/// full frame) is what the view centres on; `pan` (-1..1 each way) moves it within reach.
#[allow(clippy::too_many_arguments)]
pub fn screen_placement(
    frame_w: u32,
    frame_h: u32,
    crop: (f32, f32, f32, f32),
    region: (u32, u32, u32, u32),
    zoom: f32,
    center: (f32, f32),
    pan: (f32, f32),
) -> ((u32, u32, u32, u32), (f32, f32, f32, f32)) {
    let (rx, ry, rw, rh) = region;
    let (cx, cy, cw, ch) = crop;
    let crop_px = (
        (cw * frame_w.max(1) as f32).max(1.0),
        (ch * frame_h.max(1) as f32).max(1.0),
    );
    // Screen pixels to frame pixels: covering the region at zoom 1.
    let scale = (rw as f32 / crop_px.0).max(rh as f32 / crop_px.1) * zoom.max(MIN_SCREEN_ZOOM);
    // One axis at a time: (layer start, layer length, uv start, uv length).
    let axis = |r0: u32, rl: u32, c0: f32, cl: f32, frame: u32, center: f32, pan: f32| {
        let shown = cl * frame.max(1) as f32 * scale;
        if shown >= rl as f32 {
            // Bigger than the region: a window of it, panned, kept inside the crop.
            let window = rl as f32 / (frame.max(1) as f32 * scale);
            let middle = center + pan * cl / 2.0;
            let start = (middle - window / 2.0).clamp(c0, c0 + cl - window);
            (r0, rl, start, window)
        } else {
            // Smaller: all of it, centred, and the pan slides it through the free space.
            let free = rl as f32 - shown;
            let start = r0 as f32 + free / 2.0 + pan.clamp(-1.0, 1.0) * free / 2.0;
            (start.round() as u32, (shown.round() as u32).max(1), c0, cl)
        }
    };
    let (x, w, uv_x, uv_w) = axis(rx, rw, cx, cw, frame_w, center.0, pan.0);
    let (y, h, uv_y, uv_h) = axis(ry, rh, cy, ch, frame_h, center.1, pan.1);
    ((x, y, w, h), (uv_x, uv_y, uv_w, uv_h))
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
    /// The imported file (or recording) whose clock the start and end are on; `None` is the
    /// project's recording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// The short's own timeline, once it was edited on its own. Until then it is its stretch
    /// of the video and follows the video's edits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<ShortEdit>,
    /// How long the short plays now. Filled in for the UI and cleared before storing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_us: Option<u64>,
    /// Where the short lies on the edited timeline; absent when an end was cut.
    /// Filled in for the UI and cleared before the document is stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_start_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_end_us: Option<u64>,
}

/// A short's own timeline: V1, its split points and the tracks beside it, in the short's time.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShortEdit {
    pub retained_intervals: Vec<RetainedInterval>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub split_points_us: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlay_tracks: Vec<crate::tracks::OverlayTrack>,
}

impl ShortEdit {
    pub fn of(timeline: &EditDocument) -> Self {
        Self {
            retained_intervals: timeline.retained_intervals.clone(),
            split_points_us: timeline.split_points_us.clone(),
            overlay_tracks: timeline.overlay_tracks.clone(),
        }
    }
}

/// The tracks' clips within edited `[start, end)`, moved to start at 0 and cut to fit.
fn slice_tracks(
    tracks: &[crate::tracks::OverlayTrack],
    start: u64,
    end: u64,
    stills: &dyn Fn(&str) -> bool,
) -> Vec<crate::tracks::OverlayTrack> {
    tracks
        .iter()
        .map(|track| {
            let mut track = track.clone();
            track.clips = track
                .clips
                .into_iter()
                .filter_map(|clip| {
                    let (a, b) = (clip.start_us.max(start), clip.end_us().min(end));
                    if b <= a || b - a < crate::tracks::MIN_CLIP_US {
                        return None;
                    }
                    let skip = a - clip.start_us;
                    Some(crate::tracks::OverlayClip {
                        start_us: a - start,
                        in_us: if stills(&clip.asset_id) {
                            0
                        } else {
                            clip.in_us + skip
                        },
                        duration_us: b - a,
                        ..clip
                    })
                })
                .collect();
            track
        })
        .collect()
}

/// The short as a timeline of its own, in the short's time: its own edit when it has one,
/// else its stretch of the video (V1 and the tracks beside it).
pub fn short_timeline(base: &EditDocument, short: &Short) -> Result<EditDocument, String> {
    let mut timeline = base.clone();
    timeline.shorts.clear();
    timeline.chapters.clear();
    timeline.short_layout = None;
    match &short.edit {
        Some(own) => {
            timeline.retained_intervals = own.retained_intervals.clone();
            timeline.split_points_us = own.split_points_us.clone();
            timeline.overlay_tracks = own.overlay_tracks.clone();
        }
        None => {
            let (start, end) = edited_range_in(short, base)
                .ok_or("Part of this short was cut from the video; adjust it first")?;
            let stills = |asset: &str| {
                base.media_assets
                    .iter()
                    .any(|m| m.id == asset && m.kind == crate::media_bin::MediaKind::Image)
            };
            timeline.retained_intervals = slice_retained(&base.retained_intervals, start, end);
            timeline.overlay_tracks = slice_tracks(&base.overlay_tracks, start, end, &stills);
        }
    }
    Ok(timeline)
}

/// `base` with short `short_id`'s own timeline set from `timeline`.
pub fn with_short_timeline(
    base: &EditDocument,
    short_id: &str,
    timeline: &EditDocument,
) -> Result<EditDocument, String> {
    let mut next = base.clone();
    let short = next
        .shorts
        .iter_mut()
        .find(|s| s.id == short_id)
        .ok_or("That short no longer exists")?;
    short.edit = Some(ShortEdit::of(timeline));
    // Anything else the edit changed (imported media, tracks' new ids) is the project's.
    next.media_assets = timeline.media_assets.clone();
    Ok(next)
}

/// Checks each short's own timeline the way the project's is checked.
pub fn validate_edits(base: &EditDocument) -> Result<(), String> {
    for short in base.shorts.iter().filter(|s| s.edit.is_some()) {
        let timeline = short_timeline(base, short)?;
        crate::project::revision::validate_retained(&timeline.retained_intervals)?;
        crate::project::revision::mapper_for(&timeline.retained_intervals)?;
        crate::project::revision::validate_split_points(&timeline.split_points_us)?;
        crate::tracks::validate(&timeline)?;
        if let Some(missing) = timeline.retained_intervals.iter().find_map(|entry| {
            entry
                .media
                .as_ref()
                .filter(|id| !base.media_assets.iter().any(|asset| &asset.id == *id))
        }) {
            return Err(format!("A short uses media {missing} that is not imported"));
        }
    }
    Ok(())
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
        short.length_us = None;
    }
    shorts.sort_by(|a, b| {
        a.source_start_us
            .cmp(&b.source_start_us)
            .then(a.id.cmp(&b.id))
    });
    shorts
}

/// The edited range a short covers now, if both ends are still on the timeline in order.
pub fn edited_range_in(short: &Short, document: &EditDocument) -> Option<(u64, u64)> {
    match &short.media {
        Some(asset) => edited_range(short, &document.mapper_for_media(asset)),
        None => edited_range(short, &document.mapper().ok()?),
    }
}

/// The edited range of a short whose ends are on `mapper`'s clock.
pub fn edited_range(short: &Short, mapper: &TimelineMapper) -> Option<(u64, u64)> {
    let start = mapper.source_to_edited_us(short.source_start_us)?;
    let end = mapper
        .source_to_edited_us(short.source_end_us.saturating_sub(1))?
        .saturating_add(1);
    (end > start).then_some((start, end))
}

pub fn attach_edited(shorts: &mut [Short], document: &EditDocument) {
    for short in shorts {
        short.length_us = short_timeline(document, short)
            .ok()
            .and_then(|t| t.edited_duration_us().ok());
        // A short with its own edit no longer sits on the video's timeline.
        if short.edit.is_some() {
            short.edited_start_us = None;
            short.edited_end_us = None;
            continue;
        }
        let range = edited_range_in(short, document);
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
                audio_unlinked: entry.audio_unlinked,
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
    let mut document = short_timeline(base, short)?;
    let length = document.edited_duration_us()?;
    if !(MIN_SHORT_US..=MAX_SHORT_US).contains(&length) {
        return Err("A short must be between 3 seconds and 3 minutes long".into());
    }
    document.layout.aspect_ratio = "9:16".into();
    document.captions.enabled = captions && short.layout.captions;
    // The short's own caption choices: fewer words and lines suit a narrow frame.
    if short.layout.caption_max_words > 0 {
        document.captions.max_words = short.layout.caption_max_words;
    }
    document.captions.max_lines = short.layout.caption_lines.clamp(1, 3);
    if short.layout.caption_size_pct > 0.0 {
        document.captions.font_size_pct = short.layout.caption_size_pct;
    }
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
            audio_unlinked: false,
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
            media: None,
            edit: None,
            length_us: None,
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
                audio_unlinked: false,
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
                    media: Some("m1".into()),
                    audio_unlinked: false,
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
        let document = EditDocument::from_retained(vec![ri(0, 60 * S)]).unwrap();
        attach_edited(&mut shorts, &document);
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

    #[test]
    fn a_short_on_an_imported_file_follows_its_clip_and_takes_its_caption_choices() {
        // Recording 0..10 s, then 20 s of the imported file from 5 s into it.
        let base = EditDocument::from_retained(vec![
            ri(0, 10 * S),
            RetainedInterval {
                start_us: 5 * S,
                end_us: 25 * S,
                media: Some("m1".into()),
                audio_unlinked: false,
            },
        ])
        .unwrap();
        let mut on_file = short(10 * S, 20 * S);
        on_file.media = Some("m1".into());
        assert_eq!(edited_range_in(&on_file, &base), Some((15 * S, 25 * S)));
        on_file.layout.caption_max_words = 3;
        on_file.layout.caption_lines = 1;
        on_file.layout.screen_pan_x = -0.5;
        validate(std::slice::from_ref(&on_file)).unwrap();
        let document = short_document(&base, &on_file, true).unwrap();
        assert_eq!(document.edited_duration_us().unwrap(), 10 * S);
        assert_eq!(
            (document.captions.max_words, document.captions.max_lines),
            (3, 1)
        );
        on_file.layout.screen_pan_x = 2.0;
        assert!(validate(&[on_file]).is_err());
    }

    #[test]
    fn screen_placement_fills_zooms_in_and_zooms_out() {
        let full = (0.0, 0.0, 1.0, 1.0);
        // A 1600x900 screen in a 1080x1000 region: covered by height, a window of it shows.
        let region = (0, 920, 1080, 1000);
        let ((x, y, w, h), (_, _, uv_w, uv_h)) =
            screen_placement(1600, 900, full, region, 1.0, (0.5, 0.5), (0.0, 0.0));
        assert_eq!((x, y, w, h), region);
        assert!((uv_h - 1.0).abs() < 1e-4 && uv_w < 1.0);
        // Zoomed in: a smaller window, still filling the region.
        let (_, (_, _, uv_w2, _)) =
            screen_placement(1600, 900, full, region, 2.0, (0.5, 0.5), (0.0, 0.0));
        assert!((uv_w2 - uv_w / 2.0).abs() < 1e-4);
        // Zoomed out far: all of it, smaller than the region and centred in it.
        let ((x, y, w, h), uv) =
            screen_placement(1600, 900, full, region, 0.3, (0.5, 0.5), (0.0, 0.0));
        assert_eq!(uv, full);
        assert!(w < 1080 && h < 1000);
        assert_eq!(x, (1080 - w) / 2);
        assert_eq!(y, 920 + (1000 - h) / 2);
        // Panning left moves the window toward the screen's left edge.
        let (_, (uv_x, _, _, _)) =
            screen_placement(1600, 900, full, region, 1.0, (0.5, 0.5), (-1.0, 0.0));
        assert!(uv_x.abs() < 1e-4);
    }

    #[test]
    fn a_short_can_pick_its_own_background() {
        let project = crate::project::EditLayout::default();
        let mut short = ShortLayout::default();
        assert_eq!(background_layout(&project, &short), project);
        short.background_type = "gradient".into();
        short.background_color_start = "#112233".into();
        short.validate().unwrap();
        let layout = background_layout(&project, &short);
        assert_eq!(
            (layout.background_type.as_str(), layout.color_start.as_str()),
            ("gradient", "#112233")
        );
        short.background_color_end = "blue".into();
        assert!(short.validate().is_err());
    }
}
