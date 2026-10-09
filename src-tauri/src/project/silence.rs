//! Project-scoped silence analysis over E2 PCM segments.
use super::manifest::TrackType;
use super::pcm::{PcmReader, READ_FRAME_CHUNK};
use super::reader::safe_path;
use super::waveform::WaveformTrackContext;
use crate::dsp::silence::{
    channel_policy_name, finalize_regions, resolve_threshold, silent_runs, ChannelPolicy,
    LevelBlock, SilenceConfig, SilenceCutInterval, SilenceDetectionResult, SoundThreshold,
    StreamingSilenceDetector,
};
use crate::timeline::{SourceInterval, TimelineMapper};
use std::fs;

const MAX_DIAGNOSTICS: usize = 32;

/// Fixed protection for the secondary source: even quiet, brief sounds count. It is
/// independent of silence presets and has no minimum sound duration. Only measured,
/// near-zero PCM is cuttable; unavailable audio remains protected by the coverage scan.
/// Apply sound margins after mapping onto timeline clips, so they survive trimmed edges.
pub(crate) fn secondary_audio_guard_config() -> SilenceConfig {
    SilenceConfig {
        threshold_db: -90.0,
        min_duration_ms: 1,
        padding_ms: 0,
        window_ms: Some(10),
        step_ms: Some(5),
        channel_policy: Some("max_peak".into()),
        ..SilenceConfig::default()
    }
}

/// Silence found on one audio track, in source time, before it is mapped onto the edit.
pub(crate) struct SilenceScan {
    /// Silent ranges after the minimum-duration filter and padding.
    pub source_ranges: Vec<(u64, u64)>,
    /// Source ranges whose audio was actually read. Anything outside is a gap, not silence.
    pub covered: Vec<(u64, u64)>,
    pub sample_rate: u32,
    pub channels: u16,
    pub policy: ChannelPolicy,
    pub diagnostics: Vec<String>,
    pub threshold_db: f32,
}

pub(crate) fn scan_track_silence(
    ctx: &WaveformTrackContext,
    config: &SilenceConfig,
) -> Result<SilenceScan, String> {
    config.validate()?;
    if !matches!(ctx.track_type, TrackType::MicAudio | TrackType::SystemAudio) {
        return Err("Track is not audio".into());
    }
    let policy = config.resolved_policy()?;
    let mut diagnostics = Vec::new();
    let mut blocks: Vec<LevelBlock> = Vec::new();
    let mut detector: Option<StreamingSilenceDetector> = None;
    let mut sample_rate = 0u32;
    let mut channels = 0u16;

    if ctx.segments.is_empty() {
        push_diagnostic(
            &mut diagnostics,
            "Audio track is empty; no PCM segments to analyze",
        );
        return Ok(SilenceScan {
            source_ranges: Vec::new(),
            covered: Vec::new(),
            sample_rate,
            channels,
            policy,
            diagnostics,
            threshold_db: config.threshold_db,
        });
    }
    let mut covered = Vec::new();

    for segment in &ctx.segments {
        let path = match safe_path(&ctx.root, &segment.relative_path) {
            Ok(path) => path,
            Err(error) => {
                reset_detector(&mut detector, &mut blocks);
                push_diagnostic(
                    &mut diagnostics,
                    &format!(
                        "Unsafe audio path treated as a gap, not silence: {} ({error})",
                        segment.relative_path
                    ),
                );
                continue;
            }
        };
        if !segment.available || !path.is_file() {
            reset_detector(&mut detector, &mut blocks);
            push_diagnostic(
                &mut diagnostics,
                &format!(
                    "Missing file treated as a gap, not silence: {}",
                    segment.relative_path
                ),
            );
            continue;
        }
        let file_len = match fs::metadata(&path) {
            Ok(meta) => meta.len(),
            Err(_) => {
                reset_detector(&mut detector, &mut blocks);
                push_diagnostic(
                    &mut diagnostics,
                    &format!(
                        "Missing file treated as a gap, not silence: {}",
                        segment.relative_path
                    ),
                );
                continue;
            }
        };
        if file_len != segment.size_bytes {
            reset_detector(&mut detector, &mut blocks);
            push_diagnostic(
                &mut diagnostics,
                &format!(
                    "Size-mismatched audio treated as a gap, not silence: {}",
                    segment.relative_path
                ),
            );
            continue;
        }

        let mut reader = match PcmReader::open(&path) {
            Ok(reader) => reader,
            Err(error) => {
                reset_detector(&mut detector, &mut blocks);
                push_diagnostic(
                    &mut diagnostics,
                    &format!(
                        "Unsupported encoding treated as a gap, not silence: {} ({error})",
                        segment.relative_path
                    ),
                );
                continue;
            }
        };
        let info = reader.info().clone();
        if let Some(active) = detector.as_mut() {
            if active.sample_rate() != info.sample_rate || active.channels() != info.channels {
                blocks.extend(active.take_blocks());
                detector = None;
            }
        }
        if detector.is_none() {
            match StreamingSilenceDetector::new(info.sample_rate, info.channels, config) {
                Ok(created) => {
                    sample_rate = info.sample_rate;
                    channels = info.channels;
                    if let ChannelPolicy::Channel(index) = policy {
                        if index as usize >= info.channels as usize {
                            return Err("Selected silence channel is out of range".into());
                        }
                    }
                    detector = Some(created);
                }
                Err(error) => {
                    push_diagnostic(&mut diagnostics, &error);
                    continue;
                }
            }
        }

        let ch = info.channels as usize;
        let mut interleaved = vec![0.0f32; READ_FRAME_CHUNK * ch];
        let mut frame_index = 0u64;
        let mut feed_failed = None;
        let mut read_failed = false;
        loop {
            let frames = match reader.read_frames(&mut interleaved, READ_FRAME_CHUNK) {
                Ok(0) => break,
                Ok(frames) => frames,
                Err(error) => {
                    push_diagnostic(
                        &mut diagnostics,
                        &format!(
                            "Unreadable audio treated as a gap, not silence: {} ({error})",
                            segment.relative_path
                        ),
                    );
                    reset_detector(&mut detector, &mut blocks);
                    read_failed = true;
                    break;
                }
            };
            let local_us = info.frame_us(frame_index);
            let source_us = segment.start_us.saturating_add(local_us);
            if source_us >= segment.end_us {
                break;
            }
            let mut use_frames = frames;
            while use_frames > 0 {
                let end_us = segment
                    .start_us
                    .saturating_add(info.frame_us(frame_index + use_frames as u64));
                if end_us <= segment.end_us {
                    break;
                }
                use_frames -= 1;
            }
            if use_frames == 0 {
                break;
            }
            let samples = &interleaved[..use_frames * ch];
            if let Some(active) = detector.as_mut() {
                if let Err(error) = active.feed(samples, source_us) {
                    feed_failed = Some(error);
                    break;
                }
            }
            frame_index += use_frames as u64;
        }
        if let Some(error) = feed_failed {
            return Err(error);
        }
        if !read_failed && detector.is_some() {
            let read_end = segment.start_us.saturating_add(info.frame_us(frame_index));
            if read_end > segment.start_us {
                covered.push((segment.start_us, read_end.min(segment.end_us)));
            }
        }
    }

    if let Some(mut active) = detector.take() {
        blocks.extend(active.take_blocks());
    }

    let threshold_db = resolve_threshold(&blocks, config);
    Ok(SilenceScan {
        source_ranges: finalize_regions(silent_runs(&blocks, threshold_db), config),
        covered,
        sample_rate,
        channels,
        policy,
        diagnostics,
        threshold_db,
    })
}

pub fn detect_track_silence(
    ctx: &WaveformTrackContext,
    config: &SilenceConfig,
) -> Result<SilenceDetectionResult, String> {
    detect_track_silence_with_words(ctx, config, None)
}

pub(crate) fn detect_track_silence_with_words(
    ctx: &WaveformTrackContext,
    config: &SilenceConfig,
    transcript: Option<&crate::transcript::Transcript>,
) -> Result<SilenceDetectionResult, String> {
    let SilenceScan {
        source_ranges,
        sample_rate,
        channels,
        policy,
        diagnostics,
        threshold_db,
        ..
    } = scan_track_silence(ctx, config)?;
    let mapper = TimelineMapper::try_new(
        ctx.retained
            .iter()
            .enumerate()
            .map(|(i, interval)| {
                SourceInterval::new(format!("ret-{i}"), interval.start_us, interval.end_us)
                    .with_media(interval.media.clone())
            })
            .collect(),
    )?;

    let source_ranges = if let Some(transcript) = transcript {
        crate::transcript::pauses::protect_words(source_ranges, transcript, config.padding_ms)
    } else {
        source_ranges
    };
    let min_us =
        u64::from(config.min_duration_ms).saturating_sub(2 * u64::from(config.padding_ms)) * 1_000;
    let mut suggestions = Vec::new();
    for (source_start, source_end) in source_ranges {
        for (edited_start, edited_end) in mapper.source_range_to_edited(source_start, source_end) {
            if edited_end.saturating_sub(edited_start) < min_us.max(1) {
                continue;
            }
            suggestions.push(SilenceCutInterval {
                id: format!("silence-{}", suggestions.len() + 1),
                start_us: edited_start,
                end_us: edited_end,
                duration_ms: edited_end.saturating_sub(edited_start) / 1_000,
                selected: true,
                source_start_us: source_start,
                source_end_us: source_end,
            });
        }
    }
    // Reordered clips map later source time earlier: list suggestions in timeline order.
    suggestions.sort_by_key(|s| s.start_us);
    for (index, suggestion) in suggestions.iter_mut().enumerate() {
        suggestion.id = format!("silence-{}", index + 1);
    }

    Ok(SilenceDetectionResult {
        track_id: ctx.track_id.clone(),
        sample_rate,
        channels,
        channel_policy: channel_policy_name(policy),
        suggestions,
        diagnostics,
        thresholds: vec![SoundThreshold {
            track_id: ctx.track_id.clone(),
            threshold_db,
        }],
        transcript_dependencies: transcript.map(|t| vec![t.dependency()]).unwrap_or_default(),
    })
}

/// Where a sound plays on the timeline, and its pauses there.
pub(crate) type PlaysAndPauses = (Vec<(u64, u64)>, Vec<(u64, u64)>);

/// The pauses in several speech sounds at once, in timeline time: where some speech plays and
/// every speech playing there is silent. Each sound comes as where its clips play on the
/// timeline and its own pauses there (from [`detect_track_silence`]). Pieces shorter than
/// `min_us` (left where one sound's pause ends inside another's) are dropped.
pub(crate) fn common_pauses(sounds: &[PlaysAndPauses], min_us: u64) -> Vec<(u64, u64)> {
    let merged = |mut ranges: Vec<(u64, u64)>| {
        ranges.retain(|(a, b)| b > a);
        ranges.sort_unstable();
        let mut out: Vec<(u64, u64)> = Vec::new();
        for (a, b) in ranges {
            match out.last_mut() {
                Some(last) if a <= last.1 => last.1 = last.1.max(b),
                _ => out.push((a, b)),
            }
        }
        out
    };
    let sounds: Vec<PlaysAndPauses> = sounds
        .iter()
        .map(|(plays, silent)| (merged(plays.clone()), merged(silent.clone())))
        .collect();
    // Sweep play/pause boundaries once. A source with many clips (or repeated clips) must
    // not require checking every clip at every boundary.
    let mut edges: Vec<(u64, i64, i64)> = Vec::new();
    for (plays, silent) in &sounds {
        for &(a, b) in plays {
            edges.extend([(a, 1, 0), (b, -1, 0)]);
            let first = silent.partition_point(|&(_, end)| end <= a);
            for &(start, end) in &silent[first..] {
                if start >= b {
                    break;
                }
                edges.extend([(start.max(a), 0, 1), (end.min(b), 0, -1)]);
            }
        }
    }
    edges.sort_unstable_by_key(|edge| edge.0);
    let mut pauses: Vec<(u64, u64)> = Vec::new();
    let (mut playing, mut quiet) = (0i64, 0i64);
    let mut previous = 0;
    for (at, play_delta, quiet_delta) in edges {
        let (a, b) = (previous, at);
        if b > a && playing > 0 && playing == quiet {
            match pauses.last_mut() {
                Some(last) if last.1 == a => last.1 = b,
                _ => pauses.push((a, b)),
            }
        }
        playing += play_delta;
        quiet += quiet_delta;
        previous = at;
    }
    pauses.retain(|(a, b)| b - a >= min_us.max(1));
    pauses
}

fn reset_detector(detector: &mut Option<StreamingSilenceDetector>, blocks: &mut Vec<LevelBlock>) {
    if let Some(active) = detector.as_mut() {
        active.notify_discontinuity();
        blocks.extend(active.take_blocks());
    }
}

fn push_diagnostic(diagnostics: &mut Vec<String>, message: &str) {
    if diagnostics.len() < MAX_DIAGNOSTICS {
        diagnostics.push(message.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::reader::{RetainedInterval, SegmentSummary};

    #[test]
    fn common_pauses_need_every_speech_playing_to_be_silent() {
        const S: u64 = 1_000_000;
        // A recording's speech plays 0..10s, a video's 10..20s and again 15..20s on another
        // track under it, so 15..20s has both.
        let recording = (vec![(0, 10 * S)], vec![(2 * S, 3 * S), (9 * S, 10 * S)]);
        let video = (
            vec![(10 * S, 20 * S)],
            vec![(10 * S, 11 * S), (12 * S, 13 * S), (16 * S, 18 * S)],
        );
        let under = (vec![(15 * S, 20 * S)], vec![(17 * S, 19 * S)]);
        assert_eq!(
            common_pauses(&[recording.clone(), video.clone()], S / 10),
            // The recording's pause running into the video's is one pause.
            vec![
                (2 * S, 3 * S),
                (9 * S, 11 * S),
                (12 * S, 13 * S),
                (16 * S, 18 * S)
            ]
        );
        assert_eq!(
            common_pauses(&[recording, video, under], S / 10),
            vec![
                (2 * S, 3 * S),
                (9 * S, 11 * S),
                (12 * S, 13 * S),
                (17 * S, 18 * S)
            ]
        );
        // Silence where no speech plays is no pause; slivers go.
        let edge = (vec![(0, 4 * S)], vec![(3 * S, 6 * S)]);
        let other = (vec![(0, 4 * S)], vec![(3 * S + S / 20, 4 * S)]);
        assert_eq!(common_pauses(&[edge.clone()], S / 10), vec![(3 * S, 4 * S)]);
        assert_eq!(common_pauses(&[edge, other], S), Vec::<(u64, u64)>::new());
    }

    #[test]
    fn scan_reports_covered_audio_so_speech_can_be_derived() {
        let dir = tempfile::tempdir().unwrap();
        let rate = 16_000u32;
        // 2s tone, 1s silence, 2s tone.
        let samples: Vec<i16> = (0..rate * 5)
            .map(|i| {
                let t = i as f32 / rate as f32;
                if (2.0..3.0).contains(&t) {
                    0
                } else {
                    ((t * 440.0 * std::f32::consts::TAU).sin() * 12_000.0) as i16
                }
            })
            .collect();
        let wav = crate::fixtures::generate_pcm16_wav(rate, 1, &samples);
        std::fs::create_dir_all(dir.path().join("audio")).unwrap();
        std::fs::write(dir.path().join("audio/mic.wav"), &wav).unwrap();
        let ctx = WaveformTrackContext {
            root: dir.path().to_path_buf(),
            track_id: "mic".into(),
            track_type: TrackType::MicAudio,
            segments: vec![SegmentSummary {
                track_id: "mic".into(),
                relative_path: "audio/mic.wav".into(),
                start_us: 0,
                end_us: 5_000_000,
                size_bytes: wav.len() as u64,
                media_timescale: rate,
                media_start_value: 0,
                host_anchor_us: 0,
                is_keyframe_start: None,
                available: true,
            }],
            retained: vec![RetainedInterval {
                start_us: 0,
                end_us: 5_000_000,
                media: None,
            }],
            edited_duration_us: 5_000_000,
        };
        let config = SilenceConfig {
            padding_ms: 0,
            ..SilenceConfig::default()
        };
        let scan = scan_track_silence(&ctx, &config).unwrap();
        assert_eq!(scan.covered, vec![(0, 5_000_000)]);
        let speech = crate::webcam_focus::speech_ranges(&scan.covered, &scan.source_ranges);
        assert_eq!(speech.len(), 2, "{speech:?}");
        assert!(speech[0].1.abs_diff(2_000_000) < 60_000, "{speech:?}");
        assert!(speech[1].0.abs_diff(3_000_000) < 60_000, "{speech:?}");
        // A journal can declare more time than the WAV contains. Transcript gap
        // analysis must not treat that missing tail as available audio.
        let mut longer = ctx;
        longer.segments[0].end_us = 6_000_000;
        let short_file = scan_track_silence(&longer, &config).unwrap();
        assert_eq!(short_file.covered, vec![(0, 5_000_000)]);
    }
}
