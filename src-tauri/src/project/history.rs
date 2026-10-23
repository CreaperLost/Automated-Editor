//! The edit history on disk, so undo and redo survive closing the project.
//!
//! `history/states/<hash>.json.gz` holds one edit document each, named by the SHA-256 of its
//! stored JSON with `revision` zeroed, so going back and forth writes nothing new.
//! `history/log.jsonl` is append-only: a header, then one line per commit, undo, redo or drop.
//! Opening replays it, checks it against `project.json` and rewrites it compactly.
//! Plan: `docs/plans/history.md`.
use super::reader::{open_regular, safe_path};
use super::revision::{stored_form, EditDocument, MAX_EDIT_BYTES};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

pub const HISTORY_DIR: &str = "history";
const STATES_DIR: &str = "history/states";
const LOG_FILE: &str = "history/log.jsonl";
const LOG_SCHEMA_VERSION: u32 = 1;
/// Saved states a project keeps before its oldest steps go (George, 23 Oct).
pub const MAX_HISTORY_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_LABEL_CHARS: usize = 120;
const MAX_LOG_LINE_BYTES: usize = 64 * 1024;

/// Who made a step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Author {
    #[default]
    User,
    Agent,
}

/// What the caller says about the step it is about to make.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepMeta {
    /// Said by the caller; without it the step is named from what changed ([`describe`]).
    pub label: Option<String>,
    pub by: Author,
    /// The MCP client's name, for an agent's step.
    pub agent: Option<String>,
    /// The MCP tool that made it.
    pub tool: Option<String>,
    /// The proposal batch the step accepted.
    pub batch: Option<String>,
}

impl StepMeta {
    /// Who is making a step on this thread: the agent while one of its calls runs, else the
    /// user.
    pub fn for_this_thread() -> Self {
        AGENT_CALL.with(|call| match &*call.borrow() {
            Some((agent, tool)) => Self {
                by: Author::Agent,
                agent: Some(agent.clone()),
                tool: Some(tool.clone()),
                ..Self::default()
            },
            None => Self::default(),
        })
    }
}

thread_local! {
    /// The agent and tool of the MCP call running on this thread. Tool calls run start to
    /// end on one blocking thread, so every step they make is theirs.
    static AGENT_CALL: std::cell::RefCell<Option<(String, String)>> = const { std::cell::RefCell::new(None) };
}

/// Runs `call` with the steps it makes on this thread marked as `agent`'s, by `tool`.
pub fn as_agent<T>(agent: &str, tool: &str, call: impl FnOnce() -> T) -> T {
    struct Reset(Option<(String, String)>);
    impl Drop for Reset {
        fn drop(&mut self) {
            AGENT_CALL.with(|c| *c.borrow_mut() = self.0.take());
        }
    }
    let previous = AGENT_CALL.with(|c| c.borrow_mut().replace((agent.into(), tool.into())));
    let _reset = Reset(previous);
    call()
}

/// A short name for the change from `before` to `after`, from the parts that differ.
pub fn describe(before: &EditDocument, after: &EditDocument) -> String {
    let mut parts: Vec<String> = Vec::new();
    if before.sequence != after.sequence {
        let (was, now) = (before.duration_us(), after.duration_us());
        let seconds = |us: u64| format!("{:.1} s", us as f64 / 1e6);
        parts.push(if now < was {
            format!("Cut {}", seconds(was - now))
        } else if now > was {
            format!("Added {}", seconds(now - was))
        } else {
            "Timeline edit".into()
        });
    }
    let named: [(bool, &str); 12] = [
        (before.assets != after.assets, "Media"),
        (before.noise_mutes != after.noise_mutes, "Mouth noises"),
        (
            before.zooms != after.zooms
                || before.dismissed_zoom_ids != after.dismissed_zoom_ids
                || before.zoom_settings != after.zoom_settings,
            "Zooms",
        ),
        (before.layout != after.layout, "Layout"),
        (before.layouts != after.layouts, "Layout sections"),
        (before.audio != after.audio, "Sound"),
        (before.captions != after.captions, "Captions"),
        (before.chapters != after.chapters, "Chapters"),
        (before.shorts != after.shorts, "Shorts"),
        (before.graphics_look != after.graphics_look, "Graphics look"),
        (before.graphics_brand != after.graphics_brand, "Brand kit"),
        (before.short_layout != after.short_layout, "Short layout"),
    ];
    parts.extend(
        named
            .iter()
            .filter(|(changed, _)| *changed)
            .map(|(_, n)| n.to_string()),
    );
    match parts.len() {
        0 => "Edit".into(),
        1 | 2 => parts.join(", "),
        n => format!("{}, {} and {} more", parts[0], parts[1], n - 2),
    }
}

/// One undoable change: the states on either side and what made it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub before: String,
    pub after: String,
    pub label: String,
    #[serde(default)]
    pub by: Author,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch: Option<String>,
    pub at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
enum Line {
    Header {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
    },
    Commit {
        step: Step,
    },
    Undo,
    Redo,
    /// The oldest `n` undo steps were let go to stay under the size limit.
    Drop {
        n: usize,
    },
}

/// The undo and redo stacks as steps; the documents stay on disk until one is needed.
#[derive(Clone, Debug, Default)]
pub struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// The hash of the current document, once it has been saved as a state.
    current: Option<String>,
    /// Size on disk of each saved state.
    sizes: HashMap<String, u64>,
    limit_bytes: u64,
}

impl History {
    pub fn empty() -> Self {
        Self {
            limit_bytes: MAX_HISTORY_BYTES,
            ..Self::default()
        }
    }

    pub fn undo_step(&self) -> Option<&Step> {
        self.undo.last()
    }

    pub fn redo_step(&self) -> Option<&Step> {
        self.redo.last()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    #[cfg(test)]
    pub fn set_limit_bytes(&mut self, limit: u64) {
        self.limit_bytes = limit;
    }

    #[cfg(test)]
    pub fn stored_bytes(&self) -> u64 {
        self.sizes.values().sum()
    }

    /// The history saved beside `current` (the document just read from `project.json`), with
    /// a note when something had to be repaired. Never fails: a history that cannot be read
    /// is set aside and the project starts with none, as before this existed.
    pub fn open(root: &Path, current: &EditDocument) -> (Self, Option<String>) {
        let mut history = Self::empty();
        let replayed = match read_log(root) {
            Ok(Some(lines)) => lines,
            Ok(None) => return (history, None),
            Err(error) => {
                let note = set_aside_log(root)
                    .map(|kept| {
                        format!(
                            "The undo history could not be read ({error}); it was kept as {kept}"
                        )
                    })
                    .unwrap_or_else(|e| {
                        format!("The undo history could not be read ({error}) or set aside ({e})")
                    });
                return (history, Some(note));
            }
        };
        for line in replayed {
            history.replay(line);
        }
        let sizes = state_sizes(root);
        let have = |hash: &String| sizes.contains_key(hash);
        trim_unreachable(&mut history.undo, &have);
        trim_unreachable(&mut history.redo, &have);
        let known = history.current.clone().filter(|h| have(h));
        history.sizes = sizes;
        let mut note = None;
        match state_hash(current) {
            Ok((hash, _)) if Some(&hash) != history.current.as_ref() => {
                // A crash between saving the edit and the log, or a change made elsewhere:
                // the edit on disk wins and becomes a step, so nothing is lost either way.
                if let Some(before) = known {
                    if store_state(root, current, &mut history.sizes).is_ok() {
                        history.redo.clear();
                        history.undo.push(Step {
                            before,
                            after: hash.clone(),
                            label: "Change made outside AeroEdits".into(),
                            by: Author::User,
                            agent: None,
                            tool: None,
                            batch: None,
                            at: now(),
                        });
                        note = Some(
                            "The edit had changed since its undo history was saved; the change \
                             is kept as an undo step"
                                .into(),
                        );
                    }
                } else {
                    history.undo.clear();
                    history.redo.clear();
                }
                history.current = Some(hash).filter(|h| history.sizes.contains_key(h));
            }
            Ok(_) => {}
            Err(_) => {
                history.undo.clear();
                history.redo.clear();
                history.current = None;
            }
        }
        if let Err(error) = history.compact(root) {
            eprintln!("history: could not rewrite the log: {error}");
        }
        history.prune(root);
        (history, note)
    }

    fn replay(&mut self, line: Line) {
        match line {
            Line::Header { .. } => {}
            Line::Commit { step } => {
                self.current = Some(step.after.clone());
                self.undo.push(step);
                self.redo.clear();
            }
            Line::Undo => {
                if let Some(step) = self.undo.pop() {
                    self.current = Some(step.before.clone());
                    self.redo.push(step);
                }
            }
            Line::Redo => {
                if let Some(step) = self.redo.pop() {
                    self.current = Some(step.after.clone());
                    self.undo.push(step);
                }
            }
            Line::Drop { n } => {
                self.undo.drain(..n.min(self.undo.len()));
            }
        }
    }

    /// Saves `document` as a state if it is new and returns its hash.
    pub fn save_state(&mut self, root: &Path, document: &EditDocument) -> Result<String, String> {
        store_state(root, document, &mut self.sizes)
    }

    /// The hash of the current document, saving it first when history starts with it.
    pub fn current_state(
        &mut self,
        root: &Path,
        document: &EditDocument,
    ) -> Result<String, String> {
        if let Some(hash) = self.current.clone().filter(|h| self.sizes.contains_key(h)) {
            return Ok(hash);
        }
        let hash = self.save_state(root, document)?;
        self.current = Some(hash.clone());
        Ok(hash)
    }

    /// The document saved as `hash`.
    pub fn load_state(&self, root: &Path, hash: &str) -> Result<EditDocument, String> {
        load_state(root, hash)
    }

    /// Records a step whose edit is already saved in `project.json`; `label` is used when
    /// `meta` has none.
    pub fn record_commit(
        &mut self,
        root: &Path,
        before: String,
        after: String,
        meta: StepMeta,
        label: impl FnOnce() -> String,
    ) {
        let step = Step {
            before,
            after: after.clone(),
            label: clean_label(&meta.label.unwrap_or_else(label)),
            by: meta.by,
            agent: meta.agent.map(|a| clean_label(&a)),
            tool: meta.tool.map(|t| clean_label(&t)),
            batch: meta.batch,
            at: now(),
        };
        self.current = Some(after);
        self.undo.push(step.clone());
        self.redo.clear();
        self.append(root, &Line::Commit { step });
        self.prune(root);
    }

    pub fn record_undo(&mut self, root: &Path) {
        if let Some(step) = self.undo.pop() {
            self.current = Some(step.before.clone());
            self.redo.push(step);
            self.append(root, &Line::Undo);
        }
    }

    pub fn record_redo(&mut self, root: &Path) {
        if let Some(step) = self.redo.pop() {
            self.current = Some(step.after.clone());
            self.undo.push(step);
            self.append(root, &Line::Redo);
        }
    }

    /// The edit is already saved, so a log that cannot be written is repaired at the next
    /// open (the change comes back as one step) rather than failing the edit.
    fn append(&self, root: &Path, line: &Line) {
        if let Err(error) = append_line(root, line) {
            eprintln!("history: could not write the log: {error}");
        }
    }

    /// Lets the oldest steps go while the saved states are over the limit, and deletes states
    /// nothing refers to any more.
    fn prune(&mut self, root: &Path) {
        let mut uses: HashMap<&str, usize> = HashMap::new();
        for step in self.undo.iter().chain(self.redo.iter()) {
            *uses.entry(&step.before).or_default() += 1;
            *uses.entry(&step.after).or_default() += 1;
        }
        if let Some(current) = &self.current {
            *uses.entry(current).or_default() += 1;
        }
        let mut total: u64 = self
            .sizes
            .iter()
            .filter(|(hash, _)| uses.contains_key(hash.as_str()))
            .map(|(_, size)| size)
            .sum();
        let mut dropped = 0;
        while total > self.limit_bytes && dropped < self.undo.len() {
            let step = &self.undo[dropped];
            for hash in [step.before.as_str(), step.after.as_str()] {
                let count = uses.get_mut(hash).expect("counted above");
                *count -= 1;
                if *count == 0 {
                    total = total.saturating_sub(self.sizes.get(hash).copied().unwrap_or(0));
                }
            }
            dropped += 1;
        }
        let unused: Vec<String> = self
            .sizes
            .keys()
            .filter(|hash| uses.get(hash.as_str()).copied().unwrap_or(0) == 0)
            .cloned()
            .collect();
        if dropped > 0 {
            self.undo.drain(..dropped);
            self.append(root, &Line::Drop { n: dropped });
        }
        for hash in unused {
            if let Ok(path) = state_path(root, &hash) {
                if fs::remove_file(path).is_ok() {
                    self.sizes.remove(&hash);
                }
            }
        }
    }

    /// Rewrites the log as just the steps it still holds.
    fn compact(&self, root: &Path) -> Result<(), String> {
        let mut text = line_text(&Line::Header {
            schema_version: LOG_SCHEMA_VERSION,
        })?;
        // Commit every step in order, then undo back to where the edit is now.
        for step in self.undo.iter().chain(self.redo.iter().rev()) {
            text.push_str(&line_text(&Line::Commit { step: step.clone() })?);
        }
        for _ in &self.redo {
            text.push_str(&line_text(&Line::Undo)?);
        }
        ensure_dirs(root)?;
        let path = safe_path(root, LOG_FILE)?;
        let dir = safe_path(root, HISTORY_DIR)?;
        let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
        temp.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Drops steps that can no longer be reached because a state they need is missing: on the
/// undo stack everything older than such a step, on the redo stack everything after it.
fn trim_unreachable(steps: &mut Vec<Step>, have: &impl Fn(&String) -> bool) {
    if let Some(i) = steps
        .iter()
        .rposition(|s| !have(&s.before) || !have(&s.after))
    {
        steps.drain(..=i);
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn clean_label(label: &str) -> String {
    let label: String = label
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(MAX_LABEL_CHARS)
        .collect();
    let label = label.trim();
    if label.is_empty() {
        "Edit".into()
    } else {
        label.to_string()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn ensure_dirs(root: &Path) -> Result<(), String> {
    let states = safe_path(root, STATES_DIR)?;
    fs::create_dir_all(states).map_err(|e| e.to_string())
}

fn state_path(root: &Path, hash: &str) -> Result<std::path::PathBuf, String> {
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid history state".into());
    }
    safe_path(root, &format!("{STATES_DIR}/{hash}.json.gz"))
}

/// The JSON a state is saved as, and its hash.
fn state_hash(document: &EditDocument) -> Result<(String, Vec<u8>), String> {
    let mut stored = stored_form(document);
    stored.revision = 0;
    let json = serde_json::to_vec(&stored).map_err(|e| e.to_string())?;
    let hash = hex(&Sha256::digest(&json));
    Ok((hash, json))
}

fn store_state(
    root: &Path,
    document: &EditDocument,
    sizes: &mut HashMap<String, u64>,
) -> Result<String, String> {
    let (hash, json) = state_hash(document)?;
    if sizes.contains_key(&hash) {
        return Ok(hash);
    }
    ensure_dirs(root)?;
    let path = state_path(root, &hash)?;
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if meta.is_file() {
            sizes.insert(hash.clone(), meta.len());
            return Ok(hash);
        }
    }
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&json).map_err(|e| e.to_string())?;
    let packed = encoder.finish().map_err(|e| e.to_string())?;
    let dir = safe_path(root, STATES_DIR)?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    temp.write_all(&packed).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(&path).map_err(|e| e.to_string())?;
    sizes.insert(hash.clone(), packed.len() as u64);
    Ok(hash)
}

fn load_state(root: &Path, hash: &str) -> Result<EditDocument, String> {
    let path = state_path(root, hash)?;
    let file = open_regular(&path).map_err(|_| "That step's saved edit is missing")?;
    let mut json = Vec::new();
    flate2::read::GzDecoder::new(file)
        .take(MAX_EDIT_BYTES + 1)
        .read_to_end(&mut json)
        .map_err(|e| format!("That step's saved edit could not be read: {e}"))?;
    if json.len() as u64 > MAX_EDIT_BYTES {
        return Err("That step's saved edit exceeds the size limit".into());
    }
    // No migration: a state from an earlier AeroEdits simply cannot be gone back to.
    let mut document: EditDocument = serde_json::from_slice(&json)
        .map_err(|_| "That step was saved by an earlier AeroEdits and cannot be restored")?;
    document.validate()?;
    document.attach_zoom_ranges();
    Ok(document)
}

fn state_sizes(root: &Path) -> HashMap<String, u64> {
    let mut sizes = HashMap::new();
    let Ok(dir) = safe_path(root, STATES_DIR) else {
        return sizes;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return sizes;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(hash) = name.to_str().and_then(|n| n.strip_suffix(".json.gz")) else {
            continue;
        };
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() && state_path(root, hash).is_ok() {
                sizes.insert(hash.to_string(), meta.len());
            }
        }
    }
    sizes
}

fn line_text(line: &Line) -> Result<String, String> {
    let mut text = serde_json::to_string(line).map_err(|e| e.to_string())?;
    text.push('\n');
    Ok(text)
}

fn append_line(root: &Path, line: &Line) -> Result<(), String> {
    ensure_dirs(root)?;
    let path = safe_path(root, LOG_FILE)?;
    let fresh = !path.is_file();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let mut text = String::new();
    if fresh {
        text.push_str(&line_text(&Line::Header {
            schema_version: LOG_SCHEMA_VERSION,
        })?);
    }
    text.push_str(&line_text(line)?);
    file.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    file.sync_data().map_err(|e| e.to_string())
}

/// The log's lines, `None` when there is none. A last line cut short by a crash is ignored.
fn read_log(root: &Path) -> Result<Option<Vec<Line>>, String> {
    let path = safe_path(root, LOG_FILE)?;
    if !path.is_file() {
        return Ok(None);
    }
    let file = open_regular(&path)?;
    let mut lines = Vec::new();
    let mut raw: Vec<String> = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.len() > MAX_LOG_LINE_BYTES {
            return Err("a line is too long".into());
        }
        if !line.trim().is_empty() {
            raw.push(line);
        }
    }
    let count = raw.len();
    for (i, text) in raw.into_iter().enumerate() {
        match serde_json::from_str::<Line>(&text) {
            Ok(Line::Header { schema_version }) if schema_version != LOG_SCHEMA_VERSION => {
                return Err(format!("unsupported version {schema_version}"));
            }
            Ok(line) => lines.push(line),
            Err(_) if i + 1 == count => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(Some(lines))
}

fn set_aside_log(root: &Path) -> Result<String, String> {
    let path = safe_path(root, LOG_FILE)?;
    for n in 1..1000 {
        let name = format!("log.unreadable-{n}.jsonl");
        let target = safe_path(root, &format!("{HISTORY_DIR}/{name}"))?;
        if fs::symlink_metadata(&target).is_ok() {
            continue;
        }
        fs::rename(&path, &target).map_err(|e| e.to_string())?;
        return Ok(format!("{HISTORY_DIR}/{name}"));
    }
    Err("There are too many set-aside histories".into())
}
