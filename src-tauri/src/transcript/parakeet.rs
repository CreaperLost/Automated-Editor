//! Local NVIDIA Parakeet TDT 0.6B transcription through ONNX Runtime (`parakeet-rs`).
//!
//! Inference is compiled only with the `parakeet` cargo feature; the GPU features pick the
//! execution provider (`parakeet-cuda` or `parakeet-directml` on Windows, `parakeet-webgpu`
//! for Metal on macOS) and ONNX Runtime falls back to the CPU when the GPU is unavailable.
//! Model download and status checks work in every build.
use super::audio::ChunkPlan;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MODEL_NAME: &str = "parakeet-tdt-0.6b-v3";
const MODEL_REPO: &str = "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main";
/// INT8 weights: about 670 MB in total instead of 2.5 GB, with near-identical accuracy.
const MODEL_FILES: &[&str] = &[
    "vocab.txt",
    "decoder_joint-model.int8.onnx",
    "encoder-model.int8.onnx",
];

/// Parakeet's encoder attends over the whole chunk, so memory grows with chunk length.
/// Ninety seconds fits comfortably on 6-8 GB GPUs.
pub const PLAN: ChunkPlan = ChunkPlan {
    max_chunk_us: 90_000_000,
    search_window_us: 15_000_000,
};

pub fn default_model_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("AeroEdits")
        .join("models")
        .join(MODEL_NAME)
}

/// True when `dir` has a vocabulary plus an encoder and decoder graph `parakeet-rs` can load.
pub fn model_present(dir: &Path) -> bool {
    let has = |names: &[&str]| names.iter().any(|n| dir.join(n).is_file());
    has(&["vocab.txt"])
        && has(&[
            "encoder-model.onnx",
            "encoder-model.int8.onnx",
            "encoder.onnx",
        ])
        && has(&[
            "decoder_joint-model.onnx",
            "decoder_joint-model.int8.onnx",
            "decoder_joint.onnx",
        ])
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

/// Downloads the INT8 model into `dir`, reporting `(bytes_done, bytes_total)` when known.
pub fn download_model(
    dir: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let client = reqwest::blocking::Client::builder()
        .timeout(None)
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let mut sizes = Vec::new();
    for name in MODEL_FILES {
        let size = client
            .head(format!("{MODEL_REPO}/{name}"))
            .send()
            .ok()
            .filter(|r| r.status().is_success())
            .and_then(|r| r.content_length());
        sizes.push(size);
    }
    let total = sizes.iter().copied().sum::<Option<u64>>();
    let mut done = 0u64;
    for name in MODEL_FILES {
        let target = dir.join(name);
        if target.is_file() {
            done += std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
            progress(done, total);
            continue;
        }
        let mut response = client
            .get(format!("{MODEL_REPO}/{name}"))
            .send()
            .map_err(|e| format!("Could not download {name}: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("Downloading {name} failed: {}", response.status()));
        }
        let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Download cancelled".into());
            }
            let n = response
                .read(&mut buf)
                .map_err(|e| format!("Download of {name} was interrupted: {e}"))?;
            if n == 0 {
                break;
            }
            temp.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            done += n as u64;
            progress(done, total);
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
    use parakeet_rs::{
        ExecutionConfig, ExecutionProvider, ParakeetTDT, TimestampMode, Transcriber,
    };
    use parking_lot::Mutex;
    use std::path::{Path, PathBuf};

    /// Loading the model takes seconds, so the last one loaded is kept for the next run.
    static LOADED: Mutex<Option<(PathBuf, ParakeetTDT)>> = Mutex::new(None);

    fn execution_config() -> ExecutionConfig {
        #[allow(unused_mut)]
        let mut config = ExecutionConfig::new();
        #[cfg(feature = "parakeet-cuda")]
        {
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
        model_dir: PathBuf,
    }

    impl ParakeetTranscriber {
        pub fn new(model_dir: &Path) -> Result<Self, String> {
            if !super::model_present(model_dir) {
                return Err(format!(
                    "The Parakeet model is not downloaded yet (looked in {})",
                    model_dir.display()
                ));
            }
            let mut loaded = LOADED.lock();
            if loaded.as_ref().map(|(p, _)| p.as_path()) != Some(model_dir) {
                *loaded = None;
                let model = ParakeetTDT::from_pretrained(model_dir, Some(execution_config()))
                    .map_err(|e| format!("Could not load the Parakeet model: {e}"))?;
                *loaded = Some((model_dir.to_path_buf(), model));
            }
            Ok(Self {
                model_dir: model_dir.to_path_buf(),
            })
        }
    }

    impl ChunkTranscriber for ParakeetTranscriber {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Parakeet
        }

        fn model(&self) -> String {
            super::MODEL_NAME.into()
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
            let (_, model) = loaded
                .as_mut()
                .filter(|(p, _)| *p == self.model_dir)
                .ok_or("The Parakeet model was unloaded")?;
            let result = model
                .transcribe_samples(
                    chunk.samples.clone(),
                    ASR_SAMPLE_RATE,
                    1,
                    Some(TimestampMode::Tokens),
                )
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
    pub fn new(_model_dir: &Path) -> Result<Self, String> {
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
        MODEL_NAME.into()
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
    fn model_presence_needs_all_three_parts() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!model_present(dir.path()));
        for name in MODEL_FILES {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        assert!(model_present(dir.path()));
        std::fs::remove_file(dir.path().join("vocab.txt")).unwrap();
        assert!(!model_present(dir.path()));
    }

    #[cfg(not(feature = "parakeet"))]
    #[test]
    fn builds_without_the_feature_explain_how_to_enable_it() {
        let error = ParakeetTranscriber::new(Path::new("/nonexistent"))
            .err()
            .unwrap();
        assert!(error.contains("--features"));
    }
}
