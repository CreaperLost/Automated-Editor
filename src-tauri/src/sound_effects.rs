//! Subtle sound effects on their own lane: a soft click on recorded mouse clicks and a light
//! whoosh on some transitions (zooms going in or out, the webcam growing or shrinking, a
//! short's scene switching). The lane has its own mute and volume and is lowered under speech
//! through the polish path's ducking.
//!
//! Never overdone: rapid clicks are thinned out and the second click of a double click is
//! silent; tiny zooms and back-to-back transitions get no whoosh; nothing plays inside a
//! graphic, and a click a cut removed makes no sound. The sounds are synthesized here, so the
//! app ships no sound files and nothing needs a licence.
//!
//! The events are worked out from the edit document alone and mixed as a function of timeline
//! time, so playback and export make the same samples however they read them.
use crate::media::audio::SAMPLE_RATE;
use crate::project::revision::EditDocument;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

/// The lane's id for the mixer's ducking (no audio track has it).
pub const LANE_ID: &str = "sound-effects";
/// How far the lane goes down under speech.
pub const DUCK_DB: f32 = 8.0;
pub const VOLUME_DB_RANGE: (f32, f32) = (-30.0, 6.0);
/// At most one click sound this often.
const MIN_CLICK_GAP_US: u64 = 180_000;
/// A press this soon after another, about where it was, is the second half of a double click.
const DOUBLE_CLICK_US: u64 = 500_000;
const DOUBLE_CLICK_DISTANCE: f64 = 0.02;
/// Zooms closer than this make no whoosh.
pub const MIN_WHOOSH_SCALE: f64 = 1.3;
/// At most one whoosh this often: back-to-back transitions get one.
const MIN_WHOOSH_GAP_US: u64 = 1_500_000;
/// The whoosh starts this much before its transition, so it swells with the move.
const WHOOSH_LEAD_US: u64 = 60_000;
/// Peak levels of the built-in sounds before the lane's volume: quiet beside speech.
const CLICK_PEAK: f64 = 0.12;
const WHOOSH_PEAK: f64 = 0.07;
/// A zoom out is a little softer than a zoom in.
const WHOOSH_OUT_GAIN: f64 = 0.75;

/// The sound effects lane's settings, kept with the project's audio.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SoundEffects {
    /// A soft click on each recorded mouse click (thinned out). Off by default: on a busy
    /// recording they came across as spammy.
    #[serde(default)]
    pub clicks: bool,
    /// A light whoosh on zooms and layout switches. Off by default, like the clicks.
    #[serde(default)]
    pub whooshes: bool,
    /// The lane's own mute.
    #[serde(default)]
    pub muted: bool,
    /// The lane's volume, in dB.
    #[serde(default)]
    pub volume_db: f32,
}

impl Default for SoundEffects {
    fn default() -> Self {
        Self {
            clicks: false,
            whooshes: false,
            muted: false,
            volume_db: 0.0,
        }
    }
}

impl SoundEffects {
    pub fn validate(&self) -> Result<(), String> {
        let (lo, hi) = VOLUME_DB_RANGE;
        if !self.volume_db.is_finite() || !(lo..=hi).contains(&self.volume_db) {
            return Err(format!(
                "Sound effects volume must be between {lo} and {hi}"
            ));
        }
        Ok(())
    }

    /// Whether the lane makes any sound.
    pub fn audible(&self) -> bool {
        !self.muted && (self.clicks || self.whooshes)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EffectKind {
    Click,
    Whoosh,
}

/// One sound on the lane, at timeline time `at_us`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    pub at_us: u64,
    pub kind: EffectKind,
    pub gain: f64,
}

// ---------------------------------------------------------------------------------------
// The built-in sounds, synthesized once.

/// A small deterministic noise source, so every build makes the same sounds.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    }
}

fn normalized(mut samples: Vec<[f32; 2]>, peak: f64) -> Vec<[f32; 2]> {
    let max = samples
        .iter()
        .flat_map(|s| s.iter())
        .fold(0.0f32, |m, v| m.max(v.abs()));
    if max > 0.0 {
        let k = (peak / max as f64) as f32;
        for s in &mut samples {
            s[0] *= k;
            s[1] *= k;
        }
    }
    samples
}

/// A soft mouse click: a short, dull tick (28 ms).
fn synth_click() -> Vec<[f32; 2]> {
    let rate = SAMPLE_RATE as f64;
    let frames = (0.028 * rate) as usize;
    let mut noise = Noise(0x9E37_79B9_7F4A_7C15);
    let mut low = 0.0;
    let out = (0..frames)
        .map(|i| {
            let t = i as f64 / rate;
            // Noise through a gentle low-pass: the tap of the button, not a hiss.
            low += 0.35 * (noise.next() - low);
            let tap = low * (-t / 0.003).exp();
            let tick = (std::f64::consts::TAU * 2_400.0 * t).sin() * (-t / 0.004).exp();
            let body = (std::f64::consts::TAU * 820.0 * t).sin() * (-t / 0.009).exp();
            // A 0.3 ms ramp in, so the first sample does not snap.
            let ramp = (t / 0.0003).min(1.0);
            let s = ramp * (0.55 * tap + 0.35 * tick + 0.6 * body);
            [s as f32, (s * 0.96) as f32]
        })
        .collect();
    normalized(out, CLICK_PEAK)
}

/// A light whoosh: band-passed noise sweeping up and back down, drifting left to right
/// (480 ms).
fn synth_whoosh() -> Vec<[f32; 2]> {
    let rate = SAMPLE_RATE as f64;
    let frames = (0.48 * rate) as usize;
    let mut noise = Noise(0xD1B5_4A32_D192_ED03);
    let (mut low, mut band) = (0.0f64, 0.0f64);
    let out = (0..frames)
        .map(|i| {
            let p = i as f64 / frames as f64;
            // Swells over the first 55 %, then dies away.
            let env = if p < 0.55 {
                (p / 0.55 * std::f64::consts::FRAC_PI_2).sin().powi(2)
            } else {
                (((1.0 - p) / 0.45) * std::f64::consts::FRAC_PI_2)
                    .sin()
                    .powi(2)
            };
            let cutoff = 350.0 + 1_900.0 * (p * std::f64::consts::PI).sin();
            let f = 2.0 * (std::f64::consts::PI * cutoff / rate).sin();
            let high = noise.next() - low - 0.8 * band;
            band += f * high;
            low += f * band;
            let s = band * env;
            let pan = 0.35 + 0.3 * p;
            let angle = pan * std::f64::consts::FRAC_PI_2;
            [(s * angle.cos()) as f32, (s * angle.sin()) as f32]
        })
        .collect();
    normalized(out, WHOOSH_PEAK)
}

fn sound(kind: EffectKind) -> &'static [[f32; 2]] {
    static CLICK: OnceLock<Vec<[f32; 2]>> = OnceLock::new();
    static WHOOSH: OnceLock<Vec<[f32; 2]>> = OnceLock::new();
    match kind {
        EffectKind::Click => CLICK.get_or_init(synth_click),
        EffectKind::Whoosh => WHOOSH.get_or_init(synth_whoosh),
    }
}

// ---------------------------------------------------------------------------------------
// When the sounds play.

/// The presses (timeline time, x, y) that get a click sound: at most one per
/// [`MIN_CLICK_GAP_US`], and none for the second press of a double click.
pub fn thin_clicks(presses: &[(u64, f64, f64)]) -> Vec<u64> {
    let mut sorted = presses.to_vec();
    sorted.sort_by_key(|p| p.0);
    let mut out: Vec<u64> = Vec::new();
    let mut previous: Option<(u64, f64, f64)> = None;
    for press in sorted {
        let echo = previous.is_some_and(|(t, x, y)| {
            press.0 - t < DOUBLE_CLICK_US
                && (press.1 - x).hypot(press.2 - y) < DOUBLE_CLICK_DISTANCE
        });
        previous = Some(press);
        if echo || out.last().is_some_and(|&t| press.0 - t < MIN_CLICK_GAP_US) {
            continue;
        }
        out.push(press.0);
    }
    out
}

/// Transitions (timeline time, gain) that get a whoosh: none within [`MIN_WHOOSH_GAP_US`] of
/// the last one played.
pub fn thin_whooshes(transitions: &[(u64, f64)]) -> Vec<(u64, f64)> {
    let mut sorted = transitions.to_vec();
    sorted.sort_by_key(|t| t.0);
    let mut out: Vec<(u64, f64)> = Vec::new();
    for transition in sorted {
        if out
            .last()
            .is_some_and(|&(t, _)| transition.0 - t < MIN_WHOOSH_GAP_US)
        {
            continue;
        }
        out.push(transition);
    }
    out
}

fn inside(ranges: &[(u64, u64)], t: u64) -> bool {
    ranges.iter().any(|&(a, b)| a <= t && t < b)
}

/// Where graphics play on the timeline: no sound effects in there.
fn graphic_ranges(document: &EditDocument) -> Vec<(u64, u64)> {
    document
        .sequence
        .video_tracks()
        .filter(|track| !track.hidden)
        .flat_map(|track| track.clips.iter())
        .filter(|clip| document.asset(&clip.asset).is_some_and(|a| a.is_graphic()))
        .map(|clip| (clip.start_us, clip.end_us()))
        .collect()
}

/// The lane's sounds for `document`, in timeline order.
pub fn plan(document: &EditDocument) -> Vec<Effect> {
    let settings = &document.audio.sound_effects;
    if !settings.audible() {
        return Vec::new();
    }
    let graphics = graphic_ranges(document);
    // A short's camera scenes show no screen, so its clicks are not heard either.
    let camera_scenes: Vec<(u64, u64)> = match &document.short_layout {
        Some(short) => crate::short_scenes::edited_ranges(&short.scenes, |media| {
            document
                .clock_asset(media)
                .map(|asset| document.asset_clock(asset))
        })
        .into_iter()
        .filter(|(_, _, mode)| *mode == crate::short_scenes::SceneMode::Camera)
        .map(|(a, b, _)| (a, b))
        .collect(),
        None => Vec::new(),
    };
    let mut effects = Vec::new();
    let recordings: Vec<&crate::sequence::Asset> = document
        .assets
        .iter()
        .filter(|a| a.is_recording())
        .collect();
    if settings.clicks {
        let mut presses = Vec::new();
        for asset in &recordings {
            let clock = document.picture_clock(&asset.id);
            if clock.intervals().is_empty() {
                continue;
            }
            let activity = crate::telemetry::activity::recording_activity(Path::new(&asset.path));
            for press in &activity.presses {
                // A cut that removed the click removes its sound.
                let Some(at) = clock.source_to_edited_us(press.t_us) else {
                    continue;
                };
                if !inside(&graphics, at) && !inside(&camera_scenes, at) {
                    presses.push((at, press.x, press.y));
                }
            }
        }
        effects.extend(thin_clicks(&presses).into_iter().map(|at_us| Effect {
            at_us,
            kind: EffectKind::Click,
            gain: 1.0,
        }));
    }
    if settings.whooshes {
        let mut transitions = Vec::new();
        for asset in &recordings {
            let clock = document.picture_clock(&asset.id);
            for zoom in document
                .zooms
                .iter()
                .filter(|z| document.clock_asset(z.media.as_deref()) == Some(asset.id.as_str()))
                .filter(|z| z.scale >= MIN_WHOOSH_SCALE)
            {
                let zoom_out = zoom.source_end_us.saturating_sub(zoom.transition_us);
                for (source, gain) in [(zoom.source_start_us, 1.0), (zoom_out, WHOOSH_OUT_GAIN)] {
                    if let Some(at) = clock.source_to_edited_us(source) {
                        transitions.push((at, gain));
                    }
                }
            }
        }
        // The webcam growing to fill the frame and going back.
        if document.short_layout.is_none() && document.layout.webcam_enabled {
            for (a, b) in document.webcam_focus.edited_ranges(&document.focus_clock()) {
                transitions.push((a, 1.0));
                transitions.push((
                    b.saturating_sub(document.webcam_focus.settings.transition_us()),
                    WHOOSH_OUT_GAIN,
                ));
            }
        }
        // A short's scenes switching.
        if let Some(short) = &document.short_layout {
            let scenes = crate::short_scenes::edited_ranges(&short.scenes, |media| {
                document
                    .clock_asset(media)
                    .map(|asset| document.asset_clock(asset))
            });
            for (a, b, _) in scenes {
                transitions.push((a, 1.0));
                transitions.push((b, WHOOSH_OUT_GAIN));
            }
        }
        let duration = document.duration_us();
        transitions.retain(|&(at, _)| at > 0 && at < duration && !inside(&graphics, at));
        effects.extend(
            thin_whooshes(&transitions)
                .into_iter()
                .filter(|&(at, _)| !inside(&graphics, at.saturating_sub(WHOOSH_LEAD_US)))
                .map(|(at, gain)| Effect {
                    at_us: at.saturating_sub(WHOOSH_LEAD_US),
                    kind: EffectKind::Whoosh,
                    gain,
                }),
        );
    }
    effects.sort_by_key(|e| e.at_us);
    effects
}

// ---------------------------------------------------------------------------------------
// Mixing.

fn frame_of(us: u64) -> u64 {
    (us as u128 * SAMPLE_RATE as u128 / 1_000_000) as u64
}

/// The lane as the mixer plays it.
pub struct SoundEffectsLane {
    /// (first frame, kind, gain), by first frame.
    effects: Vec<(u64, EffectKind, f64)>,
    gain: f64,
    longest: u64,
}

impl SoundEffectsLane {
    /// `None` when the lane is silent (switched off, muted, or nothing to play).
    pub fn new(document: &EditDocument) -> Option<Self> {
        let effects: Vec<(u64, EffectKind, f64)> = plan(document)
            .into_iter()
            .map(|e| (frame_of(e.at_us), e.kind, e.gain))
            .collect();
        if effects.is_empty() {
            return None;
        }
        let longest = [EffectKind::Click, EffectKind::Whoosh]
            .iter()
            .map(|&k| sound(k).len() as u64)
            .max()
            .unwrap_or(0);
        Some(Self {
            effects,
            gain: 10f64.powf(document.audio.sound_effects.volume_db as f64 / 20.0),
            longest,
        })
    }

    /// Adds frames `start..start + out.len() / 2` of the lane to `out` (interleaved stereo),
    /// `duck(t)` giving the speech ducking gain at timeline microsecond `t`.
    pub fn mix(&self, start: u64, out: &mut [f64], duck: impl Fn(f64) -> f64) {
        let end = start + (out.len() / 2) as u64;
        let first = self
            .effects
            .partition_point(|(at, _, _)| at + self.longest <= start);
        for &(at, kind, gain) in self.effects[first..].iter().take_while(|e| e.0 < end) {
            let samples = sound(kind);
            let from = start.max(at);
            let to = end.min(at + samples.len() as u64);
            for frame in from..to {
                let s = samples[(frame - at) as usize];
                let g = gain * self.gain * duck(frame as f64 * 1e6 / SAMPLE_RATE as f64);
                let i = (frame - start) as usize * 2;
                out[i] += s[0] as f64 * g;
                out[i + 1] += s[1] as f64 * g;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000;

    #[test]
    fn rapid_clicks_are_thinned_and_double_clicks_click_once() {
        // A burst every 50 ms at different places: one sound per 180 ms.
        let burst: Vec<(u64, f64, f64)> = (0..10)
            .map(|i| (i * 50 * MS, 0.1 * i as f64, 0.5))
            .collect();
        let kept = thin_clicks(&burst);
        assert_eq!(kept, vec![0, 200 * MS, 400 * MS]);
        // A double click: one sound. Then a third press there, still part of it.
        let double = [(0, 0.5, 0.5), (250 * MS, 0.505, 0.5), (600 * MS, 0.5, 0.5)];
        assert_eq!(thin_clicks(&double), vec![0]);
        // Two clicks at different places a quarter second apart: both.
        assert_eq!(
            thin_clicks(&[(0, 0.2, 0.2), (250 * MS, 0.8, 0.8)]),
            vec![0, 250 * MS]
        );
        // The same place, slow: every one.
        assert_eq!(
            thin_clicks(&[(0, 0.5, 0.5), (700 * MS, 0.5, 0.5)]),
            vec![0, 700 * MS]
        );
    }

    #[test]
    fn back_to_back_transitions_get_one_whoosh() {
        let kept = thin_whooshes(&[(1_000 * MS, 1.0), (1_400 * MS, 0.75), (5_000 * MS, 1.0)]);
        assert_eq!(kept, vec![(1_000 * MS, 1.0), (5_000 * MS, 1.0)]);
    }

    #[test]
    fn the_built_in_sounds_are_short_and_quiet() {
        let click = sound(EffectKind::Click);
        let whoosh = sound(EffectKind::Whoosh);
        assert!(click.len() < SAMPLE_RATE as usize / 20);
        assert!(whoosh.len() < SAMPLE_RATE as usize / 2);
        let peak = |s: &[[f32; 2]]| s.iter().flatten().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((peak(click) as f64 - CLICK_PEAK).abs() < 1e-3);
        assert!((peak(whoosh) as f64 - WHOOSH_PEAK).abs() < 1e-3);
        // Starts from silence and ends there: no snap at either end.
        assert!(click[0][0].abs() < 0.01 && whoosh[0][0].abs() < 0.01);
        assert!(whoosh.last().unwrap()[0].abs() < 0.005);
        // Built the same every time.
        assert_eq!(synth_click(), click);
    }

    #[test]
    fn mixing_in_chunks_matches_one_read_and_ducking_scales_it() {
        let lane = SoundEffectsLane {
            effects: vec![
                (1_000, EffectKind::Click, 1.0),
                (2_000, EffectKind::Whoosh, 0.75),
            ],
            gain: 0.5,
            longest: sound(EffectKind::Whoosh).len() as u64,
        };
        let frames = 30_000usize;
        let mut whole = vec![0.0; frames * 2];
        lane.mix(0, &mut whole, |_| 1.0);
        let mut chunked = vec![0.0; frames * 2];
        for start in (0..frames).step_by(4_800) {
            let n = 4_800.min(frames - start);
            let mut chunk = vec![0.0; n * 2];
            lane.mix(start as u64, &mut chunk, |_| 1.0);
            chunked[start * 2..(start + n) * 2].copy_from_slice(&chunk);
        }
        assert_eq!(whole, chunked);
        assert!(whole[..2_000].iter().all(|&s| s == 0.0));
        let peak = whole.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        assert!(peak > 0.0 && peak <= CLICK_PEAK * 0.5 + 1e-6);
        let mut ducked = vec![0.0; frames * 2];
        lane.mix(0, &mut ducked, |_| 0.5);
        assert!(ducked
            .iter()
            .zip(&whole)
            .all(|(d, w)| (d - w * 0.5).abs() < 1e-12));
    }

    #[test]
    fn settings_default_off_and_validate_the_volume() {
        let settings: SoundEffects = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, SoundEffects::default());
        assert!(!settings.audible());
        let on = SoundEffects {
            clicks: true,
            ..Default::default()
        };
        assert!(on.audible());
        assert!(!SoundEffects { muted: true, ..on }.audible());
        assert!(SoundEffects {
            volume_db: 12.0,
            ..Default::default()
        }
        .validate()
        .is_err());
    }
}
