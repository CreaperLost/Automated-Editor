//! Shorts made from a take: a new recording (a camera video, which may carry its own
//! microphone, a separate microphone file, or an AeroEdits recording) that tells one part of
//! the video again. The take is the short's spine: its voice is the sound and the captions,
//! its camera the camera part. Footage from the video (or any media) plays under its words on
//! a screen track; where none plays, the camera fills the frame.
//!
//! The short's own timeline holds, bottom to top: the Footage track (screen), the Camera track
//! (webcam); then the Voice track (speech) and the Footage sound track (background, ducked
//! under the voice).

use crate::project::revision::EditDocument;
use crate::sequence::{
    clock, Asset, Clip, Role, Sequence, StreamKind, StreamRef, Track, TrackKind,
};
use crate::short_scenes::{Scene, SceneMode};
use crate::shorts::{Short, ShortEdit, ShortLayout, ShortTake};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// How finely the take's two sounds are compared to line them up: 5 ms steps.
const ENVELOPE_HZ: u64 = 200;
/// How far apart a camera's own sound and a separate microphone may start.
const MAX_OFFSET_US: u64 = 10_000_000;
/// How much of each sound is compared.
const COMPARED_US: u64 = 120_000_000;
/// Below this correlation the two sounds are taken to start together.
const MIN_MATCH: f32 = 0.3;

/// What a take is made of: the camera's picture (optional) and the voice.
#[derive(Clone, Debug, PartialEq)]
pub struct TakeSources {
    pub camera: Option<StreamRef>,
    pub voice: StreamRef,
}

/// How the camera was lined up with a separate voice.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeSync {
    /// Camera time minus voice time, in microseconds.
    pub offset_us: i64,
    /// How well the camera's own sound matched the voice (0 to 1); `None` when nothing was
    /// compared (one file, or a camera without sound).
    pub score: Option<f32>,
}

/// Works out what each of a take's files is: the one with a picture is the camera (a
/// recording's camera); the voice is a sound-only file when there is one (a separate
/// microphone records better than a camera's), else a recording's microphone, else the
/// camera's own sound.
pub fn guess(document: &EditDocument, media: &[String]) -> Result<TakeSources, String> {
    let assets = media
        .iter()
        .map(|id| {
            document
                .asset(id)
                .filter(|a| !a.is_still())
                .ok_or_else(|| format!("No recording or file {id} in the project"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let picture = |a: &Asset| {
        a.streams
            .iter()
            .find(|s| s.kind == StreamKind::Picture && s.role == Role::Webcam)
            .or_else(|| {
                (!a.is_recording())
                    .then(|| a.streams.iter().find(|s| s.kind == StreamKind::Picture))
                    .flatten()
            })
            .map(|s| StreamRef::new(&a.id, &s.id))
    };
    let first_sound = |a: &Asset, role: Option<Role>| {
        a.streams
            .iter()
            .find(|s| s.kind == StreamKind::Sound && role.is_none_or(|r| s.role == r))
            .map(|s| StreamRef::new(&a.id, &s.id))
    };
    let camera = assets.iter().find_map(|a| picture(a));
    let separate = assets
        .iter()
        .filter(|a| !a.streams.iter().any(|s| s.kind == StreamKind::Picture))
        .find_map(|a| first_sound(a, None));
    let recorded = assets
        .iter()
        .filter(|a| a.is_recording())
        .find_map(|a| first_sound(a, Some(Role::Mic)));
    let own = camera
        .as_ref()
        .and_then(|c| document.asset(&c.asset))
        .and_then(|a| first_sound(a, None));
    let voice = separate
        .or(recorded)
        .or(own)
        .ok_or("None of these has sound: add the microphone file too")?;
    Ok(TakeSources { camera, voice })
}

/// The streams of `document` a take uses, from what the caller named: stream keys, or asset
/// ids (the asset's camera picture and its speech, else its first picture and sound).
pub fn resolve(
    document: &EditDocument,
    camera: Option<&str>,
    voice: Option<&str>,
) -> Result<TakeSources, String> {
    let camera = camera
        .map(|name| pick(document, name, StreamKind::Picture))
        .transpose()?;
    let voice = match voice {
        Some(name) => pick(document, name, StreamKind::Sound)?,
        None => {
            let camera = camera
                .as_ref()
                .ok_or("Name the take's camera, its voice, or both")?;
            pick(document, &camera.asset, StreamKind::Sound).map_err(|_| {
                "The camera file has no sound: name the microphone file as the voice".to_string()
            })?
        }
    };
    Ok(TakeSources { camera, voice })
}

fn pick(document: &EditDocument, name: &str, kind: StreamKind) -> Result<StreamRef, String> {
    let (asset_id, stream) = match StreamRef::parse(name) {
        Some(r) if document.asset(&r.asset).is_some() => (r.asset, Some(r.stream)),
        _ => (name.to_string(), None),
    };
    let asset = document
        .asset(&asset_id)
        .ok_or_else(|| format!("No media {name} in the project"))?;
    if asset.is_still() {
        return Err(format!("{} is a picture, not a recording", asset.name));
    }
    let wanted = match kind {
        StreamKind::Picture => Role::Webcam,
        StreamKind::Sound => Role::Mic,
    };
    let found = match stream {
        Some(id) => asset.stream(&id).filter(|s| s.kind == kind),
        None => asset
            .streams
            .iter()
            .find(|s| s.kind == kind && s.role == wanted)
            .or_else(|| asset.streams.iter().find(|s| s.kind == kind)),
    }
    .ok_or_else(|| match kind {
        StreamKind::Picture => format!("{} has no picture", asset.name),
        StreamKind::Sound => format!("{} has no sound", asset.name),
    })?;
    Ok(StreamRef::new(&asset.id, &found.id))
}

/// The roles the take's streams take on, so the camera is framed and the voice captioned.
pub fn roles(document: &EditDocument, sources: &TakeSources) -> Vec<(String, String, Role)> {
    let mut out = Vec::new();
    let mut want = |r: &StreamRef, role: Role| {
        let current = document
            .asset(&r.asset)
            .and_then(|a| a.stream(&r.stream))
            .map(|s| s.role);
        if current != Some(role) {
            out.push((r.asset.clone(), r.stream.clone(), role));
        }
    };
    if let Some(camera) = &sources.camera {
        want(camera, Role::Webcam);
    }
    want(&sources.voice, Role::Mic);
    out
}

/// Lines a camera up with a separate voice by comparing the camera's own sound with it.
pub fn sync(root: &Path, document: &EditDocument, sources: &TakeSources) -> TakeSync {
    let Some(camera) = &sources.camera else {
        return TakeSync {
            offset_us: 0,
            score: None,
        };
    };
    if camera.asset == sources.voice.asset {
        return TakeSync {
            offset_us: 0,
            score: None,
        };
    }
    let camera_sound = document.asset(&camera.asset).and_then(|a| {
        a.streams
            .iter()
            .find(|s| s.kind == StreamKind::Sound)
            .map(|s| (a, s.id.clone()))
    });
    let voice = document.asset(&sources.voice.asset);
    let (Some((camera_asset, camera_stream)), Some(voice_asset)) = (camera_sound, voice) else {
        return TakeSync {
            offset_us: 0,
            score: None,
        };
    };
    let (Ok(a), Ok(b)) = (
        envelope(root, voice_asset, &sources.voice.stream),
        envelope(root, camera_asset, &camera_stream),
    ) else {
        return TakeSync {
            offset_us: 0,
            score: None,
        };
    };
    match best_lag(&a, &b, (MAX_OFFSET_US * ENVELOPE_HZ / 1_000_000) as i64) {
        Some((lag, score)) if score >= MIN_MATCH => TakeSync {
            offset_us: lag * (1_000_000 / ENVELOPE_HZ) as i64,
            score: Some(score),
        },
        found => TakeSync {
            offset_us: 0,
            score: found.map(|f| f.1),
        },
    }
}

/// A sound's loudness, `ENVELOPE_HZ` values a second over its first `COMPARED_US`, on the
/// asset's clock.
fn envelope(root: &Path, asset: &Asset, stream: &str) -> Result<Vec<f32>, String> {
    let bins = (asset.duration_us.min(COMPARED_US) * ENVELOPE_HZ / 1_000_000) as usize;
    let mut out = vec![0f32; bins];
    let bin_us = 1_000_000 / ENVELOPE_HZ;
    for segment in crate::sequence::sources::sound_segments(root, asset, stream)? {
        if !segment.available || segment.start_us >= COMPARED_US {
            continue;
        }
        let path = crate::project::reader::safe_path(root, &segment.relative_path)?;
        let mut reader = crate::project::pcm::PcmReader::open(&path)?;
        let info = reader.info().clone();
        let channels = info.channels.max(1) as usize;
        let hop = (info.sample_rate as u64 / ENVELOPE_HZ).max(1) as usize;
        let mut buf = vec![0f32; hop * channels];
        let mut index = (segment.start_us / bin_us) as usize;
        while index < bins {
            let got = reader.read_frames(&mut buf, hop)?;
            if got == 0 {
                break;
            }
            let sum: f32 = buf[..got * channels].iter().map(|v| v * v).sum();
            out[index] = (1.0 + 1000.0 * (sum / (got * channels) as f32).sqrt()).ln();
            index += 1;
        }
    }
    Ok(out)
}

/// The shift of `b` against `a` (in bins, within `max_lag`) where they match best, and how well
/// (Pearson correlation over the overlap): `b[i + lag]` matches `a[i]`.
fn best_lag(a: &[f32], b: &[f32], max_lag: i64) -> Option<(i64, f32)> {
    let mut best: Option<(i64, f32)> = None;
    for lag in -max_lag..=max_lag {
        let start = 0i64.max(-lag) as usize;
        let end = (a.len() as i64).min(b.len() as i64 - lag);
        if end - (start as i64) < 2 * ENVELOPE_HZ as i64 {
            continue;
        }
        let end = end as usize;
        let n = (end - start) as f32;
        let pairs = || (start..end).map(|i| (a[i], b[(i as i64 + lag) as usize]));
        let (sa, sb) = pairs().fold((0.0, 0.0), |(x, y), (p, q)| (x + p, y + q));
        let (ma, mb) = (sa / n, sb / n);
        let (mut cov, mut va, mut vb) = (0f32, 0f32, 0f32);
        for (p, q) in pairs() {
            cov += (p - ma) * (q - mb);
            va += (p - ma) * (p - ma);
            vb += (q - mb) * (q - mb);
        }
        if va <= f32::EPSILON || vb <= f32::EPSILON {
            continue;
        }
        let r = cov / (va * vb).sqrt();
        if best.is_none_or(|(_, s)| r > s) {
            best = Some((lag, r));
        }
    }
    best
}

/// Numbers past every `<prefix><n>` id in `ids`.
fn next_number<'a>(prefix: char, ids: impl Iterator<Item = &'a str>) -> u64 {
    ids.filter_map(|id| id.strip_prefix(prefix)?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

/// The short's own timeline for a take: the camera and voice over the time both play,
/// lined up by `sync`, and empty Footage tracks.
pub fn timeline(
    document: &EditDocument,
    sources: &TakeSources,
    sync: &TakeSync,
) -> Result<Sequence, String> {
    let voice_asset = document
        .asset(&sources.voice.asset)
        .ok_or("The take's voice is not in the project")?;
    // Timeline time is the voice's own; the camera at t plays camera time t + offset.
    let mut start = 0u64;
    let mut end = voice_asset.duration_us;
    if let Some(camera) = &sources.camera {
        let camera_asset = document
            .asset(&camera.asset)
            .ok_or("The take's camera is not in the project")?;
        let offset = sync.offset_us;
        start = start.max((-offset).max(0) as u64);
        end = end.min((camera_asset.duration_us as i64 - offset).max(0) as u64);
    }
    if end <= start + crate::shorts::MIN_SHORT_US {
        return Err("The camera and the voice hardly overlap; check the files".into());
    }
    let length = (end - start).min(crate::shorts::MAX_SHORT_US);
    let mut tracks_number = next_number('t', document.track_ids_in_use());
    let mut track = |kind: TrackKind, name: &str, role: Role| {
        let mut track = Track::new(format!("t{tracks_number}"), kind);
        tracks_number += 1;
        track.name = name.into();
        track.role = Some(role);
        track
    };
    let mut footage = track(TrackKind::Video, "Footage", Role::Screen);
    let mut camera_track = track(TrackKind::Video, "Camera", Role::Webcam);
    let mut voice_track = track(TrackKind::Audio, "Voice", Role::Mic);
    let footage_sound = track(TrackKind::Audio, "Footage sound", Role::Background);
    let clip = |id: &str, source: &StreamRef, in_us: u64| Clip {
        id: id.into(),
        asset: source.asset.clone(),
        stream: source.stream.clone(),
        start_us: 0,
        in_us,
        duration_us: length,
        link: Some("l1".into()),
        fit: Default::default(),
    };
    voice_track.clips.push(clip("c1", &sources.voice, start));
    if let Some(camera) = &sources.camera {
        let in_us = (start as i64 + sync.offset_us) as u64;
        camera_track.clips.push(clip("c2", camera, in_us));
    }
    footage.clips.clear();
    let sequence = Sequence {
        tracks: vec![footage, camera_track, voice_track, footage_sound],
        // Footage is placed over the take, never pushing it along.
        magnetic: false,
    };
    crate::sequence::validate_sequence(&sequence, &document.assets)?;
    Ok(sequence)
}

/// A new short from a take, its timeline built and its camera-only stretches set.
pub fn new_short(
    document: &EditDocument,
    id: String,
    title: &str,
    reason: &str,
    sources: &TakeSources,
    sync: &TakeSync,
) -> Result<Short, String> {
    let sequence = timeline(document, sources, sync)?;
    let voice = document
        .asset(&sources.voice.asset)
        .ok_or("The take's voice is not in the project")?;
    // The look of the shorts made so far, so they match; each short has its own scenes.
    let mut layout: ShortLayout = document
        .shorts
        .first()
        .map(|s| s.layout.clone())
        .unwrap_or_default();
    layout.scenes.clear();
    layout.graphic_slots.clear();
    let mut short = Short {
        id,
        title: title.trim().to_string(),
        source_start_us: 0,
        source_end_us: voice.duration_us.max(1),
        reason: reason.trim().to_string(),
        layout,
        media: Some(voice.id.clone()),
        edit: Some(ShortEdit {
            sequence,
            ..Default::default()
        }),
        take: Some(ShortTake {
            camera: sources.camera.as_ref().map(StreamRef::key),
            voice: sources.voice.key(),
        }),
        length_us: None,
        edited_start_us: None,
        edited_end_us: None,
    };
    short.layout.scenes = camera_scenes(&short)?;
    Ok(short)
}

/// A stretch of footage to play under a take: a stretch of the video as it is edited, so only
/// what the video keeps (its cuts included).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Footage {
    /// Where it plays on the short's own timeline.
    pub at_us: u64,
    pub to_us: u64,
    /// Where it starts on the video's timeline.
    pub video_us: u64,
    /// Also play the footage's own background sound (game, desktop), ducked under the voice.
    #[serde(default = "yes")]
    pub sound: bool,
}

fn yes() -> bool {
    true
}

/// What footage a take short plays, in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacedFootage {
    pub clip_id: String,
    pub at_us: u64,
    pub to_us: u64,
    pub media: String,
    pub stream: String,
    /// Where it is on the video's timeline now; `None` once the video cut it.
    pub video_us: Option<u64>,
    pub sound: bool,
}

fn own(short: &Short) -> Result<&Sequence, String> {
    match (&short.take, &short.edit) {
        (Some(_), Some(edit)) => Ok(&edit.sequence),
        _ => Err("Footage goes under shorts made from a take".into()),
    }
}

fn own_mut(short: &mut Short) -> Result<&mut Sequence, String> {
    match (&short.take, &mut short.edit) {
        (Some(_), Some(edit)) => Ok(&mut edit.sequence),
        _ => Err("Footage goes under shorts made from a take".into()),
    }
}

/// The track of `kind` with `role` (the take's Footage tracks), made if it was removed.
fn role_track(sequence: &mut Sequence, kind: TrackKind, role: Role, taken: &mut u64) -> usize {
    if let Some(i) = sequence
        .tracks
        .iter()
        .position(|t| t.kind == kind && t.role == Some(role))
    {
        return i;
    }
    let mut track = Track::new(format!("t{taken}"), kind);
    *taken += 1;
    track.name = match kind {
        TrackKind::Video => "Footage",
        TrackKind::Audio => "Footage sound",
    }
    .into();
    track.role = Some(role);
    // Footage goes under the camera; its sound after the voice.
    let at = match kind {
        TrackKind::Video => 0,
        TrackKind::Audio => sequence.tracks.len(),
    };
    sequence.tracks.insert(at, track);
    at
}

/// Clears `[from, to)` on a track: clips over it are cut back or split around it.
fn clear_range(track: &mut Track, from: u64, to: u64, next_clip: &mut u64) {
    let mut out = Vec::new();
    for clip in std::mem::take(&mut track.clips) {
        let (start, end) = (clip.start_us, clip.end_us());
        if end <= from || start >= to {
            out.push(clip);
            continue;
        }
        if start < from {
            out.push(Clip {
                duration_us: from - start,
                ..clip.clone()
            });
        }
        if end > to {
            let id = if start < from {
                *next_clip += 1;
                format!("c{}", *next_clip - 1)
            } else {
                clip.id.clone()
            };
            out.push(Clip {
                id,
                start_us: to,
                in_us: clip.in_us + (to - start),
                duration_us: end - to,
                ..clip
            });
        }
    }
    out.retain(|c| c.duration_us >= crate::sequence::MIN_PIECE_US);
    out.sort_by_key(|c| c.start_us);
    track.clips = out;
}

/// The short's length: where its camera and voice end.
fn length(sequence: &Sequence) -> u64 {
    sequence
        .tracks
        .iter()
        .filter(|t| matches!(t.role, Some(Role::Webcam) | Some(Role::Mic)))
        .flat_map(|t| t.clips.iter().map(Clip::end_us))
        .max()
        .unwrap_or(0)
}

/// What `[from, to)` of the video's timeline plays, by role, from 0: its screen pictures
/// (lower tracks first, so a higher one wins) and its background sounds.
fn video_stretch(document: &EditDocument, from: u64, to: u64) -> (Vec<Clip>, Vec<Clip>) {
    let slice = crate::shorts::slice_sequence(&document.sequence, &document.assets, from, to);
    let mut pictures = Vec::new();
    let mut sounds = Vec::new();
    for track in &slice.tracks {
        let skip = match track.kind {
            TrackKind::Video => track.hidden,
            TrackKind::Audio => track.muted,
        };
        if skip {
            continue;
        }
        for clip in &track.clips {
            if document.asset(&clip.asset).is_some_and(|a| a.is_graphic()) {
                continue;
            }
            match crate::sequence::clip_role(&document.assets, track, clip) {
                Some(Role::Screen) => pictures.push(clip.clone()),
                Some(Role::Background) => sounds.push(clip.clone()),
                _ => {}
            }
        }
    }
    (pictures, sounds)
}

/// Plays `footage` under the take, over whatever footage was there.
pub fn place(document: &EditDocument, short: &mut Short, footage: &Footage) -> Result<(), String> {
    let mut taken = next_number('t', document.track_ids_in_use());
    let sequence = own_mut(short)?;
    let end = length(sequence);
    let (at, to) = (footage.at_us, footage.to_us.min(end));
    if to <= at || to - at < crate::sequence::MIN_PIECE_US {
        return Err(format!(
            "Footage at {:.1}-{:.1} s is outside the short ({:.1} s)",
            footage.at_us as f64 / 1e6,
            footage.to_us as f64 / 1e6,
            end as f64 / 1e6
        ));
    }
    let duration = to - at;
    let video_end = document.duration_us();
    if footage.video_us + duration > video_end {
        return Err(format!(
            "The video has no {:.1} s from {:.1} s (it is {:.1} s long)",
            duration as f64 / 1e6,
            footage.video_us as f64 / 1e6,
            video_end as f64 / 1e6
        ));
    }
    let (pictures, sounds) = video_stretch(document, footage.video_us, footage.video_us + duration);
    if pictures.is_empty() {
        return Err(format!(
            "No screen plays at {:.1} s of the video",
            footage.video_us as f64 / 1e6
        ));
    }
    let mut next_clip = next_number('c', sequence.clips().map(|(_, c)| c.id.as_str()));
    let link = format!(
        "l{}",
        next_number('l', sequence.clips().filter_map(|(_, c)| c.link.as_deref()))
    );
    let mut lay = |sequence: &mut Sequence, kind, role, clips: &[Clip]| {
        let index = role_track(sequence, kind, role, &mut taken);
        let track = &mut sequence.tracks[index];
        // Any footage there before goes, picture and sound.
        clear_range(track, at, to, &mut next_clip);
        for clip in clips {
            let start = at + clip.start_us;
            clear_range(track, start, start + clip.duration_us, &mut next_clip);
            track.clips.push(Clip {
                id: format!("c{next_clip}"),
                start_us: start,
                link: Some(link.clone()),
                ..clip.clone()
            });
            next_clip += 1;
            track.clips.sort_by_key(|c| c.start_us);
        }
    };
    lay(sequence, TrackKind::Video, Role::Screen, &pictures);
    let sounds = if footage.sound { sounds } else { Vec::new() };
    lay(sequence, TrackKind::Audio, Role::Background, &sounds);
    crate::sequence::validate_sequence(sequence, &document.assets)?;
    short.layout.scenes = camera_scenes(short)?;
    Ok(())
}

/// Takes the footage off `[from, to)` of the short's timeline (all of it without a range).
pub fn remove(
    document: &EditDocument,
    short: &mut Short,
    range: Option<(u64, u64)>,
) -> Result<(), String> {
    let sequence = own_mut(short)?;
    let mut next_clip = next_number('c', sequence.clips().map(|(_, c)| c.id.as_str()));
    let (from, to) = range.unwrap_or((0, u64::MAX));
    for track in &mut sequence.tracks {
        if matches!(track.role, Some(Role::Screen) | Some(Role::Background)) {
            clear_range(track, from, to, &mut next_clip);
        }
    }
    crate::sequence::validate_sequence(sequence, &document.assets)?;
    short.layout.scenes = camera_scenes(short)?;
    Ok(())
}

/// The footage the short plays, in order, and where each piece is on the video now.
pub fn placed(document: &EditDocument, short: &Short) -> Result<Vec<PlacedFootage>, String> {
    let sequence = own(short)?;
    let sounds: Vec<&Clip> = sequence
        .tracks
        .iter()
        .filter(|t| t.kind == TrackKind::Audio && t.role == Some(Role::Background))
        .flat_map(|t| t.clips.iter())
        .collect();
    let mut out: Vec<PlacedFootage> = sequence
        .tracks
        .iter()
        .filter(|t| t.kind == TrackKind::Video && t.role == Some(Role::Screen))
        .flat_map(|t| t.clips.iter())
        .map(|c| PlacedFootage {
            clip_id: c.id.clone(),
            at_us: c.start_us,
            to_us: c.end_us(),
            media: c.asset.clone(),
            stream: c.stream.clone(),
            video_us: document
                .picture_clock(&c.asset)
                .source_to_edited_us(c.in_us),
            sound: c.link.is_some() && sounds.iter().any(|s| s.link == c.link),
        })
        .collect();
    out.sort_by_key(|f| f.at_us);
    Ok(out)
}

/// The camera-only scenes of a take short: wherever no footage plays, the camera fills the
/// frame. Scenes set by hand stay; the automatic ones are worked out again.
pub fn camera_scenes(short: &Short) -> Result<Vec<Scene>, String> {
    let sequence = own(short)?;
    let voice = short
        .media
        .as_deref()
        .ok_or("A take short plays on its voice's clock")?;
    let clock = clock::asset_clock(sequence, voice);
    let manual: Vec<Scene> = short
        .layout
        .scenes
        .iter()
        .filter(|s| !s.auto)
        .cloned()
        .collect();
    // Timeline stretches already set by hand, then the footage, are not gaps.
    let mut covered: Vec<(u64, u64)> = manual
        .iter()
        .flat_map(|s| clock.source_range_to_edited(s.source_start_us, s.source_end_us))
        .collect();
    covered.extend(
        sequence
            .tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Video && t.role == Some(Role::Screen))
            .flat_map(|t| t.clips.iter().map(|c| (c.start_us, c.end_us()))),
    );
    covered.sort();
    let mut gaps = Vec::new();
    let mut at = 0u64;
    for (a, b) in covered {
        if a > at {
            gaps.push((at, a));
        }
        at = at.max(b);
    }
    let end = length(sequence);
    if end > at {
        gaps.push((at, end));
    }
    let mut out = manual;
    for (a, b) in gaps {
        for (s, e) in clock.edited_range_to_source(a, b) {
            out.push(Scene {
                auto: true,
                media: Some(voice.to_string()),
                ..Scene::new(format!("gap-{}", s / 1000), s, e, SceneMode::Camera)
            });
        }
    }
    Ok(crate::short_scenes::normalized(out))
}

/// Where the take's words `from..=to` play on the short's own timeline.
pub fn words_at(
    short: &Short,
    transcript: &crate::transcript::Transcript,
    from: &str,
    to: &str,
) -> Result<(u64, u64), String> {
    let sequence = own(short)?;
    let take = short.take.as_ref().ok_or("Not a take short")?;
    let voice = StreamRef::parse(&take.voice).ok_or("The take has no voice")?;
    let clock = clock::stream_clock(sequence, &voice.asset, &voice.stream);
    let word = |id: &str| {
        transcript
            .words
            .iter()
            .find(|w| w.id == id)
            .ok_or_else(|| format!("No word {id} in the take's transcript"))
    };
    let (first, last) = (word(from)?, word(to)?);
    let start = clock
        .source_to_edited_us(first.source_start_us)
        .ok_or_else(|| format!("Word {from} is outside the short"))?;
    let end = clock
        .source_to_edited_us(last.source_end_us.saturating_sub(1))
        .ok_or_else(|| format!("Word {to} is outside the short"))?
        + 1;
    if end <= start {
        return Err("`toWord` must come after `fromWord`".into());
    }
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_best_lag_finds_a_shifted_copy() {
        // A bumpy signal and the same signal 37 bins later.
        let a: Vec<f32> = (0..2000)
            .map(|i| ((i * 7919 % 101) as f32 / 101.0) + if i % 53 < 5 { 2.0 } else { 0.0 })
            .collect();
        let b: Vec<f32> = (0..2000)
            .map(|i| if i >= 37 { a[i - 37] } else { 0.0 })
            .collect();
        let (lag, score) = best_lag(&a, &b, 400).unwrap();
        assert_eq!(lag, 37);
        assert!(score > 0.95, "{score}");
    }

    const S: u64 = 1_000_000;

    /// A video "rec" of 60 s, a camera file "cam" of 40 s with its own sound, and a microphone
    /// file "mic" of 30 s.
    fn document() -> EditDocument {
        use crate::sequence::tests::{recording, video};
        let mut document = EditDocument::from_recording(recording("rec", 60 * S, &[])).unwrap();
        document.assets.push(video("cam", 40 * S));
        let mut mic = video("mic", 30 * S);
        mic.kind = crate::sequence::AssetKind::Audio;
        mic.streams.retain(|s| s.kind == StreamKind::Sound);
        document.assets.push(mic);
        document
    }

    fn scenes(short: &Short) -> Vec<(u64, u64)> {
        short
            .layout
            .scenes
            .iter()
            .map(|s| (s.source_start_us, s.source_end_us))
            .collect()
    }

    #[test]
    fn a_camera_file_with_its_own_sound_makes_a_short_of_the_camera_alone() {
        let document = document();
        let sources = resolve(&document, Some("cam"), None).unwrap();
        assert_eq!(sources.camera, Some(StreamRef::new("cam", "picture")));
        assert_eq!(sources.voice, StreamRef::new("cam", "sound0"));
        // The camera file's picture comes in as a screen: it becomes the camera.
        assert_eq!(
            roles(&document, &sources),
            vec![("cam".into(), "picture".into(), Role::Webcam)]
        );
        let sync = TakeSync {
            offset_us: 0,
            score: None,
        };
        let short = new_short(&document, "t1".into(), "Myth 1", "", &sources, &sync).unwrap();
        let sequence = &short.edit.as_ref().unwrap().sequence;
        assert_eq!(sequence.duration_us(), 40 * S);
        // No footage yet: the camera fills the frame throughout.
        assert_eq!(scenes(&short), vec![(0, 40 * S)]);
        crate::shorts::validate(std::slice::from_ref(&short)).unwrap();
        let timeline = crate::shorts::short_timeline(&document, &short).unwrap();
        assert_eq!(timeline.captions.track_id.as_deref(), Some("cam.sound0"));
    }

    #[test]
    fn a_takes_files_are_told_apart() {
        let document = document();
        // A camera with its own sound and a microphone file: the microphone is the voice.
        let both = guess(&document, &["mic".into(), "cam".into()]).unwrap();
        assert_eq!(both.camera, Some(StreamRef::new("cam", "picture")));
        assert_eq!(both.voice, StreamRef::new("mic", "sound0"));
        // The camera alone: its own sound.
        let camera = guess(&document, &["cam".into()]).unwrap();
        assert_eq!(camera.voice, StreamRef::new("cam", "sound0"));
        // A recording: its camera and its microphone.
        let recording = guess(&document, &["rec".into()]).unwrap();
        assert_eq!(recording.camera, Some(StreamRef::new("rec", "webcam")));
        assert_eq!(recording.voice, StreamRef::new("rec", "mic"));
        // A microphone alone: a voice-only take.
        let voice = guess(&document, &["mic".into()]).unwrap();
        assert_eq!(
            (voice.camera, voice.voice),
            (None, StreamRef::new("mic", "sound0"))
        );
    }

    #[test]
    fn a_separate_microphone_is_the_voice_and_the_camera_is_lined_up_with_it() {
        let document = document();
        let sources = resolve(&document, Some("cam"), Some("mic")).unwrap();
        // The camera started 2 s before the microphone.
        let sync = TakeSync {
            offset_us: 2 * S as i64,
            score: Some(0.8),
        };
        let short = new_short(&document, "t1".into(), "Myth 2", "", &sources, &sync).unwrap();
        let sequence = &short.edit.as_ref().unwrap().sequence;
        let clip = |role| {
            sequence
                .tracks
                .iter()
                .find(|t| t.role == Some(role))
                .and_then(|t| t.clips.first())
                .map(|c| (c.asset.as_str(), c.in_us, c.duration_us))
        };
        assert_eq!(clip(Role::Mic), Some(("mic", 0, 30 * S)));
        assert_eq!(clip(Role::Webcam), Some(("cam", 2 * S, 30 * S)));
        assert_eq!(short.media.as_deref(), Some("mic"));
    }

    #[test]
    fn footage_plays_under_the_take_and_the_camera_fills_the_gaps() {
        let mut document = document();
        // The video cut 20-25 s: footage from 18 s plays 18-20 s, then 25 s on.
        document.sequence = crate::sequence::edit::apply(
            &document.sequence,
            &document.assets,
            &crate::sequence::edit::SequenceEdit::DeleteRange {
                ranges: vec![crate::zoom::EditedRange {
                    start_us: 20 * S,
                    end_us: 25 * S,
                }],
                ripple: Some(true),
            },
        )
        .unwrap()
        .0;
        let sources = resolve(&document, Some("cam"), None).unwrap();
        let sync = TakeSync {
            offset_us: 0,
            score: None,
        };
        let mut short = new_short(&document, "t1".into(), "Myth", "", &sources, &sync).unwrap();
        document.shorts.push(short.clone());
        let footage = Footage {
            at_us: 5 * S,
            to_us: 10 * S,
            video_us: 18 * S,
            sound: true,
        };
        place(&document, &mut short, &footage).unwrap();
        let sequence = &short.edit.as_ref().unwrap().sequence;
        let on = |role: Role| -> Vec<(String, u64, u64, u64)> {
            sequence
                .tracks
                .iter()
                .filter(|t| t.role == Some(role))
                .flat_map(|t| t.clips.iter())
                .map(|c| (c.stream.clone(), c.start_us, c.in_us, c.duration_us))
                .collect()
        };
        // Only what the video keeps: the cut 5 s are skipped, and the game sound comes along.
        assert_eq!(
            on(Role::Screen),
            vec![
                ("screen".into(), 5 * S, 18 * S, 2 * S),
                ("screen".into(), 7 * S, 25 * S, 3 * S)
            ]
        );
        assert_eq!(
            on(Role::Background),
            vec![
                ("system".into(), 5 * S, 18 * S, 2 * S),
                ("system".into(), 7 * S, 25 * S, 3 * S)
            ]
        );
        let listed = placed(&document, &short).unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|f| (f.video_us, f.sound))
                .collect::<Vec<_>>(),
            vec![(Some(18 * S), true), (Some(20 * S), true)]
        );
        assert_eq!(scenes(&short), vec![(0, 5 * S), (10 * S, 40 * S)]);
        // Taking a second out of the middle; the camera fills that second.
        remove(&document, &mut short, Some((7 * S, 8 * S))).unwrap();
        assert_eq!(
            scenes(&short),
            vec![(0, 5 * S), (7 * S, 8 * S), (10 * S, 40 * S)]
        );
        // Without its sound, the footage plays silent.
        let quiet = Footage {
            at_us: 12 * S,
            to_us: 14 * S,
            video_us: 30 * S,
            sound: false,
        };
        place(&document, &mut short, &quiet).unwrap();
        assert!(!placed(&document, &short).unwrap().last().unwrap().sound);
        // Past the end of the take it is cut to it; outside it, or past the video, refused.
        let late = Footage {
            at_us: 38 * S,
            to_us: 45 * S,
            ..footage.clone()
        };
        place(&document, &mut short, &late).unwrap();
        assert_eq!(
            placed(&document, &short).unwrap().last().unwrap().to_us,
            40 * S
        );
        let outside = Footage {
            at_us: 41 * S,
            to_us: 45 * S,
            ..footage.clone()
        };
        assert!(place(&document, &mut short, &outside).is_err());
        let past_the_video = Footage {
            video_us: 54 * S,
            ..footage
        };
        assert!(place(&document, &mut short, &past_the_video).is_err());
        let sequence = &short.edit.as_ref().unwrap().sequence;
        crate::sequence::validate_sequence(sequence, &document.assets).unwrap();
        // Everything off: the camera throughout again.
        remove(&document, &mut short, None).unwrap();
        assert!(placed(&document, &short).unwrap().is_empty());
        assert_eq!(scenes(&short), vec![(0, 40 * S)]);
    }

    #[test]
    fn clearing_a_range_cuts_and_splits_clips() {
        let mut track = Track::new("t1".into(), TrackKind::Video);
        track.clips.push(Clip {
            id: "c1".into(),
            asset: "a".into(),
            stream: "screen".into(),
            start_us: 0,
            in_us: 100,
            duration_us: 10_000_000,
            link: None,
            fit: Default::default(),
        });
        let mut next = 2;
        clear_range(&mut track, 2_000_000, 5_000_000, &mut next);
        let spans: Vec<_> = track
            .clips
            .iter()
            .map(|c| (c.id.as_str(), c.start_us, c.in_us, c.duration_us))
            .collect();
        assert_eq!(
            spans,
            vec![
                ("c1", 0, 100, 2_000_000),
                ("c2", 5_000_000, 5_000_100, 5_000_000)
            ]
        );
    }
}
