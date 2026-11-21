//! What a piece of footage shows, in words (FTG-3): the agent looks at a sparse board of a
//! source once (a few frames spread over it), writes a description, tags and the moments worth
//! finding, and the app keeps them for the app by the media's own bytes, so the same footage in
//! another project, folder or name is never looked at again. `source_search` then finds
//! footage by those words, beside what was said in it.
use crate::project::reader::safe_path;
use crate::sequence::sources::{stream_source, StreamSource};
use crate::sequence::{Asset, AssetKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const FOLDER: &str = "source-notes";
/// How much of each file's start and end goes into its fingerprint.
const SAMPLE_BYTES: u64 = 256 * 1024;
const MAX_DESCRIPTION_CHARS: usize = 1_000;
const MAX_TAGS: usize = 24;
const MAX_MOMENTS: usize = 60;
const MAX_LINE_CHARS: usize = 200;

/// One moment worth finding: on the media's own clock.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NoteMoment {
    pub at_us: u64,
    pub text: String,
}

/// What a source shows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Notes {
    pub description: String,
    pub tags: Vec<String>,
    pub moments: Vec<NoteMoment>,
    /// The media's name when they were written, for the user.
    #[schemars(skip)]
    pub name: String,
    #[schemars(skip)]
    pub saved_at: String,
}

impl Notes {
    pub fn validate(&self) -> Result<(), String> {
        if self.description.trim().is_empty() && self.tags.is_empty() && self.moments.is_empty() {
            return Err("Notes need a `description`, `tags` or `moments`".into());
        }
        if self.description.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(format!(
                "Keep the description to {MAX_DESCRIPTION_CHARS} characters"
            ));
        }
        if self.tags.len() > MAX_TAGS || self.moments.len() > MAX_MOMENTS {
            return Err(format!(
                "Keep at most {MAX_TAGS} tags and {MAX_MOMENTS} moments"
            ));
        }
        let long = |s: &str| s.trim().is_empty() || s.chars().count() > MAX_LINE_CHARS;
        if self.tags.iter().any(|t| long(t)) || self.moments.iter().any(|m| long(&m.text)) {
            return Err(format!(
                "Each tag and moment is 1 to {MAX_LINE_CHARS} characters"
            ));
        }
        Ok(())
    }

    /// The lines matching every word of `query`: (a moment's time, its text) or (none, the
    /// description and tags).
    pub fn matching(&self, query: &str) -> Vec<(Option<u64>, String)> {
        let words: Vec<String> = words(query);
        if words.is_empty() {
            return Vec::new();
        }
        let has_all = |text: &str| {
            let found = self::words(text);
            words
                .iter()
                .all(|w| found.iter().any(|f| f.starts_with(w.as_str())))
        };
        let mut out = Vec::new();
        let whole = format!("{} {}", self.description, self.tags.join(" "));
        if has_all(&whole) {
            let mut line = self.description.trim().to_string();
            if !self.tags.is_empty() {
                line = format!("{line} [{}]", self.tags.join(", "));
            }
            out.push((None, line.trim().to_string()));
        }
        for moment in &self.moments {
            if has_all(&format!("{} {}", moment.text, self.tags.join(" "))) {
                out.push((Some(moment.at_us), moment.text.clone()));
            }
        }
        out
    }
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Where the app keeps notes, under its settings folder.
pub fn dir(config_dir: &Path) -> PathBuf {
    config_dir.join(FOLDER)
}

/// The files `asset` is: a recording's segments, else the file it was imported from (not the
/// sound the project extracted from it).
fn files(root: &Path, asset: &Asset) -> Vec<PathBuf> {
    let mut out = Vec::new();
    match asset.kind {
        AssetKind::Recording => {
            for stream in &asset.streams {
                if let Ok(StreamSource::Segments(segments)) = stream_source(root, asset, &stream.id)
                {
                    out.extend(
                        segments
                            .iter()
                            .filter(|s| s.available)
                            .filter_map(|s| safe_path(root, &s.relative_path).ok()),
                    );
                }
            }
        }
        AssetKind::Graphic => {}
        AssetKind::Video | AssetKind::Audio | AssetKind::Image => {
            out.push(PathBuf::from(&asset.path))
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The media's fingerprint: its files' sizes and first and last bytes, not their names or
/// folders, so a copy of the footage anywhere has the same one.
pub fn fingerprint(root: &Path, asset: &Asset) -> Result<String, String> {
    let files = files(root, asset);
    if files.is_empty() {
        return Err(format!("{} has no files to know it by", asset.name));
    }
    let mut parts: Vec<Vec<u8>> = Vec::new();
    for path in &files {
        let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let len = file.metadata().map_err(|e| e.to_string())?.len();
        let mut part = len.to_le_bytes().to_vec();
        let mut head = vec![0; SAMPLE_BYTES.min(len) as usize];
        file.read_exact(&mut head).map_err(|e| e.to_string())?;
        part.extend(head);
        if len > SAMPLE_BYTES {
            let from = len.saturating_sub(SAMPLE_BYTES).max(SAMPLE_BYTES);
            file.seek(SeekFrom::Start(from))
                .map_err(|e| e.to_string())?;
            let mut tail = Vec::new();
            file.read_to_end(&mut tail).map_err(|e| e.to_string())?;
            part.extend(tail);
        }
        parts.push(Sha256::digest(&part).to_vec());
    }
    // The order of the files' contents, not their names.
    parts.sort();
    let mut all = Sha256::new();
    for part in parts {
        all.update(part);
    }
    Ok(all.finalize()[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

pub fn load(dir: &Path, fingerprint: &str) -> Option<Notes> {
    crate::files::load_json(&dir.join(format!("{fingerprint}.json")))
}

pub fn save(dir: &Path, fingerprint: &str, notes: &Notes) -> Result<(), String> {
    notes.validate()?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    crate::files::save_json(&dir.join(format!("{fingerprint}.json")), notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_match_every_word_by_its_start() {
        let notes = Notes {
            description: "A drone shot over the harbour at dusk".into(),
            tags: vec!["boats".into(), "aerial".into()],
            moments: vec![
                NoteMoment {
                    at_us: 4_000_000,
                    text: "Ferry leaves the pier".into(),
                },
                NoteMoment {
                    at_us: 9_000_000,
                    text: "Lights come on".into(),
                },
            ],
            ..Notes::default()
        };
        notes.validate().unwrap();
        let found = notes.matching("harb dusk");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, None);
        assert!(found[0].1.contains("[boats, aerial]"));
        // A moment matches with the source's tags.
        assert_eq!(
            notes.matching("ferry aerial"),
            vec![(Some(4_000_000), "Ferry leaves the pier".to_string())]
        );
        assert!(notes.matching("night").is_empty());
        assert!(notes.matching("").is_empty());
        assert!(Notes::default().validate().is_err());
    }

    #[test]
    fn a_copy_of_the_footage_has_the_same_fingerprint_and_notes() {
        let dir = tempfile::tempdir().unwrap();
        let bytes: Vec<u8> = (0..600_000u32).map(|i| (i % 251) as u8).collect();
        let asset_at = |name: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, &bytes).unwrap();
            crate::sequence::Asset {
                id: name.into(),
                name: name.into(),
                kind: crate::sequence::AssetKind::Image,
                path: path.display().to_string(),
                streams: vec![crate::sequence::Stream {
                    id: "picture".into(),
                    kind: crate::sequence::StreamKind::Picture,
                    role: crate::sequence::Role::Screen,
                    name: "Picture".into(),
                    audio_path: None,
                    ..serde_json::from_value(serde_json::json!({
                        "id": "picture", "kind": "picture", "role": "screen", "name": "Picture"
                    }))
                    .unwrap()
                }],
                ..serde_json::from_value(serde_json::json!({
                    "id": name, "name": name, "kind": "image", "path": "", "streams": [],
                    "durationUs": 5_000_000
                }))
                .unwrap()
            }
        };
        let a = fingerprint(dir.path(), &asset_at("a.png")).unwrap();
        let b = fingerprint(dir.path(), &asset_at("copy.png")).unwrap();
        assert_eq!(a, b);
        let notes_dir = dir.path().join("notes");
        let notes = Notes {
            description: "Settings page".into(),
            ..Notes::default()
        };
        save(&notes_dir, &a, &notes).unwrap();
        assert_eq!(load(&notes_dir, &b), Some(notes));
        // Different bytes, a different fingerprint.
        std::fs::write(dir.path().join("other.png"), b"other").unwrap();
        let mut other = asset_at("x.png");
        other.path = dir.path().join("other.png").display().to_string();
        assert_ne!(fingerprint(dir.path(), &other).unwrap(), a);
    }
}
