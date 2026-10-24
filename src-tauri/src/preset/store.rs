//! Where presets are kept: the built-in ones in code, the user's one folder each in
//! `<config>/AeroEdits/presets/<name>/` (`preset.json`, `method.md`, `rules.md`). The user's
//! can be renamed: projects store a preset's name, so `renamed.json` leads old names to it.
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

/// The app's presets folder (`AEROEDITS_PRESETS_DIR` overrides it).
pub fn dir() -> PathBuf {
    std::env::var_os("AEROEDITS_PRESETS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::style::store::config().join("presets"))
}

/// A preset's name is also a folder name: letters, digits, spaces, `-` and `_`, 1 to 40.
pub fn check_name(name: &str) -> Result<(), String> {
    let ok = !name.trim().is_empty()
        && name == name.trim()
        && name.chars().count() <= 40
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'));
    if ok {
        Ok(())
    } else {
        Err("Name a preset with letters, digits, spaces, - or _ (at most 40)".into())
    }
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
    let at = presets.join(&folder);
    let text = std::fs::read_to_string(at.join(FILE)).map_err(|_| no_preset(presets, wanted))?;
    let mut preset: Preset =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", at.join(FILE).display()))?;
    preset.name = folder;
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
    std::fs::read_to_string(presets.join(RENAMES_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_renames(presets: &Path, map: &BTreeMap<String, String>) -> Result<(), String> {
    let path = presets.join(RENAMES_FILE);
    let text = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(&path, text + "\n").map_err(|e| format!("{}: {e}", path.display()))
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
    write_renames(presets, &map)
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
    let mut own: Vec<Preset> = std::fs::read_dir(presets)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(FILE).is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            (!is_builtin(&name)).then(|| load_in(presets, &name).ok())?
        })
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
    let write = |file: &str, text: &str| {
        let path = target.join(file);
        let temp = target.join(format!(".{file}.tmp"));
        std::fs::write(&temp, text).map_err(|e| format!("{}: {e}", path.display()))?;
        std::fs::rename(&temp, &path).map_err(|e| format!("{}: {e}", path.display()))
    };
    let json = serde_json::to_string_pretty(&preset).map_err(|e| e.to_string())?;
    write(FILE, &(json + "\n"))?;
    write(METHOD, &preset.method)?;
    write(RULES, &preset.rules)?;
    Ok(preset)
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
        let _ = write_renames(presets, &map);
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
