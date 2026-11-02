//! What the sound and music libraries share: the user's own files (copied into `mine`, listed in
//! `mine/mine.json`), tags and search, and a library file as a project's media.
use crate::project::revision::EditDocument;
use crate::sequence::{Asset, Licence, Role};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MINE_DIR: &str = "mine";
const MINE_FILE: &str = "mine.json";
const MAX_MINE: usize = 500;

/// A file of the user's own, as `mine.json` keeps it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Mine {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub duration_us: u64,
    /// The copy's file name inside `mine`.
    pub file: String,
    pub licence: Licence,
}

/// Where a copy of the user's own is kept.
pub(crate) fn mine_path(root: &Path, file: &str) -> PathBuf {
    root.join(MINE_DIR).join(file)
}

pub(crate) fn load(root: &Path) -> Vec<Mine> {
    crate::files::load_json(&mine_path(root, MINE_FILE)).unwrap_or_default()
}

pub(crate) fn save(root: &Path, mine: &[Mine]) -> Result<(), String> {
    crate::files::save_json(&mine_path(root, MINE_FILE), &mine)
}

/// Copies `file` into the library as `noun` ("sound", "track") under a free id (`taken` by a
/// built-in one otherwise) and adds it to `mine`. Answers its id.
#[allow(clippy::too_many_arguments)]
pub(crate) fn copy_in(
    root: &Path,
    mine: &mut Vec<Mine>,
    file: &Path,
    noun: &str,
    default_extension: &str,
    name: Option<String>,
    tags: Vec<String>,
    duration_us: u64,
    licence: Licence,
    taken: impl Fn(&str) -> bool,
) -> Result<String, String> {
    if mine.len() >= MAX_MINE {
        return Err(format!(
            "The library holds {MAX_MINE} {noun}s of your own at most"
        ));
    }
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| noun.into());
    let slug = slug(&stem);
    let base = format!("my-{}", if slug.is_empty() { noun } else { &slug });
    let id = crate::ids::free_id(&base, |id| taken(id) || mine.iter().any(|m| m.id == id));
    let extension = file
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| default_extension.into());
    let copy = format!("{id}.{extension}");
    fs::create_dir_all(root.join(MINE_DIR)).map_err(|e| e.to_string())?;
    fs::copy(file, mine_path(root, &copy))
        .map_err(|e| format!("Could not copy the {noun}: {e}"))?;
    mine.push(Mine {
        id: id.clone(),
        name: name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or(stem),
        tags: tidy_tags(tags),
        duration_us,
        file: copy,
        licence,
    });
    Ok(id)
}

/// Changes what was given of a file of the user's own.
pub(crate) fn change(
    entry: &mut Mine,
    name: Option<String>,
    tags: Option<Vec<String>>,
    licence: Option<Licence>,
) {
    if let Some(name) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
        entry.name = name;
    }
    if let Some(tags) = tags {
        entry.tags = tidy_tags(tags);
    }
    if let Some(licence) = licence {
        entry.licence = licence;
    }
}

/// Takes a file of the user's out of the library (projects that used it keep their media).
pub(crate) fn remove(root: &Path, id: &str, noun: &str) -> Result<(), String> {
    let mut mine = load(root);
    let Some(at) = mine.iter().position(|m| m.id == id) else {
        return Err(format!("No {noun} {id} of your own"));
    };
    let entry = mine.remove(at);
    save(root, &mine)?;
    let _ = fs::remove_file(mine_path(root, &entry.file));
    Ok(())
}

pub(crate) fn tidy_tags(tags: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag = tag.trim().to_lowercase();
        if !tag.is_empty() && tag.chars().count() <= 30 && !out.contains(&tag) {
            out.push(tag);
        }
    }
    out.truncate(12);
    out
}

/// A file name as an id: lower case letters and digits, at most six words joined by dashes.
fn slug(stem: &str) -> String {
    stem.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("-")
}

/// The items matching `query`: every word of it in what `searched` says of them.
pub(crate) fn search<T>(items: Vec<T>, query: &str, searched: impl Fn(&T) -> String) -> Vec<T> {
    let words: Vec<String> = query
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    items
        .into_iter()
        .filter(|item| {
            let hay = searched(item).to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .collect()
}

/// The project's media for a library file: the asset already holding it, or a new one imported
/// from it with its sounds in `role` and the file's licence. The second value says whether it
/// is new.
pub(crate) fn asset_for(
    document: &EditDocument,
    project_root: &Path,
    file: &Path,
    name: &str,
    licence: &Licence,
    role: Role,
) -> Result<(Asset, bool), String> {
    let same = |a: &Asset| {
        Path::new(&a.path) == file
            || dunce::canonicalize(file)
                .map(|p| Path::new(&a.path) == p)
                .unwrap_or(false)
    };
    if let Some(asset) = document.assets.iter().find(|a| same(a)) {
        return Ok((asset.clone(), false));
    }
    let mut asset = crate::media_bin::import(project_root, file)?;
    asset.name = name.to_string();
    for stream in &mut asset.streams {
        stream.role = role;
    }
    asset.licence = Some(licence.clone());
    Ok((asset, true))
}
