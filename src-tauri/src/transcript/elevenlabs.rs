//! ElevenLabs Scribe (cloud) speech-to-text with word timestamps, sent either to ElevenLabs
//! (`POST /v1/speech-to-text`, the user's ElevenLabs key) or through OpenRouter
//! (`POST /api/v1/audio/transcriptions`, the user's OpenRouter key).
use super::audio::{AudioChunk, ChunkPlan};
use super::provider::{ChunkResult, ChunkTranscriber, ProviderKind};
use super::{seconds_to_us, TranscriptWord, WordKind};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const DEFAULT_MODEL: &str = "scribe_v2";
/// Scribe's model id on OpenRouter.
pub const OPENROUTER_MODEL: &str = "elevenlabs/scribe-v2";
const ENDPOINT: &str = "https://api.elevenlabs.io/v1/speech-to-text";
const OPENROUTER_ENDPOINT: &str = "https://openrouter.ai/api/v1/audio/transcriptions";
const MAX_ATTEMPTS: u32 = 3;
const MAX_KEYTERMS: usize = 100;
/// How often a request waiting on the network looks at Cancel.
const CANCEL_POLL: Duration = Duration::from_millis(100);
const CANCELLED: &str = "Transcription cancelled";

/// Which service carries the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    ElevenLabs,
    OpenRouter,
}

impl Route {
    fn service(self) -> &'static str {
        match self {
            Route::ElevenLabs => "ElevenLabs",
            Route::OpenRouter => "OpenRouter",
        }
    }
}

pub struct ScribeTranscriber {
    client: reqwest::blocking::Client,
    route: Route,
    endpoint: String,
    cancel: Arc<AtomicBool>,
    api_key: String,
    model: String,
    language: Option<String>,
    keyterms: Vec<String>,
}

impl ScribeTranscriber {
    pub fn new(
        api_key: String,
        model: Option<String>,
        language: Option<String>,
        keyterms: Vec<String>,
    ) -> Result<Self, String> {
        Self::build(Route::ElevenLabs, api_key, model, language, keyterms)
    }

    /// Scribe v2 through OpenRouter, billed to the OpenRouter key.
    pub fn via_openrouter(
        api_key: String,
        language: Option<String>,
        keyterms: Vec<String>,
    ) -> Result<Self, String> {
        Self::build(
            Route::OpenRouter,
            api_key,
            Some(OPENROUTER_MODEL.into()),
            language,
            keyterms,
        )
    }

    fn build(
        route: Route,
        api_key: String,
        model: Option<String>,
        language: Option<String>,
        keyterms: Vec<String>,
    ) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err(format!(
                "Add your {} API key in transcription settings first",
                route.service()
            ));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15 * 60))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            route,
            endpoint: match route {
                Route::ElevenLabs => ENDPOINT,
                Route::OpenRouter => OPENROUTER_ENDPOINT,
            }
            .into(),
            cancel: Arc::new(AtomicBool::new(false)),
            api_key: api_key.trim().to_string(),
            model: model
                .filter(|m| !m.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_MODEL.into()),
            language: language.filter(|l| !l.trim().is_empty()),
            keyterms: keyterms
                .into_iter()
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty() && k.chars().count() <= 50)
                .take(MAX_KEYTERMS)
                .collect(),
        })
    }

    /// Stops waiting on an upload, and stops retrying, as soon as `cancel` is set.
    pub fn with_cancel(mut self, cancel: Arc<AtomicBool>) -> Self {
        self.cancel = cancel;
        self
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// Sends on a helper thread and waits for it, checking Cancel during the upload and while
    /// the service works. A cancelled request is left to finish on its own; its answer is dropped.
    fn send(&self, wav: Vec<u8>) -> Result<Result<(u16, String), String>, String> {
        let request = self.client.post(&self.endpoint);
        let request = match self.route {
            Route::ElevenLabs => request
                .header("xi-api-key", &self.api_key)
                .multipart(self.form(wav)?),
            Route::OpenRouter => request
                .bearer_auth(&self.api_key)
                // OpenRouter's optional app attribution headers.
                .header(
                    "HTTP-Referer",
                    "https://github.com/CreaperLost/Automated-Editor",
                )
                .header("X-Title", "AeroEdits")
                .multipart(self.openrouter_form(wav)?),
        };
        let service = self.route.service();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = request.send().and_then(|response| {
                let status = response.status().as_u16();
                response.text().map(|text| (status, text))
            });
            let _ = tx.send(result);
        });
        loop {
            if self.cancelled() {
                return Err(CANCELLED.into());
            }
            match rx.recv_timeout(CANCEL_POLL) {
                Ok(result) => {
                    return Ok(result.map_err(|e| format!("Could not reach {service}: {e}")))
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Ok(Err(format!("The request to {service} stopped")))
                }
            }
        }
    }

    fn form(&self, wav: Vec<u8>) -> Result<reqwest::blocking::multipart::Form, String> {
        let file = reqwest::blocking::multipart::Part::bytes(wav)
            .file_name("chunk.wav")
            .mime_str("audio/wav")
            .map_err(|e| e.to_string())?;
        let mut form = reqwest::blocking::multipart::Form::new()
            .text("model_id", self.model.clone())
            .text("timestamps_granularity", "word")
            .text("tag_audio_events", "true")
            .text("diarize", "false")
            // Fillers and false starts must stay in the transcript so they can be cut.
            .text("no_verbatim", "false")
            .part("file", file);
        if let Some(language) = &self.language {
            form = form.text("language_code", language.clone());
        }
        for term in &self.keyterms {
            form = form.text("keyterms", term.clone());
        }
        Ok(form)
    }

    /// OpenRouter's OpenAI-style upload. Word times need `verbose_json`; ElevenLabs' own
    /// switches go in `provider.options.elevenlabs`, which OpenRouter drops if it doesn't know them.
    fn openrouter_form(&self, wav: Vec<u8>) -> Result<reqwest::blocking::multipart::Form, String> {
        let file = reqwest::blocking::multipart::Part::bytes(wav)
            .file_name("chunk.wav")
            .mime_str("audio/wav")
            .map_err(|e| e.to_string())?;
        let provider = serde_json::json!({
            "options": { "elevenlabs": { "tag_audio_events": true, "no_verbatim": false } }
        });
        let mut form = reqwest::blocking::multipart::Form::new()
            .text("model", self.model.clone())
            .text("response_format", "verbose_json")
            .text("timestamp_granularities[]", "word")
            .text("provider", provider.to_string())
            .part("file", file);
        if let Some(language) = &self.language {
            form = form.text("language", language.clone());
        }
        for term in &self.keyterms {
            form = form.text("keyterms[]", term.clone());
        }
        Ok(form)
    }
}

impl ChunkTranscriber for ScribeTranscriber {
    fn kind(&self) -> ProviderKind {
        match self.route {
            Route::ElevenLabs => ProviderKind::ElevenLabs,
            Route::OpenRouter => ProviderKind::OpenRouter,
        }
    }

    fn model(&self) -> String {
        self.model.clone()
    }

    fn plan(&self) -> ChunkPlan {
        // Ten-minute uploads (about 19 MB of 16-bit WAV) keep each request well inside the
        // API's limits and let progress move during long recordings.
        ChunkPlan {
            max_chunk_us: 600_000_000,
            search_window_us: 20_000_000,
        }
    }

    fn transcribe_chunk(&mut self, chunk: &AudioChunk) -> Result<ChunkResult, String> {
        let wav = chunk.to_wav_bytes();
        let mut last_error = String::new();
        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                // Back off before retrying, still answering Cancel.
                let wait = Duration::from_secs(2u64.pow(attempt));
                let started = std::time::Instant::now();
                while started.elapsed() < wait {
                    if self.cancelled() {
                        return Err(CANCELLED.into());
                    }
                    std::thread::sleep(CANCEL_POLL);
                }
            }
            let (status, body) = match self.send(wav.clone())? {
                Ok(answer) => answer,
                Err(error) => {
                    last_error = error;
                    continue;
                }
            };
            if (200..300).contains(&status) {
                return match self.route {
                    Route::ElevenLabs => parse_response(&body),
                    Route::OpenRouter => parse_openrouter_response(&body),
                };
            }
            last_error = describe_error(self.route, status, &body);
            if !(status == 429 || status >= 500) {
                break;
            }
        }
        Err(last_error)
    }
}

fn describe_error(route: Route, status: u16, body: &str) -> String {
    // ElevenLabs answers `{"detail": ...}`, OpenRouter `{"error": {"message": ...}}`.
    let detail = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            let d = v.get("detail").or_else(|| v.get("error"))?;
            d.get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
                .or_else(|| d.as_str().map(str::to_string))
        })
        .unwrap_or_else(|| body.chars().take(200).collect());
    let service = route.service();
    match status {
        401 => format!("{service} rejected the API key: {detail}"),
        402 if route == Route::OpenRouter => {
            format!("OpenRouter needs more credits: {detail}")
        }
        _ => format!("{service} returned {status}: {detail}"),
    }
}

#[derive(Deserialize)]
struct ScribeResponse {
    #[serde(default)]
    language_code: Option<String>,
    #[serde(default)]
    words: Vec<ScribeWord>,
}

#[derive(Deserialize)]
struct ScribeWord {
    text: String,
    #[serde(default)]
    start: Option<f64>,
    #[serde(default)]
    end: Option<f64>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    speaker_id: Option<String>,
    #[serde(default)]
    logprob: Option<f64>,
}

pub(crate) fn parse_response(body: &str) -> Result<ChunkResult, String> {
    let response: ScribeResponse =
        serde_json::from_str(body).map_err(|e| format!("Unexpected ElevenLabs response: {e}"))?;
    let words = response
        .words
        .into_iter()
        .filter_map(|w| {
            let kind = match w.kind.as_deref() {
                Some("spacing") => return None,
                Some("audio_event") => WordKind::AudioEvent,
                _ => WordKind::Word,
            };
            let start = seconds_to_us(w.start?);
            let end = seconds_to_us(w.end.unwrap_or(0.0)).max(start);
            Some(TranscriptWord {
                id: String::new(),
                text: w.text,
                kind,
                source_start_us: start,
                source_end_us: end,
                confidence: w
                    .logprob
                    .filter(|l| l.is_finite())
                    .map(|l| l.exp().clamp(0.0, 1.0) as f32),
                speaker: w.speaker_id,
            })
        })
        .collect();
    Ok(ChunkResult {
        words,
        language: response.language_code,
    })
}

#[derive(Deserialize)]
struct OpenRouterResponse {
    #[serde(default)]
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    words: Vec<OpenRouterWord>,
}

#[derive(Deserialize)]
struct OpenRouterWord {
    word: String,
    start: f64,
    end: f64,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    speaker_label: Option<String>,
}

pub(crate) fn parse_openrouter_response(body: &str) -> Result<ChunkResult, String> {
    let response: OpenRouterResponse =
        serde_json::from_str(body).map_err(|e| format!("Unexpected OpenRouter response: {e}"))?;
    // Cuts are made at word times, so text without them is no use.
    if response.words.is_empty() && !response.text.trim().is_empty() {
        return Err("OpenRouter returned the text without word times".into());
    }
    let words = response
        .words
        .into_iter()
        .filter_map(|w| {
            let kind = match w.kind.as_deref() {
                Some("spacing") => return None,
                Some("audio_event") => WordKind::AudioEvent,
                _ => WordKind::Word,
            };
            let start = seconds_to_us(w.start);
            Some(TranscriptWord {
                id: String::new(),
                text: w.word,
                kind,
                source_start_us: start,
                source_end_us: seconds_to_us(w.end).max(start),
                confidence: w
                    .confidence
                    .filter(|c| c.is_finite())
                    .map(|c| c.clamp(0.0, 1.0) as f32),
                speaker: w.speaker_label,
            })
        })
        .collect();
    Ok(ChunkResult {
        words,
        language: response.language,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_words_and_skips_spacing() {
        let body = r#"{
            "language_code": "eng",
            "language_probability": 0.98,
            "text": "Um, hello (laughs)",
            "words": [
                {"text": "Um,", "start": 0.1, "end": 0.4, "type": "word", "speaker_id": "speaker_0", "logprob": -0.1},
                {"text": " ", "start": 0.4, "end": 0.5, "type": "spacing"},
                {"text": "hello", "start": 0.5, "end": 0.9, "type": "word"},
                {"text": "(laughs)", "start": 1.0, "end": 1.6, "type": "audio_event"}
            ]
        }"#;
        let result = parse_response(body).unwrap();
        assert_eq!(result.language.as_deref(), Some("eng"));
        assert_eq!(result.words.len(), 3);
        assert_eq!(result.words[0].text, "Um,");
        assert_eq!(result.words[0].source_start_us, 100_000);
        assert_eq!(result.words[0].speaker.as_deref(), Some("speaker_0"));
        assert!(result.words[0].confidence.unwrap() > 0.9);
        assert_eq!(result.words[2].kind, WordKind::AudioEvent);
    }

    #[test]
    fn error_detail_is_surfaced() {
        let msg = describe_error(
            Route::ElevenLabs,
            401,
            r#"{"detail":{"status":"invalid_api_key","message":"Invalid API key"}}"#,
        );
        assert!(msg.contains("Invalid API key"));
        assert!(describe_error(Route::ElevenLabs, 500, "oops").contains("500"));
        let msg = describe_error(
            Route::OpenRouter,
            402,
            r#"{"error":{"code":402,"message":"Insufficient credits"}}"#,
        );
        assert!(msg.starts_with("OpenRouter needs more credits"), "{msg}");
        assert!(msg.contains("Insufficient credits"));
    }

    #[test]
    fn parses_openrouter_words_and_refuses_text_without_times() {
        let body = r#"{
            "text": "Um, hello (laughs)",
            "language": "english",
            "words": [
                {"word": "Um,", "start": 0.1, "end": 0.4, "confidence": 0.9, "speaker_label": "speaker_0"},
                {"word": " ", "start": 0.4, "end": 0.5, "type": "spacing"},
                {"word": "hello", "start": 0.5, "end": 0.9, "type": "word"},
                {"word": "(laughs)", "start": 1.0, "end": 1.6, "type": "audio_event"}
            ],
            "usage": {"seconds": 1.6, "cost": 0.0001}
        }"#;
        let result = parse_openrouter_response(body).unwrap();
        assert_eq!(result.language.as_deref(), Some("english"));
        assert_eq!(result.words.len(), 3);
        assert_eq!(result.words[0].source_start_us, 100_000);
        assert_eq!(result.words[0].speaker.as_deref(), Some("speaker_0"));
        assert_eq!(result.words[2].kind, WordKind::AudioEvent);
        assert!(parse_openrouter_response(r#"{"text": "hello"}"#).is_err());
        assert!(parse_openrouter_response(r#"{"text": ""}"#)
            .unwrap()
            .words
            .is_empty());
    }

    /// Sends one real chunk through OpenRouter with the stored key and prints the raw answer.
    /// `AEROEDITS_OPENROUTER_PROBE=<16 kHz mono wav>;<from s>;<to s>`
    #[test]
    #[ignore]
    fn openrouter_probe() {
        let spec = std::env::var("AEROEDITS_OPENROUTER_PROBE").unwrap();
        let parts: Vec<&str> = spec.split(';').collect();
        let mut reader =
            crate::project::pcm::PcmReader::open(std::path::Path::new(parts[0])).unwrap();
        let from = (parts[1].parse::<f64>().unwrap() * 16_000.0) as usize;
        let to = (parts[2].parse::<f64>().unwrap() * 16_000.0) as usize;
        let mut all = vec![0f32; reader.info().frame_count as usize];
        let len = all.len();
        let n = reader.read_frames(&mut all, len).unwrap();
        let chunk = AudioChunk {
            source_start_us: 0,
            samples: all[from..to.min(n)].to_vec(),
        };
        let dir = super::super::settings::config_dir();
        let (key, _) =
            crate::secrets::get(&dir, crate::ai::AiProvider::OpenRouter.key_spec()).unwrap();
        let scribe = ScribeTranscriber::via_openrouter(key, Some("en".into()), Vec::new()).unwrap();
        let (status, body) = scribe.send(chunk.to_wav_bytes()).unwrap().unwrap();
        println!("status {status}\n{body}");
    }

    #[test]
    fn empty_key_is_rejected() {
        assert!(ScribeTranscriber::new("  ".into(), None, None, Vec::new()).is_err());
    }

    #[test]
    fn cancel_stops_an_upload_that_is_still_waiting_and_does_not_retry() {
        // A server that takes the upload and never answers; it counts the requests it gets.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let accepted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = accepted.clone();
        std::thread::spawn(move || {
            let mut held = Vec::new();
            let started = std::time::Instant::now();
            while started.elapsed() < Duration::from_secs(10) {
                if let Ok((stream, _)) = listener.accept() {
                    count.fetch_add(1, Ordering::SeqCst);
                    held.push(stream);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let cancel = Arc::new(AtomicBool::new(false));
        let mut scribe = ScribeTranscriber::new("key".into(), None, None, Vec::new())
            .unwrap()
            .with_cancel(cancel.clone());
        scribe.endpoint = url;
        let flag = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            flag.store(true, Ordering::SeqCst);
        });
        let chunk = AudioChunk {
            source_start_us: 0,
            samples: vec![0.0; 1600],
        };
        let started = std::time::Instant::now();
        assert_eq!(
            scribe.transcribe_chunk(&chunk).err().as_deref(),
            Some(CANCELLED)
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "{:?}",
            started.elapsed()
        );
        std::thread::sleep(Duration::from_millis(500));
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
    }
}
