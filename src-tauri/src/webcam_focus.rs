//! Auto webcam layout: the webcam fills the canvas once the mouse has rested for a set time
//! (optionally only while the speaker talks), and goes back to its bubble before the next
//! click, scroll or cursor move. Clips marked "normal view" never switch.
//!
//! Detection runs once, in source time, and stores plain segments in the edit document, so
//! the user can switch each one off or add their own. Rendering only reads those segments,
//! mapped onto the edited timeline, which keeps preview and export identical.
use crate::telemetry::{CanonicalKind, TelemetryStream};
use crate::timeline::TimelineMapper;
use crate::zoom::EditedRange;
use serde::{Deserialize, Serialize};

pub const MAX_FOCUS_SEGMENTS: usize = 1_024;
pub const MAX_NORMAL_VIEW_RANGES: usize = 1_024;
/// Longest mouse rest the trigger can wait for.
pub const MAX_IDLE_MS: u32 = 60_000;
/// Cursor travel, as a fraction of the screen diagonal, that counts as activity.
/// Smaller moves are hand tremor or a resting mouse being resampled.
const CURSOR_MOVE_EPSILON: f64 = 0.006;
/// Extra time the bubble is back in place before a click, on top of the transition.
const ACTIVITY_LEAD_US: u64 = 150_000;

fn default_speech_threshold_db() -> f32 {
    -38.0
}

fn default_pause_tolerance_ms() -> u32 {
    800
}

fn default_idle_ms() -> u32 {
    30_000
}

fn default_min_focus_ms() -> u32 {
    2_500
}

/// Zero is a hard cut: the webcam switches to full frame instantly.
fn default_transition_ms() -> u32 {
    0
}

fn default_focus_size_pct() -> f32 {
    100.0
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebcamFocusSettings {
    /// Mic level above which audio counts as speech.
    #[serde(default = "default_speech_threshold_db")]
    pub speech_threshold_db: f32,
    /// Pauses in speech shorter than this keep the webcam large.
    #[serde(default = "default_pause_tolerance_ms")]
    pub pause_tolerance_ms: u32,
    /// How long the mouse must rest after activity before the webcam fills the frame.
    #[serde(default = "default_idle_ms")]
    pub idle_ms: u32,
    /// Also require speech: the webcam only fills the frame while the speaker talks.
    #[serde(default = "default_true")]
    pub require_speech: bool,
    /// Shorter talking-while-idle stretches are ignored.
    #[serde(default = "default_min_focus_ms")]
    pub min_focus_ms: u32,
    /// Grow and shrink animation length; zero switches instantly.
    #[serde(default = "default_transition_ms")]
    pub transition_ms: u32,
    /// Treat cursor movement, not only clicks and scrolls, as screen activity.
    #[serde(default = "default_true")]
    pub cursor_moves_are_activity: bool,
    /// Size of the enlarged webcam as a percent of the canvas; 100 fills it edge to edge.
    #[serde(default = "default_focus_size_pct")]
    pub focus_size_pct: f32,
}

impl Default for WebcamFocusSettings {
    fn default() -> Self {
        Self {
            speech_threshold_db: default_speech_threshold_db(),
            pause_tolerance_ms: default_pause_tolerance_ms(),
            idle_ms: default_idle_ms(),
            require_speech: true,
            min_focus_ms: default_min_focus_ms(),
            transition_ms: default_transition_ms(),
            cursor_moves_are_activity: true,
            focus_size_pct: default_focus_size_pct(),
        }
    }
}

impl WebcamFocusSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !self.speech_threshold_db.is_finite()
            || !(-80.0..=-5.0).contains(&self.speech_threshold_db)
        {
            return Err("Speech threshold is out of range".into());
        }
        if !(100..=5_000).contains(&self.pause_tolerance_ms) {
            return Err("Pause tolerance is out of range".into());
        }
        if self.idle_ms > MAX_IDLE_MS {
            return Err("Idle time is out of range".into());
        }
        if !(500..=30_000).contains(&self.min_focus_ms) {
            return Err("Minimum focus length is out of range".into());
        }
        if self.transition_ms > 2_000 {
            return Err("Transition length is out of range".into());
        }
        if !self.focus_size_pct.is_finite() || !(40.0..=100.0).contains(&self.focus_size_pct) {
            return Err("Focus size is out of range".into());
        }
        Ok(())
    }

    pub fn transition_us(&self) -> u64 {
        self.transition_ms as u64 * 1_000
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FocusSegmentSource {
    Auto,
    Manual,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebcamFocusSegment {
    pub id: String,
    pub source_start_us: u64,
    pub source_end_us: u64,
    pub source: FocusSegmentSource,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Where the segment lands on the edited timeline. Filled in for the UI, never stored.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edited_ranges: Vec<EditedRange>,
}

/// A source range (usually a clip) where the webcam stays in its bubble whatever the
/// segments say.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NormalViewRange {
    pub source_start_us: u64,
    pub source_end_us: u64,
    /// Where the range lands on the edited timeline. Filled in for the UI, never stored.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edited_ranges: Vec<EditedRange>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebcamFocus {
    /// Master switch. Off keeps the segments so turning it back on restores them.
    #[serde(default)]
    pub enabled: bool,
    /// The recording whose camera the segments are in (its own time); `None` is the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    #[serde(default)]
    pub settings: WebcamFocusSettings,
    #[serde(default)]
    pub segments: Vec<WebcamFocusSegment>,
    /// Ranges kept in normal view (webcam bubble), e.g. clips the user switched back.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub normal_view: Vec<NormalViewRange>,
}

impl WebcamFocus {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        self.settings.validate()?;
        if self.segments.len() > MAX_FOCUS_SEGMENTS {
            return Err("Too many webcam focus segments".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for segment in &self.segments {
            if segment.id.is_empty() || segment.id.len() > 128 || !seen.insert(&segment.id) {
                return Err("Invalid webcam focus segment id".into());
            }
            if segment.source_end_us <= segment.source_start_us {
                return Err("Webcam focus segment must be a half-open source range".into());
            }
        }
        if self.normal_view.len() > MAX_NORMAL_VIEW_RANGES {
            return Err("Too many normal-view ranges".into());
        }
        if self
            .normal_view
            .iter()
            .any(|range| range.source_end_us <= range.source_start_us)
        {
            return Err("Normal-view range must be a half-open source range".into());
        }
        Ok(())
    }

    /// Drops UI-only fields and orders segments, so equal edits compare equal.
    pub fn normalized(mut self) -> Self {
        for segment in &mut self.segments {
            segment.edited_ranges.clear();
        }
        self.segments.sort_by(|a, b| {
            a.source_start_us
                .cmp(&b.source_start_us)
                .then(a.id.cmp(&b.id))
        });
        let ranges = self
            .normal_view
            .iter()
            .map(|range| (range.source_start_us, range.source_end_us))
            .collect();
        self.normal_view = merge_ranges(ranges)
            .into_iter()
            .map(|(source_start_us, source_end_us)| NormalViewRange {
                source_start_us,
                source_end_us,
                edited_ranges: Vec::new(),
            })
            .collect();
        self
    }

    pub fn attach_edited_ranges(&mut self, mapper: &TimelineMapper) {
        for segment in &mut self.segments {
            segment.edited_ranges = mapper
                .source_range_to_edited(segment.source_start_us, segment.source_end_us)
                .into_iter()
                .map(|(start_us, end_us)| EditedRange { start_us, end_us })
                .collect();
        }
        for range in &mut self.normal_view {
            range.edited_ranges = mapper
                .source_range_to_edited(range.source_start_us, range.source_end_us)
                .into_iter()
                .map(|(start_us, end_us)| EditedRange { start_us, end_us })
                .collect();
        }
    }

    /// Enabled segments on the edited timeline minus the normal-view ranges, sorted and
    /// merged. Empty when switched off.
    pub fn edited_ranges(&self, mapper: &TimelineMapper) -> Vec<(u64, u64)> {
        if !self.enabled {
            return Vec::new();
        }
        let ranges = self
            .segments
            .iter()
            .filter(|segment| segment.enabled)
            .flat_map(|segment| {
                mapper.source_range_to_edited(segment.source_start_us, segment.source_end_us)
            })
            .collect();
        let normal = self
            .normal_view
            .iter()
            .flat_map(|range| {
                mapper.source_range_to_edited(range.source_start_us, range.source_end_us)
            })
            .collect();
        subtract_ranges(&merge_ranges(ranges), &merge_ranges(normal))
    }

    /// Makes the webcam fill the frame over the source `pieces`, without stacking: parts
    /// already covered by an enabled segment are left alone, new manual focus merges with the
    /// manual focus it touches, and switched-off segments give way.
    pub fn add_focus(&mut self, pieces: &[(u64, u64)]) {
        let pieces = merge_ranges(pieces.to_vec());
        let covered_auto = merge_ranges(
            self.segments
                .iter()
                .filter(|s| s.enabled && s.source == FocusSegmentSource::Auto)
                .map(|s| (s.source_start_us, s.source_end_us))
                .collect(),
        );
        let mut manual: Vec<(u64, u64)> = self
            .segments
            .iter()
            .filter(|s| s.enabled && s.source == FocusSegmentSource::Manual)
            .map(|s| (s.source_start_us, s.source_end_us))
            .collect();
        manual.extend(subtract_ranges(&pieces, &covered_auto));
        let mut kept: Vec<WebcamFocusSegment> = Vec::new();
        for segment in std::mem::take(&mut self.segments) {
            match (segment.enabled, segment.source) {
                (true, FocusSegmentSource::Auto) => kept.push(segment),
                (true, FocusSegmentSource::Manual) => {}
                (false, _) => kept.extend(trimmed(segment, &pieces)),
            }
        }
        self.segments = kept;
        for (start, end) in merge_ranges(manual) {
            let id = self.unique_id(&format!("manual-{start}"));
            self.segments.push(WebcamFocusSegment {
                id,
                source_start_us: start,
                source_end_us: end,
                source: FocusSegmentSource::Manual,
                enabled: true,
                edited_ranges: Vec::new(),
            });
        }
        *self = std::mem::take(self).normalized();
    }

    /// Takes the source `pieces` out of every segment, auto or manual, so the webcam stays in
    /// its bubble there. Re-detecting can find auto focus there again.
    pub fn remove_focus(&mut self, pieces: &[(u64, u64)]) {
        let pieces = merge_ranges(pieces.to_vec());
        let segments = std::mem::take(&mut self.segments);
        for segment in segments {
            for mut piece in trimmed(segment, &pieces) {
                piece.id = self.unique_id(&piece.id);
                self.segments.push(piece);
            }
        }
        *self = std::mem::take(self).normalized();
    }

    fn unique_id(&self, base: &str) -> String {
        let mut id = base.to_string();
        let mut n = 1;
        while self.segments.iter().any(|s| s.id == id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }

    /// Replaces the auto-detected segments, keeping manual ones and any auto segment the
    /// user switched off that is detected again.
    pub fn replace_auto_segments(&mut self, detected: &[(u64, u64)]) {
        let switched_off: std::collections::BTreeSet<String> = self
            .segments
            .iter()
            .filter(|s| s.source == FocusSegmentSource::Auto && !s.enabled)
            .map(|s| s.id.clone())
            .collect();
        self.segments
            .retain(|s| s.source == FocusSegmentSource::Manual);
        let room = MAX_FOCUS_SEGMENTS.saturating_sub(self.segments.len());
        for &(start, end) in detected.iter().take(room) {
            let id = format!("auto-{start}");
            if self.segments.iter().any(|s| s.id == id) {
                continue;
            }
            self.segments.push(WebcamFocusSegment {
                enabled: !switched_off.contains(&id),
                id,
                source_start_us: start,
                source_end_us: end,
                source: FocusSegmentSource::Auto,
                edited_ranges: Vec::new(),
            });
        }
        *self = std::mem::take(self).normalized();
    }
}

/// How far the webcam has grown towards its focus size at `edited_us`, from 0 (bubble) to 1.
/// Each range eases in over its first `transition_us` and out over its last, so the webcam is
/// back in its bubble by the end of the range.
pub fn focus_weight(ranges: &[(u64, u64)], edited_us: u64, transition_us: u64) -> f32 {
    let index = ranges.partition_point(|&(_, end)| end <= edited_us);
    let Some(&(start, end)) = ranges.get(index) else {
        return 0.0;
    };
    if edited_us < start {
        return 0.0;
    }
    let transition = transition_us.max(1) as f64;
    let rise = (edited_us - start) as f64 / transition;
    let fall = (end - edited_us) as f64 / transition;
    crate::zoom::cubic_bezier_unit(rise.min(fall)) as f32
}

/// Speech is the audio that was read minus the silence found in it.
pub fn speech_ranges(covered: &[(u64, u64)], silent: &[(u64, u64)]) -> Vec<(u64, u64)> {
    subtract_ranges(
        &merge_ranges(covered.to_vec()),
        &merge_ranges(silent.to_vec()),
    )
}

/// Source ranges inside `speech` where the screen is idle. Without the speech requirement,
/// pass the whole recording as `speech`.
pub fn detect_focus_ranges(
    speech: &[(u64, u64)],
    telemetry: &TelemetryStream,
    settings: &WebcamFocusSettings,
) -> Vec<(u64, u64)> {
    let lead = settings.transition_us() + ACTIVITY_LEAD_US;
    let idle = settings.idle_ms as u64 * 1_000;
    let mut busy = Vec::new();
    let mut last_move: Option<(f64, f64)> = None;
    for event in &telemetry.events {
        let t = event.t_us;
        let active = match &event.kind {
            CanonicalKind::Move {
                norm_x,
                norm_y,
                inside_source,
            } => {
                let moved = match last_move {
                    Some((x, y)) => (norm_x - x).hypot(norm_y - y) > CURSOR_MOVE_EPSILON,
                    None => false,
                };
                if moved || last_move.is_none() {
                    last_move = Some((*norm_x, *norm_y));
                }
                settings.cursor_moves_are_activity && moved && *inside_source
            }
            CanonicalKind::ButtonDown { .. }
            | CanonicalKind::ButtonUp { .. }
            | CanonicalKind::Click { .. }
            | CanonicalKind::Scroll { .. } => true,
            CanonicalKind::Note => false,
            CanonicalKind::Gap {
                start_us, end_us, ..
            } => {
                // Nothing is known about the screen during a gap, so never go full screen there.
                busy.push((start_us.saturating_sub(lead), end_us.saturating_add(idle)));
                false
            }
        };
        if active {
            busy.push((t.saturating_sub(lead), t.saturating_add(idle)));
        }
    }
    let min_focus = settings.min_focus_ms as u64 * 1_000;
    subtract_ranges(&merge_ranges(speech.to_vec()), &merge_ranges(busy))
        .into_iter()
        .filter(|(start, end)| end - start >= min_focus)
        .take(MAX_FOCUS_SEGMENTS)
        .collect()
}

/// What is left of `segment` outside `remove`. A piece that keeps the segment's start keeps
/// its id; later pieces are named after their own start.
fn trimmed(segment: WebcamFocusSegment, remove: &[(u64, u64)]) -> Vec<WebcamFocusSegment> {
    let prefix = match segment.source {
        FocusSegmentSource::Auto => "auto",
        FocusSegmentSource::Manual => "manual",
    };
    subtract_ranges(&[(segment.source_start_us, segment.source_end_us)], remove)
        .into_iter()
        .map(|(start, end)| WebcamFocusSegment {
            id: if start == segment.source_start_us {
                segment.id.clone()
            } else {
                format!("{prefix}-{start}")
            },
            source_start_us: start,
            source_end_us: end,
            source: segment.source,
            enabled: segment.enabled,
            edited_ranges: Vec::new(),
        })
        .collect()
}

fn merge_ranges(mut ranges: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    ranges.retain(|(start, end)| end > start);
    ranges.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// `keep` minus `remove`; both sorted and non-overlapping.
fn subtract_ranges(keep: &[(u64, u64)], remove: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut r = 0;
    for &(start, end) in keep {
        let mut cursor = start;
        while r < remove.len() && remove[r].1 <= cursor {
            r += 1;
        }
        let mut i = r;
        while i < remove.len() && remove[i].0 < end {
            let (cut_start, cut_end) = remove[i];
            if cut_start > cursor {
                out.push((cursor, cut_start));
            }
            cursor = cursor.max(cut_end);
            if cursor >= end {
                break;
            }
            i += 1;
        }
        if cursor < end {
            out.push((cursor, end));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::CanonicalEvent;
    use crate::timeline::SourceInterval;
    use std::collections::BTreeMap;

    /// Detection with a five-second idle wait, so a few seconds of rest are enough.
    fn quick() -> WebcamFocusSettings {
        WebcamFocusSettings {
            idle_ms: 5_000,
            ..WebcamFocusSettings::default()
        }
    }

    fn stream(events: Vec<(u64, CanonicalKind)>) -> TelemetryStream {
        TelemetryStream {
            events: events
                .into_iter()
                .enumerate()
                .map(|(seq, (t_us, kind))| CanonicalEvent {
                    version: 2,
                    seq: seq as u64,
                    t_us,
                    geometry_id: None,
                    kind,
                })
                .collect(),
            geometries: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn click(t_us: u64) -> (u64, CanonicalKind) {
        (
            t_us,
            CanonicalKind::ButtonDown {
                button: Some(0),
                norm_x: 0.5,
                norm_y: 0.5,
                inside_source: true,
            },
        )
    }

    fn mv(t_us: u64, x: f64) -> (u64, CanonicalKind) {
        (
            t_us,
            CanonicalKind::Move {
                norm_x: x,
                norm_y: 0.5,
                inside_source: true,
            },
        )
    }

    const S: u64 = 1_000_000;

    #[test]
    fn speech_is_covered_audio_minus_silence() {
        let speech = speech_ranges(
            &[(0, 10 * S), (20 * S, 30 * S)],
            &[(2 * S, 4 * S), (8 * S, 22 * S)],
        );
        assert_eq!(speech, vec![(0, 2 * S), (4 * S, 8 * S), (22 * S, 30 * S)]);
    }

    #[test]
    fn clicks_carve_activity_out_of_speech_with_lead_and_idle() {
        let settings = quick();
        let ranges = detect_focus_ranges(&[(0, 30 * S)], &stream(vec![click(10 * S)]), &settings);
        let lead = settings.transition_us() + ACTIVITY_LEAD_US;
        let idle = settings.idle_ms as u64 * 1_000;
        assert_eq!(ranges, vec![(0, 10 * S - lead), (10 * S + idle, 30 * S)]);
    }

    #[test]
    fn short_stretches_and_silence_never_focus() {
        let settings = quick();
        // Clicks every second leave no idle gap long enough.
        let busy: Vec<_> = (0..10).map(|i| click(i * S)).collect();
        assert!(detect_focus_ranges(&[(0, 10 * S)], &stream(busy), &settings).is_empty());
        // No speech, no focus, however idle the screen is.
        assert!(detect_focus_ranges(&[], &stream(vec![]), &settings).is_empty());
    }

    #[test]
    fn cursor_jitter_is_not_activity_but_real_moves_are() {
        let mut settings = quick();
        let jitter = stream(vec![mv(1 * S, 0.5), mv(5 * S, 0.501), mv(9 * S, 0.5)]);
        assert_eq!(
            detect_focus_ranges(&[(0, 20 * S)], &jitter, &settings),
            vec![(0, 20 * S)]
        );
        let moving = stream(vec![mv(1 * S, 0.2), mv(10 * S, 0.8)]);
        assert_eq!(
            detect_focus_ranges(&[(0, 20 * S)], &moving, &settings).len(),
            2
        );
        settings.cursor_moves_are_activity = false;
        assert_eq!(
            detect_focus_ranges(&[(0, 20 * S)], &moving, &settings),
            vec![(0, 20 * S)]
        );
    }

    #[test]
    fn telemetry_gaps_block_focus() {
        let settings = quick();
        let gap = stream(vec![(
            10 * S,
            CanonicalKind::Gap {
                reason: "lost".into(),
                start_us: 10 * S,
                end_us: 12 * S,
                dropped_events: 0,
            },
        )]);
        let ranges = detect_focus_ranges(&[(0, 30 * S)], &gap, &settings);
        assert_eq!(ranges.len(), 2);
        assert!(ranges.iter().all(|&(a, b)| b <= 10 * S || a >= 12 * S));
    }

    #[test]
    fn weight_eases_in_and_out_and_is_zero_outside() {
        let ranges = [(2 * S, 6 * S)];
        let t = 500_000;
        assert_eq!(focus_weight(&ranges, 0, t), 0.0);
        assert_eq!(focus_weight(&ranges, 2 * S, t), 0.0);
        assert!((focus_weight(&ranges, 2 * S + 250_000, t) - 0.5).abs() < 1e-6);
        assert_eq!(focus_weight(&ranges, 4 * S, t), 1.0);
        assert!(focus_weight(&ranges, 6 * S - 100_000, t) < 0.2);
        assert_eq!(focus_weight(&ranges, 6 * S, t), 0.0);
        // Monotonic on the way in.
        let samples: Vec<f32> = (0..=10)
            .map(|i| focus_weight(&ranges, 2 * S + i * 50_000, t))
            .collect();
        assert!(samples.windows(2).all(|w| w[1] >= w[0]));
    }

    #[test]
    fn edited_ranges_follow_cuts_and_skip_disabled_segments() {
        let mapper = TimelineMapper::try_new(vec![
            SourceInterval::new("a".into(), 0, 10 * S),
            SourceInterval::new("b".into(), 20 * S, 30 * S),
        ])
        .unwrap();
        let mut focus = WebcamFocus {
            enabled: true,
            ..Default::default()
        };
        focus.replace_auto_segments(&[(5 * S, 25 * S), (26 * S, 28 * S)]);
        // The cut joins the two halves of the first segment, and it touches the second.
        assert_eq!(
            focus.edited_ranges(&mapper),
            vec![(5 * S, 15 * S), (16 * S, 18 * S)]
        );
        focus.segments[1].enabled = false;
        assert_eq!(focus.edited_ranges(&mapper), vec![(5 * S, 15 * S)]);
        focus.enabled = false;
        assert!(focus.edited_ranges(&mapper).is_empty());
    }

    #[test]
    fn manual_focus_never_stacks_and_can_be_removed() {
        let mut focus = WebcamFocus::default();
        focus.replace_auto_segments(&[(10 * S, 20 * S)]);
        focus.add_focus(&[(2 * S, 5 * S)]);
        // Adding the same range again, or one inside it, changes nothing.
        let once = focus.clone();
        focus.add_focus(&[(2 * S, 5 * S)]);
        focus.add_focus(&[(3 * S, 4 * S)]);
        assert_eq!(focus, once);
        // Overlapping focus merges; the part already covered by auto focus is not doubled.
        focus.add_focus(&[(4 * S, 12 * S)]);
        let ranges = |focus: &WebcamFocus| {
            focus
                .segments
                .iter()
                .map(|s| (s.source, s.source_start_us, s.source_end_us, s.enabled))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ranges(&focus),
            vec![
                (FocusSegmentSource::Manual, 2 * S, 10 * S, true),
                (FocusSegmentSource::Auto, 10 * S, 20 * S, true),
            ]
        );
        // Removing a range splits whatever covers it, auto or manual.
        focus.remove_focus(&[(8 * S, 15 * S)]);
        assert_eq!(
            ranges(&focus),
            vec![
                (FocusSegmentSource::Manual, 2 * S, 8 * S, true),
                (FocusSegmentSource::Auto, 15 * S, 20 * S, true),
            ]
        );
        let ids: std::collections::BTreeSet<_> = focus.segments.iter().map(|s| &s.id).collect();
        assert_eq!(ids.len(), focus.segments.len());
        // A switched-off segment gives way to new focus instead of sitting under it.
        focus.segments[1].enabled = false;
        focus.add_focus(&[(16 * S, 18 * S)]);
        assert_eq!(
            ranges(&focus),
            vec![
                (FocusSegmentSource::Manual, 2 * S, 8 * S, true),
                (FocusSegmentSource::Auto, 15 * S, 16 * S, false),
                (FocusSegmentSource::Manual, 16 * S, 18 * S, true),
                (FocusSegmentSource::Auto, 18 * S, 20 * S, false),
            ]
        );
        focus.validate().unwrap();
    }

    #[test]
    fn redetect_keeps_manual_segments_and_switched_off_auto_ones() {
        let mut focus = WebcamFocus::default();
        focus.replace_auto_segments(&[(S, 2 * S), (5 * S, 9 * S)]);
        focus.segments[0].enabled = false;
        focus.segments.push(WebcamFocusSegment {
            id: "manual-1".into(),
            source_start_us: 3 * S,
            source_end_us: 4 * S,
            source: FocusSegmentSource::Manual,
            enabled: true,
            edited_ranges: Vec::new(),
        });
        focus.replace_auto_segments(&[(S, 2 * S), (6 * S, 9 * S)]);
        let ids: Vec<_> = focus
            .segments
            .iter()
            .map(|s| (s.id.as_str(), s.enabled))
            .collect();
        assert_eq!(
            ids,
            vec![
                ("auto-1000000", false),
                ("manual-1", true),
                ("auto-6000000", true)
            ]
        );
        focus.validate().unwrap();
    }

    #[test]
    fn settings_round_trip_with_defaults() {
        let parsed: WebcamFocus = serde_json::from_str(r#"{"enabled":true}"#).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.settings, WebcamFocusSettings::default());
        parsed.validate().unwrap();
        // Zero is an instant cut, and the idle trigger reaches a minute.
        let mut ok = parsed.clone();
        ok.settings.transition_ms = 0;
        ok.settings.idle_ms = MAX_IDLE_MS;
        ok.validate().unwrap();
        let mut bad = parsed.clone();
        bad.settings.transition_ms = 2_001;
        assert!(bad.validate().is_err());
        let mut bad = parsed.clone();
        bad.settings.idle_ms = MAX_IDLE_MS + 1;
        assert!(bad.validate().is_err());
        // Missing fields take the defaults: only while talking, after 30 s of rest. An empty
        // normal view is left out when saved.
        assert!(parsed.settings.require_speech);
        assert_eq!(parsed.settings.idle_ms, 30_000);
        assert!(parsed.normal_view.is_empty());
        assert!(!serde_json::to_string(&parsed)
            .unwrap()
            .contains("normalView"));
    }

    #[test]
    fn zero_transition_switches_instantly() {
        let ranges = [(10 * S, 20 * S)];
        assert_eq!(focus_weight(&ranges, 10 * S - 1, 0), 0.0);
        assert_eq!(focus_weight(&ranges, 10 * S + 1, 0), 1.0);
        assert_eq!(focus_weight(&ranges, 20 * S - 1, 0), 1.0);
        assert_eq!(focus_weight(&ranges, 20 * S, 0), 0.0);
    }

    #[test]
    fn mouse_rest_alone_triggers_full_frame_after_the_idle_time() {
        // The whole recording is a candidate when speech is not required.
        let mut settings = WebcamFocusSettings::default();
        settings.idle_ms = 10_000;
        let telemetry = stream(vec![click(1 * S), click(30 * S)]);
        let ranges = detect_focus_ranges(&[(0, 60 * S)], &telemetry, &settings);
        let lead = settings.transition_us() + ACTIVITY_LEAD_US;
        // Full frame from ten seconds after the last click until just before the next one.
        assert_eq!(ranges, vec![(11 * S, 30 * S - lead), (40 * S, 60 * S)]);
    }

    #[test]
    fn normal_view_ranges_keep_the_bubble() {
        let mapper = TimelineMapper::new(vec![SourceInterval::new("a".into(), 0, 30 * S)]);
        let mut focus = WebcamFocus {
            enabled: true,
            ..WebcamFocus::default()
        };
        focus.replace_auto_segments(&[(5 * S, 25 * S)]);
        focus.normal_view = vec![
            NormalViewRange {
                source_start_us: 12 * S,
                source_end_us: 15 * S,
                edited_ranges: Vec::new(),
            },
            NormalViewRange {
                source_start_us: 10 * S,
                source_end_us: 13 * S,
                edited_ranges: Vec::new(),
            },
        ];
        focus.validate().unwrap();
        assert_eq!(
            focus.edited_ranges(&mapper),
            vec![(5 * S, 10 * S), (15 * S, 25 * S)]
        );
        // Overlapping ranges merge when the document is normalized.
        let normalized = focus.normalized();
        assert_eq!(normalized.normal_view.len(), 1);
        assert_eq!(
            (
                normalized.normal_view[0].source_start_us,
                normalized.normal_view[0].source_end_us
            ),
            (10 * S, 15 * S)
        );
    }
}
