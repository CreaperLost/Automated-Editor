//! Motion graphics: HyperFrames compositions kept as editable source in the project and
//! rendered by the app for the canvas they play on.
//!
//! A graphic is an asset ([`AssetKind::Graphic`]) whose path is its folder,
//! `<project>/graphics/<asset id>/`: an `index.html` composition (and any files it uses) plus
//! `graphic.json` with its name, the template it came from and its variable values. The folder
//! is the source and is never written by a render. Renders are a cache (see [`render`]), one per
//! canvas size and length, so the same graphic plays full screen in a 16:9 video and in a 9:16
//! short. On the timeline a graphic is drawn over everything (captions too) and is never cut
//! into (see `sequence::edit`).
pub mod brands;
pub mod gate;
pub mod library;
pub mod render;
pub mod runtime;
pub mod templates;

use crate::sequence::{Asset, AssetKind, Role, Stream, StreamKind, IMAGE_MAX_US};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const GRAPHICS_DIR: &str = "graphics";
pub const SOURCE_FILE: &str = "graphic.json";
pub const ENTRY: &str = "index.html";
/// Rendered graphics, by asset: `<project>/assets/graphics/<asset id>/`.
pub const CACHE_DIR: &str = "assets/graphics";
const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_NAME_CHARS: usize = 80;

/// What `graphic.json` holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicSource {
    pub name: String,
    /// The template it was made from, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Values for the composition's declared variables; missing ones use its defaults.
    #[serde(default)]
    pub variables: Map<String, Value>,
    /// The brand kit it uses: absent for the project's default, `none` for no kit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
}

/// One variable a composition declares in `data-composition-variables`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableDecl {
    pub id: String,
    /// string, number, color, boolean or enum.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub default: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<EnumOption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumOption {
    pub value: Value,
    #[serde(default)]
    pub label: String,
}

pub fn folder(root: &Path, asset_id: &str) -> PathBuf {
    root.join(GRAPHICS_DIR).join(asset_id)
}

pub fn cache_folder(root: &Path, asset_id: &str) -> PathBuf {
    root.join(CACHE_DIR).join(asset_id)
}

pub fn read_source(dir: &Path) -> Result<GraphicSource, String> {
    let path = dir.join(SOURCE_FILE);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn write_source(dir: &Path, source: &GraphicSource) -> Result<(), String> {
    let text = serde_json::to_string_pretty(source).map_err(|e| e.to_string())?;
    let path = dir.join(SOURCE_FILE);
    std::fs::write(&path, text + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
    render::sources_changed();
    Ok(())
}

/// The variables `index.html` declares, in its order.
pub fn declarations(dir: &Path) -> Result<Vec<VariableDecl>, String> {
    let path = dir.join(ENTRY);
    let html = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_declarations(&html)
}

pub fn parse_declarations(html: &str) -> Result<Vec<VariableDecl>, String> {
    let Some(raw) = attribute(html, "data-composition-variables") else {
        return Ok(Vec::new());
    };
    serde_json::from_str(&raw).map_err(|e| format!("data-composition-variables: {e}"))
}

/// The value of the first `name="..."` (or `'...'`) attribute in `html`, entities decoded.
fn attribute(html: &str, name: &str) -> Option<String> {
    let (start, end) = attribute_span(html, name)?;
    Some(decode_entities(&html[start..end]))
}

/// Where the value of the first `name="..."` (or `'...'`) attribute in `html` lies, quotes
/// excluded.
fn attribute_span(html: &str, name: &str) -> Option<(usize, usize)> {
    let at = html.find(&format!("{name}="))? + name.len() + 1;
    let rest = &html[at..];
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let end = rest[1..].find(quote)? + 1;
    Some((at + 1, at + end))
}

fn decode_entities(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// `html` with the declared defaults of the variables in `values` set to those values, so a
/// staged copy shows them without a variables file (HyperFrames' `check` and `snapshot` take
/// none).
pub fn with_defaults(html: &str, values: &Map<String, Value>) -> Result<String, String> {
    let Some((start, end)) = attribute_span(html, "data-composition-variables") else {
        return Ok(html.to_string());
    };
    let quote = &html[start - 1..start];
    let mut declared: Vec<Value> = serde_json::from_str(&decode_entities(&html[start..end]))
        .map_err(|e| format!("data-composition-variables: {e}"))?;
    for decl in &mut declared {
        if let Some(value) = decl["id"].as_str().and_then(|id| values.get(id)) {
            decl["default"] = value.clone();
        }
    }
    let json = serde_json::to_string(&declared).map_err(|e| e.to_string())?;
    let escaped = json
        .replace('&', "&amp;")
        .replace(quote, if quote == "\"" { "&quot;" } else { "&#39;" });
    Ok(format!("{}{escaped}{}", &html[..start], &html[end..]))
}

/// Checks `patch` against the declarations: known ids, values of the declared type. `null`
/// puts a variable back to its default.
pub fn merge_variables(
    declared: &[VariableDecl],
    current: &Map<String, Value>,
    patch: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    let mut out = current.clone();
    for (id, value) in patch {
        let Some(decl) = declared.iter().find(|d| &d.id == id) else {
            let known: Vec<&str> = declared.iter().map(|d| d.id.as_str()).collect();
            return Err(format!(
                "This graphic has no variable \"{id}\" (it has: {})",
                known.join(", ")
            ));
        };
        if value.is_null() {
            out.remove(id);
            continue;
        }
        let fits = match decl.kind.as_str() {
            "number" => value.as_f64().is_some_and(|n| {
                n.is_finite()
                    && decl.min.is_none_or(|min| n >= min)
                    && decl.max.is_none_or(|max| n <= max)
            }),
            "boolean" => value.is_boolean(),
            "color" => value.as_str().is_some_and(is_color),
            "enum" => decl.options.iter().any(|o| &o.value == value),
            _ => value
                .as_str()
                .is_some_and(|s| s.chars().count() <= 2000 && !s.contains('\0')),
        };
        if !fits {
            return Err(format!(
                "\"{id}\" takes a {}{}",
                decl.kind,
                match decl.kind.as_str() {
                    "enum" => format!(
                        ": one of {}",
                        decl.options
                            .iter()
                            .map(|o| o.value.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    "color" => " like #FACC15".into(),
                    "number" => match (decl.min, decl.max) {
                        (Some(min), Some(max)) => format!(" from {min} to {max}"),
                        _ => String::new(),
                    },
                    _ => String::new(),
                }
            ));
        }
        out.insert(id.clone(), value.clone());
    }
    Ok(out)
}

fn is_color(s: &str) -> bool {
    let hex = s.strip_prefix('#').unwrap_or("");
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// What the editor shows about a graphic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicView {
    pub asset_id: String,
    pub name: String,
    pub template: Option<String>,
    pub folder: String,
    pub variables: Map<String, Value>,
    pub declarations: Vec<VariableDecl>,
    pub render: render::RenderState,
    /// Its own brand kit choice: absent for the project's default, `none` for no kit.
    pub brand: Option<String>,
}

pub fn view(root: &Path, asset: &Asset) -> Result<GraphicView, String> {
    let dir = folder(root, &asset.id);
    let source = read_source(&dir)?;
    Ok(GraphicView {
        asset_id: asset.id.clone(),
        name: source.name,
        template: source.template,
        folder: dir.display().to_string(),
        variables: source.variables,
        declarations: declarations(&dir).unwrap_or_default(),
        render: render::state_of(&asset.id),
        brand: source.brand,
    })
}

/// Checks a graphic's brand choice as the editor or an agent gives it: `default` (the
/// project's), `none`, or a kit in `brands`. Returns what `graphic.json` stores.
pub fn brand_choice(brands_dir: &Path, given: &str) -> Result<Option<String>, String> {
    match given.trim() {
        "" | brands::DEFAULT => Ok(None),
        brands::NONE => Ok(Some(brands::NONE.into())),
        name => {
            brands::load(brands_dir, name)?;
            Ok(Some(name.to_string()))
        }
    }
}

/// Makes a graphic in the project from template `template` (see [`templates::all`]): copies
/// its files into `graphics/<new id>/` and returns the asset for the edit document.
pub fn create(
    root: &Path,
    template: &str,
    name: Option<&str>,
    variables: &Map<String, Value>,
) -> Result<Asset, String> {
    let found = templates::find(template)?;
    let id = crate::sequence::sources::new_asset_id(false);
    let dir = folder(root, &id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let made = (|| {
        for (path, contents) in found.files {
            let target = dir.join(path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&target, contents).map_err(|e| format!("{}: {e}", target.display()))?;
        }
        let declared = parse_declarations(found.html())?;
        let variables = merge_variables(&declared, &Map::new(), variables)?;
        let name = name
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(|n| n.chars().take(MAX_NAME_CHARS).collect::<String>())
            .unwrap_or_else(|| found.label.to_string());
        write_source(
            &dir,
            &GraphicSource {
                name,
                template: Some(found.name.to_string()),
                variables,
                brand: None,
            },
        )
    })();
    if let Err(error) = made {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(error);
    }
    asset_for(&id, &dir)
}

/// Makes a graphic in the project as a copy of graphic folder `from` (a library item): the
/// project gets its own copy and never points outside itself.
pub fn create_from_folder(root: &Path, from: &Path, name: Option<&str>) -> Result<Asset, String> {
    let id = crate::sequence::sources::new_asset_id(false);
    let dir = folder(root, &id);
    let made = (|| {
        render::copy_dir(from, &dir)?;
        let _ = std::fs::remove_file(dir.join(library::ITEM_FILE));
        let mut source = read_source(&dir)?;
        if let Some(name) = name.map(str::trim).filter(|n| !n.is_empty()) {
            source.name = name.chars().take(MAX_NAME_CHARS).collect();
        }
        write_source(&dir, &source)
    })();
    if let Err(error) = made {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(error);
    }
    asset_for(&id, &dir)
}

/// The asset for graphic folder `dir` (already in the project's `graphics/`).
pub fn asset_for(id: &str, dir: &Path) -> Result<Asset, String> {
    let source = read_source(dir)?;
    let dir = dunce::canonicalize(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(Asset {
        id: id.to_string(),
        name: source.name,
        kind: AssetKind::Graphic,
        path: dir.to_string_lossy().into_owned(),
        streams: vec![Stream {
            id: "picture".into(),
            kind: StreamKind::Picture,
            role: Role::Overlay,
            name: "Picture".into(),
            audio_path: None,
            fps: None,
        }],
        duration_us: IMAGE_MAX_US,
        width: 1920,
        height: 1080,
        pauses: Vec::new(),
        missing: false,
    })
}

/// Shortest and longest a placed graphic plays.
pub const MIN_LENGTH_US: u64 = 500_000;
pub const MAX_LENGTH_US: u64 = 600_000_000;
/// The name AeroEdits gives the video tracks it puts graphics on.
pub const TRACK_NAME: &str = "Graphics";

/// `document` with graphic `asset` added and placed at `at_us` for `length_us`, as one change:
/// on a graphics track free there (a new one on top when none is). Placing never pushes the
/// video along. Returns the new document and the clip's id.
pub fn place(
    document: &crate::project::revision::EditDocument,
    asset: Asset,
    at_us: u64,
    length_us: u64,
) -> Result<(crate::project::revision::EditDocument, String), String> {
    use crate::sequence::edit::{apply_reserving, SequenceEdit};
    use crate::sequence::{SourceRange, TrackKind};
    let length = length_us.clamp(MIN_LENGTH_US, MAX_LENGTH_US);
    let at = at_us.min(document.duration_us());
    let mut next = document.clone();
    let asset_id = asset.id.clone();
    if next.asset(&asset_id).is_none() {
        next.assets.push(asset);
    }
    let taken: Vec<String> = next.track_ids_in_use().map(String::from).collect();
    let graphic = |id: &str| next.assets.iter().any(|a| a.id == id && a.is_graphic());
    // A track that holds only graphics (or is named for them) and is free for the whole span.
    let free = next
        .sequence
        .video_tracks()
        .rev()
        .find(|t| {
            !t.locked
                && (t.name == TRACK_NAME
                    || (!t.clips.is_empty() && t.clips.iter().all(|c| graphic(&c.asset))))
                && t.is_free(at, at + length)
        })
        .map(|t| t.id.clone());
    let mut sequence = next.sequence.clone();
    let track_id = match free {
        Some(id) => id,
        None => {
            let (added, _) = apply_reserving(
                &sequence,
                &next.assets,
                &SequenceEdit::AddTrack {
                    track_kind: TrackKind::Video,
                },
                taken.iter().map(String::as_str),
            )?;
            let id = added
                .video_tracks()
                .find(|t| sequence.track(&t.id).is_none())
                .map(|t| t.id.clone())
                .ok_or("Could not add a track for the graphic")?;
            let (named, _) = apply_reserving(
                &added,
                &next.assets,
                &SequenceEdit::SetTrack {
                    track_id: id.clone(),
                    name: TRACK_NAME.into(),
                    hidden: false,
                    muted: false,
                    locked: false,
                    role: None,
                },
                taken.iter().map(String::as_str),
            )?;
            sequence = named;
            id
        }
    };
    let before: Vec<String> = sequence.clips().map(|(_, c)| c.id.clone()).collect();
    let (placed, _) = apply_reserving(
        &sequence,
        &next.assets,
        &SequenceEdit::PlaceAsset {
            asset_id: asset_id.clone(),
            at_us: at,
            track_id: Some(track_id),
            streams: Vec::new(),
            range: Some(SourceRange {
                start_us: 0,
                end_us: length,
            }),
        },
        taken.iter().map(String::as_str),
    )?;
    if placed.duration_us() > sequence.duration_us().max(at + length) {
        return Err("Placing the graphic would have moved the video".into());
    }
    let clip_id = placed
        .clips()
        .find(|(_, c)| c.asset == asset_id && !before.contains(&c.id))
        .map(|(_, c)| c.id.clone())
        .ok_or("The graphic was not placed")?;
    next.sequence = placed;
    Ok((next, clip_id))
}

/// Writes `contents` to `relative` inside a graphic's folder, for agents without a shell.
/// Only plain relative paths inside the folder; `graphic.json` goes through the variables.
pub fn write_file(dir: &Path, relative: &str, contents: &[u8]) -> Result<(), String> {
    let path = Path::new(relative);
    let plain = !relative.is_empty()
        && relative.len() <= 200
        && !path.is_absolute()
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));
    if !plain {
        return Err(
            "Give a path inside the graphic's folder, like index.html or assets/logo.svg".into(),
        );
    }
    if relative.eq_ignore_ascii_case(SOURCE_FILE) {
        return Err("graphic.json is changed through the graphic's variables".into());
    }
    if contents.len() as u64 > MAX_SOURCE_BYTES {
        return Err("That file is too big for a graphic".into());
    }
    let target = dir.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&target, contents).map_err(|e| format!("{}: {e}", target.display()))?;
    render::sources_changed();
    Ok(())
}

/// How a graphic comes on and goes off. The app decides it from where the graphic plays and
/// sets it on the root as `data-entrance` when it renders; the composition reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Entrance {
    /// Shown whole from its first frame to its last: no fade or slide in or out. Every graphic
    /// that fills the frame, and every graphic over the screen.
    #[default]
    Cut,
    /// Its card fades (or slides) in and out: only a graphic laid over the full-screen camera,
    /// such as a lower third on a camera-only stretch.
    Ease,
}

impl Entrance {
    pub fn as_str(self) -> &'static str {
        match self {
            Entrance::Cut => "cut",
            Entrance::Ease => "ease",
        }
    }
}

/// On the root: a graphic that covers only part of the frame (a lower third, a callout), so it
/// may ease in and out over the full-screen camera. Without it a graphic fills the frame.
pub const OVERLAY_ATTR: &str = "data-overlay";

/// Whether the root composition element's opening tag has attribute `name`.
pub fn root_has(html: &str, name: &str) -> bool {
    let Some(at) = html.find("data-composition-id") else {
        return false;
    };
    let start = html[..at].rfind('<').unwrap_or(0);
    let end = html[at..].find('>').map_or(html.len(), |e| at + e);
    html[start..end]
        .split(|c: char| c.is_whitespace() || c == '<')
        .any(|word| word == name || word.starts_with(&format!("{name}=")))
}

/// Whether the composition says it covers only part of the frame ([`OVERLAY_ATTR`]).
pub fn is_overlay(html: &str) -> bool {
    root_has(html, OVERLAY_ATTR)
}

/// Where the camera fills the frame on `document`'s timeline, sorted and merged: a short's
/// camera scenes, or the video's webcam focus. Empty when the camera is off.
pub fn full_camera_ranges(document: &crate::project::revision::EditDocument) -> Vec<(u64, u64)> {
    if !document.layout.webcam_enabled {
        return Vec::new();
    }
    let ranges: Vec<(u64, u64)> = match &document.short_layout {
        Some(short) => crate::short_scenes::edited_ranges(&short.scenes, |media| {
            document
                .clock_asset(media)
                .map(|asset| document.asset_clock(asset))
        })
        .into_iter()
        .filter(|r| r.2 == crate::short_scenes::SceneMode::Camera)
        .map(|r| (r.0, r.1))
        .collect(),
        None => document
            .webcam_focus
            .edited_ranges(&document.focus_clock()),
    };
    let mut ranges: Vec<(u64, u64)> = ranges.into_iter().filter(|r| r.1 > r.0).collect();
    ranges.sort();
    let mut merged: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (a, b) in ranges {
        match merged.last_mut() {
            Some(last) if a <= last.1 + ENTRANCE_SLACK_US => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    merged
}

/// About a frame: a graphic this close to a camera stretch's ends is still over it.
const ENTRANCE_SLACK_US: u64 = 34_000;

/// How graphic clip `clip` comes on and goes off, given where the camera fills the frame
/// ([`full_camera_ranges`]). It eases only when it plays wholly over the full-screen camera and
/// does not fill the frame itself; `fills_frame` is asked only then.
pub fn entrance_for(
    full_camera: &[(u64, u64)],
    clip: &crate::sequence::Clip,
    fills_frame: impl FnOnce() -> bool,
) -> Entrance {
    let over_camera = full_camera.iter().any(|&(a, b)| {
        a <= clip.start_us + ENTRANCE_SLACK_US && clip.end_us() <= b + ENTRANCE_SLACK_US
    });
    if over_camera && !fills_frame() {
        Entrance::Ease
    } else {
        Entrance::Cut
    }
}

/// Sets the root composition's `data-*` attributes and the viewport to the canvas a render is
/// for, and how it comes on and goes off. The composition lays itself out in the viewport, so
/// one source fits any canvas.
pub fn size_composition(
    html: &str,
    width: u32,
    height: u32,
    seconds: f64,
    entrance: Entrance,
) -> String {
    let mut html = html.to_string();
    if let Some(at) = html.find("data-composition-id") {
        let start = html[..at].rfind('<').unwrap_or(0);
        let end = html[at..].find('>').map_or(html.len(), |e| at + e);
        let mut tag = html[start..end].to_string();
        tag = set_attribute(&tag, "data-width", &width.to_string());
        tag = set_attribute(&tag, "data-height", &height.to_string());
        tag = set_attribute(&tag, "data-duration", &format!("{seconds:.3}"));
        tag = set_attribute(&tag, "data-entrance", entrance.as_str());
        html.replace_range(start..end, &tag);
    }
    if let Some(at) = html.find("name=\"viewport\"") {
        let start = html[..at].rfind('<').unwrap_or(0);
        let end = html[at..].find('>').map_or(html.len(), |e| at + e);
        let tag = set_attribute(
            &html[start..end],
            "content",
            &format!("width={width}, height={height}"),
        );
        html.replace_range(start..end, &tag);
    }
    html
}

/// `tag` (an element's opening tag without its `>`) with attribute `name` set to `value`.
fn set_attribute(tag: &str, name: &str, value: &str) -> String {
    let needle = format!(" {name}=");
    // After a space or a line break (an indented attribute has spaces before it anyway).
    let found = tag
        .find(&needle)
        .or_else(|| tag.find(&format!("\n{name}=")));
    if let Some(at) = found {
        let value_at = at + needle.len();
        let quote = tag[value_at..].chars().next();
        if let Some(q @ ('"' | '\'')) = quote {
            if let Some(close) = tag[value_at + 1..].find(q) {
                let mut out = tag.to_string();
                out.replace_range(value_at + 1..value_at + 1 + close, value);
                return out;
            }
        }
    }
    let trimmed = tag.trim_end_matches('/').trim_end();
    format!("{trimmed} {name}=\"{value}\"")
}

/// A picture that stands in while a graphic renders: dark, with its name.
pub fn placeholder_frame(width: u32, height: u32, label: &str) -> crate::media::VideoFrame {
    use crate::media::{ColorInfo, PixelFormat, VideoFrame};
    // Small: the GPU scales it to the canvas.
    let scale = (960.0 / width.max(height).max(1) as f32).min(1.0);
    let w = ((width as f32 * scale).round() as u32).max(16);
    let h = ((height as f32 * scale).round() as u32).max(16);
    let mut bgra = vec![0u8; (w * h * 4) as usize];
    for px in bgra.chunks_exact_mut(4) {
        px.copy_from_slice(&[42, 23, 15, 235]);
    }
    let font = crate::captions::font();
    let size = (w.min(h) as f32 * 0.07).max(10.0);
    let glyphs: Vec<_> = label.chars().map(|c| font.rasterize(c, size)).collect();
    let text_w: f32 = glyphs.iter().map(|(m, _)| m.advance_width).sum();
    let mut pen_x = ((w as f32 - text_w) / 2.0).max(0.0);
    let baseline = h as f32 / 2.0 + size * 0.35;
    for (metrics, coverage) in glyphs {
        let gx = (pen_x + metrics.xmin as f32).round() as i64;
        let gy = (baseline - metrics.height as f32 - metrics.ymin as f32).round() as i64;
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let (x, y) = (gx + col as i64, gy + row as i64);
                if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
                    continue;
                }
                let a = coverage[row * metrics.width + col] as u32;
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                for c in 0..3 {
                    let under = bgra[i + c] as u32;
                    bgra[i + c] = ((under * (255 - a) + 230 * a) / 255) as u8;
                }
            }
        }
        pen_x += metrics.advance_width;
    }
    VideoFrame {
        pts_us: 0,
        width: w,
        height: h,
        stride: w * 4,
        format: PixelFormat::Bgra8888,
        color: ColorInfo::rec709_full(),
        data: bgra.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn declarations_are_read_from_the_html_attribute() {
        let html = r##"<html data-composition-variables='[{"id":"title","type":"string","label":"Title","default":"Hi"},{"id":"accent","type":"color","label":"Accent","default":"#FACC15"}]'>"##;
        let declared = parse_declarations(html).unwrap();
        assert_eq!(declared.len(), 2);
        assert_eq!(declared[1].kind, "color");
        assert_eq!(declared[0].default, json!("Hi"));
    }

    #[test]
    fn variables_are_checked_against_their_declarations() {
        let declared: Vec<VariableDecl> = serde_json::from_value(json!([
            {"id":"title","type":"string","label":"Title","default":""},
            {"id":"accent","type":"color","label":"Accent","default":"#fff"},
            {"id":"count","type":"number","label":"Count","default":1,"min":1,"max":9},
            {"id":"side","type":"enum","label":"Side","default":"left",
             "options":[{"value":"left","label":"Left"},{"value":"right","label":"Right"}]}
        ]))
        .unwrap();
        let patch = |v: Value| v.as_object().unwrap().clone();
        let merged = merge_variables(
            &declared,
            &Map::new(),
            &patch(json!({"title":"A","accent":"#FACC15","count":3,"side":"right"})),
        )
        .unwrap();
        assert_eq!(merged.len(), 4);
        let unknown =
            merge_variables(&declared, &merged, &patch(json!({"titel":"A"}))).unwrap_err();
        assert!(
            unknown.contains("titel") && unknown.contains("title"),
            "{unknown}"
        );
        assert!(merge_variables(&declared, &merged, &patch(json!({"accent":"yellow"}))).is_err());
        assert!(merge_variables(&declared, &merged, &patch(json!({"count":12}))).is_err());
        assert!(merge_variables(&declared, &merged, &patch(json!({"side":"up"}))).is_err());
        let reset = merge_variables(&declared, &merged, &patch(json!({"title":null}))).unwrap();
        assert!(!reset.contains_key("title"));
    }

    #[test]
    fn the_root_and_viewport_take_the_canvas_size() {
        let html = "<meta name=\"viewport\" content=\"width=1920, height=1080\" />\n<div\n  id=\"root\"\n  data-composition-id=\"graphic\"\n  data-width=\"1920\"\n  data-height=\"1080\"\n  data-duration=\"4\"\n>";
        let sized = size_composition(html, 1080, 1920, 6.5, Entrance::Cut);
        assert!(sized.contains("data-width=\"1080\""), "{sized}");
        assert!(sized.contains("data-height=\"1920\""));
        assert!(sized.contains("data-duration=\"6.500\""));
        assert!(sized.contains("data-entrance=\"cut\""));
        assert!(sized.contains("content=\"width=1080, height=1920\""));
        // A root without the attributes gets them.
        let bare = size_composition("<div data-composition-id=\"g\">", 640, 360, 2.0, Entrance::Ease);
        assert!(bare.contains("data-width=\"640\"") && bare.contains("data-duration=\"2.000\""));
        assert!(bare.contains("data-entrance=\"ease\""), "{bare}");
        // Rendering again replaces it rather than adding a second one.
        let again = size_composition(&sized, 1080, 1920, 6.5, Entrance::Ease);
        assert_eq!(again.matches("data-entrance").count(), 1, "{again}");
        assert!(again.contains("data-entrance=\"ease\""));
    }

    fn graphic_clip(start_us: u64, duration_us: u64) -> crate::sequence::Clip {
        crate::sequence::Clip {
            id: "g1".into(),
            asset: "graphic-1".into(),
            stream: "v".into(),
            start_us,
            in_us: 0,
            duration_us,
            link: None,
            fit: Default::default(),
        }
    }

    #[test]
    fn a_graphic_filling_the_frame_always_cuts() {
        let camera = [(0, 20_000_000)];
        // Over the screen, and over the full-screen camera: a full-screen card cuts either way.
        assert_eq!(
            entrance_for(&[], &graphic_clip(1_000_000, 4_000_000), || true),
            Entrance::Cut
        );
        assert_eq!(
            entrance_for(&camera, &graphic_clip(1_000_000, 4_000_000), || true),
            Entrance::Cut
        );
    }

    #[test]
    fn an_overlay_eases_only_over_the_full_screen_camera() {
        let camera = [(2_000_000, 10_000_000)];
        let overlay = || false;
        // Wholly over the camera stretch: it eases.
        assert_eq!(
            entrance_for(&camera, &graphic_clip(3_000_000, 4_000_000), overlay),
            Entrance::Ease
        );
        // A frame's slack at the stretch's ends still counts.
        assert_eq!(
            entrance_for(&camera, &graphic_clip(1_990_000, 8_020_000), overlay),
            Entrance::Ease
        );
        // Over the screen, or running past the camera stretch: it cuts.
        assert_eq!(
            entrance_for(&[], &graphic_clip(3_000_000, 4_000_000), overlay),
            Entrance::Cut
        );
        assert_eq!(
            entrance_for(&camera, &graphic_clip(8_000_000, 4_000_000), overlay),
            Entrance::Cut
        );
        assert_eq!(
            entrance_for(&camera, &graphic_clip(500_000, 4_000_000), overlay),
            Entrance::Cut
        );
    }

    #[test]
    fn the_overlay_mark_is_read_from_the_root() {
        assert!(is_overlay(
            "<div id=\"root\" data-composition-id=\"g\" data-overlay data-duration=\"3\">"
        ));
        assert!(is_overlay("<div\n  data-composition-id=\"g\"\n  data-overlay\n>"));
        assert!(!is_overlay(
            "<div data-composition-id=\"g\"></div><div data-overlay></div>"
        ));
        assert!(!is_overlay("<div data-composition-id=\"g\" data-overlay-ish>"));
        for template in templates::all() {
            assert!(!is_overlay(template.html()), "{} fills the frame", template.name);
        }
    }

    #[test]
    fn every_template_cuts_unless_told_to_ease() {
        // The bundled cards fill the frame, so their fades run only when the app asks for an
        // ease (a template in a short's slot over a camera scene), never by default.
        for template in templates::all() {
            let html = template.html();
            assert!(
                html.contains("root.dataset.entrance === \"ease\""),
                "{} does not read data-entrance",
                template.name
            );
        }
    }

    #[test]
    fn every_template_is_a_valid_graphic() {
        let root = tempfile::tempdir().unwrap();
        for template in templates::all() {
            let declared = parse_declarations(template.html()).unwrap();
            assert!(
                !declared.is_empty() || template.name == "blank",
                "{}",
                template.name
            );
            let asset = create(root.path(), template.name, None, &Map::new()).unwrap();
            assert_eq!(asset.kind, AssetKind::Graphic);
            let dir = PathBuf::from(&asset.path);
            assert!(dir.join(ENTRY).is_file());
            assert_eq!(
                read_source(&dir).unwrap().template.as_deref(),
                Some(template.name)
            );
            let html = std::fs::read_to_string(dir.join(ENTRY)).unwrap();
            assert!(html.contains("data-composition-id"), "{}", template.name);
            assert!(
                !html.contains("http://") && !html.contains("https://"),
                "{} reaches the network",
                template.name
            );
        }
    }

    #[test]
    fn written_files_stay_inside_the_graphic() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "assets/logo.svg", b"<svg/>").unwrap();
        assert!(dir.path().join("assets/logo.svg").is_file());
        for bad in ["../x.html", "/abs.html", "", "graphic.json", "a/../../b"] {
            assert!(write_file(dir.path(), bad, b"x").is_err(), "{bad}");
        }
    }

    #[test]
    fn the_placeholder_has_the_requested_shape() {
        let frame = placeholder_frame(1920, 1080, "Rendering");
        assert_eq!((frame.width, frame.height), (960, 540));
        let frame = placeholder_frame(480, 270, "Rendering");
        assert_eq!((frame.width, frame.height), (480, 270));
    }
}
