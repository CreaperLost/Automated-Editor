//! Picking layout sections from what the recording knows, without a model:
//!
//! - **rest:** camera only while the speaker talks and the mouse rests (what webcam focus was);
//! - **busy:** screen only while clicks, scrolls and typing come thick and fast;
//! - **hook:** camera only for the opening line (a short's).
//!
//! The screen wins where both want the frame. Picked sections are anchored in the recording's
//! time and replace the earlier picks; sections made by hand stay
//! ([`super::LayoutTrack::replace_auto`]).
use super::{merge, subtract, CameraOnly, Layout, LayoutSection, ScreenOnly};
use crate::telemetry::{CanonicalKind, TelemetryStream};
use serde::{Deserialize, Serialize};

/// Longest mouse rest the rest rule can wait for.
pub const MAX_IDLE_MS: u32 = 60_000;
/// Cursor travel, as a fraction of the screen diagonal, that counts as activity. Smaller moves
/// are hand tremor or a resting mouse being resampled.
const CURSOR_MOVE_EPSILON: f64 = 0.006;
/// Extra time the screen is back before a click, on top of the move.
const ACTIVITY_LEAD_US: u64 = 150_000;
/// The opening line of a short shows the speaker this long, before the screen takes over.
const HOOK_US: u64 = 2_500_000;
/// Activity closer than this belongs to one busy stretch of the screen.
const BUSY_GAP_US: u64 = 1_500_000;
/// A busy stretch needs this many clicks or scrolls over at least this long.
const BUSY_EVENTS: usize = 4;
const BUSY_MIN_US: u64 = 2_500_000;
/// Typing counts as one action per this much of it.
const TYPING_STEP_US: u64 = 1_000_000;
/// The screen shows a little before the first click and stays a little after the last.
const BUSY_LEAD_US: u64 = 400_000;
const BUSY_TAIL_US: u64 = 1_000_000;
/// Picks of one kind this close together become one.
const MERGE_GAP_US: u64 = 1_500_000;
/// A camera and a screen pick this close together meet, with the base never shown between.
const MEET_GAP_US: u64 = 1_000_000;
/// Picks shorter than this flicker.
const MIN_PICK_US: u64 = 1_500_000;

fn default_speech_threshold_db() -> f32 {
    -38.0
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PickSettings {
    /// Camera only while the speaker talks and the mouse rests.
    pub rest: bool,
    /// Screen only while clicks, scrolls and typing come thick and fast.
    pub busy: bool,
    /// Camera only for the opening line.
    pub hook: bool,
    /// Mic level above which audio counts as speech.
    pub speech_threshold_db: f32,
    /// Pauses in speech shorter than this keep the camera.
    pub pause_tolerance_ms: u32,
    /// How long the mouse must rest after activity before the camera takes the frame.
    pub idle_ms: u32,
    /// The camera takes the frame only while the speaker talks.
    pub require_speech: bool,
    /// Shorter talking-while-idle stretches are left alone.
    pub min_rest_ms: u32,
    /// Cursor movement counts as screen activity, not only clicks and scrolls.
    pub cursor_moves_are_activity: bool,
    /// The camera's size while it takes the frame, percent of the canvas (40 to 100).
    pub rest_size_pct: f32,
}

impl Default for PickSettings {
    fn default() -> Self {
        Self {
            rest: true,
            busy: false,
            hook: false,
            speech_threshold_db: default_speech_threshold_db(),
            pause_tolerance_ms: 800,
            idle_ms: 30_000,
            require_speech: true,
            min_rest_ms: 2_500,
            cursor_moves_are_activity: true,
            rest_size_pct: 100.0,
        }
    }
}

impl PickSettings {
    /// A short moves faster: a shorter rest, the busy screen and the opening line.
    pub fn for_short() -> Self {
        Self {
            busy: true,
            hook: true,
            idle_ms: 8_000,
            min_rest_ms: 3_000,
            ..Self::default()
        }
    }

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
        if !(500..=30_000).contains(&self.min_rest_ms) {
            return Err("Minimum rest length is out of range".into());
        }
        if !self.rest_size_pct.is_finite()
            || !(super::MIN_CAMERA_ONLY_PCT..=100.0).contains(&self.rest_size_pct)
        {
            return Err("The camera's size is 40 to 100 %".into());
        }
        Ok(())
    }
}

/// What picking reads, all in the recording's time.
pub struct PickInput<'a> {
    /// The stretch to pick in.
    pub start_us: u64,
    pub end_us: u64,
    /// The asset the picks are anchored on.
    pub media: Option<String>,
    pub telemetry: &'a TelemetryStream,
    /// Speech: talk the camera may take the frame for. Pass the whole stretch when speech is
    /// not required.
    pub speech: &'a [(u64, u64)],
    /// How long a move takes, so the screen is back before a click.
    pub move_us: u64,
}

/// Auto sections for `input` under `settings`.
pub fn pick(input: &PickInput, settings: &PickSettings) -> Vec<LayoutSection> {
    let (start, end) = (input.start_us, input.end_us);
    if end <= start {
        return Vec::new();
    }
    let clip = |(a, b): (u64, u64)| (a.max(start), b.min(end));
    let screen: Vec<(u64, u64)> = if settings.busy {
        busy_ranges(input.telemetry, start, end)
            .into_iter()
            .map(clip)
            .collect()
    } else {
        Vec::new()
    };
    let mut camera: Vec<(u64, u64)> = Vec::new();
    if settings.rest {
        let speech: Vec<(u64, u64)> = input
            .speech
            .iter()
            .map(|&r| clip(r))
            .filter(|(a, b)| b > a)
            .collect();
        camera.extend(rest_ranges(
            &speech,
            input.telemetry,
            settings,
            input.move_us,
        ));
    }
    if settings.hook {
        // The opening line, unless the screen is busy from the start.
        let hook_end = (start + HOOK_US).min(end);
        let busy_soon = screen.first().is_some_and(|s| s.0 < start + MIN_PICK_US);
        if !busy_soon {
            camera.push((
                start,
                screen.first().map_or(hook_end, |s| hook_end.min(s.0)),
            ));
        }
    }
    let screen = merge(screen, MERGE_GAP_US);
    let camera = subtract(&merge(camera, MERGE_GAP_US), &screen);
    let camera_layout = Layout::CameraOnly(CameraOnly {
        size_pct: settings.rest_size_pct,
        punch_in: 1.0,
    });
    let mut picked: Vec<(u64, u64, Layout, &str)> = Vec::new();
    for &(a, b) in screen.iter().filter(|(a, b)| b - a >= MIN_PICK_US) {
        picked.push((a, b, Layout::ScreenOnly(ScreenOnly::default()), "busy"));
    }
    for &(a, b) in camera.iter().filter(|(a, b)| b - a >= MIN_PICK_US) {
        let rule = if settings.hook && a == start {
            "hook"
        } else {
            "rest"
        };
        picked.push((a, b, camera_layout.clone(), rule));
    }
    picked.sort_by_key(|p| p.0);
    // A moment of the base between a camera and a screen pick would only flicker: they meet.
    for i in 1..picked.len() {
        let next_start = picked[i].0;
        let previous = &mut picked[i - 1];
        if next_start > previous.1 && next_start - previous.1 < MEET_GAP_US {
            previous.1 = next_start;
        }
    }
    picked
        .into_iter()
        .map(|(a, b, layout, rule)| LayoutSection {
            auto: true,
            rule: Some(rule.into()),
            ..LayoutSection::new(
                format!("auto-{}", a / 1000),
                input.media.clone(),
                a,
                b,
                Some(layout),
            )
        })
        .collect()
}

/// Speech is the audio that was read minus the silence found in it.
pub fn speech_ranges(covered: &[(u64, u64)], silent: &[(u64, u64)]) -> Vec<(u64, u64)> {
    subtract(&merge(covered.to_vec(), 0), silent)
}

/// Spoken words (start, end) as stretches of speech: words this close together are one.
pub fn speech_from_words(words: &[(u64, u64)]) -> Vec<(u64, u64)> {
    merge(words.to_vec(), 800_000)
}

/// Stretches inside `speech` where the screen is idle: no click, scroll, typing or (when it
/// counts) cursor move for `idle_ms`, ending `move_us` and a little more before the next.
pub fn rest_ranges(
    speech: &[(u64, u64)],
    telemetry: &TelemetryStream,
    settings: &PickSettings,
    move_us: u64,
) -> Vec<(u64, u64)> {
    let lead = move_us + ACTIVITY_LEAD_US;
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
                // Nothing is known about the screen during a gap, so never leave it there.
                busy.push((start_us.saturating_sub(lead), end_us.saturating_add(idle)));
                false
            }
        };
        if active {
            busy.push((t.saturating_sub(lead), t.saturating_add(idle)));
        }
    }
    // Typing with the mouse at rest is still working on the screen.
    if let Some(keys) = &telemetry.keys {
        for (start, end) in keys.activity() {
            busy.push((start.saturating_sub(lead), end.saturating_add(idle)));
        }
    }
    let min = settings.min_rest_ms as u64 * 1_000;
    subtract(&merge(speech.to_vec(), 0), &busy)
        .into_iter()
        .filter(|(start, end)| end - start >= min)
        .take(super::MAX_SECTIONS)
        .collect()
}

/// Busy stretches of the screen within `[start, end)`: clicks, scrolls and typing close
/// together, a little before the first and after the last.
fn busy_ranges(telemetry: &TelemetryStream, start: u64, end: u64) -> Vec<(u64, u64)> {
    let mut activity: Vec<u64> = telemetry
        .events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                CanonicalKind::ButtonDown { .. }
                    | CanonicalKind::Click { .. }
                    | CanonicalKind::Scroll { .. }
            )
        })
        .map(|e| e.t_us)
        // Typing counts like a stream of clicks: a moment each second it lasts, and each
        // shortcut.
        .chain(telemetry.keys.iter().flat_map(|keys| {
            keys.typing
                .iter()
                .flat_map(|run| {
                    (run.start_us..run.end_us)
                        .step_by(TYPING_STEP_US as usize)
                        .chain(std::iter::once(run.end_us))
                })
                .chain(keys.shortcuts.iter().map(|s| s.t_us))
                .collect::<Vec<_>>()
        }))
        .filter(|&t| t + BUSY_TAIL_US >= start && t <= end + BUSY_LEAD_US)
        .collect();
    activity.sort_unstable();
    let mut out = Vec::new();
    let mut first = 0;
    for i in 0..activity.len() {
        let last_of_run = i + 1 == activity.len() || activity[i + 1] - activity[i] > BUSY_GAP_US;
        if last_of_run {
            let (a, b) = (activity[first], activity[i]);
            if i + 1 - first >= BUSY_EVENTS && b - a >= BUSY_MIN_US {
                out.push((a.saturating_sub(BUSY_LEAD_US), b + BUSY_TAIL_US));
            }
            first = i + 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::CanonicalEvent;
    use std::collections::BTreeMap;

    const S: u64 = 1_000_000;
    const MS: u64 = 1_000;

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
            keys: None,
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

    /// The rest rule with a five-second idle wait, so a few seconds of rest are enough.
    fn quick() -> PickSettings {
        PickSettings {
            idle_ms: 5_000,
            ..PickSettings::default()
        }
    }

    const MOVE: u64 = 350 * MS;

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
        let ranges = rest_ranges(
            &[(0, 30 * S)],
            &stream(vec![click(10 * S)]),
            &settings,
            MOVE,
        );
        let lead = MOVE + ACTIVITY_LEAD_US;
        let idle = settings.idle_ms as u64 * 1_000;
        assert_eq!(ranges, vec![(0, 10 * S - lead), (10 * S + idle, 30 * S)]);
    }

    #[test]
    fn typing_with_a_resting_mouse_keeps_the_screen() {
        use crate::telemetry::keys::{KeyStream, TypingRun};
        let settings = quick();
        let mut typing = stream(vec![]);
        typing.keys = Some(KeyStream {
            typing: vec![TypingRun {
                start_us: 10 * S,
                end_us: 14 * S,
                keys: 30,
                caret_start: None,
                caret_end: None,
            }],
            ..Default::default()
        });
        let lead = MOVE + ACTIVITY_LEAD_US;
        let idle = settings.idle_ms as u64 * 1_000;
        assert_eq!(
            rest_ranges(&[(0, 30 * S)], &typing, &settings, MOVE),
            vec![(0, 10 * S - lead), (14 * S + idle, 30 * S)]
        );
    }

    #[test]
    fn short_stretches_silence_and_gaps_never_rest() {
        let settings = quick();
        let busy: Vec<_> = (0..10).map(|i| click(i * S)).collect();
        assert!(rest_ranges(&[(0, 10 * S)], &stream(busy), &settings, MOVE).is_empty());
        assert!(rest_ranges(&[], &stream(vec![]), &settings, MOVE).is_empty());
        let gap = stream(vec![(
            10 * S,
            CanonicalKind::Gap {
                reason: "lost".into(),
                start_us: 10 * S,
                end_us: 12 * S,
                dropped_events: 0,
            },
        )]);
        let ranges = rest_ranges(&[(0, 30 * S)], &gap, &settings, MOVE);
        assert_eq!(ranges.len(), 2);
        assert!(ranges.iter().all(|&(a, b)| b <= 10 * S || a >= 12 * S));
    }

    #[test]
    fn cursor_jitter_is_not_activity_but_real_moves_are() {
        let mut settings = quick();
        let jitter = stream(vec![mv(S, 0.5), mv(5 * S, 0.501), mv(9 * S, 0.5)]);
        assert_eq!(
            rest_ranges(&[(0, 20 * S)], &jitter, &settings, MOVE),
            vec![(0, 20 * S)]
        );
        let moving = stream(vec![mv(S, 0.2), mv(10 * S, 0.8)]);
        assert_eq!(
            rest_ranges(&[(0, 20 * S)], &moving, &settings, MOVE).len(),
            2
        );
        settings.cursor_moves_are_activity = false;
        assert_eq!(
            rest_ranges(&[(0, 20 * S)], &moving, &settings, MOVE),
            vec![(0, 20 * S)]
        );
    }

    #[test]
    fn a_short_shows_the_screen_while_busy_and_the_camera_while_talking() {
        // A 40 s short from 100 s: clicks every 0.5 s from 110 to 115 s, then nothing.
        let clicks: Vec<_> = (0..11).map(|i| click(110 * S + i * 500 * MS)).collect();
        let telemetry = stream(clicks);
        let words: Vec<(u64, u64)> = (0..80)
            .map(|i| (100 * S + i * 500 * MS, 100 * S + i * 500 * MS + 400 * MS))
            .collect();
        let speech = speech_from_words(&words);
        let picked = pick(
            &PickInput {
                start_us: 100 * S,
                end_us: 140 * S,
                media: None,
                telemetry: &telemetry,
                speech: &speech,
                move_us: 0,
            },
            &PickSettings::for_short(),
        );
        let screen: Vec<&LayoutSection> = picked
            .iter()
            .filter(|s| matches!(s.layout, Some(Layout::ScreenOnly(_))))
            .collect();
        assert_eq!(screen.len(), 1, "{picked:?}");
        assert_eq!(screen[0].source_start_us, 110 * S - BUSY_LEAD_US);
        assert_eq!(screen[0].source_end_us, 115 * S + BUSY_TAIL_US);
        assert_eq!(screen[0].rule.as_deref(), Some("busy"));
        let camera: Vec<&LayoutSection> = picked
            .iter()
            .filter(|s| matches!(s.layout, Some(Layout::CameraOnly(_))))
            .collect();
        assert!(camera
            .iter()
            .any(|s| s.source_start_us == 100 * S && s.rule.as_deref() == Some("hook")));
        assert!(camera.iter().any(|s| s.source_start_us >= 116 * S));
        assert!(picked.iter().all(|s| s.auto));
        // Nothing overlaps.
        let mut track = super::super::LayoutTrack::default();
        track.replace_auto(picked);
        track.validate().unwrap();
    }

    #[test]
    fn the_video_picks_only_the_rest_by_default() {
        let telemetry = stream(vec![click(S), click(30 * S)]);
        let settings = PickSettings {
            idle_ms: 10_000,
            ..PickSettings::default()
        };
        let picked = pick(
            &PickInput {
                start_us: 0,
                end_us: 60 * S,
                media: Some("rec".into()),
                telemetry: &telemetry,
                speech: &[(0, 60 * S)],
                move_us: MOVE,
            },
            &settings,
        );
        let spans: Vec<(u64, u64)> = picked
            .iter()
            .map(|s| (s.source_start_us, s.source_end_us))
            .collect();
        let lead = MOVE + ACTIVITY_LEAD_US;
        assert_eq!(spans, vec![(11 * S, 30 * S - lead), (40 * S, 60 * S)]);
        assert!(picked.iter().all(|s| s.media.as_deref() == Some("rec")));
    }
}
