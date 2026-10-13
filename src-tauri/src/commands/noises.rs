//! Mouth noises and knocks: found in a sound's own audio (see [`crate::dsp::noises`]),
//! reviewed, then muted in place. Muting keeps the timing of every track; pause detection
//! hears a muted moment as silence, so jump cuts remove it with the pause around it.
use super::{mutate_opened, require_handle, AppState};
use crate::dsp::noises::{find_noises, NoiseConfig, NoiseKind};
use crate::project::{pcm::PcmReader, reader::safe_path, OpenedProject};
use crate::sequence::{SourceRange, StreamKind};
use serde::{Deserialize, Serialize};

/// Sound analyzed at a time, and the sound either side of it read with it (so a noise at the
/// edge of a piece is judged against what is around it).
const PIECE_US: u64 = 30_000_000;
const MARGIN_US: u64 = 500_000;

/// One noise found, in its sound's own time and where it plays on the timeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundNoise {
    pub source_start_us: u64,
    pub source_end_us: u64,
    pub start_us: u64,
    pub end_us: u64,
    pub kind: NoiseKind,
    pub contrast_db: f32,
    /// Voiced speech is right next to it: it may be part of a word.
    pub near_speech: bool,
    /// Already muted.
    pub muted: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoiseScan {
    pub key: String,
    /// The project revision the timeline positions are for.
    pub revision: u64,
    pub noises: Vec<FoundNoise>,
    pub diagnostics: Vec<String>,
}

/// Finds the noises in sound `key` (`<asset>.<stream>`) where the timeline plays it.
pub fn detect_mouth_noises_impl(
    state: &AppState,
    project_handle: String,
    key: String,
    config: NoiseConfig,
) -> Result<NoiseScan, String> {
    config.validate()?;
    let (root, segments, clock, muted, revision) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        let document = reader.document();
        let source = document.stream_ref(&key)?;
        let asset = document.asset(&source.asset).ok_or("Unknown sound")?;
        if asset
            .stream(&source.stream)
            .is_none_or(|s| s.kind != StreamKind::Sound)
        {
            return Err("That stream has no sound".into());
        }
        (
            reader.root().to_path_buf(),
            crate::sequence::sources::sound_segments(reader.root(), asset, &source.stream)?,
            crate::sequence::clock::stream_clock(&document.sequence, &asset.id, &source.stream),
            document
                .noise_mutes
                .get(&key)
                .map(|ranges| {
                    ranges
                        .iter()
                        .map(|r| (r.start_us, r.end_us))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            document.revision,
        )
    };
    let mut diagnostics = Vec::new();
    let mut found = Vec::new();
    for segment in segments.iter().filter(|s| s.available) {
        let path = match safe_path(&root, &segment.relative_path) {
            Ok(path) => path,
            Err(error) => {
                diagnostics.push(format!("Skipped {}: {error}", segment.relative_path));
                continue;
            }
        };
        match segment_noises(&path, &config) {
            Ok(noises) => found.extend(noises.into_iter().filter_map(|mut noise| {
                noise.start_us += segment.start_us;
                noise.end_us = (noise.end_us + segment.start_us).min(segment.end_us);
                (noise.end_us > noise.start_us).then_some(noise)
            })),
            Err(error) => diagnostics.push(format!("Skipped {}: {error}", segment.relative_path)),
        }
    }
    // Segments of a long recording overlap a little: a noise found in both counts once.
    found.sort_by_key(|n| n.start_us);
    found.dedup_by(|later, earlier| later.start_us < earlier.end_us);
    let noises = found
        .into_iter()
        .filter_map(|noise| {
            let (start_us, end_us) = clock.edited_span_of(noise.start_us, noise.end_us)?;
            let first = muted.partition_point(|&(_, b)| b <= noise.start_us);
            Some(FoundNoise {
                source_start_us: noise.start_us,
                source_end_us: noise.end_us,
                start_us,
                end_us: end_us.max(start_us + 1),
                kind: noise.kind,
                contrast_db: noise.contrast_db,
                near_speech: noise.near_speech,
                muted: muted
                    .get(first)
                    .is_some_and(|&(a, b)| a <= noise.start_us && noise.end_us <= b),
            })
        })
        .collect::<Vec<_>>();
    let mut noises = noises;
    noises.sort_by_key(|n| n.start_us);
    Ok(NoiseScan {
        key,
        revision,
        noises,
        diagnostics,
    })
}

/// The noises in one sound file, in its own time, read a piece at a time.
fn segment_noises(
    path: &std::path::Path,
    config: &NoiseConfig,
) -> Result<Vec<crate::dsp::noises::Noise>, String> {
    let mut reader = PcmReader::open(path)?;
    let info = reader.info().clone();
    let channels = info.channels as usize;
    let rate = info.sample_rate;
    let frames_for = |us: u64| (us as u128 * rate as u128 / 1_000_000) as u64;
    let (piece, margin) = (frames_for(PIECE_US), frames_for(MARGIN_US));
    let mut out = Vec::new();
    let mut start = 0u64;
    while start < info.frame_count {
        let from = start.saturating_sub(margin);
        let to = (start + piece + margin).min(info.frame_count);
        reader.seek_to_frame(from)?;
        let want = (to - from) as usize;
        let mut samples = vec![0f32; want * channels];
        let got = reader.read_frames(&mut samples, want)?;
        let mono: Vec<f32> = samples[..got * channels]
            .chunks_exact(channels.max(1))
            .map(|frame| frame.iter().sum::<f32>() / channels.max(1) as f32)
            .collect();
        let offset = info.frame_us(from);
        let (core_start, core_end) = (info.frame_us(start), info.frame_us(start + piece));
        out.extend(
            find_noises(&mono, rate, config)
                .into_iter()
                .map(|mut noise| {
                    noise.start_us += offset;
                    noise.end_us += offset;
                    noise
                })
                .filter(|noise| core_start <= noise.start_us && noise.start_us < core_end),
        );
        start += piece;
    }
    Ok(out)
}

/// Mutes (or with `unmute`, unmutes) moments of sound `key`'s own time: one undoable edit.
pub fn project_noise_mutes_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    key: String,
    ranges: Vec<SourceRange>,
    unmute: bool,
) -> Result<OpenedProject, String> {
    if ranges.is_empty() {
        return Err("Choose what to mute".into());
    }
    mutate_opened(state, project_handle, |reader| {
        reader.update_noise_mutes(expected_revision, &key, &ranges, unmute)
    })
}
