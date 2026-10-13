//! Transcript-assisted jump cuts using existing per-source transcripts and measured PCM.
use super::{require_handle, sound_context, speech_on_timeline, AppState, ALL_SPEECH};
use crate::dsp::silence::{
    channel_policy_name, SilenceConfig, SilenceCutInterval, SilenceDetectionResult,
};
use crate::project::silence::{
    common_pauses, scan_track_silence, secondary_audio_guard_config, PlaysAndPauses,
};
use crate::sequence::StreamRef;
use crate::transcript::{pauses, store};

pub fn detect_transcript_pauses_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    track_ids: Option<Vec<String>>,
    config: SilenceConfig,
) -> Result<SilenceDetectionResult, String> {
    config.validate()?;
    detect_non_speech_gaps_impl(
        state,
        project_handle,
        track_id,
        track_ids,
        pauses::TranscriptGapConfig {
            min_duration_ms: config.min_duration_ms,
            padding_ms: config.padding_ms,
            refine_word_edges: false,
            edge_threshold_db: -42.0,
        },
    )
}

pub fn detect_non_speech_gaps_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    track_ids: Option<Vec<String>>,
    config: pauses::TranscriptGapConfig,
) -> Result<SilenceDetectionResult, String> {
    config.validate()?;
    let guard_secondary = track_ids.is_some();
    if let Some(ids) = &track_ids {
        if ids.len() != 2 || ids[0] == ids[1] {
            return Err("Choose two different sound sources".into());
        }
        if ids[0] != track_id {
            return Err("The first source must be the primary transcript source".into());
        }
    }
    let (sounds, revision) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        let chosen = if track_ids.is_none() && track_id == ALL_SPEECH {
            speech_on_timeline(reader.document())
        } else {
            let ids = track_ids.unwrap_or_else(|| vec![track_id.clone()]);
            ids.into_iter()
                .map(|key| {
                    let clips = reader
                        .document()
                        .sequence
                        .tracks
                        .iter()
                        .filter(|track| !track.muted)
                        .flat_map(|track| &track.clips)
                        .filter(|clip| StreamRef::new(&clip.asset, &clip.stream).key() == key)
                        .cloned()
                        .collect::<Vec<_>>();
                    (key, clips)
                })
                .collect()
        };
        if chosen.is_empty() || chosen.iter().any(|(_, clips)| clips.is_empty()) {
            return Err("Choose sounds on unmuted timeline tracks".into());
        }
        chosen.into_iter().enumerate().map(|(index, (key, clips))| {
            let ctx = sound_context(reader, &key, false)?;
            let transcript = store::load_transcript(reader.root(), &key)?;
            if !guard_secondary || index == 0 {
                let saved = transcript.as_ref().ok_or_else(|| format!("Transcribe the primary source first. No saved transcript for {key}; use the Transcript panel."))?;
                if saved.words.iter().filter(|w| w.kind == crate::transcript::WordKind::Word).count() < 2 {
                    return Err(format!("The transcript for {key} needs at least two words. Transcribe this source first, or use Remove silence."));
                }
            }
            Ok((ctx, clips, transcript))
        }).collect::<Result<Vec<_>, String>>().map(|sounds| (sounds, reader.document().revision))?
    };
    let found = std::thread::scope(|scope| {
        let jobs: Vec<_> = sounds
            .iter()
            .enumerate()
            .map(|(index, (ctx, _, transcript))| {
                let config = &config;
                scope.spawn(move || {
                    let is_guard = guard_secondary && index > 0;
                    let scan = scan_track_silence(
                        ctx,
                        &if is_guard {
                            secondary_audio_guard_config()
                        } else {
                            SilenceConfig {
                                threshold_db: config.edge_threshold_db,
                                min_duration_ms: 20,
                                padding_ms: 0,
                                window_ms: Some(5),
                                step_ms: Some(5),
                                ..SilenceConfig::default()
                            }
                        },
                    )?;
                    let ranges = if is_guard {
                        if let Some(saved) = transcript {
                            pauses::protect_words(scan.source_ranges.clone(), saved, 0)
                        } else {
                            scan.source_ranges.clone()
                        }
                    } else {
                        let saved = transcript.as_ref().unwrap();
                        let refined;
                        let words = if config.refine_word_edges {
                            refined = pauses::refine_quiet_edges(
                                saved,
                                &scan.source_ranges,
                                &scan.covered,
                            );
                            &refined
                        } else {
                            saved
                        };
                        pauses::gap_ranges(words, &scan.covered, config)
                    };
                    Ok((scan, ranges))
                })
            })
            .collect();
        jobs.into_iter()
            .map(|job| {
                job.join()
                    .unwrap_or_else(|_| Err("Transcript gap analysis failed".to_string()))
            })
            .collect::<Result<Vec<_>, String>>()
    })?;
    let lists: Vec<PlaysAndPauses> = sounds
        .iter()
        .zip(&found)
        .flat_map(|((_, clips, _), (_, gaps))| {
            clips.iter().map(move |clip| {
                let mapped = gaps
                    .iter()
                    .filter_map(|&(a, b)| {
                        let start = a.max(clip.in_us);
                        let end = b.min(clip.out_us());
                        (end > start).then(|| {
                            (
                                clip.start_us + start - clip.in_us,
                                clip.start_us + end - clip.in_us,
                            )
                        })
                    })
                    .collect();
                (vec![(clip.start_us, clip.end_us())], mapped)
            })
        })
        .collect();
    let padding = u64::from(config.padding_ms);
    let min_us = u64::from(config.min_duration_ms).saturating_sub(2 * padding) * 1_000;
    let gaps = if guard_secondary {
        let primary_count = sounds[0].1.len();
        guarded_primary_gaps(
            &lists[..primary_count],
            &lists[primary_count..],
            min_us,
            config.padding_ms,
        )
    } else {
        common_pauses(&lists, min_us)
    };
    let mut diagnostics = vec!["Gaps can contain missed words or useful tutorial sounds. Listen before selecting cuts. Noises inside recognized words are kept.".into()];
    if config.refine_word_edges {
        diagnostics.push("Quiet word edges are refined from audio for this review. Saved transcript timings stay unchanged. Listen for soft word endings.".into());
    }
    if config.padding_ms < 40 {
        diagnostics.push(
            "Tight word margins can expose timestamp errors. Listen to word edges before applying."
                .into(),
        );
    }
    if guard_secondary {
        diagnostics.push("Cuts follow gaps in the primary transcript. Speech, music, video audio, and other sounds on the protected source are kept.".into());
    }
    for (scan, _) in &found {
        for line in &scan.diagnostics {
            if !diagnostics.contains(line) {
                diagnostics.push(line.clone());
            }
        }
    }
    if gaps.is_empty() {
        diagnostics.push("No gaps between recognized words matched these settings. Audio before the first word and after the last word is kept.".into());
    }
    let first = &found[0].0;
    let dependencies = sounds
        .iter()
        .map(|(ctx, _, t)| {
            t.as_ref().map(|saved| saved.dependency()).unwrap_or(
                crate::transcript::TranscriptDependency {
                    track_id: ctx.track_id.clone(),
                    word_stamp: None,
                },
            )
        })
        .collect::<Vec<_>>();
    validate_analysis(state, &project_handle, revision, &dependencies)?;
    Ok(SilenceDetectionResult {
        track_id: if sounds.len() == 1 {
            sounds[0].0.track_id.clone()
        } else {
            ALL_SPEECH.into()
        },
        sample_rate: first.sample_rate,
        channels: first.channels,
        channel_policy: channel_policy_name(first.policy),
        suggestions: gaps
            .into_iter()
            .enumerate()
            .map(|(index, (start_us, end_us))| SilenceCutInterval {
                id: format!("transcript-gap-{}", index + 1),
                start_us,
                end_us,
                duration_ms: (end_us - start_us) / 1_000,
                selected: false,
                // Combined/reordered sources have no single source-time interval.
                source_start_us: 0,
                source_end_us: 0,
            })
            .collect(),
        diagnostics,
        thresholds: Vec::new(),
        transcript_dependencies: dependencies,
    })
}

/// The secondary source only vetoes primary candidates; quiet PC-only footage must
/// never introduce new cuts. Separate clips also protect repeated/overlapping playback.
/// Its quiet spans are unpadded. Expand sound/unknown spans in timeline time exactly
/// once, including beyond clip edges; quiet edges themselves need no extra protection.
fn guarded_primary_gaps(
    primary: &[PlaysAndPauses],
    secondary: &[PlaysAndPauses],
    min_us: u64,
    padding_ms: u32,
) -> Vec<(u64, u64)> {
    let candidates = common_pauses(primary, 1);
    let end = primary
        .iter()
        .flat_map(|(plays, _)| plays)
        .map(|&(_, b)| b)
        .max()
        .unwrap_or(0);
    let padding_us = u64::from(padding_ms.max(pauses::PC_AUDIO_PADDING_MS)) * 1_000;
    let mut protected = Vec::new();
    for (plays, quiet) in secondary {
        let quiet = pauses::merge_ranges(quiet.clone());
        for &(start, end) in plays {
            let mut cursor = start;
            let first = quiet.partition_point(|&(_, b)| b <= start);
            for &(a, b) in &quiet[first..] {
                if a >= end {
                    break;
                }
                if a > cursor {
                    protected.push((cursor, a.min(end)));
                }
                cursor = cursor.max(b.min(end));
            }
            if cursor < end {
                protected.push((cursor, end));
            }
        }
    }
    let protected = pauses::merge_ranges(
        protected
            .into_iter()
            .map(|(a, b)| (a.saturating_sub(padding_us), b.saturating_add(padding_us)))
            .collect(),
    );
    common_pauses(
        &[(vec![(0, end)], candidates), (protected, Vec::new())],
        min_us.max(1),
    )
}

pub(crate) fn validate_analysis(
    state: &AppState,
    handle: &str,
    revision: u64,
    dependencies: &[crate::transcript::TranscriptDependency],
) -> Result<(), String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, handle)?;
    if reader.document().revision != revision {
        return Err("The timeline changed during this analysis. Find suggestions again.".into());
    }
    store::validate_dependencies(reader.root(), dependencies)
}

pub fn apply_jump_cuts_impl(
    state: &AppState,
    handle: String,
    revision: u64,
    ranges: Vec<crate::zoom::EditedRange>,
    dependencies: Vec<crate::transcript::TranscriptDependency>,
) -> Result<crate::project::OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &handle)?;
    store::validate_dependencies(reader.root(), &dependencies)?;
    super::edit_sequence_locked(
        state,
        reader,
        revision,
        &crate::sequence::edit::SequenceEdit::DeleteRange {
            ranges,
            ripple: Some(true),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{detect_silence_impl, project_sequence_edit_impl, project_undo_impl};
    use crate::fixtures::{generate_pcm16_wav, TestProject};
    use crate::project::{JournalRecord, ProjectReader, TrackDescriptor, TrackType};
    use crate::sequence::edit::SequenceEdit;
    use crate::transcript::{test_word, ProviderKind, Transcript};
    use std::fs;

    /// Read an existing tutorial through a temporary metadata copy. Never open the
    /// original for editing or persist a cut/transcript there.
    #[test]
    #[ignore = "set AEROEDITS_CLEANUP_PROBE to an existing tutorial project"]
    fn tutorial_cleanup_probe() {
        let source = std::path::PathBuf::from(
            std::env::var("AEROEDITS_CLEANUP_PROBE").expect("project path"),
        );
        let scratch = tempfile::tempdir().unwrap();
        for name in ["project.json", "aeroedits.json"] {
            fs::copy(source.join(name), scratch.path().join(name)).unwrap();
        }
        fs::create_dir(scratch.path().join("transcripts")).unwrap();
        for item in fs::read_dir(source.join("transcripts")).unwrap() {
            let item = item.unwrap();
            if item.path().extension().is_some_and(|ext| ext == "json") {
                fs::copy(
                    item.path(),
                    scratch.path().join("transcripts").join(item.file_name()),
                )
                .unwrap();
            }
        }
        let reader = ProjectReader::open(scratch.path()).unwrap();
        let doc = reader.document();
        let mut sources = doc
            .assets
            .iter()
            .flat_map(|asset| {
                asset
                    .streams
                    .iter()
                    .filter(|stream| stream.kind == crate::sequence::StreamKind::Sound)
                    .map(|stream| (StreamRef::new(&asset.id, &stream.id).key(), stream.role))
            })
            .collect::<Vec<_>>();
        sources.sort_by_key(|(_, role)| *role != crate::sequence::Role::Mic);
        let keys = sources
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        assert!(keys.len() >= 2);
        let handle = reader.summary.project_handle.clone();
        let ctx = sound_context(&reader, &keys[0], false).unwrap();
        let t = store::load_transcript(reader.root(), &keys[0])
            .unwrap()
            .unwrap();
        let scan = scan_track_silence(
            &ctx,
            &SilenceConfig {
                threshold_db: -42.0,
                min_duration_ms: 20,
                padding_ms: 0,
                window_ms: Some(5),
                step_ms: Some(5),
                ..SilenceConfig::default()
            },
        )
        .unwrap();
        let refined = pauses::refine_quiet_edges(&t, &scan.source_ranges, &scan.covered);
        println!(
            "PROBE words={} refined={} coverage_s={:.1}",
            t.words.len(),
            t.words
                .iter()
                .zip(&refined.words)
                .filter(|(a, b)| a.source_start_us != b.source_start_us
                    || a.source_end_us != b.source_end_us)
                .count(),
            scan.covered.iter().map(|(a, b)| b - a).sum::<u64>() as f64 / 1e6
        );
        let state = AppState::new();
        *state.opened_project.lock() = Some(reader);
        for level in [0.2, 0.5, 0.8] {
            let result = detect_silence_impl(
                &state,
                handle.clone(),
                keys[0].clone(),
                SilenceConfig {
                    auto_level: Some(level),
                    min_duration_ms: 250,
                    padding_ms: 40,
                    ..SilenceConfig::default()
                },
            )
            .unwrap();
            println!(
                "PROBE auto={level} thresholds={:?} cuts={} removed_s={:.2}",
                result.thresholds,
                result.suggestions.len(),
                result
                    .suggestions
                    .iter()
                    .map(|c| c.end_us - c.start_us)
                    .sum::<u64>() as f64
                    / 1e6
            );
        }
        let fixed = detect_silence_impl(
            &state,
            handle.clone(),
            keys[0].clone(),
            SilenceConfig {
                threshold_db: -42.0,
                min_duration_ms: 250,
                padding_ms: 40,
                ..SilenceConfig::default()
            },
        )
        .unwrap();
        println!(
            "PROBE fixed=-42 cuts={} removed_s={:.2}",
            fixed.suggestions.len(),
            fixed
                .suggestions
                .iter()
                .map(|c| c.end_us - c.start_us)
                .sum::<u64>() as f64
                / 1e6
        );
        for (label, min, pad, refine) in [
            ("old", 200, 80, false),
            ("balanced", 150, 40, true),
            ("tight", 60, 10, true),
            ("aggressive", 20, 0, true),
        ] {
            for two in [false, true] {
                let result = detect_non_speech_gaps_impl(
                    &state,
                    handle.clone(),
                    keys[0].clone(),
                    two.then(|| keys[..2].to_vec()),
                    pauses::TranscriptGapConfig {
                        min_duration_ms: min,
                        padding_ms: pad,
                        refine_word_edges: refine,
                        edge_threshold_db: -42.0,
                    },
                )
                .unwrap();
                println!(
                    "PROBE gaps={label} two={two} cuts={} removed_s={:.2}",
                    result.suggestions.len(),
                    result
                        .suggestions
                        .iter()
                        .map(|c| c.end_us - c.start_us)
                        .sum::<u64>() as f64
                        / 1e6
                );
            }
        }
    }

    #[test]
    fn transcript_pauses_reuse_saved_words_protect_both_voices_and_remap_after_edits() {
        let dir = tempfile::tempdir().unwrap();
        let mut bundle = TestProject::create(dir.path(), "transcript-conversation");
        let rate = 8_000;
        for (id, track_type, amplitude) in [
            ("mic", TrackType::MicAudio, 1000),
            ("system", TrackType::SystemAudio, 10000),
        ] {
            let path = format!("media/{id}/voice.wav");
            let words = if id == "mic" {
                vec![(200, 1000), (3000, 3500), (6000, 6500), (7000, 7500)]
            } else {
                vec![(200, 1000), (2000, 2500), (6000, 6500), (7000, 7500)]
            };
            let samples: Vec<i16> = (0..rate * 8)
                .map(|i| {
                    let ms = i as u64 * 1000 / rate as u64;
                    let level = if (4000..5000).contains(&ms) {
                        0
                    } else if words.iter().any(|&(a, b)| ms >= a && ms < b) {
                        amplitude
                    } else {
                        amplitude * 2
                    };
                    ((i as f32 / rate as f32 * 440.0 * std::f32::consts::TAU).sin() * level as f32)
                        as i16
                })
                .collect();
            let wav = generate_pcm16_wav(rate, 1, &samples);
            fs::write(bundle.root_path().join(&path), &wav).unwrap();
            bundle.manifest_mut().tracks.push(TrackDescriptor {
                id: id.into(),
                track_type,
                codec: "pcm".into(),
                relative_path: path.clone(),
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
                relative_path: path,
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
        let original = reader.summary.clone();
        let sources: Vec<_> = reader
            .document()
            .assets
            .iter()
            .flat_map(|asset| {
                asset
                    .streams
                    .iter()
                    .filter(|stream| stream.kind == crate::sequence::StreamKind::Sound)
                    .map(|stream| (StreamRef::new(&asset.id, &stream.id).key(), stream.role))
            })
            .collect();
        let keys: Vec<_> = sources.iter().map(|(key, _)| key.clone()).collect();
        let mic_key = sources
            .iter()
            .find(|(_, role)| *role == crate::sequence::Role::Mic)
            .unwrap()
            .0
            .clone();
        let state = AppState::new();
        *state.opened_project.lock() = Some(reader);
        let config = SilenceConfig {
            min_duration_ms: 500,
            padding_ms: 120,
            auto_level: Some(0.2),
            ..SilenceConfig::default()
        };
        // Missing transcripts fail explicitly; they never silently become cuttable audio.
        assert!(detect_transcript_pauses_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            Some(keys.clone()),
            config.clone()
        )
        .unwrap_err()
        .contains("Transcribe"));
        for (key, role) in &sources {
            let words = if *role == crate::sequence::Role::Mic {
                vec![(200, 1000), (3000, 3500), (6000, 6500), (7000, 7500)]
            } else {
                vec![(200, 1000), (2000, 2500), (6000, 6500), (7000, 7500)]
            };
            let transcript = Transcript::new(
                key.clone(),
                ProviderKind::Parakeet,
                "test".into(),
                None,
                words
                    .into_iter()
                    .map(|(a, b)| test_word(if a == 3000 { "um" } else { "voice" }, a, b))
                    .collect(),
            );
            store::save_transcript(&folder, &transcript).unwrap();
            if key == &mic_key {
                assert!(
                    detect_transcript_pauses_impl(
                        &state,
                        original.project_handle.clone(),
                        mic_key.clone(),
                        Some(keys.clone()),
                        config.clone()
                    )
                    .is_ok(),
                    "PC sound protection does not require a PC transcript"
                );
            }
        }
        let combined = detect_transcript_pauses_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            Some(keys.clone()),
            config.clone(),
        )
        .unwrap();
        let ranges: Vec<_> = combined
            .suggestions
            .iter()
            .map(|gap| (gap.start_us, gap.end_us))
            .collect();
        assert_eq!(
            ranges,
            vec![(4_120_000, 4_875_000)],
            "PC sounds between its words are protected just like PC speech"
        );
        assert!(combined.suggestions.iter().all(|gap| !gap.selected));
        let single = detect_transcript_pauses_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            None,
            config.clone(),
        )
        .unwrap();
        assert_eq!(single.suggestions.len(), 3);
        let all = detect_transcript_pauses_impl(
            &state,
            original.project_handle.clone(),
            ALL_SPEECH.into(),
            None,
            config.clone(),
        )
        .unwrap();
        assert_eq!(single.suggestions, all.suggestions);
        let pc_key = keys.iter().find(|key| *key != &mic_key).unwrap().clone();
        let pc_saved = store::load_transcript(&folder, &pc_key).unwrap().unwrap();
        let pc_ctx = {
            let opened = state.opened_project.lock();
            sound_context(opened.as_ref().unwrap(), &pc_key, false).unwrap()
        };
        let pc_path =
            crate::project::reader::safe_path(&pc_ctx.root, &pc_ctx.segments[0].relative_path)
                .unwrap();
        let original_pcm = fs::read(&pc_path).unwrap();
        // Speech, music/video audio without words, and a very quiet one-frame click
        // all veto microphone candidates. Neither a missing nor empty PC transcript
        // can turn those sounds into cuttable gaps.
        let sound_spans = [
            (1_400_000, 1_600_000),
            (4_000_000, 4_500_000),
            (6_750_000, 6_750_125),
        ];
        let pcm: Vec<i16> = (0..rate * 8)
            .map(|i| {
                let at = u64::from(i) * 1_000_000 / u64::from(rate);
                if (6_750_000..6_750_125).contains(&at) {
                    4
                } else if sound_spans[..2].iter().any(|&(a, b)| at >= a && at < b) {
                    ((i as f32 / rate as f32 * 440.0 * std::f32::consts::TAU).sin() * 1000.0) as i16
                } else {
                    0
                }
            })
            .collect();
        fs::write(&pc_path, generate_pcm16_wav(rate, 1, &pcm)).unwrap();
        store::delete_transcript(&folder, &pc_key).unwrap();
        let protected = detect_transcript_pauses_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            Some(keys.clone()),
            config.clone(),
        )
        .unwrap();
        assert!(
            !protected.suggestions.is_empty(),
            "Quiet PC gaps are still removable"
        );
        assert_eq!(
            protected
                .transcript_dependencies
                .iter()
                .find(|d| d.track_id == pc_key)
                .unwrap()
                .word_stamp,
            None
        );
        for cut in &protected.suggestions {
            assert!(
                sound_spans
                    .iter()
                    .all(|&(a, b)| cut.end_us <= a.saturating_sub(120_000)
                        || cut.start_us >= b + 120_000),
                "{cut:?}"
            );
            assert!(single
                .suggestions
                .iter()
                .any(|s| cut.start_us >= s.start_us && cut.end_us <= s.end_us));
        }
        let empty_pc = Transcript::new(
            pc_key.clone(),
            ProviderKind::Parakeet,
            "test".into(),
            None,
            vec![],
        );
        // Zero microphone margins and audio refinement must never weaken PC protection.
        let before_words = store::load_transcript(&folder, &mic_key).unwrap().unwrap();
        let aggressive = detect_non_speech_gaps_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            Some(keys.clone()),
            pauses::TranscriptGapConfig {
                min_duration_ms: 20,
                padding_ms: 0,
                refine_word_edges: true,
                edge_threshold_db: -42.0,
            },
        )
        .unwrap();
        assert!(!aggressive.suggestions.is_empty());
        assert!(aggressive.suggestions.iter().all(|cut| sound_spans
            .iter()
            .all(|&(a, b)| cut.end_us <= a.saturating_sub(80_000) || cut.start_us >= b + 80_000)));
        assert_eq!(
            store::load_transcript(&folder, &mic_key).unwrap().unwrap(),
            before_words
        );
        store::save_transcript(&folder, &empty_pc).unwrap();
        let empty_guard = detect_transcript_pauses_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            Some(keys.clone()),
            config.clone(),
        )
        .unwrap();
        assert_eq!(empty_guard.suggestions, protected.suggestions);
        assert!(
            validate_analysis(
                &state,
                &original.project_handle,
                original.revision,
                &protected.transcript_dependencies
            )
            .is_err(),
            "Adding a PC transcript invalidates a scan that had none"
        );
        // A saved PC word adds protection even if its PCM happens to be near zero.
        let quiet_pc_word = Transcript::new(
            pc_key.clone(),
            ProviderKind::Parakeet,
            "test".into(),
            None,
            vec![test_word("quiet PC voice", 2100, 2200)],
        );
        store::save_transcript(&folder, &quiet_pc_word).unwrap();
        let quiet_guard = detect_transcript_pauses_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            Some(keys.clone()),
            config.clone(),
        )
        .unwrap();
        assert!(quiet_guard
            .suggestions
            .iter()
            .all(|s| s.end_us <= 1_980_000 || s.start_us >= 2_320_000));
        fs::write(&pc_path, original_pcm).unwrap();
        store::save_transcript(&folder, &pc_saved).unwrap();
        let legacy = detect_silence_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            SilenceConfig {
                auto_level: None,
                threshold_db: -60.0,
                ..config.clone()
            },
        )
        .unwrap();
        assert_eq!(
            legacy.suggestions.len(),
            1,
            "Only the real silence belongs to the silence pass"
        );
        assert!(legacy.suggestions[0].start_us >= 4_000_000);
        assert!(legacy.suggestions[0].end_us <= 5_000_000);
        // A transcript can change without advancing the timeline revision. Both completed
        // scans and applying their results must reject changed, regenerated, or deleted words.
        let saved = store::load_transcript(&folder, &mic_key).unwrap().unwrap();
        for change in 0..4 {
            let mut changed = saved.clone();
            match change {
                0 => changed.words[1].source_start_us -= 100_000,
                1 => changed.words[1].text = "new word".into(),
                2 => changed.words.push(test_word("missed", 4100, 4500)),
                _ => {}
            }
            if change == 3 {
                store::delete_transcript(&folder, &mic_key).unwrap();
            } else {
                changed.words.sort_by_key(|w| w.source_start_us);
                store::save_transcript(&folder, &changed).unwrap();
            }
            assert!(validate_analysis(
                &state,
                &original.project_handle,
                original.revision,
                &combined.transcript_dependencies
            )
            .unwrap_err()
            .contains("transcript changed"));
            assert!(apply_jump_cuts_impl(
                &state,
                original.project_handle.clone(),
                original.revision,
                vec![crate::zoom::EditedRange {
                    start_us: ranges[0].0,
                    end_us: ranges[0].1
                }],
                combined.transcript_dependencies.clone()
            )
            .unwrap_err()
            .contains("transcript changed"));
            assert_eq!(
                state
                    .opened_project
                    .lock()
                    .as_ref()
                    .unwrap()
                    .document()
                    .revision,
                original.revision
            );
            store::save_transcript(&folder, &saved).unwrap();
        }
        // Captions and rejected filler suggestions do not change word protection.
        let mut dismissed = saved.clone();
        dismissed
            .set_dismissed(&["filler-example".into()], true)
            .unwrap();
        assert_eq!(saved.dependency(), dismissed.dependency());
        // Optional transcription stays optional, but creating it invalidates an older scan.
        store::delete_transcript(&folder, &mic_key).unwrap();
        let without_words = detect_silence_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            config.clone(),
        )
        .unwrap();
        assert_eq!(without_words.transcript_dependencies[0].word_stamp, None);
        store::save_transcript(&folder, &saved).unwrap();
        assert!(validate_analysis(
            &state,
            &original.project_handle,
            original.revision,
            &without_words.transcript_dependencies
        )
        .is_err());
        assert!(detect_transcript_pauses_impl(
            &state,
            "stale".into(),
            mic_key.clone(),
            None,
            config.clone()
        )
        .is_err());
        assert!(detect_transcript_pauses_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            Some(vec![mic_key.clone(); 2]),
            config.clone()
        )
        .is_err());
        let removed = ranges[0].1 - ranges[0].0;
        let applied = project_sequence_edit_impl(
            &state,
            original.project_handle.clone(),
            original.revision,
            SequenceEdit::DeleteRange {
                ranges: vec![crate::zoom::EditedRange {
                    start_us: ranges[0].0,
                    end_us: ranges[0].1,
                }],
                ripple: Some(true),
            },
            None,
        )
        .unwrap();
        let after = detect_transcript_pauses_impl(
            &state,
            original.project_handle.clone(),
            mic_key.clone(),
            Some(keys.clone()),
            config.clone(),
        )
        .unwrap();
        assert_eq!(
            after
                .suggestions
                .iter()
                .map(|gap| (gap.start_us, gap.end_us))
                .collect::<Vec<_>>(),
            ranges[1..]
                .iter()
                .map(|&(a, b)| (a - removed, b - removed))
                .collect::<Vec<_>>()
        );
        let reopened = ProjectReader::open(&folder).unwrap();
        assert_eq!(reopened.document().sequence, applied.sequence);
        let restored = project_undo_impl(
            &state,
            original.project_handle.clone(),
            applied.revision,
            None,
        )
        .unwrap();
        assert_eq!(restored.sequence, original.sequence);
        // The same two sources work when any pass is skipped or the order changes. Each
        // result is computed against the current timeline and each pass has its own undo.
        for order in [
            vec![0, 1, 2],
            vec![1, 0, 2],
            vec![2, 0, 1],
            vec![1],
            vec![0],
            vec![2],
        ] {
            let mut revisions = Vec::new();
            for pass in order {
                let revision = state
                    .opened_project
                    .lock()
                    .as_ref()
                    .unwrap()
                    .document()
                    .revision;
                let result = if pass == 2 {
                    let transcript = store::load_transcript(&folder, &mic_key).unwrap().unwrap();
                    let id = transcript
                        .words
                        .iter()
                        .find(|w| w.text == "um")
                        .unwrap()
                        .id
                        .clone();
                    Some(
                        super::super::transcript::transcript_cut_words_checked_impl(
                            &state,
                            original.project_handle.clone(),
                            revision,
                            mic_key.clone(),
                            vec![id],
                            transcript.dependency().word_stamp,
                        )
                        .unwrap(),
                    )
                } else {
                    let scan = if pass == 0 {
                        super::super::detect_silence_sources_impl(
                            &state,
                            original.project_handle.clone(),
                            keys.clone(),
                            config.clone(),
                        )
                        .unwrap()
                    } else {
                        detect_transcript_pauses_impl(
                            &state,
                            original.project_handle.clone(),
                            mic_key.clone(),
                            Some(keys.clone()),
                            config.clone(),
                        )
                        .unwrap()
                    };
                    if scan.suggestions.is_empty() {
                        None
                    } else {
                        Some(
                            apply_jump_cuts_impl(
                                &state,
                                original.project_handle.clone(),
                                revision,
                                scan.suggestions
                                    .iter()
                                    .map(|s| crate::zoom::EditedRange {
                                        start_us: s.start_us,
                                        end_us: s.end_us,
                                    })
                                    .collect(),
                                scan.transcript_dependencies,
                            )
                            .unwrap(),
                        )
                    }
                };
                if let Some(result) = result {
                    assert!(result.revision > revision);
                    revisions.push(result.revision);
                    // Applying another pass makes every earlier scan's revision stale.
                    assert!(validate_analysis(
                        &state,
                        &original.project_handle,
                        original.revision,
                        &combined.transcript_dependencies
                    )
                    .is_err());
                }
            }
            assert!(!revisions.is_empty());
            for _ in revisions {
                let revision = state
                    .opened_project
                    .lock()
                    .as_ref()
                    .unwrap()
                    .document()
                    .revision;
                project_undo_impl(&state, original.project_handle.clone(), revision, None).unwrap();
            }
            assert_eq!(
                state
                    .opened_project
                    .lock()
                    .as_ref()
                    .unwrap()
                    .document()
                    .sequence,
                original.sequence
            );
        }

        // Full command path: trimming/repositioning a PC clip must retain its margins
        // beyond the timeline clip, even when microphone padding is zero.
        let reader = state.opened_project.lock().take().unwrap();
        let mut boundary_doc = reader.document().clone();
        drop(reader);
        let pc_track = boundary_doc
            .sequence
            .tracks
            .iter()
            .position(|track| track.clips.iter().any(|clip| clip.source().key() == pc_key))
            .unwrap();
        let clip = &mut boundary_doc.sequence.tracks[pc_track].clips[0];
        clip.start_us = 4_000_000;
        clip.in_us = 1_000_000;
        clip.duration_us = 1_000_000;
        clip.link = None;
        store::delete_transcript(&folder, &pc_key).unwrap();
        let detect_boundary = |document: &crate::project::revision::EditDocument, padding_ms| {
            state.opened_project.lock().take();
            crate::project::revision::save_edit_document(&folder, document).unwrap();
            let reader = ProjectReader::open(&folder).unwrap();
            let handle = reader.summary.project_handle.clone();
            *state.opened_project.lock() = Some(reader);
            detect_non_speech_gaps_impl(
                &state,
                handle,
                mic_key.clone(),
                Some(keys.clone()),
                pauses::TranscriptGapConfig {
                    min_duration_ms: 20,
                    padding_ms,
                    refine_word_edges: true,
                    edge_threshold_db: -42.0,
                },
            )
            .unwrap()
            .suggestions
            .into_iter()
            .map(|cut| (cut.start_us, cut.end_us))
            .collect::<Vec<_>>()
        };
        let cuts = detect_boundary(&boundary_doc, 0);
        assert!(cuts.contains(&(3_500_000, 3_920_000)), "{cuts:?}");
        assert!(cuts.contains(&(5_080_000, 6_000_000)), "{cuts:?}");
        assert!(cuts.iter().all(|&(a, b)| b <= 3_920_000 || a >= 5_080_000));
        let cuts = detect_boundary(&boundary_doc, 120);
        assert!(cuts.iter().all(|&(a, b)| b <= 3_880_000 || a >= 5_120_000));

        let mut repeated = boundary_doc.clone();
        let mut overlap = repeated.sequence.tracks[pc_track].clone();
        overlap.id = "pc-repeat".into();
        overlap.clips[0].id = "pc-repeat-clip".into();
        overlap.clips[0].start_us = 4_500_000;
        repeated.sequence.tracks.push(overlap);
        let cuts = detect_boundary(&repeated, 0);
        assert!(cuts.contains(&(5_580_000, 6_000_000)), "{cuts:?}");
        assert!(cuts.iter().all(|&(a, b)| b <= 3_920_000 || a >= 5_580_000));

        // Quiet clipped footage must remain removable; unknown PCM needs the same
        // timeline margins as actual sound, including outside the clip.
        fs::write(
            &pc_path,
            generate_pcm16_wav(rate, 1, &vec![0; rate as usize * 8]),
        )
        .unwrap();
        assert!(detect_boundary(&boundary_doc, 0).contains(&(3_500_000, 6_000_000)));
        fs::remove_file(&pc_path).unwrap();
        let cuts = detect_boundary(&boundary_doc, 0);
        assert!(cuts.iter().all(|&(a, b)| b <= 3_920_000 || a >= 5_080_000));
        assert!(cuts.contains(&(5_080_000, 6_000_000)));
    }

    #[test]
    fn secondary_guard_only_vetoes_primary_gaps_and_respects_each_clip() {
        const S: u64 = 1_000_000;
        let mic = (vec![(2 * S, 8 * S)], vec![(3 * S, 6 * S)]);
        let pc = (vec![(0, 10 * S)], vec![(0, 4 * S), (5 * S, 10 * S)]);
        assert_eq!(
            guarded_primary_gaps(&[mic.clone()], &[pc.clone()], 1, 0),
            vec![(3 * S, 4 * S - 80_000), (5 * S + 80_000, 6 * S)]
        );
        // Repeated source material can sound over a quiet copy; each instance vetoes cuts.
        let overlap = (vec![(3 * S + S / 2, 5 * S + S / 2)], vec![]);
        assert_eq!(
            guarded_primary_gaps(&[mic.clone()], &[pc, overlap], 1, 0),
            vec![
                (3 * S, 3 * S + S / 2 - 80_000),
                (5 * S + S / 2 + 80_000, 6 * S)
            ]
        );
        // The PC need only be quiet while it is placed and audible. Unavailable PCM,
        // represented by a playing clip with no known quiet span, stays protected.
        assert_eq!(
            guarded_primary_gaps(&[mic.clone()], &[(vec![(4 * S, 5 * S)], vec![])], 1, 0),
            vec![(3 * S, 4 * S - 80_000), (5 * S + 80_000, 6 * S)]
        );
        assert_eq!(
            guarded_primary_gaps(&[mic], &[(vec![(0, 10 * S)], vec![])], 1, 0),
            vec![]
        );
    }

    #[test]
    fn secondary_guard_pads_sounds_once_and_keeps_quiet_clip_edges_cuttable() {
        let mic = (vec![(0, 10_000_000)], vec![(3_000_000, 6_000_000)]);
        let quiet_clip = (vec![(4_000_000, 5_000_000)], vec![(4_000_000, 5_000_000)]);
        assert_eq!(
            guarded_primary_gaps(&[mic.clone()], &[quiet_clip], 1, 0),
            vec![(3_000_000, 6_000_000)]
        );
        // Only the interior sound gets padding, not the silent clip boundaries.
        let sound = (
            vec![(4_000_000, 5_000_000)],
            vec![(4_000_000, 4_400_000), (4_600_000, 5_000_000)],
        );
        assert_eq!(
            guarded_primary_gaps(&[mic.clone()], &[sound.clone()], 1, 0),
            vec![(3_000_000, 4_320_000), (4_680_000, 6_000_000)]
        );
        assert_eq!(
            guarded_primary_gaps(&[mic], &[sound], 1, 120),
            vec![(3_000_000, 4_280_000), (4_720_000, 6_000_000)]
        );
        // Timeline start uses saturating padding; short remaining slivers are filtered.
        let mic = (vec![(0, 200_000)], vec![(0, 200_000)]);
        let pc = (vec![(0, 50_000)], vec![]);
        assert_eq!(
            guarded_primary_gaps(&[mic.clone()], &[pc.clone()], 1, 0),
            vec![(130_000, 200_000)]
        );
        assert!(guarded_primary_gaps(&[mic], &[pc], 80_000, 0).is_empty());
    }
}
