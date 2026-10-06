//! Rebuildable waveform cache and viewport queries over retained edited time.
use super::manifest::TrackType;
use super::pcm::{max_energy, PcmReader, WavInfo, READ_FRAME_CHUNK};
use super::reader::{is_safe_track_id, safe_path, SegmentSummary};
use super::RetainedInterval;
use crate::timeline::{SourceInterval, TimelineMapper};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const ANALYSIS_VERSION: u32 = 3;
pub const CACHE_BUCKET_US: u64 = 10_000;
pub const MAX_QUERY_BUCKETS: usize = 512;
pub const MAX_CACHE_BUCKETS: usize = 200_000;
pub const CHANNEL_POLICY: &str = "max_energy";
const CACHE_HEADER_SIZE: usize = 64;

#[derive(Clone, Debug)]
pub struct WaveformTrackContext {
    pub root: PathBuf,
    pub track_id: String,
    pub track_type: TrackType,
    pub segments: Vec<SegmentSummary>,
    pub retained: Vec<RetainedInterval>,
    pub edited_duration_us: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WaveformBucket {
    pub start_us: u64,
    pub end_us: u64,
    pub peak: f32,
    pub rms: f32,
    pub gap: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WaveformPage {
    pub track_id: String,
    pub start_us: u64,
    pub end_us: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub channel_policy: String,
    pub buckets: Vec<WaveformBucket>,
    pub diagnostics: Vec<String>,
    pub cancelled: bool,
}

#[derive(Clone, Debug)]
struct CachedSegment {
    source_start_us: u64,
    source_end_us: u64,
    sample_rate: u32,
    channels: u16,
    bucket_us: u64,
    peaks: Vec<f32>,
    rms: Vec<f32>,
}

/// A sound's whole waveform, a bar every [`OVERVIEW_BUCKET_US`] or so, packed small: the
/// timeline draws each clip's part of it, at any zoom, from this one answer.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WaveformOverview {
    pub track_id: String,
    /// The bars split `[0, duration_us)` of the sound's own time evenly.
    pub duration_us: u64,
    /// Each bar's peak and RMS level as `round(sqrt(level) * 254)`, which keeps detail in quiet
    /// sound; [`OVERVIEW_GAP`] where there is no sound to read (a missing file).
    pub peaks: Vec<u8>,
    pub rms: Vec<u8>,
    pub diagnostics: Vec<String>,
}

/// A bar with no sound behind it, in [`WaveformOverview`].
pub const OVERVIEW_GAP: u8 = 255;
/// Fine enough to see a pause between words.
pub const OVERVIEW_BUCKET_US: u64 = 20_000;
/// A two-hour sound still fits (at 36 ms a bar).
const MAX_OVERVIEW_BUCKETS: u64 = 200_000;

/// The whole of `ctx`'s sound as a [`WaveformOverview`], from the same per-segment summaries
/// (cached on disk) as [`query_waveform`].
pub fn waveform_overview(
    ctx: &WaveformTrackContext,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<WaveformOverview, String> {
    let duration_us = ctx.edited_duration_us;
    if duration_us == 0 {
        return Err("That sound is empty".into());
    }
    let bucket_us = OVERVIEW_BUCKET_US.max(duration_us.div_ceil(MAX_OVERVIEW_BUCKETS));
    let count = duration_us.div_ceil(bucket_us) as usize;
    let page = query_buckets(ctx, 0, duration_us, count, cancelled)?;
    let level = |v: f32| (v.clamp(0.0, 1.0).sqrt() * 254.0).round() as u8;
    let (peaks, rms) = page
        .buckets
        .iter()
        .map(|b| {
            if b.gap {
                (OVERVIEW_GAP, OVERVIEW_GAP)
            } else {
                (level(b.peak), level(b.rms))
            }
        })
        .unzip();
    Ok(WaveformOverview {
        track_id: page.track_id,
        duration_us: page.end_us,
        peaks,
        rms,
        diagnostics: page.diagnostics,
    })
}

pub fn query_waveform(
    ctx: &WaveformTrackContext,
    start_us: u64,
    end_us: u64,
    bucket_count: usize,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<WaveformPage, String> {
    if bucket_count == 0 || bucket_count > MAX_QUERY_BUCKETS {
        return Err("Waveform bucket count must be 1–512".into());
    }
    query_buckets(ctx, start_us, end_us, bucket_count, cancelled)
}

/// [`query_waveform`] for any number of buckets.
fn query_buckets(
    ctx: &WaveformTrackContext,
    start_us: u64,
    end_us: u64,
    bucket_count: usize,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<WaveformPage, String> {
    if !matches!(ctx.track_type, TrackType::MicAudio | TrackType::SystemAudio) {
        return Err("Track is not audio".into());
    }
    if start_us >= end_us {
        return Err("Waveform range must be a half-open interval".into());
    }
    if cancelled() {
        return Err("Waveform query cancelled".into());
    }

    let mapper = TimelineMapper::new(
        ctx.retained
            .iter()
            .enumerate()
            .map(|(i, interval)| {
                SourceInterval::new(format!("ret-{i}"), interval.start_us, interval.end_us)
                    .with_media(interval.media.clone())
            })
            .collect(),
    );
    let edited_end = mapper
        .total_edited_duration_us()
        .min(ctx.edited_duration_us);
    let query_start = start_us.min(edited_end);
    let query_end = end_us.min(edited_end).max(query_start);

    let mut diagnostics = Vec::new();
    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let width = query_end.saturating_sub(query_start);
    let count = bucket_count.min(width as usize);
    let mut buckets = Vec::with_capacity(count);
    let mut ranges = Vec::with_capacity(count);
    for i in 0..count {
        let a = query_start + (width as u128 * i as u128 / count as u128) as u64;
        let b = query_start + (width as u128 * (i + 1) as u128 / count as u128) as u64;
        buckets.push(WaveformBucket {
            start_us: a,
            end_us: b,
            peak: 0.0,
            rms: 0.0,
            gap: true,
        });
        ranges.push(edited_range_to_source(&mapper, a, b));
    }
    let visible = edited_range_to_source(&mapper, query_start, query_end);
    let mut energies = vec![0.0f64; count];
    let mut weights = vec![0u64; count];
    // The visible segments' summaries, read or built side by side: a long recording is many
    // segment files, and building each reads the whole file. Summaries are small (a bucket per
    // 10 ms or more), so holding them all is fine.
    let wanted: Vec<&SegmentSummary> = ctx
        .segments
        .iter()
        .filter(|segment| {
            visible
                .iter()
                .any(|(a, b)| *a < segment.end_us && segment.start_us < *b)
        })
        .collect();
    let built = build_segments(ctx, &wanted, cancelled);
    for (segment, result) in wanted.iter().copied().zip(built) {
        if cancelled() {
            return Err("Waveform query cancelled".into());
        }
        match result {
            Ok(Some(cache)) => {
                if sample_rate == 0 {
                    sample_rate = cache.sample_rate;
                    channels = cache.channels;
                }
                for (i, spans) in ranges.iter().enumerate() {
                    for &(a, b) in spans {
                        if b <= cache.source_start_us || a >= cache.source_end_us {
                            continue;
                        }
                        if let Some((peak, rms, weight)) =
                            lookup_source_range(std::slice::from_ref(&cache), a, b)
                        {
                            buckets[i].gap = false;
                            buckets[i].peak = buckets[i].peak.max(peak);
                            energies[i] += (rms as f64).powi(2) * weight as f64;
                            weights[i] += weight;
                        }
                    }
                }
            }
            Ok(None) => {
                if diagnostics.len() < 32 {
                    diagnostics.push(format!(
                        "Missing or unreadable audio treated as a gap: {}",
                        segment.relative_path
                    ));
                }
            }
            Err(error) => {
                if cancelled() {
                    return Err("Waveform query cancelled".into());
                }
                if diagnostics.len() < 32 {
                    diagnostics.push(error);
                }
            }
        }
    }
    if cancelled() {
        return Err("Waveform query cancelled".into());
    }
    for i in 0..count {
        if weights[i] > 0 {
            buckets[i].rms = (energies[i] / weights[i] as f64).sqrt() as f32;
        }
    }

    Ok(WaveformPage {
        track_id: ctx.track_id.clone(),
        start_us: query_start,
        end_us: query_end,
        sample_rate,
        channels,
        channel_policy: CHANNEL_POLICY.into(),
        buckets,
        diagnostics,
        cancelled: false,
    })
}

/// The source ranges playing over `[start_us, end_us)` of the clock, gaps left out.
fn edited_range_to_source(mapper: &TimelineMapper, start_us: u64, end_us: u64) -> Vec<(u64, u64)> {
    mapper.edited_range_to_source(start_us, end_us)
}

fn lookup_source_range(
    caches: &[CachedSegment],
    start_us: u64,
    end_us: u64,
) -> Option<(f32, f32, u64)> {
    let mut peak = 0.0f32;
    let mut energy = 0.0f32;
    let mut n = 0u64;
    for cache in caches {
        let a = start_us.max(cache.source_start_us);
        let b = end_us.min(cache.source_end_us);
        if a >= b {
            continue;
        }
        let bucket_us = cache.bucket_us.max(1);
        let first = ((a - cache.source_start_us) / bucket_us) as usize;
        let last = ((b - 1 - cache.source_start_us) / bucket_us) as usize;
        for idx in first..=last.min(cache.peaks.len().saturating_sub(1)) {
            peak = peak.max(cache.peaks[idx]);
            let bucket_start = cache.source_start_us + idx as u64 * bucket_us;
            let weight = b.min(bucket_start + bucket_us) - a.max(bucket_start);
            energy += cache.rms[idx] * cache.rms[idx] * weight as f32;
            n += weight;
        }
    }
    if n == 0 {
        None
    } else {
        Some((peak, (energy / n as f32).sqrt(), n))
    }
}

/// [`load_or_build_segment`] for each of `segments`, a few at a time, in order.
fn build_segments(
    ctx: &WaveformTrackContext,
    segments: &[&SegmentSummary],
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Vec<Result<Option<CachedSegment>, String>> {
    let workers = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(1, 8)
        .min(segments.len().max(1));
    if workers <= 1 {
        return segments
            .iter()
            .map(|segment| load_or_build_segment(ctx, segment, cancelled))
            .collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<Result<Option<CachedSegment>, String>>>> = segments
        .iter()
        .map(|_| std::sync::Mutex::new(None))
        .collect();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(segment) = segments.get(i) else {
                    break;
                };
                let result = load_or_build_segment(ctx, segment, cancelled);
                *results[i].lock().unwrap_or_else(|e| e.into_inner()) = Some(result);
            });
        }
    });
    results
        .into_iter()
        .map(|slot| {
            slot.into_inner()
                .unwrap_or_else(|e| e.into_inner())
                .unwrap_or_else(|| Err("Waveform query cancelled".into()))
        })
        .collect()
}

fn load_or_build_segment(
    ctx: &WaveformTrackContext,
    segment: &SegmentSummary,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<Option<CachedSegment>, String> {
    if !segment.available {
        return Ok(None);
    }
    let path = safe_path(&ctx.root, &segment.relative_path)?;
    if !path.is_file() {
        return Ok(None);
    }
    let file_len = fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if file_len != segment.size_bytes {
        return Err(format!(
            "Size-mismatched audio treated as a gap: {}",
            segment.relative_path
        ));
    }
    let info = super::pcm::parse_wav(&path)?;
    let mut effective = segment.clone();
    effective.end_us = effective.end_us.min(
        effective
            .start_us
            .saturating_add(info.frame_us(info.frame_count)),
    );
    if effective.end_us <= effective.start_us {
        return Ok(None);
    }
    let segment = &effective;
    let fingerprint = wav_fingerprint(&path, &info, file_len, cancelled)?;
    let cache_path = cache_path(&ctx.root, &ctx.track_id, segment, file_len, fingerprint)?;
    if let Some(cached) = read_cache(&cache_path, segment, file_len, fingerprint) {
        return Ok(Some(cached));
    }
    let built = analyze_segment(segment, &path, cancelled)?;
    let _ = write_cache(&cache_path, segment, file_len, fingerprint, &built);
    Ok(Some(built))
}

fn analyze_segment(
    segment: &SegmentSummary,
    path: &Path,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<CachedSegment, String> {
    let mut reader = PcmReader::open(path)?;
    let info = reader.info().clone();
    let bucket_us = bucket_us_for(&info, segment);
    let bucket_count = bucket_count_for(segment, bucket_us);
    if bucket_count == 0 {
        return Err(format!("Audio segment is empty: {}", segment.relative_path));
    }
    let mut peaks = vec![0.0f32; bucket_count];
    let mut rms = vec![0.0f32; bucket_count];
    let mut counts = vec![0u32; bucket_count];
    let channels = info.channels as usize;
    let mut channel_peak = vec![0.0f32; bucket_count * channels];
    let mut channel_sum_sq = vec![0.0f32; bucket_count * channels];
    // Frame `f` is at `floor(f * 1e6 / rate)` us into the segment, so bucket `i` starts at the
    // first frame with `f * 1e6 / rate >= i * bucket_us`. Working out where each bucket (and
    // the segment) ends once, rather than dividing for every sample, keeps this loop cheap.
    let rate = info.sample_rate.max(1) as u128;
    let first_frame_at = |us: u64| (us as u128 * rate).div_ceil(1_000_000) as u64;
    let end_frame = first_frame_at(segment.end_us.saturating_sub(segment.start_us));
    let mut idx = 0usize;
    let mut next_bucket = first_frame_at(bucket_us);
    let mut frame_index = 0u64;
    let mut interleaved = vec![0.0f32; READ_FRAME_CHUNK * channels];
    'read: loop {
        if cancelled() {
            return Err("Waveform query cancelled".into());
        }
        let frames = reader.read_frames(&mut interleaved, READ_FRAME_CHUNK)?;
        if frames == 0 {
            break;
        }
        let mut frame = 0usize;
        while frame < frames {
            if frame_index >= end_frame || idx >= bucket_count {
                break 'read;
            }
            // The frames of this chunk that fall in bucket `idx`.
            let run = ((next_bucket.min(end_frame) - frame_index) as usize).min(frames - frame);
            let acc = idx * channels;
            for f in frame..frame + run {
                let base = f * channels;
                for ch in 0..channels {
                    let x = interleaved[base + ch];
                    channel_peak[acc + ch] = channel_peak[acc + ch].max(x.abs());
                    channel_sum_sq[acc + ch] += x * x;
                }
            }
            counts[idx] += run as u32;
            frame += run;
            frame_index += run as u64;
            if frame_index >= next_bucket {
                idx += 1;
                next_bucket = first_frame_at(bucket_us * (idx as u64 + 1));
            }
        }
    }
    for i in 0..bucket_count {
        if counts[i] == 0 {
            continue;
        }
        let n = counts[i] as f32;
        let acc = i * channels;
        let mut per = Vec::with_capacity(channels);
        for ch in 0..channels {
            per.push((
                channel_peak[acc + ch],
                (channel_sum_sq[acc + ch] / n).sqrt(),
            ));
        }
        let (p, r) = max_energy(&per);
        peaks[i] = p;
        rms[i] = r;
    }
    Ok(CachedSegment {
        source_start_us: segment.start_us,
        source_end_us: segment.end_us,
        sample_rate: info.sample_rate,
        channels: info.channels,
        bucket_us,
        peaks,
        rms,
    })
}

fn bucket_us_for(info: &WavInfo, segment: &SegmentSummary) -> u64 {
    let duration = segment.end_us.saturating_sub(segment.start_us).max(1);
    let mut bucket_us = CACHE_BUCKET_US;
    while duration / bucket_us > MAX_CACHE_BUCKETS as u64 {
        bucket_us = bucket_us.saturating_mul(2);
    }
    let frames = info.frames_for_us(bucket_us).max(1);
    (frames as u128 * 1_000_000 / info.sample_rate.max(1) as u128) as u64
}

fn bucket_count_for(segment: &SegmentSummary, bucket_us: u64) -> usize {
    let duration = segment.end_us.saturating_sub(segment.start_us);
    ((duration + bucket_us - 1) / bucket_us).min(MAX_CACHE_BUCKETS as u64) as usize
}

fn cache_path(
    root: &Path,
    track_id: &str,
    segment: &SegmentSummary,
    file_len: u64,
    fingerprint: u64,
) -> Result<PathBuf, String> {
    if !is_safe_track_id(track_id) {
        return Err("Invalid track ID".into());
    }
    let key = format!(
        "v{ANALYSIS_VERSION}:{CHANNEL_POLICY}:{}:{}:{}:{}:{}:{}",
        segment.relative_path,
        file_len,
        fingerprint,
        segment.start_us,
        segment.end_us,
        CACHE_BUCKET_US
    );
    let name = format!("{:016x}.wf1", fnv1a64(key.as_bytes()));
    let relative = format!("cache/waveforms/{track_id}/{name}");
    safe_path(root, &relative)
}

/// A hash of the whole file, so a cache entry is used only for the bytes it was built from.
/// Remembered while the file's size and modification time stay the same: a cached waveform of
/// a long recording then loads without reading the recording again.
fn wav_fingerprint(
    path: &Path,
    info: &WavInfo,
    file_len: u64,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<u64, String> {
    use std::sync::{Mutex, OnceLock};
    type Key = (std::path::PathBuf, u64, Option<std::time::SystemTime>);
    static KNOWN: OnceLock<Mutex<std::collections::HashMap<Key, u64>>> = OnceLock::new();
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let key: Key = (path.to_path_buf(), file_len, modified);
    let known = KNOWN.get_or_init(Default::default);
    if let Some(hash) = known.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return Ok(*hash);
    }
    let hash = hash_wav(path, info, file_len, cancelled)?;
    // A file written within the last moments may still change at the same size and time.
    let settled = modified
        .and_then(|m| m.elapsed().ok())
        .is_some_and(|age| age > std::time::Duration::from_secs(2));
    if settled {
        let mut known = known.lock().unwrap_or_else(|e| e.into_inner());
        if known.len() >= 256 {
            known.clear();
        }
        known.insert(key, hash);
    }
    Ok(hash)
}

fn hash_wav(
    path: &Path,
    _info: &WavInfo,
    file_len: u64,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<u64, String> {
    let mut file = super::reader::open_regular(path)?;
    let mut hash = fnv1a64(&file_len.to_le_bytes());
    let mut buf = [0u8; 65_536];
    let mut remaining = file_len;
    while remaining > 0 {
        if cancelled() {
            return Err("Waveform query cancelled".into());
        }
        let wanted = remaining.min(buf.len() as u64) as usize;
        file.read_exact(&mut buf[..wanted])
            .map_err(|e| e.to_string())?;
        hash = hash_words(hash, &buf[..wanted]);
        remaining -= wanted as u64;
    }
    Ok(hash)
}

/// Mixes `bytes` into `hash` eight at a time: a long recording is hundreds of MB, and a byte
/// at a time took a quarter of a cold waveform load. Only tells content apart, nothing more.
fn hash_words(mut hash: u64, bytes: &[u8]) -> u64 {
    for chunk in bytes.chunks(8) {
        let mut word = [0u8; 8];
        word[..chunk.len()].copy_from_slice(chunk);
        hash = (hash ^ u64::from_le_bytes(word)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        hash ^= hash >> 29;
    }
    hash
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    fnv1a64_continue(0xcbf29ce484222325u64, bytes)
}

fn fnv1a64_continue(mut hash: u64, bytes: &[u8]) -> u64 {
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn read_cache(
    path: &Path,
    segment: &SegmentSummary,
    file_len: u64,
    fingerprint: u64,
) -> Option<CachedSegment> {
    let mut file = super::reader::open_regular(path).ok()?;
    let mut header = [0u8; CACHE_HEADER_SIZE];
    file.read_exact(&mut header).ok()?;
    if &header[0..4] != b"ASWF" {
        return None;
    }
    let version = u32::from_le_bytes(header[4..8].try_into().ok()?);
    let bucket_us = u64::from_le_bytes(header[8..16].try_into().ok()?);
    let sample_rate = u32::from_le_bytes(header[16..20].try_into().ok()?);
    let channels = u16::from_le_bytes(header[20..22].try_into().ok()?);
    let stored_len = u64::from_le_bytes(header[24..32].try_into().ok()?);
    let start_us = u64::from_le_bytes(header[32..40].try_into().ok()?);
    let stored_fingerprint = u64::from_le_bytes(header[40..48].try_into().ok()?);
    let count = u32::from_le_bytes(header[48..52].try_into().ok()?) as usize;
    if version != ANALYSIS_VERSION
        || stored_len != file_len
        || stored_fingerprint != fingerprint
        || start_us != segment.start_us
        || count == 0
        || count > MAX_CACHE_BUCKETS
        || bucket_us == 0
        || count != bucket_count_for(segment, bucket_us)
        || !(super::pcm::MIN_SAMPLE_RATE..=super::pcm::MAX_SAMPLE_RATE).contains(&sample_rate)
        || !(1..=super::pcm::MAX_CHANNELS).contains(&channels)
        || file.metadata().ok()?.len() != (CACHE_HEADER_SIZE + count * 8) as u64
    {
        return None;
    }
    let mut body = vec![0u8; count * 8];
    file.read_exact(&mut body).ok()?;
    let checksum = u64::from_le_bytes(header[56..64].try_into().ok()?);
    if checksum != fnv1a64_continue(fnv1a64(&header[..56]), &body) {
        return None;
    }
    let mut peaks = Vec::with_capacity(count);
    let mut rms = Vec::with_capacity(count);
    for i in 0..count {
        let o = i * 8;
        peaks.push(f32::from_le_bytes(body[o..o + 4].try_into().ok()?));
        rms.push(f32::from_le_bytes(body[o + 4..o + 8].try_into().ok()?));
    }
    if peaks
        .iter()
        .chain(rms.iter())
        .any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0)
    {
        return None;
    }
    Some(CachedSegment {
        source_start_us: segment.start_us,
        source_end_us: segment.end_us,
        sample_rate,
        channels,
        bucket_us,
        peaks,
        rms,
    })
}

fn write_cache(
    path: &Path,
    segment: &SegmentSummary,
    file_len: u64,
    fingerprint: u64,
    cache: &CachedSegment,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        if fs::symlink_metadata(parent)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("Cache directory must not be a symlink".into());
        }
    }
    let parent = path.parent().ok_or("Cache path has no parent")?;
    let mut header = [0u8; CACHE_HEADER_SIZE];
    header[0..4].copy_from_slice(b"ASWF");
    header[4..8].copy_from_slice(&ANALYSIS_VERSION.to_le_bytes());
    header[8..16].copy_from_slice(&cache.bucket_us.to_le_bytes());
    header[16..20].copy_from_slice(&cache.sample_rate.to_le_bytes());
    header[20..22].copy_from_slice(&cache.channels.to_le_bytes());
    header[24..32].copy_from_slice(&file_len.to_le_bytes());
    header[32..40].copy_from_slice(&segment.start_us.to_le_bytes());
    header[40..48].copy_from_slice(&fingerprint.to_le_bytes());
    header[48..52].copy_from_slice(&(cache.peaks.len() as u32).to_le_bytes());
    let mut body = Vec::with_capacity(CACHE_HEADER_SIZE + cache.peaks.len() * 8);
    body.extend_from_slice(&header);
    for i in 0..cache.peaks.len() {
        body.extend_from_slice(&cache.peaks[i].to_le_bytes());
        body.extend_from_slice(&cache.rms[i].to_le_bytes());
    }
    let checksum = fnv1a64_continue(fnv1a64(&header[..56]), &body[CACHE_HEADER_SIZE..]);
    body[56..64].copy_from_slice(&checksum.to_le_bytes());
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(&body).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {

    /// Buckets hold exactly the frames the per-sample formula puts in them, at rates that do
    /// not divide a microsecond evenly and with an end that is not on a frame.
    #[test]
    fn buckets_match_the_per_sample_formula() {
        for (rate, channels) in [(44_100u32, 2u16), (48_000, 1), (22_050, 2)] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("a.wav");
            let frames = rate as usize * 3 + 123;
            let values: Vec<i16> = (0..frames * channels as usize)
                .map(|i| ((i * 7919) % 20_000) as i16 - 10_000)
                .collect();
            std::fs::write(
                &path,
                crate::fixtures::generate_pcm16_wav(rate, channels, &values),
            )
            .unwrap();
            let segment = SegmentSummary {
                track_id: "mic".into(),
                relative_path: "a.wav".into(),
                start_us: 1_000_000,
                end_us: 3_712_345,
                size_bytes: 0,
                media_timescale: rate,
                media_start_value: 0,
                host_anchor_us: 0,
                is_keyframe_start: Some(true),
                available: true,
            };
            let built = analyze_segment(&segment, &path, &|| false).unwrap();
            // The per-sample formula, as the loop used to work it out.
            let info = super::super::pcm::parse_wav(&path).unwrap();
            let mut peaks = vec![0.0f32; built.peaks.len()];
            for f in 0..frames as u64 {
                let local = info.frame_us(f);
                if segment.start_us + local >= segment.end_us {
                    break;
                }
                let idx = (local / built.bucket_us) as usize;
                if idx < peaks.len() {
                    for ch in 0..channels as usize {
                        let x = values[f as usize * channels as usize + ch] as f32 / 32768.0;
                        peaks[idx] = peaks[idx].max(x.abs());
                    }
                }
            }
            for (i, (a, b)) in built.peaks.iter().zip(&peaks).enumerate() {
                // Multi-channel buckets report the loudest channel's peak: never above these.
                assert!(*a <= *b + 1e-6, "rate {rate} bucket {i}: {a} > {b}");
                if channels == 1 {
                    assert_eq!(a, b, "rate {rate} bucket {i}");
                }
            }
            assert_eq!(
                built.peaks.iter().filter(|p| **p > 0.0).count(),
                peaks.iter().filter(|p| **p > 0.0).count(),
                "rate {rate}: the same buckets have sound"
            );
        }
    }

    use super::*;
    use crate::fixtures::generate_pcm16_wav;
    use crate::fixtures::TestProject;
    use crate::project::manifest::{TrackDescriptor, TrackType};
    use crate::project::JournalRecord;
    use std::sync::atomic::Ordering;
    use tempfile::tempdir;

    fn audio_bundle(
        samples: &[i16],
        channels: u16,
        start_us: u64,
        sample_rate: u32,
    ) -> (tempfile::TempDir, PathBuf, String) {
        let dir = tempdir().unwrap();
        let mut bundle = TestProject::create(dir.path(), "wf");
        let wav = generate_pcm16_wav(sample_rate, channels, samples);
        let relative = "media/mic/000001.wav";
        fs::write(bundle.root_path().join(relative), &wav).unwrap();
        let frames = samples.len() as u64 / channels as u64;
        let duration_us = (frames * 1_000_000) / sample_rate as u64;
        let end_us = start_us + duration_us;
        bundle.manifest_mut().tracks.push(TrackDescriptor {
            id: "mic".into(),
            track_type: TrackType::MicAudio,
            codec: "pcm".into(),
            relative_path: relative.into(),
            width: None,
            height: None,
            fps: None,
            sample_rate: Some(sample_rate),
            channels: Some(channels),
            gaps_total: 0,
            media_timescale: Some(sample_rate),
        });
        bundle.append_journal(JournalRecord::SegmentCommitted {
            seq: 0,
            track_id: "mic".into(),
            relative_path: relative.into(),
            start_us,
            end_us,
            size_bytes: wav.len() as u64,
            is_keyframe_start: true,
            media_timescale: sample_rate,
            media_start_value: 0,
            host_anchor_us: start_us as i64,
        });
        bundle.manifest_mut().duration_us = end_us;
        bundle.manifest_mut().active_duration_us = end_us;
        bundle.save_manifest();
        let root = bundle.root_path().to_path_buf();
        drop(bundle);
        (dir, root, relative.into())
    }

    fn ctx_from(
        root: PathBuf,
        start_us: u64,
        end_us: u64,
        available: bool,
        size: u64,
    ) -> WaveformTrackContext {
        WaveformTrackContext {
            root,
            track_id: "mic".into(),
            track_type: TrackType::MicAudio,
            segments: vec![SegmentSummary {
                track_id: "mic".into(),
                relative_path: "media/mic/000001.wav".into(),
                start_us,
                end_us,
                size_bytes: size,
                media_timescale: 48_000,
                media_start_value: 0,
                host_anchor_us: start_us as i64,
                is_keyframe_start: Some(true),
                available,
            }],
            retained: vec![RetainedInterval {
                start_us: 0,
                end_us,
                media: None,
            }],
            edited_duration_us: end_us,
        }
    }

    /// The overview has a bar every 20 ms over the whole sound, packed as sqrt levels: a
    /// pause between two loud stretches reads as near-silent bars.
    #[test]
    fn overview_shows_pauses_bar_by_bar() {
        // 0.3 s loud (half scale), 0.2 s silent, 0.3 s at a quarter.
        let rate = 48_000usize;
        let samples: Vec<i16> = (0..rate * 8 / 10)
            .map(|i| match i * 10 / rate {
                0..=2 => {
                    if i % 2 == 0 {
                        16_384
                    } else {
                        -16_384
                    }
                }
                3..=4 => 0,
                _ => {
                    if i % 2 == 0 {
                        8_192
                    } else {
                        -8_192
                    }
                }
            })
            .collect();
        let (dir, root, _) = audio_bundle(&samples, 1, 0, 48_000);
        let size = fs::metadata(root.join("media/mic/000001.wav"))
            .unwrap()
            .len();
        let overview =
            waveform_overview(&ctx_from(root, 0, 800_000, true, size), &|| false).unwrap();
        assert_eq!(overview.duration_us, 800_000);
        assert_eq!(overview.peaks.len(), 40, "a bar every 20 ms");
        let level = |v: f32| (v.sqrt() * 254.0).round() as u8;
        assert!(overview.peaks[..15].iter().all(|&p| p == level(0.5)));
        assert!(overview.peaks[15..25].iter().all(|&p| p == 0), "the pause");
        assert!(overview.peaks[25..].iter().all(|&p| p == level(0.25)));
        assert_eq!(
            overview.rms[0],
            level(0.5),
            "a square wave's RMS is its peak"
        );
        drop(dir);
        // A missing file is no sound at all, not silence.
        let overview = waveform_overview(
            &ctx_from(PathBuf::from("nowhere"), 0, 800_000, false, 0),
            &|| false,
        )
        .unwrap();
        assert!(overview.peaks.iter().all(|&p| p == OVERVIEW_GAP));
    }

    #[test]
    fn zero_constant_and_opposite_stereo_from_real_wav() {
        let n = 4_800usize;
        let zeros = vec![0i16; n];
        let (dir, root, _) = audio_bundle(&zeros, 1, 0, 48_000);
        let size = fs::metadata(root.join("media/mic/000001.wav"))
            .unwrap()
            .len();
        let page = query_waveform(
            &ctx_from(root, 0, 100_000, true, size),
            0,
            100_000,
            10,
            &|| false,
        )
        .unwrap();
        assert!(page
            .buckets
            .iter()
            .all(|b| !b.gap && b.peak == 0.0 && b.rms == 0.0));
        drop(dir);

        let constant = vec![16384i16; n];
        let (dir, root, _) = audio_bundle(&constant, 1, 0, 48_000);
        let size = fs::metadata(root.join("media/mic/000001.wav"))
            .unwrap()
            .len();
        let page = query_waveform(
            &ctx_from(root, 0, 100_000, true, size),
            0,
            100_000,
            10,
            &|| false,
        )
        .unwrap();
        assert!(page
            .buckets
            .iter()
            .all(|b| !b.gap && (b.peak - 0.5).abs() < 0.02 && (b.rms - 0.5).abs() < 0.02));
        drop(dir);

        let mut stereo = Vec::with_capacity(n * 2);
        for _ in 0..n {
            stereo.push(16384);
            stereo.push(-16384);
        }
        let (dir, root, _) = audio_bundle(&stereo, 2, 0, 48_000);
        let size = fs::metadata(root.join("media/mic/000001.wav"))
            .unwrap()
            .len();
        let page = query_waveform(
            &ctx_from(root, 0, 100_000, true, size),
            0,
            100_000,
            10,
            &|| false,
        )
        .unwrap();
        assert_eq!(page.channels, 2);
        assert_eq!(page.channel_policy, "max_energy");
        assert!(page
            .buckets
            .iter()
            .all(|b| !b.gap && (b.peak - 0.5).abs() < 0.02 && (b.rms - 0.5).abs() < 0.02));
        drop(dir);
    }

    #[test]
    fn late_segment_and_missing_file_are_gaps() {
        let samples = vec![16384i16; 4_800];
        let (dir, root, _) = audio_bundle(&samples, 1, 2_000_000, 48_000);
        let size = fs::metadata(root.join("media/mic/000001.wav"))
            .unwrap()
            .len();
        let mut ctx = ctx_from(root.clone(), 2_000_000, 2_100_000, true, size);
        ctx.retained = vec![RetainedInterval {
            start_us: 0,
            end_us: 2_100_000,
            media: None,
        }];
        ctx.edited_duration_us = 2_100_000;
        let page = query_waveform(&ctx, 0, 2_100_000, 21, &|| false).unwrap();
        let early = &page.buckets[0];
        let late = &page.buckets[20];
        assert!(early.gap && early.peak == 0.0);
        assert!(!late.gap && (late.peak - 0.5).abs() < 0.05);
        drop(dir);

        let (dir, root, _) = audio_bundle(&samples, 1, 0, 48_000);
        fs::remove_file(root.join("media/mic/000001.wav")).unwrap();
        let page = query_waveform(
            &ctx_from(root, 0, 100_000, true, 12),
            0,
            100_000,
            4,
            &|| false,
        )
        .unwrap();
        assert!(page.buckets.iter().all(|b| b.gap));
        drop(dir);
    }

    #[test]
    fn unsupported_encoding_is_reported_as_a_gap() {
        let samples = vec![16384i16; 4_800];
        let (dir, root, _) = audio_bundle(&samples, 1, 0, 48_000);
        let path = root.join("media/mic/000001.wav");
        let mut bad = fs::read(&path).unwrap();
        bad[20] = 7;
        bad[21] = 0;
        fs::write(&path, &bad).unwrap();
        let size = bad.len() as u64;
        let page = query_waveform(
            &ctx_from(root, 0, 100_000, true, size),
            0,
            100_000,
            8,
            &|| false,
        )
        .unwrap();
        assert!(page.buckets.iter().all(|b| b.gap && b.peak == 0.0));
        assert!(page
            .diagnostics
            .iter()
            .any(|d| d.to_lowercase().contains("unsupported") || d.contains("encoding")));
        drop(dir);
    }

    #[test]
    fn cache_recomputes_when_stale_and_cancel_stops() {
        let samples = vec![16384i16; 4_800];
        let (dir, root, _) = audio_bundle(&samples, 1, 0, 48_000);
        let wav_path = root.join("media/mic/000001.wav");
        let size = fs::metadata(&wav_path).unwrap().len();
        let ctx = ctx_from(root.clone(), 0, 100_000, true, size);
        query_waveform(&ctx, 0, 100_000, 8, &|| false).unwrap();
        let cache_dir = root.join("cache/waveforms/mic");
        assert!(cache_dir.exists());
        let first: Vec<_> = fs::read_dir(&cache_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(first.len(), 1);
        let original = fs::read(&first[0]).unwrap();

        let zeros = generate_pcm16_wav(48_000, 1, &vec![0i16; 4_800]);
        fs::write(&wav_path, &zeros).unwrap();
        let ctx = ctx_from(root.clone(), 0, 100_000, true, zeros.len() as u64);
        let page = query_waveform(&ctx, 0, 100_000, 8, &|| false).unwrap();
        assert!(
            page.buckets
                .iter()
                .all(|b| !b.gap && b.peak == 0.0 && b.rms == 0.0),
            "same-size PCM rewrite must miss the stale 0.5 cache, got {:?}",
            page.buckets.iter().map(|b| b.peak).collect::<Vec<_>>()
        );
        let second: Vec<_> = fs::read_dir(&cache_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("wf1"))
            .collect();
        assert!(
            second.len() >= 2,
            "content change must write a distinct cache file"
        );
        assert!(second
            .iter()
            .any(|p| fs::read(p).ok().as_deref() != Some(original.as_slice())));

        for entry in fs::read_dir(&cache_dir).unwrap() {
            let path = entry.unwrap().path();
            let _ = fs::remove_file(path);
        }
        let checks = std::sync::atomic::AtomicUsize::new(0);
        let err = query_waveform(&ctx, 0, 100_000, 8, &|| {
            // Allow the query/segment entry checks, then cancel inside PCM analysis.
            checks.fetch_add(1, Ordering::SeqCst) >= 2
        })
        .unwrap_err();
        assert!(err.contains("cancelled"));
        drop(dir);
    }

    #[test]
    fn interior_rewrite_invalidates_and_corrupt_cache_rebuilds() {
        let (_dir, root, _) = audio_bundle(&vec![0; 48_000], 1, 0, 48_000);
        let path = root.join("media/mic/000001.wav");
        let size = fs::metadata(&path).unwrap().len();
        let ctx = ctx_from(root.clone(), 0, 1_000_000, true, size);
        query_waveform(&ctx, 0, 1_000_000, 100, &|| false).unwrap();
        let mut bytes = fs::read(&path).unwrap();
        for i in 4_800..5_280 {
            bytes[44 + i * 2..46 + i * 2].copy_from_slice(&16384i16.to_le_bytes());
        }
        fs::write(&path, &bytes).unwrap();
        let page = query_waveform(&ctx, 0, 1_000_000, 100, &|| false).unwrap();
        assert!(page.buckets[10].peak > 0.49);
        for entry in fs::read_dir(root.join("cache/waveforms/mic")).unwrap() {
            let path = entry.unwrap().path();
            let mut bytes = fs::read(&path).unwrap();
            bytes[CACHE_HEADER_SIZE..].fill(0);
            fs::write(path, bytes).unwrap();
        }
        let page = query_waveform(&ctx, 0, 1_000_000, 100, &|| false).unwrap();
        assert!(page.buckets[10].peak > 0.49);
    }

    #[cfg(unix)]
    #[test]
    fn temporary_symlink_does_not_overwrite_and_fifo_cache_does_not_block() {
        let (_dir, root, _) = audio_bundle(&vec![16384; 4_800], 1, 0, 48_000);
        let size = fs::metadata(root.join("media/mic/000001.wav"))
            .unwrap()
            .len();
        let ctx = ctx_from(root.clone(), 0, 100_000, true, size);
        query_waveform(&ctx, 0, 100_000, 10, &|| false).unwrap();
        let cache = fs::read_dir(root.join("cache/waveforms/mic"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::remove_file(&cache).unwrap();
        let victim = root.join("preserve.json");
        fs::write(&victim, b"keep me").unwrap();
        std::os::unix::fs::symlink(&victim, cache.with_extension("tmp")).unwrap();
        let fifo = std::ffi::CString::new(cache.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        let page = query_waveform(&ctx, 0, 100_000, 10, &|| false).unwrap();
        assert!(page.buckets.iter().all(|b| !b.gap && b.peak > 0.49));
        assert_eq!(fs::read(victim).unwrap(), b"keep me");
    }

    #[test]
    fn viewport_does_not_open_unrelated_segments() {
        let (_dir, root, _) = audio_bundle(&vec![16384; 4_800], 1, 0, 48_000);
        let size = fs::metadata(root.join("media/mic/000001.wav"))
            .unwrap()
            .len();
        let mut ctx = ctx_from(root, 0, 100_000, true, size);
        for i in 1..10_000 {
            let mut segment = ctx.segments[0].clone();
            segment.start_us = i * 100_000;
            segment.end_us = segment.start_us + 100_000;
            segment.relative_path = "../must-not-read".into();
            ctx.segments.push(segment);
        }
        ctx.retained[0].end_us = 1_000_000_000;
        ctx.edited_duration_us = 1_000_000_000;
        let page = query_waveform(&ctx, 0, 100_000, 10, &|| false).unwrap();
        assert!(page.diagnostics.is_empty());
        assert!(page.buckets.iter().all(|b| !b.gap && b.peak > 0.49));
    }

    #[test]
    fn same_size_zero_and_constant_pcm_have_distinct_fingerprints() {
        let dir = tempdir().unwrap();
        let zero_path = dir.path().join("z.wav");
        let const_path = dir.path().join("c.wav");
        let zeros = generate_pcm16_wav(48_000, 1, &vec![0i16; 4_800]);
        let constant = generate_pcm16_wav(48_000, 1, &vec![16384i16; 4_800]);
        fs::write(&zero_path, &zeros).unwrap();
        fs::write(&const_path, &constant).unwrap();
        assert_eq!(zeros.len(), constant.len());
        let z = crate::project::pcm::parse_wav(&zero_path).unwrap();
        let c = crate::project::pcm::parse_wav(&const_path).unwrap();
        let zf = wav_fingerprint(&zero_path, &z, zeros.len() as u64, &|| false).unwrap();
        let cf = wav_fingerprint(&const_path, &c, constant.len() as u64, &|| false).unwrap();
        assert_ne!(zf, cf);
    }
}
