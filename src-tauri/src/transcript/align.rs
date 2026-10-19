//! Word alignment: gives each transcript word the start and end the audio shows, with a CTC
//! character model (wav2vec2-base-960h, 20 ms frames) and forced alignment, the way WhisperX
//! times words. Transcribers time words loosely: Parakeet on an 80 ms grid and stretched over
//! the pauses beside them, ElevenLabs now and then over seconds of pause or noise.
//!
//! The words are known, so the model only has to find where their letters are. Each stretch of
//! speech is aligned on its own (a Viterbi path through the letters), then each word's edges are
//! snapped to where its sound starts and stops. Guardrails keep it from making anything worse:
//! a word only moves within 120 ms of where the transcriber put it, a word or stretch the model
//! is unsure of keeps its old times, and words stay in order without overlapping. Text and ids
//! never change, so everything that refers to words keeps working.
//!
//! The model runs with the `parakeet` feature (ONNX Runtime); download and status checks work
//! in every build.
use super::audio::{for_each_chunk, AudioChunk, ChunkPlan, ASR_SAMPLE_RATE};
use super::{Transcript, TranscriptWord, WordKind};
use crate::project::reader::SegmentSummary;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MODEL_NAME: &str = "wav2vec2-base-960h";
/// Bumped when the alignment rules change, so old transcripts can be told apart.
pub const ALIGN_VERSION: u32 = 1;
const MODEL_REPO: &str =
    "https://huggingface.co/onnx-community/wav2vec2-base-960h-ONNX/resolve/main";
/// Remote path, local name. Full-precision weights: quantized ones blur letter timing.
const MODEL_FILES: &[(&str, &str)] = &[
    ("vocab.json", "vocab.json"),
    ("onnx/model.onnx", "model.onnx"),
];
/// About 378 MB.
pub const MODEL_BYTES: u64 = 378_000_000;

/// One model frame: 320 samples at 16 kHz.
pub const FRAME_US: u64 = 20_000;
const FRAME_SAMPLES: usize = 320;
/// A new stretch of speech starts after a pause this long.
const UTTERANCE_GAP_US: u64 = 500_000;
/// The longest stretch aligned at once; longer speech is split between words.
const UTTERANCE_MAX_US: u64 = 20_000_000;
/// Audio around a stretch, so its first and last letters are not at the very edge.
const WINDOW_PAD_US: u64 = 300_000;
/// How far a word may start before, or end after, where the transcriber put it.
const MAX_GROW_US: u64 = 120_000;
/// Edge snapping: how far a word's end may run on into its sound, and its start back.
const SNAP_END_US: u64 = 250_000;
const SNAP_START_US: u64 = 150_000;
/// Loudness frames for edge snapping.
const LEVEL_FRAME_SAMPLES: usize = 160;
/// Sound is this far above the window's noise floor.
const SOUND_ABOVE_FLOOR_DB: f32 = 12.0;
const SOUND_MIN_DB: f32 = -62.0;
/// A stretch whose letters average less than this probability keeps its old times.
const MIN_UTTERANCE_SCORE: f32 = 0.25;
/// A word whose letters average less than this keeps its old times.
const MIN_WORD_SCORE: f32 = 0.08;
/// Audio read per chunk while aligning; long enough for any stretch plus its padding.
const PLAN: ChunkPlan = ChunkPlan {
    max_chunk_us: 60_000_000,
    search_window_us: 10_000_000,
};

/// What alignment did to a transcript, saved with it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlignmentInfo {
    pub model: String,
    pub version: u32,
    /// Words given new times from the audio.
    pub aligned_words: u32,
    /// Words that kept the transcriber's times (unsure, no letters to find, or no audio).
    pub kept_words: u32,
}

pub fn default_model_dir() -> PathBuf {
    super::parakeet::default_model_dir()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir)
        .join(MODEL_NAME)
}

pub fn model_present(dir: &Path) -> bool {
    MODEL_FILES.iter().all(|(_, name)| dir.join(name).is_file())
}

pub fn compiled_in() -> bool {
    cfg!(feature = "parakeet")
}

pub fn download_model(
    dir: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<(), String> {
    super::parakeet::download_files(dir, MODEL_REPO, MODEL_FILES, cancel, progress)
}

/// The model's letters: CTC blank, the gap between words, and characters.
#[derive(Clone, Debug)]
pub struct Vocab {
    pub size: usize,
    pub blank: usize,
    pub delimiter: Option<usize>,
    pub letters: HashMap<char, usize>,
}

impl Vocab {
    /// From a Hugging Face `vocab.json` (token to id).
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let map: HashMap<String, usize> =
            serde_json::from_slice(bytes).map_err(|e| format!("Bad aligner vocabulary: {e}"))?;
        let blank = *map
            .get("<pad>")
            .ok_or("The aligner vocabulary has no <pad> token")?;
        let letters = map
            .iter()
            .filter_map(|(token, &id)| {
                let mut chars = token.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) if c != '|' => Some((c, id)),
                    _ => None,
                }
            })
            .collect();
        Ok(Self {
            size: map.values().max().map_or(0, |m| m + 1),
            blank,
            delimiter: map.get("|").copied(),
            letters,
        })
    }

    /// The letters of `text` the model can find, or `None` when the word cannot be spelled
    /// in them (digits, symbols, letters outside the model's alphabet): then it is found as
    /// "any sound" between its neighbours.
    fn spell(&self, text: &str) -> Option<Vec<usize>> {
        let mut ids = Vec::new();
        for c in text.chars() {
            let c = match c {
                '’' | '‘' | '`' => '\'',
                c => c.to_ascii_uppercase(),
            };
            match self.letters.get(&c) {
                Some(&id) => ids.push(id),
                None if c.is_alphanumeric() => return None,
                // Punctuation is not spoken.
                None => {}
            }
        }
        (!ids.is_empty()).then_some(ids)
    }
}

/// Log-probabilities per frame from the model.
pub struct Emissions {
    pub frames: usize,
    pub vocab: usize,
    /// Row-major, `frames` x `vocab`, log-softmaxed.
    pub log_probs: Vec<f32>,
}

impl Emissions {
    fn at(&self, frame: usize, token: usize) -> f32 {
        self.log_probs[frame * self.vocab + token]
    }
}

/// Something that turns 16 kHz mono audio into [`Emissions`]: the ONNX model, or a stand-in in
/// tests.
pub trait EmissionModel {
    fn emissions(&mut self, samples: &[f32]) -> Result<Emissions, String>;
}

/// One thing to find in the audio: a letter, the gap between words, or any sound (a word that
/// cannot be spelled).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Target {
    Token(usize),
    Any,
}

/// Where each target sits on the best path: (first frame, frame after the last, mean probability).
type Span = (usize, usize, f32);

/// The most likely CTC path through `targets`, with blanks allowed between and around them.
/// `None` when the targets cannot fit in the frames.
fn viterbi(emissions: &Emissions, blank: usize, targets: &[Target]) -> Option<Vec<Span>> {
    let frames = emissions.frames;
    if targets.is_empty() || frames == 0 || targets.len() > frames {
        return None;
    }
    // "Any sound": the likeliest non-blank token at each frame.
    let any: Vec<f32> = (0..frames)
        .map(|t| {
            (0..emissions.vocab)
                .filter(|&v| v != blank)
                .map(|v| emissions.at(t, v))
                .fold(f32::NEG_INFINITY, f32::max)
        })
        .collect();
    let emit = |t: usize, s: usize| -> f32 {
        if s % 2 == 0 {
            emissions.at(t, blank)
        } else {
            match targets[s / 2] {
                Target::Token(id) => emissions.at(t, id),
                Target::Any => any[t],
            }
        }
    };
    let states = targets.len() * 2 + 1;
    // A skip over a blank is allowed unless it joins two equal letters.
    let can_skip = |s: usize| -> bool {
        s % 2 == 1 && s >= 3 && {
            let (a, b) = (targets[s / 2], targets[s / 2 - 1]);
            a != b || a == Target::Any
        }
    };
    let mut score = vec![f32::NEG_INFINITY; states];
    let mut back = vec![0u8; frames * states];
    score[0] = emit(0, 0);
    score[1] = emit(0, 1);
    let mut next = vec![f32::NEG_INFINITY; states];
    for t in 1..frames {
        // States beyond this many can't be reached yet, nor can the end be from behind these.
        let lo = states.saturating_sub(2 * (frames - t) + 1);
        let hi = (2 * t + 2).min(states);
        next.iter_mut().for_each(|v| *v = f32::NEG_INFINITY);
        for s in lo..hi {
            let mut best = score[s];
            let mut from = 0u8;
            if s >= 1 && score[s - 1] > best {
                best = score[s - 1];
                from = 1;
            }
            if can_skip(s) && score[s - 2] > best {
                best = score[s - 2];
                from = 2;
            }
            if best > f32::NEG_INFINITY {
                next[s] = best + emit(t, s);
                back[t * states + s] = from;
            }
        }
        std::mem::swap(&mut score, &mut next);
    }
    let last = states - 1;
    let mut s = if score[last] >= score[last - 1] {
        last
    } else {
        last - 1
    };
    if !score[s].is_finite() {
        return None;
    }
    let mut path = vec![0usize; frames];
    for t in (0..frames).rev() {
        path[t] = s;
        if t > 0 {
            s -= back[t * states + s] as usize;
        }
    }
    let mut spans: Vec<Option<(usize, usize, f32)>> = vec![None; targets.len()];
    for (t, &s) in path.iter().enumerate() {
        if s % 2 == 1 {
            let k = s / 2;
            let p = emit(t, s).exp();
            let span = spans[k].get_or_insert((t, t, 0.0));
            span.1 = t + 1;
            span.2 += p;
        }
    }
    spans
        .into_iter()
        .map(|span| span.map(|(a, b, sum)| (a, b, sum / (b - a) as f32)))
        .collect()
}

/// Loudness of 10 ms frames, in dBFS.
fn levels(samples: &[f32]) -> Vec<f32> {
    samples
        .chunks(LEVEL_FRAME_SAMPLES)
        .map(|frame| {
            let power = frame.iter().map(|s| s * s).sum::<f32>() / frame.len().max(1) as f32;
            10.0 * power.max(1e-12).log10()
        })
        .collect()
}

/// The level above which a 10 ms frame is sound: a margin over the quiet tenth of the window.
fn sound_threshold(levels: &[f32]) -> f32 {
    if levels.is_empty() {
        return SOUND_MIN_DB;
    }
    let mut sorted = levels.to_vec();
    sorted.sort_by(f32::total_cmp);
    let floor = sorted[sorted.len() / 10];
    (floor + SOUND_ABOVE_FLOOR_DB).max(SOUND_MIN_DB)
}

/// One stretch of speech: the indexes of its words in the transcript.
struct Utterance {
    words: Vec<usize>,
    start_us: u64,
    end_us: u64,
}

/// Words split into stretches at pauses, and at most [`UTTERANCE_MAX_US`] each. Sounds the
/// transcriber tagged (`[typing]`) are left out: they keep their times.
fn utterances(words: &[TranscriptWord]) -> Vec<Utterance> {
    let mut out: Vec<Utterance> = Vec::new();
    for (i, word) in words.iter().enumerate() {
        if word.kind != WordKind::Word {
            continue;
        }
        match out.last_mut() {
            Some(u)
                if word.source_start_us <= u.end_us + UTTERANCE_GAP_US
                    && word.source_end_us <= u.start_us + UTTERANCE_MAX_US =>
            {
                u.words.push(i);
                u.end_us = u.end_us.max(word.source_end_us);
            }
            _ => out.push(Utterance {
                words: vec![i],
                start_us: word.source_start_us,
                end_us: word.source_end_us,
            }),
        }
    }
    out
}

/// New times for the words of one stretch, from audio starting at `audio_start_us`.
/// `None` for a word that keeps its old times.
fn align_utterance(
    model: &mut dyn EmissionModel,
    vocab: &Vocab,
    words: &[&TranscriptWord],
    samples: &[f32],
    audio_start_us: u64,
) -> Result<Vec<Option<(u64, u64)>>, String> {
    let kept = vec![None; words.len()];
    if samples.len() < FRAME_SAMPLES * 2 {
        return Ok(kept);
    }
    let emissions = model.emissions(samples)?;
    if emissions.vocab < vocab.size {
        return Err("The aligner model does not match its vocabulary".into());
    }
    // Letters, with the word gap between words; each word's range of targets.
    let mut targets = Vec::new();
    let mut ranges = Vec::with_capacity(words.len());
    let mut spelled = Vec::with_capacity(words.len());
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            if let Some(gap) = vocab.delimiter {
                targets.push(Target::Token(gap));
            }
        }
        let start = targets.len();
        match vocab.spell(&word.text) {
            Some(ids) => {
                targets.extend(ids.into_iter().map(Target::Token));
                spelled.push(true);
            }
            None => {
                targets.push(Target::Any);
                spelled.push(false);
            }
        }
        ranges.push(start..targets.len());
    }
    let Some(spans) = viterbi(&emissions, vocab.blank, &targets) else {
        return Ok(kept);
    };
    let letters: Vec<f32> = ranges
        .iter()
        .zip(&spelled)
        .filter(|(_, &s)| s)
        .flat_map(|(range, _)| spans[range.clone()].iter().map(|s| s.2))
        .collect();
    if letters.is_empty()
        || letters.iter().sum::<f32>() / (letters.len() as f32) < MIN_UTTERANCE_SCORE
    {
        return Ok(kept);
    }

    let frame_us = |frame: usize| {
        audio_start_us + (frame as u64 * FRAME_SAMPLES as u64 * 1_000_000) / ASR_SAMPLE_RATE as u64
    };
    let mut found: Vec<Option<(u64, u64)>> = ranges
        .iter()
        .zip(&spelled)
        .map(|(range, &spelled)| {
            let word_spans = &spans[range.clone()];
            let score = word_spans.iter().map(|s| s.2).sum::<f32>() / word_spans.len() as f32;
            if spelled && score < MIN_WORD_SCORE {
                return None;
            }
            let first = word_spans.first()?.0;
            let last = word_spans.last()?.1;
            Some((frame_us(first), frame_us(last)))
        })
        .collect();

    // Edges: letters are found where the model is surest, often inside a sound's first or last
    // frames. Run each edge out to where the sound itself starts or stops, never into a
    // neighbour.
    let levels = levels(samples);
    let threshold = sound_threshold(&levels);
    let level_us = (LEVEL_FRAME_SAMPLES as u64 * 1_000_000) / ASR_SAMPLE_RATE as u64;
    let loud = |us: u64| -> bool {
        let index = (us.saturating_sub(audio_start_us) / level_us) as usize;
        levels.get(index).is_some_and(|&db| db >= threshold)
    };
    let audio_end_us = audio_start_us + level_us * levels.len() as u64;
    for i in 0..found.len() {
        let Some((start, end)) = found[i] else {
            continue;
        };
        let previous_end = found[..i]
            .iter()
            .rev()
            .find_map(|f| f.map(|f| f.1))
            .unwrap_or(audio_start_us);
        let next_start = found[i + 1..]
            .iter()
            .find_map(|f| f.map(|f| f.0))
            .unwrap_or(audio_end_us);
        let mut new_end = end;
        while new_end + level_us <= next_start.min(end + SNAP_END_US) && loud(new_end) {
            new_end += level_us;
        }
        let mut new_start = start;
        while new_start >= previous_end.max(start.saturating_sub(SNAP_START_US)) + level_us
            && loud(new_start - level_us)
        {
            new_start -= level_us;
        }
        found[i] = Some((new_start, new_end));
    }

    // A word that could not be spelled ("$20", "GPT-6.1") was found as any sound: enough to
    // keep its neighbours off its audio, too rough to time it. It keeps its old times.
    for (slot, &spelled) in found.iter_mut().zip(&spelled) {
        if !spelled {
            *slot = None;
        }
    }
    // Guardrails: close to where the transcriber heard the word, or its old times.
    for (slot, word) in found.iter_mut().zip(words) {
        if let Some((start, end)) = *slot {
            let start = start.max(word.source_start_us.saturating_sub(MAX_GROW_US));
            let end = end.min(word.source_end_us + MAX_GROW_US);
            *slot = (end > start).then_some((start, end));
        }
    }
    Ok(found)
}

/// Aligns `transcript`'s words to the audio of its track, read from `segments` under `root`.
/// Words keep their old times wherever alignment is unsure. Progress is 0-1.
pub fn align_transcript(
    transcript: &mut Transcript,
    model: &mut dyn EmissionModel,
    vocab: &Vocab,
    root: &Path,
    segments: &[SegmentSummary],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f64),
) -> Result<AlignmentInfo, String> {
    let plan = utterances(&transcript.words);
    let total = plan.len().max(1) as f64;
    let mut new_times: Vec<Option<(u64, u64)>> = vec![None; transcript.words.len()];
    // Audio kept from the first stretch not yet aligned onwards, contiguous.
    let mut buffer: Option<(u64, Vec<f32>)> = None;
    let mut next = 0usize;
    let samples_to_us = |n: usize| (n as u64 * 1_000_000) / ASR_SAMPLE_RATE as u64;
    let us_to_samples = |us: u64| (us as u128 * ASR_SAMPLE_RATE as u128 / 1_000_000) as usize;

    // Aligns every stretch the buffer holds in full, and those starting before `flush_before`
    // with whatever audio there is (the recording ends, or skips ahead, after them).
    let mut run = |buffer: &mut Option<(u64, Vec<f32>)>,
                   next: &mut usize,
                   flush_before: u64|
     -> Result<(), String> {
        while let Some(utterance) = plan.get(*next) {
            if cancel.load(Ordering::Relaxed) {
                return Err("Alignment cancelled".into());
            }
            let flushing = utterance.start_us < flush_before;
            let Some((buffer_start, samples)) = buffer.as_ref() else {
                if flushing {
                    *next += 1;
                    continue;
                }
                return Ok(());
            };
            let buffer_end = buffer_start + samples_to_us(samples.len());
            let want_start = utterance.start_us.saturating_sub(WINDOW_PAD_US);
            let want_end = utterance.end_us + WINDOW_PAD_US;
            if want_end > buffer_end && !flushing {
                return Ok(());
            }
            // Audio for it was skipped (a gap in the recording): it keeps its times.
            if utterance.start_us < *buffer_start || utterance.end_us > buffer_end {
                *next += 1;
                continue;
            }
            let from = us_to_samples(want_start.max(*buffer_start) - buffer_start);
            let to = us_to_samples(want_end.min(buffer_end) - buffer_start).min(samples.len());
            let words: Vec<&TranscriptWord> = utterance
                .words
                .iter()
                .map(|&i| &transcript.words[i])
                .collect();
            let audio_start = buffer_start + samples_to_us(from);
            let found = align_utterance(model, vocab, &words, &samples[from..to], audio_start)?;
            for (&i, times) in utterance.words.iter().zip(found) {
                new_times[i] = times;
            }
            *next += 1;
            progress(*next as f64 / total);
            // Drop audio no later stretch needs.
            if let (Some(following), Some((start, samples))) = (plan.get(*next), buffer.as_mut()) {
                let keep_from = following.start_us.saturating_sub(WINDOW_PAD_US);
                if keep_from > *start {
                    let drop = us_to_samples(keep_from - *start).min(samples.len());
                    samples.drain(..drop);
                    *start += samples_to_us(drop);
                }
            }
        }
        Ok(())
    };

    for_each_chunk(
        root,
        segments,
        PLAN,
        cancel,
        |_| {},
        |chunk: AudioChunk| {
            let contiguous = buffer.as_ref().is_some_and(|(start, samples)| {
                let end = start + samples_to_us(samples.len());
                end.abs_diff(chunk.source_start_us) <= 1_000
            });
            if contiguous {
                buffer.as_mut().unwrap().1.extend_from_slice(&chunk.samples);
            } else {
                // The recording skips ahead: finish what came before with the audio there is.
                run(&mut buffer, &mut next, chunk.source_start_us)?;
                buffer = Some((chunk.source_start_us, chunk.samples));
            }
            run(&mut buffer, &mut next, 0)
        },
    )?;
    run(&mut buffer, &mut next, u64::MAX)?;

    Ok(apply(transcript, &new_times))
}

/// Writes the new times, keeping words in order and apart: a word that kept its old times is
/// trimmed to the room its aligned neighbours leave.
fn apply(transcript: &mut Transcript, new_times: &[Option<(u64, u64)>]) -> AlignmentInfo {
    let mut aligned = 0u32;
    let mut kept = 0u32;
    let len = transcript.words.len();
    for i in 0..len {
        if transcript.words[i].kind != WordKind::Word {
            continue;
        }
        match new_times[i] {
            Some((start, end)) => {
                transcript.words[i].source_start_us = start;
                transcript.words[i].source_end_us = end;
                aligned += 1;
            }
            None => kept += 1,
        }
    }
    // A kept word between aligned ones must not overlap them.
    for i in 0..len {
        if new_times[i].is_some() || transcript.words[i].kind != WordKind::Word {
            continue;
        }
        let previous_end = (0..i)
            .rev()
            .find(|&j| new_times[j].is_some())
            .map(|j| transcript.words[j].source_end_us);
        let next_start = (i + 1..len)
            .find(|&j| new_times[j].is_some())
            .map(|j| transcript.words[j].source_start_us);
        let word = &mut transcript.words[i];
        if let Some(previous_end) = previous_end {
            word.source_start_us = word.source_start_us.max(previous_end);
        }
        if let Some(next_start) = next_start {
            word.source_end_us = word.source_end_us.min(next_start);
        }
        word.source_end_us = word.source_end_us.max(word.source_start_us);
    }
    // Stretches are aligned one at a time, in windows that overlap, so the last word of one can
    // land after the first word of the next (or after a tagged sound that kept its times). No
    // word starts before the one ahead of it.
    let mut previous_start = 0u64;
    for word in &mut transcript.words {
        word.source_start_us = word.source_start_us.max(previous_start);
        word.source_end_us = word.source_end_us.max(word.source_start_us);
        previous_start = word.source_start_us;
    }
    let info = AlignmentInfo {
        model: MODEL_NAME.into(),
        version: ALIGN_VERSION,
        aligned_words: aligned,
        kept_words: kept,
    };
    transcript.alignment = Some(info.clone());
    info
}

#[cfg(feature = "parakeet")]
mod engine {
    use super::{EmissionModel, Emissions};
    use parking_lot::Mutex;
    use std::path::{Path, PathBuf};

    /// Loading takes a second or two, so the last model loaded is kept.
    static LOADED: Mutex<Option<(PathBuf, ort::session::Session)>> = Mutex::new(None);

    pub struct Wav2Vec2 {
        dir: PathBuf,
    }

    impl Wav2Vec2 {
        pub fn new(dir: &Path) -> Result<Self, String> {
            if !super::model_present(dir) {
                return Err(format!(
                    "The word alignment model is not downloaded yet (looked in {})",
                    dir.display()
                ));
            }
            let mut loaded = LOADED.lock();
            if loaded.as_ref().map(|(p, _)| p.as_path()) != Some(dir) {
                *loaded = None;
                let session = crate::transcript::parakeet::execution_config()
                    .build_session(&dir.join("model.onnx"))
                    .map_err(|e| format!("Could not load the word alignment model: {e}"))?;
                *loaded = Some((dir.to_path_buf(), session));
            }
            Ok(Self {
                dir: dir.to_path_buf(),
            })
        }
    }

    impl EmissionModel for Wav2Vec2 {
        fn emissions(&mut self, samples: &[f32]) -> Result<Emissions, String> {
            // The model was trained on audio scaled to zero mean and unit variance.
            let n = samples.len() as f32;
            let mean = samples.iter().sum::<f32>() / n;
            let var = samples.iter().map(|s| (s - mean) * (s - mean)).sum::<f32>() / n;
            let scale = 1.0 / (var + 1e-7).sqrt();
            let input: Vec<f32> = samples.iter().map(|s| (s - mean) * scale).collect();
            let tensor = ort::value::Tensor::from_array(([1usize, input.len()], input))
                .map_err(|e| e.to_string())?;
            let mut loaded = LOADED.lock();
            let (_, session) = loaded
                .as_mut()
                .filter(|(p, _)| *p == self.dir)
                .ok_or("The word alignment model was unloaded")?;
            let outputs = session
                .run(ort::inputs![tensor])
                .map_err(|e| format!("Word alignment failed: {e}"))?;
            let (shape, data) = outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| e.to_string())?;
            let dims: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
            let [1, frames, vocab] = dims[..] else {
                return Err(format!("Unexpected aligner output shape {dims:?}"));
            };
            let mut log_probs = data.to_vec();
            for row in log_probs.chunks_mut(vocab) {
                let max = row.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                let sum = row.iter().map(|v| (v - max).exp()).sum::<f32>().ln() + max;
                row.iter_mut().for_each(|v| *v -= sum);
            }
            Ok(Emissions {
                frames,
                vocab,
                log_probs,
            })
        }
    }
}

#[cfg(feature = "parakeet")]
pub use engine::Wav2Vec2;

/// The model and vocabulary in `dir`, ready to align.
pub fn load(dir: &Path) -> Result<(Box<dyn EmissionModel>, Vocab), String> {
    let vocab = Vocab::from_json(
        &std::fs::read(dir.join("vocab.json"))
            .map_err(|e| format!("Could not read the aligner vocabulary: {e}"))?,
    )?;
    #[cfg(feature = "parakeet")]
    {
        Ok((Box::new(Wav2Vec2::new(dir)?), vocab))
    }
    #[cfg(not(feature = "parakeet"))]
    {
        let _ = vocab;
        Err("This build of AeroEdits does not include word alignment".into())
    }
}

#[cfg(all(test, feature = "parakeet"))]
mod probe;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::ProviderKind;

    const VOCAB: &str = r#"{"<pad>": 0, "<s>": 1, "</s>": 2, "<unk>": 3, "|": 4, "E": 5, "T": 6, "A": 7, "O": 8, "N": 9, "I": 10, "H": 11, "S": 12, "R": 13, "D": 14, "L": 15, "U": 16, "M": 17, "W": 18, "C": 19, "F": 20, "G": 21, "Y": 22, "P": 23, "B": 24, "V": 25, "K": 26, "'": 27, "X": 28, "J": 29, "Q": 30, "Z": 31}"#;

    fn vocab() -> Vocab {
        Vocab::from_json(VOCAB.as_bytes()).unwrap()
    }

    /// A model that "hears" the given tokens at the given frames, blank elsewhere.
    struct Scripted {
        /// (frame, token).
        hits: Vec<(usize, usize)>,
    }

    impl EmissionModel for Scripted {
        fn emissions(&mut self, samples: &[f32]) -> Result<Emissions, String> {
            let frames = samples.len() / FRAME_SAMPLES;
            let vocab = 32;
            let low = (0.01f32 / 31.0).ln();
            let mut log_probs = vec![low; frames * vocab];
            for t in 0..frames {
                let hit = self.hits.iter().find(|h| h.0 == t).map(|h| h.1);
                let token = hit.unwrap_or(0);
                log_probs[t * vocab + token] = 0.99f32.ln();
            }
            Ok(Emissions {
                frames,
                vocab,
                log_probs,
            })
        }
    }

    fn word(text: &str, start_ms: u64, end_ms: u64) -> TranscriptWord {
        crate::transcript::test_word(text, start_ms, end_ms)
    }

    #[test]
    fn spelling_keeps_letters_and_apostrophes_and_gives_up_on_digits() {
        let v = vocab();
        assert_eq!(v.blank, 0);
        assert_eq!(v.delimiter, Some(4));
        assert_eq!(
            v.spell("Don’t,").unwrap(),
            v.spell("DONT")
                .map(|mut s| {
                    s.insert(3, 27);
                    s
                })
                .unwrap()
        );
        assert_eq!(v.spell("GPT-4"), None);
        assert_eq!(v.spell("café"), None);
        assert_eq!(v.spell("..."), None);
    }

    #[test]
    fn viterbi_puts_each_letter_where_it_is_heard() {
        let v = vocab();
        // "HI" heard at frames 10-11 and 13, in 30 frames.
        let mut model = Scripted {
            hits: vec![(10, 11), (11, 11), (13, 10)],
        };
        let em = model.emissions(&vec![0.0; 30 * FRAME_SAMPLES]).unwrap();
        let spans = viterbi(&em, v.blank, &[Target::Token(11), Target::Token(10)]).unwrap();
        assert_eq!((spans[0].0, spans[0].1), (10, 12));
        assert_eq!((spans[1].0, spans[1].1), (13, 14));
        assert!(spans[0].2 > 0.9);
        // Doubled letters need a blank between them: "LL" in 1 frame cannot fit.
        let em = Scripted { hits: vec![] }
            .emissions(&vec![0.0; FRAME_SAMPLES * 2])
            .unwrap();
        assert!(viterbi(
            &em,
            v.blank,
            &[Target::Token(15), Target::Token(15), Target::Token(15)]
        )
        .is_none());
    }

    /// Words of a sentence heard at known frames, aligned inside a 2 s window.
    #[test]
    fn a_loose_word_is_pulled_in_to_its_letters() {
        let v = vocab();
        // "NO" at frames 20-21 (0.40-0.44 s), "SIR" at 60-62 (1.20-1.26 s), gap token at 40.
        let mut model = Scripted {
            hits: vec![(20, 9), (21, 8), (40, 4), (60, 12), (61, 10), (62, 13)],
        };
        // The transcriber stretched "NO" over the whole pause.
        let words = [word("no", 380, 1_150), word("sir.", 1_150, 1_300)];
        let refs: Vec<&TranscriptWord> = words.iter().collect();
        let samples = vec![0.0f32; 100 * FRAME_SAMPLES];
        let found = align_utterance(&mut model, &v, &refs, &samples, 0).unwrap();
        assert_eq!(found[0], Some((400_000, 440_000)));
        assert_eq!(found[1], Some((1_200_000, 1_260_000)));
    }

    #[test]
    fn edges_snap_out_to_the_sound_but_not_into_a_neighbour() {
        let v = vocab();
        let mut model = Scripted {
            hits: vec![(20, 9), (21, 8), (40, 4), (60, 12), (61, 10), (62, 13)],
        };
        let words = [word("no", 380, 600), word("sir", 1_150, 1_300)];
        let refs: Vec<&TranscriptWord> = words.iter().collect();
        // Sound from 0.38 s to 0.52 s, then quiet; "sir" sounds 1.19-1.32 s.
        let mut samples = vec![0.0f32; 100 * FRAME_SAMPLES];
        for (a, b) in [(380, 520), (1_190, 1_320)] {
            for s in &mut samples[a * 16..b * 16] {
                *s = 0.3;
            }
        }
        let found = align_utterance(&mut model, &v, &refs, &samples, 0).unwrap();
        assert_eq!(found[0], Some((380_000, 520_000)));
        assert_eq!(found[1], Some((1_190_000, 1_320_000)));
    }

    #[test]
    fn unsure_or_distant_words_keep_their_times() {
        let v = vocab();
        // The model hears nothing: the stretch keeps its times.
        let words = [word("hello", 0, 500)];
        let refs: Vec<&TranscriptWord> = words.iter().collect();
        let samples = vec![0.0f32; 50 * FRAME_SAMPLES];
        let found =
            align_utterance(&mut Scripted { hits: vec![] }, &v, &refs, &samples, 0).unwrap();
        assert_eq!(found, vec![None]);
        // Heard far from where the transcriber put it: clamped to 120 ms around the old span.
        let mut model = Scripted {
            hits: vec![(45, 9), (46, 8)],
        };
        let words = [word("no", 100, 300)];
        let refs: Vec<&TranscriptWord> = words.iter().collect();
        let found = align_utterance(&mut model, &v, &refs, &samples, 0).unwrap();
        assert_eq!(found, vec![None]);
    }

    #[test]
    fn a_word_that_cannot_be_spelled_keeps_its_times_and_its_neighbours_off_its_sound() {
        let v = vocab();
        // "NO 4 SIR": the digit is any sound at frames 30-33.
        let mut model = Scripted {
            hits: vec![
                (20, 9),
                (21, 8),
                (25, 4),
                (30, 28),
                (31, 28),
                (32, 28),
                (33, 28),
                (35, 4),
                (60, 12),
                (61, 10),
                (62, 13),
            ],
        };
        let words = [
            word("no", 400, 600),
            word("4", 620, 900),
            word("sir", 1_150, 1_300),
        ];
        let refs: Vec<&TranscriptWord> = words.iter().collect();
        // Sound up to 1.5 s: "no" would run on into the digit's sound, but stops where it
        // starts (0.6 s), and its start stops 120 ms before where the transcriber put it.
        let mut samples = vec![0.0f32; 100 * FRAME_SAMPLES];
        samples[..1_500 * 16].iter_mut().for_each(|s| *s = 0.3);
        let found = align_utterance(&mut model, &v, &refs, &samples, 0).unwrap();
        assert_eq!(found[1], None, "the digit keeps its old times");
        assert_eq!(found[0], Some((280_000, 600_000)));
        assert_eq!(found[2], Some((1_050_000, 1_420_000)));
    }

    #[test]
    fn kept_words_make_room_for_aligned_neighbours_and_info_is_saved() {
        let mut transcript = Transcript::new(
            "mic".into(),
            ProviderKind::Parakeet,
            "parakeet".into(),
            None,
            vec![
                word("a", 0, 500),
                word("b", 400, 900),
                word("c", 850, 1_200),
            ],
        );
        let info = apply(
            &mut transcript,
            &[Some((100_000, 300_000)), None, Some((950_000, 1_100_000))],
        );
        assert_eq!(info.aligned_words, 2);
        assert_eq!(info.kept_words, 1);
        let b = &transcript.words[1];
        assert_eq!((b.source_start_us, b.source_end_us), (400_000, 900_000));
        assert_eq!(transcript.alignment, Some(info));
        let mut overlapping = transcript.clone();
        apply(
            &mut overlapping,
            &[Some((100_000, 500_000)), None, Some((800_000, 1_100_000))],
        );
        let b = &overlapping.words[1];
        assert_eq!((b.source_start_us, b.source_end_us), (500_000, 800_000));
    }

    #[test]
    fn words_of_neighbouring_stretches_stay_in_time_order() {
        let mut typing = word("[typing]", 1_000, 1_100);
        typing.kind = WordKind::AudioEvent;
        let mut transcript = Transcript::new(
            "mic".into(),
            ProviderKind::Parakeet,
            "parakeet".into(),
            None,
            vec![
                word("end", 400, 600),
                word("start", 650, 900),
                typing,
                word("next", 1_150, 1_300),
            ],
        );
        // "end" was aligned in its stretch's padded window, past where "start" was aligned in
        // the next stretch; "next" grew back over the tagged sound.
        apply(
            &mut transcript,
            &[
                Some((700_000, 800_000)),
                Some((600_000, 900_000)),
                None,
                Some((1_030_000, 1_300_000)),
            ],
        );
        let times: Vec<_> = transcript
            .words
            .iter()
            .map(|w| (w.source_start_us, w.source_end_us))
            .collect();
        assert_eq!(
            times,
            vec![
                (700_000, 800_000),
                (700_000, 900_000),
                (1_000_000, 1_100_000),
                (1_030_000, 1_300_000)
            ]
        );
        transcript.validate().unwrap();
    }

    #[test]
    fn stretches_split_at_pauses_and_skip_tagged_sounds() {
        let mut typing = word("[typing]", 600, 900);
        typing.kind = WordKind::AudioEvent;
        let words = vec![
            word("one", 0, 300),
            word("two", 400, 700),
            typing,
            word("three", 1_400, 1_700),
        ];
        let plan = utterances(&words);
        assert_eq!(
            plan.iter().map(|u| u.words.clone()).collect::<Vec<_>>(),
            [vec![0, 1], vec![3]]
        );
    }
}
