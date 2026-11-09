//! Files the app replaces whole: a reader sees the old file or the new, never part of one.
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::Write;
use std::path::Path;

fn replace(path: &Path, bytes: &[u8], durable: bool) -> Result<(), String> {
    let dir = path.parent().ok_or("A file needs a folder")?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    temp.write_all(bytes).map_err(|e| e.to_string())?;
    if durable {
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
    }
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Replaces `path`, in a folder that exists, with `bytes`.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    replace(path, bytes, false)
}

/// [`write_atomic`] that also survives a power cut: for what can't be made again.
pub fn write_durable(path: &Path, bytes: &[u8]) -> Result<(), String> {
    replace(path, bytes, true)
}

/// Writes `value` as readable JSON, making the folder if it isn't there.
pub fn save_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let dir = path.parent().ok_or("A file needs a folder")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    write_atomic(path, &bytes)
}

/// The JSON in `path` for a store whose loss matters (the user's work, a journal): `None` only
/// when the file is missing. A file that can't be read or parsed is an error, so a caller never
/// mistakes it for an empty store and writes over it.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{} can't be read: {e}", path.display())),
    };
    serde_json::from_slice(&bytes).map(Some).map_err(|e| {
        format!(
            "{} is damaged ({e}). It was left as it is: fix it or move it away",
            path.display()
        )
    })
}

/// The JSON in `path`; `None` when it is missing or can't be read.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_comes_back_and_a_bad_file_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new/folder/value.json");
        assert_eq!(load_json::<Vec<u32>>(&path), None);
        save_json(&path, &vec![1, 2]).unwrap();
        assert_eq!(load_json::<Vec<u32>>(&path), Some(vec![1, 2]));
        write_durable(&path, b"{ not json").unwrap();
        assert_eq!(load_json::<Vec<u32>>(&path), None);
    }
}
