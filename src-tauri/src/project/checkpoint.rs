//! What a checkpoint keeps beside the edit, and putting it back: each graphic's source folder
//! (`graphics/<id>/`, its renders are named by those files so they follow) and each
//! transcript's caption marks. The edit itself is a state in the history (`history.rs`).
//! Design: `docs/architecture.md`, "Checkpoints".
use super::history::{CaptionSnapshot, Checkpoint, History, StepMeta, MAX_FILE_BYTES};
use super::reader::safe_path;
use crate::graphics::GRAPHICS_DIR;
use crate::transcript::store::{self, TRANSCRIPTS_DIR};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// What putting a checkpoint back did beside the edit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Restored {
    /// Graphics whose files changed back.
    pub graphics: usize,
    /// Transcripts whose caption marks changed back.
    pub captions: usize,
    /// Sounds whose caption marks stayed as they are, and why.
    pub skipped: Vec<String>,
    /// What could not be written.
    pub problems: Vec<String>,
}

impl Restored {
    /// One line for the user or the agent, empty when nothing beside the edit changed.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.graphics > 0 {
            parts.push(plural(self.graphics, "graphic"));
        }
        if self.captions > 0 {
            parts.push(format!(
                "caption marks of {}",
                plural(self.captions, "sound")
            ));
        }
        let mut line = if parts.is_empty() {
            String::new()
        } else {
            format!("Put back {}.", parts.join(" and "))
        };
        for skipped in &self.skipped {
            line.push_str(&format!(" Caption marks of {skipped}."));
        }
        for problem in &self.problems {
            line.push_str(&format!(" {problem}."));
        }
        line.trim().to_string()
    }
}

fn plural(n: usize, what: &str) -> String {
    format!("{n} {what}{}", if n == 1 { "" } else { "s" })
}

/// A checkpoint of the project at `root` as it is now: `state` is the edit's saved state.
/// Graphic files are kept in `history`. Its id is given when it is added.
pub fn capture(
    root: &Path,
    history: &mut History,
    state: String,
    name: &str,
    meta: &StepMeta,
    auto: bool,
) -> Result<Checkpoint, String> {
    let mut graphics = BTreeMap::new();
    for id in graphic_ids(root) {
        let dir = safe_path(root, &format!("{GRAPHICS_DIR}/{id}"))?;
        let mut files = Vec::new();
        crate::graphics::render::list_files(&dir, &dir, &mut files)?;
        let mut kept = BTreeMap::new();
        for (relative, path) in files {
            let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
            if size > MAX_FILE_BYTES {
                return Err(format!(
                    "Graphic {id}'s {relative} is too big for a checkpoint"
                ));
            }
            let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            kept.insert(relative, history.save_file(root, &bytes)?);
        }
        graphics.insert(id, kept);
    }
    let mut captions = BTreeMap::new();
    for track in transcript_tracks(root) {
        if let Ok(Some(transcript)) = store::load_transcript(root, &track) {
            captions.insert(
                track,
                CaptionSnapshot {
                    stamp: word_stamp(&transcript),
                    marks: transcript.caption_marks.clone(),
                },
            );
        }
    }
    Ok(Checkpoint {
        id: String::new(),
        name: name.into(),
        state,
        by: meta.by,
        agent: meta.agent.clone(),
        auto,
        at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        graphics,
        captions,
    })
}

/// Puts `checkpoint`'s graphic files and caption marks back. Graphics made since it are left
/// alone (the restored edit no longer places them); caption marks come back only onto the
/// same words.
pub fn restore(root: &Path, history: &History, checkpoint: &Checkpoint) -> Restored {
    let mut restored = Restored::default();
    for (id, files) in &checkpoint.graphics {
        match restore_graphic(root, history, id, files) {
            Ok(true) => restored.graphics += 1,
            Ok(false) => {}
            Err(error) => restored
                .problems
                .push(format!("Graphic {id} could not be put back: {error}")),
        }
    }
    for (track, snapshot) in &checkpoint.captions {
        let transcript = match store::load_transcript(root, track) {
            Ok(Some(transcript)) => transcript,
            _ => {
                restored.skipped.push(format!(
                    "{track} stay as they are: it has no transcript now"
                ));
                continue;
            }
        };
        if word_stamp(&transcript) != snapshot.stamp {
            restored.skipped.push(format!(
                "{track} stay as they are: it was transcribed again since"
            ));
            continue;
        }
        if transcript.caption_marks == snapshot.marks {
            continue;
        }
        let mut transcript = transcript;
        transcript.caption_marks = snapshot.marks.clone();
        match store::save_transcript(root, &transcript) {
            Ok(()) => restored.captions += 1,
            Err(error) => restored.problems.push(format!(
                "{track}'s caption marks could not be saved: {error}"
            )),
        }
    }
    restored
}

/// Writes graphic `id`'s folder back to `files`; whether anything changed.
fn restore_graphic(
    root: &Path,
    history: &History,
    id: &str,
    files: &BTreeMap<String, String>,
) -> Result<bool, String> {
    if !valid_id(id) {
        return Err("invalid graphic id".into());
    }
    let dir = safe_path(root, &format!("{GRAPHICS_DIR}/{id}"))?;
    let mut now = Vec::new();
    if dir.is_dir() {
        crate::graphics::render::list_files(&dir, &dir, &mut now)?;
    }
    // Read everything first, so a missing kept file changes nothing.
    let mut wanted = Vec::new();
    for (relative, hash) in files {
        if relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(format!("invalid file name {relative}"));
        }
        wanted.push((relative, history.load_file(root, hash)?, hash));
    }
    let mut changed = false;
    for (relative, path) in &now {
        if !files.contains_key(relative) {
            fs::remove_file(path).map_err(|e| e.to_string())?;
            changed = true;
        }
    }
    for (relative, bytes, hash) in wanted {
        let path = safe_path(root, &format!("{GRAPHICS_DIR}/{id}/{relative}"))?;
        let same = fs::read(&path).is_ok_and(|b| hex(&Sha256::digest(&b)) == *hash);
        if same {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(&dir))
            .map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut temp, &bytes).map_err(|e| e.to_string())?;
        temp.persist(&path).map_err(|e| e.to_string())?;
        changed = true;
    }
    Ok(changed)
}

/// The project's graphic folders.
fn graphic_ids(root: &Path) -> Vec<String> {
    let Ok(dir) = safe_path(root, GRAPHICS_DIR) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter(|id| valid_id(id))
        .collect();
    ids.sort();
    ids
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The sounds that have a transcript.
fn transcript_tracks(root: &Path) -> Vec<String> {
    let Ok(dir) = safe_path(root, TRANSCRIPTS_DIR) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut tracks: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            e.file_name()
                .to_str()?
                .strip_suffix(".json")
                .map(str::to_string)
        })
        .collect();
    tracks.sort();
    tracks
}

/// The transcript's word ids, hashed: a word fix keeps them, transcribing again does not.
fn word_stamp(transcript: &crate::transcript::Transcript) -> String {
    let mut hasher = Sha256::new();
    hasher.update(transcript.created_at.as_bytes());
    for word in &transcript.words {
        hasher.update([0]);
        hasher.update(word.id.as_bytes());
    }
    hex(&hasher.finalize()[..8])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
