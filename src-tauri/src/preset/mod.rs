//! Presets: what the app and the agent do for one kind of video. A preset holds the edit's
//! behaviour settings (canvas, captions behaviour, zooms, sound, layouts and how they are
//! picked), the defaults a new short takes, the choices the agent's method asks about (cleanup
//! depth and passes, graphics, export), the style a new project wears, and the method itself:
//! the agent's steps (`method.md`) and the user's own rules (`rules.md`).
//!
//! Presets are kept by the app: built-in ones in code (Tutorial, Shorts, Clips) and the user's in
//! `<config>/AeroEdits/presets/<name>/` (`preset.json`, `method.md`, `rules.md`). A project names
//! one preset (and one for its shorts) and stores only what it changed from it ([`stored`]): a
//! value the project never changed follows the preset, also when the preset changes later.
pub mod settings;
pub mod store;

use crate::project::revision::EditDocument;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The preset a new project follows, and the one a missing preset falls back to.
pub const DEFAULT: &str = "Tutorial";
/// The preset a project's shorts follow unless it picks another.
pub const SHORTS: &str = "Shorts";

/// The parts of the edit document a preset sets, by their names in it. Everything else (media,
/// the edit, chapters, shorts, graphics...) is the project's own.
pub const KEYS: [&str; 7] = [
    "layout",
    "captions",
    "zoomSettings",
    "audio",
    "layouts",
    "cleanup",
    "export",
];

/// Values in those parts that are always the project's own: they are never saved into a
/// preset (they name this project's files, clips and stretches).
const OWN: [(&str, &str); 6] = [
    ("layout", "wallpaperAsset"),
    ("layout", "cameraClips"),
    ("layout", "windowCropOff"),
    ("captions", "trackId"),
    ("layouts", "sections"),
    ("audio", "tracks"),
];

/// What `preset.json` holds. Every part may be left out.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Preset {
    /// Its folder's name (or the built-in's).
    #[serde(skip)]
    pub name: String,
    /// One line on what kind of video it is for.
    pub description: String,
    /// The style a project made with it wears.
    pub style: String,
    /// Edit settings, by part of the edit document (`layout`, `captions`, `zoomSettings`,
    /// `audio`, `layouts`, `cleanup`, `export`), only the fields that differ from the app's
    /// defaults.
    #[schemars(with = "Map<String, Value>")]
    pub values: Map<String, Value>,
    /// What a new short of a project whose shorts follow this preset starts with (fields of a
    /// short's look: captions, captionSpot, captionMaxWords, screenZoom, followZooms...).
    #[schemars(with = "Map<String, Value>")]
    pub short: Map<String, Value>,
    /// The choices the method asks about (cleanup passes and depth, graphics, export), in the
    /// shape the method gives: the agent reads and saves them here.
    #[schemars(with = "Map<String, Value>")]
    pub choices: Map<String, Value>,
    /// The agent's steps for this kind of video (Markdown, `method.md`).
    #[serde(skip)]
    pub method: String,
    /// The user's own rules for this kind of video (Markdown, `rules.md`).
    #[serde(skip)]
    pub rules: String,
}

/// The choices the tutorial method starts from.
fn tutorial_choices() -> Map<String, Value> {
    serde_json::json!({
        "review": "fast",
        "mouthNoises": { "apply": true, "preset": "balanced" },
        "silence": { "apply": true, "preset": "balanced", "microphones": "all" },
        "gaps": { "apply": false, "preset": "balanced" },
        "fillers": { "apply": true },
        "webcam": { "keep": true, "focus": true },
        "zooms": { "apply": true, "settings": { "minHoldMs": 5000, "mergeGapMs": 5000, "clickScale": 1.6 } },
        "graphics": { "apply": false },
        "export": { "resolution": "1080p", "quality": "high" }
    })
    .as_object()
    .cloned()
    .unwrap_or_default()
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

/// The presets the app ships. They are read-only: changing one saves a copy.
pub fn builtin() -> Vec<Preset> {
    vec![
        Preset {
            name: DEFAULT.into(),
            description: "A screen-recorded tutorial, long form (16:9): clean speech, steady \
                pacing, the webcam and zooms where they help, no captions, chapters."
                .into(),
            style: crate::style::DEFAULT.into(),
            // A new project's sound is loudness-normalized (`AudioSettings::for_new_project`).
            values: object(serde_json::json!({ "audio": { "normalize": true } })),
            short: Map::new(),
            choices: tutorial_choices(),
            method: include_str!("methods/tutorial.md").replace("\r\n", "\n"),
            rules: String::new(),
        },
        Preset {
            name: SHORTS.into(),
            description: "A vertical short recorded in a new take (camera and microphone): the \
                long video's footage and graphics under each sentence, captions on."
                .into(),
            style: crate::style::DEFAULT.into(),
            values: object(serde_json::json!({
                "layout": { "aspectRatio": "9:16" },
                "captions": { "enabled": true, "maxWords": 3 },
                "audio": { "normalize": true }
            })),
            short: object(serde_json::json!({ "captions": true, "captionMaxWords": 3 })),
            choices: object(serde_json::json!({
                "graphics": { "apply": true },
                "export": { "resolution": "1080p", "quality": "high" }
            })),
            method: include_str!("methods/shorts.md").replace("\r\n", "\n"),
            rules: String::new(),
        },
        Preset {
            name: "Clips".into(),
            description: "Vertical clips cut from the edited video: the best moments of its \
                transcript in the split-screen look, captions on."
                .into(),
            style: crate::style::DEFAULT.into(),
            values: object(serde_json::json!({
                "layout": { "aspectRatio": "9:16" },
                "captions": { "enabled": true, "maxWords": 3 },
                "audio": { "normalize": true }
            })),
            short: object(serde_json::json!({ "captions": true })),
            choices: object(serde_json::json!({
                "export": { "resolution": "1080p", "quality": "high" }
            })),
            method: include_str!("methods/clips.md").replace("\r\n", "\n"),
            rules: String::new(),
        },
    ]
}

pub fn is_builtin(name: &str) -> bool {
    builtin()
        .iter()
        .any(|p| p.name.eq_ignore_ascii_case(name.trim()))
}

/// The app's defaults for each part a preset sets, every field written out.
fn app_defaults() -> Map<String, Value> {
    let document = EditDocument::default();
    let value = |v: serde_json::Result<Value>| v.unwrap_or_default();
    Map::from_iter([
        (
            "layout".into(),
            value(serde_json::to_value(&document.layout)),
        ),
        (
            "captions".into(),
            value(serde_json::to_value(&document.captions)),
        ),
        (
            "zoomSettings".into(),
            value(serde_json::to_value(&document.zoom_settings)),
        ),
        ("audio".into(), value(serde_json::to_value(&document.audio))),
        (
            "layouts".into(),
            value(serde_json::to_value(&document.layouts)),
        ),
        (
            "cleanup".into(),
            value(serde_json::to_value(&document.cleanup)),
        ),
        (
            "export".into(),
            value(serde_json::to_value(&document.export)),
        ),
    ])
}

/// JSON merge patch: objects merge, `null` removes a value (its default comes back), anything
/// else replaces.
pub fn merge(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                if value.is_null() {
                    target.remove(key);
                    continue;
                }
                match target.get_mut(key) {
                    Some(slot) if slot.is_object() && value.is_object() => merge(slot, value),
                    _ => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (target, patch) => *target = patch.clone(),
    }
}

/// What `now` changed from `base`, as a merge patch (`null` for a value `now` left out).
pub fn diff(now: &Value, base: &Value) -> Option<Value> {
    match (now, base) {
        (Value::Object(now), Value::Object(base)) => {
            let mut out = Map::new();
            for (key, value) in now {
                match base.get(key) {
                    Some(was) if was == value => {}
                    Some(was) if was.is_object() && value.is_object() => {
                        if let Some(changed) = diff(value, was) {
                            out.insert(key.clone(), changed);
                        }
                    }
                    _ => {
                        out.insert(key.clone(), value.clone());
                    }
                }
            }
            for key in base.keys() {
                if !now.contains_key(key) {
                    out.insert(key.clone(), Value::Null);
                }
            }
            (!out.is_empty()).then_some(Value::Object(out))
        }
        _ if now == base => None,
        _ => Some(now.clone()),
    }
}

impl Preset {
    /// What a project following this preset starts from, part by part.
    pub fn inherited(&self) -> Map<String, Value> {
        let mut out = app_defaults();
        for key in KEYS {
            if let (Some(base), Some(values)) = (out.get_mut(key), self.values.get(key)) {
                merge(base, values);
            }
        }
        out
    }

    /// Checks that its values make a valid edit.
    pub fn validate(&self) -> Result<(), String> {
        for key in self.values.keys() {
            if !KEYS.contains(&key.as_str()) {
                return Err(format!("A preset sets {} (not \"{key}\")", KEYS.join(", ")));
            }
        }
        let mut document =
            serde_json::to_value(EditDocument::default()).map_err(|e| e.to_string())?;
        for (key, value) in self.inherited() {
            document[key.as_str()] = value;
        }
        let document: EditDocument =
            serde_json::from_value(document).map_err(|e| format!("The preset's values: {e}"))?;
        document
            .validate()
            .map_err(|e| format!("The preset's values: {e}"))?;
        let mut short = serde_json::to_value(crate::shorts::ShortLayout::default())
            .map_err(|e| e.to_string())?;
        merge(&mut short, &Value::Object(self.short.clone()));
        serde_json::from_value::<crate::shorts::ShortLayout>(short)
            .map_err(|e| format!("The preset's short: {e}"))?
            .validate()
            .map_err(|e| format!("The preset's short: {e}"))?;
        if self.method.len() > 200_000 || self.rules.len() > 100_000 {
            return Err("A preset's method or rules are too long".into());
        }
        Ok(())
    }

    /// The look a new short starts with.
    pub fn short_layout(&self) -> crate::shorts::ShortLayout {
        let mut short =
            serde_json::to_value(crate::shorts::ShortLayout::default()).unwrap_or_default();
        merge(&mut short, &Value::Object(self.short.clone()));
        serde_json::from_value(short).unwrap_or_default()
    }
}

/// The preset `document` follows (Tutorial when it is missing).
pub fn of(document: &EditDocument) -> Preset {
    store::resolve(&document.preset)
}

/// What `document` changed from its preset, part by part (only the parts it changed).
pub fn changes(document: &EditDocument) -> Map<String, Value> {
    changes_from(document, &of(document))
}

pub fn changes_from(document: &EditDocument, preset: &Preset) -> Map<String, Value> {
    let Ok(value) = serde_json::to_value(document) else {
        return Map::new();
    };
    let inherited = preset.inherited();
    let mut out = Map::new();
    for key in KEYS {
        let now = value.get(key).cloned().unwrap_or(Value::Null);
        let base = inherited.get(key).cloned().unwrap_or(Value::Null);
        // A part the document leaves out is its default (both are written the same way: empty
        // lists and absent options left out).
        let now = if now.is_null() {
            app_defaults().remove(key).unwrap_or_default()
        } else {
            now
        };
        if let Some(changed) = diff(&now, &base) {
            out.insert(key.into(), changed);
        }
    }
    out
}

/// How many values `document` changed from its preset, by part, not counting what is always
/// the project's own.
pub fn changed_count(document: &EditDocument) -> usize {
    let mut changes = changes(document);
    strip_own(&mut changes);
    fn count(value: &Value) -> usize {
        match value {
            Value::Object(map) => map.values().map(count).sum(),
            _ => 1,
        }
    }
    changes.values().map(count).sum()
}

/// `changes` without what is always the project's own.
pub fn strip_own(changes: &mut Map<String, Value>) {
    for (part, field) in OWN {
        if let Some(Value::Object(map)) = changes.get_mut(part) {
            map.remove(field);
            if map.is_empty() {
                changes.remove(part);
            }
        }
    }
}

/// The JSON a document is stored as: the parts a preset sets as `presetChanges`, what differs
/// from the preset only.
pub fn stored(document: &EditDocument) -> Result<Value, String> {
    let mut value = serde_json::to_value(document).map_err(|e| e.to_string())?;
    let changes = changes(document);
    if let Value::Object(map) = &mut value {
        for key in KEYS {
            map.remove(key);
        }
        if !changes.is_empty() {
            map.insert("presetChanges".into(), Value::Object(changes));
        }
    }
    Ok(value)
}

/// The document stored as `value`: its preset's values with its changes over them.
pub fn read(mut value: Value) -> Result<EditDocument, String> {
    let preset = store::resolve(
        value
            .get("preset")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT),
    );
    let changes = value
        .as_object_mut()
        .and_then(|map| map.remove("presetChanges"))
        .unwrap_or(Value::Null);
    let mut inherited = preset.inherited();
    for key in KEYS {
        if let (Some(part), Some(patch)) = (inherited.get_mut(key), changes.get(key)) {
            merge(part, patch);
        }
    }
    if let Value::Object(map) = &mut value {
        for (key, part) in inherited {
            map.insert(key, part);
        }
    }
    serde_json::from_value(value).map_err(|e| format!("Invalid edit document: {e}"))
}

/// `document` following `preset` instead, keeping what it changed from its own preset.
pub fn switch(document: &EditDocument, preset: &str) -> Result<EditDocument, String> {
    let mut value = stored(document)?;
    value["preset"] = Value::String(preset.to_string());
    read(value)
}

/// `document` with every value it changed from its preset taken back (what is always its own
/// stays), or only `part`'s.
pub fn reset(document: &EditDocument, part: Option<&str>) -> Result<EditDocument, String> {
    let mut value = stored(document)?;
    if let Some(Value::Object(changes)) = value.get_mut("presetChanges") {
        let mut kept = Map::new();
        for (key, own) in OWN {
            if part.is_some_and(|p| p != key) {
                continue;
            }
            if let Some(v) = changes.get(key).and_then(|c| c.get(own)) {
                kept.entry(key.to_string())
                    .or_insert_with(|| Value::Object(Map::new()))[own] = v.clone();
            }
        }
        match part {
            Some(part) => {
                changes.remove(part);
                if let Some(v) = kept.remove(part) {
                    changes.insert(part.into(), v);
                }
            }
            None => *changes = kept,
        }
    }
    read(value)
}

/// The preset `document`'s changes would make: its preset's values with them, without what is
/// always the project's own (to save the project's settings as a preset).
pub fn captured(document: &EditDocument) -> Map<String, Value> {
    let preset = of(document);
    let mut values = preset.values.clone();
    let mut changes = changes(document);
    strip_own(&mut changes);
    for (key, patch) in changes {
        let slot = values
            .entry(key)
            .or_insert_with(|| Value::Object(Map::new()));
        merge(slot, &patch);
    }
    // Only what differs from the app's defaults.
    let defaults = app_defaults();
    let mut out = Map::new();
    for (key, value) in values {
        let mut full = defaults.get(&key).cloned().unwrap_or_default();
        merge(&mut full, &value);
        if let Some(changed) = diff(&full, &defaults[&key]) {
            out.insert(key, changed);
        }
    }
    out
}

/// What the editor and agents see of a preset.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetView {
    pub name: String,
    pub builtin: bool,
    pub preset: Preset,
    pub method: String,
    pub rules: String,
    pub folder: Option<String>,
}

pub fn view(preset: &Preset) -> PresetView {
    PresetView {
        name: preset.name.clone(),
        builtin: is_builtin(&preset.name),
        preset: preset.clone(),
        method: preset.method.clone(),
        rules: preset.rules.clone(),
        folder: store::folder_of(preset).map(|f| f.display().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtins_are_valid() {
        for preset in builtin() {
            preset.validate().unwrap();
            assert!(!preset.method.is_empty(), "{}", preset.name);
        }
        assert!(is_builtin("tutorial") && is_builtin("Clips") && !is_builtin("Gaming"));
    }

    #[test]
    fn a_document_stores_only_what_it_changed_and_follows_its_preset() {
        // A new project is as its preset: nothing changed here.
        let fresh = EditDocument {
            audio: crate::project::AudioSettings::for_new_project(),
            ..EditDocument::default()
        };
        assert_eq!(changed_count(&fresh), 0);
        let mut document = fresh.clone();
        let stored_plain = stored(&document).unwrap();
        assert!(
            stored_plain.get("presetChanges").is_none(),
            "{stored_plain}"
        );
        assert!(stored_plain.get("captions").is_none() && stored_plain.get("layout").is_none());
        assert_eq!(read(stored_plain).unwrap(), document);
        document.captions.font_size_pct = 7.0;
        document.layout.wallpaper_asset = Some("assets/wallpapers/a.png".into());
        let value = stored(&document).unwrap();
        assert_eq!(value["presetChanges"]["captions"]["fontSizePct"], 7.0);
        assert_eq!(read(value).unwrap(), document);
        assert_eq!(
            changed_count(&document),
            1,
            "the wallpaper is the project's own"
        );
        // Following Shorts: its values, and the change made here stays.
        let short = switch(&document, SHORTS).unwrap();
        assert_eq!(short.layout.aspect_ratio, "9:16");
        assert!(short.captions.enabled);
        assert_eq!(short.captions.font_size_pct, 7.0);
        // Reset: back to the preset's, the project's own kept.
        let back = reset(&short, None).unwrap();
        assert_eq!(back.captions.font_size_pct, 5.5);
        assert_eq!(
            back.layout.wallpaper_asset.as_deref(),
            Some("assets/wallpapers/a.png")
        );
        // A value set back to the app default while the preset sets another stays changed.
        let mut tall = switch(&EditDocument::default(), SHORTS).unwrap();
        tall.layout.aspect_ratio = "16:9".into();
        let value = stored(&tall).unwrap();
        assert_eq!(value["presetChanges"]["layout"]["aspectRatio"], "16:9");
        assert_eq!(read(value).unwrap().layout.aspect_ratio, "16:9");
    }

    #[test]
    fn captured_values_are_the_preset_with_the_changes() {
        let mut document = switch(&EditDocument::default(), SHORTS).unwrap();
        document.captions.max_words = 4;
        document.layout.camera_clips.insert(
            "c1".into(),
            crate::shorts::ClipFraming {
                offset_x: 0.1,
                offset_y: 0.0,
            },
        );
        let values = captured(&document);
        assert_eq!(values["captions"]["maxWords"], 4);
        assert_eq!(values["layout"]["aspectRatio"], "9:16");
        assert!(values["layout"].get("cameraClips").is_none());
    }
}
