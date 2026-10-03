//! Deterministic telemetry-driven zoom suggestions (Z1).
//!
//! Interest comes from known primary button-down transitions, v1 click records
//! that are not paired with downs, and cursor dwell. Auxiliary buttons, off-source
//! coordinates, unsupported geometry, and uncertainty/gap intervals never become
//! targets. Camera viewport clamping does not rewrite recorded coordinates.
mod eval;

pub use eval::{
    clamp_camera, cubic_bezier_unit, evaluate_at_edited, evaluate_at_source, CameraTransform,
};

use crate::telemetry::reader::{CanonicalKind, TelemetryStream};
use crate::timeline::TimelineMapper;
use serde::{Deserialize, Serialize};

pub const ZOOM_GENERATION_VERSION: u32 = 1;
pub const MAX_SUGGESTIONS: usize = 512;
pub const MAX_PRIMARY_BUTTONS: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZoomConfig {
    /// Must match [`ZOOM_GENERATION_VERSION`]. Unknown versions are rejected.
    pub generation_version: u32,
    pub dwell_radius_norm: f64,
    pub min_dwell_us: u64,
    pub cluster_radius_norm: f64,
    pub cluster_gap_us: u64,
    /// Primary downs inside this window merge. Auxiliary buttons never generate
    /// interest. V1 `click` records are used only when the stream has no downs.
    pub rapid_click_window_us: u64,
    pub min_hold_us: u64,
    pub transition_us: u64,
    pub max_scale: f64,
    pub click_scale: f64,
    pub dwell_scale: f64,
    pub geometry_uncertainty_us: u64,
    pub primary_buttons: Vec<u32>,
    pub viewport_margin: f64,
    /// Activity closer than this is one zoom that follows the mouse, never two.
    #[serde(default = "default_merge_gap")]
    pub merge_gap_us: u64,
    /// The most zooms per recording: the strongest moments are kept.
    #[serde(default = "default_max_zooms")]
    pub max_zooms: u32,
    /// A zoom longer than this lets go, so the viewer sees the whole screen again.
    #[serde(default = "default_max_length")]
    pub max_length_us: u64,
}

fn default_merge_gap() -> u64 {
    2_500_000
}
fn default_max_zooms() -> u32 {
    10
}
fn default_max_length() -> u64 {
    15_000_000
}

impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            generation_version: ZOOM_GENERATION_VERSION,
            dwell_radius_norm: 0.04,
            min_dwell_us: 1_500_000,
            cluster_radius_norm: 0.12,
            cluster_gap_us: 800_000,
            rapid_click_window_us: 350_000,
            min_hold_us: 1_200_000,
            transition_us: 400_000,
            max_scale: 2.0,
            click_scale: 2.0,
            dwell_scale: 1.5,
            geometry_uncertainty_us: 100_000,
            primary_buttons: vec![0],
            viewport_margin: 0.0,
            merge_gap_us: default_merge_gap(),
            max_zooms: default_max_zooms(),
            max_length_us: default_max_length(),
        }
    }
}

/// The project's auto-zoom settings: one place, so every window and every zoom agrees.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ZoomSettings {
    /// How far a zoom on clicks goes in.
    pub click_scale: f64,
    /// How far a zoom where the mouse rests goes in; 1 switches hover zooms off.
    pub hover_scale: f64,
    /// Time to zoom in, and to zoom out.
    pub transition_ms: u32,
    /// The shortest zoom.
    pub min_hold_ms: u32,
    /// Activity closer than this is one zoom that follows the mouse.
    pub merge_gap_ms: u32,
    /// The most zooms per recording.
    pub max_zooms: u32,
    /// The camera follows the mouse inside a zoom.
    pub follow: bool,
    /// How calmly it follows: the camera's lag behind the mouse.
    pub follow_ms: u32,
}

impl Default for ZoomSettings {
    fn default() -> Self {
        Self {
            click_scale: 1.8,
            hover_scale: 1.4,
            transition_ms: 700,
            min_hold_ms: 1_800,
            merge_gap_ms: 2_500,
            max_zooms: 10,
            follow: true,
            follow_ms: 700,
        }
    }
}

impl ZoomSettings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        let finite = self.click_scale.is_finite() && self.hover_scale.is_finite();
        if !finite
            || !(1.1..=4.0).contains(&self.click_scale)
            || !(1.0..=4.0).contains(&self.hover_scale)
        {
            return Err("Zoom amounts are out of range".into());
        }
        if !(100..=3_000).contains(&self.transition_ms)
            || !(500..=20_000).contains(&self.min_hold_ms)
            || !(0..=10_000).contains(&self.merge_gap_ms)
            || !(1..=200).contains(&self.max_zooms)
            || !(100..=3_000).contains(&self.follow_ms)
        {
            return Err("Auto-zoom settings are out of range".into());
        }
        Ok(())
    }

    /// The generator's config for these settings.
    pub fn config(&self) -> ZoomConfig {
        let hover = self.hover_scale > 1.0;
        let defaults = ZoomConfig::default();
        ZoomConfig {
            click_scale: self.click_scale,
            dwell_scale: if hover { self.hover_scale } else { 1.0 },
            max_scale: self.click_scale.max(self.hover_scale).clamp(1.0, 8.0),
            transition_us: self.transition_ms as u64 * 1_000,
            min_hold_us: self.min_hold_ms as u64 * 1_000,
            // Hover off: a rest no one reaches.
            min_dwell_us: if hover {
                defaults.min_dwell_us
            } else {
                3_600_000_000
            },
            merge_gap_us: self.merge_gap_ms as u64 * 1_000,
            max_zooms: self.max_zooms,
            ..defaults
        }
    }

    /// The zoom amount for a zoom of this kind.
    pub fn scale_for(&self, origin: ZoomOrigin) -> f64 {
        match origin {
            ZoomOrigin::Dwell if self.hover_scale > 1.0 => self.hover_scale,
            _ => self.click_scale,
        }
    }
}

impl ZoomConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.generation_version != ZOOM_GENERATION_VERSION {
            return Err(format!(
                "Unsupported zoom generation version {}",
                self.generation_version
            ));
        }
        let finite_positive = |name: &str, value: f64| {
            if value.is_finite() && value > 0.0 {
                Ok(())
            } else {
                Err(format!("{name} must be a finite positive number"))
            }
        };
        finite_positive("dwellRadiusNorm", self.dwell_radius_norm)?;
        finite_positive("clusterRadiusNorm", self.cluster_radius_norm)?;
        finite_positive("maxScale", self.max_scale)?;
        finite_positive("clickScale", self.click_scale)?;
        finite_positive("dwellScale", self.dwell_scale)?;
        if self.dwell_radius_norm > 1.0 || self.cluster_radius_norm > 1.0 {
            return Err("Spatial radii must be within one normalized screen".into());
        }
        if !(1.0..=8.0).contains(&self.max_scale) {
            return Err("maxScale must be between 1 and 8".into());
        }
        if self.click_scale > self.max_scale || self.dwell_scale > self.max_scale {
            return Err("Zoom scales cannot exceed maxScale".into());
        }
        if self.click_scale < 1.0 || self.dwell_scale < 1.0 {
            return Err("Zoom scales must be at least 1".into());
        }
        if self.min_dwell_us == 0 || self.min_hold_us == 0 || self.transition_us == 0 {
            return Err("Dwell, hold and transition durations must be nonzero".into());
        }
        if self.transition_us > 10_000_000 || self.min_hold_us > 60_000_000 {
            return Err("Hold/transition durations exceed supported bounds".into());
        }
        if self.primary_buttons.len() > MAX_PRIMARY_BUTTONS {
            return Err("Too many primary buttons".into());
        }
        if !self.viewport_margin.is_finite()
            || self.viewport_margin < 0.0
            || self.viewport_margin >= 0.5
        {
            return Err("viewportMargin must be in [0, 0.5)".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ZoomOrigin {
    Click,
    Dwell,
    Cluster,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditedRange {
    pub start_us: u64,
    pub end_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZoomSuggestion {
    pub id: String,
    pub source_start_us: u64,
    pub source_end_us: u64,
    pub center_x: f64,
    pub center_y: f64,
    pub scale: f64,
    pub transition_us: u64,
    pub origin: ZoomOrigin,
    pub contributing_event_seqs: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edited_ranges: Vec<EditedRange>,
    /// The imported recording whose clock the times are on; `None` is the project's own
    /// recording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Where the camera looks while zoomed, (source time, x, y), when it follows the mouse.
    /// Worked out from the recording when drawing; never stored.
    #[serde(default, skip)]
    pub path: Vec<(u64, f32, f32)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ZoomSource {
    Generated,
    Manual,
}

pub const MAX_ZOOMS: usize = 512;
pub const MAX_DISMISSED_ZOOMS: usize = 1_024;

/// Source-anchored zoom persisted in `project.json`. Regeneration never
/// overwrites an existing id or a dismissed generated suggestion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZoomKeyframe {
    pub id: String,
    pub source_start_us: u64,
    pub source_end_us: u64,
    pub center_x: f64,
    pub center_y: f64,
    pub scale: f64,
    pub transition_us: u64,
    pub origin: ZoomOrigin,
    #[serde(default)]
    pub contributing_event_seqs: Vec<u64>,
    pub source: ZoomSource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edited_ranges: Vec<EditedRange>,
    /// The imported recording whose clock the times are on; `None` is the project's own
    /// recording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Stays on its center instead of following the mouse.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fixed: bool,
}

impl ZoomKeyframe {
    pub fn from_suggestion(suggestion: ZoomSuggestion, source: ZoomSource) -> Self {
        Self {
            id: suggestion.id,
            source_start_us: suggestion.source_start_us,
            source_end_us: suggestion.source_end_us,
            center_x: suggestion.center_x,
            center_y: suggestion.center_y,
            scale: suggestion.scale,
            transition_us: suggestion.transition_us,
            origin: suggestion.origin,
            contributing_event_seqs: suggestion.contributing_event_seqs,
            source,
            edited_ranges: suggestion.edited_ranges,
            media: suggestion.media,
            fixed: false,
        }
    }

    pub fn as_suggestion(&self) -> ZoomSuggestion {
        ZoomSuggestion {
            path: Vec::new(),
            id: self.id.clone(),
            source_start_us: self.source_start_us,
            source_end_us: self.source_end_us,
            center_x: self.center_x,
            center_y: self.center_y,
            scale: self.scale,
            transition_us: self.transition_us,
            origin: self.origin,
            contributing_event_seqs: self.contributing_event_seqs.clone(),
            edited_ranges: self.edited_ranges.clone(),
            media: self.media.clone(),
        }
    }
}

pub fn validate_zooms(zooms: &[ZoomKeyframe]) -> Result<(), String> {
    if zooms.len() > MAX_ZOOMS {
        return Err("Too many zoom keyframes".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for zoom in zooms {
        if zoom.id.is_empty() || zoom.id.len() > 128 {
            return Err("Invalid zoom id".into());
        }
        if !seen.insert(&zoom.id) {
            return Err(format!("Duplicate zoom id {}", zoom.id));
        }
        if zoom.source_end_us <= zoom.source_start_us {
            return Err("Zoom must be a half-open source range".into());
        }
        if zoom.source_end_us > 9_007_199_254_740_991 {
            return Err("Zoom timestamp exceeds supported precision".into());
        }
        if !zoom.center_x.is_finite()
            || !zoom.center_y.is_finite()
            || !zoom.scale.is_finite()
            || !(0.0..=1.0).contains(&zoom.center_x)
            || !(0.0..=1.0).contains(&zoom.center_y)
            || !(1.0..=8.0).contains(&zoom.scale)
        {
            return Err("Zoom center/scale is out of range".into());
        }
        if zoom.transition_us == 0
            || zoom.transition_us >= zoom.source_end_us.saturating_sub(zoom.source_start_us)
        {
            return Err("Zoom transition must fit inside the source range".into());
        }
    }
    Ok(())
}

/// Where each zoom lands on the edited timeline. `mapper_for` gives the mapper of a zoom's
/// source (the recording, or an imported recording's own clips).
pub fn attach_zoom_edited_ranges_with(
    zooms: &mut [ZoomKeyframe],
    mapper_for: &dyn Fn(Option<&str>) -> Option<TimelineMapper>,
) {
    for zoom in zooms {
        zoom.edited_ranges = mapper_for(zoom.media.as_deref())
            .map(|mapper| {
                mapper
                    .source_range_to_edited(zoom.source_start_us, zoom.source_end_us)
                    .into_iter()
                    .map(|(start_us, end_us)| EditedRange { start_us, end_us })
                    .collect()
            })
            .unwrap_or_default();
    }
}

pub fn attach_zoom_edited_ranges(zooms: &mut [ZoomKeyframe], mapper: &TimelineMapper) {
    for zoom in zooms {
        zoom.edited_ranges = mapper
            .source_range_to_edited(zoom.source_start_us, zoom.source_end_us)
            .into_iter()
            .map(|(start_us, end_us)| EditedRange { start_us, end_us })
            .collect();
    }
}

pub fn eval_config_for(zooms: &[ZoomKeyframe]) -> ZoomConfig {
    let mut config = ZoomConfig::default();
    let max = zooms
        .iter()
        .map(|z| z.scale)
        .fold(config.max_scale, f64::max)
        .clamp(1.0, 8.0);
    config.max_scale = max;
    config
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZoomGeneration {
    pub version: u32,
    pub config: ZoomConfig,
    pub suggestions: Vec<ZoomSuggestion>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone)]
struct InterestPoint {
    t_us: u64,
    end_us: u64,
    seq: u64,
    seqs: Vec<u64>,
    x: f64,
    y: f64,
    origin: ZoomOrigin,
}

struct Interval {
    start: u64,
    end: u64,
}

impl Interval {
    fn contains(&self, t: u64) -> bool {
        t >= self.start && t < self.end
    }
}

/// Build source-anchored zoom suggestions. Identical input and config yield
/// identical IDs, ordering and transforms.
pub fn generate_zoom_suggestions(
    stream: &TelemetryStream,
    config: &ZoomConfig,
) -> Result<ZoomGeneration, String> {
    config.validate()?;
    let mut diagnostics = stream.diagnostics.clone();
    let uncertainty = collect_uncertainty(stream, config);
    let ignore_v1_clicks = stream
        .events
        .iter()
        .any(|event| matches!(event.kind, CanonicalKind::ButtonDown { .. }));
    let interest = extract_interest(
        stream,
        config,
        &uncertainty,
        ignore_v1_clicks,
        &mut diagnostics,
    );
    let mut suggestions = plan_zooms(&interest, config);
    suggestions.truncate(MAX_SUGGESTIONS);
    if suggestions.is_empty()
        && !diagnostics
            .iter()
            .any(|d| d.contains("No zoom interest") || d.contains("No telemetry"))
    {
        diagnostics.push(
            "No zoom interest; telemetry is empty, denied, gapped, or has no usable targets".into(),
        );
    }
    diagnostics.truncate(256);
    Ok(ZoomGeneration {
        version: config.generation_version,
        config: config.clone(),
        suggestions,
        diagnostics,
    })
}

pub fn attach_edited_ranges(generation: &mut ZoomGeneration, mapper: &TimelineMapper) {
    for suggestion in &mut generation.suggestions {
        suggestion.edited_ranges = mapper
            .source_range_to_edited(suggestion.source_start_us, suggestion.source_end_us)
            .into_iter()
            .map(|(start_us, end_us)| EditedRange { start_us, end_us })
            .collect();
    }
}

fn collect_uncertainty(stream: &TelemetryStream, config: &ZoomConfig) -> Vec<Interval> {
    let mut intervals = Vec::new();
    for geometry in stream.geometries.values() {
        let span = geometry
            .sampling_interval_us
            .max(config.geometry_uncertainty_us);
        intervals.push(Interval {
            start: geometry.t_us,
            end: geometry.t_us.saturating_add(span),
        });
    }
    for event in &stream.events {
        if let CanonicalKind::Gap {
            start_us, end_us, ..
        } = event.kind
        {
            intervals.push(Interval {
                start: start_us,
                end: end_us.max(start_us.saturating_add(1)),
            });
        }
    }
    intervals.sort_by_key(|i| (i.start, i.end));
    intervals
}

fn in_uncertainty(intervals: &[Interval], t: u64) -> bool {
    intervals.iter().any(|interval| interval.contains(t))
}

fn is_primary(button: Option<u32>, config: &ZoomConfig) -> bool {
    match button {
        None => true,
        Some(button) => config.primary_buttons.contains(&button),
    }
}

fn usable_target(
    stream: &TelemetryStream,
    geometry_id: Option<&String>,
    x: f64,
    y: f64,
    inside: bool,
    t_us: u64,
    uncertainty: &[Interval],
) -> bool {
    if !inside || !x.is_finite() || !y.is_finite() {
        return false;
    }
    if !(0.0..1.0).contains(&x) || !(0.0..1.0).contains(&y) {
        return false;
    }
    if in_uncertainty(uncertainty, t_us) {
        return false;
    }
    let Some(id) = geometry_id else {
        return false;
    };
    match stream.geometries.get(id) {
        Some(geometry) if geometry.supported => true,
        _ => false,
    }
}

fn dist(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
}

fn extract_interest(
    stream: &TelemetryStream,
    config: &ZoomConfig,
    uncertainty: &[Interval],
    ignore_v1_clicks: bool,
    diagnostics: &mut Vec<String>,
) -> Vec<InterestPoint> {
    let mut interest = Vec::new();
    let mut held: Option<Vec<u32>> = None;
    let mut dwell: Option<Dwell> = None;

    let flush_dwell = |dwell: &mut Option<Dwell>,
                       end_us: u64,
                       interest: &mut Vec<InterestPoint>| {
        if let Some(active) = dwell.take() {
            if end_us.saturating_sub(active.start_us) >= config.min_dwell_us && active.count > 0 {
                interest.push(InterestPoint {
                    t_us: active.start_us,
                    end_us,
                    seq: active.seqs[0],
                    seqs: active.seqs,
                    x: active.sum_x / active.count as f64,
                    y: active.sum_y / active.count as f64,
                    origin: ZoomOrigin::Dwell,
                });
            }
        }
    };

    for event in &stream.events {
        match &event.kind {
            // Cursor shape changes and unknown records are not interest.
            CanonicalKind::Note => {}
            CanonicalKind::Gap { start_us, .. } => {
                let end = dwell
                    .as_ref()
                    .map(|active| active.last_us.min(*start_us))
                    .unwrap_or(*start_us);
                flush_dwell(&mut dwell, end, &mut interest);
                held = None;
            }
            CanonicalKind::Move {
                norm_x,
                norm_y,
                inside_source,
            }
            | CanonicalKind::ButtonDown {
                norm_x,
                norm_y,
                inside_source,
                ..
            }
            | CanonicalKind::ButtonUp {
                norm_x,
                norm_y,
                inside_source,
                ..
            }
            | CanonicalKind::Click {
                norm_x,
                norm_y,
                inside_source,
            } => {
                let usable = usable_target(
                    stream,
                    event.geometry_id.as_ref(),
                    *norm_x,
                    *norm_y,
                    *inside_source,
                    event.t_us,
                    uncertainty,
                );
                if usable {
                    match &mut dwell {
                        Some(active)
                            if dist(active.anchor_x, active.anchor_y, *norm_x, *norm_y)
                                <= config.dwell_radius_norm =>
                        {
                            active.sum_x += *norm_x;
                            active.sum_y += *norm_y;
                            active.count += 1;
                            active.seqs.push(event.seq);
                            active.last_us = event.t_us;
                        }
                        _ => {
                            // A resting mouse sends nothing: the rest lasts until it moves off.
                            flush_dwell(&mut dwell, event.t_us, &mut interest);
                            dwell = Some(Dwell {
                                start_us: event.t_us,
                                last_us: event.t_us,
                                anchor_x: *norm_x,
                                anchor_y: *norm_y,
                                sum_x: *norm_x,
                                sum_y: *norm_y,
                                count: 1,
                                seqs: vec![event.seq],
                            });
                        }
                    }
                } else {
                    flush_dwell(&mut dwell, event.t_us, &mut interest);
                    if *inside_source == false
                        && diagnostics.len() < 256
                        && !diagnostics.iter().any(|d| d.contains("off-source"))
                    {
                        diagnostics.push(
                            "Off-source coordinates ignored; they are not clamped into edge clicks"
                                .into(),
                        );
                    }
                }

                match &event.kind {
                    CanonicalKind::ButtonDown { button, .. } => {
                        match &mut held {
                            None => held = Some(button.into_iter().copied().collect()),
                            Some(buttons) => {
                                if let Some(button) = button {
                                    if !buttons.contains(button) {
                                        buttons.push(*button);
                                        buttons.sort_unstable();
                                    }
                                }
                            }
                        }
                        if usable && is_primary(*button, config) {
                            interest.push(InterestPoint {
                                t_us: event.t_us,
                                end_us: event.t_us,
                                seq: event.seq,
                                seqs: vec![event.seq],
                                x: *norm_x,
                                y: *norm_y,
                                origin: ZoomOrigin::Click,
                            });
                        }
                    }
                    CanonicalKind::ButtonUp { button, .. } => {
                        if let Some(buttons) = held.as_mut() {
                            if let Some(button) = button {
                                buttons.retain(|b| b != button);
                            } else {
                                buttons.clear();
                            }
                        }
                    }
                    CanonicalKind::Click { .. } => {
                        if usable && !ignore_v1_clicks {
                            interest.push(InterestPoint {
                                t_us: event.t_us,
                                end_us: event.t_us,
                                seq: event.seq,
                                seqs: vec![event.seq],
                                x: *norm_x,
                                y: *norm_y,
                                origin: ZoomOrigin::Click,
                            });
                        }
                    }
                    _ => {}
                }
            }
            CanonicalKind::Scroll { .. } => {}
        }
    }
    let end = dwell.as_ref().map(|d| d.last_us).unwrap_or(0);
    flush_dwell(&mut dwell, end, &mut interest);
    interest.sort_by_key(|p| (p.t_us, p.seq));
    interest
}

struct Dwell {
    start_us: u64,
    last_us: u64,
    anchor_x: f64,
    anchor_y: f64,
    sum_x: f64,
    sum_y: f64,
    count: u32,
    seqs: Vec<u64>,
}

/// Zooms closer than this become one: zooming out and straight back in is jarring.
const MIN_ZOOM_GAP_US: u64 = 1_500_000;

/// A run of activity that makes one zoom: clicks and rests close together in time.
struct Session<'a> {
    points: Vec<&'a InterestPoint>,
    start_us: u64,
    end_us: u64,
}

impl Session<'_> {
    fn clicks(&self) -> usize {
        self.points
            .iter()
            .filter(|p| p.origin == ZoomOrigin::Click)
            .count()
    }

    /// How much this moment deserves a zoom: clicks count most, a rest a little, and a long
    /// session no more than a few clicks would.
    fn score(&self) -> f64 {
        let rest_us: u64 = self
            .points
            .iter()
            .filter(|p| p.origin == ZoomOrigin::Dwell)
            .map(|p| p.end_us.saturating_sub(p.t_us).min(4_000_000))
            .sum();
        3.0 * (self.clicks() as f64).min(6.0) + (rest_us as f64 / 1e6).min(4.0)
    }
}

/// Zooms worth watching: activity close together is one zoom (the camera follows the mouse
/// through it), the strongest `max_zooms` are kept, and none overlap. Hover-only zooms come
/// from real rests and only when hover zooms are on.
pub fn plan_zooms(points: &[InterestPoint], config: &ZoomConfig) -> Vec<ZoomSuggestion> {
    let hover = config.dwell_scale > 1.0;
    let mut sessions: Vec<Session> = Vec::new();
    for point in points {
        if point.origin == ZoomOrigin::Dwell && !hover {
            continue;
        }
        // A long rest (reading, thinking) holds a zoom a little, not for all of it.
        let end = match point.origin {
            ZoomOrigin::Dwell => point
                .end_us
                .min(point.t_us.saturating_add(config.min_hold_us)),
            _ => point.end_us,
        }
        .max(point.t_us);
        if let Some(session) = sessions.last_mut() {
            let joins = point.t_us <= session.end_us.saturating_add(config.merge_gap_us)
                && point.t_us.saturating_sub(session.start_us) < config.max_length_us;
            if joins {
                session.end_us = session.end_us.max(end);
                session.points.push(point);
                continue;
            }
        }
        sessions.push(Session {
            points: vec![point],
            start_us: point.t_us,
            end_us: end,
        });
    }
    // The strongest moments, back in time order.
    let mut ranked: Vec<(usize, f64)> = sessions
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.score()))
        .collect();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    let keep: std::collections::BTreeSet<usize> = ranked
        .into_iter()
        .take(config.max_zooms.max(1) as usize)
        .map(|(i, _)| i)
        .collect();
    let mut zooms: Vec<ZoomSuggestion> = Vec::new();
    for (index, session) in sessions.iter().enumerate() {
        if !keep.contains(&index) {
            continue;
        }
        let clicks = session.clicks();
        let first = session.points[0];
        // In before the first action, out a little after the last.
        let hold_start = session.start_us;
        let hold_end = session
            .end_us
            .saturating_add(config.min_hold_us / 2)
            .max(hold_start.saturating_add(config.min_hold_us))
            .min(hold_start.saturating_add(config.max_length_us));
        let source_start = hold_start.saturating_sub(config.transition_us);
        let source_end = hold_end.saturating_add(config.transition_us);
        let mut seqs: Vec<u64> = session
            .points
            .iter()
            .flat_map(|p| p.seqs.iter().copied())
            .collect();
        seqs.sort_unstable();
        seqs.dedup();
        let origin = if clicks == 0 {
            ZoomOrigin::Dwell
        } else if clicks == session.points.len() {
            ZoomOrigin::Click
        } else {
            ZoomOrigin::Cluster
        };
        let zoom = ZoomSuggestion {
            path: Vec::new(),
            id: format!("z-{}-n{}", seqs[0], seqs.len()),
            source_start_us: source_start,
            source_end_us: source_end,
            center_x: first.x,
            center_y: first.y,
            scale: if clicks > 0 {
                config.click_scale
            } else {
                config.dwell_scale
            }
            .min(config.max_scale),
            transition_us: config.transition_us,
            origin,
            contributing_event_seqs: seqs,
            edited_ranges: Vec::new(),
            media: None,
        };
        // Overlapping the one before, or so close that the view would pop out and straight
        // back in: one zoom carries on, following the mouse.
        if let Some(previous) = zooms.last_mut() {
            if zoom.source_start_us < previous.source_end_us.saturating_add(MIN_ZOOM_GAP_US) {
                previous.source_end_us = previous.source_end_us.max(zoom.source_end_us);
                previous.scale = previous.scale.max(zoom.scale);
                if zoom.origin != ZoomOrigin::Dwell {
                    previous.origin = ZoomOrigin::Cluster;
                }
                previous
                    .contributing_event_seqs
                    .extend(zoom.contributing_event_seqs);
                previous.contributing_event_seqs.sort_unstable();
                previous.contributing_event_seqs.dedup();
                continue;
            }
        }
        zooms.push(zoom);
    }
    for zoom in &mut zooms {
        let length = zoom.source_end_us - zoom.source_start_us;
        zoom.transition_us = zoom.transition_us.min(length.saturating_sub(1) / 2).max(1);
    }
    zooms
}

/// Where the mouse was, (time, x, y) on the recorded screen, from a recording's telemetry.
pub fn cursor_samples(stream: &TelemetryStream) -> Vec<(u64, f64, f64)> {
    let mut samples: Vec<(u64, f64, f64)> = stream
        .events
        .iter()
        .filter_map(|event| match event.kind {
            CanonicalKind::Move {
                norm_x,
                norm_y,
                inside_source,
            }
            | CanonicalKind::ButtonDown {
                norm_x,
                norm_y,
                inside_source,
                ..
            }
            | CanonicalKind::ButtonUp {
                norm_x,
                norm_y,
                inside_source,
                ..
            }
            | CanonicalKind::Click {
                norm_x,
                norm_y,
                inside_source,
            } => (inside_source && (0.0..=1.0).contains(&norm_x) && (0.0..=1.0).contains(&norm_y))
                .then_some((event.t_us, norm_x, norm_y)),
            _ => None,
        })
        .collect();
    samples.sort_by_key(|s| s.0);
    samples
}

/// The mouse at `t_us`: still where it rested, moving straight between close samples.
fn cursor_at(samples: &[(u64, f64, f64)], t_us: f64) -> (f64, f64) {
    let index = samples.partition_point(|s| (s.0 as f64) <= t_us);
    if index == 0 {
        return (samples[0].1, samples[0].2);
    }
    let a = samples[index - 1];
    let Some(&b) = samples.get(index) else {
        return (a.1, a.2);
    };
    let start = (a.0 as f64).max(b.0 as f64 - 150_000.0);
    if t_us <= start {
        return (a.1, a.2);
    }
    let k = ((t_us - start) / (b.0 as f64 - start).max(1.0)).clamp(0.0, 1.0);
    (a.1 + (b.1 - a.1) * k, a.2 + (b.2 - a.2) * k)
}

/// Where the camera looks through `zoom`, following the mouse calmly: it waits on the first
/// action while zooming in, then moves only when the mouse nears the edge of the zoomed view,
/// gliding there (a critically damped spring with lag `follow_us`).
pub fn follow_path(
    samples: &[(u64, f64, f64)],
    zoom: &ZoomSuggestion,
    follow_us: u64,
) -> Vec<(u64, f32, f32)> {
    if samples.is_empty() || zoom.source_end_us <= zoom.source_start_us {
        return Vec::new();
    }
    let scale = zoom.scale.max(1.0);
    // The mouse may roam this far from the camera's center before it moves.
    let zone = 0.5 / scale * 0.45;
    let omega = 4.0 / (follow_us.max(50_000) as f64 / 1e6);
    let hold_start = zoom.source_start_us + zoom.transition_us;
    let (mut cx, mut cy) = cursor_at(samples, hold_start as f64);
    let (mut vx, mut vy) = (0.0f64, 0.0f64);
    let step_us = 1_000_000.0 / 120.0;
    let dt = step_us / 1e6;
    let mut path = vec![(zoom.source_start_us, cx as f32, cy as f32)];
    let mut t = zoom.source_start_us as f64;
    let mut next_out = zoom.source_start_us + 50_000;
    let end = zoom.source_end_us as f64;
    let toward = |camera: f64, mouse: f64| {
        let offset = mouse - camera;
        if offset > zone {
            mouse - zone
        } else if offset < -zone {
            mouse + zone
        } else {
            camera
        }
    };
    while t < end {
        t += step_us;
        if t >= hold_start as f64 {
            let (mx, my) = cursor_at(samples, t);
            let (tx, ty) = (toward(cx, mx), toward(cy, my));
            vx += (omega * omega * (tx - cx) - 2.0 * omega * vx) * dt;
            vy += (omega * omega * (ty - cy) - 2.0 * omega * vy) * dt;
            cx += vx * dt;
            cy += vy * dt;
        }
        if t as u64 >= next_out || t >= end {
            path.push((
                t.min(end) as u64,
                cx.clamp(0.0, 1.0) as f32,
                cy.clamp(0.0, 1.0) as f32,
            ));
            next_out += 50_000;
        }
    }
    path
}

/// The camera's center on `path` at `t_us`, smoothly between points (Catmull-Rom).
pub fn path_center(path: &[(u64, f32, f32)], t_us: u64) -> Option<(f64, f64)> {
    let first = path.first()?;
    if path.len() == 1 || t_us <= first.0 {
        return Some((first.1 as f64, first.2 as f64));
    }
    let index = path.partition_point(|p| p.0 <= t_us);
    if index >= path.len() {
        let last = path[path.len() - 1];
        return Some((last.1 as f64, last.2 as f64));
    }
    let p1 = path[index - 1];
    let p2 = path[index];
    let p0 = if index >= 2 { path[index - 2] } else { p1 };
    let p3 = path.get(index + 1).copied().unwrap_or(p2);
    let k = (t_us - p1.0) as f64 / (p2.0 - p1.0).max(1) as f64;
    let spline = |a: f32, b: f32, c: f32, d: f32| {
        let (a, b, c, d) = (a as f64, b as f64, c as f64, d as f64);
        0.5 * ((2.0 * b)
            + (-a + c) * k
            + (2.0 * a - 5.0 * b + 4.0 * c - d) * k * k
            + (-a + 3.0 * b - 3.0 * c + d) * k * k * k)
    };
    Some((
        spline(p0.1, p1.1, p2.1, p3.1).clamp(0.0, 1.0),
        spline(p0.2, p1.2, p2.2, p3.2).clamp(0.0, 1.0),
    ))
}

/// The cursor samples of the recording in `folder`, read once while the file is unchanged.
pub fn recording_cursor_samples(folder: &std::path::Path) -> std::sync::Arc<Vec<(u64, f64, f64)>> {
    use std::sync::{Arc, Mutex, OnceLock};
    type Cache = Mutex<
        std::collections::HashMap<
            std::path::PathBuf,
            (Option<std::time::SystemTime>, Arc<Vec<(u64, f64, f64)>>),
        >,
    >;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let modified = std::fs::metadata(folder.join("telemetry").join("events.jsonl"))
        .and_then(|m| m.modified())
        .ok();
    let cache = CACHE.get_or_init(Default::default);
    if let Some((stamp, samples)) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(folder) {
        if *stamp == modified {
            return samples.clone();
        }
    }
    let samples = Arc::new(
        crate::telemetry::reader::read_telemetry(folder)
            .map(|stream| cursor_samples(&stream))
            .unwrap_or_default(),
    );
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(folder.to_path_buf(), (modified, samples.clone()));
    samples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::reader::{CanonicalEvent, CanonicalGeometry, CanonicalKind};
    use crate::timeline::{SourceInterval, TimelineMapper};
    use std::collections::BTreeMap;

    fn geo(id: &str) -> CanonicalGeometry {
        CanonicalGeometry {
            geometry_id: id.into(),
            t_us: 0,
            version: 2,
            supported: true,
            unsupported_reason: None,
            sampling_interval_us: 100_000,
            source_id: Some("display:1".into()),
        }
    }

    fn stream(events: Vec<CanonicalEvent>) -> TelemetryStream {
        let mut geometries = BTreeMap::new();
        geometries.insert("g1".into(), geo("g1"));
        TelemetryStream {
            events,
            geometries,
            diagnostics: Vec::new(),
        }
    }

    fn move_at(seq: u64, t: u64, x: f64, y: f64) -> CanonicalEvent {
        CanonicalEvent {
            version: 2,
            seq,
            t_us: t,
            geometry_id: Some("g1".into()),
            kind: CanonicalKind::Move {
                norm_x: x,
                norm_y: y,
                inside_source: true,
            },
        }
    }

    fn down(seq: u64, t: u64, button: u32, x: f64, y: f64) -> CanonicalEvent {
        CanonicalEvent {
            version: 2,
            seq,
            t_us: t,
            geometry_id: Some("g1".into()),
            kind: CanonicalKind::ButtonDown {
                button: Some(button),
                norm_x: x,
                norm_y: y,
                inside_source: true,
            },
        }
    }

    fn up(seq: u64, t: u64, button: u32, x: f64, y: f64) -> CanonicalEvent {
        CanonicalEvent {
            version: 2,
            seq,
            t_us: t,
            geometry_id: Some("g1".into()),
            kind: CanonicalKind::ButtonUp {
                button: Some(button),
                norm_x: x,
                norm_y: y,
                inside_source: true,
            },
        }
    }

    fn gap(seq: u64, start: u64, end: u64, reason: &str) -> CanonicalEvent {
        CanonicalEvent {
            version: 2,
            seq,
            t_us: end,
            geometry_id: None,
            kind: CanonicalKind::Gap {
                reason: reason.into(),
                start_us: start,
                end_us: end,
                dropped_events: 0,
            },
        }
    }

    #[test]
    fn identical_input_is_deterministic() {
        let config = ZoomConfig::default();
        let data = stream(vec![
            move_at(0, 1_000_000, 0.4, 0.4),
            down(1, 1_100_000, 0, 0.41, 0.39),
            up(2, 1_150_000, 0, 0.41, 0.39),
        ]);
        let a = generate_zoom_suggestions(&data, &config).unwrap();
        let b = generate_zoom_suggestions(&data, &config).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.suggestions.len(), 1);
        assert_eq!(a.suggestions[0].id, "z-1-n1");
        assert_eq!(a.version, ZOOM_GENERATION_VERSION);
    }

    #[test]
    fn empty_denied_and_gapped_telemetry_do_not_fabricate_zooms() {
        let config = ZoomConfig::default();
        let empty = generate_zoom_suggestions(&stream(vec![]), &config).unwrap();
        assert!(empty.suggestions.is_empty());
        let denied = generate_zoom_suggestions(
            &stream(vec![gap(0, 0, 5_000_000, "input_monitoring_unavailable")]),
            &config,
        )
        .unwrap();
        assert!(denied.suggestions.is_empty());
        let revoked = generate_zoom_suggestions(
            &stream(vec![gap(0, 0, 5_000_000, "input_monitoring_revoked")]),
            &config,
        )
        .unwrap();
        assert!(revoked.suggestions.is_empty());
        let gapped = generate_zoom_suggestions(
            &stream(vec![
                down(0, 1_000_000, 0, 0.2, 0.2),
                gap(1, 1_050_000, 4_000_000, "queue_overflow"),
                up(2, 4_100_000, 0, 0.8, 0.8),
            ]),
            &config,
        )
        .unwrap();
        assert_eq!(gapped.suggestions.len(), 1);
        assert!(gapped.suggestions[0].contributing_event_seqs.contains(&0));
        assert!(!gapped.suggestions[0].contributing_event_seqs.contains(&2));
    }

    #[test]
    fn corrupt_interior_gap_does_not_join_dwells() {
        let config = ZoomConfig::default();
        let data = stream(vec![
            move_at(0, 200_000, 0.5, 0.5),
            gap(1, 200_000, 1_200_000, "corrupt_record"),
            move_at(2, 1_200_000, 0.5, 0.5),
        ]);
        let generated = generate_zoom_suggestions(&data, &config).unwrap();
        assert!(
            generated.suggestions.iter().all(|suggestion| {
                suggestion
                    .source_end_us
                    .saturating_sub(suggestion.source_start_us)
                    < 1_000_000
            }),
            "dwell must not span a corrupt telemetry gap: {:?}",
            generated.suggestions
        );
    }

    #[test]
    fn geometry_uncertainty_interval_skips_clicks_inside_poll_window() {
        let config = ZoomConfig::default();
        assert_eq!(config.geometry_uncertainty_us, 100_000);
        let data = stream(vec![
            gap(0, 1_000_000, 1_100_000, "geometry_changed"),
            down(1, 1_050_000, 0, 0.4, 0.4),
            up(2, 1_080_000, 0, 0.4, 0.4),
            down(3, 1_500_000, 0, 0.5, 0.5),
            up(4, 1_540_000, 0, 0.5, 0.5),
        ]);
        let generated = generate_zoom_suggestions(&data, &config).unwrap();
        assert_eq!(generated.suggestions.len(), 1);
        assert_eq!(generated.suggestions[0].contributing_event_seqs, vec![3]);
    }

    #[test]
    fn auxiliary_buttons_are_ignored_and_rapid_clicks_merge() {
        let config = ZoomConfig::default();
        let data = stream(vec![
            down(0, 1_000_000, 4, 0.5, 0.5),
            up(1, 1_050_000, 4, 0.5, 0.5),
            down(2, 2_000_000, 0, 0.30, 0.30),
            up(3, 2_040_000, 0, 0.30, 0.30),
            down(4, 2_200_000, 0, 0.31, 0.29),
            up(5, 2_240_000, 0, 0.31, 0.29),
        ]);
        let generated = generate_zoom_suggestions(&data, &config).unwrap();
        assert_eq!(generated.suggestions.len(), 1);
        assert_eq!(generated.suggestions[0].origin, ZoomOrigin::Click);
        assert_eq!(generated.suggestions[0].contributing_event_seqs, vec![2, 4]);
        assert_eq!(generated.suggestions[0].scale, config.click_scale);
    }

    #[test]
    fn v1_click_is_not_double_counted_with_downs() {
        let config = ZoomConfig::default();
        let mixed = stream(vec![
            CanonicalEvent {
                version: 1,
                seq: 0,
                t_us: 1_000_000,
                geometry_id: Some("g1".into()),
                kind: CanonicalKind::ButtonDown {
                    button: None,
                    norm_x: 0.4,
                    norm_y: 0.4,
                    inside_source: true,
                },
            },
            CanonicalEvent {
                version: 1,
                seq: 1,
                t_us: 1_010_000,
                geometry_id: Some("g1".into()),
                kind: CanonicalKind::Click {
                    norm_x: 0.4,
                    norm_y: 0.4,
                    inside_source: true,
                },
            },
        ]);
        let generated = generate_zoom_suggestions(&mixed, &config).unwrap();
        assert_eq!(generated.suggestions.len(), 1);
        assert_eq!(generated.suggestions[0].contributing_event_seqs, vec![0]);

        let click_only = stream(vec![CanonicalEvent {
            version: 1,
            seq: 7,
            t_us: 2_000_000,
            geometry_id: Some("g1".into()),
            kind: CanonicalKind::Click {
                norm_x: 0.6,
                norm_y: 0.6,
                inside_source: true,
            },
        }]);
        let from_click = generate_zoom_suggestions(&click_only, &config).unwrap();
        assert_eq!(from_click.suggestions.len(), 1);
        assert_eq!(from_click.suggestions[0].contributing_event_seqs, vec![7]);
    }

    #[test]
    fn off_source_and_unsupported_geometry_are_not_clamped_into_targets() {
        let config = ZoomConfig::default();
        let mut data = stream(vec![CanonicalEvent {
            version: 2,
            seq: 0,
            t_us: 1_000_000,
            geometry_id: Some("g1".into()),
            kind: CanonicalKind::ButtonDown {
                button: Some(0),
                norm_x: -0.2,
                norm_y: 1.4,
                inside_source: false,
            },
        }]);
        data.geometries.insert(
            "win".into(),
            CanonicalGeometry {
                geometry_id: "win".into(),
                t_us: 0,
                version: 2,
                supported: false,
                unsupported_reason: Some("window geometry is missing physical transforms".into()),
                sampling_interval_us: 100_000,
                source_id: Some("window:3".into()),
            },
        );
        data.events.push(CanonicalEvent {
            version: 2,
            seq: 1,
            t_us: 2_000_000,
            geometry_id: Some("win".into()),
            kind: CanonicalKind::ButtonDown {
                button: Some(0),
                norm_x: 0.5,
                norm_y: 0.5,
                inside_source: true,
            },
        });
        let generated = generate_zoom_suggestions(&data, &config).unwrap();
        assert!(generated.suggestions.is_empty());
        assert!(generated
            .diagnostics
            .iter()
            .any(|d| d.contains("Off-source")));
    }

    #[test]
    fn dwell_emits_lower_scale_than_clicks() {
        let config = ZoomConfig::default();
        let data = stream(vec![
            move_at(0, 1_000_000, 0.5, 0.5),
            move_at(1, 1_400_000, 0.51, 0.49),
            move_at(2, 1_800_000, 0.50, 0.50),
            // It rests until it moves away at 3.5 s.
            move_at(3, 3_500_000, 0.9, 0.9),
        ]);
        let generated = generate_zoom_suggestions(&data, &config).unwrap();
        assert_eq!(generated.suggestions.len(), 1);
        assert_eq!(generated.suggestions[0].origin, ZoomOrigin::Dwell);
        assert_eq!(generated.suggestions[0].scale, config.dwell_scale);
    }

    #[test]
    fn unreachable_min_dwell_switches_hover_zooms_off() {
        // The Zoom panel's "Hover zoom: Off" sends dwell scale 1 and a one-hour dwell.
        let config = ZoomConfig {
            dwell_scale: 1.0,
            min_dwell_us: 3_600_000_000,
            ..ZoomConfig::default()
        };
        config.validate().unwrap();
        let data = stream(vec![
            move_at(0, 1_000_000, 0.5, 0.5),
            move_at(1, 1_400_000, 0.51, 0.49),
            move_at(2, 1_800_000, 0.50, 0.50),
        ]);
        let generated = generate_zoom_suggestions(&data, &config).unwrap();
        assert!(generated.suggestions.is_empty());
    }

    #[test]
    fn bezier_evaluator_respects_cuts_and_clamps_viewport_not_source() {
        let config = ZoomConfig::default();
        let suggestion = ZoomSuggestion {
            path: Vec::new(),
            id: "z-1-n1".into(),
            source_start_us: 1_000_000,
            source_end_us: 3_000_000,
            center_x: 0.05,
            center_y: 0.05,
            scale: 2.0,
            transition_us: 400_000,
            origin: ZoomOrigin::Click,
            contributing_event_seqs: vec![1],
            edited_ranges: Vec::new(),
            media: None,
        };
        let identity = evaluate_at_source(&[suggestion.clone()], 0, &config);
        assert_eq!(identity.scale, 1.0);
        assert_eq!(identity.center_x, 0.5);

        let hold = evaluate_at_source(&[suggestion.clone()], 1_600_000, &config);
        assert!((hold.scale - 2.0).abs() < 1e-9);
        assert!(hold.center_x > 0.05);
        assert!(hold.center_x >= 0.25 - 1e-9);
        assert!(hold.center_x <= 0.75 + 1e-9);

        let mapper = TimelineMapper::try_new(vec![
            SourceInterval::new("a".into(), 0, 1_200_000),
            SourceInterval::new("b".into(), 2_000_000, 4_000_000),
        ])
        .unwrap();
        let before_cut =
            evaluate_at_edited(&[suggestion.clone()], &mapper, 1_199_999, &config).unwrap();
        let after_cut =
            evaluate_at_edited(&[suggestion.clone()], &mapper, 1_200_000, &config).unwrap();
        let source_before = evaluate_at_source(&[suggestion.clone()], 1_199_999, &config);
        let source_after = evaluate_at_source(&[suggestion], 2_000_000, &config);
        assert_eq!(before_cut, source_before);
        assert_eq!(after_cut, source_after);
        assert_ne!(before_cut.scale, after_cut.scale);
        assert!((cubic_bezier_unit(0.0) - 0.0).abs() < 1e-12);
        assert!((cubic_bezier_unit(1.0) - 1.0).abs() < 1e-12);
        assert!((cubic_bezier_unit(0.5) - 0.5).abs() < 1e-12);

        let identity_uv = CameraTransform::identity().uv_rect();
        assert_eq!(identity_uv, (0.0, 0.0, 1.0, 1.0));
        let hold_uv = hold.uv_rect();
        assert!((hold_uv.0 - (hold.center_x as f32 - hold_uv.2 * 0.5)).abs() < 1e-5);
        assert!((hold_uv.2 - 0.5).abs() < 1e-5);
        assert!((hold_uv.3 - 0.5).abs() < 1e-5);
        let corner_uv = CameraTransform {
            center_x: 0.05,
            center_y: 0.05,
            scale: 2.0,
        }
        .uv_rect();
        assert_eq!(corner_uv.0, 0.0);
        assert_eq!(corner_uv.1, 0.0);
        assert!((corner_uv.2 - 0.5).abs() < 1e-5);
    }

    #[test]
    fn attach_edited_ranges_omits_removed_source_time() {
        let mut generation = ZoomGeneration {
            version: 1,
            config: ZoomConfig::default(),
            suggestions: vec![ZoomSuggestion {
                path: Vec::new(),
                id: "z-1-n1".into(),
                source_start_us: 1_000_000,
                source_end_us: 6_000_000,
                center_x: 0.4,
                center_y: 0.4,
                scale: 2.0,
                transition_us: 400_000,
                origin: ZoomOrigin::Click,
                contributing_event_seqs: vec![1],
                edited_ranges: Vec::new(),
                media: None,
            }],
            diagnostics: Vec::new(),
        };
        let mapper = TimelineMapper::try_new(vec![
            SourceInterval::new("a".into(), 0, 2_000_000),
            SourceInterval::new("b".into(), 5_000_000, 10_000_000),
        ])
        .unwrap();
        attach_edited_ranges(&mut generation, &mapper);
        assert_eq!(
            generation.suggestions[0].edited_ranges,
            vec![
                EditedRange {
                    start_us: 1_000_000,
                    end_us: 2_000_000
                },
                EditedRange {
                    start_us: 2_000_000,
                    end_us: 3_000_000
                }
            ]
        );
    }

    fn window(id: &str, start: u64, end: u64, origin: ZoomOrigin) -> ZoomSuggestion {
        ZoomSuggestion {
            path: Vec::new(),
            id: id.into(),
            source_start_us: start,
            source_end_us: end,
            center_x: 0.5,
            center_y: 0.5,
            scale: 2.0,
            transition_us: 400_000,
            origin,
            contributing_event_seqs: vec![1],
            edited_ranges: Vec::new(),
            media: None,
        }
    }

    fn click(seq: u64, t: u64, x: f64, y: f64) -> Vec<CanonicalEvent> {
        vec![down(seq, t, 0, x, y), up(seq + 1, t + 40_000, 0, x, y)]
    }

    #[test]
    fn activity_close_together_is_one_zoom_that_does_not_overlap_the_next() {
        let config = ZoomConfig::default();
        // Two clicks far apart on screen but 2 s apart in time: one zoom, following the mouse.
        let mut events = click(0, 1_000_000, 0.1, 0.1);
        events.extend(click(2, 3_000_000, 0.9, 0.9));
        // A click 10 s later is a zoom of its own.
        events.extend(click(4, 13_000_000, 0.5, 0.5));
        let generated = generate_zoom_suggestions(&stream(events), &config).unwrap();
        let spans: Vec<_> = generated
            .suggestions
            .iter()
            .map(|z| (z.origin, z.contributing_event_seqs.clone()))
            .collect();
        assert_eq!(spans.len(), 2, "{spans:?}");
        assert!(spans[0].1.contains(&0) && spans[0].1.contains(&2) && !spans[0].1.contains(&4));
        assert!(spans[1].1.contains(&4));
        let pair = &generated.suggestions;
        assert!(pair[0].source_end_us <= pair[1].source_start_us);
        // Every zoom of a kind gets the same amount.
        assert!(pair.iter().all(|z| z.scale == config.click_scale));
    }

    #[test]
    fn only_the_strongest_moments_are_kept() {
        let config = ZoomConfig {
            max_zooms: 3,
            ..ZoomConfig::default()
        };
        let mut events = Vec::new();
        let mut seq = 0;
        for i in 0..8u64 {
            let t = 1_000_000 + i * 10_000_000;
            // Moments 2, 5 and 7 have three clicks; the rest one.
            let clicks = if [2, 5, 7].contains(&i) { 3 } else { 1 };
            for c in 0..clicks {
                events.extend(click(seq, t + c * 300_000, 0.5, 0.5));
                seq += 2;
            }
        }
        let generated = generate_zoom_suggestions(&stream(events), &config).unwrap();
        let starts: Vec<u64> = generated
            .suggestions
            .iter()
            .map(|z| (z.source_start_us + z.transition_us - 1_000_000) / 10_000_000)
            .collect();
        assert_eq!(starts, vec![2, 5, 7]);
    }

    #[test]
    fn a_resting_mouse_is_a_hover_zoom_only_when_hover_zooms_are_on() {
        // The mouse rests from 1 s and only moves off at 4 s: a resting mouse sends nothing.
        let events = vec![
            move_at(0, 1_000_000, 0.5, 0.5),
            move_at(1, 1_100_000, 0.505, 0.5),
            move_at(2, 4_000_000, 0.9, 0.9),
        ];
        let on = ZoomSettings::default().config();
        let generated = generate_zoom_suggestions(&stream(events.clone()), &on).unwrap();
        assert_eq!(generated.suggestions.len(), 1);
        assert_eq!(generated.suggestions[0].origin, ZoomOrigin::Dwell);
        assert_eq!(
            generated.suggestions[0].scale,
            ZoomSettings::default().hover_scale
        );
        let off = ZoomSettings {
            hover_scale: 1.0,
            ..ZoomSettings::default()
        }
        .config();
        assert!(generate_zoom_suggestions(&stream(events), &off)
            .unwrap()
            .suggestions
            .is_empty());
    }

    #[test]
    fn the_camera_waits_on_small_moves_and_glides_after_big_ones() {
        let mut zoom = window("z", 0, 6_000_000, ZoomOrigin::Click);
        zoom.transition_us = 500_000;
        let samples = vec![(0, 0.5, 0.5), (2_000_000, 0.52, 0.5), (3_000_000, 0.9, 0.5)];
        let path = follow_path(&samples, &zoom, 700_000);
        let at = |t| path_center(&path, t).unwrap();
        // A nudge inside the view: the camera stays put.
        assert!((at(2_500_000).0 - 0.5).abs() < 1e-3);
        // A move to the edge: the camera follows, behind the mouse, without overshooting it.
        let later = at(5_000_000).0;
        assert!(later > 0.7 && later < 0.9, "camera at {later}");
        let mut previous = at(3_000_000).0;
        for t in (3_050_000..6_000_000).step_by(50_000) {
            let x = at(t).0;
            assert!(x + 1e-6 >= previous, "camera turned back at {t}");
            previous = x;
        }
    }

    #[test]
    fn zoom_in_is_even_and_never_slides_along_an_edge() {
        let config = ZoomConfig::default();
        // A corner target: the old camera slid along the edge, then turned.
        let mut zoom = window("z", 0, 4_000_000, ZoomOrigin::Click);
        zoom.center_x = 0.95;
        zoom.center_y = 0.1;
        let zooms = [zoom];
        let mut last = evaluate_at_source(&zooms, 0, &config);
        let mut steps = Vec::new();
        for t in (10_000..=400_000).step_by(10_000) {
            let camera = evaluate_at_source(&zooms, t, &config);
            // Equal ratios of zoom for equal ease: log-scale speed peaks mid-way, not at the start.
            steps.push((camera.scale / last.scale).ln());
            let (x0, y0, w0, _) = last.uv_rect();
            let (x1, y1, w1, _) = camera.uv_rect();
            // The frame closes toward one fixed point: each edge moves one way only.
            assert!(x1 >= x0 - 1e-6 && x1 + w1 <= x0 + w0 + 1e-6);
            assert!(y1 >= y0 - 1e-6 && w1 <= w0 + 1e-6);
            last = camera;
        }
        let peak = steps
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        assert!((15..=25).contains(&peak), "fastest step at {peak}");
        assert!((last.scale - 2.0).abs() < 1e-9);
    }
}
