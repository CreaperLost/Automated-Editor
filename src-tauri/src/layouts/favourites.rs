//! Favourite layouts: your own, under a name, kept for the app rather than one project (next to
//! the templates). Using one copies its layout into the section, so changing or removing a
//! favourite later leaves every project as it was.
use super::Layout;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MAX_FAVOURITES: usize = 100;
const FILE: &str = "layouts.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Favourite {
    pub id: String,
    pub name: String,
    pub layout: Layout,
}

/// The saved favourites, in the order they were saved; none when the file is missing or bad.
pub fn list(dir: &Path) -> Vec<Favourite> {
    std::fs::read(dir.join(FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn store(dir: &Path, favourites: &[Favourite]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(favourites).map_err(|e| e.to_string())?;
    let temp = dir.join(format!("{FILE}.tmp"));
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, dir.join(FILE)).map_err(|e| e.to_string())
}

/// Saves `layout` as `name`, replacing a favourite of the same name (it keeps that one's id).
pub fn save(dir: &Path, name: &str, layout: Layout) -> Result<Vec<Favourite>, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) {
        return Err("A favourite needs a one-line name of at most 60 characters".into());
    }
    if super::built_in(name).is_some() {
        return Err(format!(
            "\"{name}\" is a built-in layout; pick another name"
        ));
    }
    layout.validate()?;
    let mut favourites = list(dir);
    match favourites
        .iter_mut()
        .find(|f| f.name.eq_ignore_ascii_case(name))
    {
        Some(same) => {
            same.name = name.to_string();
            same.layout = layout;
        }
        None => {
            if favourites.len() >= MAX_FAVOURITES {
                return Err(format!("Keep at most {MAX_FAVOURITES} favourite layouts"));
            }
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or_default();
            let mut id = format!("layout-{stamp:x}");
            while favourites.iter().any(|f| f.id == id) {
                id.push('x');
            }
            favourites.push(Favourite {
                id,
                name: name.to_string(),
                layout,
            });
        }
    }
    store(dir, &favourites)?;
    Ok(favourites)
}

/// Removes the favourite with this id or name.
pub fn remove(dir: &Path, which: &str) -> Result<Vec<Favourite>, String> {
    let mut favourites = list(dir);
    let before = favourites.len();
    favourites.retain(|f| f.id != which && !f.name.eq_ignore_ascii_case(which.trim()));
    if favourites.len() == before {
        return Err("That favourite layout is gone".into());
    }
    store(dir, &favourites)?;
    Ok(favourites)
}

/// A layout by name: a built-in, else a favourite (by name or id). Returns its display name.
pub fn named(dir: &Path, which: &str) -> Option<(String, Layout)> {
    if let Some(layout) = super::built_in(which) {
        let name = super::built_ins()
            .into_iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(which.trim()))
            .map(|(n, _)| n.to_string())
            .unwrap_or_default();
        return Some((name, layout));
    }
    list(dir)
        .into_iter()
        .find(|f| f.id == which || f.name.eq_ignore_ascii_case(which.trim()))
        .map(|f| (f.name, f.layout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::{CameraOnly, SideBySide};

    #[test]
    fn saving_by_name_replaces_and_built_in_names_are_taken() {
        let dir = tempfile::tempdir().unwrap();
        let wide = Layout::SideBySide(SideBySide {
            camera_pct: 25.0,
            ..SideBySide::default()
        });
        let saved = save(dir.path(), "Narrow camera", wide.clone()).unwrap();
        assert_eq!(saved.len(), 1);
        let again = save(
            dir.path(),
            "narrow camera",
            Layout::CameraOnly(CameraOnly::default()),
        )
        .unwrap();
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].id, saved[0].id);
        assert!(save(dir.path(), "Screen only", wide.clone()).is_err());
        assert_eq!(
            named(dir.path(), "NARROW CAMERA").map(|(_, l)| l),
            Some(Layout::CameraOnly(CameraOnly::default()))
        );
        assert_eq!(named(dir.path(), "side by side").unwrap().0, "Side by side");
        assert!(remove(dir.path(), &saved[0].id).unwrap().is_empty());
        assert!(named(dir.path(), "Narrow camera").is_none());
    }
}
