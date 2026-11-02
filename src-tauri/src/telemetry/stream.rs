//! What reading the recorder's optional JSON Lines streams (keys, windows) has in common, and
//! keeping what was read from a recording until its files change.
use crate::project::reader::{open_regular, safe_path};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

const STREAM_LIMIT: u64 = 64 * 1024 * 1024;
const LINE_LIMIT: u64 = 16 * 1024;
const RECORD_LIMIT: usize = 1_000_000;
const DIAGNOSTIC_LIMIT: usize = 64;
pub(super) const MAX_SAFE_TIME: u64 = 9_007_199_254_740_991;

#[derive(Deserialize)]
struct RawRecord<K> {
    version: u32,
    t_us: u64,
    #[serde(flatten)]
    kind: K,
}

/// Reads `telemetry/<file>` under `root`, calling `on_record` with each record's time and kind.
/// Answers what went wrong along the way, or `None` when the recording has no such file or it
/// is of a version this build does not read (`diagnostics` says why). `what` names the stream
/// in those messages ("Keyboard", "Window").
pub(super) fn read_stream<K: DeserializeOwned>(
    root: &Path,
    file: &str,
    what: &str,
    version: u32,
    diagnostics: &mut Vec<String>,
    mut on_record: impl FnMut(u64, K),
) -> Result<Option<Vec<String>>, String> {
    let lower = what.to_lowercase();
    let path = match safe_path(root, &format!("telemetry/{file}")) {
        Ok(path) => path,
        Err(error) => {
            diagnostics.push(format!("{what} telemetry path rejected: {error}"));
            return Ok(None);
        }
    };
    if !path.exists() {
        return Ok(None);
    }
    let mut reader = BufReader::new(open_regular(&path)?.take(STREAM_LIMIT + 1));
    let mut problems = Vec::new();
    let mut total = 0u64;
    for line_number in 1..=RECORD_LIMIT {
        let mut line = Vec::new();
        let count = reader
            .by_ref()
            .take(LINE_LIMIT + 1)
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if count as u64 > LINE_LIMIT || total > STREAM_LIMIT {
            problems.push(format!(
                "{what} telemetry is too large; the rest was not read"
            ));
            break;
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let had_newline = line.ends_with(b"\n");
        let record: RawRecord<K> = match serde_json::from_slice(&line) {
            Ok(record) => record,
            // A crash mid-write leaves a cut-off last line.
            Err(_) if !had_newline => break,
            Err(error) => {
                if problems.len() < DIAGNOSTIC_LIMIT {
                    problems.push(format!(
                        "Corrupt {lower} telemetry line {line_number}: {error}"
                    ));
                }
                continue;
            }
        };
        if record.version != version {
            diagnostics.push(format!(
                "{what} telemetry version {} is not supported; it was ignored",
                record.version
            ));
            return Ok(None);
        }
        if record.t_us <= MAX_SAFE_TIME {
            on_record(record.t_us, record.kind);
        }
    }
    Ok(Some(problems))
}

/// What was read from each recording, with the times its files were last changed.
pub(crate) type FolderCache<T> = OnceLock<Mutex<HashMap<PathBuf, (Vec<Stamp>, Arc<T>)>>>;
type Stamp = Option<SystemTime>;

/// `read`'s answer for the recording in `folder`, read once while its telemetry `files` are
/// unchanged.
pub(crate) fn read_once<T>(
    cache: &'static FolderCache<T>,
    folder: &Path,
    files: &[&str],
    read: impl FnOnce() -> T,
) -> Arc<T> {
    let stamps: Vec<Stamp> = files
        .iter()
        .map(|name| {
            std::fs::metadata(folder.join("telemetry").join(name))
                .and_then(|m| m.modified())
                .ok()
        })
        .collect();
    let cache = cache.get_or_init(Default::default);
    if let Some((seen, value)) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(folder) {
        if *seen == stamps {
            return value.clone();
        }
    }
    let value = Arc::new(read());
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(folder.to_path_buf(), (stamps, value.clone()));
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_was_read_is_kept_until_the_file_changes() {
        static CACHE: FolderCache<u32> = OnceLock::new();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("telemetry")).unwrap();
        let file = dir.path().join("telemetry/a.jsonl");
        std::fs::write(&file, "1").unwrap();
        let first = read_once(&CACHE, dir.path(), &["a.jsonl"], || 1);
        assert_eq!(*read_once(&CACHE, dir.path(), &["a.jsonl"], || 2), *first);
        let later = SystemTime::now() + std::time::Duration::from_secs(60);
        let handle = std::fs::File::options().write(true).open(&file).unwrap();
        handle.set_modified(later).unwrap();
        assert_eq!(*read_once(&CACHE, dir.path(), &["a.jsonl"], || 3), 3);
    }
}
