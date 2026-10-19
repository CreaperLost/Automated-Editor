//! Keyboard activity recorded beside the mouse stream (`telemetry/keys.jsonl`), as specified
//! in `docs/keyboard-telemetry.md`: shortcuts by key, typing as stretches with a key count and
//! never the text, and gaps where the keyboard was not seen. Same clock as `events.jsonl`.
//!
//! No file means the keyboard was not recorded, and everything falls back to the mouse.
use crate::project::reader::{open_regular, safe_path};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

pub const KEYS_VERSION: u32 = 1;
const KEYS_LIMIT: u64 = 64 * 1024 * 1024;
const LINE_LIMIT: u64 = 16 * 1024;
const RECORD_LIMIT: usize = 1_000_000;
const DIAGNOSTIC_LIMIT: usize = 64;
const MAX_SAFE_TIME: u64 = 9_007_199_254_740_991;
/// How long a shortcut badge stays up.
pub const BADGE_US: u64 = 1_400_000;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub t_us: u64,
    /// `ctrl`, `alt`, `shift`, `meta`, in that order.
    pub modifiers: Vec<String>,
    /// W3C `KeyboardEvent.code`; `None` for a modifier tapped alone.
    pub code: Option<String>,
    /// The key as the active layout shows it.
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Caret {
    pub norm_x: f64,
    pub norm_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypingRun {
    pub start_us: u64,
    pub end_us: u64,
    pub keys: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caret_start: Option<Caret>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caret_end: Option<Caret>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyGap {
    pub reason: String,
    pub start_us: u64,
    pub end_us: u64,
}

impl KeyGap {
    /// The hook was never there: nothing is known, so the mouse alone decides, as if the
    /// keyboard had not been recorded. Other gaps are short and might hide typing.
    pub fn hides_typing(&self) -> bool {
        self.reason != "hook_unavailable"
    }
}

/// A recording's keyboard activity, sorted by time. Keys pressed in a window outside the
/// recorded source are left out.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStream {
    pub platform: Option<String>,
    pub shortcuts: Vec<Shortcut>,
    pub typing: Vec<TypingRun>,
    pub gaps: Vec<KeyGap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

impl KeyStream {
    /// Times (start, end) when the keyboard was in use or might have been: typing, each
    /// shortcut as a moment, and gaps that might hide typing.
    pub fn activity(&self) -> Vec<(u64, u64)> {
        let mut out: Vec<(u64, u64)> = self
            .typing
            .iter()
            .map(|run| (run.start_us, run.end_us))
            .chain(self.shortcuts.iter().map(|s| (s.t_us, s.t_us)))
            .chain(
                self.gaps
                    .iter()
                    .filter(|gap| gap.hides_typing())
                    .map(|gap| (gap.start_us, gap.end_us)),
            )
            .collect();
        out.sort_unstable();
        out
    }

    /// The shortcut whose badge is up at `t_us`: the latest one pressed less than
    /// [`BADGE_US`] before, with its index.
    pub fn badge_at(&self, t_us: u64) -> Option<(usize, &Shortcut)> {
        let index = self.shortcuts.partition_point(|s| s.t_us <= t_us);
        let index = index.checked_sub(1)?;
        let shortcut = &self.shortcuts[index];
        (t_us - shortcut.t_us < BADGE_US).then_some((index, shortcut))
    }
}

/// The badge text for `shortcut` on `platform`, one item per key: `["Ctrl", "Shift", "P"]`.
pub fn badge_keys(shortcut: &Shortcut, platform: Option<&str>) -> Vec<String> {
    let mac = platform == Some("macos");
    let mut keys: Vec<String> = shortcut
        .modifiers
        .iter()
        .map(|m| {
            match (m.as_str(), mac) {
                ("ctrl", false) => "Ctrl",
                ("ctrl", true) => "⌃",
                ("alt", false) => "Alt",
                ("alt", true) => "⌥",
                ("shift", false) => "Shift",
                ("shift", true) => "⇧",
                ("meta", false) => "Win",
                ("meta", true) => "⌘",
                (other, _) => other,
            }
            .to_string()
        })
        .collect();
    let label = shortcut.label.trim();
    if !label.is_empty() {
        keys.push(label.to_string());
    }
    keys
}

#[derive(Deserialize)]
struct RawCaret {
    norm_x: f64,
    norm_y: f64,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RawKind {
    Capture {
        #[serde(default)]
        platform: Option<String>,
    },
    Shortcut {
        #[serde(default)]
        modifiers: Vec<String>,
        #[serde(default)]
        code: Option<String>,
        #[serde(default)]
        label: String,
        #[serde(default)]
        focus_inside_source: Option<bool>,
    },
    Typing {
        start_us: u64,
        end_us: u64,
        #[serde(default)]
        keys: u32,
        #[serde(default)]
        caret_start: Option<RawCaret>,
        #[serde(default)]
        caret_end: Option<RawCaret>,
        #[serde(default)]
        focus_inside_source: Option<bool>,
    },
    Gap {
        #[serde(default)]
        reason: String,
        start_us: u64,
        end_us: u64,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct RawRecord {
    version: u32,
    t_us: u64,
    #[serde(flatten)]
    kind: RawKind,
}

const MODIFIERS: [&str; 4] = ["ctrl", "alt", "shift", "meta"];

fn caret(raw: Option<RawCaret>) -> Option<Caret> {
    raw.filter(|c| (0.0..=1.0).contains(&c.norm_x) && (0.0..=1.0).contains(&c.norm_y))
        .map(|c| Caret {
            norm_x: c.norm_x,
            norm_y: c.norm_y,
        })
}

/// Reads `telemetry/keys.jsonl` under `root`. `Ok(None)` when the recording has none or the
/// file is of a version this build does not read (`diagnostics` says why).
pub fn read_keys(root: &Path, diagnostics: &mut Vec<String>) -> Result<Option<KeyStream>, String> {
    let path = match safe_path(root, "telemetry/keys.jsonl") {
        Ok(path) => path,
        Err(error) => {
            diagnostics.push(format!("Keyboard telemetry path rejected: {error}"));
            return Ok(None);
        }
    };
    if !path.exists() {
        return Ok(None);
    }
    let file = open_regular(&path)?;
    let mut reader = BufReader::new(file.take(KEYS_LIMIT + 1));
    let mut stream = KeyStream::default();
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
        if count as u64 > LINE_LIMIT || total > KEYS_LIMIT {
            stream
                .diagnostics
                .push("Keyboard telemetry is too large; the rest was not read".into());
            break;
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let had_newline = line.ends_with(b"\n");
        let record: RawRecord = match serde_json::from_slice(&line) {
            Ok(record) => record,
            // A crash mid-write leaves a cut-off last line.
            Err(_) if !had_newline => break,
            Err(error) => {
                if stream.diagnostics.len() < DIAGNOSTIC_LIMIT {
                    stream.diagnostics.push(format!(
                        "Corrupt keyboard telemetry line {line_number}: {error}"
                    ));
                }
                continue;
            }
        };
        if record.version != KEYS_VERSION {
            diagnostics.push(format!(
                "Keyboard telemetry version {} is not supported; it was ignored",
                record.version
            ));
            return Ok(None);
        }
        if record.t_us > MAX_SAFE_TIME {
            continue;
        }
        match record.kind {
            RawKind::Capture { platform } => stream.platform = platform,
            RawKind::Shortcut {
                modifiers,
                code,
                label,
                focus_inside_source,
            } => {
                if focus_inside_source == Some(false) {
                    continue;
                }
                let modifiers: Vec<String> = MODIFIERS
                    .iter()
                    .filter(|m| modifiers.iter().any(|given| given == *m))
                    .map(|m| m.to_string())
                    .collect();
                let label: String = label.chars().take(16).collect();
                if modifiers.is_empty() && label.trim().is_empty() {
                    continue;
                }
                stream.shortcuts.push(Shortcut {
                    t_us: record.t_us,
                    modifiers,
                    code: code.map(|c| c.chars().take(32).collect()),
                    label,
                });
            }
            RawKind::Typing {
                start_us,
                end_us,
                keys,
                caret_start,
                caret_end,
                focus_inside_source,
            } => {
                if focus_inside_source == Some(false) || end_us < start_us || end_us > MAX_SAFE_TIME
                {
                    continue;
                }
                stream.typing.push(TypingRun {
                    start_us,
                    end_us,
                    keys,
                    caret_start: caret(caret_start),
                    caret_end: caret(caret_end),
                });
            }
            RawKind::Gap {
                reason,
                start_us,
                end_us,
            } => {
                if end_us < start_us || end_us > MAX_SAFE_TIME {
                    continue;
                }
                stream.gaps.push(KeyGap {
                    reason: reason.chars().take(64).collect(),
                    start_us,
                    end_us,
                });
            }
            RawKind::Unknown => {}
        }
    }
    stream.shortcuts.sort_by_key(|s| s.t_us);
    stream.typing.sort_by_key(|r| (r.start_us, r.end_us));
    stream.gaps.sort_by_key(|g| (g.start_us, g.end_us));
    Ok(Some(stream))
}

/// The keyboard activity of the recording in `folder`, read once while the file is unchanged.
pub fn recording_keys(folder: &Path) -> std::sync::Arc<Option<KeyStream>> {
    use std::sync::{Arc, Mutex, OnceLock};
    type Cache = Mutex<
        std::collections::HashMap<
            std::path::PathBuf,
            (Option<std::time::SystemTime>, Arc<Option<KeyStream>>),
        >,
    >;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let modified = std::fs::metadata(folder.join("telemetry").join("keys.jsonl"))
        .and_then(|m| m.modified())
        .ok();
    let cache = CACHE.get_or_init(Default::default);
    if let Some((stamp, keys)) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(folder) {
        if *stamp == modified {
            return keys.clone();
        }
    }
    let keys = Arc::new(read_keys(folder, &mut Vec::new()).ok().flatten());
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(folder.to_path_buf(), (modified, keys.clone()));
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, lines: &[&str]) {
        std::fs::create_dir_all(dir.join("telemetry")).unwrap();
        std::fs::write(dir.join("telemetry/keys.jsonl"), lines.join("\n")).unwrap();
    }

    #[test]
    fn reads_every_kind_and_skips_keys_outside_the_source() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            &[
                r#"{"version":1,"seq":0,"t_us":0,"kind":"capture","platform":"windows","shortcuts":true,"typing":true,"caret":true}"#,
                r#"{"version":1,"seq":1,"t_us":8412000,"kind":"shortcut","modifiers":["shift","ctrl"],"code":"KeyP","label":"P","focus_inside_source":true}"#,
                r#"{"version":1,"seq":2,"t_us":15230000,"kind":"typing","start_us":12010000,"end_us":15230000,"keys":38,"caret_start":{"geometry_id":"g1","norm_x":0.41,"norm_y":0.63}}"#,
                r#"{"version":1,"seq":3,"t_us":16000000,"kind":"shortcut","modifiers":["alt"],"code":"Tab","label":"Tab","focus_inside_source":false}"#,
                r#"{"version":1,"seq":4,"t_us":61000000,"kind":"gap","reason":"recording_paused","start_us":42000000,"end_us":61000000,"dropped_events":0}"#,
                r#"{"version":1,"seq":5,"t_us":62000000,"kind":"something_new","x":1}"#,
                r#"{"version":1,"seq":6,"t_us":63000000,"kind":"typ"#,
            ],
        );
        let mut diagnostics = Vec::new();
        let keys = read_keys(dir.path(), &mut diagnostics).unwrap().unwrap();
        assert_eq!(keys.platform.as_deref(), Some("windows"));
        assert_eq!(keys.shortcuts.len(), 1);
        assert_eq!(keys.shortcuts[0].modifiers, ["ctrl", "shift"]);
        assert_eq!(
            badge_keys(&keys.shortcuts[0], Some("windows")),
            ["Ctrl", "Shift", "P"]
        );
        assert_eq!(keys.typing.len(), 1);
        assert_eq!(keys.typing[0].keys, 38);
        assert_eq!(keys.typing[0].caret_start.unwrap().norm_x, 0.41);
        assert_eq!(keys.gaps.len(), 1);
        assert!(keys.diagnostics.is_empty() && diagnostics.is_empty());
        assert_eq!(
            keys.activity(),
            [
                (8_412_000, 8_412_000),
                (12_010_000, 15_230_000),
                (42_000_000, 61_000_000)
            ]
        );
    }

    #[test]
    fn no_file_or_another_version_means_no_keyboard() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_keys(dir.path(), &mut Vec::new()).unwrap(), None);
        write(
            dir.path(),
            &[r#"{"version":2,"seq":0,"t_us":0,"kind":"capture"}"#],
        );
        let mut diagnostics = Vec::new();
        assert_eq!(read_keys(dir.path(), &mut diagnostics).unwrap(), None);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn a_missing_hook_leaves_the_mouse_in_charge() {
        let keys = KeyStream {
            gaps: vec![KeyGap {
                reason: "hook_unavailable".into(),
                start_us: 0,
                end_us: 600_000_000,
            }],
            ..Default::default()
        };
        assert!(keys.activity().is_empty());
    }

    #[test]
    fn a_badge_shows_the_latest_shortcut_for_a_while() {
        let shortcut = |t_us| Shortcut {
            t_us,
            modifiers: vec!["ctrl".into()],
            code: Some("KeyS".into()),
            label: "S".into(),
        };
        let keys = KeyStream {
            shortcuts: vec![shortcut(1_000_000), shortcut(2_000_000)],
            ..Default::default()
        };
        assert_eq!(keys.badge_at(500_000), None);
        assert_eq!(keys.badge_at(1_500_000).map(|b| b.0), Some(0));
        assert_eq!(keys.badge_at(2_100_000).map(|b| b.0), Some(1));
        assert_eq!(keys.badge_at(2_000_000 + BADGE_US), None);
        let lone_win = Shortcut {
            t_us: 0,
            modifiers: vec!["meta".into()],
            code: None,
            label: String::new(),
        };
        assert_eq!(badge_keys(&lone_win, Some("macos")), ["⌘"]);
    }
}
