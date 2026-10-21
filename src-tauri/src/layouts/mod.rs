//! Layouts: how the screen and the camera share the frame, and the sections of a timeline that
//! change it. Every timeline (the video, each short) has a base layout and sections over it.
//!
//! A section is anchored in an asset's own time, like chapters and shorts, so it stays on its
//! moment through cuts, trims and reorders. Where no section shows, the base does: there is no
//! built-in default to flash. Sections are independent records; what plays is worked out from
//! them on every read ([`resolve`]) and never written back.
//!
//! The look shared by every layout (background, corners, borders, mirror) is the timeline's
//! style, [`crate::project::EditLayout`]. A layout only places the screen and the camera
//! ([`arrange`]), the same layout working on a 16:9 and a 9:16 canvas.
pub mod arrange;
pub mod favourites;
pub mod pick;
pub mod resolve;

use crate::timeline::TimelineMapper;
use crate::zoom::EditedRange;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use resolve::{resolve, Resolved};

pub const MAX_SECTIONS: usize = 1_000;
/// A stretch shorter than this would only flicker: a section is at least this long when made,
/// and a piece that cuts leave shorter is not shown.
pub const MIN_SHOWN_US: u64 = 1_000_000;
/// The base showing for less than this between two sections would flicker: the earlier one
/// runs on to the next.
pub const MIN_BASE_GAP_US: u64 = 1_000_000;
pub const DEFAULT_TRANSITION_MS: u32 = 350;
pub const MAX_TRANSITION_MS: u32 = 2_000;

pub const MIN_BUBBLE_PCT: f32 = 5.0;
pub const MAX_BUBBLE_PCT: f32 = 60.0;
/// The bubble's Small, Medium and Large, as a percent of the canvas short side.
pub const BUBBLE_SIZES: [(&str, f32); 3] = [("small", 20.0), ("medium", 35.0), ("large", 50.0)];
pub const MAX_ROUNDNESS_PCT: f32 = 50.0;
pub const MIN_SIDE_PCT: f32 = 20.0;
pub const MAX_SIDE_PCT: f32 = 80.0;
pub const MIN_CAMERA_ONLY_PCT: f32 = 40.0;
pub const MAX_PUNCH_IN: f32 = 2.0;

/// Where a bubble sits: a spot on a 3x3 grid, or anywhere (`custom`).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BubbleSpot {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    #[default]
    BottomLeft,
    Bottom,
    BottomRight,
    Custom,
}

impl BubbleSpot {
    /// Column and row on the grid, 0 to 2; `None` for a custom spot.
    pub fn cell(self) -> Option<(u8, u8)> {
        use BubbleSpot::*;
        Some(match self {
            TopLeft => (0, 0),
            Top => (1, 0),
            TopRight => (2, 0),
            Left => (0, 1),
            Center => (1, 1),
            Right => (2, 1),
            BottomLeft => (0, 2),
            Bottom => (1, 2),
            BottomRight => (2, 2),
            Custom => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CameraShape {
    /// A rectangle in the camera's own shape, its corners rounded by `roundnessPct`.
    Rect,
    Circle,
    #[default]
    Squircle,
}

/// How the screen takes its part of the frame.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ScreenFit {
    /// Fit on a landscape canvas, fill on a portrait one.
    #[default]
    Auto,
    /// The whole screen, letterboxed in its part with the background around it.
    Fit,
    /// Its part filled, cut to that shape and following the zooms.
    Fill,
}

impl ScreenFit {
    pub fn fills(self, canvas_w: u32, canvas_h: u32) -> bool {
        match self {
            ScreenFit::Auto => canvas_h > canvas_w,
            ScreenFit::Fit => false,
            ScreenFit::Fill => true,
        }
    }
}

/// The screen in its frame with the camera in a bubble over it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Bubble {
    pub spot: BubbleSpot,
    /// With `spot: custom`: where the bubble sits, 0 to 100 across the room it can move in.
    pub custom_x: f32,
    pub custom_y: f32,
    /// The bubble's long side, as a percent of the canvas short side (5 to 60).
    pub size_pct: f32,
    pub shape: CameraShape,
    /// Corner radius of a `rect` bubble, as a percent of its short side (0 to 50).
    pub roundness_pct: f32,
    pub screen: ScreenFit,
}

impl Default for Bubble {
    fn default() -> Self {
        Self {
            spot: BubbleSpot::BottomLeft,
            custom_x: 80.0,
            custom_y: 80.0,
            size_pct: MAX_BUBBLE_PCT,
            shape: CameraShape::Squircle,
            roundness_pct: 0.0,
            screen: ScreenFit::Auto,
        }
    }
}

/// Camera and screen side by side: left and right on a landscape canvas, top and bottom on a
/// portrait one.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct SideBySide {
    /// The camera on the left (or on top); false puts it right (or below).
    pub camera_first: bool,
    /// The camera's share of the width (or height), 20 to 80 %.
    pub camera_pct: f32,
    /// The camera as a card overlapping the screen's edge, rather than an even split.
    pub overlap: bool,
    pub screen: ScreenFit,
}

impl Default for SideBySide {
    fn default() -> Self {
        Self {
            camera_first: true,
            camera_pct: 40.0,
            overlap: false,
            screen: ScreenFit::Auto,
        }
    }
}

/// Only the camera.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct CameraOnly {
    /// 100 fills the frame edge to edge; below that (down to 40) it is a centred rectangle
    /// over the background.
    pub size_pct: f32,
    /// Closer on the face: 1 (none) to 2.
    pub punch_in: f32,
}

impl Default for CameraOnly {
    fn default() -> Self {
        Self {
            size_pct: 100.0,
            punch_in: 1.0,
        }
    }
}

/// Only the screen.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenOnly {
    pub screen: ScreenFit,
}

/// How the screen and the camera share the frame.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Layout {
    Bubble(Bubble),
    SideBySide(SideBySide),
    CameraOnly(CameraOnly),
    ScreenOnly(ScreenOnly),
}

impl Default for Layout {
    fn default() -> Self {
        Layout::Bubble(Bubble::default())
    }
}

impl Layout {
    pub fn validate(&self) -> Result<(), String> {
        let in_range = |v: f32, lo: f32, hi: f32| v.is_finite() && (lo..=hi).contains(&v);
        match self {
            Layout::Bubble(b) => {
                if !in_range(b.size_pct, MIN_BUBBLE_PCT, MAX_BUBBLE_PCT) {
                    return Err(format!(
                        "The bubble is {MIN_BUBBLE_PCT} to {MAX_BUBBLE_PCT} % of the short side"
                    ));
                }
                if !in_range(b.roundness_pct, 0.0, MAX_ROUNDNESS_PCT) {
                    return Err("Bubble roundness is 0 to 50 %".into());
                }
                if !in_range(b.custom_x, 0.0, 100.0) || !in_range(b.custom_y, 0.0, 100.0) {
                    return Err("A custom bubble spot is 0 to 100 each way".into());
                }
            }
            Layout::SideBySide(s) => {
                if !in_range(s.camera_pct, MIN_SIDE_PCT, MAX_SIDE_PCT) {
                    return Err(format!(
                        "Side by side gives the camera {MIN_SIDE_PCT} to {MAX_SIDE_PCT} %"
                    ));
                }
            }
            Layout::CameraOnly(c) => {
                if !in_range(c.size_pct, MIN_CAMERA_ONLY_PCT, 100.0) {
                    return Err("Camera only is 40 to 100 % of the frame".into());
                }
                if !in_range(c.punch_in, 1.0, MAX_PUNCH_IN) {
                    return Err("A punch-in is 1x to 2x".into());
                }
            }
            Layout::ScreenOnly(_) => {}
        }
        Ok(())
    }

    /// What it is called in lists: "Camera bubble", "Side by side", ...
    pub fn kind_name(&self) -> &'static str {
        match self {
            Layout::Bubble(_) => "Camera bubble",
            Layout::SideBySide(_) => "Side by side",
            Layout::CameraOnly(c) if c.punch_in > 1.0 => "Punch-in",
            Layout::CameraOnly(_) => "Camera only",
            Layout::ScreenOnly(_) => "Screen only",
        }
    }

    /// One subject fills the frame (the camera alone or the screen alone): changes between two
    /// of these are cuts, like between two shots.
    pub fn is_single(&self) -> bool {
        matches!(self, Layout::CameraOnly(_) | Layout::ScreenOnly(_))
    }

    pub fn shows_camera(&self) -> bool {
        !matches!(self, Layout::ScreenOnly(_))
    }

    pub fn shows_screen(&self) -> bool {
        !matches!(self, Layout::CameraOnly(_))
    }

    /// The camera alone, edge to edge.
    pub fn camera_fills(&self) -> bool {
        matches!(self, Layout::CameraOnly(c) if c.size_pct >= 100.0)
    }
}

/// The layouts that are always there, by name.
pub fn built_ins() -> Vec<(&'static str, Layout)> {
    vec![
        (
            "Camera bubble",
            Layout::Bubble(Bubble {
                spot: BubbleSpot::BottomRight,
                size_pct: 35.0,
                ..Bubble::default()
            }),
        ),
        ("Side by side", Layout::SideBySide(SideBySide::default())),
        ("Camera only", Layout::CameraOnly(CameraOnly::default())),
        (
            "Punch-in",
            Layout::CameraOnly(CameraOnly {
                size_pct: 100.0,
                punch_in: 1.4,
            }),
        ),
        ("Screen only", Layout::ScreenOnly(ScreenOnly::default())),
    ]
}

/// The built-in layout called `name` (any case).
pub fn built_in(name: &str) -> Option<Layout> {
    let name = name.trim();
    built_ins()
        .into_iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, l)| l)
}

/// How a section changes into its layout and back.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Transition {
    /// A cut between two shots that each fill the frame, or where the change lands on a cut
    /// in the video; an animated move otherwise.
    #[default]
    Auto,
    Cut,
    Move,
}

/// A stretch of a timeline with its own layout, anchored in an asset's time.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutSection {
    pub id: String,
    /// The asset whose time it is in; `None` is the first recording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    pub source_start_us: u64,
    pub source_end_us: u64,
    /// The layout shown; absent shows the base (which keeps picked sections away).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,
    /// The favourite or built-in it was made from, for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub transition: Transition,
    /// The move's length; absent uses the timeline's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_ms: Option<u32>,
    /// Picked by a rule ([`pick`]); picking again replaces these and keeps the others.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub auto: bool,
    /// The rule that picked it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    /// The camera asset to show; absent is the camera on the timeline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    /// Where it plays on the timeline. Filled in for the UI, never stored.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edited_ranges: Vec<EditedRange>,
}

impl LayoutSection {
    pub fn new(
        id: String,
        media: Option<String>,
        start: u64,
        end: u64,
        layout: Option<Layout>,
    ) -> Self {
        Self {
            id,
            media,
            source_start_us: start,
            source_end_us: end,
            layout,
            name: None,
            transition: Transition::Auto,
            transition_ms: None,
            auto: false,
            rule: None,
            camera: None,
            edited_ranges: Vec::new(),
        }
    }
}

/// A timeline's layouts: the base, the sections over it, and how they change.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LayoutTrack {
    pub base: Layout,
    pub sections: Vec<LayoutSection>,
    /// How long a move takes, unless a section sets its own.
    pub transition_ms: u32,
    /// What picking sections ([`pick`]) looks for.
    pub pick: pick::PickSettings,
}

impl Default for LayoutTrack {
    fn default() -> Self {
        Self {
            base: Layout::default(),
            sections: Vec::new(),
            transition_ms: DEFAULT_TRANSITION_MS,
            pick: pick::PickSettings::default(),
        }
    }
}

impl LayoutTrack {
    /// A short's: camera on top of the screen at 35 %, and picking tuned for a short.
    pub fn for_short() -> Self {
        Self {
            base: Layout::SideBySide(SideBySide {
                camera_first: true,
                camera_pct: 35.0,
                overlap: false,
                screen: ScreenFit::Auto,
            }),
            pick: pick::PickSettings::for_short(),
            ..Self::default()
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        self.base.validate()?;
        self.pick.validate()?;
        if self.transition_ms > MAX_TRANSITION_MS {
            return Err(format!("A move takes at most {MAX_TRANSITION_MS} ms"));
        }
        if self.sections.len() > MAX_SECTIONS {
            return Err(format!(
                "A timeline has at most {MAX_SECTIONS} layout sections"
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for section in &self.sections {
            if section.id.is_empty() || section.id.len() > 64 || !ids.insert(section.id.as_str()) {
                return Err("Layout section ids must be unique".into());
            }
            if section.source_end_us <= section.source_start_us {
                return Err("A layout section must end after it starts".into());
            }
            if let Some(layout) = &section.layout {
                layout.validate()?;
            }
            if section
                .transition_ms
                .is_some_and(|ms| ms > MAX_TRANSITION_MS)
            {
                return Err(format!("A move takes at most {MAX_TRANSITION_MS} ms"));
            }
            if section
                .name
                .as_ref()
                .is_some_and(|n| n.chars().count() > 60)
            {
                return Err("A layout name is at most 60 characters".into());
            }
        }
        let mut sorted: Vec<&LayoutSection> = self.sections.iter().collect();
        sorted.sort_by(|a, b| (&a.media, a.source_start_us).cmp(&(&b.media, b.source_start_us)));
        if sorted
            .windows(2)
            .any(|p| p[0].media == p[1].media && p[1].source_start_us < p[0].source_end_us)
        {
            return Err("Layout sections can't overlap".into());
        }
        Ok(())
    }

    /// Sorted, without UI-only fields, so equal tracks compare equal.
    pub fn normalized(mut self) -> Self {
        for section in &mut self.sections {
            section.edited_ranges.clear();
        }
        self.sections.sort_by(|a, b| {
            (&a.media, a.source_start_us, &a.id).cmp(&(&b.media, b.source_start_us, &b.id))
        });
        self
    }

    /// Fills in where each section plays, for the UI.
    pub fn attach_edited_ranges(&mut self, clock: impl Fn(Option<&str>) -> Option<TimelineMapper>) {
        let mut clocks = Clocks::new(clock);
        for section in &mut self.sections {
            let pieces = clocks
                .get(section.media.as_deref())
                .map(|c| c.source_range_to_edited(section.source_start_us, section.source_end_us))
                .unwrap_or_default();
            // Pieces back to back (a cut inside it) are one stretch.
            section.edited_ranges = merge(pieces, 0)
                .into_iter()
                .map(|(start_us, end_us)| EditedRange { start_us, end_us })
                .collect();
        }
    }

    fn unique_id(&self, base: &str) -> String {
        let mut id = base.to_string();
        let mut n = 2;
        while self.sections.iter().any(|s| s.id == id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }

    /// A new id for a section made by hand.
    pub fn new_id(&self) -> String {
        let n = self.sections.len() + 1;
        self.unique_id(&format!("s{n}"))
    }

    /// Adds `added` (sections in source time), taking their time from every section they
    /// overlap on the same asset, auto or not.
    pub fn add(&mut self, added: Vec<LayoutSection>) {
        for new in added {
            self.take_out(
                new.media.as_deref(),
                &[(new.source_start_us, new.source_end_us)],
            );
            let mut new = new;
            new.id = self.unique_id(&new.id);
            self.sections.push(new);
        }
        *self = std::mem::take(self).normalized();
    }

    /// Takes the source `pieces` of asset `media` out of every section on it: what is left of
    /// a section keeps its id (its first piece) or takes a new one.
    pub fn take_out(&mut self, media: Option<&str>, pieces: &[(u64, u64)]) {
        let sections = std::mem::take(&mut self.sections);
        for section in sections {
            if section.media.as_deref() != media {
                self.sections.push(section);
                continue;
            }
            let left = subtract(&[(section.source_start_us, section.source_end_us)], pieces);
            for (i, (a, b)) in left.into_iter().enumerate() {
                let id = if i == 0 {
                    section.id.clone()
                } else {
                    format!("{}-{i}", section.id)
                };
                let piece = LayoutSection {
                    id: String::new(),
                    source_start_us: a,
                    source_end_us: b,
                    ..section.clone()
                };
                let id = self.unique_id(&id);
                self.sections.push(LayoutSection { id, ..piece });
            }
        }
    }

    /// Replaces the picked sections with `picked`; sections made by hand stay, and picked
    /// ones give way to them.
    pub fn replace_auto(&mut self, picked: Vec<LayoutSection>) {
        self.sections.retain(|s| !s.auto);
        let manual = self.sections.clone();
        let room = MAX_SECTIONS.saturating_sub(manual.len());
        let mut added = 0;
        for section in picked {
            let taken: Vec<(u64, u64)> = manual
                .iter()
                .filter(|m| m.media == section.media)
                .map(|m| (m.source_start_us, m.source_end_us))
                .collect();
            for (a, b) in subtract(&[(section.source_start_us, section.source_end_us)], &taken) {
                if b - a < MIN_SHOWN_US || added >= room {
                    continue;
                }
                let id = self.unique_id(&format!("auto-{}", a / 1000));
                self.sections.push(LayoutSection {
                    id,
                    source_start_us: a,
                    source_end_us: b,
                    auto: true,
                    ..section.clone()
                });
                added += 1;
            }
        }
        *self = std::mem::take(self).normalized();
    }
}

/// Clocks by asset, each built once: building one walks the whole sequence.
pub struct Clocks<F: Fn(Option<&str>) -> Option<TimelineMapper>> {
    make: F,
    built: std::collections::HashMap<Option<String>, Option<TimelineMapper>>,
}

impl<F: Fn(Option<&str>) -> Option<TimelineMapper>> Clocks<F> {
    pub fn new(make: F) -> Self {
        Self {
            make,
            built: Default::default(),
        }
    }

    pub fn get(&mut self, media: Option<&str>) -> Option<&TimelineMapper> {
        let key = media.map(String::from);
        if !self.built.contains_key(&key) {
            let clock = (self.make)(media);
            self.built.insert(key.clone(), clock);
        }
        self.built.get(&key).and_then(Option::as_ref)
    }
}

/// `ranges` sorted, with any closer than `gap` joined.
pub fn merge(mut ranges: Vec<(u64, u64)>, gap: u64) -> Vec<(u64, u64)> {
    ranges.retain(|(a, b)| b > a);
    ranges.sort_unstable();
    let mut out: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (a, b) in ranges {
        match out.last_mut() {
            Some(last) if a <= last.1.saturating_add(gap) => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

/// What is left of `ranges` outside `remove`.
pub fn subtract(ranges: &[(u64, u64)], remove: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let remove = merge(remove.to_vec(), 0);
    let mut out = Vec::new();
    for &(a, b) in ranges {
        let mut at = a;
        for &(x, y) in remove.iter().filter(|(x, y)| *x < b && *y > a) {
            if x > at {
                out.push((at, x));
            }
            at = at.max(y);
        }
        if at < b {
            out.push((at, b));
        }
    }
    out
}

/// The sections a timeline range makes on `clock` (asset `media`): one over the whole source
/// span when that span plays as exactly this range (a cut inside it maps nowhere), else one
/// per piece.
pub fn sections_for(
    clock: &TimelineMapper,
    media: Option<String>,
    start: u64,
    end: u64,
    template: &LayoutSection,
) -> Vec<LayoutSection> {
    let pieces = clock.edited_range_to_source(start, end);
    if pieces.is_empty() {
        return Vec::new();
    }
    let in_order = pieces.windows(2).all(|p| p[0].1 <= p[1].0);
    let hull = (pieces[0].0, pieces[pieces.len() - 1].1);
    let back = merge(clock.source_range_to_edited(hull.0, hull.1), 0);
    // Clips played faster or slower map back to within rounding.
    let slack = crate::sequence::join_slack(crate::sequence::MAX_SPEED);
    let whole = in_order
        && matches!(back.as_slice(), [(a, b)] if a.abs_diff(start) <= slack && b.abs_diff(end) <= slack);
    let spans = if whole { vec![hull] } else { pieces };
    spans
        .into_iter()
        .enumerate()
        .map(|(i, (a, b))| LayoutSection {
            id: if i == 0 {
                template.id.clone()
            } else {
                format!("{}-{}", template.id, i + 1)
            },
            media: media.clone(),
            source_start_us: a,
            source_end_us: b,
            edited_ranges: Vec::new(),
            ..template.clone()
        })
        .collect()
}

/// The asset a section over `[start, end)` of `document`'s timeline is anchored on: what the
/// screen shows at its start, else the camera, else any picture, else the first recording.
pub fn anchor_at(
    document: &crate::project::revision::EditDocument,
    start: u64,
    end: u64,
) -> Option<String> {
    use crate::sequence::Role;
    let probe = start + (end.saturating_sub(start)).min(1_000) / 2;
    document
        .top_clip_at(probe, Role::Screen)
        .or_else(|| document.top_clip_at(probe, Role::Webcam))
        .map(|(_, c)| c.asset.clone())
        .or_else(|| {
            // Any picture there, else any sound (a voice-only take).
            let video = document
                .sequence
                .video_tracks()
                .find_map(|t| t.clip_at(probe));
            video
                .or_else(|| {
                    document
                        .sequence
                        .tracks
                        .iter()
                        .find_map(|t| t.clip_at(probe))
                })
                .map(|c| c.asset.clone())
        })
        .or_else(|| document.clock_asset(None).map(String::from))
}

/// Sections like `template` over `[start, end)` of `document`'s timeline, anchored on what
/// plays there.
pub fn sections_on(
    document: &crate::project::revision::EditDocument,
    start: u64,
    end: u64,
    template: &LayoutSection,
) -> Result<Vec<LayoutSection>, String> {
    if end <= start {
        return Err("A layout section runs from a time to a later one".into());
    }
    if end - start < MIN_SHOWN_US {
        return Err("A layout section is at least a second long".into());
    }
    let media = anchor_at(document, start, end).ok_or("Nothing plays there")?;
    let clock = document.asset_clock(&media);
    let made = sections_for(&clock, Some(media), start, end, template);
    if made.is_empty() {
        return Err("Nothing plays there".into());
    }
    Ok(made)
}

/// Takes `[start, end)` of `document`'s timeline out of every section of `track` that plays
/// there, whatever asset it is on.
pub fn remove_on(
    document: &crate::project::revision::EditDocument,
    track: &mut LayoutTrack,
    start: u64,
    end: u64,
) {
    let medias: std::collections::BTreeSet<Option<String>> =
        track.sections.iter().map(|s| s.media.clone()).collect();
    for media in medias {
        let Some(clock) = document.anchor_clock(media.as_deref()) else {
            continue;
        };
        let pieces = clock.edited_range_to_source(start, end);
        track.take_out(media.as_deref(), &pieces);
    }
    *track = std::mem::take(track).normalized();
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u64 = 1_000_000;

    fn section(id: &str, a: u64, b: u64, layout: Option<Layout>) -> LayoutSection {
        LayoutSection::new(id.into(), None, a, b, layout)
    }

    #[test]
    fn layouts_round_trip_as_tagged_json() {
        let layout = Layout::SideBySide(SideBySide {
            camera_pct: 30.0,
            ..SideBySide::default()
        });
        let json = serde_json::to_value(&layout).unwrap();
        assert_eq!(json["kind"], "sideBySide");
        assert_eq!(json["cameraPct"], 30.0);
        let back: Layout = serde_json::from_value(json).unwrap();
        assert_eq!(back, layout);
        // Missing fields take their defaults.
        let sparse: Layout = serde_json::from_str(r#"{"kind":"bubble","spot":"top"}"#).unwrap();
        assert_eq!(
            sparse,
            Layout::Bubble(Bubble {
                spot: BubbleSpot::Top,
                ..Bubble::default()
            })
        );
        for (_, layout) in built_ins() {
            layout.validate().unwrap();
        }
        assert!(Layout::CameraOnly(CameraOnly {
            size_pct: 100.0,
            punch_in: 3.0
        })
        .validate()
        .is_err());
    }

    #[test]
    fn adding_takes_time_from_what_it_overlaps_and_picking_keeps_manual() {
        let mut track = LayoutTrack::default();
        track.add(vec![section(
            "a",
            10 * S,
            20 * S,
            Some(Layout::ScreenOnly(Default::default())),
        )]);
        track.add(vec![section("b", 14 * S, 16 * S, None)]);
        let spans: Vec<(&str, u64, u64)> = track
            .sections
            .iter()
            .map(|s| (s.id.as_str(), s.source_start_us, s.source_end_us))
            .collect();
        assert_eq!(
            spans,
            vec![
                ("a", 10 * S, 14 * S),
                ("b", 14 * S, 16 * S),
                ("a-1", 16 * S, 20 * S)
            ]
        );
        track.validate().unwrap();
        // Picked sections go around the manual ones, and too-short leftovers are dropped.
        let picked = vec![LayoutSection {
            auto: true,
            ..section("x", 0, 30 * S, Some(Layout::CameraOnly(Default::default())))
        }];
        track.replace_auto(picked.clone());
        let auto: Vec<(u64, u64)> = track
            .sections
            .iter()
            .filter(|s| s.auto)
            .map(|s| (s.source_start_us, s.source_end_us))
            .collect();
        assert_eq!(auto, vec![(0, 10 * S), (20 * S, 30 * S)]);
        // Picking again replaces them rather than piling up.
        track.replace_auto(picked);
        assert_eq!(track.sections.iter().filter(|s| s.auto).count(), 2);
        track.validate().unwrap();
    }

    /// A range over a clip at half speed and the normal-speed one after it is one section,
    /// though half speed maps back a microsecond off.
    #[test]
    fn a_range_over_a_slower_clip_is_one_section() {
        use crate::timeline::SourceInterval;
        let clock = TimelineMapper::new(vec![
            SourceInterval::new("a".into(), 0, 10 * S).with_edited_len(Some(20 * S)),
            SourceInterval::new("b".into(), 10 * S, 20 * S),
        ]);
        let made = sections_for(&clock, None, S + 1, 25 * S, &section("n", 0, 1, None));
        let spans: Vec<(u64, u64)> = made
            .iter()
            .map(|s| (s.source_start_us, s.source_end_us))
            .collect();
        assert_eq!(spans, vec![(S / 2, 15 * S)]);
    }

    #[test]
    fn a_range_over_a_cut_is_one_section() {
        use crate::timeline::SourceInterval;
        // The recording plays 0-10 s then 15-30 s: 5 s were cut.
        let clock = TimelineMapper::new(vec![
            SourceInterval::new("a".into(), 0, 10 * S),
            SourceInterval::new("b".into(), 15 * S, 30 * S),
        ]);
        let made = sections_for(&clock, None, 8 * S, 12 * S, &section("n", 0, 1, None));
        assert_eq!(made.len(), 1);
        assert_eq!(
            (made[0].source_start_us, made[0].source_end_us),
            (8 * S, 17 * S)
        );
        // Reordered: 15-30 s plays first, then 0-10 s.
        let reordered = TimelineMapper::new(vec![
            SourceInterval::new("b".into(), 15 * S, 30 * S),
            SourceInterval::new("a".into(), 0, 10 * S),
        ]);
        let made = sections_for(&reordered, None, 13 * S, 17 * S, &section("n", 0, 1, None));
        let spans: Vec<(u64, u64)> = made
            .iter()
            .map(|s| (s.source_start_us, s.source_end_us))
            .collect();
        assert_eq!(spans, vec![(28 * S, 30 * S), (0, 2 * S)]);
        assert_eq!(made[1].id, "n-2");
    }
}
