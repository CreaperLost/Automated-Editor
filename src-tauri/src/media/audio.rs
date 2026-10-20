//! Bounded stereo PCM mixer shared by playback and export. Times are output frames.
use super::polish::PolishPlan;
use crate::project::{
    pcm::PcmReader,
    reader::{safe_path, SegmentSummary},
    revision::EditDocument,
};
use crate::sequence::{join_slack, source_len, sources::sound_segments, timeline_len, Role};
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
    /// Moments of its sound that are muted, in the sound's own time.
    mutes: Mutes,
    /// How fast it plays its sound; other than 1, stretched in time keeping its pitch.
    speed: f64,
    /// Clips that carry straight on from each other share the grains of one stretch (so a
    /// split plays as if it were not there): its number, and where it starts on the timeline
    /// and in the sound.
    stretch: usize,
    stretch_edited: u64,
    stretch_source: u64,
}

/// How long a muted moment fades out before it and back in after it (so the mute itself
/// makes no click).
const MUTE_FADE_US: f64 = 3_000.0;

/// Muted moments of a sound, in its own time (merged, in order).
type Mutes = Arc<Vec<(u64, u64)>>;

/// The gain at `source_us` of a sound with `mutes` (merged, in order): silent inside a muted
/// moment, fading over [`MUTE_FADE_US`] either side of it.
fn mute_gain(mutes: &[(u64, u64)], source_us: f64) -> f64 {
    let at = mutes.partition_point(|&(_, end)| (end as f64) < source_us);
    let distance = |i: usize| {
        mutes.get(i).map_or(f64::MAX, |&(a, b)| {
            if source_us < a as f64 {
                a as f64 - source_us
            } else if source_us >= b as f64 {
                source_us - b as f64
            } else {
                0.0
            }
        })
    };
    let nearest = distance(at).min(at.checked_sub(1).map_or(f64::MAX, distance));
    (nearest / MUTE_FADE_US).clamp(0.0, 1.0)
}

impl Span {
    /// The sound itself, at the output rate: output frame `n` is its time `n / SAMPLE_RATE`.
    fn source(&self) -> Self {
        Self {
            edited_start: 0,
            edited_end: SOURCE_END_US,
            source_start: 0,
            source_end: SOURCE_END_US,
            fade_in: false,
            fade_out: false,
            lane: self.lane.clone(),
            gain: 1.0,
            segments: self.segments.clone(),
            mutes: self.mutes.clone(),
            speed: 1.0,
            stretch: 0,
            stretch_edited: 0,
            stretch_source: 0,
        }
    }
}

/// Far past any sound (about 140 years), where [`Span::source`] ends.
const SOURCE_END_US: u64 = 1 << 52;

pub struct AudioMixer {
    root: PathBuf,
    /// Every audible clip, by lane, each lane's by where they start. Clips on one lane never
    /// overlap, so a chunk finds the few it needs by binary search.
    lanes: Vec<Vec<Span>>,
    polish: Option<PolishPlan>,
    clean_files: super::declick::CleanFiles,
    /// Open sound files, so sequential chunks do not open and parse them again.
    readers: std::sync::Mutex<Vec<(PathBuf, PcmReader)>>,
    /// Grain shifts of stretched sound, by stretch and block of grains.
    shifts: std::sync::Mutex<GrainShifts>,
    /// Clicks and whooshes on their own lane, when it makes any sound.
    effects: Option<crate::sound_effects::SoundEffectsLane>,
    pub total_frames: u64,
}

/// Grain moves of stretched sound, by stretch and block of grains.
type GrainShifts = HashMap<(usize, i64), Arc<Vec<i64>>>;

/// Sound files a mixer keeps open.
const MAX_OPEN_READERS: usize = 16;

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
    /// How much of its sound it plays.
    pub len: u64,
    /// How long it plays on the timeline (`len` at `speed`).
    pub edited_len: u64,
    pub speed: f64,
    /// Where its sound ends: a clip running to it needs no fade out.
    pub source_end: u64,
    pub segments: Arc<Vec<SegmentSummary>>,
    /// Moments of its sound that are muted, in the sound's own time (merged, in order).
    pub mutes: Arc<Vec<(u64, u64)>>,
}

impl LaneClip {
    /// Where its sound's time `source_us` plays on the timeline.
    pub fn edited_at(&self, source_us: u64) -> u64 {
        self.edited_start + timeline_len(source_us.saturating_sub(self.in_us), self.speed)
    }

    /// Its sound's time playing at timeline time `edited_us`, unless that is before it.
    pub fn source_at(&self, edited_us: u64) -> Option<u64> {
        Some(self.in_us + source_len(edited_us.checked_sub(self.edited_start)?, self.speed))
    }
}

/// The audible audio tracks of a document (muted ones and silent clips left out).
pub fn lanes(root: &Path, document: &EditDocument) -> Vec<Lane> {
    let mut sounds: HashMap<(String, String), Arc<Vec<SegmentSummary>>> = HashMap::new();
    let mut muted: HashMap<(String, String), Mutes> = HashMap::new();
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
            let mutes = muted
                .entry((clip.asset.clone(), clip.stream.clone()))
                .or_insert_with(|| {
                    let key = crate::sequence::StreamRef::new(&clip.asset, &clip.stream).key();
                    Arc::new(
                        document
                            .noise_mutes
                            .get(&key)
                            .map(|ranges| ranges.iter().map(|r| (r.start_us, r.end_us)).collect())
                            .unwrap_or_default(),
                    )
                })
                .clone();
            clips.push(LaneClip {
                edited_start: clip.start_us,
                in_us: clip.in_us,
                len: clip.source_len_us(),
                edited_len: clip.duration_us,
                speed: clip.speed,
                source_end: asset.duration_us,
                segments,
                mutes,
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
        let mut lanes = lanes(root, document);
        let clean_files = super::declick::prepare(root, &document.audio, &lanes)?;
        // The sound effects lane ducks under speech like background sound.
        let effects = crate::sound_effects::SoundEffectsLane::new(document);
        let mut audio = std::borrow::Cow::Borrowed(&document.audio);
        if effects.is_some() {
            use crate::sound_effects::{DUCK_DB, LANE_ID};
            audio
                .to_mut()
                .tracks
                .entry(LANE_ID.into())
                .or_default()
                .duck_db = Some(DUCK_DB);
            lanes.push(Lane {
                id: LANE_ID.into(),
                speech: false,
                gain: 1.0,
                clips: Vec::new(),
            });
        }
        let polish = PolishPlan::build(root, &audio, &lanes, duration);
        let mut by_lane = Vec::new();
        let mut stretches = 0;
        for lane in &lanes {
            let mut spans: Vec<Span> = Vec::new();
            for (i, clip) in lane.clips.iter().enumerate() {
                // Sound fades in and out where a clip cuts into it, not where it starts or ends
                // anyway, nor where it carries straight on into the next clip on the lane.
                let joins = |a: &LaneClip, b: &LaneClip| {
                    a.edited_start + a.edited_len == b.edited_start
                        && a.speed == b.speed
                        && (a.in_us + a.len).abs_diff(b.in_us) <= join_slack(a.speed)
                        && Arc::ptr_eq(&a.segments, &b.segments)
                };
                let joined = i > 0 && joins(&lane.clips[i - 1], clip);
                let (stretch, stretch_edited, stretch_source) = match spans.last() {
                    Some(last) if joined => {
                        (last.stretch, last.stretch_edited, last.stretch_source)
                    }
                    _ => {
                        stretches += 1;
                        (stretches, clip.edited_start, clip.in_us)
                    }
                };
                spans.push(Span {
                    edited_start: clip.edited_start,
                    edited_end: clip.edited_start + clip.edited_len,
                    source_start: clip.in_us,
                    source_end: clip.in_us + clip.len,
                    fade_in: clip.in_us > 0 && !joined,
                    fade_out: clip.in_us + clip.len < clip.source_end
                        && !lane.clips.get(i + 1).is_some_and(|next| joins(clip, next)),
                    lane: lane.id.clone(),
                    gain: lane.gain,
                    segments: clip.segments.clone(),
                    mutes: clip.mutes.clone(),
                    speed: clip.speed,
                    stretch,
                    stretch_edited,
                    stretch_source,
                });
            }
            spans.sort_by_key(|s| s.edited_start);
            by_lane.push(spans);
        }
        Ok(Self {
            root: root.into(),
            lanes: by_lane,
            polish,
            clean_files,
            readers: std::sync::Mutex::new(Vec::new()),
            shifts: std::sync::Mutex::new(HashMap::new()),
            effects,
            total_frames: (duration as u128 * SAMPLE_RATE as u128 / 1_000_000) as u64,
        })
    }
    pub fn has_audio(&self) -> bool {
        self.effects.is_some()
            || self
                .lanes
                .iter()
                .flatten()
                .any(|s| s.segments.iter().any(|segment| segment.available))
    }

    /// Runs `read` on an open reader of `path`, opening it once.
    fn with_reader<T>(
        &self,
        path: &Path,
        read: impl FnOnce(&mut PcmReader) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut readers = self.readers.lock().unwrap_or_else(|e| e.into_inner());
        let index = match readers.iter().position(|(p, _)| p == path) {
            Some(index) => index,
            None => {
                if readers.len() >= MAX_OPEN_READERS {
                    readers.remove(0);
                }
                readers.push((path.to_path_buf(), PcmReader::open(path)?));
                readers.len() - 1
            }
        };
        read(&mut readers[index].1)
    }
    pub fn read_frames(&self, start: u64, count: usize) -> Result<Vec<i16>, String> {
        if count > CHUNK_FRAMES {
            return Err("Audio request exceeds chunk limit".into());
        }
        let count = count.min(self.total_frames.saturating_sub(start) as usize);
        let end = start + count as u64;
        let mut out = vec![0f64; count * 2];
        for spans in &self.lanes {
            let first = spans.partition_point(|s| ceil_frame(s.edited_end) <= start);
            for span in spans[first..]
                .iter()
                .take_while(|s| ceil_frame(s.edited_start) < end)
            {
                self.mix_span(span, start, end, &mut out)?;
            }
        }
        if let Some(effects) = &self.effects {
            effects.mix(start, &mut out, |t| match &self.polish {
                Some(plan) => plan.lane_duck_gain(crate::sound_effects::LANE_ID, t),
                None => 1.0,
            });
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
        let mut mixed = if span.speed == 1.0 {
            self.render(span, start, end, true)?
        } else {
            self.stretch(span, start, end)?
        };
        let a = start.max(ceil_frame(span.edited_start));
        let b = end.min(ceil_frame(span.edited_end));
        for frame in a..b {
            let t = frame as f64 * 1e6 / SAMPLE_RATE as f64;
            let mut gain = 1.0f64;
            if span.fade_in {
                gain = gain.min(((t - span.edited_start as f64) / FADE_US as f64).clamp(0.0, 1.0));
            }
            if span.fade_out {
                gain = gain.min(((span.edited_end as f64 - t) / FADE_US as f64).clamp(0.0, 1.0));
            }
            // A stretched span's mutes are in the sound it stretches (see `sound`).
            if span.speed == 1.0 && !span.mutes.is_empty() {
                let source = span.source_start as f64 + (t - span.edited_start as f64);
                gain = gain.min(mute_gain(&span.mutes, source));
            }
            for ch in 0..2 {
                mixed[(frame - start) as usize * 2 + ch] *= gain;
            }
        }
        for (sum, sample) in out.iter_mut().zip(mixed) {
            *sum += sample;
        }
        Ok(())
    }

    /// One span's audio at normal speed for frames `start..end`. On the timeline it has its
    /// track's gain and ducking; off it (the sound itself, to be stretched) it has neither.
    fn render(
        &self,
        span: &Span,
        start: u64,
        end: u64,
        on_timeline: bool,
    ) -> Result<Vec<f64>, String> {
        let mut mixed = vec![0f64; (end - start) as usize * 2];
        let gain = if on_timeline { span.gain } else { 1.0 };
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
            for (track_gain, track) in [(&gain, span.segments.as_slice())] {
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
                    let path = match self
                        .clean_files
                        .get(&(span.lane.clone(), segment.relative_path.clone()))
                    {
                        Some(clean) => clean.path().to_path_buf(),
                        None => safe_path(&self.root, &segment.relative_path)?,
                    };
                    let info = self
                        .with_reader(&path, |reader| Ok(reader.info().clone()))
                        .map_err(|e| format!("{}: {}", segment.relative_path, e))?;
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
                        .and_then(|plan| plan.denoiser(&span.lane, &segment.relative_path));
                    let (samples, got) = match denoiser {
                        Some(denoiser) => {
                            let (from, to) = denoiser.input_range(read_start, read_end);
                            let input =
                                self.with_reader(&path, |reader| read_padded(reader, from, to))?;
                            let samples = denoiser.process(&input, channels, read_start, read_end);
                            (samples, (read_end - read_start) as usize)
                        }
                        None => self.with_reader(&path, |reader| {
                            reader.seek_to_frame(read_start)?;
                            let want = (read_end - read_start) as usize;
                            let mut samples = vec![0f32; want * channels];
                            let got = reader.read_frames(&mut samples, want)?;
                            Ok((samples, got))
                        })?,
                    };
                    if got == 0 {
                        continue;
                    }
                    // Ducking where its lane has it on, under speech anywhere on the timeline.
                    let ducked = on_timeline
                        && self
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
        }
        Ok(mixed)
    }

    /// A span played faster or slower, for frames `start..end`: its sound stretched in time
    /// keeping its pitch (WSOLA), with its track's gain and ducking.
    ///
    /// Grains of [`GRAIN_HOP`] either side of their centre, one every [`GRAIN_HOP`] output
    /// frames from where the stretch starts, each read from the sound where the speed puts it,
    /// moved by up to [`GRAIN_SEEK`] to line up with the grain before. The moves are worked
    /// out a block of grains at a time, each block from a few grains before it, so any chunk
    /// of output is the same however the output is read.
    fn stretch(&self, span: &Span, start: u64, end: u64) -> Result<Vec<f64>, String> {
        let mut mixed = vec![0f64; (end - start) as usize * 2];
        let a = (start.max(ceil_frame(span.edited_start))) as i64;
        let b = (end.min(ceil_frame(span.edited_end))) as i64;
        if b <= a {
            return Ok(mixed);
        }
        let grid = Grains::of(span);
        let (k0, k1) = (grid.first_reaching(a), grid.last_reaching(b - 1));
        let shifts = self.grain_shifts(span, &grid, k0, k1)?;
        let centres: Vec<i64> = (k0..=k1)
            .zip(&shifts)
            .map(|(k, shift)| grid.input(k) + shift)
            .collect();
        let sound = self.sound(
            span,
            centres
                .iter()
                .map(|c| (c - GRAIN_HOP, c + GRAIN_HOP))
                .collect(),
        )?;
        for (k, centre) in (k0..=k1).zip(centres) {
            let at = grid.output(k);
            let samples = sound.frames(centre - GRAIN_HOP, 2 * GRAIN_HOP as usize);
            for n in (at - GRAIN_HOP).max(a)..(at + GRAIN_HOP).min(b) {
                let j = n - at;
                let weight =
                    0.5 * (1.0 + (std::f64::consts::PI * j as f64 / GRAIN_HOP as f64).cos());
                let from = (j + GRAIN_HOP) as usize * 2;
                for ch in 0..2 {
                    mixed[(n as u64 - start) as usize * 2 + ch] += weight * samples[from + ch];
                }
            }
        }
        let ducked = self.polish.as_ref().filter(|plan| plan.ducks(&span.lane));
        for n in a as u64..b as u64 {
            let gain = span.gain
                * ducked.map_or(1.0, |plan| {
                    plan.lane_duck_gain(&span.lane, n as f64 * 1e6 / SAMPLE_RATE as f64)
                });
            for ch in 0..2 {
                mixed[(n - start) as usize * 2 + ch] *= gain;
            }
        }
        Ok(mixed)
    }

    /// The moves of grains `k0..=k1` of `span`'s stretch, from blocks worked out once.
    fn grain_shifts(
        &self,
        span: &Span,
        grid: &Grains,
        k0: i64,
        k1: i64,
    ) -> Result<Vec<i64>, String> {
        let mut out = Vec::with_capacity((k1 - k0 + 1) as usize);
        for block in k0.div_euclid(SHIFT_BLOCK)..=k1.div_euclid(SHIFT_BLOCK) {
            let key = (span.stretch, block);
            let cached = self
                .shifts
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&key)
                .cloned();
            let shifts = match cached {
                Some(shifts) => shifts,
                None => {
                    let shifts = Arc::new(self.block_shifts(span, grid, block)?);
                    let mut cache = self.shifts.lock().unwrap_or_else(|e| e.into_inner());
                    if cache.len() >= MAX_SHIFT_BLOCKS {
                        cache.clear();
                    }
                    cache.insert(key, shifts.clone());
                    shifts
                }
            };
            let first = block * SHIFT_BLOCK;
            for k in k0.max(first)..=k1.min(first + SHIFT_BLOCK - 1) {
                out.push(shifts[(k - first) as usize]);
            }
        }
        Ok(out)
    }

    /// Works out the moves of block `block`'s grains: from a few grains before it (or the
    /// stretch's first, which stays where it is), each grain moves to where its start best
    /// carries on from the last one's centre.
    fn block_shifts(&self, span: &Span, grid: &Grains, block: i64) -> Result<Vec<i64>, String> {
        let first = block * SHIFT_BLOCK;
        let from = (first - SHIFT_LEAD).max(0);
        let end = first + SHIFT_BLOCK;
        let sound = self.sound(
            span,
            (from..end)
                .map(|k| {
                    let m = grid.input(k);
                    (m - GRAIN_SEEK - GRAIN_HOP, m + GRAIN_SEEK + GRAIN_HOP)
                })
                .collect(),
        )?;
        let mono = |at: i64, len: usize| -> Vec<f32> {
            sound
                .frames(at, len)
                .chunks_exact(2)
                .map(|s| ((s[0] + s[1]) * 0.5) as f32)
                .collect()
        };
        let mut shifts = Vec::with_capacity(SHIFT_BLOCK as usize);
        if from == first {
            shifts.push(0);
        }
        let mut last = grid.input(from);
        for k in from + 1..end {
            let m = grid.input(k);
            let target = mono(last, GRAIN_HOP as usize);
            let candidates = mono(
                m - GRAIN_SEEK - GRAIN_HOP,
                (2 * GRAIN_SEEK + GRAIN_HOP) as usize,
            );
            let shift = best_shift(&candidates, &target);
            last = m + shift;
            if k >= first {
                shifts.push(shift);
            }
        }
        Ok(shifts)
    }

    /// `span`'s sound itself at the output rate over `windows` of frames of its own time
    /// (merged where they meet), silent before it starts.
    fn sound(&self, span: &Span, mut windows: Vec<(i64, i64)>) -> Result<Sound, String> {
        windows.sort_unstable();
        let mut merged: Vec<(i64, i64)> = Vec::new();
        for (a, b) in windows {
            match merged.last_mut() {
                Some(last) if a <= last.1 => last.1 = last.1.max(b),
                _ => merged.push((a, b)),
            }
        }
        let source = span.source();
        let mut pieces = Vec::with_capacity(merged.len());
        for (a, b) in merged {
            let mut samples = vec![0f64; (b - a) as usize * 2];
            let from = a.max(0);
            if b > from {
                let rendered = self.render(&source, from as u64, b as u64, false)?;
                samples[(from - a) as usize * 2..].copy_from_slice(&rendered);
                // Its muted moments, before it is stretched.
                if !span.mutes.is_empty() {
                    for n in from..b {
                        let gain = mute_gain(&span.mutes, n as f64 * 1e6 / SAMPLE_RATE as f64);
                        if gain < 1.0 {
                            let at = (n - a) as usize * 2;
                            samples[at] *= gain;
                            samples[at + 1] *= gain;
                        }
                    }
                }
            }
            pieces.push((a, samples));
        }
        Ok(Sound { pieces })
    }
}

/// Output frames either side of a stretch grain's centre, and between centres (15 ms).
const GRAIN_HOP: i64 = 720;
/// How far a grain may move in the sound to line up with the one before (10 ms).
const GRAIN_SEEK: i64 = 480;
/// Grains whose moves are worked out together.
const SHIFT_BLOCK: i64 = 32;
/// Grains before a block its moves are worked out from.
const SHIFT_LEAD: i64 = 4;
/// Blocks of moves kept (about 8 minutes of stretched sound).
const MAX_SHIFT_BLOCKS: usize = 1024;

/// The grains of a stretch: grain `k` is centred `k` hops after where the stretch starts on
/// the timeline, read from as far into its sound as the speed takes it.
struct Grains {
    output: i64,
    source: f64,
    speed: f64,
}

impl Grains {
    fn of(span: &Span) -> Self {
        Self {
            output: ceil_frame(span.stretch_edited) as i64,
            source: span.stretch_source as f64 * SAMPLE_RATE as f64 / 1e6,
            speed: span.speed,
        }
    }

    fn output(&self, k: i64) -> i64 {
        self.output + k * GRAIN_HOP
    }

    /// Where grain `k` is read from before it moves.
    fn input(&self, k: i64) -> i64 {
        (self.source + (k * GRAIN_HOP) as f64 * self.speed).round() as i64
    }

    /// The first grain reaching output frame `n` or later.
    fn first_reaching(&self, n: i64) -> i64 {
        ((n - self.output - GRAIN_HOP).div_euclid(GRAIN_HOP) + 1).max(0)
    }

    /// The last grain reaching output frame `n` or earlier.
    fn last_reaching(&self, n: i64) -> i64 {
        (n - self.output + GRAIN_HOP).div_euclid(GRAIN_HOP)
    }
}

/// How far (within [`GRAIN_SEEK`]) to move a grain so the start of it best matches `target`:
/// `candidates` holds the sound from `GRAIN_SEEK` before its start to `GRAIN_SEEK` after.
/// Searched coarsely, then around the best; no move wins ties.
fn best_shift(candidates: &[f32], target: &[f32]) -> i64 {
    let len = target.len();
    let score = |shift: i64, step: usize| {
        let window = &candidates[(shift + GRAIN_SEEK) as usize..][..len];
        let (mut cross, mut energy) = (0f32, 0f32);
        for i in (0..len).step_by(step) {
            cross += window[i] * target[i];
            energy += window[i] * window[i];
        }
        cross / (energy + 1e-9).sqrt()
    };
    let mut best = (score(0, 4), 0);
    for shift in (-GRAIN_SEEK..=GRAIN_SEEK).step_by(4) {
        let s = score(shift, 4);
        if s > best.0 {
            best = (s, shift);
        }
    }
    let coarse = best.1;
    best = (score(coarse, 1), coarse);
    for shift in (coarse - 3).max(-GRAIN_SEEK)..=(coarse + 3).min(GRAIN_SEEK) {
        let s = score(shift, 1);
        if s > best.0 {
            best = (s, shift);
        }
    }
    best.1
}

/// Stretches of a sound at the output rate: (first frame, stereo samples).
struct Sound {
    pieces: Vec<(i64, Vec<f64>)>,
}

impl Sound {
    /// Frames `[at, at + len)`, which lie in one stretch.
    fn frames(&self, at: i64, len: usize) -> &[f64] {
        let i = self.pieces.partition_point(|(start, _)| *start <= at) - 1;
        let (start, samples) = &self.pieces[i];
        let from = (at - start) as usize * 2;
        &samples[from..from + len * 2]
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

    #[test]
    fn mouth_click_cleanup_preserves_timing_background_and_chunked_reads() {
        if super::super::ffmpeg::ffmpeg_path().is_err() {
            return;
        }
        let mut values = vec![0i16; 48_000];
        values[24_000] = 25_000;
        let (dir, mut doc) = files(vec![
            ("mic", Role::Mic, 48_000, 1, values.clone()),
            ("pc", Role::Background, 48_000, 1, values),
        ]);
        let original = std::fs::read(dir.path().join("assets/media/mic.wav")).unwrap();
        let before = AudioMixer::new(dir.path(), &doc).unwrap();
        let plain = before.read_frames(23_000, 2_000).unwrap();
        doc.audio.mouth_clicks = true;
        doc.audio.mouth_click_strength = 75;
        let repaired = AudioMixer::new(dir.path(), &doc).unwrap();
        assert_eq!(before.total_frames, repaired.total_frames);
        assert_eq!(
            repaired.clean_files.len(),
            1,
            "Background audio is never repaired"
        );
        let whole = repaired.read_frames(23_000, 2_000).unwrap();
        let pieces: Vec<_> = [
            repaired.read_frames(23_000, 777).unwrap(),
            repaired.read_frames(23_777, 1_223).unwrap(),
        ]
        .concat();
        assert_eq!(whole, pieces);
        assert!(
            whole[2_000].abs() < plain[2_000].abs(),
            "The mic impulse must be reduced"
        );
        assert!(whole[2_000] > 20_000, "The PC impulse must remain");
        assert_eq!(
            std::fs::read(dir.path().join("assets/media/mic.wav")).unwrap(),
            original
        );
        // Bypass is sample-identical, including after another mixer has prepared cleanup.
        doc.audio.mouth_clicks = false;
        assert_eq!(
            AudioMixer::new(dir.path(), &doc)
                .unwrap()
                .read_frames(23_000, 2_000)
                .unwrap(),
            plain
        );
        // Reordered/repeated excerpts read the same repaired source, not a new repair at cuts.
        doc.audio.mouth_clicks = true;
        doc.sequence
            .tracks
            .retain(|t| t.clips.iter().any(|c| c.asset == "mic"));
        clips(
            &mut doc,
            &[(0, 400_000, 200_000), (200_000, 400_000, 200_000)],
        );
        let repeated = AudioMixer::new(dir.path(), &doc).unwrap();
        assert_eq!(
            repeated.read_frames(2_000, 3_000).unwrap(),
            repeated.read_frames(11_600, 3_000).unwrap()
        );
    }

    #[test]
    fn noise_reduction_is_each_tracks_own_where_two_play_one_sound() {
        // A second of faint hiss, then a tone over it: enough quiet for a noise profile.
        let rate = 48_000u32;
        let mut state = 1u64;
        let values: Vec<i16> = (0..rate * 2)
            .map(|i| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let hiss = ((state >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 300.0;
                let tone = if i >= rate {
                    8000.0 * (std::f32::consts::TAU * 220.0 * i as f32 / rate as f32).sin()
                } else {
                    0.0
                };
                (hiss + tone) as i16
            })
            .collect();
        let (dir, mut doc) = fixture(rate, 1, values);
        doc.sequence = crate::sequence::edit::apply(
            &doc.sequence,
            &doc.assets,
            &SequenceEdit::PlaceAsset {
                asset_id: "mic".into(),
                at_us: 0,
                track_id: None,
                streams: vec![],
                range: None,
            },
        )
        .unwrap()
        .0;
        let (a1, a2) = (lane(&doc, 0), lane(&doc, 1));
        doc.audio.tracks.insert(
            a1.clone(),
            TrackMix {
                denoise_db: Some(18.0),
                ..Default::default()
            },
        );
        let lanes = lanes(dir.path(), &doc);
        let plan = PolishPlan::build(dir.path(), &doc.audio, &lanes, doc.duration_us()).unwrap();
        assert!(plan.denoiser(&a1, "assets/media/mic.wav").is_some());
        assert!(
            plan.denoiser(&a2, "assets/media/mic.wav").is_none(),
            "A2 has it off"
        );
    }

    /// The id of audio track `n` (0 is A1).
    fn lane(doc: &EditDocument, n: usize) -> String {
        doc.sequence.audio_tracks().nth(n).unwrap().id.clone()
    }

    /// Clips of the first track's sound at `(start, in, length)`.
    fn clips(doc: &mut EditDocument, parts: &[(u64, u64, u64)]) {
        let parts: Vec<_> = parts.iter().map(|&(s, i, d)| (s, i, d, 1.0)).collect();
        clips_at_speed(doc, &parts);
    }

    /// Clips of the first track's sound at `(start, in, length, speed)`.
    fn clips_at_speed(doc: &mut EditDocument, parts: &[(u64, u64, u64, f64)]) {
        let asset = doc.sequence.tracks[0].clips[0].asset.clone();
        doc.sequence.tracks[0].clips = parts
            .iter()
            .enumerate()
            .map(|(i, &(start_us, in_us, duration_us, speed))| Clip {
                id: format!("c{}", i + 100),
                asset: asset.clone(),
                stream: "sound0".into(),
                start_us,
                in_us,
                duration_us,
                link: None,
                fit: Fit::Contain,
                speed,
            })
            .collect();
    }

    /// Times a signal crosses zero upwards in `samples` (interleaved stereo, left channel).
    fn rising_crossings(samples: &[f64]) -> usize {
        samples
            .chunks_exact(2)
            .map(|f| f[0])
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count()
    }

    /// A clip at twice the speed plays its sound in half the time at the same pitch and level,
    /// reads the same however it is chunked, and split in two sounds as if it were whole.
    #[test]
    fn a_faster_clip_keeps_its_pitch_and_splits_without_a_seam() {
        let rate = 48_000usize;
        let values: Vec<i16> = (0..rate * 2)
            .map(|i| {
                (8000.0 * (std::f64::consts::TAU * 220.0 * i as f64 / rate as f64).sin()) as i16
            })
            .collect();
        let (dir, mut doc) = fixture(rate as u32, 1, values);
        clips_at_speed(&mut doc, &[(0, 0, 1_000_000, 2.0)]);
        let mixer = AudioMixer::new(dir.path(), &doc).unwrap();
        assert_eq!(mixer.total_frames, 48_000);
        let whole = mix_all(&mixer);
        let middle = seconds(&whole, 0.1, 0.9);
        // 220 Hz for 0.8 s: 176 cycles (it would be 352 if the pitch went up).
        let cycles = rising_crossings(middle);
        assert!((cycles as i64 - 176).abs() <= 2, "{cycles} cycles");
        let level = rms(middle) / (8000.0 / 32767.0 / 2f64.sqrt());
        assert!((level - 1.0).abs() < 0.1, "level {level}");
        let a = mixer.read_frames(20_000, 3_000).unwrap();
        let mut b = mixer.read_frames(20_000, 1_111).unwrap();
        b.extend(mixer.read_frames(21_111, 1_889).unwrap());
        assert_eq!(a, b);
        // Split at 0.4 s of the timeline (0.8 s of the sound).
        clips_at_speed(
            &mut doc,
            &[(0, 0, 400_000, 2.0), (400_000, 800_000, 600_000, 2.0)],
        );
        let split = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        assert_eq!(split, whole);
        // Half speed: twice as long, same pitch.
        clips_at_speed(&mut doc, &[(0, 0, 4_000_000, 0.5)]);
        let slow = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        assert_eq!(slow.len(), 4 * 96_000);
        let cycles = rising_crossings(seconds(&slow, 0.5, 3.5));
        assert!((cycles as i64 - 660).abs() <= 2, "{cycles} cycles");
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

    /// A muted moment of a sound is silent wherever it plays, fading at its edges; the rest
    /// plays as before.
    #[test]
    fn muted_moments_are_silent_and_fade() {
        let (dir, mut doc) = fixture(48_000, 1, vec![16384; 48_000]);
        let plain = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let key = crate::sequence::StreamRef::new("mic", "sound0").key();
        doc.noise_mutes.insert(
            key,
            vec![crate::sequence::SourceRange {
                start_us: 500_000,
                end_us: 520_000,
            }],
        );
        let muted = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let frame = |samples: &[f64], us: u64| samples[(us * 48 / 1000) as usize * 2];
        assert_eq!(frame(&muted, 510_000), 0.0);
        let edge = frame(&muted, 498_500);
        assert!(
            edge > 0.1 && edge < frame(&plain, 498_500),
            "fading: {edge}"
        );
        assert_eq!(frame(&muted, 490_000), frame(&plain, 490_000));
        assert_eq!(frame(&muted, 530_000), frame(&plain, 530_000));
    }

    /// A muted moment of a clip played at twice the speed is silent where it plays, half as
    /// far into the timeline.
    #[test]
    fn muted_moments_follow_speed() {
        let (dir, mut doc) = fixture(48_000, 1, vec![16384; 96_000]);
        clips_at_speed(&mut doc, &[(0, 0, 1_000_000, 2.0)]);
        let plain = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let key = crate::sequence::StreamRef::new("mic", "sound0").key();
        doc.noise_mutes.insert(
            key,
            vec![crate::sequence::SourceRange {
                start_us: 1_000_000,
                end_us: 1_100_000,
            }],
        );
        let muted = mix_all(&AudioMixer::new(dir.path(), &doc).unwrap());
        let frame = |samples: &[f64], us: u64| samples[(us * 48 / 1000) as usize * 2];
        assert!(frame(&muted, 525_000).abs() < 1e-3, "{}", frame(&muted, 525_000));
        assert!((frame(&muted, 400_000) - frame(&plain, 400_000)).abs() < 1e-3);
        assert!((frame(&muted, 650_000) - frame(&plain, 650_000)).abs() < 1e-3);
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

    /// Mixes a short's sound as playback does and lists its quiet stretches. Run on a copy of a
    /// project (its project.json, aeroedits.json and transcripts), never the original.
    #[test]
    #[ignore = "set AEROEDITS_SHORT_PROBE to a copied project folder"]
    fn short_sound_probe() {
        let root = std::path::PathBuf::from(std::env::var("AEROEDITS_SHORT_PROBE").unwrap());
        let reader = crate::project::ProjectReader::open(&root).unwrap();
        let base = reader.document();
        let short = base.shorts.iter().find(|s| s.take.is_some()).unwrap();
        let doc = crate::shorts::short_document(base, short, true).unwrap();
        println!(
            "PROBE short {} length {:.2}s",
            short.id,
            doc.duration_us() as f64 / 1e6
        );
        for track in &doc.sequence.tracks {
            println!(
                "PROBE track {} {:?} clips {}",
                track.id,
                track.kind,
                track.clips.len()
            );
        }
        let mixer = AudioMixer::new(reader.root(), &doc).unwrap();
        let mut frames = Vec::new();
        let mut at = 0u64;
        while at < mixer.total_frames {
            let chunk = mixer.read_frames(at, 4_800).unwrap();
            if chunk.is_empty() {
                break;
            }
            at += (chunk.len() / 2) as u64;
            frames.extend(chunk);
        }
        let window = (SAMPLE_RATE / 100) as usize;
        let mut quiet_from: Option<usize> = None;
        let level = |w: &[i16]| {
            let sum: f64 = w.iter().map(|&x| (x as f64 / 32768.0).powi(2)).sum();
            10.0 * (sum / w.len().max(1) as f64).log10()
        };
        let chunks: Vec<f64> = frames.chunks(window * 2).map(level).collect();
        let mut stretches = Vec::new();
        for (i, db) in chunks.iter().enumerate() {
            match (quiet_from, *db < -35.0) {
                (None, true) => quiet_from = Some(i),
                (Some(from), false) => {
                    if i - from >= 4 {
                        stretches.push((from, i));
                    }
                    quiet_from = None;
                }
                _ => {}
            }
        }
        for (a, b) in &stretches {
            println!(
                "PROBE quiet {:.2}-{:.2} ({} ms)",
                *a as f64 / 100.0,
                *b as f64 / 100.0,
                (b - a) * 10
            );
        }
        println!("PROBE quiet stretches over 120 ms: {}", stretches.len());
    }
}
