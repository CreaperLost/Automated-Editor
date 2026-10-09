//! Transcripts live next to `project.json` as `transcripts/<track id>.json`. They are
//! analysis results like the waveform cache, not edits, so they sit outside the undo history.
use super::Transcript;
use crate::project::reader::{open_regular, safe_path};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const TRANSCRIPTS_DIR: &str = "transcripts";
pub const MAX_TRANSCRIPT_BYTES: u64 = 64 * 1024 * 1024;

pub fn validate_dependencies(
    root: &Path,
    dependencies: &[super::TranscriptDependency],
) -> Result<(), String> {
    for expected in dependencies {
        let current = load_transcript(root, &expected.track_id)?;
        let actual = current
            .as_ref()
            .map(Transcript::dependency)
            .unwrap_or_else(|| super::TranscriptDependency {
                track_id: expected.track_id.clone(),
                word_stamp: None,
            });
        if &actual != expected {
            return Err(
                "The transcript changed after this analysis. Find suggestions again.".into(),
            );
        }
    }
    Ok(())
}

fn transcript_path(root: &Path, track_id: &str) -> Result<PathBuf, String> {
    // A sound's key, `<asset>.<stream>`.
    if track_id.is_empty()
        || track_id.len() > 160
        || track_id.starts_with('.')
        || track_id.contains("..")
        || !track_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("Invalid track id for a transcript".into());
    }
    safe_path(root, &format!("{TRANSCRIPTS_DIR}/{track_id}.json"))
}

pub fn load_transcript(root: &Path, track_id: &str) -> Result<Option<Transcript>, String> {
    let path = transcript_path(root, track_id)?;
    if !path.is_file() {
        return Ok(None);
    }
    let file = open_regular(&path)?;
    let mut bytes = Vec::new();
    file.take(MAX_TRANSCRIPT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_TRANSCRIPT_BYTES {
        return Err("Transcript exceeds size limit".into());
    }
    let transcript: Transcript =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid transcript: {e}"))?;
    if transcript.track_id != track_id {
        return Err("Transcript belongs to a different track".into());
    }
    transcript.validate()?;
    Ok(Some(transcript))
}

pub fn save_transcript(root: &Path, transcript: &Transcript) -> Result<(), String> {
    transcript.validate()?;
    let path = transcript_path(root, &transcript.track_id)?;
    let dir = path.parent().ok_or("Invalid transcript path")?;
    if let Ok(meta) = fs::symlink_metadata(dir) {
        if !meta.is_dir() {
            return Err("Transcripts folder is not a directory".into());
        }
    } else {
        fs::create_dir(dir).map_err(|e| e.to_string())?;
    }
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if meta.file_type().is_symlink() {
            return Err("Transcript cannot be a symlink".into());
        }
    }
    let serialized = serde_json::to_vec(transcript).map_err(|e| e.to_string())?;
    if serialized.len() as u64 > MAX_TRANSCRIPT_BYTES {
        return Err("Transcript exceeds size limit".into());
    }
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    temp.write_all(&serialized).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(&path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_transcript(root: &Path, track_id: &str) -> Result<(), String> {
    let path = transcript_path(root, track_id)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::{test_word, ProviderKind};

    #[test]
    fn roundtrip_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_transcript(dir.path(), "mic-1").unwrap().is_none());
        let t = Transcript::new(
            "mic-1".into(),
            ProviderKind::ElevenLabs,
            "scribe_v2".into(),
            Some("en".into()),
            vec![test_word("hi", 0, 300)],
        );
        save_transcript(dir.path(), &t).unwrap();
        assert_eq!(load_transcript(dir.path(), "mic-1").unwrap().unwrap(), t);
        delete_transcript(dir.path(), "mic-1").unwrap();
        assert!(load_transcript(dir.path(), "mic-1").unwrap().is_none());
    }

    #[test]
    fn rejects_path_like_track_ids() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_transcript(dir.path(), "../x").is_err());
        assert!(load_transcript(dir.path(), "a/b").is_err());
        assert!(load_transcript(dir.path(), "").is_err());
    }

    #[test]
    fn rejects_transcript_for_another_track() {
        let dir = tempfile::tempdir().unwrap();
        let t = Transcript::new(
            "mic".into(),
            ProviderKind::Parakeet,
            "m".into(),
            None,
            vec![test_word("hi", 0, 300)],
        );
        save_transcript(dir.path(), &t).unwrap();
        fs::rename(
            dir.path().join("transcripts/mic.json"),
            dir.path().join("transcripts/other.json"),
        )
        .unwrap();
        assert!(load_transcript(dir.path(), "other").is_err());
    }
}
