//! ElevenLabs Scribe (cloud) speech-to-text: `POST /v1/speech-to-text` with word timestamps.
use super::audio::{AudioChunk, ChunkPlan};
use super::provider::{ChunkResult, ChunkTranscriber, ProviderKind};
use super::{seconds_to_us, TranscriptWord, WordKind};
use serde::Deserialize;
use std::time::Duration;

pub const DEFAULT_MODEL: &str = "scribe_v2";
const ENDPOINT: &str = "https://api.elevenlabs.io/v1/speech-to-text";
const MAX_ATTEMPTS: u32 = 3;
const MAX_KEYTERMS: usize = 100;

pub struct ScribeTranscriber {
    client: reqwest::blocking::Client,
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
        if api_key.trim().is_empty() {
            return Err("Add your ElevenLabs API key in transcription settings first".into());
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15 * 60))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client,
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
}

impl ChunkTranscriber for ScribeTranscriber {
    fn kind(&self) -> ProviderKind {
        ProviderKind::ElevenLabs
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
                std::thread::sleep(Duration::from_secs(2u64.pow(attempt)));
            }
            let response = self
                .client
                .post(ENDPOINT)
                .header("xi-api-key", &self.api_key)
                .multipart(self.form(wav.clone())?)
                .send();
            let response = match response {
                Ok(response) => response,
                Err(error) => {
                    last_error = format!("Could not reach ElevenLabs: {error}");
                    continue;
                }
            };
            let status = response.status();
            let body = response.text().map_err(|e| e.to_string())?;
            if status.is_success() {
                return parse_response(&body);
            }
            last_error = describe_error(status.as_u16(), &body);
            if !(status.as_u16() == 429 || status.is_server_error()) {
                break;
            }
        }
        Err(last_error)
    }
}

fn describe_error(status: u16, body: &str) -> String {
    let detail = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            let d = v.get("detail")?;
            d.get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
                .or_else(|| d.as_str().map(str::to_string))
        })
        .unwrap_or_else(|| body.chars().take(200).collect());
    match status {
        401 => format!("ElevenLabs rejected the API key: {detail}"),
        _ => format!("ElevenLabs returned {status}: {detail}"),
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
            401,
            r#"{"detail":{"status":"invalid_api_key","message":"Invalid API key"}}"#,
        );
        assert!(msg.contains("Invalid API key"));
        assert!(describe_error(500, "oops").contains("500"));
    }

    #[test]
    fn empty_key_is_rejected() {
        assert!(ScribeTranscriber::new("  ".into(), None, None, Vec::new()).is_err());
    }
}
