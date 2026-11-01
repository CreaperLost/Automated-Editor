//! Shorts: vertical clips cut from the long video. A short is anchored in source time at the
//! first and last word it covers, so it follows later edits; it exports as a 9:16 video made of
//! that stretch of the current edit, with the project's zooms and captions and its own layouts.
use crate::project::revision::EditDocument;
use crate::sequence::Sequence;
use crate::timeline::TimelineMapper;
use serde::{Deserialize, Serialize};

pub const MAX_SHORTS: usize = 50;
pub const MIN_SHORT_US: u64 = 3_000_000;
pub const MAX_SHORT_US: u64 = 180_000_000;

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

/// A camera clip framed by hand instead of centred on the face: where its crop sits, -1 to 1
/// each way from the centre (see `render::focus_uv`).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ClipFraming {
    pub offset_x: f32,
    pub offset_y: f32,
}

/// More clips framed by hand than any short has.
pub const MAX_FRAMED_CLIPS: usize = 500;

pub const MAX_SCREEN_ZOOM: f32 = 3.0;
/// Below 1 the screen zooms out: smaller than its part of the frame, background around it.
pub const MIN_SCREEN_ZOOM: f32 = 0.3;

/// The look of a short: how its screen and camera are framed, its captions and background.
/// Where they go is its layouts' business ([`Short::layouts`]).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortLayout {
    /// Camera clips (by clip id) framed by hand. Every other camera clip is lined up with the
    /// face as it starts, zooming in a little when the face is near an edge.
    pub camera_clips: std::collections::BTreeMap<String, ClipFraming>,
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
    /// Where each graphic plays in this short, by its clip id; a graphic not listed plays
    /// full screen.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub graphic_slots: std::collections::BTreeMap<String, GraphicSlot>,
    /// Camera push-ins (FX-3): at a jump cut in the camera, the next stretch plays a little
    /// closer (and the one after back out), so the cut reads as a cut in rather than a jump.
    /// Off unless turned on.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub push_ins: bool,
}

/// The part of a short's frame a graphic plays in (and is rendered for).
#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum GraphicSlot {
    /// The whole frame, over everything.
    #[default]
    Full,
    /// The camera's part of the short's base layout, over everything there.
    Camera,
    /// The screen's part of the base layout, over everything there.
    Screen,
    /// Over the screen's part, under the captions; the graphic's see-through parts show the
    /// screen.
    OverScreen,
    /// Over the face: the camera fills the frame while it plays and the graphic is drawn over
    /// the whole of it, its see-through parts showing the face (callouts, labels, arrows).
    OverFace,
}

impl GraphicSlot {
    /// How it shows, for the user.
    pub fn describe(self) -> &'static str {
        match self {
            GraphicSlot::Full => "over the whole frame",
            GraphicSlot::Camera => "in the camera's part of the split",
            GraphicSlot::Screen => "split: the face above, the graphic below",
            GraphicSlot::OverScreen => "over the screen's part, under the captions",
            GraphicSlot::OverFace => "over the face, which fills the frame",
        }
    }

    /// What the graphic is told of it (`data-mode` on its root): `full`, `split` (a part of the
    /// frame beside the face) or `over` (drawn over a picture that shows through).
    pub fn data_mode(self) -> &'static str {
        match self {
            GraphicSlot::Full => "full",
            GraphicSlot::Camera | GraphicSlot::Screen => "split",
            GraphicSlot::OverScreen | GraphicSlot::OverFace => "over",
        }
    }
}

/// The split a graphic in the camera's or screen's part plays in: the short's base layout when
/// that is a split, else the shorts' default one.
pub fn split_layout(layouts: &crate::layouts::LayoutTrack) -> crate::layouts::Layout {
    match &layouts.base {
        base @ crate::layouts::Layout::SideBySide(_) => base.clone(),
        _ => crate::layouts::LayoutTrack::for_short().base,
    }
}

/// While a graphic shows split or over the face, the layout it needs: the split, or the camera
/// filling the frame. Sections in the short's anchor media's time (the take's voice, or the
/// recording), made fresh each time the short is played or exported, never stored.
fn graphic_sections(timeline: &EditDocument, short: &Short) -> Vec<crate::layouts::LayoutSection> {
    use crate::layouts::{CameraOnly, Layout, LayoutSection};
    let anchor = match &short.media {
        Some(media) => Some(media.clone()),
        None => timeline
            .assets
            .iter()
            .find(|a| a.is_recording())
            .map(|a| a.id.clone()),
    };
    let Some(anchor) = anchor else {
        return Vec::new();
    };
    let clock = crate::sequence::clock::asset_clock(&timeline.sequence, &anchor);
    let mut out = Vec::new();
    for track in timeline.sequence.video_tracks().filter(|t| !t.hidden) {
        for clip in &track.clips {
            let layout = match graphic_slot(&short.layout, &clip.id) {
                GraphicSlot::Camera | GraphicSlot::Screen => split_layout(&timeline.layouts),
                GraphicSlot::OverFace => Layout::CameraOnly(CameraOnly::default()),
                _ => continue,
            };
            if !timeline.asset(&clip.asset).is_some_and(|a| a.is_graphic()) {
                continue;
            }
            for (s, e) in clock.edited_range_to_source(clip.start_us, clip.end_us()) {
                out.push(LayoutSection {
                    rule: Some("graphic".into()),
                    ..LayoutSection::new(
                        format!("graphic-{}", clip.id),
                        short.media.clone(),
                        s,
                        e,
                        Some(layout.clone()),
                    )
                });
            }
        }
    }
    out
}

/// At most this many graphics have a slot in one short.
pub const MAX_GRAPHIC_SLOTS: usize = 200;

/// The slot graphic clip `clip_id` plays in.
pub fn graphic_slot(layout: &ShortLayout, clip_id: &str) -> GraphicSlot {
    layout
        .graphic_slots
        .get(clip_id)
        .copied()
        .unwrap_or_default()
}

/// Where graphic clip `clip_id` plays in a `width` x `height` frame of `document` (a short's):
/// the whole frame, or a part of its base layout (sections do not move it).
pub fn graphic_rect(
    document: &EditDocument,
    clip_id: &str,
    width: u32,
    height: u32,
) -> (u32, u32, u32, u32) {
    match &document.short_layout {
        Some(short) => slot_rect(
            graphic_slot(short, clip_id),
            &document.layouts,
            &document.layout,
            width,
            height,
        ),
        None => (0, 0, width, height),
    }
}

/// The box a graphic in `slot` of `short` lays itself out in (its root's `data-width` x
/// `data-height`): the graphic render size of the 9:16 frame, or of its part of the split as the
/// short's split is set now. What the agent is told so it writes for that box.
pub fn graphic_box(base: &EditDocument, short: &Short, slot: GraphicSlot) -> (u32, u32) {
    let (width, height) = crate::graphics::render::render_size(1080, 1920);
    let (_, _, w, h) = slot_rect(slot, &short.layouts, &base.layout, width, height);
    ((w & !1).max(2), (h & !1).max(2))
}

/// `graphic_box` for the agent: "a 1080x1248 box (portrait)", and which of its layouts it
/// takes when that is not its own shape's (see [`crate::graphics::as_split_part`]).
pub fn describe_box((width, height): (u32, u32)) -> String {
    let ratio = width as f64 / height.max(1) as f64;
    let shape = if (ratio - 1.0).abs() < 0.1 {
        "about square, portrait rules"
    } else if ratio < 1.0 {
        "portrait"
    } else if ratio <= crate::graphics::SPLIT_PORTRAIT_MAX {
        "landscape, portrait rules"
    } else {
        "landscape"
    };
    format!("a {width}x{height} box ({shape})")
}

/// Where a graphic in `slot` plays in a `width` x `height` frame of a short with `layouts` (its
/// base layout is the split) and `style`.
fn slot_rect(
    slot: GraphicSlot,
    layouts: &crate::layouts::LayoutTrack,
    style: &crate::project::EditLayout,
    width: u32,
    height: u32,
) -> (u32, u32, u32, u32) {
    use crate::layouts::arrange::{arrange, Canvas, Sources};
    let whole = (0, 0, width, height);
    if matches!(slot, GraphicSlot::Full | GraphicSlot::OverFace) {
        return whole;
    }
    let parts = arrange(
        &split_layout(layouts),
        style,
        Canvas::new(width, height),
        Sources {
            screen: Some((1920, 1080)),
            camera: Some((1920, 1080)),
        },
    );
    let rect = match slot {
        GraphicSlot::Camera => parts.camera.map(|c| c.rect),
        _ => parts.screen.map(|s| s.rect),
    };
    rect.map_or(whole, |r| r.pixels(width, height))
}

impl Default for ShortLayout {
    fn default() -> Self {
        Self {
            camera_clips: Default::default(),
            screen_zoom: 1.0,
            follow_zooms: true,
            screen_pan_x: 0.0,
            screen_pan_y: 0.0,
            // A short is watched on a phone, often muted: captions on unless turned off.
            captions: true,
            caption_spot: CaptionSpot::Seam,
            caption_max_words: 0,
            caption_lines: 1,
            caption_size_pct: 0.0,
            graphic_slots: Default::default(),
            push_ins: false,
        }
    }
}

impl ShortLayout {
    pub fn validate(&self) -> Result<(), String> {
        if !(MIN_SCREEN_ZOOM..=MAX_SCREEN_ZOOM).contains(&self.screen_zoom) {
            return Err(format!(
                "Screen zoom is {MIN_SCREEN_ZOOM}x to {MAX_SCREEN_ZOOM}x"
            ));
        }
        if !(-1.0..=1.0).contains(&self.screen_pan_x) || !(-1.0..=1.0).contains(&self.screen_pan_y)
        {
            return Err("The screen pans -1 to 1 each way".into());
        }
        validate_framing(&self.camera_clips)?;
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
        if self.graphic_slots.len() > MAX_GRAPHIC_SLOTS
            || self
                .graphic_slots
                .keys()
                .any(|id| id.is_empty() || id.len() > 64)
        {
            return Err("Too many graphic slots, or a bad clip id".into());
        }
        Ok(())
    }
}

/// Checks camera clips framed by hand.
pub fn validate_framing(
    clips: &std::collections::BTreeMap<String, ClipFraming>,
) -> Result<(), String> {
    if clips.len() > MAX_FRAMED_CLIPS {
        return Err("Too many camera clips framed by hand".into());
    }
    for (clip, framing) in clips {
        if clip.is_empty() || clip.len() > 128 {
            return Err("Unknown camera clip".into());
        }
        if !(-1.0..=1.0).contains(&framing.offset_x) || !(-1.0..=1.0).contains(&framing.offset_y) {
            return Err("The camera framing moves -1 to 1 each way".into());
        }
    }
    Ok(())
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

/// How much closer a pushed-in camera stretch plays (FX-3).
pub const PUSH_IN: f32 = 1.15;
/// The least a framing holds before a jump cut may change it: a cleaned take is cut every few
/// tenths of a second, and changing at each of those would strobe.
pub const PUSH_HOLD_US: u64 = 2_500_000;

/// With push-ins on, how much closer each camera clip plays, by clip id: at a jump cut (the
/// next camera clip starts where the last ended but plays another moment of the camera) the
/// framing goes in or back out, once it has held for [`PUSH_HOLD_US`]; a clip that carries
/// straight on keeps it, and anything between them (footage, a gap) starts it over at the
/// normal framing.
pub fn push_ins(document: &EditDocument) -> std::collections::HashMap<String, f32> {
    let mut found = std::collections::HashMap::new();
    if !document.short_layout.as_ref().is_some_and(|l| l.push_ins) {
        return found;
    }
    let mut cameras: Vec<&crate::sequence::Clip> = document
        .sequence
        .video_tracks()
        .flat_map(|track| {
            track.clips.iter().filter(move |clip| {
                crate::sequence::clip_role(&document.assets, track, clip)
                    == Some(crate::sequence::Role::Webcam)
            })
        })
        .collect();
    cameras.sort_by_key(|c| c.start_us);
    let mut last: Option<&crate::sequence::Clip> = None;
    let mut closer = false;
    let mut since = 0;
    for clip in cameras {
        match last {
            // Straight on from the last one: the same take carries on.
            Some(prev)
                if prev.end_us() == clip.start_us
                    && prev.asset == clip.asset
                    && prev.out_us() == clip.in_us => {}
            // A jump cut: in, or back out, once the framing has held long enough.
            Some(prev) if prev.end_us() == clip.start_us => {
                if clip.start_us - since >= PUSH_HOLD_US {
                    closer = !closer;
                    since = clip.start_us;
                }
            }
            _ => {
                closer = false;
                since = clip.start_us;
            }
        }
        if closer {
            found.insert(clip.id.clone(), PUSH_IN);
        }
        last = Some(clip);
    }
    found
}

/// How camera clip `clip` is framed under `style`, as `render::focus_uv` takes it: the face to
/// centre on and an offset. A clip framed by hand has no face and its own offset (flipped for
/// a mirrored camera, so it moves the picture the way it looks). Every other clip is lined up
/// once with the typical face over the part of the camera it plays, and holds for the whole
/// clip; face framing off, no face track, or no face found keeps the centre crop.
pub fn camera_framing(
    root: &std::path::Path,
    style: &crate::project::EditLayout,
    clip: &crate::sequence::Clip,
) -> (Option<crate::face::FocusPoint>, (f32, f32)) {
    if let Some(framing) = style.camera_clips.get(&clip.id) {
        let x = if style.webcam_mirror {
            -framing.offset_x
        } else {
            framing.offset_x
        };
        return (None, (x, framing.offset_y));
    }
    if !style.face_framing {
        return (None, (0.0, 0.0));
    }
    let focus = crate::face::cached_track(root, &clip.asset, &clip.stream)
        .and_then(|track| crate::face::clip_focus(&track, clip.in_us, clip.out_us()));
    (focus, (0.0, 0.0))
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
            // `max`: when the window is the crop's size, rounding can put the end a hair
            // before the start, and `clamp` panics on that.
            let start = (middle - window / 2.0).clamp(c0, (c0 + cl - window).max(c0));
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

/// Where captions sit when one picture fills a short's frame: their middle at this share of
/// the height, above the controls short-video apps draw over the bottom.
pub const FULL_FRAME_CAPTION_AT: f32 = 0.70;

/// The caption's top edge in a short's frame laid out as `parts`: at `spot` when camera and
/// screen are stacked, else at the full-frame spot.
pub fn caption_y(
    spot: CaptionSpot,
    parts: &crate::layouts::arrange::Arrangement,
    caption_h: u32,
    canvas_h: u32,
) -> f32 {
    let (h, canvas) = (caption_h as f32, canvas_h as f32);
    let margin = (canvas_h / 40) as f32;
    let bottom_of = |r: crate::layouts::arrange::Rect| r.bottom() - h - margin;
    let full = canvas * FULL_FRAME_CAPTION_AT - h / 2.0;
    let y = match (parts.seam, parts.camera, parts.screen) {
        (Some(seam), Some(camera), Some(screen)) => match spot {
            CaptionSpot::Seam => seam - h / 2.0,
            CaptionSpot::Camera => bottom_of(camera.rect),
            CaptionSpot::Screen => bottom_of(screen.rect),
        },
        _ => full,
    };
    y.clamp(0.0, (canvas - h).max(0.0))
}

/// Where captions sit while a graphic fills one part of a split (`filled`): out of that part,
/// at the bottom of the other, since the graphic is drawn over the part it fills and would
/// hide the half of a seam caption on its side.
pub fn caption_spot_beside(spot: CaptionSpot, filled: Option<GraphicSlot>) -> CaptionSpot {
    match (filled, spot) {
        (Some(GraphicSlot::Screen), CaptionSpot::Seam | CaptionSpot::Screen) => CaptionSpot::Camera,
        (Some(GraphicSlot::Camera), CaptionSpot::Seam | CaptionSpot::Camera) => CaptionSpot::Screen,
        _ => spot,
    }
}

/// One stretch of a source a clip plays, in the source's own time.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortPiece {
    pub source_start_us: u64,
    pub source_end_us: u64,
    /// Where it starts in the video now; absent when cut from it since. Filled in for the UI
    /// and cleared before storing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_start_us: Option<u64>,
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
    /// How its screen and camera are framed, its captions and background.
    #[serde(default)]
    pub layout: ShortLayout,
    /// Its base layout and sections, on its own timeline.
    #[serde(default = "crate::layouts::LayoutTrack::for_short")]
    pub layouts: crate::layouts::LayoutTrack,
    /// The asset whose clock the start and end are on; `None` is the first recording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// The short's own timeline, once it was edited on its own. Until then it is its stretch
    /// of the video and follows the video's edits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<ShortEdit>,
    /// A short made from a take recorded for it (see [`crate::take_short`]): its own timeline
    /// plays the take's camera and voice, with footage from the video under it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take: Option<ShortTake>,
    /// Its own caption breaks, joins, hidden words and emphasis, where they differ from the
    /// video's. Caption text stays shared with the video.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub caption_marks: CaptionMarks,
    /// What it is about: the premise or hook a clip gathered from anywhere serves.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub idea: String,
    /// A clip gathered from several stretches of the video (SHT-1): each on the `media`
    /// clock, in play order. Its own timeline joins them, as edited when it was made.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pieces: Vec<ShortPiece>,
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

/// The look and base layout a new short starts with: those of the first one of its kind made
/// so far (a short from a take, `take`, or a clip cut from the video), so they match, else its
/// preset's. Each gets its own layout sections, graphic places and camera framing.
pub fn new_look(document: &EditDocument, take: bool) -> (ShortLayout, crate::layouts::LayoutTrack) {
    let first = document.shorts.iter().find(|s| s.take.is_some() == take);
    let preset = if take {
        &document.shorts_preset
    } else {
        &document.clips_preset
    };
    let mut layout = first
        .map(|s| s.layout.clone())
        .unwrap_or_else(|| crate::preset::store::resolve(preset).short_layout());
    layout.graphic_slots.clear();
    layout.camera_clips.clear();
    let mut layouts = first
        .map(|s| s.layouts.clone())
        .unwrap_or_else(crate::layouts::LayoutTrack::for_short);
    layouts.sections.clear();
    (layout, layouts)
}

/// The project's shorts with the clips `found` in the transcript of `track_id` added after
/// them, each with the clips' look and an id of its own. A found clip that overlaps one already
/// made is left out; shorts made from a take are never touched.
pub fn with_found_clips(
    document: &EditDocument,
    track_id: &str,
    found: Vec<crate::ai::shorts::Found>,
) -> Result<Vec<Short>, String> {
    // The words' times are their recording's or file's own.
    let media = crate::sequence::StreamRef::parse(track_id)
        .map(|r| r.asset)
        .or_else(|| document.clock_asset(None).map(String::from))
        .ok_or("This transcript plays no recording")?;
    let mut shorts = document.shorts.clone();
    let mut added = 0;
    for pick in found {
        let Some((&(start, end), rest)) = pick.pieces.split_first() else {
            continue;
        };
        let overlaps = shorts.iter().any(|s| {
            s.take.is_none()
                && document.clock_asset(s.media.as_deref()) == Some(media.as_str())
                && stretches(s)
                    .iter()
                    .any(|&(a, b)| pick.pieces.iter().any(|&(c, d)| a < d && c < b))
        });
        if overlaps {
            continue;
        }
        let mut clip = new_clip(document, media.clone(), start, end, pick.title);
        clip.idea = pick.idea;
        clip.reason = pick.reason;
        // A gathered pick plays its pieces one after another, as the video has them now; one
        // a later edit cut is left out.
        if rest
            .iter()
            .any(|&(a, b)| add_piece(document, &mut clip, a, b).is_err())
        {
            continue;
        }
        clip.id = crate::ids::free_id(&format!("clip-{}", start / 1000), |id| {
            shorts.iter().any(|s| s.id == id)
        });
        shorts.push(clip);
        added += 1;
    }
    if added == 0 {
        return Err("The AI found no new moment: each one overlaps a clip already made".into());
    }
    Ok(shorts)
}

/// The source stretches a clip plays: its pieces, or its one stretch.
fn stretches(short: &Short) -> Vec<(u64, u64)> {
    if short.pieces.is_empty() {
        vec![(short.source_start_us, short.source_end_us)]
    } else {
        short
            .pieces
            .iter()
            .map(|p| (p.source_start_us, p.source_end_us))
            .collect()
    }
}

/// Caption marks by transcript (`<asset>.<stream>`), then word id. A mark all off still
/// counts: it turns off one the video's captions have.
pub type CaptionMarks = std::collections::BTreeMap<
    String,
    std::collections::BTreeMap<String, crate::transcript::CaptionMark>,
>;

/// Sets one of `short`'s own caption marks for word `word_id` of `transcript`, starting from
/// how the word sits in the short now. A mark the same as the video's is dropped, so the short
/// follows the video's there again.
pub fn set_caption_mark(
    short: &mut Short,
    transcript: &crate::transcript::Transcript,
    word_id: &str,
    change: impl FnOnce(&mut crate::transcript::CaptionMark),
) -> Result<(), String> {
    if !transcript.words.iter().any(|w| w.id == word_id) {
        return Err("Unknown word".into());
    }
    let shared = transcript.caption_mark(word_id);
    let marks = short
        .caption_marks
        .entry(transcript.track_id.clone())
        .or_default();
    let mut mark = marks.get(word_id).copied().unwrap_or(shared);
    change(&mut mark);
    if mark == shared {
        marks.remove(word_id);
    } else {
        marks.insert(word_id.to_string(), mark);
    }
    short.caption_marks.retain(|_, marks| !marks.is_empty());
    Ok(())
}

/// Puts `short`'s own caption marks on `transcript`, for views of the short's words.
pub fn overlay_caption_marks(short: &Short, transcript: &mut crate::transcript::Transcript) {
    for (word_id, mark) in short
        .caption_marks
        .get(&transcript.track_id)
        .into_iter()
        .flatten()
    {
        if *mark == Default::default() {
            transcript.caption_marks.remove(word_id);
        } else {
            transcript.caption_marks.insert(word_id.clone(), *mark);
        }
    }
}

/// The take a short was made from, as stream keys (`<asset>.<stream>`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShortTake {
    /// The camera's picture; none for a voice-only take.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    /// The voice: the camera file's own sound, a separate microphone's, or a recording's.
    pub voice: String,
}

/// A short's own timeline, in the short's time, and its own muted mouth noises.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShortEdit {
    pub sequence: Sequence,
    /// Muted moments by sound, in that sound's own time, as the project keeps them for the
    /// video; they play in this short only.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub noise_mutes: crate::project::revision::NoiseMutes,
}

/// Gives every short that has none a timeline of its own: its stretch of the video as edited
/// now, with the video's muted mouth noises. From then on its cuts and mutes are its own and
/// the video's later edits leave it alone. A short whose ends were cut stays without one.
pub fn give_own_timelines(document: &mut EditDocument) {
    let mut owned = Vec::new();
    for (i, short) in document.shorts.iter().enumerate() {
        if short.edit.is_some() {
            continue;
        }
        if let Ok(sequence) = video_sequence(short, document) {
            owned.push((
                i,
                ShortEdit {
                    sequence,
                    noise_mutes: document.noise_mutes.clone(),
                },
            ));
        }
    }
    for (i, edit) in owned {
        document.shorts[i].edit = Some(edit);
    }
}

/// The sequence's clips within timeline `[start, end)`, moved to start at 0 and cut to fit.
pub fn slice_sequence(
    sequence: &Sequence,
    assets: &[crate::sequence::Asset],
    start: u64,
    end: u64,
) -> Sequence {
    let still = |asset: &str| assets.iter().any(|a| a.id == asset && a.is_still());
    let mut out = sequence.clone();
    for track in &mut out.tracks {
        track.clips = std::mem::take(&mut track.clips)
            .into_iter()
            .filter_map(|clip| {
                let (a, b) = (clip.start_us.max(start), clip.end_us().min(end));
                if b <= a || b - a < crate::sequence::MIN_PIECE_US {
                    return None;
                }
                let skip = a - clip.start_us;
                Some(crate::sequence::Clip {
                    start_us: a - start,
                    in_us: if still(&clip.asset) {
                        0
                    } else {
                        clip.in_us + clip.source_offset(skip)
                    },
                    duration_us: b - a,
                    ..clip
                })
            })
            .collect();
    }
    out
}

/// Stretches of the edited timeline played one after another, every track: a clip gathered
/// from anywhere in the video.
pub fn join_slices(
    sequence: &Sequence,
    assets: &[crate::sequence::Asset],
    ranges: &[(u64, u64)],
) -> Sequence {
    let mut empty = sequence.clone();
    for track in &mut empty.tracks {
        track.clips.clear();
    }
    append_slices(empty, sequence, assets, ranges)
}

/// `onto` with stretches of `source` (the edited timeline) played after what it has, every
/// track by its id (a track the video gained since is added). A clip sliced again gets a new
/// id (and link), as the stretches no longer move together.
pub fn append_slices(
    mut onto: Sequence,
    source: &Sequence,
    assets: &[crate::sequence::Asset],
    ranges: &[(u64, u64)],
) -> Sequence {
    let mut ids: std::collections::HashSet<String> =
        onto.clips().map(|(_, c)| c.id.clone()).collect();
    let mut offset = onto.duration_us();
    for &(start, end) in ranges {
        let slice = slice_sequence(source, assets, start, end);
        for sliced in slice.tracks {
            let at = match onto.tracks.iter().position(|t| t.id == sliced.id) {
                Some(at) => at,
                None => {
                    // Video tracks stay before audio tracks.
                    let at = if sliced.kind == crate::sequence::TrackKind::Video {
                        onto.video_tracks().count()
                    } else {
                        onto.tracks.len()
                    };
                    let mut track = sliced.clone();
                    track.clips.clear();
                    onto.tracks.insert(at, track);
                    at
                }
            };
            for mut clip in sliced.clips {
                clip.start_us += offset;
                let mut n = 2;
                let (id, link) = (clip.id.clone(), clip.link.clone());
                while ids.contains(&clip.id) {
                    clip.id = format!("{id}-p{n}");
                    clip.link = link.as_ref().map(|l| format!("{l}-p{n}"));
                    n += 1;
                }
                ids.insert(clip.id.clone());
                onto.tracks[at].clips.push(clip);
            }
        }
        offset += end - start;
    }
    onto
}

/// A new clip of the source stretch `start..end` on `media`'s clock, in the look of the clips
/// made so far, with an id from where it starts.
pub fn new_clip(
    document: &EditDocument,
    media: String,
    start: u64,
    end: u64,
    title: String,
) -> Short {
    let (layout, layouts) = new_look(document, false);
    let id = crate::ids::free_id(&format!("clip-{}", start / 1000), |id| {
        document.shorts.iter().any(|s| s.id == id)
    });
    Short {
        id,
        title,
        source_start_us: start,
        source_end_us: end,
        reason: String::new(),
        layout,
        layouts,
        media: Some(media),
        edit: None,
        take: None,
        caption_marks: Default::default(),
        idea: String::new(),
        pieces: Vec::new(),
        length_us: None,
        edited_start_us: None,
        edited_end_us: None,
    }
}

/// Adds the source stretch `start..end` (on the clip's media clock) to the end of `clip`, as
/// the video has it now. What the clip plays so far stays as it is, its own cuts too; its first
/// stretch becomes its first piece. Refused when the stretch was cut from the video, overlaps
/// a piece it has, or makes it longer than a clip may be.
pub fn add_piece(
    document: &EditDocument,
    clip: &mut Short,
    start: u64,
    end: u64,
) -> Result<(), String> {
    if clip.take.is_some() {
        return Err("A short made from a take plays its take; add footage under it instead".into());
    }
    let asset = document
        .clock_asset(clip.media.as_deref())
        .ok_or("This clip plays no recording")?
        .to_string();
    let mapper = document.asset_clock(&asset);
    let range = edited_span(&mapper, start, end)
        .ok_or("A word at one end was cut from the video; pick words that are kept")?;
    let had = match &clip.edit {
        Some(own) => own.sequence.clone(),
        None => video_sequence(clip, document)?,
    };
    if clip.pieces.is_empty() {
        clip.pieces.push(ShortPiece {
            source_start_us: clip.source_start_us,
            source_end_us: clip.source_end_us,
            edited_start_us: None,
        });
    }
    if clip
        .pieces
        .iter()
        .any(|p| start < p.source_end_us && p.source_start_us < end)
    {
        return Err("Those words are already in this clip".into());
    }
    let sequence = append_slices(had, &document.sequence, &document.assets, &[range]);
    let length = sequence.duration_us();
    if length > MAX_SHORT_US {
        return Err(format!(
            "That makes it {:.1} s; a clip is at most 3 min",
            length as f64 / 1e6
        ));
    }
    clip.pieces.push(ShortPiece {
        source_start_us: start,
        source_end_us: end,
        edited_start_us: None,
    });
    clip.source_start_us = clip.source_start_us.min(start);
    clip.source_end_us = clip.source_end_us.max(end);
    let noise_mutes = clip.edit.as_ref().map_or_else(
        || document.noise_mutes.clone(),
        |own| own.noise_mutes.clone(),
    );
    clip.edit = Some(ShortEdit {
        sequence,
        noise_mutes,
    });
    Ok(())
}

/// Where each of a gathered clip's pieces plays on its own timeline, in order: from where its
/// first kept moment plays to where the next piece's does (the last to the end), so the clip's
/// own cuts, graphics and every track between stay with their piece. `None` for a piece the
/// clip's own cuts removed whole.
pub fn piece_spans(
    clip: &Short,
    document: &EditDocument,
) -> Result<Vec<Option<(u64, u64)>>, String> {
    let own = clip
        .edit
        .as_ref()
        .filter(|_| clip.pieces.len() > 1)
        .ok_or("This clip is one stretch: there are no pieces to arrange")?;
    let asset = document
        .clock_asset(clip.media.as_deref())
        .ok_or("This clip plays no recording")?;
    let mapper = crate::sequence::clock::asset_clock(&own.sequence, asset);
    let starts: Vec<Option<u64>> = clip
        .pieces
        .iter()
        .map(|p| {
            mapper
                .source_range_to_edited(p.source_start_us, p.source_end_us)
                .iter()
                .map(|r| r.0)
                .min()
        })
        .collect();
    let kept: Vec<u64> = starts.iter().flatten().copied().collect();
    if kept.windows(2).any(|w| w[1] <= w[0]) {
        return Err("This clip's pieces no longer play in their order; make it again".into());
    }
    let length = own.sequence.duration_us();
    let mut spans = Vec::with_capacity(starts.len());
    let mut first = true;
    for (i, start) in starts.iter().enumerate() {
        spans.push(start.map(|s| {
            let from = if std::mem::take(&mut first) { 0 } else { s };
            let to = starts[i + 1..]
                .iter()
                .flatten()
                .next()
                .copied()
                .unwrap_or(length);
            (from, to)
        }));
    }
    Ok(spans)
}

/// Plays a gathered clip's pieces in `order` (indices into its pieces; one left out is
/// removed), each with what its own timeline has for it (see [`piece_spans`]).
pub fn arrange_pieces(
    document: &EditDocument,
    clip: &mut Short,
    order: &[usize],
) -> Result<(), String> {
    let spans = piece_spans(clip, document)?;
    if order.is_empty() {
        return Err("A clip keeps at least one piece; remove the clip instead".into());
    }
    let mut seen = std::collections::HashSet::new();
    if order.iter().any(|&i| i >= spans.len() || !seen.insert(i)) {
        return Err(format!(
            "Give each piece once, by its number (this clip has {})",
            spans.len()
        ));
    }
    let own = clip
        .edit
        .as_ref()
        .ok_or("This clip has no timeline of its own")?;
    let ranges: Vec<(u64, u64)> = order.iter().filter_map(|&i| spans[i]).collect();
    if ranges.is_empty() {
        return Err("Those pieces play nothing: the clip's own cuts removed them".into());
    }
    let mut empty = own.sequence.clone();
    for track in &mut empty.tracks {
        track.clips.clear();
    }
    let sequence = append_slices(empty, &own.sequence, &document.assets, &ranges);
    if sequence.duration_us() < MIN_SHORT_US {
        return Err("That leaves less than 3 s; keep more of it".into());
    }
    clip.pieces = order.iter().map(|&i| clip.pieces[i]).collect();
    clip.source_start_us = clip
        .pieces
        .iter()
        .map(|p| p.source_start_us)
        .min()
        .unwrap_or(0);
    clip.source_end_us = clip
        .pieces
        .iter()
        .map(|p| p.source_end_us)
        .max()
        .unwrap_or(0);
    let noise_mutes = own.noise_mutes.clone();
    clip.edit = Some(ShortEdit {
        sequence,
        noise_mutes,
    });
    Ok(())
}

/// The short a sound is the take of, when it plays only there (not on the video's timeline):
/// its words and cuts are then on that short's timeline.
pub fn take_short_of<'a>(document: &'a EditDocument, key: &str) -> Option<&'a Short> {
    let on_video = document
        .sequence
        .clips()
        .any(|(_, c)| format!("{}.{}", c.asset, c.stream) == key);
    if on_video {
        return None;
    }
    document
        .shorts
        .iter()
        .find(|s| s.take.as_ref().is_some_and(|t| t.voice == key))
}

/// What a clip plays of the video as edited now, every track: its stretch, or a gathered
/// clip's pieces one after another in their order. A piece the video has since trimmed plays
/// what is kept of it, and one it cut whole is left out.
pub fn video_sequence(short: &Short, document: &EditDocument) -> Result<Sequence, String> {
    if short.pieces.len() < 2 {
        let (start, end) = edited_range_in(short, document)
            .ok_or("Part of this short was cut from the video; adjust it first")?;
        return Ok(slice_sequence(
            &document.sequence,
            &document.assets,
            start,
            end,
        ));
    }
    let asset = document
        .clock_asset(short.media.as_deref())
        .ok_or("This clip plays no recording")?;
    let mapper = document.asset_clock(asset);
    let ranges: Vec<(u64, u64)> = short
        .pieces
        .iter()
        .filter_map(|p| {
            let kept = mapper.source_range_to_edited(p.source_start_us, p.source_end_us);
            let start = kept.iter().map(|r| r.0).min()?;
            let end = kept.iter().map(|r| r.1).max()?;
            (end > start).then_some((start, end))
        })
        .collect();
    if ranges.is_empty() {
        return Err("Every piece of this clip was cut from the video".into());
    }
    Ok(join_slices(&document.sequence, &document.assets, &ranges))
}

/// The short as a timeline of its own, in the short's time: its own edit when it has one,
/// else its stretch of the video (every track).
pub fn short_timeline(base: &EditDocument, short: &Short) -> Result<EditDocument, String> {
    let mut timeline = base.clone();
    timeline.shorts.clear();
    timeline.chapters.clear();
    timeline.short_layout = None;
    timeline.layouts = short.layouts.clone();
    timeline.captions.own_marks = short.caption_marks.clone();
    // A take's captions read the take's own words.
    if let Some(take) = &short.take {
        timeline.captions.track_id = Some(take.voice.clone());
    }
    timeline.sequence = match &short.edit {
        Some(own) => {
            timeline.noise_mutes = own.noise_mutes.clone();
            own.sequence.clone()
        }
        None => video_sequence(short, base)?,
    };
    // A take's footage sound is lowered under its voice, by the project's amount unless it
    // has its own.
    if let Some((track, _, duck_db)) = crate::take_short::footage_sound_mix(base, short) {
        timeline.audio.tracks.entry(track).or_default().duck_db = Some(duck_db);
    }
    // A graphic split beside the face or over it sets the layout while it plays.
    let sections = graphic_sections(&timeline, short);
    if !sections.is_empty() {
        timeline.layouts.add(sections);
    }
    Ok(timeline)
}

/// Checks each short's own timeline the way the project's is checked.
pub fn validate_edits(base: &EditDocument) -> Result<(), String> {
    for own in base.shorts.iter().filter_map(|s| s.edit.as_ref()) {
        crate::sequence::validate_sequence(&own.sequence, &base.assets)?;
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
        if short.take.is_some() && short.edit.is_none() {
            return Err("A short made from a take keeps its own timeline".into());
        }
        if short.reason.chars().count() > 300 {
            return Err("A short's note is too long".into());
        }
        short.layout.validate()?;
        short.layouts.validate()?;
    }
    Ok(())
}

pub fn normalized(mut shorts: Vec<Short>) -> Vec<Short> {
    for short in &mut shorts {
        short.title = short.title.trim().to_string();
        short.layouts = std::mem::take(&mut short.layouts).normalized();
        short.edited_start_us = None;
        short.edited_end_us = None;
        short.length_us = None;
        for piece in &mut short.pieces {
            piece.edited_start_us = None;
        }
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
    let asset = document.clock_asset(short.media.as_deref())?;
    edited_range(short, &document.asset_clock(asset))
}

/// The edited range of a short whose ends are on `mapper`'s clock.
pub fn edited_range(short: &Short, mapper: &TimelineMapper) -> Option<(u64, u64)> {
    edited_span(mapper, short.source_start_us, short.source_end_us)
}

/// The edited range of the source stretch `start`..`end` on `mapper`'s clock, if both ends are
/// still on the timeline in order.
pub fn edited_span(mapper: &TimelineMapper, start: u64, end: u64) -> Option<(u64, u64)> {
    let first = mapper.source_to_edited_us(start)?;
    let last = mapper
        .source_to_edited_us(end.saturating_sub(1))?
        .saturating_add(1);
    (last > first).then_some((first, last))
}

/// Fills in where each short sits on the video and how long it plays. Lengths come from the
/// short's own timeline or its stretch of the video, without building a whole document per
/// short (the summary is refreshed after every edit).
pub fn attach_edited(shorts: &mut [Short], document: &EditDocument) {
    // One clock per source, not one per short: building one walks the whole sequence.
    let mut clocks: std::collections::HashMap<String, TimelineMapper> = Default::default();
    for short in shorts {
        // A short with its own edit no longer sits on the video's timeline.
        if let Some(own) = &short.edit {
            short.length_us = Some(own.sequence.duration_us());
            short.edited_start_us = None;
            short.edited_end_us = None;
            // A gathered clip's pieces: where each is in the video now.
            if let Some(asset) = document.clock_asset(short.media.as_deref()) {
                let clock = clocks
                    .entry(asset.to_string())
                    .or_insert_with(|| document.asset_clock(asset));
                for piece in &mut short.pieces {
                    piece.edited_start_us =
                        edited_span(clock, piece.source_start_us, piece.source_end_us).map(|r| r.0);
                }
            }
            continue;
        }
        let range = document
            .clock_asset(short.media.as_deref())
            .and_then(|asset| {
                let clock = clocks
                    .entry(asset.to_string())
                    .or_insert_with(|| document.asset_clock(asset));
                edited_range(short, clock)
            });
        short.length_us = range.map(|(start, end)| {
            slice_sequence(&document.sequence, &document.assets, start, end).duration_us()
        });
        short.edited_start_us = range.map(|r| r.0);
        short.edited_end_us = range.map(|r| r.1);
    }
}

/// The document a short exports from: the short's stretch of the edit on a 9:16 canvas, with
/// captions on when `captions` is set.
pub fn short_document(
    base: &EditDocument,
    short: &Short,
    captions: bool,
) -> Result<EditDocument, String> {
    let mut document = short_timeline(base, short)?;
    let length = document.duration_us();
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
    // Its camera clips are its own (the same ids can be the video's).
    document.layout.camera_clips = short.layout.camera_clips.clone();
    document.short_layout = Some(short.layout.clone());
    document.chapters.clear();
    document.shorts.clear();
    Ok(document)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const S: u64 = 1_000_000;

    /// Camera push-ins (FX-3): off unless asked; on, each jump cut goes in or back out, a take
    /// carrying straight on keeps its framing, and a gap starts over.
    #[test]
    fn push_ins_go_in_and_out_at_jump_cuts_only_when_turned_on() {
        let clip = |id: &str, at: u64, from: u64, length: u64| {
            serde_json::json!({ "id": id, "asset": "cam", "stream": "picture",
                "startUs": at * S, "inUs": from * S, "durationUs": length * S })
        };
        let mut document = EditDocument::default();
        document.assets = vec![serde_json::from_value(serde_json::json!({
            "id": "cam", "name": "cam.mp4", "kind": "video", "path": "C:/cam.mp4",
            "streams": [{ "id": "picture", "kind": "picture", "role": "webcam", "name": "Camera" }],
            "durationUs": 60 * S, "width": 1920, "height": 1080
        }))
        .unwrap()];
        document.sequence.tracks = vec![serde_json::from_value(serde_json::json!({
            "id": "V1", "kind": "video", "name": "V1",
            "clips": [
                clip("a", 0, 0, 3),   // the take
                clip("b", 3, 3, 3),   // carrying straight on
                clip("c", 6, 10, 3),  // a jump cut: in
                clip("d", 9, 20, 3),  // another: back out
                clip("e", 13, 30, 1), // after a gap: starts over
                clip("f", 14, 40, 1), // a jump cut too soon after: holds
                clip("g", 15, 50, 1), // still too soon
                clip("h", 16, 60, 2)  // held 3 s: in
            ]
        }))
        .unwrap()];
        document.short_layout = Some(ShortLayout::default());
        assert!(push_ins(&document).is_empty(), "off by default");
        document.short_layout.as_mut().unwrap().push_ins = true;
        let pushed = push_ins(&document);
        let mut ids: Vec<&str> = pushed.keys().map(String::as_str).collect();
        ids.sort();
        assert_eq!(ids, ["c", "h"]);
        assert_eq!(pushed["c"], PUSH_IN);
    }

    fn doc(len: u64) -> EditDocument {
        EditDocument::from_recording(crate::sequence::tests::recording("rec", len, &[])).unwrap()
    }

    fn run(document: &mut EditDocument, edit: crate::sequence::edit::SequenceEdit) {
        document.sequence =
            crate::sequence::edit::apply(&document.sequence, &document.assets, &edit)
                .unwrap()
                .0;
    }

    fn cut(document: &mut EditDocument, start_us: u64, end_us: u64) {
        run(
            document,
            crate::sequence::edit::SequenceEdit::DeleteRange {
                ranges: vec![crate::zoom::EditedRange { start_us, end_us }],
                ripple: Some(true),
            },
        );
    }

    pub(crate) fn short(start: u64, end: u64) -> Short {
        Short {
            id: "s1".into(),
            title: "Best bit".into(),
            source_start_us: start,
            source_end_us: end,
            reason: String::new(),
            layout: ShortLayout::default(),
            layouts: crate::layouts::LayoutTrack::for_short(),
            media: None,
            edit: None,
            take: None,
            caption_marks: Default::default(),
            idea: String::new(),
            pieces: Vec::new(),
            length_us: None,
            edited_start_us: None,
            edited_end_us: None,
        }
    }

    #[test]
    fn found_clips_are_added_and_never_replace_shorts_or_clips() {
        let mut document = doc(60 * S);
        let mut take = short(0, 30 * S);
        take.id = "take".into();
        take.take = Some(ShortTake {
            camera: None,
            voice: "cam.mic".into(),
        });
        let mut made = short(10 * S, 20 * S);
        made.id = "clip-a".into();
        made.media = Some("rec".into());
        made.layout.caption_max_words = 2;
        document.shorts = vec![take, made];
        let found = |pieces: Vec<(u64, u64)>| crate::ai::shorts::Found {
            title: "Found".into(),
            idea: "An idea".into(),
            reason: String::new(),
            pieces,
        };
        // One overlaps the clip made (left out); the other is new, with an id of its own and
        // the clips' look. The take short stays though it covers the same time.
        let shorts = with_found_clips(
            &document,
            "rec.mic",
            vec![found(vec![(15 * S, 25 * S)]), found(vec![(30 * S, 45 * S)])],
        )
        .unwrap();
        let ids: Vec<&str> = shorts.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["take", "clip-a", "clip-30000"]);
        assert_eq!(shorts[2].media.as_deref(), Some("rec"));
        assert_eq!(shorts[2].layout.caption_max_words, 2);
        assert_eq!(shorts[2].idea, "An idea");
        assert!(shorts[2].edit.is_none());
        // Nothing new: refused, so the list is never emptied or replaced.
        assert!(
            with_found_clips(&document, "rec.mic", vec![found(vec![(12 * S, 18 * S)])]).is_err()
        );
        // A gathered pick plays its pieces in its order on its own timeline; one that takes a
        // piece of a clip made is left out, though its other piece is free.
        let shorts = with_found_clips(
            &document,
            "rec.mic",
            vec![
                found(vec![(40 * S, 50 * S), (25 * S, 30 * S)]),
                found(vec![(52 * S, 58 * S), (18 * S, 22 * S)]),
            ],
        )
        .unwrap();
        assert_eq!(shorts.len(), 3);
        let gathered = &shorts[2];
        assert_eq!(gathered.pieces.len(), 2);
        assert_eq!(
            (gathered.source_start_us, gathered.source_end_us),
            (25 * S, 50 * S)
        );
        assert_eq!(
            gathered.edit.as_ref().map(|e| e.sequence.duration_us()),
            Some(15 * S)
        );
    }

    #[test]
    fn layout_sections_follow_the_short_through_cuts() {
        use crate::layouts::{CameraOnly, Layout, LayoutSection, ScreenOnly};
        // A short of 10-40 s of the recording; the video then loses 15-20 s.
        let mut document = doc(60 * S);
        let mut s = short(10 * S, 40 * S);
        cut(&mut document, 15 * S, 20 * S);
        let add = |s: &mut Short, start: u64, end: u64, layout: Layout| {
            let timeline = short_timeline(&document, s).unwrap();
            let template = LayoutSection::new(s.layouts.new_id(), None, 0, 1, Some(layout));
            let made = crate::layouts::sections_on(&timeline, start, end, &template).unwrap();
            s.layouts.add(made);
        };
        // 2-8 s of the short spans the cut: 12-15 s and 20-23 s of the recording, one section.
        add(
            &mut s,
            2 * S,
            8 * S,
            Layout::CameraOnly(CameraOnly::default()),
        );
        let spans: Vec<(u64, u64)> = s
            .layouts
            .sections
            .iter()
            .map(|sc| (sc.source_start_us, sc.source_end_us))
            .collect();
        assert_eq!(spans, vec![(12 * S, 23 * S)]);
        // A screen section over 6-10 s takes its time from the camera one.
        add(
            &mut s,
            6 * S,
            10 * S,
            Layout::ScreenOnly(ScreenOnly::default()),
        );
        validate(std::slice::from_ref(&s)).unwrap();
        let timeline = short_timeline(&document, &s).unwrap();
        let shown: Vec<(u64, u64, bool)> = crate::layouts::resolve::of_document(&timeline)
            .pieces
            .iter()
            .map(|p| (p.start_us, p.end_us, p.layout.camera_fills()))
            .collect();
        assert_eq!(shown, vec![(2 * S, 6 * S, true), (6 * S, 10 * S, false)]);
    }

    #[test]
    fn shorts_slice_the_current_edit() {
        // Recording 0-10 s and 20-40 s, with a 4 s file placed between on V1.
        let mut base = doc(40 * S);
        cut(&mut base, 10 * S, 20 * S);
        base.assets.push(crate::sequence::tests::video("m1", 4 * S));
        let v1 = base.sequence.tracks[0].id.clone();
        run(
            &mut base,
            crate::sequence::edit::SequenceEdit::PlaceAsset {
                asset_id: "m1".into(),
                at_us: 10 * S,
                track_id: Some(v1),
                streams: vec![],
                range: None,
            },
        );
        // From 6 s into the recording to 25 s: crosses the file's clip.
        let mut s = short(6 * S, 25 * S);
        // Captions are off in a new short; this one has them on.
        s.layout.captions = true;
        assert_eq!(edited_range_in(&s, &base), Some((6 * S, 19 * S)));
        let doc = short_document(&base, &s, true).unwrap();
        let v1: Vec<_> = doc.sequence.tracks[0]
            .clips
            .iter()
            .map(|c| (c.asset.as_str(), c.start_us, c.in_us, c.duration_us))
            .collect();
        assert_eq!(
            v1,
            vec![
                ("rec", 0, 6 * S, 4 * S),
                ("m1", 4 * S, 0, 4 * S),
                ("rec", 8 * S, 20 * S, 5 * S)
            ]
        );
        assert_eq!(doc.layout.aspect_ratio, "9:16");
        assert!(doc.captions.enabled);
        assert_eq!(doc.duration_us(), 13 * S);

        // An end that was cut makes the short unusable until it is adjusted.
        let mut gone = super::tests::doc(40 * S);
        cut(&mut gone, 10 * S, 30 * S);
        assert!(edited_range_in(&s, &gone).is_none());
        assert!(short_document(&gone, &s, false).is_err());
        assert!(
            short_document(&base, &short(0, S), false).is_err(),
            "too short"
        );
    }

    #[test]
    fn validation_and_ui_fields() {
        let mut shorts = vec![short(5 * S, 30 * S)];
        validate(&shorts).unwrap();
        let document = doc(60 * S);
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
    fn graphic_slots_captions_and_screen_windows() {
        use crate::layouts::arrange::{arrange, Canvas, Sources};
        use crate::layouts::{Layout, SideBySide};
        let layout = ShortLayout::default();
        // The camera below the screen, half and half.
        let bottom = Layout::SideBySide(SideBySide {
            camera_first: false,
            camera_pct: 50.0,
            ..SideBySide::default()
        });
        // Graphics: full frame unless given a part of the base layout.
        let mut slotted = layout.clone();
        slotted
            .graphic_slots
            .insert("g-cam".into(), GraphicSlot::Camera);
        slotted
            .graphic_slots
            .insert("g-over".into(), GraphicSlot::OverScreen);
        let mut document = doc(10 * S);
        document.layouts.base = bottom.clone();
        document.short_layout = Some(slotted.clone());
        assert_eq!(
            graphic_rect(&document, "g-other", 1080, 1920),
            (0, 0, 1080, 1920)
        );
        assert_eq!(
            graphic_rect(&document, "g-cam", 1080, 1920),
            (0, 960, 1080, 960)
        );
        assert_eq!(
            graphic_rect(&document, "g-over", 1080, 1920),
            (0, 0, 1080, 960)
        );
        // The box a split graphic is told follows the short's split as it is set now.
        let mut moved = short(0, 10 * S);
        moved.layouts.base = bottom.clone();
        assert_eq!(
            graphic_box(&document, &moved, GraphicSlot::Screen),
            (1080, 960)
        );
        assert_eq!(
            graphic_box(&document, &moved, GraphicSlot::OverFace),
            (1080, 1920)
        );
        if let Layout::SideBySide(split) = &mut moved.layouts.base {
            split.camera_pct = 30.0;
        }
        assert_eq!(
            graphic_box(&document, &moved, GraphicSlot::Screen),
            (1080, 1344)
        );
        assert_eq!(
            describe_box((1080, 864)),
            "a 1080x864 box (landscape, portrait rules)"
        );
        assert_eq!(
            describe_box((1080, 1040)),
            "a 1080x1040 box (about square, portrait rules)"
        );
        assert_eq!(describe_box((1080, 384)), "a 1080x384 box (landscape)");
        let json = serde_json::to_value(&slotted).unwrap();
        assert_eq!(json["graphicSlots"]["g-over"], "overScreen");
        assert!(serde_json::to_value(&layout)
            .unwrap()
            .get("graphicSlots")
            .is_none());
        assert_eq!(
            serde_json::from_value::<ShortLayout>(json).unwrap(),
            slotted
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

        // Captions: on the seam, in the screen part, or in the camera part; at the full-frame
        // spot when one picture fills the frame.
        let style = crate::project::EditLayout::plain();
        let both = Sources {
            screen: Some((1920, 1080)),
            camera: Some((1920, 1080)),
        };
        let canvas = Canvas::new(1080, 1920);
        let top = arrange(
            &crate::layouts::LayoutTrack::for_short().base,
            &style,
            canvas,
            both,
        );
        assert_eq!(caption_y(CaptionSpot::Seam, &top, 100, 1920), 622.0);
        assert_eq!(
            caption_y(CaptionSpot::Screen, &top, 100, 1920),
            (1920 - 100 - 48) as f32
        );
        assert_eq!(
            caption_y(CaptionSpot::Camera, &top, 100, 1920),
            (672 - 100 - 48) as f32
        );
        // A graphic filling the screen's part takes seam captions up into the camera's.
        let beside = caption_spot_beside(CaptionSpot::Seam, Some(GraphicSlot::Screen));
        assert_eq!(beside, CaptionSpot::Camera);
        assert!(caption_y(beside, &top, 100, 1920) + 100.0 <= 672.0);
        assert_eq!(
            caption_spot_beside(CaptionSpot::Seam, Some(GraphicSlot::Camera)),
            CaptionSpot::Screen
        );
        assert_eq!(
            caption_spot_beside(CaptionSpot::Seam, Some(GraphicSlot::OverScreen)),
            CaptionSpot::Seam
        );
        let under = arrange(&bottom, &style, canvas, both);
        assert_eq!(caption_y(CaptionSpot::Seam, &under, 100, 1920), 910.0);
        let alone = arrange(
            &Layout::CameraOnly(Default::default()),
            &style,
            canvas,
            both,
        );
        assert_eq!(
            caption_y(CaptionSpot::Seam, &alone, 100, 1920),
            1920.0 * FULL_FRAME_CAPTION_AT - 50.0
        );

        let mut by_hand = ShortLayout::default();
        by_hand.camera_clips.insert(
            "c1".into(),
            ClipFraming {
                offset_x: -1.5,
                offset_y: 0.0,
            },
        );
        assert!(by_hand.validate().is_err());
        by_hand.camera_clips.get_mut("c1").unwrap().offset_x = -0.5;
        by_hand.validate().unwrap();
        assert!(ShortLayout {
            screen_zoom: 4.0,
            ..ShortLayout::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn a_short_on_an_imported_file_follows_its_clip_and_takes_its_caption_choices() {
        // Recording 0-10 s, then 20 s of the file from 5 s into it.
        let mut base = doc(10 * S);
        base.assets
            .push(crate::sequence::tests::video("m1", 30 * S));
        let v1 = base.sequence.tracks[0].id.clone();
        run(
            &mut base,
            crate::sequence::edit::SequenceEdit::PlaceAsset {
                asset_id: "m1".into(),
                at_us: 10 * S,
                track_id: Some(v1),
                streams: vec![],
                range: Some(crate::sequence::SourceRange {
                    start_us: 5 * S,
                    end_us: 25 * S,
                }),
            },
        );
        let mut on_file = short(10 * S, 20 * S);
        on_file.media = Some("m1".into());
        assert_eq!(edited_range_in(&on_file, &base), Some((15 * S, 25 * S)));
        on_file.layout.caption_max_words = 3;
        on_file.layout.caption_lines = 1;
        on_file.layout.screen_pan_x = -0.5;
        validate(std::slice::from_ref(&on_file)).unwrap();
        let document = short_document(&base, &on_file, true).unwrap();
        assert_eq!(document.duration_us(), 10 * S);
        assert_eq!(
            (document.captions.max_words, document.captions.max_lines),
            (3, 1)
        );
        on_file.layout.screen_pan_x = 2.0;
        assert!(validate(&[on_file]).is_err());
    }

    /// Edited on its own, a short keeps its timeline whatever happens to the video, and
    /// following the video again drops it.
    #[test]
    fn a_short_edited_on_its_own_leaves_the_video_alone() {
        use crate::project::revision::EditHistory;
        let dir = tempfile::tempdir().unwrap();
        let mut history = EditHistory::new(doc(60 * S));
        history
            .set_shorts(0, vec![short(10 * S, 30 * S)], dir.path())
            .unwrap();
        // Its own timeline from the start: its stretch of the video.
        let made = history.current.shorts[0].edit.clone().unwrap();
        assert_eq!(made.sequence.duration_us(), 20 * S);
        history
            .edit_short_sequence(
                1,
                "s1",
                &crate::sequence::edit::SequenceEdit::DeleteRange {
                    ranges: vec![crate::zoom::EditedRange {
                        start_us: 0,
                        end_us: 2 * S,
                    }],
                    ripple: None,
                },
                dir.path(),
            )
            .unwrap();
        assert_eq!(history.current.duration_us(), 60 * S);
        let own = history.current.shorts[0].edit.clone().unwrap();
        assert_eq!(own.sequence.tracks[0].clips[0].in_us, 12 * S);
        assert_eq!(own.sequence.duration_us(), 18 * S);
        history.ripple_cuts(2, &[(0, 20 * S)], dir.path()).unwrap();
        let after = short_timeline(&history.current, &history.current.shorts[0]).unwrap();
        assert_eq!(after.duration_us(), 18 * S);
        // Taking the video's cuts again: its first 20 s were cut, so the short's start is gone
        // and it keeps its own timeline.
        assert!(history.resync_short(3, "s1", dir.path()).is_err());
        assert_eq!(after.duration_us(), 18 * S);
        // A cut that leaves its ends: it takes the video's timeline as it is now.
        history.undo(3, dir.path()).unwrap();
        history
            .ripple_cuts(4, &[(15 * S, 17 * S)], dir.path())
            .unwrap();
        history.resync_short(5, "s1", dir.path()).unwrap();
        let synced = short_timeline(&history.current, &history.current.shorts[0]).unwrap();
        assert_eq!(synced.duration_us(), 18 * S);
    }

    #[test]
    fn a_shorts_muted_noises_are_its_own() {
        use crate::project::revision::EditHistory;
        use crate::sequence::SourceRange;
        let dir = tempfile::tempdir().unwrap();
        let mut history = EditHistory::new(doc(60 * S));
        let key = history.current.sequence.tracks[0].clips[0].clone();
        let sound = history
            .current
            .assets
            .iter()
            .find(|a| a.id == key.asset)
            .unwrap()
            .streams
            .iter()
            .find(|s| s.kind == crate::sequence::StreamKind::Sound)
            .map(|s| format!("{}.{}", key.asset, s.id))
            .expect("the recording has a sound");
        history
            .set_shorts(0, vec![short(10 * S, 30 * S)], dir.path())
            .unwrap();
        let moment = [SourceRange {
            start_us: 12 * S,
            end_us: 12 * S + 100_000,
        }];
        history
            .update_noise_mutes(1, &sound, &moment, false, Some("s1"), dir.path())
            .unwrap();
        assert!(history.current.noise_mutes.is_empty());
        let timeline = short_timeline(&history.current, &history.current.shorts[0]).unwrap();
        assert_eq!(timeline.noise_mutes.get(&sound).map(Vec::len), Some(1));
        // The video's mutes stay out of a short made before them.
        history
            .update_noise_mutes(2, &sound, &moment, false, None, dir.path())
            .unwrap();
        history
            .update_noise_mutes(3, &sound, &moment, true, Some("s1"), dir.path())
            .unwrap();
        let timeline = short_timeline(&history.current, &history.current.shorts[0]).unwrap();
        assert!(timeline.noise_mutes.is_empty());
        assert_eq!(
            history.current.noise_mutes.get(&sound).map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn summaries_give_each_short_its_timelines_length() {
        let mut base = doc(60 * S);
        cut(&mut base, 20 * S, 25 * S);
        let mut own = short(30 * S, 50 * S);
        own.id = "own".into();
        own.edit = Some(ShortEdit {
            sequence: slice_sequence(&base.sequence, &base.assets, 0, 7 * S),
            ..Default::default()
        });
        let mut gone = short(21 * S, 40 * S);
        gone.id = "gone".into();
        let mut shorts = vec![short(10 * S, 30 * S), own, gone];
        base.shorts = shorts.clone();
        attach_edited(&mut shorts, &base);
        // Each length is what its timeline plays, as a full short document has it.
        for summary in &shorts {
            let expected = short_timeline(&base, summary).ok().map(|t| t.duration_us());
            assert_eq!(summary.length_us, expected, "{}", summary.id);
        }
        assert_eq!(shorts[0].length_us, Some(15 * S));
        assert_eq!(shorts[0].edited_start_us, Some(10 * S));
        assert_eq!(shorts[1].length_us, Some(7 * S));
        assert_eq!(shorts[1].edited_start_us, None, "edited on its own");
        assert_eq!(shorts[2].length_us, None, "its start was cut");
    }

    #[test]
    fn gathered_pieces_play_one_after_another_in_their_order() {
        let base = doc(60 * S);
        // A later stretch first, then an earlier one, both from the one recording's clips.
        let joined = join_slices(
            &base.sequence,
            &base.assets,
            &[(40 * S, 45 * S), (10 * S, 13 * S)],
        );
        assert_eq!(joined.duration_us(), 8 * S);
        crate::sequence::validate_sequence(&joined, &base.assets).unwrap();
        for track in &joined.tracks {
            let [first, second] = &track.clips[..] else {
                panic!("two pieces on every track");
            };
            assert_eq!((first.start_us, first.duration_us), (0, 5 * S));
            assert_eq!((second.start_us, second.duration_us), (5 * S, 3 * S));
            // Each plays its own stretch of the source.
            assert_eq!(second.in_us + 3 * S, first.in_us - 27 * S);
            assert_ne!(first.id, second.id);
        }
    }

    #[test]
    fn a_piece_added_to_a_clip_plays_after_it_and_keeps_its_own_cuts() {
        let base = doc(60 * S);
        let mut clip = short(10 * S, 20 * S);
        // Its own timeline with a second cut out of it.
        let mut own = slice_sequence(&base.sequence, &base.assets, 10 * S, 20 * S);
        for track in &mut own.tracks {
            for c in &mut track.clips {
                c.duration_us -= S;
            }
        }
        clip.edit = Some(ShortEdit {
            sequence: own,
            ..Default::default()
        });
        add_piece(&base, &mut clip, 40 * S, 44 * S).unwrap();
        let own = &clip.edit.as_ref().unwrap().sequence;
        assert_eq!(
            own.duration_us(),
            9 * S + 4 * S,
            "its cut stays, the piece follows"
        );
        assert_eq!(clip.pieces.len(), 2);
        assert_eq!((clip.source_start_us, clip.source_end_us), (10 * S, 44 * S));
        crate::sequence::validate_sequence(own, &base.assets).unwrap();
        // The same words again, or words inside it, are refused.
        assert!(add_piece(&base, &mut clip, 41 * S, 42 * S).is_err());
        assert!(add_piece(&base, &mut clip, 12 * S, 14 * S).is_err());
    }

    #[test]
    fn pieces_move_and_go_with_what_their_stretch_holds() {
        let base = doc(60 * S);
        let mut clip = short(10 * S, 20 * S);
        add_piece(&base, &mut clip, 40 * S, 44 * S).unwrap();
        add_piece(&base, &mut clip, 30 * S, 33 * S).unwrap();
        let spans = piece_spans(&clip, &base).unwrap();
        assert_eq!(
            spans,
            vec![
                Some((0, 10 * S)),
                Some((10 * S, 14 * S)),
                Some((14 * S, 17 * S))
            ]
        );
        // The third first, the second left out.
        arrange_pieces(&base, &mut clip, &[2, 0]).unwrap();
        let own = &clip.edit.as_ref().unwrap().sequence;
        assert_eq!(own.duration_us(), 13 * S);
        crate::sequence::validate_sequence(own, &base.assets).unwrap();
        let first = &own.tracks[0].clips[0];
        assert_eq!((first.start_us, first.duration_us), (0, 3 * S));
        assert_eq!(
            clip.pieces
                .iter()
                .map(|p| p.source_start_us)
                .collect::<Vec<_>>(),
            vec![30 * S, 10 * S]
        );
        assert!(arrange_pieces(&base, &mut clip, &[0, 0]).is_err());
        assert!(arrange_pieces(&base, &mut clip, &[]).is_err());
    }

    #[test]
    fn a_gathered_clip_takes_the_videos_cuts_again_piece_by_piece() {
        use crate::project::revision::EditHistory;
        let dir = tempfile::tempdir().unwrap();
        let mut history = EditHistory::new(doc(60 * S));
        let mut clip = short(40 * S, 50 * S);
        clip.media = Some("rec".into());
        add_piece(&history.current, &mut clip, 10 * S, 20 * S).unwrap();
        history.set_shorts(0, vec![clip], dir.path()).unwrap();
        // The video then loses 12-14 s (inside the second piece) and 42-44 s (inside the
        // first); the clip's own timeline keeps them until it takes the video's cuts again.
        history
            .ripple_cuts(1, &[(12 * S, 14 * S), (42 * S, 44 * S)], dir.path())
            .unwrap();
        let before = short_timeline(&history.current, &history.current.shorts[0]).unwrap();
        assert_eq!(before.duration_us(), 20 * S);
        history.resync_short(2, "s1", dir.path()).unwrap();
        let clip = &history.current.shorts[0];
        assert_eq!(clip.pieces.len(), 2);
        let after = short_timeline(&history.current, clip).unwrap();
        // Each piece as the video has it now, still the first one first.
        assert_eq!(after.duration_us(), 16 * S);
        let first = &after.sequence.tracks[0].clips[0];
        assert_eq!((first.in_us, first.duration_us), (40 * S, 2 * S));
        // A piece cut whole is left out; the rest still plays.
        history
            .ripple_cuts(3, &[(10 * S, 18 * S)], dir.path())
            .unwrap();
        history.resync_short(4, "s1", dir.path()).unwrap();
        let clip = &history.current.shorts[0];
        assert_eq!(
            short_timeline(&history.current, clip)
                .unwrap()
                .duration_us(),
            8 * S
        );
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
    fn screen_placement_survives_a_window_rounded_past_the_crop() {
        // Small frames (a storyboard's tiles) fill a region exactly, where rounding can make
        // the window a hair wider than the crop.
        let crop = (0.1, 0.0, 0.8, 1.0);
        for height in 100..400 {
            for width in [176, 177, 320, 1080] {
                let (_, (uv_x, _, uv_w, _)) = screen_placement(
                    1920,
                    1080,
                    crop,
                    (0, 0, width, height),
                    1.0,
                    (0.5, 0.5),
                    (0.0, 0.0),
                );
                assert!(uv_x >= 0.1 - 1e-4 && uv_w <= 0.8 + 1e-4);
            }
        }
    }
}
