//! Gaps bounded by recognized words, irrespective of their loudness. Review is required:
//! a gap can contain a mouth noise, an intentional tutorial click, or an unrecognized word.
use super::{Transcript, WordKind};
use serde::{Deserialize, Serialize};

/// Secondary sound protection stays conservative even when microphone cuts get tighter.
pub const PC_AUDIO_PADDING_MS: u32 = 80;
/// A stretch without audio at least this long is the recorder paused, not a dropout: the dead
/// air on both sides of it goes like the recording's own start and end.
const RECORDING_PAUSE_US: u64 = 5_000_000;
/// Where the cut after the last word ends: past any recording, and exact as a JavaScript number.
pub(crate) const OPEN_END_US: u64 = 1 << 52;
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
    /// Keep the sounds the transcriber tagged between words ("[keyboard clacking]",
    /// "[laughs]"). Off by default: a gap without words is cut, typing and all.
    #[serde(default)]
    pub keep_sound_events: bool,
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

/// The words as spans in source time, each marked whether it is a confident anchor. Merging
/// overlapping tokens prevents an inner word from exposing the tail of an outer word as a gap.
/// Zero-length tokens are still protected.
fn word_spans(transcript: &Transcript, keep_sound_events: bool) -> Vec<(u64, u64, bool)> {
    let mut spans: Vec<(u64, u64, bool)> = Vec::new();
    for word in &transcript.words {
        if word.kind == WordKind::AudioEvent && !keep_sound_events {
            continue;
        }
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
    spans
}

/// Source-time candidates only: the gaps between words, plus [`edge_ranges`]. Tagged sounds
/// between words are cut with the gap unless `keep_sound_events`; then they stay intact.
pub(crate) fn gap_ranges(
    transcript: &Transcript,
    covered: &[(u64, u64)],
    config: &TranscriptGapConfig,
) -> Vec<(u64, u64)> {
    let spans = word_spans(transcript, config.keep_sound_events);
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
    gaps.extend(edge_ranges(
        transcript,
        &covered,
        config.padding_ms,
        config.min_duration_ms,
        config.keep_sound_events,
    ));
    gaps.sort_unstable();
    gaps
}

/// The dead air at the recording's start and end and on both sides of each pause in it
/// (settling in, reaching for the stop button): before the first word, after the last, and
/// between the words either side of a pause, keeping `padding_ms` next to the words; the head
/// only when at least `min_duration_ms` long. Only next to a confident word, and cut even where
/// the microphone has no audio to show for it: the picture often starts before it and runs on
/// after it, and would stay as a sliver. No other sound's veto keeps these (but other speech
/// does: someone else may talk there).
pub(crate) fn edge_ranges(
    transcript: &Transcript,
    covered: &[(u64, u64)],
    padding_ms: u32,
    min_duration_ms: u32,
    keep_sound_events: bool,
) -> Vec<(u64, u64)> {
    let spans = word_spans(transcript, keep_sound_events);
    let covered = merge_ranges(covered.to_vec());
    let padding = u64::from(padding_ms) * 1_000;
    let min_us = u64::from(min_duration_ms) * 1_000;
    let mut edges = Vec::new();
    if let Some(&(first, _, true)) = spans.first() {
        let end = first.saturating_sub(padding);
        if first >= min_us && end > 0 {
            edges.push((0, end));
        }
    }
    for pair in spans.windows(2) {
        let ((_, left, left_anchor), (right, _, right_anchor)) = (pair[0], pair[1]);
        let heard: u64 = covered
            .iter()
            .map(|&(a, b)| b.min(right).saturating_sub(a.max(left)))
            .sum();
        let (start, end) = (left.saturating_add(padding), right.saturating_sub(padding));
        if left_anchor
            && right_anchor
            && (right - left).saturating_sub(heard) >= RECORDING_PAUSE_US
            && end > start
        {
            edges.push((start, end));
        }
    }
    if let Some(&(_, last, true)) = spans.last() {
        edges.push((last.saturating_add(padding), OPEN_END_US));
    }
    edges
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

/// Quiet this long inside a word is a pause the word swallowed, not a stop consonant.
const SWALLOWED_PAUSE_US: u64 = 150_000;
/// Sound a swallowed pause cuts off from a word is still the word when at least this share of
/// the word's longest sound.
const DETACHED_SHARE: f64 = 0.4;

/// Parakeet times words in 80 ms steps and stretches each one over the pause next to it, so its
/// words touch and the pauses hide inside them. This tightens such words to their own sound:
/// quiet at either edge goes whatever its length. With `drop_detached`, so does sound a
/// swallowed pause cuts off from the word (a click or breath in that pause) when it is much
/// shorter than the word's own sound; only the mouth-noise pass asks for that, as it hears for
/// itself whether such a sound is a voice (a misplaced short word would be). A wholly quiet
/// word, an uncertain one and one without read audio stay as they are.
pub(crate) fn tighten_loose_words(
    transcript: &Transcript,
    quiet: &[(u64, u64)],
    covered: &[(u64, u64)],
    drop_detached: bool,
) -> Transcript {
    let mut tightened = transcript.clone();
    let quiet = merge_ranges(quiet.to_vec());
    let covered = merge_ranges(covered.to_vec());
    for word in &mut tightened.words {
        if word.kind != WordKind::Word || word.confidence.is_some_and(|c| c < 0.5) {
            continue;
        }
        let (start, end) = (word.source_start_us, word.source_end_us);
        let at = covered.partition_point(|&(_, b)| b <= start);
        if end <= start
            || !covered
                .get(at)
                .is_some_and(|&(a, b)| a <= start && b >= end)
        {
            continue;
        }
        let first = quiet.partition_point(|&(_, b)| b <= start);
        let gaps: Vec<(u64, u64)> = quiet[first..]
            .iter()
            .take_while(|&&(a, _)| a < end)
            .map(|&(a, b)| (a.max(start), b.min(end)))
            .filter(|&(a, b)| {
                a == start || b == end || (drop_detached && b - a >= SWALLOWED_PAUSE_US)
            })
            .collect();
        let pieces = subtract_ranges(&[(start, end)], &gaps);
        let Some(longest) = pieces.iter().map(|&(a, b)| b - a).max() else {
            continue;
        };
        let kept: Vec<&(u64, u64)> = pieces
            .iter()
            .filter(|&&(a, b)| (b - a) as f64 >= longest as f64 * DETACHED_SHARE)
            .collect();
        let (a, b) = (kept[0].0, kept[kept.len() - 1].1);
        if b - a >= 40_000 {
            word.source_start_us = a;
            word.source_end_us = b;
        }
    }
    tightened
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

/// `ranges` less `remove` (both any order), merged and in order.
pub(crate) fn subtract_ranges(ranges: &[(u64, u64)], remove: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let mut out = merge_ranges(ranges.to_vec());
    for &(a, b) in &merge_ranges(remove.to_vec()) {
        out = out
            .into_iter()
            .flat_map(|(x, y)| {
                if b <= x || a >= y {
                    vec![(x, y)]
                } else {
                    [(x, a.max(x)), (b.min(y), y)]
                        .into_iter()
                        .filter(|(p, q)| q > p)
                        .collect()
                }
            })
            .collect();
    }
    out
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
            keep_sound_events: false,
        }
    }
    #[test]
    fn gaps_preserve_word_edges_and_cut_dead_air_at_both_ends() {
        let t = transcript(vec![
            test_word("hello", 200, 1000),
            test_word("world", 2000, 2800),
        ]);
        // 200 ms before "hello" is under the minimum; everything after "world" goes, past the
        // audio, where the picture may run on.
        assert_eq!(
            gap_ranges(&t, &[(0, 4_000_000)], &config()),
            vec![(1_120_000, 1_880_000), (2_920_000, OPEN_END_US)]
        );
        let tight = TranscriptGapConfig {
            padding_ms: 0,
            ..config()
        };
        assert_eq!(
            gap_ranges(&t, &[(0, 4_000_000)], &tight),
            vec![(1_000_000, 2_000_000), (2_800_000, OPEN_END_US)]
        );
        // Seconds of settling in before the first word go, from the very start: also where the
        // microphone starts late or drops out (FIX-4).
        let late = transcript(vec![
            test_word("hello", 3000, 3500),
            test_word("world", 4500, 5000),
        ]);
        let expected = vec![
            (0, 2_880_000),
            (3_620_000, 4_380_000),
            (5_120_000, OPEN_END_US),
        ];
        for covered in [
            vec![(1_000_000, 5_000_000)],
            vec![(0, 1_000_000), (2_000_000, 5_000_000)],
            vec![(2_900_000, 5_000_000)],
        ] {
            assert_eq!(gap_ranges(&late, &covered, &config()), expected);
        }
    }

    #[test]
    fn a_recording_pause_is_cut_across_but_a_dropout_is_not() {
        let t = transcript(vec![
            test_word("end", 1000, 2000),
            test_word("start", 20_000, 21_000),
        ]);
        // The recorder paused from 2.5 s to 19 s: the dead air either side goes, so no sliver
        // of picture stays at the join.
        assert_eq!(
            gap_ranges(&t, &[(0, 2_500_000), (19_000_000, 22_000_000)], &config()),
            vec![
                (0, 880_000),
                (2_120_000, 19_880_000),
                (21_120_000, OPEN_END_US)
            ]
        );
        // A second without audio is a dropout: the picture there may matter, so it stays.
        assert_eq!(
            gap_ranges(&t, &[(0, 2_500_000), (3_500_000, 22_000_000)], &config()),
            vec![(0, 880_000), (21_120_000, OPEN_END_US)]
        );
    }
    #[test]
    fn tagged_sounds_between_words_are_cut_unless_kept() {
        let mut typing = test_word("[keyboard clacking]", 1100, 9000);
        typing.kind = WordKind::AudioEvent;
        let t = transcript(vec![
            test_word("hello", 200, 1000),
            typing,
            test_word("world", 9500, 10_000),
        ]);
        let covered = [(0, 11_000_000)];
        assert_eq!(
            gap_ranges(&t, &covered, &config()),
            vec![(1_120_000, 9_380_000), (10_120_000, OPEN_END_US)],
            "the typing goes with the gap"
        );
        let keep = TranscriptGapConfig {
            keep_sound_events: true,
            ..config()
        };
        assert_eq!(
            gap_ranges(&t, &covered, &keep),
            vec![(10_120_000, OPEN_END_US)]
        );
    }

    #[test]
    fn uncertain_anchors_are_kept() {
        let mut t = transcript(vec![
            test_word("hello", 200, 1000),
            test_word("world", 2000, 2800),
        ]);
        // Only the dead air after the confident last word goes.
        let tail = vec![(2_920_000, OPEN_END_US)];
        t.words[0].confidence = Some(0.3);
        assert_eq!(gap_ranges(&t, &[(0, 2_800_000)], &config()), tail);
        t.words[0].confidence = None;
        t.words[0].source_end_us = t.words[0].source_start_us;
        assert_eq!(gap_ranges(&t, &[(0, 2_800_000)], &config()), tail);
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
            gap_ranges(&t, &[(0, 230_000)], &cfg),
            vec![(100_000, 130_000), (230_000, OPEN_END_US)]
        );
        cfg.padding_ms = 10;
        assert_eq!(
            gap_ranges(&t, &[(0, 230_000)], &cfg),
            vec![(110_000, 120_000), (240_000, OPEN_END_US)]
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
            vec![(300_000, 500_000), (800_000, OPEN_END_US)]
        );
        assert_eq!(
            gap_ranges(&t, &[(0, 800_000)], &cfg),
            vec![(800_000, OPEN_END_US)]
        );
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
    fn loose_words_give_back_the_pauses_they_swallowed() {
        // "these" sounds 0.0-0.3 s, then a pause with a click at 0.5-0.56 s, all timed as one
        // word up to 0.8 s where "six" starts; "six" sounds 0.9-1.2 s.
        let t = transcript(vec![
            test_word("these", 0, 800),
            test_word("six", 800, 1200),
        ]);
        let quiet = [(300_000, 500_000), (560_000, 900_000)];
        let spans = |t: &Transcript| {
            t.words
                .iter()
                .map(|w| (w.source_start_us, w.source_end_us))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            spans(&tighten_loose_words(&t, &quiet, &[(0, 2_000_000)], false)),
            vec![(0, 560_000), (900_000, 1_200_000)],
            "the quiet after the click goes; the click may be a misplaced word"
        );
        let tight = tighten_loose_words(&t, &quiet, &[(0, 2_000_000)], true);
        assert_eq!(
            spans(&tight),
            vec![(0, 300_000), (900_000, 1_200_000)],
            "for the noise pass, the click cut off by the pause goes with the pause"
        );
        // The gaps pass: the pause after the click is cut (muting the click first cuts it all).
        let gaps = gap_ranges(
            &tighten_loose_words(&t, &quiet, &[(0, 2_000_000)], false),
            &[(0, 2_000_000)],
            &TranscriptGapConfig {
                min_duration_ms: 150,
                padding_ms: 40,
                ..config()
            },
        );
        assert_eq!(gaps, vec![(600_000, 860_000), (1_240_000, OPEN_END_US)]);
        assert!(tight.words[0].source_end_us == 300_000);
        // A stop consonant's short closure stays inside the word, and so does a quiet word.
        let t = transcript(vec![
            test_word("compact", 0, 600),
            test_word("hm", 1000, 1200),
        ]);
        let quiet = [(250_000, 330_000), (1_000_000, 1_200_000)];
        let tight = tighten_loose_words(&t, &quiet, &[(0, 2_000_000)], true);
        assert_eq!(
            (tight.words[0].source_start_us, tight.words[0].source_end_us),
            (0, 600_000)
        );
        assert_eq!(
            (tight.words[1].source_start_us, tight.words[1].source_end_us),
            (1_000_000, 1_200_000)
        );
    }

    #[test]
    fn missing_pcm_is_kept_and_audio_events_only_when_asked() {
        let mut t = transcript(vec![
            test_word("hello", 200, 1000),
            test_word("world", 2000, 2800),
        ]);
        // Only the dead air after the last word goes: the gap between has no audio to show.
        let tail = (2_920_000, OPEN_END_US);
        assert_eq!(
            gap_ranges(&t, &[(0, 1_500_000), (1_600_000, 2_800_000)], &config()),
            vec![tail]
        );
        assert_eq!(gap_ranges(&t, &[(0, 2_100_000)], &config()), vec![tail]);
        let mut event = test_word("(laughter)", 1400, 1600);
        event.kind = WordKind::AudioEvent;
        t.words.insert(1, event);
        let keep = TranscriptGapConfig {
            keep_sound_events: true,
            ..config()
        };
        assert_eq!(gap_ranges(&t, &[(0, 2_800_000)], &keep), vec![tail]);
        assert_eq!(
            gap_ranges(&t, &[(0, 2_800_000)], &config()),
            vec![(1_120_000, 1_880_000), tail]
        );
    }
    #[test]
    fn overlapping_and_short_words_stay_protected() {
        let t = transcript(vec![
            test_word("outer", 200, 1800),
            test_word("inner", 500, 700),
            test_word("next", 2000, 2800),
        ]);
        let tail = (2_920_000, OPEN_END_US);
        assert_eq!(gap_ranges(&t, &[(0, 2_800_000)], &config()), vec![tail]);
        let short = transcript(vec![
            test_word("a", 990, 1000),
            test_word("world", 2000, 2800),
        ]);
        assert_eq!(
            gap_ranges(&short, &[(0, 2_800_000)], &config()),
            vec![(0, 870_000), (1_120_000, 1_880_000), tail]
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
