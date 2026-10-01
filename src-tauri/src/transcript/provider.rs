//! The provider seam: each provider transcribes one 16 kHz mono chunk at a time, and
//! [`transcribe_segments`] stitches chunk results into one source-timed transcript.
use super::audio::{for_each_chunk, AudioChunk, ChunkPlan};
use super::{Transcript, TranscriptWord};
use crate::project::reader::SegmentSummary;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::AtomicBool;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    /// NVIDIA Parakeet TDT, run locally through ONNX Runtime.
    #[default]
    Parakeet,
    /// ElevenLabs Scribe, a cloud API.
    #[serde(rename = "elevenlabs")]
    ElevenLabs,
}

impl ProviderKind {
    pub fn label(self) -> &'static str {
        match self {
            ProviderKind::Parakeet => "Parakeet (local)",
            ProviderKind::ElevenLabs => "ElevenLabs Scribe",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionProgress {
    pub track_id: String,
    /// 0.0 to 1.0, by source time reached.
    pub fraction: f64,
    pub message: String,
}

/// Words returned for one chunk, timed from the chunk's start.
#[derive(Default)]
pub struct ChunkResult {
    pub words: Vec<TranscriptWord>,
    pub language: Option<String>,
}

pub trait ChunkTranscriber {
    fn kind(&self) -> ProviderKind;
    fn model(&self) -> String;
    fn plan(&self) -> ChunkPlan;
    fn transcribe_chunk(&mut self, chunk: &AudioChunk) -> Result<ChunkResult, String>;
}

pub struct TranscriptionOutput {
    pub transcript: Transcript,
    pub diagnostics: Vec<String>,
}

pub fn transcribe_segments(
    transcriber: &mut dyn ChunkTranscriber,
    root: &Path,
    track_id: &str,
    segments: &[SegmentSummary],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(TranscriptionProgress),
) -> Result<TranscriptionOutput, String> {
    let first = segments.iter().map(|s| s.start_us).min().unwrap_or(0);
    let last = segments.iter().map(|s| s.end_us).max().unwrap_or(0);
    let span = last.saturating_sub(first).max(1) as f64;
    let label = transcriber.kind().label();
    let mut words = Vec::new();
    let mut language = None;
    progress(TranscriptionProgress {
        track_id: track_id.into(),
        fraction: 0.0,
        message: format!("Preparing audio for {label}"),
    });
    let diagnostics = for_each_chunk(
        root,
        segments,
        transcriber.plan(),
        cancel,
        |_| {},
        |chunk| {
            progress(TranscriptionProgress {
                track_id: track_id.into(),
                fraction: (chunk.source_start_us.saturating_sub(first) as f64 / span)
                    .clamp(0.0, 1.0),
                message: format!("Transcribing with {label}"),
            });
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                return Err("Transcription cancelled".into());
            }
            let result = transcriber.transcribe_chunk(&chunk)?;
            let chunk_end = chunk.source_start_us + chunk.duration_us();
            for mut word in result.words {
                word.source_start_us =
                    (chunk.source_start_us + word.source_start_us).min(chunk_end);
                word.source_end_us = (chunk.source_start_us + word.source_end_us)
                    .clamp(word.source_start_us, chunk_end);
                if !word.text.trim().is_empty() {
                    word.text = word.text.trim().to_string();
                    words.push(word);
                }
            }
            if language.is_none() {
                language = result.language;
            }
            Ok(())
        },
    )?;
    if words.len() > super::MAX_WORDS {
        return Err("Transcript has too many words".into());
    }
    progress(TranscriptionProgress {
        track_id: track_id.into(),
        fraction: 1.0,
        message: "Saving transcript".into(),
    });
    let transcript = Transcript::new(
        track_id.into(),
        transcriber.kind(),
        transcriber.model(),
        language,
        words,
    );
    Ok(TranscriptionOutput {
        transcript,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::generate_pcm16_wav;
    use crate::transcript::test_word;

    struct Fake {
        calls: Vec<u64>,
    }

    impl ChunkTranscriber for Fake {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Parakeet
        }
        fn model(&self) -> String {
            "fake".into()
        }
        fn plan(&self) -> ChunkPlan {
            ChunkPlan {
                max_chunk_us: 1_000_000,
                search_window_us: 200_000,
            }
        }
        fn transcribe_chunk(&mut self, chunk: &AudioChunk) -> Result<ChunkResult, String> {
            self.calls.push(chunk.source_start_us);
            Ok(ChunkResult {
                words: vec![test_word("word", 100, 300), test_word(" ", 300, 310)],
                language: Some("en".into()),
            })
        }
    }

    #[test]
    fn offsets_words_by_chunk_start() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = generate_pcm16_wav(16_000, 1, &vec![500i16; 32_000]);
        std::fs::write(dir.path().join("a.wav"), &bytes).unwrap();
        let segments = vec![SegmentSummary {
            track_id: "mic".into(),
            relative_path: "a.wav".into(),
            start_us: 4_000_000,
            end_us: 6_000_000,
            size_bytes: bytes.len() as u64,
            media_timescale: 16_000,
            media_start_value: 0,
            host_anchor_us: 0,
            is_keyframe_start: None,
            available: true,
        }];
        let mut fake = Fake { calls: Vec::new() };
        let mut updates = 0;
        let out = transcribe_segments(
            &mut fake,
            dir.path(),
            "mic",
            &segments,
            &AtomicBool::new(false),
            &mut |_| updates += 1,
        )
        .unwrap();
        // Constant audio has no quiet point, so chunks split early in the search window.
        assert_eq!(fake.calls.len(), 3);
        assert_eq!(fake.calls[0], 4_000_000);
        // Blank words are dropped; one real word per chunk remains.
        assert_eq!(out.transcript.words.len(), 3);
        assert_eq!(out.transcript.words[0].source_start_us, 4_100_000);
        assert_eq!(
            out.transcript.words[1].source_start_us,
            fake.calls[1] + 100_000
        );
        assert_eq!(out.transcript.language.as_deref(), Some("en"));
        assert!(updates >= 3);
    }
}
