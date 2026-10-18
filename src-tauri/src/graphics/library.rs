//! The app's graphics library, `<config>/AeroEdits/graphics/<id>/`: graphics saved from a
//! project to use again in others. An item is a copy of the graphic's folder (its composition,
//! files and `graphic.json`) plus `library.json` with its label, when to use it, its default
//! length, its version, where it came from and whether it passed the gate (see [`super::gate`])
//! as a general template or was saved as is. The gate's report and contact sheet are kept beside
//! it as dot files, which never travel into a project. Using an item copies it into the
//! project, so a project never points outside itself.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const ITEM_FILE: &str = "library.json";
/// The gate's report and contact sheet for the version saved.
pub const CHECK_REPORT: &str = ".check.json";
pub const CHECK_SHEET: &str = ".check.png";
const MAX_ITEMS: usize = 500;
const MAX_LABEL_CHARS: usize = 80;
const MAX_USE_WHEN_CHARS: usize = 240;
const MAX_NAME_CHARS: usize = 40;

/// What `library.json` holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    /// Its short name for agents (`graphic_add library`): lowercase words joined with `-`,
    /// unique in the library and unlike any bundled template's.
    pub name: String,
    pub label: String,
    /// One line on when to use it, so an agent can pick it.
    pub use_when: String,
    /// How long it plays when used.
    pub length_us: u64,
    /// 1 when first saved, one more each time it is updated.
    pub version: u32,
    /// The project it was saved from (its name and folder), for the latest version.
    pub project: Option<String>,
    pub project_folder: Option<String>,
    /// RFC 3339.
    pub created_at: String,
    pub updated_at: String,
    /// Passed the gate: a general template. Otherwise saved as is.
    pub general: bool,
    /// The checks it failed, when saved as is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<String>,
}

/// The app's graphics library folder (`AEROEDITS_GRAPHICS_LIBRARY` overrides it).
pub fn dir() -> PathBuf {
    std::env::var_os("AEROEDITS_GRAPHICS_LIBRARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::transcript::settings::config_dir().join("graphics"))
}

/// What the Graphics page and agents see of an item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryView {
    pub id: String,
    #[serde(flatten)]
    pub item: LibraryItem,
    /// The bundled template it was first made from, if any.
    pub template: Option<String>,
    pub variables: Vec<super::VariableDecl>,
    pub folder: String,
    /// The gate's contact sheet for this version, if it was checked.
    pub sheet: Option<String>,
}

fn check_id(id: &str) -> Result<(), String> {
    let ok = id.len() <= 64
        && id.starts_with("lib-")
        && id[4..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(format!("There is no graphic \"{id}\" in the library"))
    }
}

/// The folder of library item `id_or_name` (its id, or its short name).
pub fn find(library: &Path, id_or_name: &str) -> Result<PathBuf, String> {
    let id = resolve(library, id_or_name)?;
    let dir = library.join(&id);
    if dir.join(ITEM_FILE).is_file() && dir.join(super::ENTRY).is_file() {
        Ok(dir)
    } else {
        Err(format!(
            "There is no graphic \"{id_or_name}\" in the library"
        ))
    }
}

/// The id of the item `id_or_name` names.
fn resolve(library: &Path, id_or_name: &str) -> Result<String, String> {
    let wanted = id_or_name.trim();
    if wanted.starts_with("lib-") {
        check_id(wanted)?;
        return Ok(wanted.to_string());
    }
    list(library)
        .into_iter()
        .find(|v| v.item.name == wanted)
        .map(|v| v.id)
        .ok_or_else(|| format!("There is no graphic \"{wanted}\" in the library"))
}

fn read_item(dir: &Path) -> Result<LibraryItem, String> {
    let path = dir.join(ITEM_FILE);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn view_at(dir: &Path, id: &str) -> Result<LibraryView, String> {
    let source = super::read_source(dir)?;
    let sheet = dir.join(CHECK_SHEET);
    Ok(LibraryView {
        id: id.to_string(),
        item: read_item(dir)?,
        template: source.template,
        variables: super::declarations(dir).unwrap_or_default(),
        folder: dir.display().to_string(),
        sheet: sheet
            .is_file()
            .then(|| sheet.to_string_lossy().into_owned()),
    })
}

pub fn view(library: &Path, id_or_name: &str) -> Result<LibraryView, String> {
    let dir = find(library, id_or_name)?;
    let id = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    view_at(&dir, &id)
}

/// Every item, the latest saved or updated first.
pub fn list(library: &Path) -> Vec<LibraryView> {
    let mut items: Vec<LibraryView> = std::fs::read_dir(library)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            check_id(&id).ok()?;
            let dir = e.path();
            if !dir.join(super::ENTRY).is_file() {
                return None;
            }
            view_at(&dir, &id).ok()
        })
        .collect();
    items.sort_by(|a, b| {
        b.item
            .updated_at
            .cmp(&a.item.updated_at)
            .then(a.item.label.cmp(&b.item.label))
    });
    items
}

/// A short name from a label: "Quote card!" → `quote-card`.
pub fn slug(label: &str) -> String {
    let mut out = String::new();
    for c in label.trim().chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out: String = out
        .trim_end_matches('-')
        .chars()
        .take(MAX_NAME_CHARS)
        .collect();
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "graphic".into()
    } else {
        out
    }
}

/// What a save says about the item.
#[derive(Clone, Debug, Default)]
pub struct SaveRequest {
    /// Its short name; default made from the label (or the updated item's own).
    pub name: Option<String>,
    pub label: String,
    pub use_when: String,
    pub length_us: u64,
    pub project: Option<String>,
    pub project_folder: Option<String>,
    pub general: bool,
    pub failed: Vec<String>,
    /// An item (id or name) this replaces as its next version.
    pub update: Option<String>,
    /// The gate's report and contact sheet, kept with the item.
    pub report: Option<super::gate::Report>,
}

/// Saves a copy of graphic folder `from` to the library as a new item, or as the next version
/// of `request.update`.
pub fn save(library: &Path, from: &Path, request: &SaveRequest) -> Result<LibraryView, String> {
    let label: String = request.label.trim().chars().take(MAX_LABEL_CHARS).collect();
    if label.is_empty() {
        return Err("Give the graphic a label".into());
    }
    let use_when: String = request
        .use_when
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_USE_WHEN_CHARS)
        .collect();
    let previous = match &request.update {
        Some(wanted) => {
            let id = resolve(library, wanted)?;
            let dir = find(library, &id)?;
            Some((id, read_item(&dir)?))
        }
        None => None,
    };
    let name = match (&request.name, &previous) {
        (Some(name), _) => {
            let name = name.trim();
            if slug(name) != name {
                return Err(format!(
                    "A name is lowercase letters and digits joined with -, like {}",
                    slug(name)
                ));
            }
            name.to_string()
        }
        (None, Some((_, item))) => item.name.clone(),
        (None, None) => slug(&label),
    };
    if super::templates::find(&name).is_ok() {
        return Err(format!(
            "\"{name}\" is a bundled template's name; give the graphic another `name`"
        ));
    }
    let existing = list(library);
    if let Some(other) = existing
        .iter()
        .find(|v| v.item.name == name && Some(&v.id) != previous.as_ref().map(|(id, _)| id))
    {
        return Err(format!(
            "The library already has \"{name}\" ({}, \"{}\"): update it, or give another `name`",
            other.id, other.item.label
        ));
    }
    if previous.is_none() && existing.len() >= MAX_ITEMS {
        return Err(format!(
            "The library holds at most {MAX_ITEMS} graphics; remove some first"
        ));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let (id, item) = match previous {
        Some((id, old)) => (
            id,
            LibraryItem {
                version: old.version + 1,
                created_at: old.created_at,
                ..item(request, name, label, use_when, &now)
            },
        ),
        None => (
            format!("lib-{}", &uuid::Uuid::new_v4().simple().to_string()[..12]),
            item(request, name, label, use_when, &now),
        ),
    };
    std::fs::create_dir_all(library).map_err(|e| format!("{}: {e}", library.display()))?;
    let staged = library.join(format!(".saving-{id}"));
    let _ = std::fs::remove_dir_all(&staged);
    let made = (|| {
        super::render::copy_dir(from, &staged)?;
        let _ = std::fs::remove_file(staged.join(ITEM_FILE));
        // Used in a project, it is named for its label.
        let mut source = super::read_source(&staged)?;
        source.name = item.label.clone();
        let text = serde_json::to_string_pretty(&source).map_err(|e| e.to_string())?;
        std::fs::write(staged.join(super::SOURCE_FILE), text + "\n").map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(&item).map_err(|e| e.to_string())?;
        std::fs::write(staged.join(ITEM_FILE), text + "\n").map_err(|e| e.to_string())?;
        if let Some(report) = &request.report {
            let text = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
            std::fs::write(staged.join(CHECK_REPORT), text).map_err(|e| e.to_string())?;
            if let Some(sheet) = &report.sheet {
                std::fs::copy(sheet, staged.join(CHECK_SHEET))
                    .map_err(|e| format!("{sheet}: {e}"))?;
            }
        }
        // Swap it in: an update replaces the old version whole.
        let target = library.join(&id);
        let old = library.join(format!(".old-{id}"));
        if target.exists() {
            let _ = std::fs::remove_dir_all(&old);
            std::fs::rename(&target, &old).map_err(|e| format!("{}: {e}", target.display()))?;
        }
        if let Err(e) = std::fs::rename(&staged, &target) {
            let _ = std::fs::rename(&old, &target);
            return Err(format!("{}: {e}", target.display()));
        }
        let _ = std::fs::remove_dir_all(&old);
        Ok(())
    })();
    if let Err(error) = made {
        let _ = std::fs::remove_dir_all(&staged);
        return Err(error);
    }
    view(library, &id)
}

fn item(
    request: &SaveRequest,
    name: String,
    label: String,
    use_when: String,
    now: &str,
) -> LibraryItem {
    LibraryItem {
        name,
        label,
        use_when,
        length_us: request
            .length_us
            .clamp(super::MIN_LENGTH_US, super::MAX_LENGTH_US),
        version: 1,
        project: request.project.clone(),
        project_folder: request.project_folder.clone(),
        created_at: now.to_string(),
        updated_at: now.to_string(),
        general: request.general,
        failed: if request.general {
            Vec::new()
        } else {
            request.failed.clone()
        },
    }
}

pub fn remove(library: &Path, id: &str) -> Result<(), String> {
    let dir = find(library, id)?;
    std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))
}

/// A library item or bundled template much like a graphic about to be saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseMatch {
    /// `library` or `template`.
    pub kind: String,
    /// The library item's id, or the template's name.
    pub id: String,
    pub label: String,
    /// The library item's version.
    pub version: Option<u32>,
    /// What they share.
    pub why: String,
}

/// The variables every graphic shares for its brand kit, left out of comparisons.
const KIT_VARIABLES: [&str; 3] = ["background", "text", "accent"];

fn words(label: &str) -> BTreeSet<String> {
    label
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| {
            !w.is_empty() && !["the", "a", "an", "of", "and", "for", "to", "with"].contains(w)
        })
        .map(String::from)
        .collect()
}

fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    a.intersection(b).count() as f64 / a.union(b).count() as f64
}

/// How alike two graphics are by label and variable ids (0 to 1), and why, when they are close.
fn likeness(
    label: &str,
    variables: &BTreeSet<String>,
    other_label: &str,
    other_variables: &BTreeSet<String>,
) -> Option<(f64, String)> {
    let by_label = if label.trim().eq_ignore_ascii_case(other_label.trim()) {
        1.0
    } else {
        jaccard(&words(label), &words(other_label))
    };
    let shared: Vec<&String> = variables.intersection(other_variables).collect();
    let by_variables = jaccard(variables, other_variables);
    let label_close = by_label >= 0.5;
    let variables_close = by_variables >= 0.75 && shared.len() >= 2;
    if !label_close && !variables_close {
        return None;
    }
    let list = shared
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Some(if by_label >= by_variables {
        (
            by_label,
            if by_label >= 1.0 {
                "the same label".into()
            } else {
                "a label much like it".into()
            },
        )
    } else {
        (
            by_variables,
            if by_variables >= 1.0 {
                format!("the same variables ({list})")
            } else {
                format!("mostly the same variables ({list})")
            },
        )
    })
}

/// The library item or bundled template closest to a graphic labelled `label` with variable
/// ids `variables`, if one is close; `exclude` (an item id) is left out.
pub fn closest(
    library: &Path,
    label: &str,
    variables: &[String],
    exclude: Option<&str>,
) -> Option<CloseMatch> {
    let ids = |list: &mut dyn Iterator<Item = String>| -> BTreeSet<String> {
        list.filter(|id| !KIT_VARIABLES.contains(&id.as_str()))
            .collect()
    };
    let mine = ids(&mut variables.iter().cloned());
    let mut best: Option<(f64, CloseMatch)> = None;
    let mut consider = |score: f64, found: CloseMatch| {
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, found));
        }
    };
    for item in list(library) {
        if Some(item.id.as_str()) == exclude {
            continue;
        }
        let theirs = ids(&mut item.variables.iter().map(|v| v.id.clone()));
        let same_name = slug(label) == item.item.name;
        let found = likeness(label, &mine, &item.item.label, &theirs)
            .or_else(|| same_name.then(|| (1.0, "the same name".to_string())));
        if let Some((score, why)) = found {
            consider(
                score,
                CloseMatch {
                    kind: "library".into(),
                    id: item.id.clone(),
                    label: item.item.label.clone(),
                    version: Some(item.item.version),
                    why,
                },
            );
        }
    }
    for template in super::templates::views() {
        if template.name == "blank" {
            continue;
        }
        let theirs = ids(&mut template.variables.iter().map(|v| v.id.clone()));
        if let Some((score, why)) = likeness(label, &mine, &template.label, &theirs) {
            consider(
                score,
                CloseMatch {
                    kind: "template".into(),
                    id: template.name.clone(),
                    label: template.label.clone(),
                    version: None,
                    why,
                },
            );
        }
    }
    best.map(|(_, found)| found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(label: &str) -> SaveRequest {
        SaveRequest {
            label: label.into(),
            use_when: "A new section starts".into(),
            length_us: 3_000_000,
            project: Some("Lesson".into()),
            general: true,
            ..Default::default()
        }
    }

    #[test]
    fn a_graphic_goes_to_the_library_and_back_into_a_project_as_a_copy() {
        let project = tempfile::tempdir().unwrap();
        let library = tempfile::tempdir().unwrap();
        let variables = serde_json::json!({"title": "Saved"});
        let asset = super::super::create(
            project.path(),
            "title",
            Some("Opener"),
            variables.as_object().unwrap(),
        )
        .unwrap();
        let from = PathBuf::from(&asset.path);
        std::fs::write(from.join("logo.svg"), "<svg/>").unwrap();
        let saved = save(library.path(), &from, &request("Channel opener")).unwrap();
        assert_eq!(saved.item.label, "Channel opener");
        assert_eq!(saved.item.name, "channel-opener");
        assert_eq!(saved.item.length_us, 3_000_000);
        assert_eq!(saved.item.version, 1);
        assert!(saved.item.general);
        assert_eq!(saved.template.as_deref(), Some("title"));
        assert!(!saved.variables.is_empty());
        assert_eq!(list(library.path()).len(), 1);
        // Found by its short name too.
        assert_eq!(view(library.path(), "channel-opener").unwrap().id, saved.id);

        let other = tempfile::tempdir().unwrap();
        let used = super::super::create_from_folder(
            other.path(),
            &find(library.path(), &saved.id).unwrap(),
            None,
        )
        .unwrap();
        let dir = PathBuf::from(&used.path);
        assert!(dir.starts_with(dunce::canonicalize(other.path()).unwrap()));
        assert!(dir.join("logo.svg").is_file());
        assert!(
            !dir.join(ITEM_FILE).exists(),
            "library.json stays in the library"
        );
        let source = super::super::read_source(&dir).unwrap();
        assert_eq!(source.name, "Channel opener");
        assert_eq!(source.variables, *variables.as_object().unwrap());
        assert_ne!(used.id, asset.id);

        assert!(find(library.path(), "../x").is_err());
        remove(library.path(), &saved.id).unwrap();
        assert!(list(library.path()).is_empty());
    }

    #[test]
    fn an_update_is_the_next_version_of_the_same_item() {
        let project = tempfile::tempdir().unwrap();
        let library = tempfile::tempdir().unwrap();
        let asset =
            super::super::create(project.path(), "key-point", None, &Default::default()).unwrap();
        let from = PathBuf::from(&asset.path);
        let first = save(library.path(), &from, &request("Tip card")).unwrap();
        // The same name again is refused; an update is not.
        let again = save(library.path(), &from, &request("Tip card")).unwrap_err();
        assert!(again.contains("already has \"tip-card\""), "{again}");
        std::fs::write(from.join("extra.css"), "body{}").unwrap();
        let report = super::super::gate::Report {
            passed: true,
            checks: Vec::new(),
            source_hash: "x".into(),
            checked_at: "now".into(),
            seconds: 1.0,
            sheet: None,
        };
        let second = save(
            library.path(),
            &from,
            &SaveRequest {
                update: Some("tip-card".into()),
                use_when: "A rule worth remembering".into(),
                report: Some(report),
                ..request("Tip card v2")
            },
        )
        .unwrap();
        assert_eq!(second.id, first.id);
        assert_eq!(second.item.version, 2);
        assert_eq!(second.item.name, "tip-card", "an update keeps its name");
        assert_eq!(second.item.label, "Tip card v2");
        assert_eq!(second.item.created_at, first.item.created_at);
        assert!(PathBuf::from(&second.folder).join("extra.css").is_file());
        assert!(PathBuf::from(&second.folder).join(CHECK_REPORT).is_file());
        assert_eq!(list(library.path()).len(), 1);
        // No leftovers from the swap.
        let names: Vec<String> = std::fs::read_dir(library.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec![first.id.clone()]);
        // A bundled template's name is taken.
        let taken = save(
            library.path(),
            &from,
            &SaveRequest {
                name: Some("title".into()),
                ..request("Mine")
            },
        )
        .unwrap_err();
        assert!(taken.contains("bundled template"), "{taken}");
    }

    #[test]
    fn close_matches_are_found_by_label_and_variables() {
        let project = tempfile::tempdir().unwrap();
        let library = tempfile::tempdir().unwrap();
        let ids = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // The bundled section title has kicker, title and subtitle.
        let found = closest(
            library.path(),
            "Chapter opener",
            &ids(&["kicker", "title", "subtitle", "accent"]),
            None,
        )
        .unwrap();
        assert_eq!(
            (found.kind.as_str(), found.id.as_str()),
            ("template", "title")
        );
        assert!(found.why.contains("the same variables"), "{}", found.why);
        assert!(closest(
            library.path(),
            "Lower third",
            &ids(&["person", "role"]),
            None
        )
        .is_none());

        let asset =
            super::super::create(project.path(), "blank", None, &Default::default()).unwrap();
        let saved = save(
            library.path(),
            &PathBuf::from(&asset.path),
            &request("Quote card"),
        )
        .unwrap();
        let found = closest(library.path(), "Big quote card", &ids(&["quote"]), None).unwrap();
        assert_eq!(found.id, saved.id);
        assert_eq!(found.version, Some(1));
        assert!(closest(
            library.path(),
            "Big quote card",
            &ids(&["quote"]),
            Some(&saved.id)
        )
        .is_none());
    }

    #[test]
    fn slugs_are_short_lowercase_words() {
        assert_eq!(slug("Quote card!"), "quote-card");
        assert_eq!(slug("  Ünïcode & More  "), "n-code-more");
        assert_eq!(slug("!!!"), "graphic");
        assert!(slug(&"long ".repeat(30)).len() <= MAX_NAME_CHARS);
    }
}
