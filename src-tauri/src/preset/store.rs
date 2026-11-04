//! Where presets are kept: the built-in ones in code, the user's one folder each in
//! `<config>/AeroEdits/presets/<name>/` (`preset.json`, `method.md`, `rules.md`). The user's
//! can be renamed: projects store a preset's name, so `renamed.json` leads old names to it. Each
//! save that changes one keeps what it replaced in `versions/<n>/`, the same three files.
use super::{builtin, is_builtin, Preset};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub const FILE: &str = "preset.json";
pub const METHOD: &str = "method.md";
pub const RULES: &str = "rules.md";
const MAX_PRESETS: usize = 100;
const RENAMES_FILE: &str = "renamed.json";
const MAX_RENAME_HOPS: usize = 16;
const VERSIONS: &str = "versions";
/// Versions kept of a preset; older ones go.
const MAX_VERSIONS: usize = 50;

/// The app's presets folder (`AEROEDITS_PRESETS_DIR` overrides it).
pub fn dir() -> PathBuf {
    std::env::var_os("AEROEDITS_PRESETS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::style::store::config().join("presets"))
}

/// A preset's name is also a folder name.
pub fn check_name(name: &str) -> Result<(), String> {
    crate::style::store::check_folder_name(name, "preset")
}

pub fn folder_of(preset: &Preset) -> Option<PathBuf> {
    (!is_builtin(&preset.name)).then(|| dir().join(&preset.name))
}

/// Preset `name` from `presets` (the built-ins first; any case).
pub fn load_in(presets: &Path, name: &str) -> Result<Preset, String> {
    let wanted = name.trim();
    if let Some(preset) = builtin()
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(wanted))
    {
        return Ok(preset);
    }
    check_name(wanted).map_err(|_| no_preset(presets, wanted))?;
    let folder = folder_named(presets, &current_name(presets, wanted))
        .ok_or_else(|| no_preset(presets, wanted))?;
    read_files(&presets.join(&folder), &folder).map_err(|e| match e {
        Unreadable::Missing => no_preset(presets, wanted),
        Unreadable::Invalid(why) => why,
    })
}

enum Unreadable {
    Missing,
    Invalid(String),
}

/// The preset whose files are in `at` (its folder, or one of its versions).
fn read_files(at: &Path, name: &str) -> Result<Preset, Unreadable> {
    let text = std::fs::read_to_string(at.join(FILE)).map_err(|_| Unreadable::Missing)?;
    let mut preset: Preset = serde_json::from_str(&text)
        .map_err(|e| Unreadable::Invalid(format!("{}: {e}", at.join(FILE).display())))?;
    preset.name = name.to_string();
    preset.method = std::fs::read_to_string(at.join(METHOD))
        .unwrap_or_default()
        .replace("\r\n", "\n");
    preset.rules = std::fs::read_to_string(at.join(RULES))
        .unwrap_or_default()
        .replace("\r\n", "\n");
    Ok(preset)
}

/// The folder of preset `name` (any case), as it is on disk.
fn folder_named(presets: &Path, name: &str) -> Option<String> {
    std::fs::read_dir(presets)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(FILE).is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .find(|n| n.eq_ignore_ascii_case(name))
}

fn renames(presets: &Path) -> BTreeMap<String, String> {
    crate::style::store::read_renames(&presets.join(RENAMES_FILE))
}

/// Remembers that preset `old` is now `new`; names that led to `old` now lead to `new`.
fn record_rename(presets: &Path, old: &str, new: &str) -> Result<(), String> {
    let mut map = renames(presets);
    for target in map.values_mut() {
        if target.eq_ignore_ascii_case(old) {
            *target = new.to_string();
        }
    }
    map.retain(|k, _| !k.eq_ignore_ascii_case(new));
    map.insert(old.to_lowercase(), new.to_string());
    crate::style::store::write_renames(&presets.join(RENAMES_FILE), &map)
}

/// The preset's name now: `name` while a preset has it, else the name it was renamed to.
pub fn current_name(presets: &Path, name: &str) -> String {
    let map = renames(presets);
    let mut current = name.trim().to_string();
    for _ in 0..MAX_RENAME_HOPS {
        if folder_named(presets, &current).is_some() {
            return current;
        }
        match map.get(&current.to_lowercase()) {
            Some(next) => current = next.clone(),
            None => break,
        }
    }
    name.trim().to_string()
}

fn no_preset(presets: &Path, name: &str) -> String {
    format!(
        "There is no preset \"{name}\" (there are: {})",
        list_in(presets)
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub fn load(name: &str) -> Result<Preset, String> {
    load_in(&dir(), name)
}

/// Preset `name`, or Tutorial when there is no such preset any more. Read from disk only when
/// its files changed.
pub fn resolve(name: &str) -> Preset {
    cached(name).unwrap_or_else(|| load_in(&dir(), super::DEFAULT).unwrap_or_default())
}

pub fn cached(name: &str) -> Option<Preset> {
    if is_builtin(name) {
        return load_in(&dir(), name).ok();
    }
    type Cache = BTreeMap<String, (Vec<Option<std::time::SystemTime>>, Option<Preset>)>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let presets = dir();
    let folder = presets.join(current_name(&presets, name));
    let stamp: Vec<_> = [FILE, METHOD, RULES]
        .iter()
        .map(|f| {
            std::fs::metadata(folder.join(f))
                .and_then(|m| m.modified())
                .ok()
        })
        .collect();
    let cache = CACHE.get_or_init(Default::default);
    if let Some((when, preset)) = cache.lock().ok().and_then(|c| c.get(name).cloned()) {
        if when == stamp && stamp[0].is_some() {
            return preset;
        }
    }
    let preset = load_in(&presets, name).ok();
    if let Ok(mut cache) = cache.lock() {
        cache.insert(name.to_string(), (stamp, preset.clone()));
    }
    preset
}

/// Every preset: the built-ins, then the user's by name.
pub fn list_in(presets: &Path) -> Vec<Preset> {
    let mut own: Vec<Preset> = crate::style::store::folders_with(presets, FILE)
        .iter()
        .filter(|name| !is_builtin(name))
        .filter_map(|name| load_in(presets, name).ok())
        .collect();
    own.sort_by_key(|p| p.name.to_lowercase());
    builtin().into_iter().chain(own).collect()
}

pub fn list() -> Vec<Preset> {
    list_in(&dir())
}

/// A preset as the editor or an agent saves it. `previous` renames one; a built-in is never
/// changed (save it under a name of your own).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetInput {
    pub name: String,
    #[serde(default)]
    pub previous: Option<String>,
    pub preset: Preset,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub rules: String,
}

pub fn save_in(presets: &Path, input: &PresetInput) -> Result<Preset, String> {
    let name = input.name.trim().to_string();
    check_name(&name)?;
    if is_builtin(&name) {
        return Err(format!(
            "\"{name}\" is a built-in preset: save your changes under a name of your own"
        ));
    }
    let previous = input
        .previous
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty() && !is_builtin(p))
        .map(|p| load_in(presets, p).map(|p| p.name))
        .transpose()?;
    let target = presets.join(&name);
    let renamed = previous.as_deref().is_some_and(|p| p != name);
    if previous.is_none() || renamed {
        if folder_named(presets, &name).is_some() {
            return Err(format!("There is already a preset \"{name}\""));
        }
        if target.exists() {
            return Err(format!(
                "Something called \"{name}\" is already in the presets folder"
            ));
        }
        if previous.is_none() && list_in(presets).len() >= MAX_PRESETS + builtin().len() {
            return Err(format!("Keep at most {MAX_PRESETS} presets"));
        }
    }
    let mut preset = input.preset.clone();
    preset.name = name.clone();
    preset.method = input.method.replace("\r\n", "\n");
    preset.rules = input.rules.replace("\r\n", "\n");
    preset.validate()?;
    if let (true, Some(old)) = (renamed, &previous) {
        std::fs::rename(presets.join(old), &target)
            .map_err(|e| format!("{}: {e}", target.display()))?;
        record_rename(presets, old, &name)?;
    }
    std::fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
    // What this replaces is kept when anything changed; a save that changes nothing is no new
    // version. A preset saved before versions were counted is its first.
    let replaced = read_files(&target, &name).ok();
    let same = |old: &Preset| {
        let mut was = old.clone();
        was.version = preset.version;
        was.saved_at.clone_from(&preset.saved_at);
        was == preset
    };
    match &replaced {
        Some(old) if same(old) => {
            preset.version = old.version.max(1);
            preset.saved_at.clone_from(&old.saved_at);
        }
        Some(old) => {
            keep_version(&target, old)?;
            preset.version = old.version.max(1) + 1;
            preset.saved_at = saved_now();
        }
        None => {
            preset.version = 1;
            preset.saved_at = saved_now();
        }
    }
    write_files(&target, &preset)?;
    Ok(preset)
}

fn saved_now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Writes a preset's three files into `at`, each whole or not at all.
fn write_files(at: &Path, preset: &Preset) -> Result<(), String> {
    std::fs::create_dir_all(at).map_err(|e| format!("{}: {e}", at.display()))?;
    let write = |file: &str, text: &str| {
        let path = at.join(file);
        let temp = at.join(format!(".{file}.tmp"));
        std::fs::write(&temp, text).map_err(|e| format!("{}: {e}", path.display()))?;
        std::fs::rename(&temp, &path).map_err(|e| format!("{}: {e}", path.display()))
    };
    let json = serde_json::to_string_pretty(preset).map_err(|e| e.to_string())?;
    write(FILE, &(json + "\n"))?;
    write(METHOD, &preset.method)?;
    write(RULES, &preset.rules)
}

/// The numbers of the versions kept in preset folder `at`, oldest first.
fn kept_versions(at: &Path) -> Vec<u32> {
    let mut kept: Vec<u32> = std::fs::read_dir(at.join(VERSIONS))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_string_lossy().parse().ok())
        .collect();
    kept.sort_unstable();
    kept
}

/// Keeps `old` as a version in preset folder `at`, dropping the oldest past the limit.
fn keep_version(at: &Path, old: &Preset) -> Result<(), String> {
    let number = old.version.max(1);
    let mut kept = old.clone();
    kept.version = number;
    write_files(&at.join(VERSIONS).join(number.to_string()), &kept)?;
    let all = kept_versions(at);
    for gone in all.iter().take(all.len().saturating_sub(MAX_VERSIONS)) {
        let _ = std::fs::remove_dir_all(at.join(VERSIONS).join(gone.to_string()));
    }
    Ok(())
}

/// One save of a preset, as the list of them shows it.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionView {
    pub version: u32,
    pub saved_at: String,
    pub current: bool,
    pub summary: String,
}

/// Every save kept of preset `name`, the current one first.
pub fn versions_in(presets: &Path, name: &str) -> Result<Vec<VersionView>, String> {
    let current = load_in(presets, name)?;
    let view = |preset: &Preset, current: bool| VersionView {
        version: preset.version.max(1),
        saved_at: preset.saved_at.clone(),
        current,
        summary: super::workflow::summary(preset),
    };
    let mut out = vec![view(&current, true)];
    if is_builtin(&current.name) {
        return Ok(out);
    }
    let at = presets.join(&current.name);
    for number in kept_versions(&at).into_iter().rev() {
        let folder = at.join(VERSIONS).join(number.to_string());
        if let Ok(old) = read_files(&folder, &current.name) {
            out.push(view(&old, false));
        }
    }
    Ok(out)
}

/// Makes version `version` of preset `name` the current one, as a new version: what it replaces
/// stays in the list.
pub fn restore_in(presets: &Path, name: &str, version: u32) -> Result<Preset, String> {
    let current = load_in(presets, name)?;
    if is_builtin(&current.name) {
        return Err(format!("{} is built in and has no versions", current.name));
    }
    if version == current.version.max(1) {
        return Err(format!("{} is at version {version} already", current.name));
    }
    let folder = presets
        .join(&current.name)
        .join(VERSIONS)
        .join(version.to_string());
    let old = read_files(&folder, &current.name)
        .map_err(|_| format!("{} has no version {version}", current.name))?;
    save_in(
        presets,
        &PresetInput {
            name: current.name.clone(),
            previous: Some(current.name),
            method: old.method.clone(),
            rules: old.rules.clone(),
            preset: old,
        },
    )
}

pub fn versions(name: &str) -> Result<Vec<VersionView>, String> {
    versions_in(&dir(), name)
}

pub fn restore(name: &str, version: u32) -> Result<Preset, String> {
    restore_in(&dir(), name, version)
}

pub fn save(input: &PresetInput) -> Result<Preset, String> {
    save_in(&dir(), input)
}

pub fn remove_in(presets: &Path, name: &str) -> Result<(), String> {
    if is_builtin(name) {
        return Err(format!(
            "\"{name}\" is a built-in preset and can't be removed"
        ));
    }
    let preset = load_in(presets, name)?;
    let folder = presets.join(&preset.name);
    std::fs::remove_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    // Its old names lead nowhere now.
    let mut map = renames(presets);
    let before = map.len();
    map.retain(|_, target| !target.eq_ignore_ascii_case(&preset.name));
    if map.len() != before {
        let _ = crate::style::store::write_renames(&presets.join(RENAMES_FILE), &map);
    }
    Ok(())
}

pub fn remove(name: &str) -> Result<(), String> {
    remove_in(&dir(), name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> PresetInput {
        PresetInput {
            name: name.into(),
            previous: None,
            preset: Preset {
                description: "Mine".into(),
                ..Preset::default()
            },
            method: "# Steps\n".into(),
            rules: "Never cut the intro.\n".into(),
        }
    }

    #[test]
    fn each_change_is_a_version_that_can_be_restored() {
        let dir = tempfile::tempdir().unwrap();
        let presets = dir.path();
        let saved = save_in(presets, &input("Mine")).unwrap();
        assert_eq!(saved.version, 1);
        assert!(!saved.saved_at.is_empty());

        // Saved again unchanged: no new version.
        let mut again = input("Mine");
        again.previous = Some("Mine".into());
        assert_eq!(save_in(presets, &again).unwrap().version, 1);
        assert_eq!(versions_in(presets, "Mine").unwrap().len(), 1);

        // A change: the old one is kept, whichever part changed.
        let mut changed = again.clone();
        changed.preset.checks = vec!["The intro is under 20 s".into()];
        changed.rules = "Keep the intro.\n".into();
        assert_eq!(save_in(presets, &changed).unwrap().version, 2);
        let listed = versions_in(presets, "Mine").unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|v| (v.version, v.current))
                .collect::<Vec<_>>(),
            [(2, true), (1, false)]
        );
        assert!(
            listed[0].summary.contains("The intro is under 20 s"),
            "{}",
            listed[0].summary
        );

        // Restored: version 1's content, as version 3, with version 2 still there.
        let back = restore_in(presets, "Mine", 1).unwrap();
        assert_eq!(back.version, 3);
        assert_eq!(back.rules, "Never cut the intro.\n");
        assert!(back.checks.is_empty());
        assert_eq!(load_in(presets, "Mine").unwrap().version, 3);
        assert_eq!(
            versions_in(presets, "Mine")
                .unwrap()
                .iter()
                .map(|v| v.version)
                .collect::<Vec<_>>(),
            [3, 2, 1]
        );
        assert!(restore_in(presets, "Mine", 3)
            .unwrap_err()
            .contains("already"));
        assert!(restore_in(presets, "Mine", 9)
            .unwrap_err()
            .contains("no version"));
        assert!(restore_in(presets, "Tutorial", 1).is_err());

        // A rename takes its versions along; a preset's versions aren't presets of their own.
        let mut renamed = changed.clone();
        renamed.name = "Ours".into();
        renamed.previous = Some("Mine".into());
        assert_eq!(save_in(presets, &renamed).unwrap().version, 4);
        assert_eq!(versions_in(presets, "Ours").unwrap().len(), 4);
        assert_eq!(list_in(presets).len(), builtin().len() + 1);
    }

    #[test]
    fn only_so_many_versions_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let presets = dir.path();
        save_in(presets, &input("Mine")).unwrap();
        for n in 0..MAX_VERSIONS + 5 {
            let mut next = input("Mine");
            next.previous = Some("Mine".into());
            next.rules = format!("Rule {n}\n");
            save_in(presets, &next).unwrap();
        }
        let listed = versions_in(presets, "Mine").unwrap();
        // The current one, and the limit of those before it.
        assert_eq!(listed.len(), MAX_VERSIONS + 1);
        assert_eq!(listed[0].version as usize, MAX_VERSIONS + 6);
    }

    #[test]
    fn presets_are_saved_listed_renamed_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let presets = dir.path();
        let mut first = input("Gaming");
        first.preset.values = serde_json::json!({ "captions": { "enabled": true } })
            .as_object()
            .cloned()
            .unwrap();
        save_in(presets, &first).unwrap();
        let loaded = load_in(presets, "gaming").unwrap();
        assert_eq!(loaded.name, "Gaming");
        assert_eq!(loaded.rules, "Never cut the intro.\n");
        assert_eq!(loaded.values["captions"]["enabled"], true);
        assert!(save_in(presets, &input("Gaming"))
            .unwrap_err()
            .contains("already"));
        let mut renamed = input("Gaming long");
        renamed.previous = Some("Gaming".into());
        save_in(presets, &renamed).unwrap();
        let names: Vec<String> = list_in(presets).into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["Tutorial", "Shorts", "Clips", "Gaming long"]);
        // Projects that stored the old name still find it.
        assert_eq!(load_in(presets, "gaming").unwrap().name, "Gaming long");
        for bad in ["", "../x", "Tutorial", "clips"] {
            assert!(save_in(presets, &input(bad)).is_err(), "{bad:?}");
        }
        let mut wrong = input("Bad");
        wrong.preset.values = serde_json::json!({ "captions": { "fontSizePct": 99 } })
            .as_object()
            .cloned()
            .unwrap();
        assert!(save_in(presets, &wrong).is_err());
        wrong.preset.values = serde_json::json!({ "shorts": [] })
            .as_object()
            .cloned()
            .unwrap();
        assert!(save_in(presets, &wrong).is_err());
        // A new preset may take an old name, which is its own from then on.
        save_in(presets, &input("Gaming")).unwrap();
        assert_eq!(load_in(presets, "Gaming").unwrap().name, "Gaming");
        remove_in(presets, "Gaming").unwrap();
        remove_in(presets, "Gaming long").unwrap();
        assert!(load_in(presets, "Gaming long").is_err());
        assert!(remove_in(presets, "Tutorial").is_err());
    }
}
