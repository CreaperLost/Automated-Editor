//! Visuals (FCL-6): the graphics on a production's timeline, checked as the explainer's
//! storyboard gate checked its parts. The agent places graphics with `graphic_add` at the
//! narration's words (`atWord`, `actionWord`); this reads what is there and says what fails:
//! a stretch with nothing on screen, more than 12 seconds without a visual change, a card that
//! needs more time to read than it shows, the same kind twice in a row, a graphic that starts
//! between words, too few kinds in a long video. `fill` stretches each graphic to meet the
//! next, so the picture never drops to the background.
use super::Format;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A visual change at least this often (the explainer's "no more than 12 seconds").
pub const MAX_HOLD_US: u64 = 12_000_000;
/// Reading speed: about three words a second.
pub const WORDS_PER_SECOND: f64 = 3.0;
/// A stretch shorter than this with nothing on screen is a seam, not a gap.
const SEAM_US: u64 = 400_000;
/// A graphic starts on a word when it starts this close to one.
const ON_WORD_US: u64 = 300_000;
/// A long video uses at least this many kinds.
const LONG_KINDS: usize = 6;
/// Variables that aren't read as words on screen.
const NOT_READ: [&str; 4] = ["emphasis", "highlight", "unit", "source"];

/// A graphic on the timeline, as the check sees it.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Beat {
    pub asset_id: String,
    pub clip_id: String,
    pub name: String,
    /// The template it was made from; none for a graphic of the agent's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    pub start_us: u64,
    pub end_us: u64,
    /// When its action lands, on the timeline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cue_us: Option<u64>,
    /// Words it shows.
    pub words: usize,
    /// The narration under it, its first words.
    #[serde(default)]
    pub said: String,
}

/// `faceless/beats.json`: the visuals stage's file.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Visuals {
    pub beats: Vec<Beat>,
    pub duration_us: u64,
    /// What blocks the stage.
    pub errors: Vec<String>,
    /// What is worth a look.
    pub notes: Vec<String>,
    pub at: String,
}

pub fn path(root: &Path) -> std::path::PathBuf {
    root.join(super::DIR).join("beats.json")
}

pub fn load(root: &Path) -> Option<Visuals> {
    crate::files::load_json(&path(root))
}

pub fn save(root: &Path, visuals: &Visuals) -> Result<(), String> {
    crate::files::save_json(&path(root), visuals)
}

fn clock(us: u64) -> String {
    let s = us / 1_000_000;
    format!("{}:{:02}.{}", s / 60, s % 60, (us % 1_000_000) / 100_000)
}

/// The graphics on the video's timeline, in order of start.
pub fn gather(document: &crate::project::revision::EditDocument, root: &Path) -> Vec<Beat> {
    let mut beats: Vec<Beat> = document
        .sequence
        .clips()
        .filter(|(track, _)| track.kind == crate::sequence::TrackKind::Video && !track.hidden)
        .filter_map(|(_, clip)| {
            let asset = document.asset(&clip.asset)?;
            if !asset.is_graphic() {
                return None;
            }
            let dir = crate::graphics::folder(root, &asset.id);
            let source = crate::graphics::read_source(&dir).ok();
            let declared = crate::graphics::declarations(&dir).unwrap_or_default();
            let value = |id: &str| {
                source
                    .as_ref()
                    .and_then(|s| s.variables.get(id).cloned())
                    .or_else(|| {
                        declared
                            .iter()
                            .find(|d| d.id == id)
                            .map(|d| d.default.clone())
                    })
            };
            let words = declared
                .iter()
                .filter(|d| d.kind == "string" && !NOT_READ.contains(&d.id.as_str()))
                .filter_map(|d| value(&d.id))
                .filter_map(|v| v.as_str().map(|s| s.replace('|', " ")))
                .map(|s| s.split_whitespace().count())
                .sum();
            let cue_us = declared
                .iter()
                .any(|d| d.id == "cue")
                .then(|| value("cue").and_then(|v| v.as_f64()))
                .flatten()
                .filter(|s| s.is_finite() && *s >= 0.0)
                .map(|s| clip.start_us + (s * 1e6) as u64)
                .filter(|&at| at < clip.end_us());
            Some(Beat {
                asset_id: asset.id.clone(),
                clip_id: clip.id.clone(),
                name: asset.name.clone(),
                template: source.and_then(|s| s.template),
                start_us: clip.start_us,
                end_us: clip.end_us(),
                cue_us,
                words,
                said: String::new(),
            })
        })
        .collect();
    beats.sort_by_key(|b| (b.start_us, b.end_us));
    beats
}

/// The narration under each beat, its first words.
pub fn caption(beats: &mut [Beat], words: &[(u64, String)]) {
    for beat in beats {
        let said: Vec<&str> = words
            .iter()
            .filter(|(t, _)| (beat.start_us..beat.end_us).contains(t))
            .map(|(_, w)| w.as_str())
            .take(10)
            .collect();
        beat.said = said.join(" ");
    }
}

/// Each graphic's new end so it meets the next one (and the last the end of the video):
/// `(clip id, end)` for those that fall short.
pub fn fills(beats: &[Beat], duration_us: u64) -> Vec<(String, u64)> {
    // A graphic inside another (on a layer above it) is left as it is.
    let main: Vec<&Beat> = beats
        .iter()
        .enumerate()
        .filter(|(i, b)| {
            !beats[..*i]
                .iter()
                .any(|o| o.start_us <= b.start_us && o.end_us >= b.end_us)
        })
        .map(|(_, b)| b)
        .collect();
    let mut out = Vec::new();
    for (i, beat) in main.iter().enumerate() {
        let next = main.get(i + 1).map_or(duration_us, |b| b.start_us);
        if beat.end_us < next {
            out.push((beat.clip_id.clone(), next));
        }
    }
    out
}

/// What blocks the visuals (errors) and what is worth a look (notes). `words`: the narration's
/// words with their times on the timeline.
pub fn judge(
    beats: &[Beat],
    words: &[(u64, String)],
    duration_us: u64,
    format: Format,
) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut notes = Vec::new();
    if beats.is_empty() {
        errors.push("No graphics on the timeline yet: place them with `graphic_add` at the narration's words.".into());
        return (errors, notes);
    }
    // Nothing on screen.
    let mut cursor = 0;
    let mut gaps = Vec::new();
    for beat in beats {
        if beat.start_us > cursor + SEAM_US {
            gaps.push((cursor, beat.start_us));
        }
        cursor = cursor.max(beat.end_us);
    }
    if duration_us > cursor + SEAM_US {
        gaps.push((cursor, duration_us));
    }
    for (a, b) in &gaps {
        errors.push(format!(
            "Nothing on screen from {} to {} ({:.1} s): start a graphic there or stretch the one before (`fill: true`).",
            clock(*a),
            clock(*b),
            (b - a) as f64 / 1e6
        ));
    }
    // Visual changes: each start and each action.
    let mut changes: Vec<u64> = beats
        .iter()
        .flat_map(|b| std::iter::once(b.start_us).chain(b.cue_us))
        .collect();
    changes.push(duration_us);
    changes.sort_unstable();
    changes.dedup();
    let mut last = 0;
    for &at in &changes {
        if at > last + MAX_HOLD_US {
            notes.push(format!(
                "No visual change for {:.0} s from {} to {}: add a beat, or an action word (`actionWord`) inside.",
                (at - last) as f64 / 1e6,
                clock(last),
                clock(at)
            ));
        }
        last = at;
    }
    for beat in beats {
        let shown = (beat.end_us - beat.start_us) as f64 / 1e6;
        let needed = beat.words as f64 / WORDS_PER_SECOND;
        if needed > shown + 0.5 {
            notes.push(format!(
                "\"{}\" at {} shows {} words for {shown:.1} s; reading them takes about {needed:.0} s: cut words or hold it longer.",
                beat.name,
                clock(beat.start_us),
                beat.words
            ));
        }
        let on_word = beat.start_us < ON_WORD_US
            || words
                .iter()
                .any(|(t, _)| t.abs_diff(beat.start_us) <= ON_WORD_US);
        if !on_word {
            notes.push(format!(
                "\"{}\" starts at {} between words: anchor it on the word it shows (`atWord`).",
                beat.name,
                clock(beat.start_us)
            ));
        }
    }
    for pair in beats.windows(2) {
        if let (Some(a), Some(b)) = (&pair[0].template, &pair[1].template) {
            if a == b && a != "blank" {
                notes.push(format!(
                    "Two {a} graphics in a row at {}: use another kind unless the same one explains better.",
                    clock(pair[1].start_us)
                ));
            }
        }
    }
    let mut kinds: Vec<&str> = beats
        .iter()
        .map(|b| b.template.as_deref().unwrap_or(b.name.as_str()))
        .collect();
    kinds.sort_unstable();
    kinds.dedup();
    if format == Format::Long && kinds.len() < LONG_KINDS {
        notes.push(format!(
            "A long video uses at least {LONG_KINDS} kinds of graphic; this one has {}.",
            kinds.len()
        ));
    }
    (errors, notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beat(id: &str, template: &str, start: u64, end: u64) -> Beat {
        Beat {
            asset_id: id.into(),
            clip_id: format!("c-{id}"),
            name: id.into(),
            template: Some(template.into()),
            start_us: start * 1_000_000,
            end_us: end * 1_000_000,
            ..Default::default()
        }
    }

    fn words(at: &[u64]) -> Vec<(u64, String)> {
        at.iter()
            .map(|&s| (s * 1_000_000, "word".to_string()))
            .collect()
    }

    #[test]
    fn gaps_block_and_long_holds_repeats_and_reading_are_noted() {
        let mut beats = vec![
            beat("a", "headline", 0, 5),
            beat("b", "quote", 8, 30),
            beat("c", "quote", 30, 40),
        ];
        beats[1].words = 90;
        let (errors, notes) = judge(&beats, &words(&[0, 8, 30]), 44_000_000, Format::Long);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("0:05.0 to 0:08.0"), "{errors:?}");
        assert!(errors[1].contains("0:40.0 to 0:44.0"), "{errors:?}");
        let all = notes.join("\n");
        assert!(all.contains("No visual change for 22 s"), "{all}");
        assert!(all.contains("90 words"), "{all}");
        assert!(all.contains("Two quote graphics in a row"), "{all}");
        assert!(all.contains("at least 6 kinds"), "{all}");
        // An action inside the long hold is a change; a short needs fewer kinds.
        beats[1].cue_us = Some(19_000_000);
        let (_, notes) = judge(&beats, &words(&[0, 8, 30]), 44_000_000, Format::Short);
        assert!(!notes.join("\n").contains("for 22 s"), "{notes:?}");
        assert!(!notes.join("\n").contains("kinds"));
    }

    #[test]
    fn a_graphic_between_words_is_noted() {
        let beats = vec![beat("a", "headline", 0, 6), beat("b", "bars", 6, 12)];
        let (errors, notes) = judge(&beats, &words(&[0, 2]), 12_000_000, Format::Short);
        assert!(errors.is_empty());
        assert!(
            notes.iter().any(|n| n.contains("between words")),
            "{notes:?}"
        );
    }

    #[test]
    fn fill_stretches_each_graphic_to_the_next() {
        let beats = vec![
            beat("a", "headline", 0, 5),
            beat("b", "quote", 8, 20),
            // On a layer above, inside b: left alone, and b still meets c.
            beat("tag", "blank", 10, 12),
            beat("c", "bars", 24, 30),
        ];
        let fills = fills(&beats, 40_000_000);
        assert_eq!(
            fills,
            [
                ("c-a".to_string(), 8_000_000),
                ("c-b".to_string(), 24_000_000),
                ("c-c".to_string(), 40_000_000),
            ]
        );
    }
}
