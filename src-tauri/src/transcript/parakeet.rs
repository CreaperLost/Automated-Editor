//! Local English transcription with NVIDIA Parakeet through ONNX Runtime (`parakeet-rs`), in
//! one of two models:
//!
//! - **Unified** (default): Parakeet Unified EN 0.6B, an RNN-T. On FLEURS it makes 4.0% word
//!   errors where TDT v3 makes 4.8%, and it gets names and numbers right more often, but it
//!   smooths speech over: on a 15-minute tutorial it dropped "see see", "we can can" and six
//!   of nine "uh"s.
//! - **Verbatim**: Parakeet TDT 0.6B v3, multilingual and so a little weaker at English, but it
//!   writes stutters, repeats and filler sounds down, which the filler and retake passes cut.
//!
//! Greek is transcribed online (see `language`).
//!
//! Inference is compiled only with the `parakeet` cargo feature; the GPU features pick the
//! execution provider (`parakeet-cuda` or `parakeet-directml` on Windows, `parakeet-webgpu`
//! for Metal on macOS) and ONNX Runtime falls back to the CPU when the GPU is unavailable.
//! Model download and status checks work in every build. CUDA builds download full precision
//! (about 2.5 GB, fastest on the GPU); the others INT8 (about 0.7 GB, faster on a CPU).
use super::audio::ChunkPlan;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// A file of a model: its name and its SHA-256, as Hugging Face lists it.
pub type ModelFile = (&'static str, &'static str);

/// Which Parakeet transcribes English.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ParakeetModel {
    /// Parakeet Unified EN 0.6B: the most accurate words.
    #[default]
    Unified,
    /// Parakeet TDT 0.6B v3: keeps stutters, repeats and filler sounds.
    Verbatim,
}

const UNIFIED_REPO: &str = "https://huggingface.co/bobNight/parakeet-unified-en-0.6b-onnx/resolve/09e9060322d99c5f070010724786e6ee090fd51d";
const UNIFIED_FP32: &[ModelFile] = &[
    (
        "tokenizer.model",
        "07d4e5a63840a53ab2d4d106d2874768143fb3fbdd47938b3910d2da05bfb0a9",
    ),
    (
        "decoder_joint.onnx",
        "64648c91935ea4819e9c31ea8a20f011d3b1c960be3861cb8bc74df0259cd998",
    ),
    (
        "encoder.onnx",
        "50083dd0b2af503b87ea8fcf9fd7d7dc15dda3130e4113017c8b37ed00418ba4",
    ),
    (
        "encoder.onnx.data",
        "c054b2932ee13aa39ed84982a399822f3193311fb6fd8e1e204566722614bd87",
    ),
];
const UNIFIED_FP32_BYTES: u64 = 251_056 + 35_779_240 + 41_901_475 + 2_437_091_328;
const UNIFIED_INT8: &[ModelFile] = &[
    (
        "tokenizer.model",
        "07d4e5a63840a53ab2d4d106d2874768143fb3fbdd47938b3910d2da05bfb0a9",
    ),
    (
        "decoder_joint.int8.onnx",
        "7f76ad5f35035f25630075699c6c942a2c0c05ff42cb398f966f3c256d148e1e",
    ),
    (
        "encoder.int8.onnx",
        "c81adfab77634e00c1668a221a14f244c5fb3409e7c14eeebaf6ac963425910f",
    ),
    (
        "encoder.int8.onnx.data",
        "3d54dd04646c15677bd2844a84df3770b12cc1ce183481f7b6e0def31c92114a",
    ),
];
const UNIFIED_INT8_BYTES: u64 = 251_056 + 8_995_064 + 42_606_669 + 611_491_584;

const VERBATIM_REPO: &str = "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce";
const VERBATIM_FP32: &[ModelFile] = &[
    (
        "vocab.txt",
        "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
    ),
    (
        "decoder_joint-model.onnx",
        "e978ddf6688527182c10fde2eb4b83068421648985ef23f7a86be732be8706c1",
    ),
    (
        "encoder-model.onnx",
        "98a74b21b4cc0017c1e7030319a4a96f4a9506e50f0708f3a516d02a77c96bb1",
    ),
    (
        "encoder-model.onnx.data",
        "9a22d372c51455c34f13405da2520baefb7125bd16981397561423ed32d24f36",
    ),
];
const VERBATIM_FP32_BYTES: u64 = 93_939 + 72_520_893 + 41_770_866 + 2_435_420_160;
const VERBATIM_INT8: &[ModelFile] = &[
    (
        "vocab.txt",
        "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
    ),
    (
        "decoder_joint-model.int8.onnx",
        "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70",
    ),
    (
        "encoder-model.int8.onnx",
        "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09",
    ),
];
const VERBATIM_INT8_BYTES: u64 = 93_939 + 18_202_004 + 652_183_999;

impl ParakeetModel {
    /// The model's name, recorded with its transcripts and used as its folder name.
    pub fn name(self) -> &'static str {
        match self {
            ParakeetModel::Unified => "parakeet-unified-en-0.6b",
            ParakeetModel::Verbatim => "parakeet-tdt-0.6b-v3",
        }
    }

    /// The ONNX export, pinned to one commit so the files never change.
    fn repo(self) -> &'static str {
        match self {
            ParakeetModel::Unified => UNIFIED_REPO,
            ParakeetModel::Verbatim => VERBATIM_REPO,
        }
    }

    /// The files this build downloads.
    fn files(self) -> &'static [ModelFile] {
        match (self, cfg!(feature = "parakeet-cuda")) {
            (ParakeetModel::Unified, true) => UNIFIED_FP32,
            (ParakeetModel::Unified, false) => UNIFIED_INT8,
            (ParakeetModel::Verbatim, true) => VERBATIM_FP32,
            (ParakeetModel::Verbatim, false) => VERBATIM_INT8,
        }
    }

    /// Download size of this model in this build, in bytes.
    pub fn bytes(self) -> u64 {
        match (self, cfg!(feature = "parakeet-cuda")) {
            (ParakeetModel::Unified, true) => UNIFIED_FP32_BYTES,
            (ParakeetModel::Unified, false) => UNIFIED_INT8_BYTES,
            (ParakeetModel::Verbatim, true) => VERBATIM_FP32_BYTES,
            (ParakeetModel::Verbatim, false) => VERBATIM_INT8_BYTES,
        }
    }

    pub fn default_dir(self) -> PathBuf {
        models_dir().join(self.name())
    }

    /// The files of this build's precision are all in `dir`. INT8 files left from an older
    /// download do not count in a CUDA build: they would run mostly on the CPU (the v3 INT8
    /// model took 110 s for 15 minutes even with CUDA; full precision on the GPU, about 16 s).
    pub fn present(self, dir: &Path) -> bool {
        self.files()
            .iter()
            .all(|(name, _)| dir.join(name).is_file())
    }
}

/// Where downloaded models live, each in a folder of its own.
pub fn models_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("AeroEdits")
        .join("models")
}

/// Parakeet was trained on short utterances: given 90 s at once it went silent for over a
/// minute of clear speech (48 words where 30 s and 15 s chunks found 175). Chunks of 16-24 s,
/// split at the quietest moment, stay where it is reliable.
pub const PLAN: ChunkPlan = ChunkPlan {
    max_chunk_us: 24_000_000,
    search_window_us: 8_000_000,
};

/// The default model's folder. Other models' folders sit beside it.
pub fn default_model_dir() -> PathBuf {
    ParakeetModel::default().default_dir()
}

pub fn compiled_in() -> bool {
    cfg!(feature = "parakeet")
}

/// The execution provider this build asks ONNX Runtime for.
pub fn accelerator() -> &'static str {
    if cfg!(feature = "parakeet-cuda") {
        "CUDA"
    } else if cfg!(feature = "parakeet-directml") {
        "DirectML"
    } else if cfg!(feature = "parakeet-webgpu") {
        "WebGPU"
    } else if cfg!(feature = "parakeet") {
        "CPU"
    } else {
        "not included in this build"
    }
}

/// Downloads `model` into `dir` at this build's precision, reporting `(bytes_done,
/// bytes_total)` when known.
pub fn download_model(
    model: ParakeetModel,
    dir: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<(), String> {
    let files: Vec<(&str, &str, Option<&str>)> = model
        .files()
        .iter()
        .map(|(name, sha)| (*name, *name, Some(*sha)))
        .collect();
    download_files_checked(dir, model.repo(), &files, cancel, progress)
}

/// Downloads `files` (path under `repo`, name in `dir`) into `dir`, skipping files already
/// there, reporting `(bytes_done, bytes_total)` when known.
pub fn download_files(
    dir: &Path,
    repo: &str,
    files: &[(&str, &str)],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<(), String> {
    let files: Vec<(&str, &str, Option<&str>)> = files
        .iter()
        .map(|(remote, name)| (*remote, *name, None))
        .collect();
    download_files_checked(dir, repo, &files, cancel, progress)
}

/// [`download_files`], where a file given a SHA-256 is kept only if its bytes match it.
pub fn download_files_checked(
    dir: &Path,
    repo: &str,
    files: &[(&str, &str, Option<&str>)],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    // The blocking client's timeout bounds the wait for headers and each read on its own, not
    // the whole download: a stalled server fails (and frees Cancel) within a minute.
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let cancelled = || cancel.load(Ordering::Relaxed);
    let mut sizes = Vec::new();
    for (remote, _, _) in files {
        if cancelled() {
            return Err("Download cancelled".into());
        }
        let size = client
            .head(format!("{repo}/{remote}"))
            .send()
            .ok()
            .filter(|r| r.status().is_success())
            .and_then(|r| r.content_length());
        sizes.push(size);
    }
    let total = sizes.iter().copied().sum::<Option<u64>>();
    let mut done = 0u64;
    for (remote, name, sha256) in files {
        let target = dir.join(name);
        if target.is_file() {
            done += std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
            progress(done, total);
            continue;
        }
        if cancelled() {
            return Err("Download cancelled".into());
        }
        let mut response = client
            .get(format!("{repo}/{remote}"))
            .send()
            .map_err(|e| format!("Could not download {name}: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("Downloading {name} failed: {}", response.status()));
        }
        let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 1 << 20];
        loop {
            if cancelled() {
                return Err("Download cancelled".into());
            }
            let n = response
                .read(&mut buf)
                .map_err(|e| format!("Download of {name} was interrupted: {e}"))?;
            if n == 0 {
                break;
            }
            temp.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            hasher.update(&buf[..n]);
            done += n as u64;
            progress(done, total);
        }
        if let Some(expected) = sha256 {
            let got = format!("{:x}", hasher.finalize());
            if !got.eq_ignore_ascii_case(expected) {
                return Err(format!(
                    "{name} did not download correctly (its checksum is wrong); try again"
                ));
            }
        }
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(&target).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Keep the lexical token span: punctuation/whitespace can be timed well after speech.
/// Unlike the library's Words grouping, retain repeated words ("very very").
#[cfg(any(feature = "parakeet", test))]
fn group_word_tokens(tokens: Vec<super::TranscriptWord>) -> Vec<super::TranscriptWord> {
    let mut words: Vec<super::TranscriptWord> = Vec::new();
    let mut separated = true;
    let mut prefix = String::new();
    for mut token in tokens {
        let marked = token.text.starts_with('▁') || token.text.starts_with(char::is_whitespace);
        let text = token.text.trim_start_matches('▁').trim();
        if text.is_empty() {
            separated = true;
            continue;
        }
        if text.chars().all(|c| !c.is_alphanumeric()) {
            if let Some(word) = words.last_mut() {
                word.text.push_str(text);
            } else {
                prefix.push_str(text);
            }
            continue;
        }
        let suffix = text.starts_with(['\'', '’', '-']);
        if !words.is_empty() && !separated && (!marked || suffix) {
            let word = words.last_mut().unwrap();
            word.text.push_str(text);
            word.source_end_us = word.source_end_us.max(token.source_end_us);
        } else {
            token.text = format!("{prefix}{text}");
            prefix.clear();
            words.push(token);
        }
        separated = false;
    }
    words
}

#[cfg(feature = "parakeet")]
mod engine {
    use super::super::audio::{AudioChunk, ChunkPlan, ASR_SAMPLE_RATE};
    use super::super::provider::{ChunkResult, ChunkTranscriber, ProviderKind};
    use super::super::{seconds_to_us, TranscriptWord, WordKind};
    use super::ParakeetModel;
    use parakeet_rs::{
        ExecutionConfig, ExecutionProvider, ParakeetTDT, ParakeetUnified, TimestampMode,
        Transcriber,
    };
    use parking_lot::Mutex;
    use std::path::{Path, PathBuf};

    /// One of the two models, loaded.
    enum Engine {
        Unified(ParakeetUnified),
        Verbatim(ParakeetTDT),
    }

    impl Engine {
        fn transcribe(
            &mut self,
            samples: Vec<f32>,
        ) -> parakeet_rs::Result<parakeet_rs::TranscriptionResult> {
            let mode = Some(TimestampMode::Tokens);
            match self {
                Engine::Unified(m) => m.transcribe_samples(samples, ASR_SAMPLE_RATE, 1, mode),
                Engine::Verbatim(m) => m.transcribe_samples(samples, ASR_SAMPLE_RATE, 1, mode),
            }
        }
    }

    /// Loading a model takes seconds, so the last one loaded is kept for the next run.
    type Loaded = (PathBuf, ParakeetModel, Engine);
    static LOADED: Mutex<Option<Loaded>> = Mutex::new(None);

    /// The execution provider this build asks for.
    pub(crate) fn execution_config() -> ExecutionConfig {
        #[allow(unused_mut)]
        let mut config = ExecutionConfig::new();
        #[cfg(feature = "parakeet-cuda")]
        {
            // The CUDA provider loads CUDA and cuDNN by name: the GPU pack's copies go first.
            // Without them it falls back to the CPU.
            let _ = crate::cuda::load();
            config = config.with_execution_provider(ExecutionProvider::Cuda);
        }
        #[cfg(all(feature = "parakeet-directml", not(feature = "parakeet-cuda")))]
        {
            config = config.with_execution_provider(ExecutionProvider::DirectML);
        }
        #[cfg(all(
            feature = "parakeet-webgpu",
            not(feature = "parakeet-cuda"),
            not(feature = "parakeet-directml")
        ))]
        {
            config = config.with_execution_provider(ExecutionProvider::WebGPU);
        }
        let _ = ExecutionProvider::Cpu;
        config
    }

    pub struct ParakeetTranscriber {
        model: ParakeetModel,
        model_dir: PathBuf,
    }

    impl ParakeetTranscriber {
        pub fn new(model: ParakeetModel, model_dir: &Path) -> Result<Self, String> {
            if !model.present(model_dir) {
                return Err(format!(
                    "The {} model is not downloaded yet (looked in {})",
                    model.name(),
                    model_dir.display()
                ));
            }
            let mut loaded = LOADED.lock();
            let same = |(p, m, _): &Loaded| p.as_path() == model_dir && *m == model;
            if !loaded.as_ref().is_some_and(same) {
                // The one before goes first: two would not both fit some GPUs.
                *loaded = None;
                let config = Some(execution_config());
                let failed =
                    |e: parakeet_rs::Error| format!("Could not load the Parakeet model: {e}");
                let transcriber = match model {
                    ParakeetModel::Unified => Engine::Unified(
                        ParakeetUnified::from_pretrained(model_dir, config).map_err(failed)?,
                    ),
                    ParakeetModel::Verbatim => Engine::Verbatim(
                        ParakeetTDT::from_pretrained(model_dir, config).map_err(failed)?,
                    ),
                };
                *loaded = Some((model_dir.to_path_buf(), model, transcriber));
            }
            Ok(Self {
                model,
                model_dir: model_dir.to_path_buf(),
            })
        }
    }

    impl ChunkTranscriber for ParakeetTranscriber {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Parakeet
        }

        fn model(&self) -> String {
            self.model.name().into()
        }

        fn plan(&self) -> ChunkPlan {
            super::PLAN
        }

        fn transcribe_chunk(&mut self, chunk: &AudioChunk) -> Result<ChunkResult, String> {
            // Recognising under 0.1 s of audio gives nothing but noise tokens.
            if chunk.samples.len() < (ASR_SAMPLE_RATE / 10) as usize {
                return Ok(ChunkResult::default());
            }
            let mut loaded = LOADED.lock();
            let (_, _, model) = loaded
                .as_mut()
                .filter(|(p, m, _)| *p == self.model_dir && *m == self.model)
                .ok_or("The Parakeet model was unloaded")?;
            let result = model
                .transcribe(chunk.samples.clone())
                .map_err(|e| format!("Parakeet failed: {e}"))?;
            let words = result
                .tokens
                .into_iter()
                .map(|t| {
                    let start = seconds_to_us(t.start as f64);
                    TranscriptWord {
                        id: String::new(),
                        text: t.text,
                        kind: WordKind::Word,
                        source_start_us: start,
                        source_end_us: seconds_to_us(t.end as f64).max(start),
                        confidence: None,
                        speaker: None,
                    }
                })
                .collect();
            Ok(ChunkResult {
                words: super::group_word_tokens(words),
                language: None,
            })
        }
    }
}

#[cfg(feature = "parakeet")]
pub use engine::ParakeetTranscriber;

#[cfg(not(feature = "parakeet"))]
pub struct ParakeetTranscriber;

#[cfg(not(feature = "parakeet"))]
impl ParakeetTranscriber {
    pub fn new(_model: ParakeetModel, _model_dir: &Path) -> Result<Self, String> {
        Err(
            "This build of AeroEdits does not include local Parakeet transcription. \
             Rebuild with `--features parakeet-cuda` (or `parakeet-directml`, `parakeet-webgpu`, \
             `parakeet` for CPU only), or use ElevenLabs Scribe."
                .into(),
        )
    }
}

#[cfg(not(feature = "parakeet"))]
impl super::provider::ChunkTranscriber for ParakeetTranscriber {
    fn kind(&self) -> super::ProviderKind {
        super::ProviderKind::Parakeet
    }
    fn model(&self) -> String {
        ParakeetModel::default().name().into()
    }
    fn plan(&self) -> ChunkPlan {
        PLAN
    }
    fn transcribe_chunk(
        &mut self,
        _chunk: &super::audio::AudioChunk,
    ) -> Result<super::provider::ChunkResult, String> {
        Err("Parakeet is not included in this build".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_grouping_retains_repetitions_and_excludes_punctuation_time() {
        use crate::transcript::test_word;
        let words = group_word_tokens(vec![
            test_word(" very", 0, 160),
            test_word(" very", 240, 400),
            test_word(".", 400, 800),
            test_word(" ", 800, 880),
            test_word(" to", 960, 1040),
            test_word("day", 1040, 1200),
            test_word("'s", 1200, 1280),
            test_word("!", 1280, 1600),
        ]);
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            ["very", "very.", "today's!"]
        );
        assert_eq!(words[1].source_end_us, 400_000);
        assert_eq!(words[2].source_start_us, 960_000);
        assert_eq!(words[2].source_end_us, 1_280_000);
    }

    #[test]
    fn space_markers_and_unicode_suffixes_keep_lexical_timings() {
        use crate::transcript::test_word;
        let words = group_word_tokens(vec![
            test_word(" ", 0, 80),
            test_word("▁we", 80, 160),
            test_word("’re", 160, 240),
            test_word("▁twenty", 320, 400),
            test_word("-two", 400, 480),
            test_word(" ", 480, 640),
            test_word("▁two", 720, 800),
        ]);
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            ["we’re", "twenty-two", "two"]
        );
        assert_eq!(words[1].source_end_us, 480_000);
    }

    #[test]
    fn punctuation_inside_a_number_does_not_split_its_spoken_span() {
        use crate::transcript::test_word;
        let words = group_word_tokens(vec![
            test_word(" 3", 0, 80),
            test_word(".", 80, 160),
            test_word("5", 160, 240),
            test_word(" seconds", 320, 480),
        ]);
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "3.5");
        assert_eq!(words[0].source_end_us, 240_000);
    }

    #[test]
    fn model_presence_needs_every_file_of_this_builds_precision() {
        for model in [ParakeetModel::Unified, ParakeetModel::Verbatim] {
            let dir = tempfile::tempdir().unwrap();
            assert!(!model.present(dir.path()));
            for (name, _) in model.files() {
                std::fs::write(dir.path().join(name), b"x").unwrap();
            }
            assert!(model.present(dir.path()));
            // The other model's folder is not this one.
            let other = match model {
                ParakeetModel::Unified => ParakeetModel::Verbatim,
                ParakeetModel::Verbatim => ParakeetModel::Unified,
            };
            assert!(!other.present(dir.path()));
            std::fs::remove_file(dir.path().join(model.files()[2].0)).unwrap();
            assert!(!model.present(dir.path()));
        }
        assert_ne!(
            ParakeetModel::Unified.default_dir(),
            ParakeetModel::Verbatim.default_dir()
        );
        let json = serde_json::to_string(&ParakeetModel::Verbatim).unwrap();
        assert_eq!(json, "\"verbatim\"");
    }

    /// `cargo test --release --lib [--features parakeet-cuda] parakeet_download_probe --
    /// --ignored`: downloads this build's model into the default folder.
    #[test]
    #[ignore]
    fn parakeet_download_probe() {
        for model in [ParakeetModel::Unified, ParakeetModel::Verbatim] {
            let dir = model.default_dir();
            download_model(model, &dir, &AtomicBool::new(false), &mut |_, _| {}).unwrap();
            assert!(model.present(&dir));
        }
    }

    #[test]
    fn a_file_with_the_wrong_checksum_is_not_kept() {
        use std::io::Write as _;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let mut stream = stream.unwrap();
                let mut buf = [0u8; 1024];
                let _ = std::io::Read::read(&mut stream, &mut buf);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
                );
            }
        });
        let dir = tempfile::tempdir().unwrap();
        let error = download_files_checked(
            dir.path(),
            &format!("http://127.0.0.1:{port}"),
            &[("m.onnx", "m.onnx", Some("00"))],
            &AtomicBool::new(false),
            &mut |_, _| {},
        )
        .unwrap_err();
        assert!(error.contains("checksum"), "{error}");
        assert!(!dir.path().join("m.onnx").exists());
    }

    /// The model the probes run: `AEROEDITS_PARAKEET_MODEL=verbatim` for TDT v3.
    #[cfg(feature = "parakeet")]
    fn probe_model() -> ParakeetModel {
        match std::env::var("AEROEDITS_PARAKEET_MODEL").as_deref() {
            Ok("verbatim") => ParakeetModel::Verbatim,
            _ => ParakeetModel::Unified,
        }
    }

    /// `AEROEDITS_PARAKEET_SPEED=<16 kHz mono wav> cargo test --release --lib --features
    /// parakeet-cuda parakeet_speed_probe -- --ignored --nocapture`: load and transcribe times.
    /// `AEROEDITS_PARAKEET_WORDS=<file>` also writes the words, one per line.
    #[cfg(feature = "parakeet")]
    #[test]
    #[ignore]
    fn parakeet_speed_probe() {
        use super::super::audio::AudioChunk;
        use super::super::provider::ChunkTranscriber;
        let path = std::env::var("AEROEDITS_PARAKEET_SPEED").unwrap();
        let mut reader = crate::project::pcm::PcmReader::open(Path::new(&path)).unwrap();
        let mut all = vec![0f32; reader.info().frame_count as usize];
        let len = all.len();
        let n = reader.read_frames(&mut all, len).unwrap();
        all.truncate(n);
        let started = std::time::Instant::now();
        let mut model =
            ParakeetTranscriber::new(probe_model(), &probe_model().default_dir()).unwrap();
        println!("cuda pack: {:?}", crate::cuda::loaded());
        println!("loaded in {:?}", started.elapsed());
        let started = std::time::Instant::now();
        let mut words = Vec::new();
        for piece in all.chunks(PLAN.max_chunk_us as usize * 16 / 1000) {
            let result = model
                .transcribe_chunk(&AudioChunk {
                    source_start_us: 0,
                    samples: piece.to_vec(),
                })
                .unwrap();
            words.extend(result.words.into_iter().map(|w| w.text));
        }
        let took = started.elapsed();
        let audio = n as f64 / 16_000.0;
        println!(
            "{} words from {audio:.0} s of audio in {took:?}: {:.0}x realtime",
            words.len(),
            audio / took.as_secs_f64()
        );
        if let Ok(out) = std::env::var("AEROEDITS_PARAKEET_WORDS") {
            std::fs::write(out, words.join("\n")).unwrap();
        }
    }

    /// `AEROEDITS_PARAKEET_PROBE=<16 kHz mono wav>;<from s>;<to s>;<chunk s>[,<chunk s>]`
    #[cfg(feature = "parakeet")]
    #[test]
    #[ignore]
    fn parakeet_chunk_probe() {
        use super::super::audio::AudioChunk;
        use super::super::provider::ChunkTranscriber;
        let spec = std::env::var("AEROEDITS_PARAKEET_PROBE").unwrap();
        let parts: Vec<&str> = spec.split(';').collect();
        let mut reader = crate::project::pcm::PcmReader::open(Path::new(parts[0])).unwrap();
        let from = (parts[1].parse::<f64>().unwrap() * 16_000.0) as usize;
        let to = (parts[2].parse::<f64>().unwrap() * 16_000.0) as usize;
        let mut all = vec![0f32; reader.info().frame_count as usize];
        let len = all.len();
        let n = reader.read_frames(&mut all, len).unwrap();
        let samples = &all[from..to.min(n)];
        let mut model =
            ParakeetTranscriber::new(probe_model(), &probe_model().default_dir()).unwrap();
        for chunk_s in parts[3].split(',') {
            let len = (chunk_s.parse::<f64>().unwrap() * 16_000.0) as usize;
            let mut words = Vec::new();
            for (i, piece) in samples.chunks(len).enumerate() {
                let offset = from + i * len;
                let result = model
                    .transcribe_chunk(&AudioChunk {
                        source_start_us: 0,
                        samples: piece.to_vec(),
                    })
                    .unwrap();
                for w in result.words {
                    words.push((
                        w.source_start_us + offset as u64 * 1_000_000 / 16_000,
                        w.text,
                    ));
                }
            }
            println!("--- {chunk_s} s chunks: {} words", words.len());
            let mut line = String::new();
            let mut second = u64::MAX;
            for (start, text) in &words {
                if start / 10_000_000 != second {
                    second = start / 10_000_000;
                    println!("{line}");
                    line = format!("[{:>4}s]", start / 1_000_000);
                }
                line.push(' ');
                line.push_str(text);
            }
            println!("{line}");
        }
    }

    #[cfg(not(feature = "parakeet"))]
    #[test]
    fn builds_without_the_feature_explain_how_to_enable_it() {
        let error = ParakeetTranscriber::new(ParakeetModel::Unified, Path::new("/nonexistent"))
            .err()
            .unwrap();
        assert!(error.contains("--features"));
    }
}
