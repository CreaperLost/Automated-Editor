//! Saved looks: a video's settings (canvas, camera bubble, captions, zooms, the auto webcam
//! and the graphics brand) and a short's look, under a name. They are kept for the app, not
//! one project, so any project can take one on, in one undo step.
//!
//! What belongs to one project stays with it: the transcript captions read, the wallpaper
//! file, stretches where the window crop is off, a short's scenes and hand-framed clips.

use crate::captions::CaptionSettings;
use crate::project::revision::EditDocument;
use crate::project::EditLayout;
use crate::shorts::ShortLayout;
use crate::webcam_focus::WebcamFocusSettings;
use crate::zoom::ZoomSettings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MAX_TEMPLATES: usize = 100;
const FILE: &str = "templates.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub name: String,
    pub layout: EditLayout,
    pub captions: CaptionSettings,
    pub zoom_settings: ZoomSettings,
    pub webcam_focus: WebcamFocusSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graphics_brand: Option<String>,
    /// A short's look; none when it was saved without a short.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_look: Option<ShortLayout>,
}

/// The app's templates file (`AEROEDITS_TEMPLATES_DIR` overrides its folder).
pub fn dir() -> PathBuf {
    std::env::var_os("AEROEDITS_TEMPLATES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(crate::transcript::settings::config_dir)
}

/// The saved templates, in the order they were saved; none when the file is missing or bad.
pub fn list(dir: &Path) -> Vec<Template> {
    std::fs::read(dir.join(FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn store(dir: &Path, templates: &[Template]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(templates).map_err(|e| e.to_string())?;
    let temp = dir.join(format!("{FILE}.tmp"));
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, dir.join(FILE)).map_err(|e| e.to_string())
}

fn check_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) {
        return Err("A template needs a one-line name of at most 60 characters".into());
    }
    Ok(name)
}

/// The look of a short without what belongs to its own stretch.
fn bare_short_look(layout: &ShortLayout) -> ShortLayout {
    ShortLayout {
        scenes: Vec::new(),
        camera_clips: Default::default(),
        graphic_slots: Default::default(),
        ..layout.clone()
    }
}

/// `document`'s settings, and `short`'s look when one is named, as a template called `name`.
pub fn capture(
    document: &EditDocument,
    name: &str,
    short: Option<&str>,
) -> Result<Template, String> {
    let name = check_name(name)?;
    let short_look = match short {
        Some(id) => Some(bare_short_look(
            &document
                .shorts
                .iter()
                .find(|s| s.id == id)
                .ok_or("That short is gone")?
                .layout,
        )),
        None => None,
    };
    Ok(Template {
        id: String::new(),
        name: name.to_string(),
        layout: EditLayout {
            wallpaper_asset: None,
            window_crop_off: Vec::new(),
            ..document.layout.clone()
        },
        captions: CaptionSettings {
            track_id: None,
            ..document.captions.clone()
        },
        zoom_settings: document.zoom_settings.clone(),
        webcam_focus: document.webcam_focus.settings.clone(),
        graphics_brand: document.graphics_brand.clone(),
        short_look,
    })
}

/// Saves `template`, replacing one of the same name (it keeps that one's id and place).
pub fn save(dir: &Path, mut template: Template) -> Result<Vec<Template>, String> {
    let mut templates = list(dir);
    match templates
        .iter_mut()
        .find(|t| t.name.eq_ignore_ascii_case(&template.name))
    {
        Some(same) => {
            template.id = same.id.clone();
            *same = template;
        }
        None => {
            if templates.len() >= MAX_TEMPLATES {
                return Err(format!("Keep at most {MAX_TEMPLATES} templates"));
            }
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or_default();
            template.id = format!("template-{stamp:x}");
            while templates.iter().any(|t| t.id == template.id) {
                template.id.push('x');
            }
            templates.push(template);
        }
    }
    store(dir, &templates)?;
    Ok(templates)
}

pub fn remove(dir: &Path, id: &str) -> Result<Vec<Template>, String> {
    let mut templates = list(dir);
    let before = templates.len();
    templates.retain(|t| t.id != id);
    if templates.len() == before {
        return Err("That template is gone".into());
    }
    store(dir, &templates)?;
    Ok(templates)
}

pub fn find(dir: &Path, id: &str) -> Result<Template, String> {
    list(dir)
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| "That template is gone".into())
}

/// `current`'s layout with the template's look. The wallpaper file and the window crop's off
/// stretches stay the project's; a wallpaper look on a project without one keeps its own
/// background.
pub fn layout_onto(template: &EditLayout, current: &EditLayout) -> EditLayout {
    let mut layout = EditLayout {
        wallpaper_asset: current.wallpaper_asset.clone(),
        window_crop_off: current.window_crop_off.clone(),
        ..template.clone()
    };
    if layout.background_type == "wallpaper" && layout.wallpaper_asset.is_none() {
        layout.background_type = current.background_type.clone();
        layout.background_preset = current.background_preset.clone();
        layout.color_start = current.color_start.clone();
        layout.color_end = current.color_end.clone();
    }
    layout
}

/// The template's captions, reading the transcript the project's read.
pub fn captions_onto(template: &CaptionSettings, current: &CaptionSettings) -> CaptionSettings {
    CaptionSettings {
        track_id: current.track_id.clone(),
        ..template.clone()
    }
}

/// A short's layout with the template's look; its scenes and framing stay.
pub fn short_look_onto(template: &ShortLayout, current: &ShortLayout) -> ShortLayout {
    ShortLayout {
        scenes: current.scenes.clone(),
        camera_clips: current.camera_clips.clone(),
        graphic_slots: current.graphic_slots.clone(),
        ..template.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saving_by_the_same_name_replaces_and_removing_forgets() {
        let dir = tempfile::tempdir().unwrap();
        let mut document = EditDocument::default();
        document.layout.corner_radius_px = 4;
        let first = save(dir.path(), capture(&document, "Tutorial", None).unwrap()).unwrap();
        assert_eq!(first.len(), 1);
        document.layout.corner_radius_px = 9;
        let second = save(dir.path(), capture(&document, " tutorial ", None).unwrap()).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].id, first[0].id);
        assert_eq!(second[0].layout.corner_radius_px, 9);
        assert!(capture(&document, "  ", None).is_err());
        assert!(capture(&document, "x", Some("nope")).is_err());
        assert!(remove(dir.path(), &first[0].id).unwrap().is_empty());
        assert!(list(dir.path()).is_empty());
    }

    #[test]
    fn what_belongs_to_a_project_stays_with_it() {
        let template = EditLayout {
            background_type: "wallpaper".into(),
            corner_radius_px: 3,
            ..EditLayout::default()
        };
        let current = EditLayout {
            background_type: "gradient".into(),
            ..EditLayout::default()
        };
        let layout = layout_onto(&template, &current);
        assert_eq!(layout.background_type, "gradient");
        assert_eq!(layout.corner_radius_px, 3);
        let captions = captions_onto(
            &CaptionSettings::default(),
            &CaptionSettings {
                track_id: Some("rec.mic".into()),
                ..CaptionSettings::default()
            },
        );
        assert_eq!(captions.track_id.as_deref(), Some("rec.mic"));
    }
}
