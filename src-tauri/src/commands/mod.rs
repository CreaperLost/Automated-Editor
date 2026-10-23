use crate::dsp::{SilenceConfig, SilenceDetectionResult};
use crate::media::{EncoderGate, MediaInteropStatus, MediaParityReport};
use crate::playback::{
    self, PlaybackOwner, PlaybackStatus, PreviewHitMode, PreviewOwner, PreviewStatus,
    PreviewViewport,
};
use crate::project::{OpenedProject, ProjectReader, SegmentPage, WaveformTrackContext};
use crate::sequence::edit::SequenceEdit;
use crate::sequence::{Role, StreamKind};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub mod graphics;
pub mod history;
pub mod noises;
pub mod preset;
pub mod proposals;
pub mod style;
pub mod transcript;
pub mod transcript_pauses;

pub struct AppState {
    pub command_lock: Mutex<()>,
    pub opened_project: Mutex<Option<crate::project::ProjectReader>>,
    pub playback: Mutex<PlaybackOwner>,
    pub playback_shutdown: std::sync::atomic::AtomicBool,
    pub preview: Mutex<PreviewOwner>,
    /// The quality picked in the stage toolbar; `None` uses the surface's default.
    pub preview_quality: Mutex<Option<playback::PreviewQuality>>,
    /// Woken whenever a new webview preview frame is stored.
    pub preview_frame_ready: tokio::sync::Notify,
    pub encoder_gate: Arc<EncoderGate>,
    pub export: Mutex<crate::export::ExportOwner>,
    /// Bumped when the project opens or closes (or media goes), which stops waveform reads.
    pub waveform_epoch: AtomicU64,
    pub native_capture_enabled: bool,
    /// The renderer behind the Shorts Studio preview, kept while the same short and layout
    /// are being scrubbed.
    pub short_preview: Mutex<Option<ShortPreviewCache>>,
}

pub struct ShortPreviewCache {
    key: String,
    evaluator: crate::export::SceneEvaluator,
}

/// Studio preview frames: 432x768, decoded at no more than twice that.
const SHORT_PREVIEW_SIZE: (u32, u32) = (432, 768);

impl AppState {
    pub fn new() -> Self {
        Self {
            command_lock: Mutex::new(()),
            opened_project: Mutex::new(None),
            playback: Mutex::new(PlaybackOwner::closed()),
            playback_shutdown: std::sync::atomic::AtomicBool::new(false),
            preview: Mutex::new(PreviewOwner::new()),
            preview_quality: Mutex::new(None),
            preview_frame_ready: tokio::sync::Notify::new(),
            encoder_gate: Arc::new(EncoderGate::new()),
            export: Mutex::new(crate::export::ExportOwner::new()),
            waveform_epoch: AtomicU64::new(0),
            // Playback has an audio output (and audio clock) on macOS and Windows.
            native_capture_enabled: cfg!(any(target_os = "macos", windows)),
            short_preview: Mutex::new(None),
        }
    }
}

/// Returns the cross-platform default folder for AeroEdits projects:
/// `Documents/AeroEdits/` on Windows, macOS and Linux.
pub fn default_projects_dir() -> PathBuf {
    let docs_dir = dirs::document_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Documents")))
        .unwrap_or_else(std::env::temp_dir);
    docs_dir.join("AeroEdits")
}

pub fn get_default_projects_dir_impl() -> String {
    default_projects_dir().to_string_lossy().into_owned()
}

impl Default for AppState {
    fn default() -> Self {
        let _ = fs::create_dir_all(default_projects_dir());
        Self::new()
    }
}

pub fn show_in_finder_impl(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .status()
            .map_err(|e| format!("Failed to run open -R: {e}"))?;
        if !status.success() {
            return Err(format!("open -R failed with exit status: {status}"));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        #[cfg(target_os = "windows")]
        {
            // explorer.exe exits with status 1 even when it opens the window,
            // so only a failure to launch it is an error.
            // Explorer only understands `/select,"C:\dir\file"`: the default argument quoting
            // wraps the whole switch in quotes (any path with a space), and it opens Documents
            // instead. It also needs backslashes.
            use std::os::windows::process::CommandExt;
            let native = path.replace('/', "\\");
            let mut command = std::process::Command::new("explorer");
            if p.is_dir() {
                command.raw_arg(format!("\"{native}\""));
            } else {
                command.raw_arg(format!("/select,\"{native}\""));
            }
            command
                .spawn()
                .map_err(|e| format!("Failed to run explorer: {e}"))?;
            Ok(())
        }
        #[cfg(target_os = "linux")]
        {
            let target = if p.is_dir() {
                p
            } else {
                p.parent().unwrap_or(p)
            };
            let status = std::process::Command::new("xdg-open")
                .arg(target)
                .status()
                .map_err(|e| format!("Failed to run xdg-open: {e}"))?;
            if !status.success() {
                return Err(format!("xdg-open failed with exit status: {status}"));
            }
            Ok(())
        }
        #[cfg(all(
            not(target_os = "macos"),
            not(target_os = "windows"),
            not(target_os = "linux")
        ))]
        Ok(())
    }
}

/// What a sound stream's waveform, pauses and speech are read from: `key` is
/// `<asset>.<stream>`. On the timeline, its time maps through the clips that play it;
/// otherwise the asset's own time is the timeline (a waveform drawn per clip).
/// The document a command works on: the project's, or with `short` that short's own timeline
/// (in the short's time, at the project's revision).
pub(crate) fn target_document<'a>(
    reader: &'a ProjectReader,
    short: Option<&str>,
) -> Result<std::borrow::Cow<'a, crate::project::revision::EditDocument>, String> {
    let base = reader.document();
    let Some(id) = short else {
        return Ok(std::borrow::Cow::Borrowed(base));
    };
    let short = base
        .shorts
        .iter()
        .find(|s| s.id == id)
        .ok_or("That short no longer exists")?;
    Ok(std::borrow::Cow::Owned(crate::shorts::short_timeline(
        base, short,
    )?))
}

pub(crate) fn sound_context(
    reader: &ProjectReader,
    key: &str,
    on_timeline: bool,
) -> Result<WaveformTrackContext, String> {
    sound_context_in(reader, reader.document(), key, on_timeline)
}

/// [`sound_context`] placed on `document`'s timeline (a short's own, say).
pub(crate) fn sound_context_in(
    reader: &ProjectReader,
    document: &crate::project::revision::EditDocument,
    key: &str,
    on_timeline: bool,
) -> Result<WaveformTrackContext, String> {
    let source = document.stream_ref(key)?;
    let asset = document.asset(&source.asset).ok_or("Unknown sound")?;
    let stream = asset.stream(&source.stream).ok_or("Unknown sound")?;
    if stream.kind != StreamKind::Sound {
        return Err("That stream has no sound".into());
    }
    let segments = crate::sequence::sources::sound_segments(reader.root(), asset, &source.stream)?;
    let (retained, edited_duration_us) = if on_timeline {
        (
            crate::sequence::clock::stream_entries(&document.sequence, &asset.id, &stream.id),
            document.duration_us(),
        )
    } else {
        (
            vec![crate::project::RetainedInterval::recording(
                0,
                asset.duration_us,
            )],
            asset.duration_us,
        )
    };
    Ok(WaveformTrackContext {
        root: reader.root().to_path_buf(),
        track_id: key.to_string(),
        track_type: crate::sequence::sources::track_type(stream.role),
        segments,
        retained,
        edited_duration_us,
        muted: document
            .noise_mutes
            .get(key)
            .map(|ranges| ranges.iter().map(|r| (r.start_us, r.end_us)).collect())
            .unwrap_or_default(),
    })
}

/// Asks [`detect_silence_impl`] for the pauses in all the speech on the timeline at once.
pub const ALL_SPEECH: &str = "speech";

/// Finds pauses in a sound stream (`<asset>.<stream>`), in timeline time through its clips, or
/// with [`ALL_SPEECH`], in every sound the timeline plays as speech (see
/// [`crate::project::silence::common_pauses`]).
pub fn detect_silence_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    config: SilenceConfig,
) -> Result<SilenceDetectionResult, String> {
    detect_silence_for(state, project_handle, track_id, config, None)
}

/// [`detect_silence_impl`] on the project's timeline, or on short `short`'s own.
pub fn detect_silence_for(
    state: &AppState,
    project_handle: String,
    track_id: String,
    config: SilenceConfig,
    short: Option<&str>,
) -> Result<SilenceDetectionResult, String> {
    config.validate()?;
    if track_id == ALL_SPEECH {
        return detect_combined_silence(state, &project_handle, &config, None, short);
    }
    let (ctx, transcript, revision) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        let document = target_document(reader, short)?;
        (
            sound_context_in(reader, &document, &track_id, true)?,
            crate::transcript::store::load_transcript(reader.root(), &track_id)?,
            reader.document().revision,
        )
    };
    let mut result = crate::project::silence::detect_track_silence_with_words(
        &ctx,
        &config,
        transcript.as_ref(),
    )?;
    result.transcript_dependencies = vec![transcript.as_ref().map(|t| t.dependency()).unwrap_or(
        crate::transcript::TranscriptDependency {
            track_id,
            word_stamp: None,
        },
    )];
    transcript_pauses::validate_analysis(
        state,
        &project_handle,
        revision,
        &result.transcript_dependencies,
    )?;
    Ok(result)
}

/// Every sound the timeline plays as speech (its own role, or its track's), with where its
/// clips play: muted tracks left out.
fn speech_on_timeline(
    document: &crate::project::revision::EditDocument,
) -> Vec<(String, Vec<crate::sequence::Clip>)> {
    let mut speech: Vec<(String, Vec<crate::sequence::Clip>)> = Vec::new();
    for track in document.sequence.tracks.iter().filter(|t| !t.muted) {
        for clip in &track.clips {
            if crate::sequence::clip_role(&document.assets, track, clip) != Some(Role::Mic) {
                continue;
            }
            let key = crate::sequence::StreamRef::new(&clip.asset, &clip.stream).key();
            match speech.iter_mut().find(|(k, _)| *k == key) {
                Some((_, clips)) => clips.push(clip.clone()),
                None => speech.push((key, vec![clip.clone()])),
            }
        }
    }
    speech
}

/// Explicit sources may include desktop/voice-mode audio regardless of their speech role.
/// Existing callers keep selecting one stream or ALL_SPEECH through detect_silence_impl.
pub fn detect_silence_sources_impl(
    state: &AppState,
    project_handle: String,
    track_ids: Vec<String>,
    config: SilenceConfig,
) -> Result<SilenceDetectionResult, String> {
    detect_silence_sources_for(state, project_handle, track_ids, config, None)
}

/// [`detect_silence_sources_impl`] on the project's timeline, or on short `short`'s own.
pub fn detect_silence_sources_for(
    state: &AppState,
    project_handle: String,
    track_ids: Vec<String>,
    config: SilenceConfig,
    short: Option<&str>,
) -> Result<SilenceDetectionResult, String> {
    config.validate()?;
    if track_ids.len() != 2 || track_ids[0] == track_ids[1] {
        return Err("Choose two different sound sources".into());
    }
    detect_combined_silence(state, &project_handle, &config, Some(&track_ids), short)
}

fn detect_combined_silence(
    state: &AppState,
    project_handle: &str,
    config: &SilenceConfig,
    track_ids: Option<&[String]>,
    short: Option<&str>,
) -> Result<SilenceDetectionResult, String> {
    let (sounds, revision) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, project_handle)?;
        let document = target_document(reader, short)?;
        let selected = if let Some(keys) = track_ids {
            keys.iter()
                .map(|key| {
                    let plays = document
                        .sequence
                        .tracks
                        .iter()
                        .filter(|track| !track.muted)
                        .flat_map(|track| &track.clips)
                        .filter(|clip| {
                            crate::sequence::StreamRef::new(&clip.asset, &clip.stream).key() == *key
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    if plays.is_empty() {
                        return Err(
                            "Both chosen sounds must be on an unmuted timeline track".to_string()
                        );
                    }
                    Ok((key.clone(), plays))
                })
                .collect::<Result<Vec<_>, String>>()?
        } else {
            speech_on_timeline(&document)
        };
        selected
            .into_iter()
            // Scan source time once, then map through every audible clip below. A sound
            // can play twice, even overlapping itself with different source positions.
            .map(|(key, plays)| {
                Ok((
                    sound_context(reader, &key, false)?,
                    plays,
                    crate::transcript::store::load_transcript(reader.root(), &key)?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()
            .map(|sounds| (sounds, document.revision))?
    };
    if sounds.is_empty() {
        return Err("Nothing on the timeline is marked as speech: choose a sound to scan".into());
    }
    // Each sound is read on a thread of its own.
    let found = std::thread::scope(|scope| {
        let jobs: Vec<_> = sounds
            .iter()
            .map(|(ctx, _, transcript)| {
                scope.spawn(move || {
                    crate::project::silence::detect_track_silence_with_words(
                        ctx,
                        config,
                        transcript.as_ref(),
                    )
                })
            })
            .collect();
        jobs.into_iter()
            .map(|job| {
                job.join()
                    .unwrap_or_else(|_| Err("Silence detection failed".into()))
            })
            .collect::<Result<Vec<_>, String>>()
    })?;
    let lists: Vec<crate::project::silence::PlaysAndPauses> = sounds
        .iter()
        .zip(&found)
        .flat_map(|((_, clips, _), result)| {
            clips.iter().map(move |clip| {
                let silent = result
                    .suggestions
                    .iter()
                    .filter_map(|pause| clip.timeline_range(pause.start_us, pause.end_us))
                    .collect();
                (vec![(clip.start_us, clip.end_us())], silent)
            })
        })
        .collect();
    let min_us =
        u64::from(config.min_duration_ms).saturating_sub(2 * u64::from(config.padding_ms)) * 1_000;
    let mut diagnostics: Vec<String> = Vec::new();
    for result in &found {
        for line in &result.diagnostics {
            if !diagnostics.contains(line) {
                diagnostics.push(line.clone());
            }
        }
    }
    let pauses = crate::project::silence::common_pauses(&lists, min_us);
    let first = &found[0];
    let dependencies = sounds
        .iter()
        .map(|(ctx, _, transcript)| {
            transcript.as_ref().map(|t| t.dependency()).unwrap_or(
                crate::transcript::TranscriptDependency {
                    track_id: ctx.track_id.clone(),
                    word_stamp: None,
                },
            )
        })
        .collect::<Vec<_>>();
    transcript_pauses::validate_analysis(state, project_handle, revision, &dependencies)?;
    Ok(SilenceDetectionResult {
        track_id: ALL_SPEECH.into(),
        sample_rate: first.sample_rate,
        channels: first.channels,
        channel_policy: first.channel_policy.clone(),
        suggestions: pauses
            .into_iter()
            .enumerate()
            .map(|(i, (start_us, end_us))| crate::dsp::SilenceCutInterval {
                id: format!("silence-{}", i + 1),
                start_us,
                end_us,
                duration_ms: (end_us - start_us) / 1_000,
                selected: true,
                source_start_us: start_us,
                source_end_us: end_us,
            })
            .collect(),
        diagnostics,
        thresholds: found
            .iter()
            .flat_map(|result| result.thresholds.clone())
            .collect(),
        transcript_dependencies: dependencies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captions_take_a_chosen_sound_by_its_stream_key() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, key) = crate::mcp::tests_support::lesson(dir.path());
        let state = &host.state;
        let opened = state
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .summary
            .clone();
        let captions = |track_id: &str| crate::captions::CaptionSettings {
            enabled: true,
            track_id: Some(track_id.into()),
            ..Default::default()
        };
        let saved = project_captions_update_impl(
            state,
            opened.project_handle.clone(),
            opened.revision,
            captions(&key),
        )
        .unwrap();
        assert_eq!(saved.captions.track_id.as_deref(), Some(key.as_str()));
        let (asset, _) = key.split_once('.').unwrap();
        for missing in [format!("{asset}.nothing"), "nothing.sound0".to_string()] {
            let error = project_captions_update_impl(
                state,
                saved.project_handle.clone(),
                saved.revision,
                captions(&missing),
            )
            .unwrap_err();
            assert!(error.contains("is not a sound"), "{error}");
        }
    }

    #[test]
    fn silence_cuts_cover_the_whole_clip_beyond_256_pauses() {
        use crate::fixtures::{generate_pcm16_wav, TestProject};
        use crate::project::{JournalRecord, TrackDescriptor, TrackType};
        const PAUSES: u32 = 400;
        const RATE: u32 = 8_000;
        let duration_us = u64::from(PAUSES) * 1_000_000;
        let dir = tempfile::tempdir().unwrap();
        let mut bundle = TestProject::create(dir.path(), "many-pauses");
        for (id, track_type) in [
            ("mic", TrackType::MicAudio),
            ("system", TrackType::SystemAudio),
        ] {
            // A 600 ms pause each second, with speech on either side including the end
            // of the clip. Balanced should find every pause, not only the first 256.
            let samples: Vec<i16> = (0..RATE * PAUSES)
                .map(|i| {
                    if (RATE * 3 / 10..RATE * 9 / 10).contains(&(i % RATE)) {
                        0
                    } else {
                        (((i % RATE) as f32 * 110.0 * std::f32::consts::TAU / RATE as f32).sin()
                            * 3_000.0) as i16
                    }
                })
                .collect();
            let wav = generate_pcm16_wav(RATE, 1, &samples);
            let relative_path = format!("media/{id}/voice.wav");
            fs::write(bundle.root_path().join(&relative_path), &wav).unwrap();
            bundle.manifest_mut().tracks.push(TrackDescriptor {
                id: id.into(),
                track_type,
                codec: "pcm".into(),
                relative_path: relative_path.clone(),
                width: None,
                height: None,
                fps: None,
                sample_rate: Some(RATE),
                channels: Some(1),
                gaps_total: 0,
                media_timescale: Some(RATE),
            });
            bundle.append_journal(JournalRecord::SegmentCommitted {
                seq: 0,
                track_id: id.into(),
                relative_path,
                start_us: 0,
                end_us: duration_us,
                size_bytes: wav.len() as u64,
                is_keyframe_start: true,
                media_timescale: RATE,
                media_start_value: 0,
                host_anchor_us: 0,
            });
        }
        bundle.manifest_mut().duration_us = duration_us;
        bundle.manifest_mut().active_duration_us = duration_us;
        bundle.save_manifest();
        let folder = crate::project::folder::create_project_folder(
            dir.path(),
            "Edit",
            Some(bundle.root_path()),
        )
        .unwrap();
        let state = AppState::new();
        let original = open_project_impl(&state, folder.to_string_lossy().into()).unwrap();
        let keys: Vec<_> = original
            .assets
            .iter()
            .flat_map(|asset| {
                asset
                    .streams
                    .iter()
                    .filter(|stream| stream.kind == StreamKind::Sound)
                    .map(|stream| crate::sequence::StreamRef::new(&asset.id, &stream.id).key())
            })
            .collect();
        let config = SilenceConfig {
            auto_level: Some(0.2),
            min_duration_ms: 500,
            padding_ms: 120,
            ..SilenceConfig::default()
        };
        let single = detect_silence_impl(
            &state,
            original.project_handle.clone(),
            keys[0].clone(),
            config.clone(),
        )
        .unwrap();
        assert_eq!(single.suggestions.len(), PAUSES as usize);
        let all = detect_silence_impl(
            &state,
            original.project_handle.clone(),
            ALL_SPEECH.into(),
            config.clone(),
        )
        .unwrap();
        assert_eq!(all.suggestions.len(), PAUSES as usize);
        let combined =
            detect_silence_sources_impl(&state, original.project_handle.clone(), keys, config)
                .unwrap();
        assert_eq!(combined.suggestions.len(), PAUSES as usize);
        assert_eq!(single.suggestions.last(), combined.suggestions.last());
        assert!(combined.suggestions.last().unwrap().start_us >= duration_us - 1_000_000);
        let removed: u64 = combined
            .suggestions
            .iter()
            .map(|pause| pause.end_us - pause.start_us)
            .sum();
        let applied = project_sequence_edit_impl(
            &state,
            original.project_handle.clone(),
            original.revision,
            SequenceEdit::DeleteRange {
                ranges: combined
                    .suggestions
                    .iter()
                    .map(|pause| crate::zoom::EditedRange {
                        start_us: pause.start_us,
                        end_us: pause.end_us,
                    })
                    .collect(),
                ripple: Some(true),
            },
            None,
        )
        .unwrap();
        assert_eq!(applied.duration_us, duration_us - removed);
        assert_eq!(applied.revision, original.revision + 1);
        for track in &applied.sequence.tracks {
            assert_eq!(track.clips.len(), PAUSES as usize + 1);
            assert_eq!(track.clips.last().unwrap().end_us(), applied.duration_us);
        }
        let reopened = ProjectReader::open(&folder).unwrap();
        assert_eq!(reopened.document().sequence, applied.sequence);
        let restored =
            project_undo_impl(&state, original.project_handle, applied.revision, None).unwrap();
        assert_eq!(restored.sequence, original.sequence);
        assert_eq!(restored.duration_us, duration_us);
    }

    #[test]
    fn two_source_silence_keeps_both_voices_and_single_source_still_works() {
        use crate::fixtures::{generate_pcm16_wav, TestProject};
        use crate::project::{JournalRecord, TrackDescriptor, TrackType};
        let dir = tempfile::tempdir().unwrap();
        let mut bundle = TestProject::create(dir.path(), "conversation");
        let rate = 16_000;
        for (id, track_type, amplitude) in [
            ("mic", TrackType::MicAudio, 1_000),
            ("system", TrackType::SystemAudio, 10_000),
        ] {
            let relative_path = format!("media/{id}/voice.wav");
            let samples: Vec<i16> = (0..rate * 8)
                .map(|i| {
                    let t = i as f32 / rate as f32;
                    let speaking = if id == "mic" {
                        t < 2.0 || (4.0..5.0).contains(&t)
                    } else {
                        (2.0..4.0).contains(&t) || (5.0..6.0).contains(&t)
                    };
                    if speaking {
                        // A quiet mic phrase while system audio is silent must survive
                        // both the single-source scan and the combined scan.
                        let spoken_amplitude = if id == "mic" && (0.5..1.5).contains(&t) {
                            amplitude / 10
                        } else {
                            amplitude
                        };
                        ((t * 440.0 * std::f32::consts::TAU).sin() * spoken_amplitude as f32) as i16
                    } else {
                        0
                    }
                })
                .collect();
            let wav = generate_pcm16_wav(rate, 1, &samples);
            fs::write(bundle.root_path().join(&relative_path), &wav).unwrap();
            bundle.manifest_mut().tracks.push(TrackDescriptor {
                id: id.into(),
                track_type,
                codec: "pcm".into(),
                relative_path: relative_path.clone(),
                width: None,
                height: None,
                fps: None,
                sample_rate: Some(rate),
                channels: Some(1),
                gaps_total: 0,
                media_timescale: Some(rate),
            });
            bundle.append_journal(JournalRecord::SegmentCommitted {
                seq: 0,
                track_id: id.into(),
                relative_path,
                start_us: 0,
                end_us: 8_000_000,
                size_bytes: wav.len() as u64,
                is_keyframe_start: true,
                media_timescale: rate,
                media_start_value: 0,
                host_anchor_us: 0,
            });
        }
        bundle.manifest_mut().duration_us = 8_000_000;
        bundle.manifest_mut().active_duration_us = 8_000_000;
        bundle.save_manifest();
        let folder = crate::project::folder::create_project_folder(
            dir.path(),
            "Edit",
            Some(bundle.root_path()),
        )
        .unwrap();
        let reader = ProjectReader::open(&folder).unwrap();
        let handle = reader.summary.project_handle.clone();
        let keys: Vec<String> = reader
            .document()
            .assets
            .iter()
            .flat_map(|asset| {
                asset
                    .streams
                    .iter()
                    .filter(|stream| stream.kind == StreamKind::Sound)
                    .map(|stream| crate::sequence::StreamRef::new(&asset.id, &stream.id).key())
            })
            .collect();
        assert_eq!(keys.len(), 2);
        let mic_key = reader
            .document()
            .assets
            .iter()
            .find_map(|asset| {
                asset
                    .streams
                    .iter()
                    .find(|stream| stream.role == Role::Mic)
                    .map(|stream| crate::sequence::StreamRef::new(&asset.id, &stream.id).key())
            })
            .unwrap();
        let state = AppState::new();
        *state.opened_project.lock() = Some(reader);
        let config = SilenceConfig {
            auto_level: Some(0.2),
            min_duration_ms: 500,
            padding_ms: 120,
            ..SilenceConfig::default()
        };
        let combined =
            detect_silence_sources_impl(&state, handle.clone(), keys.clone(), config.clone())
                .unwrap();
        assert_eq!(combined.suggestions.len(), 1, "{:?}", combined.suggestions);
        assert!(combined.suggestions[0].start_us >= 6_120_000);
        assert!(combined.suggestions[0].end_us <= 7_880_000);
        assert_eq!(combined.thresholds.len(), 2);
        assert!(
            (combined.thresholds[0].threshold_db - combined.thresholds[1].threshold_db).abs()
                > 30.0
        );
        let single = detect_silence_impl(&state, handle.clone(), mic_key, config.clone()).unwrap();
        assert_eq!(single.suggestions.len(), 2, "{:?}", single.suggestions);
        assert!(single
            .suggestions
            .iter()
            .all(|pause| pause.end_us <= 500_000 || pause.start_us >= 1_500_000));
        let all_speech =
            detect_silence_impl(&state, handle.clone(), ALL_SPEECH.into(), config.clone()).unwrap();
        assert_eq!(
            all_speech.thresholds.len(),
            1,
            "System audio retains its background role"
        );
        assert!(all_speech.suggestions.len() > combined.suggestions.len());
        assert!(detect_silence_sources_impl(
            &state,
            handle.clone(),
            vec![keys[0].clone(); 2],
            config.clone()
        )
        .is_err());
        assert!(
            detect_silence_sources_impl(&state, "stale".into(), keys.clone(), config.clone())
                .is_err()
        );
        // Repeat the mic's first spoken second over the final shared pause. It must protect
        // that second even though a different part of the same source is silent underneath.
        {
            let mut opened = state.opened_project.lock();
            let reader = opened.as_mut().unwrap();
            let mic = reader
                .document()
                .assets
                .iter()
                .find_map(|asset| {
                    asset
                        .streams
                        .iter()
                        .find(|stream| stream.role == Role::Mic)
                        .map(|stream| (asset.id.clone(), stream.id.clone()))
                })
                .unwrap();
            reader
                .edit_sequence(
                    reader.summary.revision,
                    &SequenceEdit::SetMagnetic { magnetic: false },
                    None,
                )
                .unwrap();
            reader
                .edit_sequence(
                    reader.summary.revision,
                    &SequenceEdit::PlaceAsset {
                        asset_id: mic.0,
                        at_us: 6_000_000,
                        track_id: None,
                        streams: vec![mic.1],
                        range: Some(crate::sequence::SourceRange {
                            start_us: 0,
                            end_us: 1_000_000,
                        }),
                    },
                    None,
                )
                .unwrap();
        }
        let repeated = detect_silence_sources_impl(&state, handle, keys, config).unwrap();
        assert_eq!(repeated.suggestions.len(), 1, "{:?}", repeated.suggestions);
        assert!(repeated.suggestions[0].start_us >= 7_000_000);
    }

    #[test]
    fn test_window_title_formatting() {
        assert_eq!(
            window_title_for_project(Some("Launch Demo")),
            "AeroEdits \u{2014} Launch Demo"
        );
        assert_eq!(window_title_for_project(None), "AeroEdits");
        assert_eq!(window_title_for_project(Some("")), "AeroEdits");
        assert_eq!(window_title_for_project(Some("   ")), "AeroEdits");
    }

    #[test]
    fn preview_without_a_native_window_attaches_to_the_webview() {
        let state = AppState::new();
        let status =
            preview_attach_impl(&state, "main".into(), PreviewHitMode::Consume, None).unwrap();
        assert!(status.attached);
        assert_eq!(status.surface, "webview");
    }

    #[test]
    fn test_show_in_finder_impl_missing_path() {
        let non_existent = "/tmp/does-not-exist-aeroedits-test-finder-12345";
        let err = show_in_finder_impl(non_existent.into()).unwrap_err();
        assert!(err.contains("Path does not exist"));
    }

    // Opens a real Finder window, so it only runs on macOS.
    #[cfg(target_os = "macos")]
    #[test]
    fn test_show_in_finder_impl() {
        let dir = tempfile::tempdir().unwrap();
        let result = show_in_finder_impl(dir.path().to_string_lossy().into_owned());
        assert!(result.is_ok());
    }
}

pub fn open_project_impl(state: &AppState, path: String) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let reader = ProjectReader::open(std::path::Path::new(&path))?;
    let summary = reader.summary.clone();
    let document = reader.history().current.clone();
    let mut owner = PlaybackOwner::open(summary.project_handle.clone(), &document)?;
    // Every camera is framed on the face unless that is off: find it in the background.
    if document.layout.face_framing || !document.shorts.is_empty() {
        crate::face::ensure(reader.root(), &document, false);
    }
    *state.opened_project.lock() = Some(reader);
    owner.native_enabled = state.native_capture_enabled;
    *state.playback.lock() = owner;
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    Ok(summary)
}

/// Where each camera stream's face track stands (shorts frame their camera on it); `start`
/// begins finding the ones that are missing or failed.
pub fn project_face_tracks_impl(
    state: &AppState,
    project_handle: String,
    start: bool,
) -> Result<Vec<crate::face::FaceTrackStatus>, String> {
    let (root, document) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        (reader.root().to_path_buf(), reader.document().clone())
    };
    if start {
        crate::face::ensure(&root, &document, true);
    }
    Ok(crate::face::status(&root, &document))
}

pub fn close_project_impl(state: &AppState, project_handle: String) -> Result<(), String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    if let Some(reader) = opened.as_ref() {
        if reader.summary.project_handle != project_handle {
            return Err("Stale project handle".into());
        }
    }
    *opened = None;
    state.playback.lock().close();
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

pub fn project_segments_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    offset: usize,
    limit: usize,
) -> Result<SegmentPage, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    if reader.summary.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    reader.page(&track_id, offset, limit)
}

/// A sound's whole waveform (`<asset>.<stream>`), fine enough to see pauses, on its own time:
/// the timeline draws each clip's part of it.
pub fn project_waveform_overview_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
) -> Result<crate::project::waveform::WaveformOverview, String> {
    let epoch = state.waveform_epoch.load(Ordering::SeqCst);
    let ctx = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        sound_context(reader, &track_id, false)?
    };
    crate::project::waveform::waveform_overview(&ctx, &|| {
        state.waveform_epoch.load(Ordering::SeqCst) != epoch
    })
}

/// Zoom suggestions from the mouse data of each recording, each on its own clock and placed
/// where its screen plays.
fn all_zoom_suggestions(
    reader: &crate::project::reader::ProjectReader,
    config: &crate::zoom::ZoomConfig,
) -> Result<crate::zoom::ZoomGeneration, String> {
    let document = reader.document();
    let mut generation = crate::zoom::ZoomGeneration {
        version: config.generation_version,
        config: config.clone(),
        suggestions: Vec::new(),
        diagnostics: Vec::new(),
    };
    for asset in reader.recordings() {
        let stream =
            match crate::telemetry::reader::read_telemetry(std::path::Path::new(&asset.path)) {
                Ok(stream) => stream,
                Err(error) => {
                    generation
                        .diagnostics
                        .push(format!("{}: {error}", asset.name));
                    continue;
                }
            };
        let mut found = crate::zoom::generate_zoom_suggestions(&stream, config)?;
        crate::zoom::attach_edited_ranges(&mut found, &document.picture_clock(&asset.id));
        generation.diagnostics.extend(found.diagnostics);
        for mut suggestion in found.suggestions {
            // Ids stay unique across recordings.
            suggestion.id = format!("{}:{}", asset.id, suggestion.id);
            suggestion.media = Some(asset.id.clone());
            generation.suggestions.push(suggestion);
        }
    }
    // Zooms scale with the edit, not the raw recordings: moments cut out of the edit take
    // no place, and the strongest of the rest are kept, back in time order.
    generation
        .suggestions
        .retain(|s| !s.edited_ranges.is_empty());
    let budget = document.zoom_settings.budget(document.duration_us());
    if generation.suggestions.len() > budget {
        let mut ranked: Vec<usize> = (0..generation.suggestions.len()).collect();
        ranked.sort_by(|&a, &b| {
            generation.suggestions[b]
                .score
                .total_cmp(&generation.suggestions[a].score)
                .then(a.cmp(&b))
        });
        let keep: std::collections::BTreeSet<usize> = ranked.into_iter().take(budget).collect();
        let mut index = 0;
        generation.suggestions.retain(|_| {
            index += 1;
            keep.contains(&(index - 1))
        });
    }
    Ok(generation)
}

pub fn project_zoom_suggestions_impl(
    state: &AppState,
    project_handle: String,
    _config: Option<crate::zoom::ZoomConfig>,
) -> Result<crate::zoom::ZoomGeneration, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    if reader.summary.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    // The project's own settings, so every window finds the same zooms.
    let config = reader.history().current.zoom_settings.config();
    zoom_suggestions_with(reader, &config)
}

/// The zooms `settings` would suggest, without saving them: an agent's dry run of other settings.
pub fn project_zoom_preview_impl(
    state: &AppState,
    project_handle: String,
    settings: &crate::zoom::ZoomSettings,
) -> Result<crate::zoom::ZoomGeneration, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    if reader.summary.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    zoom_suggestions_with(reader, &settings.config())
}

/// The suggestions found with `config`, less the zooms already in the edit or dismissed.
fn zoom_suggestions_with(
    reader: &crate::project::ProjectReader,
    config: &crate::zoom::ZoomConfig,
) -> Result<crate::zoom::ZoomGeneration, String> {
    let mut generation = all_zoom_suggestions(reader, config)?;
    let taken: std::collections::BTreeSet<_> = reader
        .summary
        .zooms
        .iter()
        .map(|z| z.id.clone())
        .chain(reader.summary.dismissed_zoom_ids.iter().cloned())
        .collect();
    generation
        .suggestions
        .retain(|suggestion| !taken.contains(&suggestion.id));
    Ok(generation)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ManualZoomInput {
    pub edited_start_us: u64,
    pub edited_end_us: u64,
    pub center_x: f64,
    pub center_y: f64,
    pub scale: f64,
}

pub(crate) fn mutate_opened(
    state: &AppState,
    project_handle: String,
    mutate: impl FnOnce(&mut crate::project::ProjectReader) -> Result<OpenedProject, String>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let summary = mutate(reader)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

/// Saves the project's auto-zoom settings (automatic zooms take the new amounts).
pub fn project_zoom_settings_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    settings: crate::zoom::ZoomSettings,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.set_zoom_settings(expected_revision, settings)
    })
}

/// Puts the recording's zooms back, found with the project's auto-zoom settings.
pub fn project_zoom_reload_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    _config: Option<crate::zoom::ZoomConfig>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        let config = reader.history().current.zoom_settings.config();
        let generation = all_zoom_suggestions(reader, &config)?;
        if generation.suggestions.is_empty() {
            return Err(generation
                .diagnostics
                .last()
                .cloned()
                .unwrap_or_else(|| "The recording has no mouse activity to zoom on".into()));
        }
        reader.reload_zooms(expected_revision, &generation.suggestions)
    })
}

pub fn project_zoom_accept_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    ids: Vec<String>,
    _config: Option<crate::zoom::ZoomConfig>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        // The settings the suggestions were found with, so what is accepted is what was shown.
        let config = reader.history().current.zoom_settings.config();
        let generation = all_zoom_suggestions(reader, &config)?;
        let selected: Vec<_> = if ids.is_empty() {
            generation.suggestions
        } else {
            generation
                .suggestions
                .into_iter()
                .filter(|s| ids.iter().any(|id| id == &s.id))
                .collect()
        };
        reader.accept_zooms(expected_revision, &selected)
    })
}

pub fn project_zoom_dismiss_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    ids: Vec<String>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.dismiss_zooms(expected_revision, &ids)
    })
}

pub fn project_zoom_update_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    zoom: crate::zoom::ZoomKeyframe,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.update_zoom(expected_revision, zoom)
    })
}

pub fn project_zoom_add_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    input: ManualZoomInput,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.add_manual_zoom(
            expected_revision,
            input.edited_start_us,
            input.edited_end_us,
            input.center_x,
            input.center_y,
            input.scale,
        )
    })
}

pub fn project_zoom_delete_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    id: String,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.delete_zoom(expected_revision, &id)
    })
}

pub fn project_layout_update_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    mut layout: crate::project::EditLayout,
    wallpaper_source: Option<String>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        if let Some(source) = wallpaper_source {
            let relative = crate::project::layout::ingest_wallpaper(
                reader.root(),
                std::path::Path::new(&source),
            )?;
            layout.wallpaper_asset = Some(relative);
        }
        reader.update_layout(expected_revision, layout)
    })
}

pub fn project_audio_update_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    audio: crate::project::AudioSettings,
) -> Result<OpenedProject, String> {
    audio.validate()?;
    // Prepare outside the project/command locks, then let the revision check fence the
    // commit. A failed repair never saves an enabled effect that cannot be played.
    let prepared = if audio.mouth_clicks {
        let (root, document) = {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().ok_or("No opened project")?;
            require_handle(reader, &project_handle)?;
            if reader.document().revision != expected_revision {
                return Err("Stale edit revision".into());
            }
            (reader.root().to_path_buf(), reader.document().clone())
        };
        Some(crate::media::declick::prepare(
            &root,
            &audio,
            &crate::media::audio::lanes(&root, &document),
        )?)
    } else {
        None
    };
    let _prepared = prepared;
    mutate_opened(state, project_handle, |reader| {
        reader.update_audio(expected_revision, audio)
    })
}

pub fn project_captions_update_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    captions: crate::captions::CaptionSettings,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.update_captions(expected_revision, captions)
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutPick {
    pub project: OpenedProject,
    /// Sections picked by this run.
    pub picked: usize,
    pub diagnostics: Vec<String>,
}

/// Sections the pick rules found, not stored yet.
pub struct LayoutsFound {
    pub sections: Vec<crate::layouts::LayoutSection>,
    pub settings: crate::layouts::pick::PickSettings,
    pub diagnostics: Vec<String>,
}

/// Picks the layout sections of the video (`short` none) or a short from its recording (see
/// [`crate::layouts::pick`]), with `settings` or the timeline's own, and stores them in place
/// of the earlier picks; sections made by hand stay. Reading the mic and the mouse happens
/// outside the project lock; the commit checks the revision.
pub fn project_layouts_pick_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short: Option<String>,
    settings: Option<crate::layouts::pick::PickSettings>,
) -> Result<LayoutPick, String> {
    let found = layouts_find_impl(state, &project_handle, short.as_deref(), settings)?;
    layouts_store_impl(state, project_handle, expected_revision, short, found)
}

/// Stores what [`layouts_find_impl`] found.
pub fn layouts_store_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short: Option<String>,
    found: LayoutsFound,
) -> Result<LayoutPick, String> {
    let picked = found.sections.len();
    let project = mutate_opened(state, project_handle, |reader| {
        reader.change_layouts(expected_revision, short.as_deref(), |_, track| {
            track.pick = found.settings;
            track.replace_auto(found.sections);
            Ok(())
        })
    })?;
    Ok(LayoutPick {
        project,
        picked,
        diagnostics: found.diagnostics,
    })
}

/// What the pick rules find on the video or a short, without storing it.
pub fn layouts_find_impl(
    state: &AppState,
    project_handle: &str,
    short: Option<&str>,
    settings: Option<crate::layouts::pick::PickSettings>,
) -> Result<LayoutsFound, String> {
    use crate::layouts::pick::{self, PickInput};
    // What to read, worked out under the lock.
    struct Plan {
        settings: pick::PickSettings,
        media: String,
        folder: std::path::PathBuf,
        range: (u64, u64),
        move_us: u64,
        /// Speech from the transcript's words, or the mic to scan for it.
        words: Option<Vec<(u64, u64)>>,
        mic: Option<WaveformTrackContext>,
    }
    let plan = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, project_handle)?;
        let document = reader.document();
        let (track, media, range) = match short {
            Some(id) => {
                let short = document
                    .shorts
                    .iter()
                    .find(|s| s.id == id)
                    .ok_or("That short no longer exists")?;
                // A take: the camera fills the frame wherever no footage plays.
                if short.take.is_some() {
                    return Ok(LayoutsFound {
                        sections: crate::take_short::camera_sections(short)?,
                        settings: settings.unwrap_or_else(|| short.layouts.pick.clone()),
                        diagnostics: Vec::new(),
                    });
                }
                let media = document
                    .clock_asset(short.media.as_deref())
                    .ok_or("This short plays no recording")?;
                (
                    &short.layouts,
                    media.to_string(),
                    (short.source_start_us, short.source_end_us),
                )
            }
            None => {
                // The recording with a camera, else the first.
                let recordings = || document.assets.iter().filter(|a| a.is_recording());
                let asset = recordings()
                    .find(|a| a.streams.iter().any(|s| s.role == Role::Webcam))
                    .or_else(|| recordings().next())
                    .ok_or("Layouts are picked from a recording's mouse data")?;
                (&document.layouts, asset.id.clone(), (0, asset.duration_us))
            }
        };
        let settings = settings.unwrap_or_else(|| track.pick.clone());
        settings.validate()?;
        let asset = document
            .asset(&media)
            .filter(|a| a.is_recording())
            .ok_or("Layouts are picked from a recording's mouse data; this is a file")?;
        if !asset.streams.iter().any(|s| s.role == Role::Webcam) {
            return Err("This recording has no camera, so it shows the screen throughout".into());
        }
        let mic = asset.streams.iter().find(|s| s.role == Role::Mic);
        let key = mic.map(|s| crate::sequence::StreamRef::new(&asset.id, &s.id).key());
        // A short reads its words; the video scans the mic, as the speech threshold says.
        let words = match (short, &key) {
            (Some(_), Some(key)) => crate::transcript::store::load_transcript(reader.root(), key)
                .ok()
                .flatten()
                .map(|t| {
                    t.words
                        .iter()
                        .filter(|w| w.kind == crate::transcript::WordKind::Word)
                        .filter(|w| w.source_end_us > range.0 && w.source_start_us < range.1)
                        .map(|w| (w.source_start_us, w.source_end_us))
                        .collect()
                }),
            _ => None,
        };
        let mic = match (settings.require_speech && words.is_none(), &key) {
            (true, None) => {
                return Err("This recording has no microphone track to find speech on".into())
            }
            (true, Some(key)) => Some(sound_context(reader, key, false)?),
            (false, _) => None,
        };
        Plan {
            move_us: track.transition_ms as u64 * 1_000,
            settings,
            media,
            folder: std::path::PathBuf::from(&asset.path),
            range,
            words,
            mic,
        }
    };
    let mut diagnostics = Vec::new();
    let speech: Vec<(u64, u64)> = if !plan.settings.require_speech {
        vec![plan.range]
    } else if let Some(words) = &plan.words {
        // Nothing transcribed in it: all of it counts as speech.
        if words.is_empty() {
            vec![plan.range]
        } else {
            pick::speech_from_words(words)
        }
    } else if let Some(ctx) = &plan.mic {
        let silence = SilenceConfig {
            threshold_db: plan.settings.speech_threshold_db,
            min_duration_ms: plan.settings.pause_tolerance_ms,
            padding_ms: 0,
            ..SilenceConfig::default()
        };
        let scan = crate::project::silence::scan_track_silence(ctx, &silence)?;
        diagnostics.extend(scan.diagnostics.iter().cloned());
        pick::speech_ranges(&scan.covered, &scan.source_ranges)
    } else {
        vec![plan.range]
    };
    let telemetry = crate::telemetry::reader::read_telemetry(&plan.folder)?;
    if telemetry.events.is_empty() {
        diagnostics.push(if plan.settings.require_speech {
            "No mouse activity was recorded, so speech alone decides the layout".into()
        } else {
            "No mouse activity was recorded, so the whole recording counts as idle".into()
        });
    }
    let sections = pick::pick(
        &PickInput {
            start_us: plan.range.0,
            end_us: plan.range.1,
            media: Some(plan.media),
            telemetry: &telemetry,
            speech: &speech,
            move_us: plan.move_us,
        },
        &plan.settings,
    );
    Ok(LayoutsFound {
        sections,
        settings: plan.settings,
        diagnostics,
    })
}

/// Changes the layouts of the video (`short` none) or a short, as `change` says.
pub fn project_layouts_change_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short: Option<String>,
    change: LayoutChange,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.change_layouts(expected_revision, short.as_deref(), |timeline, track| {
            change.apply(timeline, track)
        })
    })
}

/// One change to a timeline's layouts. Times are on that timeline.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "op", rename_all_fields = "camelCase")]
pub enum LayoutChange {
    /// A section over `[startUs, endUs)` showing `layout` (none: the base). It takes that time
    /// from the sections it overlaps.
    Add {
        start_us: u64,
        end_us: u64,
        #[serde(default)]
        layout: Option<crate::layouts::Layout>,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        transition: Option<crate::layouts::Transition>,
        #[serde(default)]
        transition_ms: Option<u32>,
    },
    /// A section's layout, name and transition; a manual one from then on.
    Update {
        id: String,
        #[serde(default)]
        layout: Option<crate::layouts::Layout>,
        /// Show the base there instead of a layout.
        #[serde(default)]
        base: bool,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        transition: Option<crate::layouts::Transition>,
        #[serde(default)]
        transition_ms: Option<u32>,
    },
    /// A section moved to `[startUs, endUs)`, keeping its id and layout.
    Retime {
        id: String,
        start_us: u64,
        end_us: u64,
    },
    /// Sections removed by id.
    Remove { ids: Vec<String> },
    /// `[startUs, endUs)` taken out of every section there.
    Clear { start_us: u64, end_us: u64 },
    /// Every section, or only the picked ones.
    ClearAll {
        #[serde(default)]
        auto_only: bool,
    },
    /// The base layout.
    Base { layout: crate::layouts::Layout },
    /// How long a move takes, and how sections are picked.
    Settings {
        #[serde(default)]
        transition_ms: Option<u32>,
        #[serde(default)]
        pick: Option<crate::layouts::pick::PickSettings>,
    },
    /// Several changes in order, all or none, as one step.
    Many { changes: Vec<LayoutChange> },
}

impl LayoutChange {
    pub fn apply(
        self,
        timeline: &crate::project::revision::EditDocument,
        track: &mut crate::layouts::LayoutTrack,
    ) -> Result<(), String> {
        use crate::layouts::{sections_on, LayoutSection, Transition};
        let find = |track: &crate::layouts::LayoutTrack, id: &str| {
            track
                .sections
                .iter()
                .position(|s| s.id == id)
                .ok_or_else(|| format!("No layout section {id}"))
        };
        match self {
            LayoutChange::Add {
                start_us,
                end_us,
                layout,
                name,
                transition,
                transition_ms,
            } => {
                if let Some(layout) = &layout {
                    layout.validate()?;
                }
                let template = LayoutSection {
                    name,
                    transition: transition.unwrap_or(Transition::Auto),
                    transition_ms,
                    ..LayoutSection::new(track.new_id(), None, 0, 1, layout)
                };
                let made = sections_on(timeline, start_us, end_us, &template)?;
                track.add(made);
            }
            LayoutChange::Update {
                id,
                layout,
                base,
                name,
                transition,
                transition_ms,
            } => {
                let index = find(track, &id)?;
                let section = &mut track.sections[index];
                if base {
                    section.layout = None;
                    section.name = None;
                } else if let Some(layout) = layout {
                    layout.validate()?;
                    section.layout = Some(layout);
                    section.name = name;
                }
                if let Some(transition) = transition {
                    section.transition = transition;
                }
                if transition_ms.is_some() {
                    section.transition_ms = transition_ms;
                }
                section.auto = false;
                section.rule = None;
            }
            LayoutChange::Retime {
                id,
                start_us,
                end_us,
            } => {
                let index = find(track, &id)?;
                let old = track.sections.remove(index);
                let made = sections_on(
                    timeline,
                    start_us,
                    end_us,
                    &LayoutSection {
                        auto: false,
                        rule: None,
                        ..old
                    },
                )?;
                track.add(made);
            }
            LayoutChange::Remove { ids } => {
                track.sections.retain(|s| !ids.contains(&s.id));
            }
            LayoutChange::Clear { start_us, end_us } => {
                if end_us <= start_us {
                    return Err("Clear a range from a time to a later one".into());
                }
                crate::layouts::remove_on(timeline, track, start_us, end_us);
            }
            LayoutChange::ClearAll { auto_only } => {
                track.sections.retain(|s| auto_only && !s.auto);
            }
            LayoutChange::Base { layout } => {
                layout.validate()?;
                track.base = layout;
            }
            LayoutChange::Settings {
                transition_ms,
                pick,
            } => {
                if let Some(ms) = transition_ms {
                    track.transition_ms = ms;
                }
                if let Some(pick) = pick {
                    pick.validate()?;
                    track.pick = pick;
                }
            }
            LayoutChange::Many { changes } => {
                for change in changes {
                    change.apply(timeline, track)?;
                }
            }
        }
        Ok(())
    }
}

pub fn project_chapters_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    chapters: Vec<crate::chapters::Chapter>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.set_chapters(expected_revision, chapters)
    })
}

/// The check before export, of the edit as it is now.
pub fn project_check_impl(
    state: &AppState,
    project_handle: String,
) -> Result<crate::check::CheckReport, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let missing: Vec<&str> = reader
        .summary
        .assets
        .iter()
        .filter(|a| a.missing)
        .map(|a| a.id.as_str())
        .collect();
    Ok(crate::check::run(
        reader.root(),
        reader.document(),
        &missing,
    ))
}

/// Applies the fixes of the findings `ids` of the check at `expected_revision`, each as its
/// own undo step. Later moments go first, so a cut never moves a finding still to fix.
pub fn project_check_fix_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    ids: Vec<String>,
) -> Result<OpenedProject, String> {
    let report = project_check_impl(state, project_handle.clone())?;
    if report.revision != expected_revision {
        return Err("The edit changed since the check; check again".into());
    }
    let all = ids.iter().any(|id| id == "all");
    let mut fixes = Vec::new();
    for finding in report.findings {
        if !(all || ids.contains(&finding.id)) {
            continue;
        }
        match finding.fix {
            Some(fix) => fixes.push((finding.start_us, fix)),
            None if all => {}
            None => return Err(format!("{} has no fix to apply", finding.id)),
        }
    }
    if !all && fixes.len() < ids.len() {
        return Err("Some of those findings are not in the check any more; check again".into());
    }
    if fixes.is_empty() {
        return Err("Nothing to fix".into());
    }
    fixes.sort_by(|a, b| b.0.cmp(&a.0));
    let mut revision = expected_revision;
    let mut summary = None;
    for (_, fix) in fixes {
        let next = apply_check_fix(state, &project_handle, revision, fix)?;
        revision = next.revision;
        summary = Some(next);
    }
    summary.ok_or_else(|| "Nothing to fix".into())
}

fn apply_check_fix(
    state: &AppState,
    project_handle: &str,
    revision: u64,
    fix: crate::check::Fix,
) -> Result<OpenedProject, String> {
    use crate::check::Fix;
    let handle = project_handle.to_string();
    let current = || -> Result<crate::project::revision::EditDocument, String> {
        let opened = state.opened_project.lock();
        Ok(opened
            .as_ref()
            .ok_or("No opened project")?
            .document()
            .clone())
    };
    match fix {
        Fix::Cut { start_us, end_us } => project_sequence_edit_impl(
            state,
            handle,
            revision,
            SequenceEdit::DeleteRange {
                ranges: vec![crate::zoom::EditedRange { start_us, end_us }],
                ripple: Some(true),
            },
            None,
        ),
        Fix::DeleteZoom { id } => project_zoom_delete_impl(state, handle, revision, id),
        Fix::CaptionOffset { offset_pct } => {
            let mut captions = current()?.captions;
            captions.offset_pct = offset_pct;
            project_captions_update_impl(state, handle, revision, captions)
        }
        Fix::RemoveChapter { id } => {
            let chapters = current()?
                .chapters
                .into_iter()
                .filter(|c| c.id != id)
                .collect();
            project_chapters_set_impl(state, handle, revision, chapters)
        }
        Fix::Normalize { target_lufs } => {
            let mut audio = current()?.audio;
            audio.normalize = true;
            audio.target_lufs = target_lufs;
            project_audio_update_impl(state, handle, revision, audio)
        }
    }
}

/// Whether some speech has a transcript to caption from.
pub(crate) fn has_speech_transcript(reader: &crate::project::ProjectReader) -> bool {
    let has = |id: &str| {
        crate::transcript::store::load_transcript(reader.root(), id)
            .ok()
            .flatten()
            .is_some()
    };
    let document = reader.document();
    document.captions.track_id.as_deref().is_some_and(has)
        || crate::export::caption_candidates(document)
            .iter()
            .any(|key| has(key))
}

/// One preview frame of a short, `offset_us` into it, drawn with `layout` (which may not be
/// saved yet), as a JPEG.
pub fn short_preview_frame_impl(
    state: &AppState,
    project_handle: String,
    short_id: String,
    layout: crate::shorts::ShortLayout,
    offset_us: u64,
) -> Result<Vec<u8>, String> {
    layout.validate()?;
    let (document, root, key) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        let base = &reader.history().current;
        let mut short = base
            .shorts
            .iter()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?
            .clone();
        short.layout = layout;
        let document = crate::shorts::short_document(base, &short, has_speech_transcript(reader))?;
        // Transcripts are saved apart from the edit: a word fixed changes the captions too.
        let key = format!(
            "{project_handle}:{}:{}:{short_id}:{}",
            base.revision,
            crate::transcript::store::changes(),
            serde_json::to_string(&short.layout).map_err(|e| e.to_string())?
        );
        (document, reader.root().to_path_buf(), key)
    };
    let at = offset_us.min(document.duration_us().saturating_sub(1));
    let mut cache = state.short_preview.lock();
    if cache.as_ref().map(|c| c.key.as_str()) != Some(key.as_str()) {
        let reuse = cache.take().and_then(|c| c.evaluator.into_reuse());
        let (width, height) = SHORT_PREVIEW_SIZE;
        let evaluator =
            crate::export::SceneEvaluator::new_reusing(root, document, width, height, reuse)?
                .with_decode_limit(crate::media::ffmpeg::DecodeLimit {
                    max_width: width * 2,
                    max_height: height * 2,
                    max_rate: 0,
                    interactive: true,
                    yuv: false,
                });
        *cache = Some(ShortPreviewCache { key, evaluator });
    }
    let frame = cache
        .as_mut()
        .ok_or("The preview renderer is not ready")?
        .evaluator
        .preview_at(at)?;
    crate::playback::preview::encode_webview_frame(&frame)
}

pub fn project_shorts_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    shorts: Vec<crate::shorts::Short>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.set_shorts(expected_revision, shorts)
    })
}

/// A short made from a take, and how its camera was lined up with its voice.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeShortMade {
    pub project: OpenedProject,
    pub short_id: String,
    pub sync: crate::take_short::TakeSync,
}

/// A new short made from a take: `camera` and `voice` name streams (`<asset>.<stream>`) or
/// assets already imported. A separate voice is lined up with the camera's own sound.
#[allow(clippy::too_many_arguments)]
pub fn project_short_from_take_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    title: String,
    camera: Option<String>,
    voice: Option<String>,
    media: Vec<String>,
    reason: Option<String>,
) -> Result<TakeShortMade, String> {
    // Lining the sounds up reads them: done without holding the project.
    let (root, document) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        (reader.root().to_path_buf(), reader.document().clone())
    };
    // Named streams win; otherwise the take's files are told apart.
    let sources = if camera.is_none() && voice.is_none() && !media.is_empty() {
        crate::take_short::guess(&document, &media)?
    } else {
        crate::take_short::resolve(&document, camera.as_deref(), voice.as_deref())?
    };
    let sync = crate::take_short::sync(&root, &document, &sources);
    let base = format!("take-{}", sources.voice.asset);
    let id = (1..)
        .map(|n| {
            if n == 1 {
                base.clone()
            } else {
                format!("{base}-{n}")
            }
        })
        .find(|id| !document.shorts.iter().any(|s| &s.id == id))
        .unwrap_or(base);
    let title = if title.trim().is_empty() {
        format!("Short {}", document.shorts.len() + 1)
    } else {
        title
    };
    let short = crate::take_short::new_short(
        &document,
        id.clone(),
        &title,
        reason.as_deref().unwrap_or_default(),
        &sources,
        &sync,
    )?;
    let roles = crate::take_short::roles(&document, &sources);
    let project = mutate_opened(state, project_handle, |reader| {
        reader.add_take_short(expected_revision, short, &roles)
    })?;
    Ok(TakeShortMade {
        project,
        short_id: id,
        sync,
    })
}

/// A change to the footage under a take short.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FootageChange {
    /// Plays each stretch under the take, in turn, over any footage there.
    Add {
        items: Vec<crate::take_short::Footage>,
    },
    /// Takes footage off `[fromUs, toUs)`; without them, all of it.
    Remove {
        #[serde(default)]
        from_us: Option<u64>,
        #[serde(default)]
        to_us: Option<u64>,
    },
}

pub fn project_short_footage_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short_id: String,
    change: FootageChange,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        let document = reader.document().clone();
        let mut shorts = document.shorts.clone();
        let short = shorts
            .iter_mut()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?;
        match &change {
            FootageChange::Add { items } => {
                if items.is_empty() {
                    return Err("Name some footage to add".into());
                }
                for item in items {
                    crate::take_short::place(&document, short, item)?;
                }
            }
            FootageChange::Remove { from_us, to_us } => {
                let range = match (from_us, to_us) {
                    (Some(a), Some(b)) if b > a => Some((*a, *b)),
                    (None, None) => None,
                    _ => return Err("Give both `fromUs` and `toUs`, or neither".into()),
                };
                crate::take_short::remove(&document, short, range)?;
            }
        }
        reader.set_shorts(expected_revision, shorts)
    })
}

/// Exports one short as a 9:16 video next to the project (or to `settings.destination`),
/// named after the project and the short. Captions are turned on when a speech track has a
/// transcript.
pub fn project_short_export_impl(
    state: &AppState,
    project_handle: String,
    short_id: String,
    settings: crate::export::ExportSettings,
) -> Result<crate::export::ExportStatus, String> {
    let _guard = state.command_lock.lock();
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let base = &reader.history().current;
    let short = base
        .shorts
        .iter()
        .find(|s| s.id == short_id)
        .ok_or("That short no longer exists")?;
    let document = crate::shorts::short_document(base, short, has_speech_transcript(reader))?;
    let root = reader.root().to_path_buf();
    let stem = format!("{} - {}", reader.summary.name, short.title);
    drop(opened);
    // The first free "<project> - <short>.mp4", then " (2)", " (3)" and so on.
    let name = (1..1000)
        .map(|n| {
            if n == 1 {
                stem.clone()
            } else {
                format!("{stem} ({n})")
            }
        })
        .find(|name| {
            settings.destination.is_some()
                || !crate::export::default_destination(&root, name, 0).exists()
        })
        .ok_or("Too many exports of this short already exist")?;
    let gate = Arc::clone(&state.encoder_gate);
    let mut owner = state.export.lock();
    match crate::export::prepare_job(&root, &name, document, settings, &mut owner) {
        Ok(captured) => Ok(crate::export::spawn_job(captured, &mut owner, gate)),
        Err(status) => {
            owner.install_failed(status.clone());
            Ok(status)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditCut {
    pub start_us: u64,
    pub end_us: u64,
}

pub(crate) fn require_handle(reader: &ProjectReader, project_handle: &str) -> Result<(), String> {
    if reader.summary.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    Ok(())
}

/// The sound effects lane's sounds on the timeline, for drawing it (a short's own when
/// `short_id` is set).
pub fn project_sound_effects_impl(
    state: &AppState,
    project_handle: String,
    short_id: Option<String>,
) -> Result<Vec<crate::sound_effects::Effect>, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let base = &reader.history().current;
    let document = match short_id.as_deref() {
        Some(id) => {
            let short = base
                .shorts
                .iter()
                .find(|s| s.id == id)
                .ok_or("That short no longer exists")?;
            crate::shorts::short_document(base, short, false)
                .or_else(|_| crate::shorts::short_timeline(base, short))?
        }
        None => base.clone(),
    };
    Ok(crate::sound_effects::plan(&document))
}

pub fn project_ripple_cuts_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    cuts: Vec<EditCut>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let ranges: Vec<(u64, u64)> = cuts
        .into_iter()
        .map(|cut| (cut.start_us, cut.end_us))
        .collect();
    let summary = reader.ripple_cuts(expected_revision, &ranges)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

/// Imports files into the project's media bin. Copying and probing run without holding
/// the project, so playback keeps going during a long import.
pub fn project_media_import_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    paths: Vec<String>,
) -> Result<OpenedProject, String> {
    if paths.is_empty() || paths.len() > 64 {
        return Err("Choose between 1 and 64 files to import".into());
    }
    let root = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        reader.root().to_path_buf()
    };
    let chosen: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
    let paths = crate::media_bin::expand_import_paths(&chosen)?;
    if paths.len() > 256 {
        return Err("Choose at most 256 files to import at once".into());
    }
    let mut assets = Vec::with_capacity(paths.len());
    for path in &paths {
        match crate::media_bin::import(&root, path) {
            Ok(asset) => assets.push(asset),
            Err(error) => {
                for asset in &assets {
                    crate::media_bin::remove_files(&root, asset);
                }
                return Err(error);
            }
        }
    }
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let Some(reader) = opened
        .as_mut()
        .filter(|r| r.summary.project_handle == project_handle)
    else {
        for asset in &assets {
            crate::media_bin::remove_files(&root, asset);
        }
        return Err("The project was closed during the import".into());
    };
    reader.add_assets(expected_revision, assets)
}

pub fn project_media_remove_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    asset_id: String,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let summary = reader.remove_asset(expected_revision, &asset_id)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    Ok(summary)
}

/// What one of an asset's streams stands for.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StreamRoleInput {
    pub stream: String,
    pub role: Role,
}

pub fn project_media_roles_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    asset_id: String,
    roles: Vec<StreamRoleInput>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let roles: Vec<(String, Role)> = roles.into_iter().map(|r| (r.stream, r.role)).collect();
    let summary = reader.set_stream_roles(expected_revision, &asset_id, &roles)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

/// One timeline edit in the project's timeline, or in short `short_id`'s own timeline.
pub fn project_sequence_edit_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    edit: SequenceEdit,
    short_id: Option<String>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    edit_sequence_locked(state, reader, expected_revision, &edit, short_id.as_deref())
}

/// The caller holds command_lock and opened_project, including any dependency validation.
fn edit_sequence_locked(
    state: &AppState,
    reader: &mut ProjectReader,
    expected_revision: u64,
    edit: &SequenceEdit,
    short_id: Option<&str>,
) -> Result<OpenedProject, String> {
    let (summary, _) = reader.edit_sequence(expected_revision, edit, short_id)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

pub fn project_undo_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short_id: Option<String>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    reader.undo(expected_revision)?;
    let summary = reader.view_or_summary(short_id.as_deref());
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

pub fn project_redo_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short_id: Option<String>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    reader.redo(expected_revision)?;
    let summary = reader.view_or_summary(short_id.as_deref());
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

pub fn project_short_view_impl(
    state: &AppState,
    project_handle: String,
    short_id: String,
) -> Result<OpenedProject, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let started = std::time::Instant::now();
    let view = reader.short_view(&short_id)?;
    if crate::media::profiling() {
        let bytes = serde_json::to_vec(&view).map(|v| v.len()).unwrap_or(0);
        eprintln!(
            "[profile] short view: {:.1} ms, {bytes} bytes",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    Ok(view)
}

pub fn project_short_resync_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short_id: String,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let view = reader.resync_short(expected_revision, &short_id)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(view)
}

/// Plays short `short_id` (its own frame, timeline and sound) instead of the video, or the
/// video again with `None`.
pub fn playback_focus_short_impl(
    state: &AppState,
    project_handle: String,
    short_id: Option<String>,
    start_us: u64,
    play: bool,
) -> Result<crate::playback::PlaybackStatus, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let started = std::time::Instant::now();
    if short_id.is_some() {
        crate::media::switch_started();
    }
    let mut playback = state.playback.lock();
    playback.focus_short(short_id, &reader.history().current, start_us)?;
    crate::media::profile("focus short", started);
    if play {
        playback.play()
    } else {
        playback.pause()
    }
}

pub fn project_rename_impl(
    state: &AppState,
    project_handle: String,
    new_name: String,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    reader.rename_project(&new_name)
}

pub fn playback_status_impl(
    state: &AppState,
    project_handle: String,
) -> Result<PlaybackStatus, String> {
    let mut playback = state.playback.lock();
    let status = playback.status()?;
    if status.state == crate::playback::PlaybackState::Closed {
        return Err("Playback is closed".into());
    }
    if status.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    Ok(status)
}

pub fn playback_play_impl(
    state: &AppState,
    project_handle: String,
) -> Result<PlaybackStatus, String> {
    let mut playback = state.playback.lock();
    if playback.status()?.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    playback.play()
}

/// Plays at `speed` (1, 1.25, 1.5 or 2), carrying on if it is playing.
pub fn playback_set_speed_impl(
    state: &AppState,
    project_handle: String,
    speed: f64,
) -> Result<PlaybackStatus, String> {
    let mut playback = state.playback.lock();
    if playback.status()?.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    playback.set_speed(speed)
}

pub fn playback_pause_impl(
    state: &AppState,
    project_handle: String,
) -> Result<PlaybackStatus, String> {
    let mut playback = state.playback.lock();
    if playback.status()?.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    playback.pause()
}

pub fn playback_seek_impl(
    state: &AppState,
    project_handle: String,
    edited_us: u64,
) -> Result<PlaybackStatus, String> {
    let mut playback = state.playback.lock();
    if playback.status()?.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    playback.seek(edited_us)
}

pub fn preview_attach_impl(
    state: &AppState,
    window_label: String,
    hit_mode: PreviewHitMode,
    target: Option<crate::playback::preview::PreviewTarget>,
) -> Result<PreviewStatus, String> {
    // No target means the webview preview, where frames are fetched by the page.
    state.preview.lock().attach(window_label, hit_mode, target)
}

pub fn preview_layout_impl(
    state: &AppState,
    viewport: PreviewViewport,
) -> Result<PreviewStatus, String> {
    if viewport.generation == 0 {
        return Err("Preview generation is required".into());
    }
    state.preview.lock().layout(viewport)
}

pub fn preview_present_fixed_impl(
    state: &AppState,
    r: f32,
    g: f32,
    b: f32,
    generation: u64,
) -> Result<PreviewStatus, String> {
    state.preview.lock().present_fixed(r, g, b, generation)
}

pub fn preview_present_fixture_impl(
    state: &AppState,
    path: String,
    generation: u64,
) -> Result<PreviewStatus, String> {
    state.preview.lock().present_fixture(&path, generation)
}

pub fn preview_status_impl(state: &AppState) -> PreviewStatus {
    state.preview.lock().status()
}

/// The preview quality in use: the one picked, or the default for the current surface.
pub fn preview_quality_impl(state: &AppState) -> playback::PreviewQuality {
    let status = state.preview.lock().status();
    let webview = if status.attached {
        status.surface == "webview"
    } else {
        !cfg!(target_os = "macos")
    };
    state
        .preview_quality
        .lock()
        .unwrap_or_else(|| playback::PreviewQuality::default_for(webview))
}

/// Takes effect on the next preview frame: the playback worker rebuilds its renderer.
pub fn preview_quality_set_impl(
    state: &AppState,
    quality: playback::PreviewQuality,
) -> Result<playback::PreviewQuality, String> {
    quality.validate()?;
    *state.preview_quality.lock() = Some(quality);
    Ok(quality)
}

pub fn preview_hit_test_impl(state: &AppState, x: f64, y: f64) -> bool {
    state.preview.lock().hit_test(x, y)
}

pub fn preview_detach_impl(
    state: &AppState,
    window_label: String,
) -> Result<PreviewStatus, String> {
    let mut preview = state.preview.lock();
    if preview.status().attached
        && preview.status().window_label.as_deref() != Some(window_label.as_str())
    {
        return Err("Stale preview window label".into());
    }
    preview.detach();
    Ok(preview.status())
}

pub fn media_interop_status_impl(state: &AppState) -> MediaInteropStatus {
    let _ = state;
    crate::media::interop_status(
        None,
        vec![
            "FFmpeg is not pinned; VideoToolbox implements the decoder/encoder contract".into(),
            "WGPU uses a CPU upload/readback fallback; Metal texture interop is untested".into(),
        ],
    )
}

pub fn media_run_parity_impl(state: &AppState) -> Result<MediaParityReport, String> {
    let dir = std::env::temp_dir().join(format!("aeroedits-f2-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let report = crate::render::run_parity(&dir, &state.encoder_gate);
    let _ = std::fs::remove_dir_all(&dir);
    report
}

pub fn export_start_impl(
    state: &AppState,
    project_handle: String,
    settings: crate::export::ExportSettings,
) -> Result<crate::export::ExportStatus, String> {
    let _guard = state.command_lock.lock();
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let document = reader.history().current.clone();
    let root = reader.root().to_path_buf();
    let name = reader.summary.name.clone();
    drop(opened);
    let gate = Arc::clone(&state.encoder_gate);
    let mut owner = state.export.lock();
    match crate::export::prepare_job(&root, &name, document, settings, &mut owner) {
        Ok(captured) => Ok(crate::export::spawn_job(captured, &mut owner, gate)),
        Err(status) => {
            owner.install_failed(status.clone());
            Ok(status)
        }
    }
}

pub fn export_status_impl(
    state: &AppState,
    job_id: Option<String>,
) -> Result<crate::export::ExportStatus, String> {
    let mut owner = state.export.lock();
    let status = owner.status();
    if let Some(id) = job_id {
        if !status.job_id.is_empty() && status.job_id != id {
            return Err("Stale export job id".into());
        }
    }
    Ok(status)
}

pub fn export_cancel_impl(
    state: &AppState,
    job_id: String,
) -> Result<crate::export::ExportStatus, String> {
    state.export.lock().cancel(&job_id)
}

/// Saves one of the edit's other files (subtitles, transcript or the sound alone) for the video
/// or the short `short_id`: at `destination`, else next to the project named after it (and the
/// short), replacing an earlier one. Returns where it went.
pub fn export_file_impl(
    state: &AppState,
    project_handle: String,
    short_id: Option<String>,
    kind: crate::export::files::FileKind,
    destination: Option<String>,
    on_progress: &mut dyn FnMut(f64),
) -> Result<PathBuf, String> {
    use crate::export::files;
    let (root, stem, assets, contents, sound) = {
        let _guard = state.command_lock.lock();
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        let base = &reader.history().current;
        let short = match &short_id {
            Some(id) => Some(
                base.shorts
                    .iter()
                    .find(|s| &s.id == id)
                    .ok_or("That short no longer exists")?,
            ),
            None => None,
        };
        let (stem, title) = match short {
            Some(short) => (
                format!("{} - {}", reader.summary.name, short.title),
                short.title.clone(),
            ),
            None => (reader.summary.name.clone(), reader.summary.name.clone()),
        };
        let (contents, sound) = if kind == files::FileKind::Wav {
            // The document the video export plays, so the sound matches it.
            let document = match short {
                Some(short) => {
                    crate::shorts::short_document(base, short, has_speech_transcript(reader))?
                }
                None => base.clone(),
            };
            (None, Some(document))
        } else {
            let (timeline, cues) =
                transcript::caption_cues(reader, short_id.as_deref(), !kind.is_subtitles())?;
            let (_, cues) =
                cues.ok_or("Nothing is transcribed yet: transcribe the speech first")?;
            if cues.is_empty() {
                return Err("No words are left in the edit to write".into());
            }
            let chapters = crate::chapters::timeline(&timeline.chapters, &timeline);
            let text = match kind {
                files::FileKind::Srt => files::srt(&cues),
                files::FileKind::Vtt => files::vtt(&cues),
                files::FileKind::Txt => files::plain(&cues, &chapters),
                _ => files::markdown(&title, &cues, &chapters),
            };
            (Some(text), None)
        };
        (
            reader.root().to_path_buf(),
            stem,
            base.assets.clone(),
            contents,
            sound,
        )
    };
    let requested = match destination {
        Some(path) if !path.trim().is_empty() => path,
        _ => crate::export::default_destination(&root, &stem, 0)
            .with_extension(kind.extension())
            .to_string_lossy()
            .into_owned(),
    };
    let path = crate::export::resolve_destination(&root, &stem, 0, Some(&requested), &assets)
        .map_err(|failure| failure.message().to_string())?;
    if let Some(text) = contents {
        fs::write(&path, text).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    } else if let Some(document) = sound {
        let mixer = crate::media::audio::AudioMixer::new(&root, &document)?;
        if !mixer.has_audio() {
            return Err("The edit has no sound".into());
        }
        crate::export::files::write_wav(&mixer, &path, on_progress)?;
    }
    Ok(path)
}

/// Formats the window title for project editing.
/// When a project is open, produces "AeroEdits — <Project Name>" (e.g. "AeroEdits — Launch Demo").
/// When no project is open (None or empty), produces "AeroEdits".
pub fn window_title_for_project(project_name: Option<&str>) -> String {
    match project_name.map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => format!("AeroEdits \u{2014} {}", name),
        None => "AeroEdits".to_string(),
    }
}
