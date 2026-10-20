//! The built-in model for the filler and retake review: Qwen3.5 4B, 4-bit (Q4_K_M, 2.7 GB),
//! run inside the app by llama.cpp on the GPU through Vulkan, so it is free, private and needs
//! no key. Vulkan comes with every GPU driver, so nothing else is installed; without a GPU it
//! runs on the CPU, slowly.
//!
//! It reads only the windows the local pass flagged (see `candidates`), one page at a time,
//! and answers in the same JSON as the OpenRouter review. A grammar made from that JSON's
//! schema holds every token it writes to it, and it decodes greedily: the same words give the
//! same cuts every time.
// Without the model, only the download and status parts are used.
#![cfg_attr(not(feature = "local-llm"), allow(dead_code))]
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

pub const MODEL_NAME: &str = "qwen3.5-4b";
/// `local/<model>`, as the review is recorded.
pub const DESCRIBE: &str = "local/qwen3.5-4b-q4_k_m";
const FILE: &str = "Qwen3.5-4B-Q4_K_M.gguf";
/// unsloth's GGUF of Qwen/Qwen3.5-4B (Apache-2.0), pinned to one commit.
const REPO: &str = "https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/resolve/e87f176479d0855a907a41277aca2f8ee7a09523";
const SHA256: &str = "00fe7986ff5f6b463e62455821146049db6f9313603938a70800d1fb69ef11a4";
pub const MODEL_BYTES: u64 = 2_740_937_888;

/// The JSON every answer must be: the same as the OpenRouter review's.
const ANSWER_SCHEMA: &str = r#"{"type":"object","properties":{"cuts":{"type":"array","items":{"type":"object","properties":{"type":{"enum":["filler","retake"]},"from":{"type":"integer","minimum":0},"to":{"type":"integer","minimum":0},"why":{"type":"string","maxLength":80}},"required":["type","from","to","why"],"additionalProperties":false}}},"required":["cuts"],"additionalProperties":false}"#;

/// Room for the prompt and the answer.
const CONTEXT_TOKENS: u32 = 8192;
/// An answer longer than this is cut off (and fails to parse): no page needs it.
const MAX_ANSWER_TOKENS: usize = 1536;

pub fn default_dir() -> PathBuf {
    crate::transcript::parakeet::default_model_dir()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir)
        .join(MODEL_NAME)
}

pub fn present(dir: &Path) -> bool {
    dir.join(FILE).is_file()
}

pub fn compiled_in() -> bool {
    cfg!(feature = "local-llm")
}

/// The model is in this build and downloaded.
pub fn ready() -> bool {
    compiled_in() && present(&default_dir())
}

pub fn download(
    dir: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<(), String> {
    crate::transcript::parakeet::download_files_checked(
        dir,
        REPO,
        &[(FILE, FILE, Some(SHA256))],
        cancel,
        progress,
    )
}

/// The prompt in Qwen's chat format, its thinking switched off (the empty think block).
fn chat_prompt(system: &str, user: &str) -> String {
    format!(
        "<|im_start|>system\n{system}<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n\
         <|im_start|>assistant\n<think>\n\n</think>\n\n"
    )
}

#[cfg(feature = "local-llm")]
mod engine {
    use super::super::client::{parse_reply, JsonModel};
    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::LlamaModel;
    use llama_cpp_2::sampling::LlamaSampler;
    use serde_json::Value;
    use std::num::NonZeroU32;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, OnceLock};

    /// llama.cpp is set up once per process.
    static BACKEND: OnceLock<Result<LlamaBackend, String>> = OnceLock::new();

    fn backend() -> Result<&'static LlamaBackend, String> {
        BACKEND
            .get_or_init(|| {
                if !vulkan_present() {
                    return Err("this computer has no Vulkan driver (vulkan-1.dll)".into());
                }
                let mut backend = LlamaBackend::init()
                    .map_err(|e| format!("Could not start the local model: {e}"))?;
                backend.void_logs();
                Ok(backend)
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    /// The Vulkan loader is delay-loaded: llama.cpp touches it only after this says it exists.
    fn vulkan_present() -> bool {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::FreeLibrary;
            use windows_sys::Win32::System::LibraryLoader::{
                LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32,
            };
            let name: Vec<u16> = "vulkan-1.dll\0".encode_utf16().collect();
            // SAFETY: a NUL-terminated name; the module is freed again at once.
            unsafe {
                let module = LoadLibraryExW(
                    name.as_ptr(),
                    std::ptr::null_mut(),
                    LOAD_LIBRARY_SEARCH_SYSTEM32,
                );
                if module.is_null() {
                    return false;
                }
                FreeLibrary(module);
            }
            true
        }
        #[cfg(not(windows))]
        {
            true
        }
    }

    /// The loaded model, ready to answer review pages. Dropping it frees the GPU memory.
    pub struct LocalModel {
        model: LlamaModel,
        grammar: String,
        cancel: Arc<AtomicBool>,
    }

    impl LocalModel {
        pub fn load(dir: &std::path::Path) -> Result<Self, String> {
            let path = dir.join(super::FILE);
            if !path.is_file() {
                return Err("Download the local model in transcription settings first".into());
            }
            let backend = backend()?;
            // Every layer on the GPU; llama.cpp keeps on the CPU what the GPU cannot hold.
            let params = LlamaModelParams::default().with_n_gpu_layers(999);
            let model = LlamaModel::load_from_file(backend, &path, &params)
                .map_err(|e| format!("Could not load the local model: {e}"))?;
            let grammar = llama_cpp_2::json_schema_to_grammar(super::ANSWER_SCHEMA)
                .map_err(|e| format!("Could not build the answer grammar: {e}"))?;
            Ok(Self {
                model,
                grammar,
                cancel: Arc::new(AtomicBool::new(false)),
            })
        }

        pub fn with_cancel(mut self, cancel: Arc<AtomicBool>) -> Self {
            self.cancel = cancel;
            self
        }

        fn answer(&self, prompt: &str) -> Result<String, String> {
            let backend = backend()?;
            let threads = std::thread::available_parallelism()
                .map(|n| (n.get() / 2).max(1) as i32)
                .unwrap_or(4);
            let params = LlamaContextParams::default()
                .with_n_ctx(NonZeroU32::new(super::CONTEXT_TOKENS))
                .with_n_batch(2048)
                .with_n_threads(threads)
                .with_n_threads_batch(threads);
            let mut ctx = self
                .model
                .new_context(backend, params)
                .map_err(|e| format!("Could not start the local model: {e}"))?;
            let vocab = self.model.vocab();
            let tokens = vocab.tokenize(prompt.as_bytes(), true, true);
            if tokens.len() + super::MAX_ANSWER_TOKENS > super::CONTEXT_TOKENS as usize {
                return Err("A review page is too long for the local model".into());
            }
            let mut batch = LlamaBatch::new(2048, 1);
            let last = tokens.len() - 1;
            for (start, piece) in tokens.chunks(2048).enumerate() {
                batch.clear();
                for (k, token) in piece.iter().enumerate() {
                    let pos = start * 2048 + k;
                    batch
                        .add(*token, pos as i32, &[0], pos == last)
                        .map_err(|e| e.to_string())?;
                }
                ctx.decode(&mut batch)
                    .map_err(|e| format!("The local model failed: {e}"))?;
            }
            // The grammar is checked against the 40 likeliest tokens only, not all ~250 000 of
            // Qwen's vocabulary: that made every token several times slower. In JSON one of the
            // 40 always fits.
            let mut sampler = LlamaSampler::chain_simple([
                LlamaSampler::top_k(40),
                LlamaSampler::grammar(&self.model, &self.grammar, "root")
                    .map_err(|e| format!("Could not use the answer grammar: {e}"))?,
                LlamaSampler::greedy(),
            ]);
            let mut out: Vec<u8> = Vec::new();
            let mut pos = tokens.len() as i32;
            for _ in 0..super::MAX_ANSWER_TOKENS {
                if self.cancel.load(Ordering::SeqCst) {
                    return Err("Cancelled".into());
                }
                let token = sampler.sample(&ctx, batch.n_tokens() - 1);
                if vocab.is_eog(token) {
                    break;
                }
                out.extend(vocab.token_to_piece(token, false, None));
                batch.clear();
                batch
                    .add(token, pos, &[0], true)
                    .map_err(|e| e.to_string())?;
                pos += 1;
                ctx.decode(&mut batch)
                    .map_err(|e| format!("The local model failed: {e}"))?;
            }
            String::from_utf8(out).map_err(|_| "The local model wrote broken text".to_string())
        }
    }

    impl JsonModel for LocalModel {
        fn complete_json(&mut self, system: &str, user: &str) -> Result<Value, String> {
            let text = self.answer(&super::chat_prompt(system, user))?;
            parse_reply(&text)
        }

        fn describe(&self) -> String {
            super::DESCRIBE.into()
        }
    }
}

#[cfg(feature = "local-llm")]
pub use engine::LocalModel;

#[cfg(not(feature = "local-llm"))]
pub struct LocalModel;

#[cfg(not(feature = "local-llm"))]
impl LocalModel {
    pub fn load(_dir: &Path) -> Result<Self, String> {
        Err("This build of AeroEdits does not include the local model (feature `local-llm`)".into())
    }

    pub fn with_cancel(self, _cancel: std::sync::Arc<AtomicBool>) -> Self {
        self
    }
}

#[cfg(not(feature = "local-llm"))]
impl super::client::JsonModel for LocalModel {
    fn complete_json(&mut self, _system: &str, _user: &str) -> Result<serde_json::Value, String> {
        Err("The local model is not included in this build".into())
    }

    fn describe(&self) -> String {
        DESCRIBE.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_switches_thinking_off() {
        let prompt = chat_prompt("rules", "[0]so [1]um");
        assert!(prompt.starts_with("<|im_start|>system\nrules<|im_end|>"));
        assert!(prompt.ends_with("<|im_start|>assistant\n<think>\n\n</think>\n\n"));
    }

    /// `cargo test --release --lib local_download_probe -- --ignored`: downloads the model.
    #[test]
    #[ignore]
    fn local_download_probe() {
        download(&default_dir(), &AtomicBool::new(false), &mut |_, _| {}).unwrap();
        assert!(present(&default_dir()));
    }

    #[test]
    fn the_answer_schema_is_json() {
        let schema: serde_json::Value = serde_json::from_str(ANSWER_SCHEMA).unwrap();
        assert_eq!(schema["required"][0], "cuts");
    }

    /// `AEROEDITS_LLM_PROBE=1 cargo test --release --lib --features local-llm local_llm_probe
    /// -- --ignored --nocapture`: answers one English and one Greek page, timed.
    #[cfg(feature = "local-llm")]
    #[test]
    #[ignore]
    fn local_llm_probe() {
        use super::super::client::JsonModel;
        use crate::transcript::Language;
        let mut model = LocalModel::load(&default_dir()).unwrap();
        // The first answer also compiles Vulkan pipelines: warm up before timing.
        let _ = model.complete_json("Answer {\"cuts\":[]}.", "[0]hi");
        for (language, page) in [
            (Language::English, "Window 1 of 1, check: [3] \"like\" (filler?); [8]-[12] \"so we can\" said again (retake?)\n[0]Today [1]we [2]are [3]like [4]going [5]to [6]build [7]it, (pause 0.8s) [8]so [9]we [10]can (pause 1.1s) [11]so [12]we [13]can [14]see [15]the [16]result."),
            (Language::Greek, "Window 1 of 1, check: [1] \"λοιπόν\" (filler?); [4] \"ξέρεις,\" (filler?)\n[0]Εεε [1]λοιπόν [2]σήμερα [3]φτιάχνουμε, [4]ξέρεις, [5]ένα [6]plugin. [7]Ξέρεις [8]τι [9]κάνει;"),
        ] {
            let started = std::time::Instant::now();
            let reply = model
                .complete_json(&super::super::fillers::system_prompt(language), page)
                .unwrap();
            println!("{language:?} in {:?}: {reply}", started.elapsed());
        }
    }
}
