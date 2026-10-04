//! Bounded stereo PCM mixer shared by playback and export. Times are output frames.
use super::polish::PolishPlan;
use crate::project::{
    pcm::PcmReader,
    reader::{safe_path, SegmentSummary},
    revision::EditDocument,
};
use crate::sequence::{sources::sound_segments, Role};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;
pub const CHUNK_FRAMES: usize = 4_800;
const FADE_US: u64 = 8_000;
const RADIUS: i64 = 24;
/// Sub-sample phases the resampling table holds; a 1/1024-sample step is far below hearing.
const SINC_PHASES: usize = 1024;
const SINC_TAPS: usize = (2 * RADIUS) as usize;

/// Windowed-sinc weights for every phase, for low-pass `cutoff`: row `p` holds the taps
/// `center - RADIUS + 1 ..= center + RADIUS` for a position `p / SINC_PHASES` past `center`.
/// Built once per cutoff and kept, so the mixer does no trigonometry per sample.
fn sinc_table(cutoff: f64) -> std::sync::Arc<Vec<f64>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static TABLES: OnceLock<Mutex<HashMap<u64, Arc<Vec<f64>>>>> = OnceLock::new();
    let tables = TABLES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut tables = tables.lock().unwrap_or_else(|e| e.into_inner());
    tables
        .entry(cutoff.to_bits())
        .or_insert_with(|| {
            let mut table = Vec::with_capacity((SINC_PHASES + 1) * SINC_TAPS);
            for phase in 0..=SINC_PHASES {
                let frac = phase as f64 / SINC_PHASES as f64;
                for j in 0..SINC_TAPS {
                    let distance = (j as i64 - RADIUS + 1) as f64 - frac;
                    let x = std::f64::consts::PI * distance * cutoff;
                    let sinc = if x.abs() < 1e-12 { 1.0 } else { x.sin() / x };
                    table.push(
                        sinc * (0.5
                            + 0.5 * (std::f64::consts::PI * distance / RADIUS as f64).cos()),
                    );
                }
            }
            Arc::new(table)
        })
        .clone()
}

/// One clip of sound where it plays, faded where it does not join on to its neighbour.
#[derive(Clone)]
struct Span {
    edited_start: u64,
    edited_end: u64,
    source_start: u64,
    source_end: u64,
    fade_in: bool,
    fade_out: bool,
    /// The audio track it plays on: its mix gain, ducking and noise reduction.
    lane: String,
    gain: f64,
    segments: Arc<Vec<SegmentSummary>>,
}

pub struct AudioMixer {
    root: PathBuf,
    /// Every audible clip, by where it starts.
    spans: Vec<Span>,
    polish: Option<PolishPlan>,
    pub total_frames: u64,
}

/// One audio track as the mixer plays it.
pub struct Lane {
    /// The audio track's id: its mix settings are kept under it.
    pub id: String,
    /// Its clips are speech (they duck background lanes and get noise reduction by default).
    pub speech: bool,
    pub gain: f64,
    pub clips: Vec<LaneClip>,
}

/// One clip of a lane: where it plays and the segments of its stream.
pub struct LaneClip {
    pub edited_start: u64,
    pub in_us: u64,
    pub len: u64,
    /// Where its sound ends: a clip running to it needs no fade out.
    pub source_end: u64,
    pub segments: Arc<Vec<SegmentSummary>>,
}

/// The audible audio tracks of a document (muted ones and silent clips left out).
pub fn lanes(root: &Path, document: &EditDocument) -> Vec<Lane> {
    let mut sounds: HashMap<(String, String), Arc<Vec<SegmentSummary>>> = HashMap::new();
    let mut out = Vec::new();
    for track in document.sequence.audio_tracks().filter(|t| !t.muted) {
        let gain = document.audio.track_gain(&track.id);
        if gain <= 0.0 {
            continue;
        }
        let mut speech = false;
        let mut clips = Vec::new();
        for clip in &track.clips {
            let Some(asset) = document.asset(&clip.asset) else {
                continue;
            };
            speech |= crate::sequence::clip_role(&document.assets, track, clip) == Some(Role::Mic);
            let segments = sounds
                .entry((clip.asset.clone(), clip.stream.clone()))
                .or_insert_with(|| {
                    // Sound that cannot be found plays as silence.
                    Arc::new(sound_segments(root, asset, &clip.stream).unwrap_or_default())
                })
                .clone();
            clips.push(LaneClip {
                edited_start: clip.start_us,
                in_us: clip.in_us,
                len: clip.duration_us,
                source_end: asset.duration_us,
                segments,
            });
        }
        out.push(Lane {
            id: track.id.clone(),
            speech,
            gain,
            clips,
        });
    }
    out
}

fn ceil_frame(us: u64) -> u64 {
    ((us as u128 * SAMPLE_RATE as u128).div_ceil(1_000_000)) as u64
}
impl AudioMixer {
    pub fn new(root: &Path, document: &EditDocument) -> Result<Self, String> {
        let duration = document.duration_us();
        let lanes = lanes(root, document);
        let polish = PolishPlan::build(root, &document.audio, &lanes, duration);
        let mut spans = Vec::new();
        for lane in &lanes {
            for (i, clip) in lane.clips.iter().enumerate() {
                // Sound fades in and out where a clip cuts into it, not where it starts or ends
                // anyway, nor where it carries straight on into the next clip on the lane.
                let joins = |a: &LaneClip, b: &LaneClip| {
                    a.edited_start + a.len == b.edited_start
                        && a.in_us + a.len == b.in_us
                        && Arc::ptr_eq(&a.segments, &b.segments)
                };
                spans.push(Span {
                    edited_start: clip.edited_start,
                    edited_end: clip.edited_start + clip.len,
                    source_start: clip.in_us,
                    source_end: clip.in_us + clip.len,
                    fade_in: clip.in_us > 0 && !(i > 0 && joins(&lane.clips[i - 1], clip)),
                    fade_out: clip.in_us + clip.len < clip.source_end
                        && !lane.clips.get(i + 1).is_some_and(|next| joins(clip, next)),
                    lane: lane.id.clone(),
                    gain: lane.gain,
                    segments: clip.segments.clone(),
                });
            }
        }
        spans.sort_by_key(|s| s.edited_start);
        Ok(Self {
            root: root.into(),
            spans,
            polish,
            total_frames: (duration as u128 * SAMPLE_RATE as u128 / 1_000_000) as u64,
        })
    }
    pub fn has_audio(&self) -> bool {
        self.spans
            .iter()
            .any(|s| s.segments.iter().any(|segment| segment.available))
    }
    pub fn read_frames(&self, start: u64, count: usize) -> Result<Vec<i16>, String> {
        if count > CHUNK_FRAMES {
            return Err("Audio request exceeds chunk limit".into());
        }
        let count = count.min(self.total_frames.saturating_sub(start) as usize);
        let end = start + count as u64;
        let mut out = vec![0f64; count * 2];
        let last = self
            .spans
            .partition_point(|s| ceil_frame(s.edited_start) < end);
        for span in self.spans[..last]
            .iter()
            .filter(|s| ceil_frame(s.edited_end) > start)
        {
            self.mix_span(span, start, end, &mut out)?;
        }
        Ok(out
            .into_iter()
            .map(|s| match &self.polish {
                Some(plan) => plan.finish(s),
                None => s,
            })
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0).round() as i16)
            .collect())
    }

    /// Adds one span's audio, faded at its cut edges, to `out` (frames `start..end`).
    fn mix_span(&self, span: &Span, start: u64, end: u64, out: &mut [f64]) -> Result<(), String> {
        let mut mixed = vec![0f64; out.len()];
        {
            let out = &mut mixed;
            let a = start.max(ceil_frame(span.edited_start));
            let b = end.min(ceil_frame(span.edited_end));
            let source_a = span.source_start
                + ((a as u128 * 1_000_000 / SAMPLE_RATE as u128) as u64)
                    .saturating_sub(span.edited_start);
            let source_b = span.source_start
                + ((b as u128 * 1_000_000 / SAMPLE_RATE as u128) as u64)
                    .saturating_sub(span.edited_start)
                + 1;
            for (track_gain, track) in [(&span.gain, span.segments.as_slice())] {
                let first = track.partition_point(|s| s.end_us <= source_a);
                for segment in track[first..].iter().take_while(|s| s.start_us < source_b) {
                    if !segment.available {
                        continue;
                    }
                    let overlap_start = span.source_start.max(segment.start_us);
                    let overlap_end = span.source_end.min(segment.end_us);
                    if overlap_end <= overlap_start {
                        continue;
                    }
                    let lo = a.max(ceil_frame(
                        span.edited_start + overlap_start - span.source_start,
                    ));
                    let hi = b.min(ceil_frame(
                        span.edited_start + overlap_end - span.source_start,
                    ));
                    if hi <= lo {
                        continue;
                    }
                    let path = safe_path(&self.root, &segment.relative_path)?;
                    let mut reader = PcmReader::open(&path)
                        .map_err(|e| format!("{}: {}", segment.relative_path, e))?;
                    let info = reader.info().clone();
                    let rate = info.sample_rate as f64;
                    let local = |frame: u64| {
                        ((frame as f64 / SAMPLE_RATE as f64 * 1e6 - span.edited_start as f64
                            + span.source_start as f64
                            - segment.start_us as f64)
                            * rate
                            / 1e6)
                            .max(0.0)
                    };
                    let read_start = (local(lo).floor() as i64 - RADIUS).max(0) as u64;
                    let read_end =
                        ((local(hi - 1).ceil() as u64) + RADIUS as u64 + 1).min(info.frame_count);
                    if read_end <= read_start {
                        continue;
                    }
                    let channels = info.channels as usize;
                    // Noise reduction where its lane has it on, recorded or imported.
                    let denoiser = self
                        .polish
                        .as_ref()
                        .and_then(|plan| plan.denoiser(&segment.relative_path));
                    let (samples, got) = match denoiser {
                        Some(denoiser) => {
                            let (from, to) = denoiser.input_range(read_start, read_end);
                            let input = read_padded(&mut reader, from, to)?;
                            let samples = denoiser.process(&input, channels, read_start, read_end);
                            (samples, (read_end - read_start) as usize)
                        }
                        None => {
                            reader.seek_to_frame(read_start)?;
                            let want = (read_end - read_start) as usize;
                            let mut samples = vec![0f32; want * channels];
                            let got = reader.read_frames(&mut samples, want)?;
                            (samples, got)
                        }
                    };
                    if got == 0 {
                        continue;
                    }
                    // Ducking where its lane has it on, under speech anywhere on the timeline.
                    let ducked = self
                        .polish
                        .as_ref()
                        .is_some_and(|plan| plan.ducks(&span.lane));
                    // Fold a sample frame to stereo: stereo stays stereo; more channels
                    // average even ones left and odd ones right.
                    let fold = |values: &[f32]| -> [f64; 2] {
                        match values {
                            [mono] => [*mono as f64, *mono as f64],
                            [left, right] => [*left as f64, *right as f64],
                            _ => {
                                let mut sums = [0.0f64; 2];
                                let mut counts = [0usize; 2];
                                for (c, v) in values.iter().enumerate() {
                                    sums[c % 2] += *v as f64;
                                    counts[c % 2] += 1;
                                }
                                [
                                    sums[0] / counts[0].max(1) as f64,
                                    sums[1] / counts[1].max(1) as f64,
                                ]
                            }
                        }
                    };
                    let frame_at = |tap: i64| {
                        let index = tap.clamp(read_start as i64, read_start as i64 + got as i64 - 1)
                            as usize
                            - read_start as usize;
                        &samples[index * channels..(index + 1) * channels]
                    };
                    // Already at the output rate (imported sound, most recordings): take the
                    // nearest sample, at most half a sample (10 us) off.
                    let direct = info.sample_rate == SAMPLE_RATE;
                    let table = (!direct).then(|| sinc_table((SAMPLE_RATE as f64 / rate).min(1.0)));
                    for frame in lo..hi {
                        let pos = local(frame);
                        if pos >= info.frame_count as f64 {
                            continue;
                        }
                        let (stereo, weights) = match &table {
                            None => (fold(frame_at(pos.round() as i64)), 1.0),
                            Some(table) => {
                                // Windowed-sinc low-pass resampling prevents aliasing when
                                // downsampling; the weights come from a table by sub-sample phase.
                                let center = pos.floor() as i64;
                                let phase =
                                    ((pos - center as f64) * SINC_PHASES as f64).round() as usize;
                                let row = &table[phase * SINC_TAPS..(phase + 1) * SINC_TAPS];
                                let mut stereo = [0f64; 2];
                                let mut weights = 0.0;
                                for (j, weight) in row.iter().enumerate() {
                                    let v = fold(frame_at(center - RADIUS + 1 + j as i64));
                                    stereo[0] += weight * v[0];
                                    stereo[1] += weight * v[1];
                                    weights += weight;
                                }
                                (stereo, weights)
                            }
                        };
                        if weights.abs() > 1e-12 {
                            let gain = track_gain
                                * match &self.polish {
                                    Some(plan) if ducked => plan.lane_duck_gain(
                                        &span.lane,
                                        frame as f64 * 1e6 / SAMPLE_RATE as f64,
                                    ),
                                    _ => 1.0,
                                };
                            for ch in 0..2 {
                                out[(frame - start) as usize * 2 + ch] +=
                                    stereo[ch] / weights * gain;
                            }
                        }
                    }
                }
            }
            for frame in a..b {
                let t = frame as f64 * 1e6 / SAMPLE_RATE as f64;
                let mut gain = 1.0f64;
                if span.fade_in {
                    gain =
                        gain.min(((t - span.edited_start as f64) / FADE_US as f64).clamp(0.0, 1.0));
                }
                if span.fade_out {
                    gain =
                        gain.min(((span.edited_end as f64 - t) / FADE_US as f64).clamp(0.0, 1.0));
                }
                for ch in 0..2 {
                    out[(frame - start) as usize * 2 + ch] *= gain;
                }
            }
        }
        for (sum, sample) in out.iter_mut().zip(mixed) {
            *sum += sample;
        }
        Ok(())
    }
}

/// Reads frames `[from, to)` interleaved, with zeros before and after the file.
fn read_padded(reader: &mut PcmReader, from: i64, to: i64) -> Result<Vec<f32>, String> {
    let info = reader.info().clone();
    let channels = info.channels as usize;
    let mut out = vec![0f32; (to - from).max(0) as usize * channels];
    let a = from.max(0) as u64;
    let b = (to.max(0) as u64).min(info.frame_count);
    if b > a {
        reader.seek_to_frame(a)?;
        let offset = (a as i64 - from) as usize * channels;
        let want = (b - a) as usize;
        reader.read_frames(&mut out[offset..offset + want * channels], want)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::generate_pcm16_wav;
    use crate::project::audio::TrackMix;
    use crate::sequence::edit::SequenceEdit;
    use crate::sequence::{Asset, AssetKind, Clip, Fit, Stream, StreamKind};

    /// A project of sound files, each `(id, role, rate, channels, samples)` an audio asset in
    /// `assets/media` placed at the start of its own audio track (A1, A2, ...).
    fn files(sounds: Vec<(&str, Role, u32, u16, Vec<i16>)>) -> (tempfile::TempDir, EditDocument) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("assets/media")).unwrap();
        let mut doc = EditDocument::default();
        for (id, role, rate, channels, values) in sounds {
            let relative = format!("assets/media/{id}.wav");
            std::fs::write(
                dir.path().join(&relative),
                generate_pcm16_wav(rate, channels, &values),
            )
            .unwrap();
            let duration =
                (values.len() as u128 / channels as u128 * 1_000_000 / rate as u128) as u64;
            doc.assets.push(Asset {
                id: id.into(),
                name: format!("{id}.wav"),
                kind: AssetKind::Audio,
                path: dir
                    .path()
                    .join(format!("{id}.wav"))
                    .to_string_lossy()
                    .into_owned(),
                streams: vec![Stream {
                    id: "sound0".into(),
                    kind: StreamKind::Sound,
                    role,
                    name: id.into(),
                    audio_path: Some(relative),
                    fps: None,
                }],
                duration_us: duration,
                width: 0,
                height: 0,
                pauses: Vec::new(),
                missing: false,
            });
            let at = doc.sequence.audio_tracks().count();
            doc.sequence = crate::sequence::edit::apply(
                &doc.sequence,
                &doc.assets,
                &SequenceEdit::PlaceAsset {
                    asset_id: id.into(),
                    at_us: 0,
                    track_id: None,
                    streams: vec![],
                    range: None,
                },
            )
            .unwrap()
            .0;
            assert_eq!(doc.sequence.audio_tracks().count(), at + 1);
        }
        (dir, doc)
    }

    fn fixture(rate: u32, channels: u16, values: Vec<i16>) -> (tempfile::TempDir, EditDocument) {
        files(vec![("mic", Role::Mic, rate, channels, values)])
    }

    /// The id of audio track `n` (0 is A1).
    fn lane(doc: &EditDocument, n: usize) -> String {
        doc.sequence.audio_tracks().nth(n).unwrap().id.clone()
    }

    /// Clips of the first track's sound at `(start, in, length)`.
    fn clips(doc: &mut EditDocument, parts: &[(u64, u64, u64)]) {
        let asset = doc.sequence.tracks[0].clips[0].asset.clone();
        doc.sequence.tracks[0].clips = parts
            .iter()
            .enumerate()
            .map(|(i, &(start_us, in_us, duration_us))| Clip {
                id: format!("c{}", i + 100),
                asset: asset.clone(),
                stream: "sound0".into(),
                start_us,
                in_us,
                duration_us,
                link: None,
                fit: Fit::Contain,
            })
            .collect();
    }

    /// Mic: hiss, then a 220 Hz "voice" from 1 s to 2 s, then hiss. System: a steady
    /// 1 kHz stereo tone. Both 3 s at 48 kHz.
    fn polish_fixture(with_system: bool) -> (tempfile::TempDir, EditDocument) {
        let rate = 48_000usize;
        let mut state = 9u64;
        let mic: Vec<i16> = (0..rate * 3)
            .map(|i| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let hiss = ((state >> 40) as f64 / (1u64 << 24) as f64 - 0.5) * 600.0;
                let voice = if (rate..rate * 2).contains(&i) {
                    9000.0 * (std::f64::consts::TAU * 220.0 * i as f64 / rate as f64).sin()
                } else {
                    0.0
                };
                (hiss + voice) as i16
            })
            .collect();
        let mut sounds = vec![("mic", Role::Mic, rate as u32, 1, mic)];
        if with_system {
            let system: Vec<i16> = (0..rate * 3)
                .flat_map(|i| {
                    let v =
                        8000.0 * (std::f64::consts::TAU * 1000.0 * i as f64 / rate as f64).sin();
                    [v as i16, v as i16]
                })
                .collect();
            sounds.push(("system", Role::Background, rate as u32, 2, system));
        }
        files(sounds)
    }

    fn mix_all(mixer: &AudioMixer) -> Vec<f64> {
        let mut out = Vec::new();
        let mut frame = 0;
        while frame < mixer.total_frames {
            let pcm = mixer.read_frames(frame, CHUNK_FRAMES).unwrap();
            out.extend(pcm.iter().map(|&s| s as f64 / 32767.0));
            frame += CHUNK_FRAMES as u64;
        }
        out
    }

    fn rms(samples: &[f64]) -> f64 {
        (samples.iter().map(|s| s * s).sum::<f64>() / samples.len() as f64).sqrt()
    }

    /// Interleaved stereo samples for `[a, b)` seconds.
    fn seconds(samples: &[f64], a: f64, b: f64) -> &[f64] {
        &samples[(a * 96_000.0) as usize..(b * 96_000.0) as usize]
    }

    #[test]
    fn ducking_lowers_background_sound_only_under_speech() {
        let (dir, mut doc) = polish_fixture(true);
        let plain = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        doc.audio.duck_system_audio = true;
        doc.audio.duck_db = 12.0;
        let ducked = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let diff: Vec<f64> = plain.iter().zip(&ducked).map(|(a, b)| a - b).collect();
        assert!(
            rms(seconds(&diff, 0.2, 0.6)) < 1e-3,
            "no speech, no ducking"
        );
        // Under speech the system tone (rms 8000/32767/sqrt 2) drops by 12 dB.
        let system_rms = 8000.0 / 32767.0 / 2f64.sqrt();
        let removed = rms(seconds(&diff, 1.2, 1.8)) / system_rms;
        let expected = 1.0 - 10f64.powf(-12.0 / 20.0);
        assert!((removed - expected).abs() < 0.03, "removed {removed}");
        // The hold keeps it ducked briefly after speech, then it recovers.
        assert!(rms(seconds(&diff, 2.0, 2.2)) > 0.05);
        assert!(rms(seconds(&diff, 2.8, 3.0)) < 1e-3);
    }

    /// Ducking set on one lane lowers it; marking the speech track as background takes the
    /// speech away, so nothing ducks.
    #[test]
    fn a_lane_ducks_under_speech_on_any_other_lane() {
        let (dir, mut doc) = polish_fixture(true);
        let plain = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let system = lane(&doc, 1);
        doc.audio.tracks.insert(
            system,
            TrackMix {
                duck_db: Some(12.0),
                ..Default::default()
            },
        );
        let ducked = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let diff: Vec<f64> = plain.iter().zip(&ducked).map(|(a, b)| a - b).collect();
        let music_rms = 8000.0 / 32767.0 / 2f64.sqrt();
        let removed = rms(seconds(&diff, 1.2, 1.8)) / music_rms;
        let expected = 1.0 - 10f64.powf(-12.0 / 20.0);
        assert!((removed - expected).abs() < 0.03, "removed {removed}");

        let speech = doc.sequence.audio_tracks().next().unwrap().id.clone();
        let index = doc
            .sequence
            .tracks
            .iter()
            .position(|t| t.id == speech)
            .unwrap();
        doc.sequence.tracks[index].role = Some(Role::Background);
        let unmarked = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let diff: Vec<f64> = plain.iter().zip(&unmarked).map(|(a, b)| a - b).collect();
        assert!(rms(seconds(&diff, 1.2, 1.8)) < 1e-3);
    }

    #[test]
    fn track_mute_and_volume_apply_to_the_mix() {
        let (dir, mut doc) = polish_fixture(true);
        let both = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let system = doc
            .sequence
            .tracks
            .iter()
            .position(|t| t.id == lane(&doc, 1))
            .unwrap();
        doc.sequence.tracks[system].muted = true;
        let mixer = AudioMixer::new(dir.path(), &doc).unwrap();
        assert!(mixer.has_audio());
        let mic_only = mix_all(&mixer);
        // The 1 kHz system tone is gone; only the mic is left.
        let system_rms = 8000.0 / 32767.0 / 2f64.sqrt();
        let diff: Vec<f64> = both.iter().zip(&mic_only).map(|(a, b)| a - b).collect();
        assert!((rms(seconds(&diff, 0.2, 2.8)) / system_rms - 1.0).abs() < 0.02);

        doc.sequence.tracks[system].muted = false;
        doc.audio.tracks.insert(
            lane(&doc, 1),
            TrackMix {
                volume_db: -6.0,
                ..Default::default()
            },
        );
        let quieter = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let diff: Vec<f64> = quieter.iter().zip(&mic_only).map(|(a, b)| a - b).collect();
        let level = rms(seconds(&diff, 0.2, 2.8)) / system_rms;
        assert!(
            (level - 10f64.powf(-6.0 / 20.0)).abs() < 0.02,
            "level {level}"
        );

        for track in doc.sequence.tracks.iter_mut() {
            track.muted = true;
        }
        let silent = AudioMixer::new(dir.path(), &doc).unwrap();
        assert!(
            !silent.has_audio(),
            "every track muted means no audio stream"
        );
    }

    #[test]
    fn noise_reduction_quiets_hiss_and_keeps_speech() {
        let (dir, mut doc) = polish_fixture(false);
        let plain = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        doc.audio.noise_reduction = true;
        doc.audio.noise_reduction_db = 18.0;
        let clean = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let hiss_drop =
            20.0 * (rms(seconds(&clean, 0.2, 0.8)) / rms(seconds(&plain, 0.2, 0.8))).log10();
        assert!(hiss_drop < -10.0, "hiss dropped {hiss_drop} dB");
        let voice_change =
            20.0 * (rms(seconds(&clean, 1.2, 1.8)) / rms(seconds(&plain, 1.2, 1.8))).log10();
        assert!(voice_change.abs() < 1.0, "voice changed {voice_change} dB");
    }

    #[test]
    fn normalization_hits_the_target_and_polish_is_chunk_independent() {
        use crate::dsp::loudness::{integrated_lufs, KWeighting};
        let (dir, mut doc) = polish_fixture(true);
        doc.audio.normalize = true;
        doc.audio.target_lufs = -20.0;
        let out = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let mut filters = [KWeighting::new(48_000), KWeighting::new(48_000)];
        let blocks: Vec<f64> = out
            .chunks(9_600)
            .map(|block| {
                block
                    .chunks_exact(2)
                    .map(|f| {
                        let l = filters[0].process(f[0] as f32);
                        let r = filters[1].process(f[1] as f32);
                        l * l + r * r
                    })
                    .sum::<f64>()
                    / (block.len() / 2) as f64
            })
            .collect();
        let lufs = integrated_lufs(&blocks).unwrap();
        assert!((lufs + 20.0).abs() < 1.0, "measured {lufs} LUFS");

        doc.audio.noise_reduction = true;
        doc.audio.duck_system_audio = true;
        doc.sequence = crate::sequence::edit::apply(
            &doc.sequence,
            &doc.assets,
            &SequenceEdit::DeleteRange {
                ranges: vec![crate::zoom::EditedRange {
                    start_us: 1_100_007,
                    end_us: 1_500_013,
                }],
                ripple: Some(true),
            },
        )
        .unwrap()
        .0;
        let mixer = AudioMixer::new(dir.path(), &doc).unwrap();
        let a = mixer.read_frames(52_700, 300).unwrap();
        let mut b = mixer.read_frames(52_700, 100).unwrap();
        b.extend(mixer.read_frames(52_800, 200).unwrap());
        assert_eq!(a, b);
    }

    #[test]
    fn resamples_24khz_and_keeps_right_channel() {
        let (dir, doc) = fixture(24_000, 2, (0..24_000).flat_map(|_| [0, 16384]).collect());
        let mixer = AudioMixer::new(dir.path(), &doc).unwrap();
        assert_eq!(mixer.total_frames, 48_000);
        for start in [0, 24_000, 43_200] {
            let pcm = mixer.read_frames(start, CHUNK_FRAMES).unwrap();
            assert_eq!(pcm.len(), 9_600);
            assert!(pcm
                .chunks_exact(2)
                .all(|s| s[0] == 0 && (s[1] as i32 - 16384).abs() <= 2));
        }
    }

    #[test]
    fn cuts_are_chunk_independent_and_long_timeline_is_streamed() {
        let (dir, mut doc) = fixture(44_100, 1, vec![16384; 44100]);
        clips(&mut doc, &[(0, 0, 100_013), (100_013, 300_017, 100_082)]);
        let mixer = AudioMixer::new(dir.path(), &doc).unwrap();
        let a = mixer.read_frames(4_700, 300).unwrap();
        let mut b = mixer.read_frames(4_700, 100).unwrap();
        b.extend(mixer.read_frames(4_800, 200).unwrap());
        assert_eq!(a, b);
        assert!(a[202].abs() < 100);
        doc.assets[0].duration_us = 3_600_000_000;
        clips(&mut doc, &[(0, 0, 3_600_000_000)]);
        let mixer = AudioMixer::new(dir.path(), &doc).unwrap();
        assert_eq!(
            mixer.read_frames(48_000 * 3599, CHUNK_FRAMES).unwrap(),
            vec![0; CHUNK_FRAMES * 2]
        );
        assert!(mixer.read_frames(0, CHUNK_FRAMES + 1).is_err());
    }

    /// Clips that carry straight on from each other play through without a dip; a real cut
    /// fades in and out.
    #[test]
    fn joined_clips_play_through_and_cuts_fade() {
        let (dir, mut doc) = fixture(48_000, 1, vec![16384; 48_000]);
        clips(&mut doc, &[(0, 0, 500_000), (500_000, 500_000, 500_000)]);
        let mixer = AudioMixer::new(dir.path(), &doc).unwrap();
        let pcm = mixer.read_frames(24_000 - 10, 20).unwrap();
        assert!(
            pcm.iter().all(|&s| (s as i32 - 16384).abs() <= 2),
            "{pcm:?}"
        );
        clips(&mut doc, &[(0, 0, 500_000), (500_000, 600_000, 400_000)]);
        let mixer = AudioMixer::new(dir.path(), &doc).unwrap();
        let pcm = mixer.read_frames(24_000 - 10, 20).unwrap();
        assert!(pcm.iter().any(|&s| (s as i32) < 8_000), "the cut fades");
    }

    #[test]
    fn downsampling_rejects_above_nyquist_energy() {
        let values = (0..19_200)
            .map(|i| {
                (16000.0 * (std::f64::consts::TAU * 60_000.0 * i as f64 / 192_000.0).sin()) as i16
            })
            .collect();
        let (dir, doc) = fixture(192_000, 1, values);
        let mixer = AudioMixer::new(dir.path(), &doc).unwrap();
        let pcm = mixer.read_frames(0, CHUNK_FRAMES).unwrap();
        let peak = pcm[100..pcm.len() - 100]
            .iter()
            .map(|s| s.abs())
            .max()
            .unwrap();
        assert!(peak < 100, "aliased signal peak={peak}");
    }
}
