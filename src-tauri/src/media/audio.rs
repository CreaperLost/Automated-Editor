//! Bounded stereo PCM mixer shared by playback and export. Times are output frames.
use super::polish::PolishPlan;
use crate::project::{
    pcm::PcmReader,
    reader::{safe_path, SegmentSummary, TrackSummary},
    revision::EditDocument,
    TrackType,
};
use std::path::{Path, PathBuf};
pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;
pub const CHUNK_FRAMES: usize = 4_800;
const FADE_US: u64 = 8_000;
const RADIUS: i64 = 24;
#[derive(Clone)]
struct Span {
    edited_start: u64,
    edited_end: u64,
    source_start: u64,
    source_end: u64,
    fade_in: bool,
    fade_out: bool,
}
pub struct AudioMixer {
    root: PathBuf,
    spans: Vec<Span>,
    /// Audible audio tracks with their mix gain; muted tracks are left out.
    tracks: Vec<(TrackType, f64, Vec<SegmentSummary>)>,
    polish: Option<PolishPlan>,
    pub total_frames: u64,
}
fn ceil_frame(us: u64) -> u64 {
    ((us as u128 * SAMPLE_RATE as u128).div_ceil(1_000_000)) as u64
}
impl AudioMixer {
    pub fn new(
        root: &Path,
        document: &EditDocument,
        tracks: &[(TrackSummary, Vec<SegmentSummary>)],
    ) -> Result<Self, String> {
        let duration = document.edited_duration_us()?;
        let mut cursor = 0;
        let intervals = &document.retained_intervals;
        let spans = intervals
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let span = Span {
                    edited_start: cursor,
                    edited_end: cursor + s.end_us - s.start_us,
                    source_start: s.start_us,
                    source_end: s.end_us,
                    fade_in: i > 0 && intervals[i - 1].end_us != s.start_us,
                    fade_out: i + 1 < intervals.len() && s.end_us != intervals[i + 1].start_us,
                };
                cursor = span.edited_end;
                span
            })
            .collect();
        let tracks = tracks
            .iter()
            .filter(|(t, _)| {
                matches!(
                    t.descriptor.track_type,
                    TrackType::MicAudio | TrackType::SystemAudio
                )
            })
            .map(|(t, s)| {
                let gain = document.audio.track_gain(&t.descriptor.id);
                (t.descriptor.track_type, gain, s.clone())
            })
            .filter(|(_, gain, _)| *gain > 0.0)
            .collect::<Vec<_>>();
        let polish =
            PolishPlan::build(root, &document.audio, &document.retained_intervals, &tracks);
        Ok(Self {
            root: root.into(),
            spans,
            tracks,
            polish,
            total_frames: (duration as u128 * SAMPLE_RATE as u128 / 1_000_000) as u64,
        })
    }
    pub fn has_audio(&self) -> bool {
        self.tracks
            .iter()
            .any(|(_, _, track)| track.iter().any(|s| s.available))
    }
    pub fn read_frames(&self, start: u64, count: usize) -> Result<Vec<i16>, String> {
        if count > CHUNK_FRAMES {
            return Err("Audio request exceeds chunk limit".into());
        }
        let count = count.min(self.total_frames.saturating_sub(start) as usize);
        let end = start + count as u64;
        let mut out = vec![0f64; count * 2];
        let first = self
            .spans
            .partition_point(|s| ceil_frame(s.edited_end) <= start);
        for span in self.spans[first..]
            .iter()
            .take_while(|s| ceil_frame(s.edited_start) < end)
        {
            let a = start.max(ceil_frame(span.edited_start));
            let b = end.min(ceil_frame(span.edited_end));
            let source_a = span.source_start
                + ((a as u128 * 1_000_000 / SAMPLE_RATE as u128) as u64)
                    .saturating_sub(span.edited_start);
            let source_b = span.source_start
                + ((b as u128 * 1_000_000 / SAMPLE_RATE as u128) as u64)
                    .saturating_sub(span.edited_start)
                + 1;
            for (track_type, track_gain, track) in &self.tracks {
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
                    let denoiser = match (track_type, &self.polish) {
                        (TrackType::MicAudio, Some(plan)) => plan.denoiser(&segment.relative_path),
                        _ => None,
                    };
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
                    let ducked = *track_type == TrackType::SystemAudio && self.polish.is_some();
                    for frame in lo..hi {
                        let pos = local(frame);
                        if pos >= info.frame_count as f64 {
                            continue;
                        }
                        let mut stereo = [0f64; 2];
                        let mut weights = 0.0;
                        // Windowed-sinc low-pass resampling prevents aliasing when downsampling.
                        let cutoff = (SAMPLE_RATE as f64 / rate).min(1.0);
                        let center = pos.floor() as i64;
                        for tap in (center - RADIUS + 1)..=(center + RADIUS) {
                            let distance = tap as f64 - pos;
                            let x = std::f64::consts::PI * distance * cutoff;
                            let sinc = if x.abs() < 1e-12 { 1.0 } else { x.sin() / x };
                            let weight = sinc
                                * (0.5
                                    + 0.5
                                        * (std::f64::consts::PI * distance / RADIUS as f64).cos());
                            let sample_frame = tap
                                .clamp(read_start as i64, read_start as i64 + got as i64 - 1)
                                as usize
                                - read_start as usize;
                            let values =
                                &samples[sample_frame * channels..(sample_frame + 1) * channels];
                            for ch in 0..2 {
                                let v = if channels == 1 {
                                    values[0] as f64
                                } else {
                                    // Stereo stays stereo; multichannel uses an explicit even/odd fold-down.
                                    let mut n = 0;
                                    let mut sum = 0.0;
                                    for c in (ch..channels).step_by(2) {
                                        sum += values[c] as f64;
                                        n += 1;
                                    }
                                    sum / n as f64
                                };
                                stereo[ch] += weight * v;
                            }
                            weights += weight;
                        }
                        if weights.abs() > 1e-12 {
                            let gain = track_gain
                                * match &self.polish {
                                    Some(plan) if ducked => plan.duck_gain(
                                        frame as f64 * 1e6 / SAMPLE_RATE as f64
                                            - span.edited_start as f64
                                            + span.source_start as f64,
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
        Ok(out
            .into_iter()
            .map(|s| match &self.polish {
                Some(plan) => plan.finish(s),
                None => s,
            })
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0).round() as i16)
            .collect())
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
    use crate::{
        fixtures::generate_pcm16_wav,
        project::{RetainedInterval, TrackDescriptor},
    };
    fn fixture(
        rate: u32,
        channels: u16,
        values: Vec<i16>,
    ) -> (
        tempfile::TempDir,
        EditDocument,
        Vec<(TrackSummary, Vec<SegmentSummary>)>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let bytes = generate_pcm16_wav(rate, channels, &values);
        std::fs::write(dir.path().join("audio.wav"), &bytes).unwrap();
        let duration = (values.len() as u128 / channels as u128 * 1_000_000 / rate as u128) as u64;
        let document = EditDocument::from_retained(vec![RetainedInterval {
            start_us: 0,
            end_us: duration,
        }])
        .unwrap();
        let track = TrackSummary {
            descriptor: TrackDescriptor {
                id: "mic".into(),
                track_type: TrackType::MicAudio,
                codec: "pcm".into(),
                relative_path: "audio.wav".into(),
                width: None,
                height: None,
                fps: None,
                sample_rate: Some(rate),
                channels: Some(channels),
                gaps_total: 0,
                media_timescale: Some(rate),
            },
            segment_count: 1,
            available_segment_count: 1,
        };
        let segment = SegmentSummary {
            track_id: "mic".into(),
            relative_path: "audio.wav".into(),
            start_us: 0,
            end_us: duration,
            size_bytes: bytes.len() as u64,
            media_timescale: rate,
            media_start_value: 0,
            host_anchor_us: 0,
            is_keyframe_start: Some(true),
            available: true,
        };
        (dir, document, vec![(track, vec![segment])])
    }
    /// Mic: hiss, then a 220 Hz "voice" from 1 s to 2 s, then hiss. System: a steady
    /// 1 kHz stereo tone. Both 3 s at 48 kHz.
    fn polish_fixture(
        with_system: bool,
    ) -> (
        tempfile::TempDir,
        EditDocument,
        Vec<(TrackSummary, Vec<SegmentSummary>)>,
    ) {
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
        let (dir, document, mut tracks) = fixture(rate as u32, 1, mic);
        if with_system {
            let system: Vec<i16> = (0..rate * 3)
                .flat_map(|i| {
                    let v =
                        8000.0 * (std::f64::consts::TAU * 1000.0 * i as f64 / rate as f64).sin();
                    [v as i16, v as i16]
                })
                .collect();
            let bytes = generate_pcm16_wav(rate as u32, 2, &system);
            std::fs::write(dir.path().join("system.wav"), &bytes).unwrap();
            let (mut track, mut segment) = tracks[0].clone();
            track.descriptor.id = "system".into();
            track.descriptor.track_type = TrackType::SystemAudio;
            track.descriptor.relative_path = "system.wav".into();
            track.descriptor.channels = Some(2);
            segment[0].track_id = "system".into();
            segment[0].relative_path = "system.wav".into();
            segment[0].size_bytes = bytes.len() as u64;
            tracks.push((track, segment));
        }
        (dir, document, tracks)
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
    fn ducking_lowers_system_audio_only_under_speech() {
        let (dir, mut doc, tracks) = polish_fixture(true);
        let plain = mix_all(&AudioMixer::new(dir.path(), &doc, &tracks).unwrap());
        doc.audio.duck_system_audio = true;
        doc.audio.duck_db = 12.0;
        let ducked = mix_all(&AudioMixer::new(dir.path(), &doc, &tracks).unwrap());
        let diff: Vec<f64> = plain.iter().zip(&ducked).map(|(a, b)| a - b).collect();
        assert!(
            rms(seconds(&diff, 0.2, 0.6)) < 1e-3,
            "no speech, no ducking"
        );
        // Under speech the system tone (rms 8000/32767/sqrt 2) drops by 12 dB.
        let system_rms = 8000.0 / 32767.0 / 2f64.sqrt();
        let removed = rms(seconds(&diff, 1.2, 1.8)) / system_rms;
        let expected = 1.0 - 10f64.powf(-12.0 / 20.0);
        assert!((removed - expected).abs() < 0.02, "removed {removed}");
        // The hold keeps it ducked briefly after speech, then it recovers.
        assert!(rms(seconds(&diff, 2.0, 2.2)) > 0.05);
        assert!(rms(seconds(&diff, 2.8, 3.0)) < 1e-3);
    }

    #[test]
    fn track_mute_and_volume_apply_to_the_mix() {
        use crate::project::audio::TrackMix;
        let (dir, mut doc, tracks) = polish_fixture(true);
        let both = mix_all(&AudioMixer::new(dir.path(), &doc, &tracks).unwrap());
        doc.audio.tracks.insert(
            "system".into(),
            TrackMix {
                muted: true,
                volume_db: 0.0,
            },
        );
        let mixer = AudioMixer::new(dir.path(), &doc, &tracks).unwrap();
        assert!(mixer.has_audio());
        let mic_only = mix_all(&mixer);
        // The 1 kHz system tone is gone; only the mic is left.
        let system_rms = 8000.0 / 32767.0 / 2f64.sqrt();
        let diff: Vec<f64> = both.iter().zip(&mic_only).map(|(a, b)| a - b).collect();
        assert!((rms(seconds(&diff, 0.2, 2.8)) / system_rms - 1.0).abs() < 0.02);

        doc.audio.tracks.get_mut("system").unwrap().muted = false;
        doc.audio.tracks.get_mut("system").unwrap().volume_db = -6.0;
        let quieter = mix_all(&AudioMixer::new(dir.path(), &doc, &tracks).unwrap());
        let diff: Vec<f64> = quieter.iter().zip(&mic_only).map(|(a, b)| a - b).collect();
        let level = rms(seconds(&diff, 0.2, 2.8)) / system_rms;
        assert!(
            (level - 10f64.powf(-6.0 / 20.0)).abs() < 0.02,
            "level {level}"
        );

        doc.audio.tracks.insert(
            "mic".into(),
            TrackMix {
                muted: true,
                volume_db: 0.0,
            },
        );
        doc.audio.tracks.get_mut("system").unwrap().muted = true;
        let silent = AudioMixer::new(dir.path(), &doc, &tracks).unwrap();
        assert!(
            !silent.has_audio(),
            "every track muted means no audio stream"
        );
    }

    #[test]
    fn noise_reduction_quiets_hiss_and_keeps_speech() {
        let (dir, mut doc, tracks) = polish_fixture(false);
        let plain = mix_all(&AudioMixer::new(dir.path(), &doc, &tracks).unwrap());
        doc.audio.noise_reduction = true;
        doc.audio.noise_reduction_db = 18.0;
        let clean = mix_all(&AudioMixer::new(dir.path(), &doc, &tracks).unwrap());
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
        let (dir, mut doc, tracks) = polish_fixture(true);
        doc.audio.normalize = true;
        doc.audio.target_lufs = -20.0;
        let out = mix_all(&AudioMixer::new(dir.path(), &doc, &tracks).unwrap());
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
        doc.retained_intervals = vec![
            RetainedInterval {
                start_us: 0,
                end_us: 1_100_007,
            },
            RetainedInterval {
                start_us: 1_500_013,
                end_us: 3_000_000,
            },
        ];
        let mixer = AudioMixer::new(dir.path(), &doc, &tracks).unwrap();
        let a = mixer.read_frames(52_700, 300).unwrap();
        let mut b = mixer.read_frames(52_700, 100).unwrap();
        b.extend(mixer.read_frames(52_800, 200).unwrap());
        assert_eq!(a, b);
    }

    #[test]
    fn resamples_24khz_and_keeps_right_channel() {
        let (dir, doc, tracks) = fixture(24_000, 2, (0..24_000).flat_map(|_| [0, 16384]).collect());
        let mixer = AudioMixer::new(dir.path(), &doc, &tracks).unwrap();
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
        let (dir, mut doc, tracks) = fixture(44_100, 1, vec![16384; 44100]);
        doc.retained_intervals = vec![
            RetainedInterval {
                start_us: 0,
                end_us: 100_013,
            },
            RetainedInterval {
                start_us: 300_017,
                end_us: 400_099,
            },
        ];
        let mixer = AudioMixer::new(dir.path(), &doc, &tracks).unwrap();
        let a = mixer.read_frames(4_700, 300).unwrap();
        let mut b = mixer.read_frames(4_700, 100).unwrap();
        b.extend(mixer.read_frames(4_800, 200).unwrap());
        assert_eq!(a, b);
        assert!(a[202].abs() < 100);
        doc.retained_intervals = vec![RetainedInterval {
            start_us: 0,
            end_us: 3_600_000_000,
        }];
        let mixer = AudioMixer::new(dir.path(), &doc, &tracks).unwrap();
        assert_eq!(
            mixer.read_frames(48_000 * 3599, CHUNK_FRAMES).unwrap(),
            vec![0; CHUNK_FRAMES * 2]
        );
        assert!(mixer.read_frames(0, CHUNK_FRAMES + 1).is_err());
    }
    #[test]
    fn downsampling_rejects_above_nyquist_energy() {
        let values = (0..19_200)
            .map(|i| {
                (16000.0 * (std::f64::consts::TAU * 60_000.0 * i as f64 / 192_000.0).sin()) as i16
            })
            .collect();
        let (dir, doc, tracks) = fixture(192_000, 1, values);
        let mixer = AudioMixer::new(dir.path(), &doc, &tracks).unwrap();
        let pcm = mixer.read_frames(0, CHUNK_FRAMES).unwrap();
        let peak = pcm[100..pcm.len() - 100]
            .iter()
            .map(|s| s.abs())
            .max()
            .unwrap();
        assert!(peak < 100, "aliased signal peak={peak}");
    }
}
