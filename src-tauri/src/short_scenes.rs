//! Scenes in a short: stretches where the vertical frame leaves its split (camera across the
//! top or bottom, screen in the rest) to show only the camera or only the screen, easing
//! between them. A scene is anchored in a recording's time, like webcam focus and chapters,
//! so it stays on the same moment through the video's cuts and the short's own edits.
//!
//! [`suggest`] picks scenes from what the recording already knows, without a model: the
//! camera for the opening line and while the mouse rests and the speaker talks, the screen
//! while it is busy with clicks and scrolls.
use crate::telemetry::{CanonicalKind, TelemetryStream};
use crate::timeline::TimelineMapper;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_SCENES: usize = 100;
pub const MAX_TRANSITION_MS: u32 = 2_000;
/// A cut: no ease in or out.
pub const DEFAULT_TRANSITION_MS: u32 = 0;
/// Scenes shorter than this flicker.
pub const MIN_SCENE_US: u64 = 1_500_000;
/// The opening line of a short shows the speaker this long, before the screen takes over.
const HOOK_US: u64 = 2_500_000;
/// Activity closer than this belongs to one busy stretch of the screen.
const BUSY_GAP_US: u64 = 1_500_000;
/// A busy stretch needs this many clicks or scrolls over at least this long.
const BUSY_EVENTS: usize = 4;
/// Typing counts as one action per this much of it.
const TYPING_STEP_US: u64 = 1_000_000;
const BUSY_MIN_US: u64 = 2_500_000;
/// The screen shows a little before the first click and stays a little after the last.
const BUSY_LEAD_US: u64 = 400_000;
const BUSY_TAIL_US: u64 = 1_000_000;
/// Scenes of one kind this close together become one.
const MERGE_GAP_US: u64 = 1_500_000;
/// A camera and a screen scene this close together meet, with no split between.
const MEET_GAP_US: u64 = 1_000_000;
/// Words this close together are one stretch of speech.
const SPEECH_GAP_US: u64 = 800_000;
/// Where captions sit in a full-frame scene: their middle at this share of the height, above
/// the controls short-video apps draw over the bottom.
pub const FULL_FRAME_CAPTION_AT: f32 = 0.70;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SceneMode {
    /// Only the camera, filling the frame.
    Camera,
    /// Only the screen, filling the frame.
    Screen,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: String,
    pub source_start_us: u64,
    pub source_end_us: u64,
    pub mode: SceneMode,
    /// Picked by [`suggest`]; picking again replaces these and keeps the others.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub auto: bool,
    /// The asset whose time the scene is in; `None` is the first recording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Where it plays on the short's own timeline; absent when it was cut. Filled in for the
    /// UI and cleared before storing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown_start_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown_end_us: Option<u64>,
}

impl Scene {
    pub fn new(id: String, source_start_us: u64, source_end_us: u64, mode: SceneMode) -> Self {
        Self {
            id,
            source_start_us,
            source_end_us,
            mode,
            auto: false,
            media: None,
            shown_start_us: None,
            shown_end_us: None,
        }
    }
}

pub fn validate(scenes: &[Scene], transition_ms: u32) -> Result<(), String> {
    if scenes.len() > MAX_SCENES {
        return Err(format!("A short has at most {MAX_SCENES} scenes"));
    }
    if transition_ms > MAX_TRANSITION_MS {
        return Err(format!(
            "A scene change takes at most {MAX_TRANSITION_MS} ms"
        ));
    }
    let mut ids = std::collections::HashSet::new();
    for scene in scenes {
        if scene.id.is_empty() || scene.id.len() > 64 || !ids.insert(scene.id.as_str()) {
            return Err("Scene ids must be unique".into());
        }
        if scene.source_end_us <= scene.source_start_us {
            return Err("A scene must end after it starts".into());
        }
    }
    let mut sorted: Vec<&Scene> = scenes.iter().collect();
    sorted.sort_by(|a, b| (&a.media, a.source_start_us).cmp(&(&b.media, b.source_start_us)));
    if sorted
        .windows(2)
        .any(|p| p[0].media == p[1].media && p[1].source_start_us < p[0].source_end_us)
    {
        return Err("Scenes can't overlap".into());
    }
    Ok(())
}

/// Where each scene plays on a timeline, as (start, end, mode), in order. `clock` gives the
/// clock of an asset (`None`: the first recording).
pub fn edited_ranges(
    scenes: &[Scene],
    clock: impl Fn(Option<&str>) -> Option<TimelineMapper>,
) -> Vec<(u64, u64, SceneMode)> {
    let mut out: Vec<(u64, u64, SceneMode)> = scenes
        .iter()
        .filter_map(|scene| {
            let mapper = clock(scene.media.as_deref())?;
            Some(
                mapper
                    .source_range_to_edited(scene.source_start_us, scene.source_end_us)
                    .into_iter()
                    .map(move |(a, b)| (a, b, scene.mode)),
            )
        })
        .flatten()
        .collect();
    out.sort_by_key(|r| r.0);
    // Back-to-back pieces of one scene (a cut inside it) play as one.
    let mut merged: Vec<(u64, u64, SceneMode)> = Vec::with_capacity(out.len());
    for range in out {
        match merged.last_mut() {
            Some(last) if last.2 == range.2 && range.0 <= last.1 => last.1 = last.1.max(range.1),
            _ => merged.push(range),
        }
    }
    merged
}

/// The camera's share of the frame height at `edited_us` (0 is only the screen, 1 only the
/// camera), and how far it is from the split toward a full-frame scene (0 to 1). `base` is the
/// split's camera share.
pub fn camera_share(
    ranges: &[(u64, u64, SceneMode)],
    base: f32,
    transition_us: u64,
    edited_us: u64,
) -> (f32, f32) {
    let target = |mode: SceneMode| match mode {
        SceneMode::Camera => 1.0,
        SceneMode::Screen => 0.0,
    };
    let ease = |t: f64| crate::zoom::cubic_bezier_unit(t.clamp(0.0, 1.0)) as f32;
    // Two scenes that meet change straight from one to the other, over the transition
    // centred on where they meet.
    let half = transition_us / 2;
    for pair in ranges.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a.1 == b.0 && a.2 != b.2 && half > 0 && edited_us + half >= a.1 && edited_us < a.1 + half
        {
            let s = (edited_us + half - a.1) as f64 / transition_us as f64;
            let (from, to) = (target(a.2), target(b.2));
            return (from + (to - from) * ease(s), 1.0);
        }
    }
    let index = ranges.partition_point(|&(_, end, _)| end <= edited_us);
    let Some(&(start, end, mode)) = ranges.get(index) else {
        return (base, 0.0);
    };
    if edited_us < start {
        return (base, 0.0);
    }
    let meets = |other: Option<&(u64, u64, SceneMode)>, at: u64| {
        other.is_some_and(|o| (o.0 == at || o.1 == at) && o.2 != mode)
    };
    let weight = if transition_us == 0 {
        1.0
    } else {
        // An edge shared with another scene was handled above; only free edges ease to the
        // split.
        let rise = if index > 0 && meets(ranges.get(index - 1), start) {
            f64::INFINITY
        } else {
            (edited_us - start) as f64 / transition_us as f64
        };
        let fall = if meets(ranges.get(index + 1), end) {
            f64::INFINITY
        } else {
            (end - edited_us) as f64 / transition_us as f64
        };
        ease(rise.min(fall))
    };
    (base + (target(mode) - base) * weight, weight)
}

/// Sorts scenes by time and drops UI-only fields, so equal lists compare equal.
pub fn normalized(mut scenes: Vec<Scene>) -> Vec<Scene> {
    for scene in &mut scenes {
        scene.shown_start_us = None;
        scene.shown_end_us = None;
    }
    scenes.sort_by(|a, b| {
        (&a.media, a.source_start_us, &a.id).cmp(&(&b.media, b.source_start_us, &b.id))
    });
    scenes
}

/// What [`suggest`] reads, all in the recording's time.
pub struct SuggestInput<'a> {
    /// The short's stretch of the recording.
    pub start_us: u64,
    pub end_us: u64,
    pub media: Option<String>,
    pub telemetry: &'a TelemetryStream,
    /// Spoken words (start, end); empty when nothing is transcribed (all counts as speech).
    pub words: &'a [(u64, u64)],
}

/// Scenes for a short from its recording: the screen while clicks, scrolls or typing come thick
/// and fast, the camera for the opening line and while the mouse rests as the speaker talks.
/// Anything else stays split.
pub fn suggest(input: &SuggestInput) -> Vec<Scene> {
    let (start, end) = (input.start_us, input.end_us);
    if end <= start {
        return Vec::new();
    }
    let clip = |(a, b): (u64, u64)| (a.max(start), b.min(end));

    // Busy screen: clicks and scrolls close together.
    let mut activity: Vec<u64> = input
        .telemetry
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
        .chain(input.telemetry.keys.iter().flat_map(|keys| {
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
    let mut screen: Vec<(u64, u64)> = Vec::new();
    let mut first = 0;
    for i in 0..activity.len() {
        let last_of_run = i + 1 == activity.len() || activity[i + 1] - activity[i] > BUSY_GAP_US;
        if last_of_run {
            let (a, b) = (activity[first], activity[i]);
            if i + 1 - first >= BUSY_EVENTS && b - a >= BUSY_MIN_US {
                screen.push(clip((a.saturating_sub(BUSY_LEAD_US), b + BUSY_TAIL_US)));
            }
            first = i + 1;
        }
    }

    // Camera: the speaker talking while the mouse rests, as webcam focus finds it. The rest is
    // shorter than the video's (a short moves faster) but long enough that the split, which
    // shows both, stays the usual look.
    let mut speech: Vec<(u64, u64)> = Vec::new();
    if input.words.is_empty() {
        speech.push((start, end));
    } else {
        let mut words: Vec<(u64, u64)> = input.words.to_vec();
        words.sort_unstable();
        for (a, b) in words {
            match speech.last_mut() {
                Some(last) if a <= last.1 + SPEECH_GAP_US => last.1 = last.1.max(b),
                _ => speech.push((a, b)),
            }
        }
    }
    let settings = crate::webcam_focus::WebcamFocusSettings {
        idle_ms: 8_000,
        min_focus_ms: 3_000,
        transition_ms: DEFAULT_TRANSITION_MS,
        ..Default::default()
    };
    let mut camera: Vec<(u64, u64)> =
        crate::webcam_focus::detect_focus_ranges(&speech, input.telemetry, &settings)
            .into_iter()
            .map(clip)
            .collect();
    // The opening line, unless the screen is busy from the start.
    let hook_end = (start + HOOK_US).min(end);
    let busy_soon = screen.first().is_some_and(|s| s.0 < start + MIN_SCENE_US);
    if !busy_soon {
        let hook_end = screen.first().map_or(hook_end, |s| hook_end.min(s.0));
        camera.push((start, hook_end));
    }
    // The screen wins where both want the frame.
    let camera = subtract(&merge(camera, MERGE_GAP_US), &screen);

    let mut scenes: Vec<Scene> = Vec::new();
    for (ranges, mode) in [
        (merge(screen, MERGE_GAP_US), SceneMode::Screen),
        (camera, SceneMode::Camera),
    ] {
        for (a, b) in ranges.into_iter().filter(|(a, b)| b - a >= MIN_SCENE_US) {
            scenes.push(Scene {
                auto: true,
                media: input.media.clone(),
                ..Scene::new(format!("auto-{}", a / 1000), a, b, mode)
            });
        }
    }
    let mut scenes = normalized(scenes);
    // A moment of split between a camera and a screen scene would only flicker: they meet.
    for i in 1..scenes.len() {
        let next_start = scenes[i].source_start_us;
        let previous = &mut scenes[i - 1];
        if next_start > previous.source_end_us && next_start - previous.source_end_us < MEET_GAP_US
        {
            previous.source_end_us = next_start;
        }
    }
    scenes
}

/// `ranges` sorted, with any closer than `gap` joined.
fn merge(mut ranges: Vec<(u64, u64)>, gap: u64) -> Vec<(u64, u64)> {
    ranges.retain(|(a, b)| b > a);
    ranges.sort_unstable();
    let mut out: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (a, b) in ranges {
        match out.last_mut() {
            Some(last) if a <= last.1 + gap => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

/// What is left of `ranges` (merged) outside `remove`.
fn subtract(ranges: &[(u64, u64)], remove: &[(u64, u64)]) -> Vec<(u64, u64)> {
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

/// `existing` with its automatic scenes replaced by `picked`; the user's own scenes stay, and
/// picked ones give way to them.
pub fn replace_auto(existing: &[Scene], picked: Vec<Scene>) -> Vec<Scene> {
    let manual: Vec<Scene> = existing.iter().filter(|s| !s.auto).cloned().collect();
    let mut out = manual.clone();
    for scene in picked {
        let taken: Vec<(u64, u64)> = manual
            .iter()
            .filter(|m| m.media == scene.media)
            .map(|m| (m.source_start_us, m.source_end_us))
            .collect();
        for (a, b) in subtract(&[(scene.source_start_us, scene.source_end_us)], &taken) {
            if b - a >= MIN_SCENE_US {
                out.push(Scene {
                    id: format!("auto-{}", a / 1000),
                    source_start_us: a,
                    source_end_us: b,
                    ..scene.clone()
                });
            }
        }
    }
    normalized(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::CanonicalEvent;
    use std::collections::BTreeMap;

    const S: u64 = 1_000_000;
    const MS: u64 = 1_000;

    fn telemetry(clicks: &[u64]) -> TelemetryStream {
        TelemetryStream {
            events: clicks
                .iter()
                .enumerate()
                .map(|(i, &t)| CanonicalEvent {
                    version: 1,
                    seq: i as u64,
                    t_us: t,
                    geometry_id: None,
                    kind: CanonicalKind::Click {
                        norm_x: 0.5,
                        norm_y: 0.5,
                        inside_source: true,
                    },
                })
                .collect(),
            geometries: BTreeMap::new(),
            diagnostics: Vec::new(),
            keys: None,
        }
    }

    #[test]
    fn busy_clicking_shows_the_screen_and_talking_shows_the_camera() {
        // A 40 s short from 100 s: clicks every 0.5 s from 110 to 115 s, then nothing.
        let clicks: Vec<u64> = (0..11).map(|i| 110 * S + i * 500 * MS).collect();
        let stream = telemetry(&clicks);
        let words: Vec<(u64, u64)> = (0..80)
            .map(|i| (100 * S + i * 500 * MS, 100 * S + i * 500 * MS + 400 * MS))
            .collect();
        let scenes = suggest(&SuggestInput {
            start_us: 100 * S,
            end_us: 140 * S,
            media: None,
            telemetry: &stream,
            words: &words,
        });
        let screen: Vec<&Scene> = scenes
            .iter()
            .filter(|s| s.mode == SceneMode::Screen)
            .collect();
        assert_eq!(screen.len(), 1, "{scenes:?}");
        assert_eq!(screen[0].source_start_us, 110 * S - BUSY_LEAD_US);
        assert_eq!(screen[0].source_end_us, 115 * S + BUSY_TAIL_US);
        // The opening line, and the talk after the clicking stops, show the camera.
        let camera: Vec<&Scene> = scenes
            .iter()
            .filter(|s| s.mode == SceneMode::Camera)
            .collect();
        assert!(
            camera.iter().any(|s| s.source_start_us == 100 * S),
            "{scenes:?}"
        );
        assert!(
            camera.iter().any(|s| s.source_start_us >= 116 * S),
            "{scenes:?}"
        );
        // Nothing overlaps, and the whole list validates.
        validate(&scenes, DEFAULT_TRANSITION_MS).unwrap();
        assert!(scenes.iter().all(|s| s.auto));
    }

    #[test]
    fn typing_with_a_resting_mouse_shows_the_screen() {
        use crate::telemetry::keys::{KeyStream, TypingRun};
        let mut stream = telemetry(&[]);
        stream.keys = Some(KeyStream {
            typing: vec![TypingRun {
                start_us: 110 * S,
                end_us: 116 * S,
                keys: 60,
                caret_start: None,
                caret_end: None,
            }],
            ..Default::default()
        });
        let words: Vec<(u64, u64)> = (0..80)
            .map(|i| (100 * S + i * 500 * MS, 100 * S + i * 500 * MS + 400 * MS))
            .collect();
        let scenes = suggest(&SuggestInput {
            start_us: 100 * S,
            end_us: 140 * S,
            media: None,
            telemetry: &stream,
            words: &words,
        });
        let screen: Vec<&Scene> = scenes
            .iter()
            .filter(|s| s.mode == SceneMode::Screen)
            .collect();
        assert_eq!(screen.len(), 1, "{scenes:?}");
        assert_eq!(screen[0].source_start_us, 110 * S - BUSY_LEAD_US);
        assert_eq!(screen[0].source_end_us, 116 * S + BUSY_TAIL_US);
        // The camera never takes the frame while the typing goes on.
        assert!(scenes
            .iter()
            .filter(|s| s.mode == SceneMode::Camera)
            .all(|s| s.source_end_us <= 110 * S || s.source_start_us >= 116 * S));
    }

    #[test]
    fn picking_again_keeps_the_users_scenes() {
        let manual = Scene::new("mine".into(), 10 * S, 14 * S, SceneMode::Screen);
        let old_auto = Scene {
            auto: true,
            ..Scene::new("auto-0".into(), 0, 3 * S, SceneMode::Camera)
        };
        let picked = vec![Scene {
            auto: true,
            ..Scene::new("auto-8000".into(), 8 * S, 20 * S, SceneMode::Camera)
        }];
        let out = replace_auto(&[manual.clone(), old_auto], picked);
        let spans: Vec<(u64, u64, bool)> = out
            .iter()
            .map(|s| (s.source_start_us, s.source_end_us, s.auto))
            .collect();
        // The old pick is gone; the new one goes around the user's scene.
        assert_eq!(
            spans,
            vec![
                (8 * S, 10 * S, true),
                (10 * S, 14 * S, false),
                (14 * S, 20 * S, true)
            ]
        );
        validate(&out, 0).unwrap();
    }

    #[test]
    fn the_camera_share_eases_into_a_scene_and_back() {
        let ranges = [
            (10 * S, 20 * S, SceneMode::Camera),
            (30 * S, 40 * S, SceneMode::Screen),
        ];
        let t = 400 * MS;
        assert_eq!(camera_share(&ranges, 0.35, t, 5 * S), (0.35, 0.0));
        assert_eq!(camera_share(&ranges, 0.35, t, 15 * S), (1.0, 1.0));
        assert_eq!(camera_share(&ranges, 0.35, t, 35 * S), (0.0, 1.0));
        let (half, weight) = camera_share(&ranges, 0.35, t, 10 * S + 200 * MS);
        assert!(half > 0.35 && half < 1.0 && weight > 0.0 && weight < 1.0);
        // A hard cut switches at once.
        assert_eq!(camera_share(&ranges, 0.35, 0, 10 * S), (1.0, 1.0));
        // Scenes that meet go straight from one to the other, never through the split.
        let meeting = [
            (10 * S, 20 * S, SceneMode::Camera),
            (20 * S, 30 * S, SceneMode::Screen),
        ];
        assert_eq!(camera_share(&meeting, 0.35, t, 19 * S), (1.0, 1.0));
        assert_eq!(camera_share(&meeting, 0.35, t, 20 * S), (0.5, 1.0));
        assert_eq!(camera_share(&meeting, 0.35, t, 21 * S), (0.0, 1.0));
        let (going, _) = camera_share(&meeting, 0.35, t, 20 * S - 100 * MS);
        assert!(going < 1.0 && going > 0.5, "{going}");
    }

    #[test]
    fn overlapping_scenes_are_refused() {
        let scene = |id: &str, a: u64, b: u64| Scene::new(id.into(), a, b, SceneMode::Camera);
        assert!(validate(&[scene("a", 0, 5 * S), scene("b", 4 * S, 8 * S)], 0).is_err());
        assert!(validate(&[scene("a", 0, 5 * S), scene("b", 5 * S, 8 * S)], 0).is_ok());
        assert!(validate(&[scene("a", 0, 5 * S)], MAX_TRANSITION_MS + 1).is_err());
    }
}
