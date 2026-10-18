//! Mouth noises and knocks: found in a sound's own audio (see [`crate::dsp::noises`]),
//! reviewed, then muted in place. Muting keeps the timing of every track; pause detection
//! hears a muted moment as silence, so jump cuts remove it with the pause around it.
//!
//! A saved transcript guards the words: no noise reaches into one. Noises close together with
//! no word between them are one place to review. The last scan of each sound is kept with the
//! project, so what was found and muted (by the editor or an agent) can be reviewed later
//! without listening again.
use super::{mutate_opened, require_handle, AppState};
use crate::dsp::noises::{find_noises, NoiseConfig, NoiseKind};
use crate::project::{pcm::PcmReader, reader::safe_path, OpenedProject};
use crate::sequence::{SourceRange, StreamKind};
use crate::transcript::pauses::{merge_ranges, subtract_ranges};
use crate::transcript::{store, ProviderKind, Transcript, WordKind};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Sound analyzed at a time, and the sound either side of it read with it (so a noise at the
/// edge of a piece is judged against what is around it).
const PIECE_US: u64 = 30_000_000;
const MARGIN_US: u64 = 500_000;
/// Noises this close together, with no word between them, are reviewed and muted as one.
const JOIN_US: u64 = 400_000;
/// How close a noise may come to a transcribed word.
const WORD_ROOM_US: u64 = 30_000;
/// What is left of a noise once the words are taken out of it: shorter is not worth muting.
const MIN_PART_US: u64 = 15_000;
/// Quieter than this is a pause when tightening Parakeet's words (as in the gaps pass).
const WORD_EDGE_DB: f32 = -42.0;
/// Muted moments left over from an earlier scan or by hand shorter than this are not listed.
const MIN_LISTED_US: u64 = 5_000;

/// A place to review: one noise, or several close together. In its sound's own time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoiseGroup {
    pub start_us: u64,
    pub end_us: u64,
    /// The kind that sounds longest in it.
    pub kind: NoiseKind,
    /// The loudest noise's height above the room, in dB.
    pub contrast_db: f32,
    /// Speech is right next to it, or a word had to be taken out of it: listen first.
    pub near_speech: bool,
    /// How many noises it joins.
    pub count: u32,
    /// What muting it silences: the noises themselves, not the room between them.
    pub parts: Vec<SourceRange>,
}

/// The last scan of a sound, kept in the project's cache.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedScan {
    key: String,
    config: NoiseConfig,
    /// The transcript guarded the words.
    guarded: bool,
    groups: Vec<NoiseGroup>,
    diagnostics: Vec<String>,
}

/// One place to review, in its sound's own time and where it plays on the timeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundNoise {
    pub source_start_us: u64,
    pub source_end_us: u64,
    pub start_us: u64,
    pub end_us: u64,
    /// None: a muted moment the last scan does not list (muted by hand or by an older scan).
    pub kind: Option<NoiseKind>,
    pub contrast_db: f32,
    /// Speech is right next to it: it may be part of a word.
    pub near_speech: bool,
    /// How many noises it joins.
    pub count: u32,
    /// What muting (or unmuting) it changes, in its sound's own time.
    pub parts: Vec<SourceRange>,
    /// All of it is muted.
    pub muted: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoiseScan {
    pub key: String,
    /// The project revision the timeline positions are for.
    pub revision: u64,
    /// The settings of the scan listed; None when this sound was never scanned.
    pub config: Option<NoiseConfig>,
    /// The transcript guarded the words in that scan.
    pub guarded: bool,
    pub noises: Vec<FoundNoise>,
    pub diagnostics: Vec<String>,
}

/// Listens for the noises in sound `key` (`<asset>.<stream>`), keeps the scan with the project
/// and returns it where the timeline plays it.
pub fn detect_mouth_noises_impl(
    state: &AppState,
    project_handle: String,
    key: String,
    config: NoiseConfig,
) -> Result<NoiseScan, String> {
    config.validate()?;
    let (root, segments, transcript, sound) = {
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
            store::load_transcript(reader.root(), &key).ok().flatten(),
            super::sound_context(reader, &key, false)?,
        )
    };
    // Parakeet's words cover the pauses next to them (and the noises in those pauses): first
    // tighten them to their own sound.
    let transcript = match transcript {
        Some(t) if t.provider == ProviderKind::Parakeet => {
            let quiet = crate::project::silence::scan_track_silence(
                &sound,
                &crate::dsp::silence::SilenceConfig {
                    threshold_db: WORD_EDGE_DB,
                    min_duration_ms: 20,
                    padding_ms: 0,
                    window_ms: Some(5),
                    step_ms: Some(5),
                    ..Default::default()
                },
            )?;
            Some(crate::transcript::pauses::tighten_loose_words(
                &t,
                &quiet.source_ranges,
                &quiet.covered,
                true,
            ))
        }
        other => other,
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
    let words = transcript.as_ref().map(guarded_words).unwrap_or_default();
    if transcript.is_none() {
        diagnostics.push(
            "No transcript for this sound: transcribe it first so no noise reaches into a word."
                .into(),
        );
    }
    let saved = SavedScan {
        key: key.clone(),
        config,
        guarded: transcript.is_some(),
        groups: group_noises(&found, &words),
        diagnostics,
    };
    if let Err(error) = save_scan(&root, &saved) {
        let mut scan = review(state, &project_handle, &key, Some(saved))?;
        scan.diagnostics
            .push(format!("This scan could not be kept for later: {error}"));
        return Ok(scan);
    }
    review(state, &project_handle, &key, Some(saved))
}

/// The last scan of sound `key` where the timeline plays it now, with every muted moment of
/// that sound (also those the scan does not list), without listening again.
pub fn noise_review_impl(
    state: &AppState,
    project_handle: String,
    key: String,
) -> Result<NoiseScan, String> {
    let root = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        reader.root().to_path_buf()
    };
    let saved = load_scan(&root, &key);
    review(state, &project_handle, &key, saved)
}

fn review(
    state: &AppState,
    project_handle: &str,
    key: &str,
    saved: Option<SavedScan>,
) -> Result<NoiseScan, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, project_handle)?;
    let document = reader.document();
    let source = document.stream_ref(key)?;
    let clock =
        crate::sequence::clock::stream_clock(&document.sequence, &source.asset, &source.stream);
    let muted: Vec<(u64, u64)> = merge_ranges(
        document
            .noise_mutes
            .get(key)
            .map(|ranges| ranges.iter().map(|r| (r.start_us, r.end_us)).collect())
            .unwrap_or_default(),
    );
    let is_muted = |a: u64, b: u64| {
        let at = muted.partition_point(|&(_, end)| end <= a);
        muted.get(at).is_some_and(|&(x, y)| x <= a && b <= y)
    };
    let groups = saved.as_ref().map(|s| s.groups.as_slice()).unwrap_or(&[]);
    let mut noises: Vec<FoundNoise> = groups
        .iter()
        .filter_map(|group| {
            let (start_us, end_us) = clock.edited_span_of(group.start_us, group.end_us)?;
            Some(FoundNoise {
                source_start_us: group.start_us,
                source_end_us: group.end_us,
                start_us,
                end_us: end_us.max(start_us + 1),
                kind: Some(group.kind),
                contrast_db: group.contrast_db,
                near_speech: group.near_speech,
                count: group.count,
                parts: group.parts.clone(),
                muted: group.parts.iter().all(|p| is_muted(p.start_us, p.end_us)),
            })
        })
        .collect();
    // Muted moments the scan does not list: still shown, so they can be heard and unmuted.
    let listed: Vec<(u64, u64)> = groups.iter().map(|g| (g.start_us, g.end_us)).collect();
    for (a, b) in subtract_ranges(&muted, &listed) {
        if b - a < MIN_LISTED_US {
            continue;
        }
        let Some((start_us, end_us)) = clock.edited_span_of(a, b) else {
            continue;
        };
        noises.push(FoundNoise {
            source_start_us: a,
            source_end_us: b,
            start_us,
            end_us: end_us.max(start_us + 1),
            kind: None,
            contrast_db: 0.0,
            near_speech: false,
            count: 1,
            parts: vec![SourceRange {
                start_us: a,
                end_us: b,
            }],
            muted: true,
        });
    }
    noises.sort_by_key(|n| n.start_us);
    Ok(NoiseScan {
        key: key.to_string(),
        revision: document.revision,
        config: saved.as_ref().map(|s| s.config),
        guarded: saved.as_ref().is_some_and(|s| s.guarded),
        noises,
        diagnostics: saved.map(|s| s.diagnostics).unwrap_or_default(),
    })
}

/// The transcribed words of `transcript` with the room no noise may reach into, merged.
fn guarded_words(transcript: &Transcript) -> Vec<(u64, u64)> {
    let room = WORD_ROOM_US;
    merge_ranges(
        transcript
            .words
            .iter()
            .filter(|w| w.kind == WordKind::Word && w.source_end_us > w.source_start_us)
            .map(|w| {
                (
                    w.source_start_us.saturating_sub(room),
                    w.source_end_us.saturating_add(room),
                )
            })
            .collect(),
    )
}

/// Takes the words out of the noises (`noises` in order) and joins those close together with no
/// word between them into one place to review.
fn group_noises(noises: &[crate::dsp::noises::Noise], words: &[(u64, u64)]) -> Vec<NoiseGroup> {
    let word_between = |a: u64, b: u64| {
        let at = words.partition_point(|&(_, end)| end <= a);
        words.get(at).is_some_and(|&(start, _)| start < b)
    };
    // Per group: how long each kind sounds in it.
    let mut groups: Vec<(NoiseGroup, Vec<(NoiseKind, u64)>)> = Vec::new();
    for noise in noises {
        let parts: Vec<(u64, u64)> = subtract_ranges(&[(noise.start_us, noise.end_us)], words)
            .into_iter()
            .filter(|(a, b)| b - a >= MIN_PART_US)
            .collect();
        let kept: u64 = parts.iter().map(|(a, b)| b - a).sum();
        // A word was in it: what is left may be the word's own edge.
        let near_speech = noise.near_speech || kept < noise.end_us - noise.start_us;
        for (a, b) in parts {
            if let Some((group, kinds)) = groups.last_mut() {
                if group.near_speech == near_speech
                    && a <= group.end_us + JOIN_US
                    && !word_between(group.end_us, a)
                {
                    group.end_us = group.end_us.max(b);
                    group.contrast_db = group.contrast_db.max(noise.contrast_db);
                    group.count += 1;
                    match group.parts.last_mut() {
                        Some(last) if a <= last.end_us => last.end_us = last.end_us.max(b),
                        _ => group.parts.push(SourceRange {
                            start_us: a,
                            end_us: b,
                        }),
                    }
                    match kinds.iter_mut().find(|(k, _)| *k == noise.kind) {
                        Some((_, us)) => *us += b - a,
                        None => kinds.push((noise.kind, b - a)),
                    }
                    group.kind = kinds.iter().max_by_key(|(_, us)| *us).unwrap().0;
                    continue;
                }
            }
            groups.push((
                NoiseGroup {
                    start_us: a,
                    end_us: b,
                    kind: noise.kind,
                    contrast_db: noise.contrast_db,
                    near_speech,
                    count: 1,
                    parts: vec![SourceRange {
                        start_us: a,
                        end_us: b,
                    }],
                },
                vec![(noise.kind, b - a)],
            ));
        }
    }
    groups.into_iter().map(|(group, _)| group).collect()
}

fn scan_path(root: &Path, key: &str) -> Result<std::path::PathBuf, String> {
    if key.is_empty()
        || key.len() > 160
        || key.starts_with('.')
        || key.contains("..")
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("Invalid sound key".into());
    }
    safe_path(root, &format!("cache/noises/{key}.json"))
}

fn save_scan(root: &Path, scan: &SavedScan) -> Result<(), String> {
    use std::io::Write;
    let path = scan_path(root, &scan.key)?;
    let dir = path.parent().ok_or("Invalid noise cache path")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    temp.write_all(&serde_json::to_vec(scan).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    temp.persist(&path).map_err(|e| e.to_string())?;
    Ok(())
}

/// The last scan of `key`, if one was kept and still reads.
fn load_scan(root: &Path, key: &str) -> Option<SavedScan> {
    let bytes = std::fs::read(scan_path(root, key).ok()?).ok()?;
    serde_json::from_slice::<SavedScan>(&bytes)
        .ok()
        .filter(|s| s.key == key)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::noises::Noise;

    fn noise(start_ms: u64, end_ms: u64, kind: NoiseKind, near_speech: bool) -> Noise {
        Noise {
            start_us: start_ms * 1000,
            end_us: end_ms * 1000,
            kind,
            contrast_db: 30.0,
            near_speech,
            release_only: false,
            core_us: (start_ms * 1000, end_ms * 1000),
        }
    }

    fn spans(groups: &[NoiseGroup]) -> Vec<(u64, u64)> {
        groups
            .iter()
            .map(|g| (g.start_us / 1000, g.end_us / 1000))
            .collect()
    }

    #[test]
    fn noises_close_together_are_one_place_and_mute_only_themselves() {
        let groups = group_noises(
            &[
                noise(1000, 1050, NoiseKind::Click, false),
                noise(1200, 1400, NoiseKind::Mouth, false),
                noise(1500, 1520, NoiseKind::Click, false),
                // Too far from the rest.
                noise(3000, 3100, NoiseKind::Click, false),
            ],
            &[],
        );
        assert_eq!(spans(&groups), vec![(1000, 1520), (3000, 3100)]);
        assert_eq!(groups[0].count, 3);
        assert_eq!(groups[0].kind, NoiseKind::Mouth, "the kind heard longest");
        assert_eq!(
            groups[0]
                .parts
                .iter()
                .map(|p| (p.start_us / 1000, p.end_us / 1000))
                .collect::<Vec<_>>(),
            vec![(1000, 1050), (1200, 1400), (1500, 1520)],
            "the room between them stays as it is"
        );
    }

    #[test]
    fn a_word_is_never_muted_and_keeps_noises_either_side_apart() {
        let words = [(2000_000, 2300_000)];
        let groups = group_noises(
            &[
                // Runs into the word: only what is before it stays, and it may be speech.
                noise(1800, 2100, NoiseKind::Click, false),
                // Wholly inside the word: gone.
                noise(2100, 2200, NoiseKind::Knock, false),
                // Close after the word, but the word is between: its own place.
                noise(2350, 2400, NoiseKind::Click, true),
            ],
            &words,
        );
        assert_eq!(spans(&groups), vec![(1800, 2000), (2350, 2400)]);
        assert!(groups[0].near_speech);
        assert!(groups.iter().all(|g| g
            .parts
            .iter()
            .all(|p| p.end_us <= 2000_000 || p.start_us >= 2300_000)));
    }

    /// Scans a copy of a real project's metadata (never the original):
    /// `AEROEDITS_NOISE_PROJECT=<project> AEROEDITS_NOISE_KEY=<asset.stream>
    /// [AEROEDITS_NOISE_TRUTH=<transcript json with exact word timings>] cargo test --release
    /// --lib noise_project_probe -- --ignored --nocapture`.
    #[test]
    #[ignore = "set AEROEDITS_NOISE_PROJECT to an existing project"]
    fn noise_project_probe() {
        let source = std::path::PathBuf::from(std::env::var("AEROEDITS_NOISE_PROJECT").unwrap());
        let key = std::env::var("AEROEDITS_NOISE_KEY").unwrap();
        let scratch = tempfile::tempdir().unwrap();
        for name in ["project.json", "aeroedits.json"] {
            std::fs::copy(source.join(name), scratch.path().join(name)).unwrap();
        }
        std::fs::create_dir(scratch.path().join("transcripts")).unwrap();
        if std::env::var("AEROEDITS_NOISE_UNGUARDED").is_err() {
            let name = format!("{key}.json");
            std::fs::copy(
                source.join("transcripts").join(&name),
                scratch.path().join("transcripts").join(&name),
            )
            .unwrap();
        }
        let reader = crate::project::ProjectReader::open(scratch.path()).unwrap();
        let handle = reader.summary.project_handle.clone();
        let state = AppState::new();
        *state.opened_project.lock() = Some(reader);
        let sensitivity = std::env::var("AERO_NOISE_SENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.5);
        let config = NoiseConfig {
            sensitivity,
            ..Default::default()
        };
        let scan = detect_mouth_noises_impl(&state, handle, key, config).unwrap();
        let places: Vec<_> = scan.noises.iter().filter(|n| n.kind.is_some()).collect();
        let chosen: Vec<_> = places.iter().filter(|n| !n.near_speech).collect();
        let muted_us: u64 = chosen
            .iter()
            .flat_map(|n| &n.parts)
            .map(|p| p.end_us - p.start_us)
            .sum();
        let noises: u32 = places.iter().map(|n| n.count).sum();
        println!(
            "PROBE {noises} noises in {} places, {} chosen, {:.1} s muted, guarded={}",
            places.len(),
            chosen.len(),
            muted_us as f64 / 1e6,
            scan.guarded
        );
        if let Ok(truth) = std::env::var("AEROEDITS_NOISE_TRUTH") {
            let truth: Transcript = serde_json::from_slice(&std::fs::read(truth).unwrap()).unwrap();
            let words: Vec<(u64, u64, &str)> = truth
                .words
                .iter()
                .filter(|w| w.kind == WordKind::Word)
                .map(|w| (w.source_start_us, w.source_end_us, w.text.as_str()))
                .collect();
            let (mut hits, mut hit_us) = (0, 0);
            for n in &chosen {
                let mut over = 0;
                let mut said = Vec::new();
                for p in &n.parts {
                    for &(a, b, text) in &words {
                        let (x, y) = (a.max(p.start_us), b.min(p.end_us));
                        if y > x {
                            over += y - x;
                            said.push(text);
                        }
                    }
                }
                if over > 0 {
                    hits += 1;
                    hit_us += over;
                    if std::env::var("AERO_NOISE_LIST").is_ok() {
                        println!(
                            "  {:9.3} into {:4} ms of {}",
                            n.source_start_us as f64 / 1e6,
                            over / 1000,
                            said.join(" ")
                        );
                    }
                }
            }
            println!(
                "PROBE {hits} chosen places reach into exact words, {} ms in all",
                hit_us / 1000
            );
        }
    }

    #[test]
    fn noises_next_to_speech_are_not_joined_with_clear_ones() {
        let groups = group_noises(
            &[
                noise(1000, 1050, NoiseKind::Click, false),
                noise(1100, 1150, NoiseKind::Click, true),
            ],
            &[],
        );
        assert_eq!(groups.len(), 2);
    }
}
