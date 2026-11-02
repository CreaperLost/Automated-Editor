//! A style the user asked for from references in the Style panel (STY-3), waiting for their
//! agent: the app doesn't look at pictures itself. The request is kept in the app's settings
//! folder, so the agent's `status` names it in any session until it runs `library` `op: propose`
//! (or the user drops it), and the panel hands the user the same request as a prompt to paste.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const FILE: &str = "style-request.json";
/// As many as `library` `op: propose` takes.
pub const MAX_REFERENCES: usize = 8;
const MAX_BRIEF_CHARS: usize = 1_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleRequest {
    /// Image or clip files (full paths), or graphics library items.
    pub references: Vec<String>,
    /// What the user likes about them, in their words.
    pub brief: String,
    /// The name to save the style under, if the user gave one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

fn path(config: &Path) -> PathBuf {
    config.join(FILE)
}

pub fn load(config: &Path) -> Option<StyleRequest> {
    let text = std::fs::read_to_string(path(config)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Keeps `request` (replacing one waiting) once it is checked: 1 to 8 references, each a file
/// there or a library item, and a brief.
pub fn save(config: &Path, mut request: StyleRequest) -> Result<StyleRequest, String> {
    request.references.retain(|r| !r.trim().is_empty());
    request.brief = request.brief.trim().chars().take(MAX_BRIEF_CHARS).collect();
    request.name = request
        .name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    if request.references.is_empty() {
        return Err("Pick at least one picture or clip to make the style from".into());
    }
    if request.references.len() > MAX_REFERENCES {
        return Err(format!("Pick at most {MAX_REFERENCES} references"));
    }
    if request.brief.is_empty() {
        return Err("Say in a sentence what you like about them".into());
    }
    if let Some(name) = &request.name {
        crate::style::store::check_name(name)?;
    }
    let library = crate::graphics::library::dir();
    for reference in &request.references {
        let file = Path::new(reference);
        let known = (file.is_absolute() && file.is_file())
            || crate::graphics::library::view(&library, reference).is_ok();
        if !known {
            return Err(format!("{reference} is not a file or a library graphic"));
        }
    }
    std::fs::create_dir_all(config).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&request).map_err(|e| e.to_string())?;
    std::fs::write(path(config), text).map_err(|e| e.to_string())?;
    Ok(request)
}

pub fn clear(config: &Path) {
    let _ = std::fs::remove_file(path(config));
}

/// The request as the user would tell their agent.
pub fn prompt(request: &StyleRequest) -> String {
    let named = match &request.name {
        Some(name) => format!(" called \"{name}\""),
        None => String::new(),
    };
    format!(
        "In AeroEdits, make me a new style{named} from these references:\n{}\n\nWhat I like: \
         {}\n\nUse `library` with `kind: style`, `op: propose` and these references. Show me \
         stills to choose from, then one clip in place, before you save it.",
        request
            .references
            .iter()
            .map(|r| format!("- {r}"))
            .collect::<Vec<_>>()
            .join("\n"),
        request.brief
    )
}

/// The line `status` gives an agent while a request waits.
pub fn status_line(config: &Path) -> Option<String> {
    let request = load(config)?;
    Some(format!(
        "The user asked in the Style panel for a new style{} from {} reference(s): {}. What \
         they like: \"{}\". Do it first (`library` `kind: style` `op: propose` with these \
         `references` and the `brief`), unless they say otherwise now.",
        request
            .name
            .as_deref()
            .map(|n| format!(" called \"{n}\""))
            .unwrap_or_default(),
        request.references.len(),
        request.references.join(", "),
        request.brief
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_is_checked_kept_and_told_to_the_agent_until_cleared() {
        let config = tempfile::tempdir().unwrap();
        let picture = config.path().join("ref.png");
        std::fs::write(&picture, b"png").unwrap();
        let picture = picture.to_string_lossy().into_owned();
        let ask = |references: Vec<String>, brief: &str| StyleRequest {
            references,
            brief: brief.into(),
            name: Some("  Bold shorts ".into()),
        };

        assert!(save(config.path(), ask(vec![], "big captions")).is_err());
        assert!(save(config.path(), ask(vec![picture.clone()], "  ")).is_err());
        let missing = save(config.path(), ask(vec!["C:/nowhere/x.png".into()], "big")).unwrap_err();
        assert!(missing.contains("not a file"), "{missing}");
        assert_eq!(status_line(config.path()), None);

        let kept = save(config.path(), ask(vec![picture.clone()], " big captions ")).unwrap();
        assert_eq!(kept.name.as_deref(), Some("Bold shorts"));
        assert_eq!(load(config.path()), Some(kept.clone()));
        let line = status_line(config.path()).unwrap();
        assert!(
            line.contains("called \"Bold shorts\"") && line.contains(&picture),
            "{line}"
        );
        let prompt = prompt(&kept);
        assert!(prompt.contains(&format!("- {picture}")) && prompt.contains("op: propose"));

        clear(config.path());
        assert_eq!(status_line(config.path()), None);
    }
}
