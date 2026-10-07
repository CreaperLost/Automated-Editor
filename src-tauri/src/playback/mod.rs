//! Playback owner: the playhead, its clock and generations that retire stale work.
//! F1 adds a discardable native preview overlay; F2 still owns decode/compositor.
pub mod audio;
mod audio_cpal;
#[cfg(feature = "tauri-app")]
pub mod engine;
mod native;
pub mod preview;

use crate::project::revision::EditDocument;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub use preview::{PreviewHitMode, PreviewOwner, PreviewQuality, PreviewStatus, PreviewViewport};

fn next_generation() -> u64 {
    static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackState {
    Closed,
    Ready,
    Playing,
    Paused,
    Seeking,
    Ended,
    Error,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClockKind {
    Audio,
    Monotonic,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackStatus {
    pub project_handle: String,
    pub state: PlaybackState,
    pub generation: u64,
    pub position_us: u64,
    pub duration_us: u64,
    pub clock_kind: ClockKind,
    /// The short playing instead of the video, when the Shorts Studio has one in focus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_id: Option<String>,
    /// How fast it plays: 1 is normal speed.
    #[serde(default = "normal_speed")]
    pub speed: f64,
    pub preview_available: bool,
    pub error: Option<String>,
    pub diagnostics: Vec<String>,
}

fn normal_speed() -> f64 {
    1.0
}

/// The playback speeds offered (the preview bar's menu, and L played twice: 2x).
pub const PLAYBACK_SPEEDS: [f64; 4] = [1.0, 1.25, 1.5, 2.0];

/// Whether this platform's audio output can play at `speed` (Windows resamples; the macOS
/// output plays at normal speed only, so faster playback there is silent).
fn audio_plays_at(speed: f64) -> bool {
    cfg!(windows) || speed == 1.0
}

pub struct PlaybackOwner {
    pub(crate) native_enabled: bool,
    pub(crate) audio: Option<audio::AudioOutput>,
    pub(crate) audio_start_frame: u64,
    #[cfg_attr(not(feature = "tauri-app"), allow(dead_code))]
    pub(crate) audio_queued_frame: u64,
    preview_available: bool,
    project_handle: String,
    state: PlaybackState,
    generation: u64,
    /// Bumped when what plays changes (an edit, another short), not on a seek: the engine
    /// keeps its scene and mixer across seeks.
    content: u64,
    position_us: u64,
    duration_us: u64,
    /// Some audio track has clips to play, so the audio device is the clock.
    has_sound: bool,
    play_anchor: Option<(Instant, u64)>,
    error: Option<String>,
    diagnostics: Vec<String>,
    /// The short that plays instead of the video.
    short_focus: Option<String>,
    /// Where the video was when a short took over playback.
    main_position_us: u64,
    /// How fast it plays (one of [`PLAYBACK_SPEEDS`]).
    speed: f64,
}

impl PlaybackOwner {
    /// What plays: the project's edit, or the short in focus as its own vertical video.
    pub fn playable_document(&self, document: &EditDocument) -> Result<EditDocument, String> {
        match &self.short_focus {
            Some(id) => {
                let short = document
                    .shorts
                    .iter()
                    .find(|s| &s.id == id)
                    .ok_or("That short no longer exists")?;
                crate::shorts::short_document(document, short, true)
            }
            None => Ok(document.clone()),
        }
    }

    /// Plays short `short` instead of the video (or the video again with `None`), from its start.
    pub fn focus_short(
        &mut self,
        short: Option<String>,
        document: &EditDocument,
        start_us: u64,
    ) -> Result<(), String> {
        if self.short_focus == short {
            if short.is_some() {
                self.seek(start_us)?;
            }
            return Ok(());
        }
        // Whatever played stops: the caller plays or pauses what comes next.
        self.advance();
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
            self.play_anchor = None;
            self.audio = None;
        }
        // The video's place is kept while a short plays, and comes back after.
        let position = match (&self.short_focus, &short) {
            (None, Some(_)) => {
                self.advance();
                self.main_position_us = self.position_us;
                start_us
            }
            (Some(_), None) => self.main_position_us,
            _ => start_us,
        };
        self.short_focus = short;
        self.position_us = position;
        self.apply_document(document)
    }

    pub fn closed() -> Self {
        Self {
            native_enabled: false,
            audio: None,
            audio_start_frame: 0,
            audio_queued_frame: 0,
            preview_available: false,
            project_handle: String::new(),
            state: PlaybackState::Closed,
            generation: 0,
            content: 0,
            position_us: 0,
            duration_us: 0,
            has_sound: false,
            play_anchor: None,
            error: None,
            diagnostics: Vec::new(),
            short_focus: None,
            main_position_us: 0,
            speed: 1.0,
        }
    }

    pub fn open(project_handle: String, document: &EditDocument) -> Result<Self, String> {
        let duration_us = document.duration_us();
        Ok(Self {
            native_enabled: false,
            audio: None,
            audio_start_frame: 0,
            audio_queued_frame: 0,
            preview_available: false,
            project_handle,
            state: if duration_us == 0 {
                PlaybackState::Ended
            } else {
                PlaybackState::Ready
            },
            generation: next_generation(),
            content: next_generation(),
            position_us: 0,
            duration_us,
            has_sound: has_sound(document),
            play_anchor: None,
            error: None,
            diagnostics: Vec::new(),
            short_focus: None,
            main_position_us: 0,
            speed: 1.0,
        })
    }

    pub fn apply_document(&mut self, document: &EditDocument) -> Result<(), String> {
        self.advance();
        // An edit while playing keeps playing (with the new edit): changing a short's look as
        // it plays, or a caption, no longer stops it.
        let was_playing = self.state == PlaybackState::Playing;
        self.bump_generation();
        self.content = next_generation();
        // A short that is gone or no longer valid hands playback back to the video.
        let playable = match self.playable_document(document) {
            Ok(playable) => playable,
            Err(_) => {
                self.short_focus = None;
                document.clone()
            }
        };
        let document = &playable;
        self.has_sound = has_sound(document);
        self.duration_us = document.duration_us();
        if self.position_us > self.duration_us {
            self.position_us = self.duration_us;
        }
        self.play_anchor = None;
        self.audio = None;
        self.state = PlaybackState::Paused;
        if self.duration_us == 0 {
            self.state = PlaybackState::Ended;
        } else if self.position_us >= self.duration_us {
            self.state = PlaybackState::Ended;
        } else if self.state == PlaybackState::Ended || self.state == PlaybackState::Closed {
            self.state = PlaybackState::Paused;
        }
        if was_playing && self.state == PlaybackState::Paused {
            self.state = PlaybackState::Playing;
            // Audio restarts from here once the engine has the new edit, as after a seek.
            self.play_anchor = (!self.needs_audio()).then(|| (Instant::now(), self.position_us));
        }
        Ok(())
    }

    pub fn close(&mut self) {
        *self = Self::closed();
    }

    /// Plays at `speed` from here on, carrying on if it is playing. With sound, the audio
    /// device plays faster (and its clock with it); where it cannot, the clock runs on its own.
    pub fn set_speed(&mut self, speed: f64) -> Result<PlaybackStatus, String> {
        self.ensure_open()?;
        if !PLAYBACK_SPEEDS.contains(&speed) {
            return Err("Choose a playback speed of 1x, 1.25x, 1.5x or 2x".into());
        }
        self.advance();
        self.speed = speed;
        if self.state == PlaybackState::Playing {
            let keeps_audio = match self.audio.as_mut() {
                Some(audio) => audio.set_speed(speed),
                None => false,
            };
            if !keeps_audio {
                // Running on the clock alone: from here, at the new speed. A device that
                // cannot play this fast stops; the engine starts one again at normal speed.
                self.audio = None;
                self.play_anchor =
                    (!self.needs_audio()).then(|| (Instant::now(), self.position_us));
            }
        }
        self.status()
    }

    /// Changes when what plays changes; seeks leave it alone.
    pub fn content_generation(&self) -> u64 {
        self.content
    }

    /// Nothing on the audio tracks can be heard (its recording is missing, say), so no audio
    /// device starts: the clock runs on its own rather than waiting for one.
    pub fn run_without_audio(&mut self, generation: u64) {
        if self.generation == generation
            && self.state == PlaybackState::Playing
            && self.audio.is_none()
            && self.play_anchor.is_none()
        {
            self.play_anchor = Some((Instant::now(), self.position_us));
        }
    }

    pub fn play(&mut self) -> Result<PlaybackStatus, String> {
        self.ensure_open()?;
        self.advance();
        if self.state == PlaybackState::Ended {
            self.seek(0)?;
        }
        self.state = PlaybackState::Playing;
        self.play_anchor = if self.needs_audio() {
            None
        } else {
            Some((Instant::now(), self.position_us))
        };
        self.status()
    }

    pub fn pause(&mut self) -> Result<PlaybackStatus, String> {
        self.ensure_open()?;
        self.advance();
        if self.state != PlaybackState::Ended {
            self.state = PlaybackState::Paused;
        }
        self.play_anchor = None;
        self.audio = None;
        self.status()
    }

    pub fn seek(&mut self, edited_us: u64) -> Result<PlaybackStatus, String> {
        self.ensure_open()?;
        let was_playing = self.state == PlaybackState::Playing;
        self.bump_generation();
        self.audio = None;
        self.state = PlaybackState::Seeking;
        self.position_us = edited_us.min(self.duration_us);
        self.play_anchor =
            if was_playing && !self.needs_audio() && self.position_us < self.duration_us {
                Some((Instant::now(), self.position_us))
            } else {
                None
            };
        if self.position_us >= self.duration_us {
            self.state = PlaybackState::Ended;
        } else if was_playing {
            self.state = PlaybackState::Playing;
        } else {
            self.state = PlaybackState::Paused;
        }
        self.status()
    }

    pub fn status(&mut self) -> Result<PlaybackStatus, String> {
        if self.state != PlaybackState::Closed {
            self.advance();
        }
        Ok(self.snapshot())
    }

    fn ensure_open(&self) -> Result<(), String> {
        if self.state == PlaybackState::Closed {
            return Err("Playback is closed".into());
        }
        if self.state == PlaybackState::Error {
            return Err(self
                .error
                .clone()
                .unwrap_or_else(|| "Playback error".into()));
        }
        Ok(())
    }

    fn bump_generation(&mut self) {
        self.preview_available = false;
        self.generation = next_generation();
    }

    fn advance(&mut self) {
        if self.state != PlaybackState::Playing {
            return;
        }
        if let Some(audio) = &self.audio {
            match audio.position_frames() {
                Ok(frames) => {
                    let total = self.audio_start_frame + frames;
                    let end = (self.duration_us as u128 * crate::media::audio::SAMPLE_RATE as u128
                        / 1_000_000) as u64;
                    self.position_us = if total >= end {
                        self.duration_us
                    } else {
                        (total as u128 * 1_000_000 / crate::media::audio::SAMPLE_RATE as u128)
                            as u64
                    };
                }
                Err(error) => {
                    self.fail(self.generation, error);
                    return;
                }
            }
        } else {
            let Some((anchor, start_us)) = self.play_anchor else {
                return;
            };
            let elapsed = Instant::now().saturating_duration_since(anchor).as_micros() as f64;
            self.position_us = start_us
                .saturating_add((elapsed * self.speed) as u64)
                .min(self.duration_us);
        }
        if self.position_us >= self.duration_us {
            self.position_us = self.duration_us;
            self.state = PlaybackState::Ended;
            self.play_anchor = None;
        }
    }

    fn clock_kind(&self) -> ClockKind {
        if self.audio.is_some() {
            ClockKind::Audio
        } else {
            ClockKind::Monotonic
        }
    }
    /// Whether the audio device is the clock (and the engine should start one).
    pub(crate) fn needs_audio(&self) -> bool {
        self.native_enabled && self.has_sound && audio_plays_at(self.speed)
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }
    pub fn fail(&mut self, generation: u64, error: String) {
        if self.generation != generation {
            return;
        }
        self.audio = None;
        self.play_anchor = None;
        self.state = PlaybackState::Error;
        self.error = Some(error);
        self.preview_available = false;
    }
    pub fn mark_presented(&mut self, generation: u64) -> bool {
        if self.generation != generation
            || self.state == PlaybackState::Closed
            || self.state == PlaybackState::Error
        {
            return false;
        }
        self.preview_available = true;
        true
    }

    fn snapshot(&self) -> PlaybackStatus {
        PlaybackStatus {
            project_handle: self.project_handle.clone(),
            state: self.state,
            generation: self.generation,
            position_us: self.position_us,
            duration_us: self.duration_us,
            clock_kind: self.clock_kind(),
            short_id: self.short_focus.clone(),
            speed: self.speed,
            preview_available: self.preview_available,
            error: self.error.clone(),
            diagnostics: self.diagnostics.clone(),
        }
    }
}

/// Whether any audible audio track has clips.
fn has_sound(document: &EditDocument) -> bool {
    document
        .sequence
        .audio_tracks()
        .any(|t| !t.muted && !t.clips.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> EditDocument {
        EditDocument::from_recording(crate::sequence::tests::recording("rec", 10_000_000, &[]))
            .unwrap()
    }

    #[test]
    fn seeks_retire_generations_and_without_a_device_the_clock_is_monotonic() {
        let mut owner = PlaybackOwner::open("h".into(), &document()).unwrap();
        let first = owner.seek(10_000).unwrap();
        let second = owner.seek(20_000).unwrap();
        assert_ne!(first.generation, second.generation);
        assert_eq!(second.clock_kind, ClockKind::Monotonic);
        owner.play().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let playing = owner.status().unwrap();
        assert!(playing.position_us > 20_000);
        assert_eq!(playing.clock_kind, ClockKind::Monotonic);
        // An empty timeline opens, ended.
        let empty = PlaybackOwner::open("e".into(), &EditDocument::default()).unwrap();
        assert_eq!(empty.snapshot().state, PlaybackState::Ended);
    }

    #[test]
    fn with_nothing_audible_the_clock_runs_on_its_own() {
        let mut owner = PlaybackOwner::open("h".into(), &document()).unwrap();
        owner.native_enabled = true;
        let played = owner.play().unwrap();
        // The clock waits for the audio device...
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(owner.status().unwrap().position_us, 0);
        // ...which never starts when the engine finds nothing to hear (a missing recording).
        owner.run_without_audio(played.generation);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let status = owner.status().unwrap();
        assert!(status.position_us > 0);
        assert_eq!(status.clock_kind, ClockKind::Monotonic);
        // A stale request (from before a seek) does nothing.
        let seeked = owner.seek(0).unwrap();
        owner.run_without_audio(played.generation);
        assert_ne!(seeked.generation, played.generation);
        assert_eq!(owner.status().unwrap().position_us, 0);
    }

    #[test]
    fn faster_playback_moves_the_clock_faster() {
        let mut owner = PlaybackOwner::open("h".into(), &document()).unwrap();
        assert!(owner.set_speed(3.0).is_err(), "only the offered speeds");
        // Rates against the time each stretch really took: a busy machine oversleeps.
        let started = std::time::Instant::now();
        owner.play().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(40));
        let normal = owner.status().unwrap().position_us as f64 / started.elapsed().as_secs_f64();
        let at = owner.set_speed(2.0).unwrap();
        let sped_up = std::time::Instant::now();
        assert_eq!(at.speed, 2.0);
        std::thread::sleep(std::time::Duration::from_millis(40));
        let moved = owner.status().unwrap().position_us - at.position_us;
        let fast = moved as f64 / sped_up.elapsed().as_secs_f64();
        assert!(
            fast > normal * 1.5,
            "2x moved {fast:.0} us/s against {normal:.0} us/s at 1x"
        );
        // Pausing keeps the speed for the next play.
        owner.pause().unwrap();
        assert_eq!(owner.status().unwrap().speed, 2.0);
    }

    #[test]
    fn seeks_keep_the_content_and_edits_change_it() {
        let document = document();
        let mut owner = PlaybackOwner::open("h".into(), &document).unwrap();
        let content = owner.content_generation();
        owner.seek(1_000_000).unwrap();
        owner.play().unwrap();
        owner.pause().unwrap();
        assert_eq!(owner.content_generation(), content);
        owner.apply_document(&document).unwrap();
        assert_ne!(owner.content_generation(), content);
    }

    #[test]
    fn edits_keep_playing_and_switching_to_a_short_stops_what_played() {
        let mut document = document();
        document.shorts.push(
            serde_json::from_value(serde_json::json!({
                "id": "s",
                "title": "Short",
                "sourceStartUs": 1_000_000,
                "sourceEndUs": 4_000_000,
            }))
            .unwrap(),
        );
        let mut owner = PlaybackOwner::open("h".into(), &document).unwrap();
        owner.play().unwrap();
        // A change to the project (a short's look, a caption) does not stop playback.
        owner.apply_document(&document).unwrap();
        assert_eq!(owner.status().unwrap().state, PlaybackState::Playing);
        // Handing over to a short stops the video; the caller then plays the short or not.
        owner.focus_short(Some("s".into()), &document, 0).unwrap();
        let status = owner.status().unwrap();
        assert_eq!(status.short_id.as_deref(), Some("s"));
        assert_eq!(status.state, PlaybackState::Paused);
        assert_eq!(status.duration_us, 3_000_000);
        owner.play().unwrap();
        owner.focus_short(None, &document, 0).unwrap();
        assert_eq!(owner.status().unwrap().state, PlaybackState::Paused);
    }
}
