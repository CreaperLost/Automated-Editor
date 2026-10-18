//! Measures alignment on real projects; ignored by default. Downloads the model when missing.
//!
//! `AEROEDITS_ALIGN_PROBE=<project>[;<project>] cargo test --features parakeet-cuda --lib
//! align_probe -- --ignored --nocapture`
//!
//! For each project it aligns the microphone transcript (in memory; nothing is saved) and
//! prints how many words hold a stretch of quiet before and after. With two projects of the same
//! recording (say a Parakeet and an ElevenLabs transcript), it also prints how far apart their
//! shared words start and end, before and after both are aligned.
use super::*;
use crate::commands::sound_context;
use crate::project::reader::ProjectReader;
use crate::transcript::store;

/// A quiet stretch at least this long inside a word means the word swallowed a pause.
const SWALLOWED_US: u64 = 150_000;

struct Probed {
    name: String,
    before: Vec<TranscriptWord>,
    after: Vec<TranscriptWord>,
}

/// 10 ms frames of the track (source time of the first frame, sound or not).
fn sound_map(root: &Path, segments: &[SegmentSummary]) -> Vec<(u64, Vec<bool>)> {
    let mut runs: Vec<(u64, Vec<bool>)> = Vec::new();
    let cancel = AtomicBool::new(false);
    for_each_chunk(
        root,
        segments,
        PLAN,
        &cancel,
        |_| {},
        |chunk| {
            let levels = levels(&chunk.samples);
            let threshold = sound_threshold(&levels);
            runs.push((
                chunk.source_start_us,
                levels.iter().map(|&db| db >= threshold).collect(),
            ));
            Ok(())
        },
    )
    .unwrap();
    runs
}

fn swallowed(words: &[TranscriptWord], map: &[(u64, Vec<bool>)]) -> (usize, u64) {
    let frame_us = 10_000u64;
    let mut count = 0;
    let mut total = 0;
    for word in words.iter().filter(|w| w.kind == WordKind::Word) {
        let Some((start, frames)) = map
            .iter()
            .rev()
            .find(|(start, _)| *start <= word.source_start_us)
        else {
            continue;
        };
        let from = ((word.source_start_us - start) / frame_us) as usize;
        let to = (((word.source_end_us - start) / frame_us) as usize).min(frames.len());
        let mut run = 0u64;
        let mut longest = 0u64;
        for &loud in frames.get(from..to).unwrap_or(&[]) {
            run = if loud { 0 } else { run + frame_us };
            longest = longest.max(run);
        }
        if longest >= SWALLOWED_US {
            count += 1;
            total += longest;
        }
    }
    (count, total)
}

fn normalized(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Edge differences (ms) of words with the same text within 1 s of each other.
fn edge_gaps(a: &[TranscriptWord], b: &[TranscriptWord]) -> (Vec<f64>, Vec<f64>) {
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let mut j0 = 0;
    for word in a.iter().filter(|w| w.kind == WordKind::Word) {
        let text = normalized(&word.text);
        while j0 < b.len() && b[j0].source_start_us + 1_000_000 < word.source_start_us {
            j0 += 1;
        }
        let other = b[j0..]
            .iter()
            .take_while(|o| o.source_start_us <= word.source_start_us + 1_000_000)
            .filter(|o| o.kind == WordKind::Word && normalized(&o.text) == text)
            .min_by_key(|o| o.source_start_us.abs_diff(word.source_start_us));
        if let Some(other) = other {
            starts.push(word.source_start_us.abs_diff(other.source_start_us) as f64 / 1e3);
            ends.push(word.source_end_us.abs_diff(other.source_end_us) as f64 / 1e3);
        }
    }
    (starts, ends)
}

fn median(mut values: Vec<f64>) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn p90(mut values: Vec<f64>) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() * 9 / 10]
}

#[test]
#[ignore]
fn align_probe() {
    let projects = std::env::var("AEROEDITS_ALIGN_PROBE").expect("project paths");
    let model_dir = default_model_dir();
    if !model_present(&model_dir) {
        println!("Downloading the alignment model to {}", model_dir.display());
        download_model(&model_dir, &AtomicBool::new(false), &mut |_, _| {}).unwrap();
    }
    let (mut model, vocab) = load(&model_dir).unwrap();
    let mut probed = Vec::new();
    let mut scratches = Vec::new();
    for project in projects.split(';').filter(|p| !p.is_empty()) {
        // A copy of the edit and transcripts, so the real project is never touched.
        let scratch = tempfile::tempdir().unwrap();
        let source = Path::new(project);
        for name in ["project.json", "aeroedits.json"] {
            if source.join(name).is_file() {
                std::fs::copy(source.join(name), scratch.path().join(name)).unwrap();
            }
        }
        std::fs::create_dir(scratch.path().join("transcripts")).unwrap();
        for item in std::fs::read_dir(source.join("transcripts")).unwrap() {
            let item = item.unwrap();
            if item.path().extension().is_some_and(|ext| ext == "json") {
                std::fs::copy(
                    item.path(),
                    scratch.path().join("transcripts").join(item.file_name()),
                )
                .unwrap();
            }
        }
        let reader = ProjectReader::open(scratch.path()).unwrap();
        scratches.push(scratch);
        let doc = reader.document();
        let key = doc
            .assets
            .iter()
            .flat_map(|asset| {
                asset
                    .streams
                    .iter()
                    .filter(|s| s.role == crate::sequence::Role::Mic)
                    .map(|s| crate::sequence::StreamRef::new(&asset.id, &s.id).key())
            })
            .find(|key| {
                store::load_transcript(reader.root(), key)
                    .ok()
                    .flatten()
                    .is_some()
            })
            .expect("a transcribed microphone");
        let ctx = sound_context(&reader, &key, false).unwrap();
        let mut transcript = store::load_transcript(reader.root(), &key)
            .unwrap()
            .unwrap();
        let before = transcript.words.clone();
        let started = std::time::Instant::now();
        let info = align_transcript(
            &mut transcript,
            model.as_mut(),
            &vocab,
            &ctx.root,
            &ctx.segments,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        let elapsed = started.elapsed();
        let map = sound_map(&ctx.root, &ctx.segments);
        let (n0, t0) = swallowed(&before, &map);
        let (n1, t1) = swallowed(&transcript.words, &map);
        let words = before.iter().filter(|w| w.kind == WordKind::Word).count();
        let audio_s = ctx
            .segments
            .iter()
            .map(|s| s.end_us - s.start_us)
            .sum::<u64>() as f64
            / 1e6;
        println!(
            "\n{project}\n  {:?} {}, {words} words, {audio_s:.0} s of audio, aligned in {:.1} s \
             ({} aligned, {} kept)\n  words holding >=150 ms of quiet: {n0} ({:.1} s) -> {n1} \
             ({:.1} s)",
            transcript.provider,
            transcript.model,
            elapsed.as_secs_f64(),
            info.aligned_words,
            info.kept_words,
            t0 as f64 / 1e6,
            t1 as f64 / 1e6,
        );
        let mut moved: Vec<(u64, &TranscriptWord, &TranscriptWord)> = before
            .iter()
            .zip(&transcript.words)
            .map(|(a, b)| {
                let shift = a.source_start_us.abs_diff(b.source_start_us)
                    + a.source_end_us.abs_diff(b.source_end_us);
                (shift, a, b)
            })
            .collect();
        moved.sort_by_key(|m| std::cmp::Reverse(m.0));
        println!("  most moved:");
        for (_, a, b) in moved.iter().take(8) {
            println!(
                "    {:>16} {:>9.3}-{:<9.3} -> {:>9.3}-{:<9.3}",
                a.text,
                a.source_start_us as f64 / 1e6,
                a.source_end_us as f64 / 1e6,
                b.source_start_us as f64 / 1e6,
                b.source_end_us as f64 / 1e6
            );
        }
        probed.push(Probed {
            name: project.to_string(),
            before,
            after: transcript.words,
        });
    }
    if let [a, b] = &probed[..] {
        let (s0, e0) = edge_gaps(&a.before, &b.before);
        let (s1, e1) = edge_gaps(&a.after, &b.after);
        println!(
            "\n{} vs {}\n  shared words: {}\n  start gap median/p90: {:.0}/{:.0} ms -> {:.0}/{:.0} \
             ms\n  end gap median/p90: {:.0}/{:.0} ms -> {:.0}/{:.0} ms",
            a.name,
            b.name,
            s1.len(),
            median(s0.clone()),
            p90(s0),
            median(s1.clone()),
            p90(s1),
            median(e0.clone()),
            p90(e0),
            median(e1.clone()),
            p90(e1),
        );
    }
}
