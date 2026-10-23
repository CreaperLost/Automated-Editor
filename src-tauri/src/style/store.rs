//! Where styles are kept: the built-in ones in code, the user's one folder each in
//! `<config>/AeroEdits/styles/<name>/` (`style.json`, with its font and logo files beside it).
//! A style renamed keeps answering to its old names (projects and graphics store names).
use super::{builtin, is_builtin, Style};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub const FILE: &str = "style.json";
const MAX_STYLES: usize = 100;
const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
const LOGO_EXTENSIONS: &[&str] = &["png", "svg", "jpg", "jpeg", "webp"];
const FONT_EXTENSIONS: &[&str] = &["woff2", "woff", "ttf", "otf"];
const RENAMES_FILE: &str = "renamed.json";
const MAX_RENAME_HOPS: usize = 16;

/// The app's styles folder (`AEROEDITS_STYLES_DIR` overrides it; tests use a folder of their
/// own, never the user's).
pub fn dir() -> PathBuf {
    std::env::var_os("AEROEDITS_STYLES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| config().join("styles"))
}

/// The app's config folder, or a test run's own.
pub(crate) fn config() -> PathBuf {
    #[cfg(test)]
    {
        static TEST: OnceLock<tempfile::TempDir> = OnceLock::new();
        TEST.get_or_init(|| tempfile::tempdir().unwrap())
            .path()
            .to_path_buf()
    }
    #[cfg(not(test))]
    crate::transcript::settings::config_dir()
}

/// A style's name is also a folder name: letters, digits, spaces, `-` and `_`, 1 to 40.
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
        Err("Name a style with letters, digits, spaces, - or _ (at most 40)".into())
    }
}

/// A file name in a style's folder (no paths).
pub fn plain_file(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 120
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':'])
        && name != FILE
}

/// A style's folder, if it has one (built-in styles have none).
pub fn folder_of(style: &Style) -> Option<PathBuf> {
    (!is_builtin(&style.name)).then(|| dir().join(&style.name))
}

/// Style `name` from `styles` (the built-ins first), or the style it was renamed to.
pub fn load_in(styles: &Path, name: &str) -> Result<Style, String> {
    let wanted = name.trim();
    if let Some(style) = builtin()
        .into_iter()
        .find(|s| s.name.eq_ignore_ascii_case(wanted))
    {
        return Ok(style);
    }
    check_name(wanted).map_err(|_| format!("There is no style \"{wanted}\""))?;
    let name = current_name(styles, wanted);
    let path = styles.join(&name).join(FILE);
    let text = std::fs::read_to_string(&path).map_err(|_| no_style(styles, wanted))?;
    let mut style: Style =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    style.name = name;
    style
        .validate()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(style)
}

fn no_style(styles: &Path, name: &str) -> String {
    format!(
        "There is no style \"{name}\" (there are: {})",
        list_in(styles)
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub fn load(name: &str) -> Result<Style, String> {
    load_in(&dir(), name)
}

/// Style `name`, or Light when there is no such style any more.
pub fn resolve(name: &str) -> Style {
    cached(name).unwrap_or_default()
}

/// Style `name` if there is one, read from disk only when its file changed, so drawing a
/// frame can ask for it.
pub fn cached(name: &str) -> Option<Style> {
    type Cache = BTreeMap<String, (Option<std::time::SystemTime>, Option<Style>)>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let styles = dir();
    let stamp = |n: &str| {
        let folder = styles.join(current_name(&styles, n));
        let style = std::fs::metadata(folder.join(FILE))
            .and_then(|m| m.modified())
            .ok();
        let renames = std::fs::metadata(styles.join(RENAMES_FILE))
            .and_then(|m| m.modified())
            .ok();
        style.max(renames)
    };
    if is_builtin(name) {
        return load_in(&styles, name).ok();
    }
    let modified = stamp(name);
    let cache = CACHE.get_or_init(Default::default);
    if let Some((when, style)) = cache.lock().ok().and_then(|c| c.get(name).cloned()) {
        if when == modified && modified.is_some() {
            return style;
        }
    }
    let style = load_in(&styles, name).ok();
    if let Ok(mut cache) = cache.lock() {
        cache.insert(name.to_string(), (modified, style.clone()));
    }
    style
}

/// Every style: the built-ins, then the user's by name.
pub fn list_in(styles: &Path) -> Vec<Style> {
    let mut own: Vec<Style> = std::fs::read_dir(styles)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(FILE).is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            (!is_builtin(&name)).then(|| load_in(styles, &name).ok())?
        })
        .collect();
    own.sort_by_key(|s| s.name.to_lowercase());
    builtin().into_iter().chain(own).collect()
}

pub fn list() -> Vec<Style> {
    list_in(&dir())
}

/// A style as the editor or an agent saves it. Its font and logo files are each the style's
/// own file (kept), a full path (copied in), or absent. `previous` names the style it was (to
/// rename it, or to copy a built-in, which is never changed itself).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleInput {
    pub name: String,
    #[serde(default)]
    pub previous: Option<String>,
    pub style: Style,
}

/// Saves a style (new, changed, renamed, or a copy of a built-in) and returns it.
pub fn save_in(styles: &Path, input: &StyleInput) -> Result<Style, String> {
    let name = input.name.trim().to_string();
    check_name(&name)?;
    if is_builtin(&name) {
        return Err(format!(
            "\"{name}\" is a built-in style: save your changes under a name of your own"
        ));
    }
    let previous = input
        .previous
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty());
    // A built-in is copied, never changed: it has no folder to take files from.
    let previous = previous
        .filter(|p| !is_builtin(p))
        .map(|p| current_name(styles, p));
    let previous = previous.as_deref();
    let renamed = previous.is_some_and(|p| p != name);
    let target = styles.join(&name);
    if previous.is_none() || renamed {
        if target.join(FILE).is_file() {
            return Err(format!("There is already a style \"{name}\""));
        }
        if target.exists() {
            return Err(format!(
                "Something called \"{name}\" is already in the styles folder"
            ));
        }
    }
    if previous.is_none() && list_in(styles).len() >= MAX_STYLES + builtin().len() {
        return Err(format!("Keep at most {MAX_STYLES} styles"));
    }
    let source = match previous {
        Some(p) => {
            check_name(p)?;
            let dir = styles.join(p);
            if !dir.join(FILE).is_file() {
                return Err(format!("There is no style \"{p}\""));
            }
            Some(dir)
        }
        None => None,
    };
    let mut style = input.style.clone().normalized();
    style.name = name.clone();
    // Checked without its files, which are taken in below.
    let mut bare = style.clone();
    bare.brand.fonts.title.file = None;
    bare.brand.fonts.body.file = None;
    bare.brand.logo = None;
    bare.validate()?;
    std::fs::create_dir_all(styles).map_err(|e| format!("{}: {e}", styles.display()))?;
    let stage = styles.join(format!(".saving-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&stage).map_err(|e| format!("{}: {e}", stage.display()))?;
    let made = (|| {
        let src = source.as_deref();
        style.brand.logo = take_file(
            input.style.brand.logo.as_deref(),
            src,
            &stage,
            "logo",
            LOGO_EXTENSIONS,
        )?;
        style.brand.fonts.title.file = take_file(
            input.style.brand.fonts.title.file.as_deref(),
            src,
            &stage,
            "title-font",
            FONT_EXTENSIONS,
        )?;
        style.brand.fonts.body.file = take_file(
            input.style.brand.fonts.body.file.as_deref(),
            src,
            &stage,
            "body-font",
            FONT_EXTENSIONS,
        )?;
        let text = serde_json::to_string_pretty(&style).map_err(|e| e.to_string())?;
        std::fs::write(stage.join(FILE), text + "\n").map_err(|e| e.to_string())
    })();
    if let Err(error) = made {
        let _ = std::fs::remove_dir_all(&stage);
        return Err(error);
    }
    if let (true, Some(old)) = (renamed, previous) {
        if let Err(error) = record_rename(styles, old, &name) {
            let _ = std::fs::remove_dir_all(&stage);
            return Err(error);
        }
    }
    // The old folder is set aside, not deleted, until the new one is in place.
    let aside = match &source {
        Some(old) => {
            let backup = styles.join(format!(".old-{}", uuid::Uuid::new_v4().simple()));
            if let Err(e) = std::fs::rename(old, &backup) {
                let _ = std::fs::remove_dir_all(&stage);
                return Err(format!("{}: {e}", old.display()));
            }
            Some((old, backup))
        }
        None => None,
    };
    if let Err(e) = std::fs::rename(&stage, &target) {
        let _ = std::fs::remove_dir_all(&stage);
        if let Some((old, backup)) = &aside {
            let _ = std::fs::rename(backup, old);
        }
        return Err(format!("{}: {e}", target.display()));
    }
    if let Some((_, backup)) = aside {
        let _ = std::fs::remove_dir_all(backup);
    }
    crate::graphics::render::sources_changed();
    Ok(style)
}

pub fn save(input: &StyleInput) -> Result<Style, String> {
    save_in(&dir(), input)
}

pub fn remove_in(styles: &Path, name: &str) -> Result<(), String> {
    if is_builtin(name) {
        return Err(format!(
            "\"{name}\" is a built-in style and can't be removed"
        ));
    }
    check_name(name)?;
    let name = current_name(styles, name);
    let dir = styles.join(&name);
    if !dir.join(FILE).is_file() {
        return Err(format!("There is no style \"{name}\""));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut map = renames(styles);
    let before = map.len();
    map.retain(|_, target| *target != name);
    if map.len() != before {
        let _ = write_renames(styles, &map);
    }
    crate::graphics::render::sources_changed();
    Ok(())
}

pub fn remove(name: &str) -> Result<(), String> {
    remove_in(&dir(), name)
}

fn renames(styles: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(styles.join(RENAMES_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_renames(styles: &Path, renames: &BTreeMap<String, String>) -> Result<(), String> {
    let path = styles.join(RENAMES_FILE);
    let text = serde_json::to_string_pretty(renames).map_err(|e| e.to_string())?;
    std::fs::write(&path, text + "\n").map_err(|e| format!("{}: {e}", path.display()))
}

fn record_rename(styles: &Path, old: &str, new: &str) -> Result<(), String> {
    let mut map = renames(styles);
    for target in map.values_mut() {
        if target == old {
            *target = new.to_string();
        }
    }
    map.remove(new);
    map.insert(old.to_string(), new.to_string());
    write_renames(styles, &map)
}

/// The style's name now: `name` while a style has it, else the name it was renamed to.
pub fn current_name(styles: &Path, name: &str) -> String {
    if is_builtin(name) {
        return builtin()
            .into_iter()
            .find(|s| s.name.eq_ignore_ascii_case(name.trim()))
            .map(|s| s.name)
            .unwrap_or_default();
    }
    let map = renames(styles);
    let mut current = name.to_string();
    for _ in 0..MAX_RENAME_HOPS {
        if styles.join(&current).join(FILE).is_file() {
            return current;
        }
        match map.get(&current) {
            Some(next) => current = next.clone(),
            None => break,
        }
    }
    if styles.join(&current).join(FILE).is_file() {
        current
    } else {
        name.to_string()
    }
}

/// The style's file for `given`: its own file kept (copied from `from`), a full path copied
/// in as `<stem>.<ext>`, or none.
fn take_file(
    given: Option<&str>,
    from: Option<&Path>,
    stage: &Path,
    stem: &str,
    extensions: &[&str],
) -> Result<Option<String>, String> {
    let Some(given) = given.map(str::trim).filter(|g| !g.is_empty()) else {
        return Ok(None);
    };
    let path = if plain_file(given) && !Path::new(given).is_absolute() {
        from.map(|k| k.join(given))
            .filter(|p| p.is_file())
            .ok_or_else(|| format!("The style has no file \"{given}\""))?
    } else {
        PathBuf::from(given)
    };
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| extensions.contains(&e.as_str()))
        .ok_or_else(|| format!("A {stem} is a {} file", extensions.join(", ")))?;
    let size = std::fs::metadata(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if size > MAX_FILE_BYTES {
        return Err(format!("That {stem} file is too big"));
    }
    let name = format!("{stem}.{ext}");
    std::fs::copy(&path, stage.join(&name)).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Some(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> StyleInput {
        StyleInput {
            name: name.into(),
            previous: None,
            style: Style::default(),
        }
    }

    #[test]
    fn styles_are_saved_listed_renamed_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let styles = dir.path();
        let logo = dir.path().join("mark.svg");
        std::fs::write(&logo, "<svg/>").unwrap();
        let mut first = input("Acme");
        first.style.brand.logo = Some(logo.display().to_string());
        first.style.brand.colors.accent = "#ff5500".into();
        let saved = save_in(styles, &first).unwrap();
        assert_eq!(saved.brand.logo.as_deref(), Some("logo.svg"));
        assert_eq!(saved.brand.colors.accent, "#FF5500");
        assert!(styles.join("Acme").join("logo.svg").is_file());
        assert!(save_in(styles, &input("Acme"))
            .unwrap_err()
            .contains("already"));
        // Renamed, keeping its logo by its own file name.
        let mut renamed = input("Acme Dark");
        renamed.previous = Some("Acme".into());
        renamed.style.brand.logo = Some("logo.svg".into());
        assert_eq!(
            save_in(styles, &renamed).unwrap().brand.logo.as_deref(),
            Some("logo.svg")
        );
        assert!(!styles.join("Acme").exists());
        assert_eq!(load_in(styles, "Acme").unwrap().name, "Acme Dark");
        let names: Vec<String> = list_in(styles).into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["Light", "Dark", "Acme Dark"]);
        for bad in ["", "../x", "a/b", "Light", "dark"] {
            assert!(save_in(styles, &input(bad)).is_err(), "{bad:?}");
        }
        remove_in(styles, "Acme").unwrap();
        assert!(load_in(styles, "Acme Dark").is_err());
        assert!(remove_in(styles, "Light").is_err());
    }

    #[test]
    fn a_builtin_is_copied_never_changed() {
        let dir = tempfile::tempdir().unwrap();
        let mut copy = input("My dark");
        copy.previous = Some("Dark".into());
        copy.style = load_in(dir.path(), "dark").unwrap();
        copy.style.brand.colors.accent = "#00FF00".into();
        save_in(dir.path(), &copy).unwrap();
        assert_eq!(
            load_in(dir.path(), "My dark").unwrap().brand.colors.accent,
            "#00FF00"
        );
        assert_eq!(
            load_in(dir.path(), "Dark").unwrap().brand.colors.accent,
            "#0A84FF"
        );
    }

    #[test]
    fn a_save_never_deletes_what_is_in_the_way() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Notes")).unwrap();
        std::fs::write(dir.path().join("Notes").join("todo.txt"), "keep").unwrap();
        assert!(save_in(dir.path(), &input("Notes")).is_err());
        assert!(dir.path().join("Notes").join("todo.txt").is_file());
    }
}
