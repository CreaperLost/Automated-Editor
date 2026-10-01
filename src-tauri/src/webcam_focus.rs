//! Auto webcam layout: the webcam grows to fill the canvas while the speaker talks and the
//! screen is idle, and shrinks back to its bubble before the next click, scroll or cursor move.
//!
//! Detection runs once, in source time, and stores plain segments in the edit document, so
//! the user can switch each one off or add their own. Rendering only reads those segments,
//! mapped onto the edited timeline, which keeps preview and export identical.
use crate::telemetry::{CanonicalKind, TelemetryStream};
use crate::timeline::TimelineMapper;
use crate::zoom::EditedRange;
use serde::{Deserialize, Serialize};

pub const MAX_FOCUS_SEGMENTS: usize = 1_024;
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
    1_500
}

fn default_min_focus_ms() -> u32 {
    2_500
}

fn default_transition_ms() -> u32 {
    450
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
    /// How long the mouse must rest after activity before the webcam grows.
    #[serde(default = "default_idle_ms")]
    pub idle_ms: u32,
    /// Shorter talking-while-idle stretches are ignored.
    #[serde(default = "default_min_focus_ms")]
    pub min_focus_ms: u32,
    /// Grow and shrink animation length.
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
        if self.idle_ms > 10_000 {
            return Err("Idle time is out of range".into());
        }
        if !(500..=30_000).contains(&self.min_focus_ms) {
            return Err("Minimum focus length is out of range".into());
        }
        if !(100..=2_000).contains(&self.transition_ms) {
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

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebcamFocus {
    /// Master switch. Off keeps the segments so turning it back on restores them.
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub settings: WebcamFocusSettings,
    #[serde(default)]
    pub segments: Vec<WebcamFocusSegment>,
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
    }

    /// Enabled segments on the edited timeline, sorted and merged. Empty when switched off.
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
        merge_ranges(ranges)
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

/// Source ranges where the speaker talks and the screen is idle.
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
        let settings = WebcamFocusSettings::default();
        let ranges = detect_focus_ranges(&[(0, 30 * S)], &stream(vec![click(10 * S)]), &settings);
        let lead = settings.transition_us() + ACTIVITY_LEAD_US;
        let idle = settings.idle_ms as u64 * 1_000;
        assert_eq!(ranges, vec![(0, 10 * S - lead), (10 * S + idle, 30 * S)]);
    }

    #[test]
    fn short_stretches_and_silence_never_focus() {
        let settings = WebcamFocusSettings::default();
        // Clicks every second leave no idle gap long enough.
        let busy: Vec<_> = (0..10).map(|i| click(i * S)).collect();
        assert!(detect_focus_ranges(&[(0, 10 * S)], &stream(busy), &settings).is_empty());
        // No speech, no focus, however idle the screen is.
        assert!(detect_focus_ranges(&[], &stream(vec![]), &settings).is_empty());
    }

    #[test]
    fn cursor_jitter_is_not_activity_but_real_moves_are() {
        let mut settings = WebcamFocusSettings::default();
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
        let settings = WebcamFocusSettings::default();
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
        let mut bad = parsed.clone();
        bad.settings.transition_ms = 5;
        assert!(bad.validate().is_err());
    }
}
