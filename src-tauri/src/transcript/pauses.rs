//! Gaps bounded by recognized words, irrespective of their loudness. Review is required:
//! a gap can contain a mouth noise, an intentional tutorial click, or an unrecognized word.
use super::{Transcript, WordKind};
use serde::{Deserialize, Serialize};

/// Secondary sound protection stays conservative even when microphone cuts get tighter.
pub const PC_AUDIO_PADDING_MS: u32 = 80;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptGapConfig {
    pub min_duration_ms: u32,
    pub padding_ms: u32,
    /// Analysis-only quiet-edge refinement; saved transcript/caption timings stay intact.
    #[serde(default)]
    pub refine_word_edges: bool,
    #[serde(default = "default_edge_threshold")]
    pub edge_threshold_db: f32,
}

fn default_edge_threshold() -> f32 {
    -42.0
}

impl TranscriptGapConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.min_duration_ms == 0 {
            return Err("Minimum gap duration must be greater than zero".into());
        }
        if !self.edge_threshold_db.is_finite() || !(-90.0..=-10.0).contains(&self.edge_threshold_db)
        {
            return Err("Word edge threshold must be between -90 and -10 dB".into());
        }
        Ok(())
    }
}

/// Source-time candidates only. Never infer a cut before the first word or after the
/// last one: those regions may not have been transcribed. Audio events also stay intact.
pub(crate) fn gap_ranges(
    transcript: &Transcript,
    covered: &[(u64, u64)],
    config: &TranscriptGapConfig,
) -> Vec<(u64, u64)> {
    // Merging overlapping tokens prevents an inner word from exposing the tail of an
    // outer word as a gap. Zero-length tokens are still protected.
    let mut spans: Vec<(u64, u64, bool)> = Vec::new();
    for word in &transcript.words {
        let anchor = word.kind == WordKind::Word
            && word.source_end_us > word.source_start_us
            && word.confidence.is_none_or(|confidence| confidence >= 0.5);
        match spans.last_mut() {
            Some(last) if word.source_start_us <= last.1 => {
                last.1 = last.1.max(word.source_end_us);
                last.2 &= anchor;
            }
            _ => spans.push((word.source_start_us, word.source_end_us, anchor)),
        }
    }
    let covered = merge_ranges(covered.to_vec());
    let padding = u64::from(config.padding_ms) * 1_000;
    let mut gaps = Vec::new();
    for pair in spans.windows(2) {
        let (left, right) = (pair[0], pair[1]);
        if !left.2
            || !right.2
            || right.0.saturating_sub(left.1) < u64::from(config.min_duration_ms) * 1_000
        {
            continue;
        }
        let start = left.1.saturating_add(padding);
        let end = right.0.saturating_sub(padding);
        if end <= start {
            continue;
        }
        // Require real, continuous PCM coverage over both anchors and their gap.
        let at = covered.partition_point(|&(_, end)| end <= left.0);
        if !covered
            .get(at)
            .is_some_and(|&(a, b)| a <= left.0 && b >= right.1)
        {
            continue;
        }
        gaps.push((start, end));
    }
    gaps
}

/// The silence pass can use saved words as an additional guard without requiring ASR.
pub(crate) fn protect_words(
    ranges: Vec<(u64, u64)>,
    transcript: &Transcript,
    padding_ms: u32,
) -> Vec<(u64, u64)> {
    let pad = u64::from(padding_ms) * 1_000;
    let protected = merge_ranges(
        transcript
            .words
            .iter()
            .map(|w| {
                (
                    w.source_start_us.saturating_sub(pad),
                    w.source_end_us.saturating_add(pad),
                )
            })
            .collect(),
    );
    let mut out = Vec::new();
    for (a, b) in ranges {
        let mut cursor = a;
        let first = protected.partition_point(|&(_, end)| end <= a);
        for &(start, end) in &protected[first..] {
            if start >= b {
                break;
            }
            if start > cursor {
                out.push((cursor, start.min(b)));
            }
            cursor = cursor.max(end);
        }
        if cursor < b {
            out.push((cursor, b));
        }
    }
    out
}

/// Trim only measured quiet tails touching a timestamp. Never infer speech from amplitude,
/// trim a wholly quiet word, or alter audio events/uncertain words. Limited to 200 ms on
/// either edge with a 40 ms core; review remains necessary for soft consonants.
pub(crate) fn refine_quiet_edges(
    transcript: &Transcript,
    quiet: &[(u64, u64)],
    covered: &[(u64, u64)],
) -> Transcript {
    let mut refined = transcript.clone();
    let quiet = merge_ranges(quiet.to_vec());
    let covered = merge_ranges(covered.to_vec());
    for word in &mut refined.words {
        if word.kind != WordKind::Word || word.confidence.is_some_and(|c| c < 0.5) {
            continue;
        }
        let (start, end) = (word.source_start_us, word.source_end_us);
        let coverage_at = covered.partition_point(|&(_, b)| b <= start);
        if end <= start
            || !covered
                .get(coverage_at)
                .is_some_and(|&(a, b)| a <= start && b >= end)
        {
            continue;
        }
        let mut a = start;
        let mut b = end;
        let first = quiet.partition_point(|&(_, b)| b <= start);
        for &(q_start, q_end) in &quiet[first..] {
            if q_start >= end {
                break;
            }
            // A quiet word has no verified sounding core: its timestamps stay protected.
            if q_start <= start && q_end >= end {
                a = start;
                b = end;
                break;
            }
            if q_start <= start && q_end > start && q_end - start <= 200_000 {
                a = q_end;
            }
            if q_start < end && q_end >= end && end - q_start <= 200_000 {
                b = q_start;
            }
        }
        if b.saturating_sub(a) >= 40_000 {
            word.source_start_us = a;
            word.source_end_us = b;
        }
    }
    refined
}

pub(crate) fn merge_ranges(mut ranges: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    ranges.retain(|&(a, b)| b > a);
    ranges.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::new();
    for (a, b) in ranges {
        match merged.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::{test_word, ProviderKind, TranscriptWord};
    fn transcript(words: Vec<TranscriptWord>) -> Transcript {
        Transcript::new(
            "mic".into(),
            ProviderKind::Parakeet,
            "test".into(),
            None,
            words,
        )
    }
    fn config() -> TranscriptGapConfig {
        TranscriptGapConfig {
            min_duration_ms: 500,
            padding_ms: 120,
            refine_word_edges: false,
            edge_threshold_db: -42.0,
        }
    }
    #[test]
    fn gaps_preserve_word_edges_and_leading_trailing_audio() {
        let t = transcript(vec![
            test_word("hello", 200, 1000),
            test_word("world", 2000, 2800),
        ]);
        assert_eq!(
            gap_ranges(&t, &[(0, 4_000_000)], &config()),
            vec![(1_120_000, 1_880_000)]
        );
        let tight = TranscriptGapConfig {
            padding_ms: 0,
            ..config()
        };
        assert_eq!(
            gap_ranges(&t, &[(0, 4_000_000)], &tight),
            vec![(1_000_000, 2_000_000)]
        );
    }
    #[test]
    fn uncertain_anchors_are_kept() {
        let mut t = transcript(vec![
            test_word("hello", 200, 1000),
            test_word("world", 2000, 2800),
        ]);
        t.words[0].confidence = Some(0.3);
        assert!(gap_ranges(&t, &[(0, 4_000_000)], &config()).is_empty());
        t.words[0].confidence = None;
        t.words[0].source_end_us = t.words[0].source_start_us;
        assert!(gap_ranges(&t, &[(0, 4_000_000)], &config()).is_empty());
    }

    #[test]
    fn tight_gaps_and_zero_padding_are_honored() {
        let t = transcript(vec![
            test_word("left", 0, 100),
            test_word("right", 130, 230),
        ]);
        let mut cfg = config();
        cfg.min_duration_ms = 20;
        cfg.padding_ms = 0;
        assert_eq!(
            gap_ranges(&t, &[(0, 300_000)], &cfg),
            vec![(100_000, 130_000)]
        );
        cfg.padding_ms = 10;
        assert_eq!(
            gap_ranges(&t, &[(0, 300_000)], &cfg),
            vec![(110_000, 120_000)]
        );
        assert_eq!(
            protect_words(vec![(0, 300_000)], &t, 0),
            vec![(100_000, 130_000), (230_000, 300_000)]
        );
    }

    #[test]
    fn measured_word_tails_expose_gaps_without_rewriting_the_transcript() {
        let t = transcript(vec![
            test_word("left", 0, 400),
            test_word("right", 400, 800),
        ]);
        let refined = refine_quiet_edges(&t, &[(300_000, 500_000)], &[(0, 800_000)]);
        assert_eq!(refined.words[0].source_end_us, 300_000);
        assert_eq!(refined.words[1].source_start_us, 500_000);
        assert_eq!(t.words[0].source_end_us, 400_000);
        assert_eq!(t.words[1].source_start_us, 400_000);
        let cfg = TranscriptGapConfig {
            min_duration_ms: 20,
            padding_ms: 0,
            ..config()
        };
        assert_eq!(
            gap_ranges(&refined, &[(0, 800_000)], &cfg),
            vec![(300_000, 500_000)]
        );
        assert!(gap_ranges(&t, &[(0, 800_000)], &cfg).is_empty());
    }

    #[test]
    fn refinement_keeps_uncertain_short_quiet_and_uncovered_words() {
        let mut t = transcript(vec![
            test_word("quiet", 0, 100),
            test_word("short", 200, 250),
            test_word("uncertain", 300, 500),
            test_word("(click)", 600, 800),
            test_word("uncovered", 900, 1100),
            test_word("long tail", 1200, 1600),
        ]);
        t.words[2].confidence = Some(0.3);
        t.words[3].kind = WordKind::AudioEvent;
        let quiet = [
            (0, 100_000),
            (225_000, 250_000),
            (400_000, 500_000),
            (700_000, 800_000),
            (1_000_000, 1_100_000),
            (1_300_000, 1_600_000),
        ];
        let refined = refine_quiet_edges(&t, &quiet, &[(0, 1_000_000), (1_200_000, 1_600_000)]);
        assert_eq!(t.words, refined.words);
    }
    #[test]
    fn missing_pcm_and_audio_events_are_kept() {
        let mut t = transcript(vec![
            test_word("hello", 200, 1000),
            test_word("world", 2000, 2800),
        ]);
        assert!(gap_ranges(&t, &[(0, 1_500_000), (1_600_000, 4_000_000)], &config()).is_empty());
        assert!(gap_ranges(&t, &[(0, 2_100_000)], &config()).is_empty());
        let mut event = test_word("(laughter)", 1400, 1600);
        event.kind = WordKind::AudioEvent;
        t.words.insert(1, event);
        assert!(gap_ranges(&t, &[(0, 4_000_000)], &config()).is_empty());
    }
    #[test]
    fn overlapping_and_short_words_stay_protected() {
        let t = transcript(vec![
            test_word("outer", 200, 1800),
            test_word("inner", 500, 700),
            test_word("next", 2000, 2800),
        ]);
        assert!(gap_ranges(&t, &[(0, 4_000_000)], &config()).is_empty());
        let short = transcript(vec![
            test_word("a", 990, 1000),
            test_word("world", 2000, 2800),
        ]);
        assert_eq!(
            gap_ranges(&short, &[(0, 4_000_000)], &config()),
            vec![(1_120_000, 1_880_000)]
        );
    }
    #[test]
    fn optional_word_guard_subtracts_words_and_audio_events_from_silence() {
        let mut event = test_word("(laughter)", 2100, 2200);
        event.kind = WordKind::AudioEvent;
        let t = transcript(vec![test_word("quiet", 1000, 1800), event]);
        assert_eq!(
            protect_words(vec![(0, 3_000_000)], &t, 120),
            vec![(0, 880_000), (1_920_000, 1_980_000), (2_320_000, 3_000_000)]
        );
    }
}
