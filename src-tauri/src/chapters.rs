//! Chapter markers. Each chapter is anchored at a source time, like zooms and words, so it
//! stays on the same moment through cuts, undo and reordering. Export writes them into the
//! MP4 as chapter metadata; the UI also offers them as a YouTube description list.
use crate::project::revision::EditDocument;
use serde::{Deserialize, Serialize};

pub const MAX_CHAPTERS: usize = 200;
pub const MAX_TITLE_CHARS: usize = 100;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: String,
    pub source_us: u64,
    pub title: String,
    /// The asset whose time `source_us` is in; `None` is the first recording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Where the chapter starts on the edited timeline; absent when that moment was cut.
    /// Filled in for the UI and cleared before the document is stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_us: Option<u64>,
}

pub fn validate(chapters: &[Chapter]) -> Result<(), String> {
    if chapters.len() > MAX_CHAPTERS {
        return Err(format!("Use at most {MAX_CHAPTERS} chapters"));
    }
    let mut ids = std::collections::HashSet::new();
    for chapter in chapters {
        if chapter.id.is_empty() || chapter.id.len() > 64 || !ids.insert(chapter.id.as_str()) {
            return Err("Chapter ids must be unique".into());
        }
        let title = chapter.title.trim();
        if title.is_empty() {
            return Err("A chapter needs a title".into());
        }
        if title.chars().count() > MAX_TITLE_CHARS || title.chars().any(char::is_control) {
            return Err(format!(
                "Chapter titles are one line of at most {MAX_TITLE_CHARS} characters"
            ));
        }
    }
    Ok(())
}

/// Trims titles, drops UI-only fields and orders by source time, so equal edits compare equal.
pub fn normalized(mut chapters: Vec<Chapter>) -> Vec<Chapter> {
    for chapter in &mut chapters {
        chapter.title = chapter.title.trim().to_string();
        chapter.edited_us = None;
    }
    chapters.sort_by(|a, b| a.source_us.cmp(&b.source_us).then(a.id.cmp(&b.id)));
    chapters
}

/// Where a chapter's moment plays now, through the clips of its asset.
pub fn edited_at(chapter: &Chapter, document: &EditDocument) -> Option<u64> {
    let asset = document.clock_asset(chapter.media.as_deref())?;
    document
        .asset_clock(asset)
        .source_to_edited_us(chapter.source_us)
}

pub fn attach_edited(chapters: &mut [Chapter], document: &EditDocument) {
    for chapter in chapters {
        chapter.edited_us = edited_at(chapter, document);
    }
}

/// The chapters that survive the edit, in playback order, as (edited start, title). The first
/// starts at 0 so the whole video is covered; chapters at the same moment keep the first one.
pub fn timeline(chapters: &[Chapter], document: &EditDocument) -> Vec<(u64, String)> {
    let mut placed: Vec<(u64, String)> = chapters
        .iter()
        .filter_map(|c| Some((edited_at(c, document)?, c.title.clone())))
        .collect();
    placed.sort_by_key(|(at, _)| *at);
    placed.dedup_by_key(|(at, _)| *at);
    if let Some(first) = placed.first_mut() {
        first.0 = 0;
    }
    placed
}

fn escape_metadata(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '=' | ';' | '#' | '\\' | '\n') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// An FFmpeg metadata file with one `[CHAPTER]` per entry, each ending where the next starts.
pub fn ffmetadata(timeline: &[(u64, String)], duration_us: u64) -> String {
    let mut out = String::from(";FFMETADATA1\n");
    for (i, (start, title)) in timeline.iter().enumerate() {
        let end = timeline.get(i + 1).map_or(duration_us, |next| next.0);
        if end <= *start {
            continue;
        }
        out.push_str(&format!(
            "[CHAPTER]\nTIMEBASE=1/1000\nSTART={}\nEND={}\ntitle={}\n",
            start / 1000,
            end / 1000,
            escape_metadata(title)
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::edit::SequenceEdit;

    const S: u64 = 1_000_000;

    fn chapter(id: &str, source_us: u64, title: &str) -> Chapter {
        Chapter {
            id: id.into(),
            source_us,
            title: title.into(),
            media: None,
            edited_us: None,
        }
    }

    #[test]
    fn chapters_follow_cuts_and_reordering() {
        // Source 0-10 s and 20-40 s kept, with the second part played first.
        let mut document =
            EditDocument::from_recording(crate::sequence::tests::recording("rec", 40 * S, &[]))
                .unwrap();
        let cut = |document: &EditDocument, edit: SequenceEdit| {
            crate::sequence::edit::apply(&document.sequence, &document.assets, &edit)
                .unwrap()
                .0
        };
        document.sequence = cut(
            &document,
            SequenceEdit::DeleteRange {
                ranges: vec![crate::zoom::EditedRange {
                    start_us: 10 * S,
                    end_us: 20 * S,
                }],
                ripple: Some(true),
            },
        );
        let second = document.sequence.tracks[0].clips[1].id.clone();
        document.sequence = cut(
            &document,
            SequenceEdit::MoveClips {
                clip_ids: vec![second],
                delta_us: -(10 * S as i64),
                track_id: None,
                anchor_id: None,
            },
        );
        let chapters = vec![
            chapter("c1", 2 * S, "Intro"),
            chapter("c2", 15 * S, "Cut away"),
            chapter("c3", 25 * S, "Main part"),
        ];
        let placed = timeline(&chapters, &document);
        // "Main part" plays first and becomes the 0:00 chapter; "Cut away" is gone.
        assert_eq!(
            placed,
            vec![(0, "Main part".to_string()), (22 * S, "Intro".to_string())]
        );
        let mut shown = chapters.clone();
        attach_edited(&mut shown, &document);
        assert_eq!(shown[1].edited_us, None);
        assert_eq!(shown[2].edited_us, Some(5 * S));
        // The UI gets the edited start; a cut chapter has none; storing drops it.
        let json = serde_json::to_value(&shown).unwrap();
        assert_eq!(json[2]["editedUs"], 5 * S);
        assert!(json[1].get("editedUs").is_none());
        let stored = serde_json::to_value(normalized(shown)).unwrap();
        assert!(stored
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c.get("editedUs").is_none()));
    }

    #[test]
    fn metadata_file_escapes_titles_and_covers_the_video() {
        let text = ffmetadata(
            &[(0, "Setup; part=1".into()), (65 * S, "Build #2".into())],
            120 * S,
        );
        assert_eq!(
            text,
            ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=65000\ntitle=Setup\\; part\\=1\n\
             [CHAPTER]\nTIMEBASE=1/1000\nSTART=65000\nEND=120000\ntitle=Build \\#2\n"
        );
    }

    #[test]
    fn validation_and_normalizing() {
        let mut chapters = vec![chapter("b", 9, " Two "), chapter("a", 1, "One")];
        validate(&chapters).unwrap();
        chapters = normalized(chapters);
        assert_eq!(chapters[0].id, "a");
        assert_eq!(chapters[1].title, "Two");
        assert!(validate(&[chapter("a", 0, "x"), chapter("a", 1, "y")]).is_err());
        assert!(validate(&[chapter("a", 0, "  ")]).is_err());
        assert!(validate(&[chapter("a", 0, "line\nbreak")]).is_err());
    }
}
