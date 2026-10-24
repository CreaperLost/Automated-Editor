//! What plays under a take short, put there together: footage from the video, B-roll from an
//! imported file, and graphics (from a template, a library item, or a copy of one of the video's,
//! made for 9:16). The Shorts tab places one at a time; an agent proposes several as one batch
//! the user accepts in the Agent panel (`proposals`), checked on a copy first.
use crate::graphics::{self, library, templates};
use crate::mcp::tools::clock;
use crate::project::revision::EditDocument;
use crate::take_short::{self, Footage};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Where a graphic under a take comes from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GraphicFrom {
    /// A bundled template.
    Template { template: String },
    /// A copy of a library item.
    Library { id: String },
    /// A copy of one of the video's graphics (its look and text), made for the short.
    Video { graphic: String },
}

/// One thing to put under a take.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Under {
    /// The video's footage, or an imported file's (`media`).
    Footage(Footage),
    Graphic {
        from: GraphicFrom,
        /// On the short's own timeline.
        at_us: u64,
        /// Default: the template's or item's own; a copy plays as long as on the video.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        length_us: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Values for its variables, by id.
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        variables: Map<String, Value>,
        /// How it shows in the short (whole frame, split beside the face, over the face), set
        /// as it is placed so it renders once, for that.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<crate::shorts::GraphicSlot>,
    },
}

/// What putting things under a take made.
pub struct Put {
    pub document: EditDocument,
    /// The graphics made: asset id and clip id, in the order given.
    pub graphics: Vec<(String, String)>,
    /// One line per item, for the user.
    pub lines: Vec<String>,
    /// The graphic folders made; remove them if the edit is not kept.
    pub folders: Vec<PathBuf>,
    /// Graphics placed here that play at the same time as another one, one line each.
    pub overlaps: Vec<String>,
}

impl Put {
    /// Removes the graphic folders made (the edit was not kept).
    pub fn discard(&self) {
        for dir in &self.folders {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// Puts `items` under take short `short_id` on a copy of `document`, in order. Graphics are
/// made in `into`'s graphics folder (the project's, or a scratch one for a check); the video's
/// graphics are read from `root`. On an error, the folders made so far are removed.
pub fn put(
    root: &Path,
    into: &Path,
    document: &EditDocument,
    short_id: &str,
    items: &[Under],
) -> Result<Put, String> {
    let mut done = Put {
        document: document.clone(),
        graphics: Vec::new(),
        lines: Vec::new(),
        folders: Vec::new(),
        overlaps: Vec::new(),
    };
    match put_all(root, into, &mut done, short_id, items) {
        Ok(()) => Ok(done),
        Err(error) => {
            done.discard();
            Err(error)
        }
    }
}

fn put_all(
    root: &Path,
    into: &Path,
    done: &mut Put,
    short_id: &str,
    items: &[Under],
) -> Result<(), String> {
    if items.is_empty() {
        return Err("Name something to put under the take".into());
    }
    let mut shorts = done.document.shorts.clone();
    let index = shorts
        .iter()
        .position(|s| s.id == short_id)
        .ok_or_else(|| format!("No short {short_id}"))?;
    for (n, item) in items.iter().enumerate() {
        let at = |error: String| format!("Item {}: {error}", n + 1);
        match item {
            Under::Footage(footage) => {
                take_short::place(&done.document, &mut shorts[index], footage).map_err(at)?;
                let what = match &footage.media {
                    Some(media) => format!(
                        "{} from {}",
                        asset_name(&done.document, media),
                        clock(footage.video_us)
                    ),
                    None => format!("the video from {}", clock(footage.video_us)),
                };
                done.lines.push(format!(
                    "{}-{}: {what}{}{}",
                    clock(footage.at_us),
                    clock(footage.to_us),
                    if footage.speed == 1.0 {
                        String::new()
                    } else {
                        format!(" at {}x", footage.speed)
                    },
                    if footage.sound { "" } else { ", muted" }
                ));
            }
            Under::Graphic {
                from,
                at_us,
                length_us,
                name,
                variables,
                mode,
            } => {
                let (asset, length, said) = make_graphic(
                    root,
                    into,
                    &done.document,
                    from,
                    *length_us,
                    name,
                    variables,
                )
                .map_err(at)?;
                done.folders.push(graphics::folder(into, &asset.id));
                done.document.assets.push(asset.clone());
                let clip = take_short::place_graphic(
                    &done.document,
                    &mut shorts[index],
                    &asset.id,
                    *at_us,
                    length,
                )
                .map_err(at)?;
                let mode = mode.unwrap_or_default();
                if mode != crate::shorts::GraphicSlot::Full {
                    shorts[index]
                        .layout
                        .graphic_slots
                        .insert(clip.clone(), mode);
                }
                done.lines.push(format!(
                    "{}-{}: graphic \"{}\" ({said}), {}, in {}",
                    clock(*at_us),
                    clock(at_us + length),
                    asset.name,
                    mode.describe(),
                    crate::shorts::describe_box(crate::shorts::graphic_box(
                        &done.document,
                        &shorts[index],
                        mode
                    ))
                ));
                done.graphics.push((asset.id, clip));
            }
        }
    }
    if !done.graphics.is_empty() && take_short::uncut(&shorts[index]) {
        done.lines.push(UNCUT.to_string());
    }
    done.overlaps = overlaps(&done.document, &shorts[index], &done.graphics);
    done.document.shorts = shorts;
    Ok(())
}

/// Said when graphics go on a take that hasn't been cut: they sit on words a cut may take away.
pub const UNCUT: &str = "Warning: the take hasn't been cut yet. Graphics on uncut words play long \
     and a cut later can take their words away: cut its pauses and fillers first, then place graphics on the kept parts.";

/// Where a graphic just placed (`placed`) plays at the same time as another graphic on the
/// short's own timeline: two over the whole frame hide each other.
fn overlaps(
    document: &EditDocument,
    short: &crate::shorts::Short,
    placed: &[(String, String)],
) -> Vec<String> {
    let Some(sequence) = short.edit.as_ref().map(|e| &e.sequence) else {
        return Vec::new();
    };
    let graphics: Vec<(&str, &str, u64, u64)> = sequence
        .clips()
        .filter(|(_, c)| document.asset(&c.asset).is_some_and(|a| a.is_graphic()))
        .map(|(_, c)| {
            (
                c.id.as_str(),
                c.asset.as_str(),
                c.start_us,
                c.start_us + c.duration_us,
            )
        })
        .collect();
    let new = |clip: &str| placed.iter().any(|(_, id)| id == clip);
    let mut out = Vec::new();
    for (i, a) in graphics.iter().enumerate() {
        for b in &graphics[i + 1..] {
            let (from, to) = (a.2.max(b.2), a.3.min(b.3));
            if to > from && (new(a.0) || new(b.0)) {
                out.push(format!(
                    "\"{}\" and \"{}\" play together at {}-{}",
                    asset_name(document, a.1),
                    asset_name(document, b.1),
                    clock(from),
                    clock(to)
                ));
            }
        }
    }
    out
}

fn asset_name(document: &EditDocument, id: &str) -> String {
    document
        .asset(id)
        .map_or_else(|| id.to_string(), |a| a.name.clone())
}

/// Makes graphic `from` in `into` with its name and values; returns it, how long it plays and
/// where it came from.
pub(crate) fn make_graphic(
    root: &Path,
    into: &Path,
    document: &EditDocument,
    from: &GraphicFrom,
    length_us: Option<u64>,
    name: &Option<String>,
    variables: &Map<String, Value>,
) -> Result<(crate::sequence::Asset, u64, String), String> {
    let (mut asset, length, said) = match from {
        GraphicFrom::Template { template } => {
            let asset = graphics::create(into, template, name.as_deref(), variables)?;
            let length = length_us.unwrap_or(templates::find(template)?.length_us);
            return Ok((asset, length, format!("template {template}")));
        }
        GraphicFrom::Library { id } => {
            let lib = library::dir();
            let item = library::view(&lib, id)?;
            let folder = library::find(&lib, id)?;
            (
                graphics::create_from_folder(into, &folder, None)?,
                length_us.unwrap_or(item.item.length_us),
                format!("library {}", item.item.name),
            )
        }
        GraphicFrom::Video { graphic } => {
            let source = document
                .asset(graphic)
                .filter(|a| a.is_graphic())
                .ok_or_else(|| format!("No graphic {graphic} in the video"))?;
            // As long as it plays on the video, by default.
            let played = document
                .sequence
                .clips()
                .find(|(_, c)| c.asset == source.id)
                .map(|(_, c)| c.duration_us);
            (
                graphics::create_from_folder(
                    into,
                    &graphics::folder(root, &source.id),
                    Some(&source.name),
                )?,
                length_us.or(played).unwrap_or(4_000_000),
                "a copy of the video's".to_string(),
            )
        }
    };
    // A copy's name and values are set on it after.
    let dir = graphics::folder(into, &asset.id);
    let set = (|| {
        let name = name.as_deref().map(str::trim).filter(|n| !n.is_empty());
        if variables.is_empty() && name.is_none() {
            return Ok(());
        }
        let mut source = graphics::read_source(&dir)?;
        if !variables.is_empty() {
            let declared = graphics::declarations(&dir)?;
            source.variables = graphics::merge_variables(&declared, &source.variables, variables)?;
        }
        if let Some(name) = name {
            source.name = name.chars().take(80).collect();
            asset.name = source.name.clone();
        }
        graphics::write_source(&dir, &source)
    })();
    if let Err(error) = set {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(error);
    }
    Ok((asset, length, said))
}

/// What a batch for take short `short_id` was checked against: the short and the video's
/// timeline (footage names times on it). Another change elsewhere leaves it as it was.
pub fn stamp(document: &EditDocument, short_id: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let short = document.shorts.iter().find(|s| s.id == short_id);
    serde_json::to_string(&short)
        .unwrap_or_default()
        .hash(&mut hasher);
    serde_json::to_string(&document.sequence)
        .unwrap_or_default()
        .hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Checks `items` on a copy of the edit, making graphics in a scratch folder that goes after;
/// returns what they would do, one line each.
pub fn check(
    root: &Path,
    document: &EditDocument,
    short_id: &str,
    items: &[Under],
) -> Result<Vec<String>, String> {
    let scratch = std::env::temp_dir().join(format!(
        "aeroedits-under-{}",
        crate::sequence::sources::new_asset_id(false)
    ));
    let checked = put(root, &scratch, document, short_id, items);
    let _ = std::fs::remove_dir_all(&scratch);
    let done = checked?;
    // One graphic at a time over the frame: an agent's batch fixes its times before the user sees it.
    if !done.overlaps.is_empty() {
        return Err(format!(
            "Graphics would overlap: {}. End one earlier or start the next later (`toS` / \
             `toWord`), or remove the one already there.",
            done.overlaps.join("; ")
        ));
    }
    Ok(done.lines)
}
