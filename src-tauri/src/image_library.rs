//! The image library (FCL-5/6): still pictures built into the app for graphics, one set per
//! look, since a video keeps one look (GFX-5). Broadsheet (engraved ink, ported from the
//! Explainer Video Automation repository on 7 Nov 2026, plus a gap set made for AI-news
//! stories) for faceless explainers; light, dark, editorial and neon (the same 60 everyday
//! subjects each, made 8 Nov) for the app's styles. Beside them, sourced reference pictures
//! (flags, maps, wordmarks, portraits, product pages) with their source and licence, shown in
//! every look.
//!
//! `assets/images/catalog.json` is the index: the looks (with the styles that wear each), the
//! topics and their groups (with the words a beat is matched on), and per picture its look,
//! what it depicts, where it goes and its licence. The pictures are WebP (SVG for flags, maps
//! and wordmarks), built into the app by `build.rs`. Using one copies it into a graphic's
//! folder as `images/<look>-<file>`, so the graphic keeps working on its own; `check` flags a
//! graphic carrying one whose licence asks for credit or isn't known, or one of another look.
use crate::sequence::Licence;
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

mod files {
    include!(concat!(env!("OUT_DIR"), "/image_files.rs"));
}

/// Where a graphic keeps library pictures, inside its folder.
pub const GRAPHIC_DIR: &str = "images";
/// Search answers at most this many pictures.
pub const MAX_FOUND: usize = 24;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub looks: Vec<Look>,
    pub topics: Vec<Topic>,
    pub images: Vec<Image>,
}

/// A look: one illustration language, and the styles that wear it.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Look {
    pub id: String,
    pub description: String,
    /// The ground colour its pictures are drawn for, and its accent (empty for sourced).
    pub ground: String,
    pub accent: String,
    /// The app's styles (by name) that take this look; others take the nearest by colour.
    pub styles: Vec<String>,
}

/// Real material from elsewhere: shown with every look.
pub const SOURCED: &str = "sourced";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topic {
    /// `look/topic` (Broadsheet), or the look itself (a style look).
    pub id: String,
    pub look: String,
    pub description: String,
    pub groups: Vec<Group>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub description: String,
    /// What a beat says when this group fits it.
    pub terms: Vec<String>,
    /// Real material from elsewhere (flags, maps, wordmarks, portraits, pages), not made for
    /// the library: each says its source and licence.
    pub sourced: bool,
    pub count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    /// `look/topic/group/name` (Broadsheet) or `look/group/name`.
    pub id: String,
    pub look: String,
    /// Its file in the library.
    pub file: String,
    pub depicts: String,
    /// Where and how it is used.
    pub placement: String,
    pub licence: Licence,
    /// What using it asks: the credit line, or what to check first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rights: Option<String>,
    /// Sourced pictures: who or what it shows, and what it is (flag, photo, wordmark, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Pixels; none for vector files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

impl Image {
    /// Its file's name, as a graphic keeps it: `<look>-<name>`, since looks share names.
    pub fn file_name(&self) -> String {
        format!(
            "{}-{}",
            self.look,
            self.file.rsplit('/').next().unwrap_or(&self.file)
        )
    }

    /// Its name inside its look (`laptop`).
    pub fn name(&self) -> &str {
        self.id.rsplit('/').next().unwrap_or(&self.id)
    }

    /// Its group's id.
    pub fn group_id(&self) -> &str {
        self.id.rsplit_once('/').map_or("", |(g, _)| g)
    }

    /// Usable in an export without asking anyone.
    pub fn free(&self) -> bool {
        self.licence.kind.needs_no_credit()
    }

    pub fn bytes(&self) -> &'static [u8] {
        files::FILES
            .iter()
            .find(|(path, _)| *path == self.file)
            .map(|(_, bytes)| *bytes)
            .unwrap_or_default()
    }

    /// Where a graphic's `index.html` finds it once used: `images/<file>`.
    pub fn src(&self) -> String {
        format!("{GRAPHIC_DIR}/{}", self.file_name())
    }
}

static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../assets/images/catalog.json"))
        .expect("assets/images/catalog.json")
});

pub fn catalog() -> &'static Catalog {
    &CATALOG
}

/// A picture by its id, its file name as a graphic keeps it (`light-laptop.webp`), or its bare
/// name when only one look has it.
pub fn find(id: &str) -> Result<&'static Image, String> {
    let id = id.trim();
    let images = &catalog().images;
    if let Some(found) = images.iter().find(|i| i.id == id || i.file_name() == id) {
        return Ok(found);
    }
    let bare = id.trim_end_matches(".webp").trim_end_matches(".svg");
    let named: Vec<&Image> = images.iter().filter(|i| i.name() == bare).collect();
    match named.as_slice() {
        [one] => Ok(one),
        [] => Err(format!("No picture {id} in the image library (op: list)")),
        many => Err(format!(
            "{bare} is in several looks; give its id: {}",
            many.iter()
                .map(|i| i.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The library picture a graphic's file of this name is, if it is one.
pub fn by_file_name(name: &str) -> Option<&'static Image> {
    catalog().images.iter().find(|i| i.file_name() == name)
}

/// A group (its id: `broadsheet/ai-assets/compute`, `light/devices`) and its pictures.
pub fn group(id: &str) -> Option<(&'static Topic, &'static Group, Vec<&'static Image>)> {
    let id = id.trim().trim_end_matches('/');
    let (topic, group) = catalog().topics.iter().find_map(|t| {
        t.groups
            .iter()
            .find(|g| format!("{}/{}", t.id, g.id) == id)
            .map(|g| (t, g))
    })?;
    let images = catalog()
        .images
        .iter()
        .filter(|i| i.group_id() == id)
        .collect();
    Some((topic, group, images))
}

pub fn look(id: &str) -> Option<&'static Look> {
    catalog()
        .looks
        .iter()
        .find(|l| l.id.eq_ignore_ascii_case(id.trim()))
}

/// The look a style's graphics take: the one naming the style, else the one whose ground and
/// accent are nearest its background and accent (a copy of a style keeps its look).
pub fn look_for(style: &crate::style::Style) -> &'static Look {
    let looks = || catalog().looks.iter().filter(|l| l.id != SOURCED);
    if let Some(named) = looks().find(|l| {
        l.styles
            .iter()
            .any(|s| s.eq_ignore_ascii_case(style.name.trim()))
    }) {
        return named;
    }
    let rgb = |hex: &str| -> [i32; 3] {
        let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
        [
            (v >> 16) as i32 & 255,
            (v >> 8) as i32 & 255,
            v as i32 & 255,
        ]
    };
    let distance = |a: &str, b: &str| -> i32 {
        rgb(a)
            .iter()
            .zip(rgb(b))
            .map(|(x, y)| (x - y) * (x - y))
            .sum()
    };
    let colors = &style.brand.colors;
    looks()
        .min_by_key(|l| {
            2 * distance(&l.ground, &colors.background) + distance(&l.accent, &colors.accent)
        })
        .expect("the catalog has looks")
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1)
        .map(stem)
        .collect()
}

/// "servers" and "server" match.
fn stem(word: &str) -> String {
    let w = word
        .strip_suffix("es")
        .filter(|w| w.ends_with('s') || w.ends_with('x'));
    w.or_else(|| word.strip_suffix('s').filter(|w| w.len() > 2))
        .unwrap_or(word)
        .to_string()
}

/// Little words a query carries that say nothing about a picture.
const STOP: &[&str] = &[
    "a", "an", "the", "of", "to", "and", "or", "with", "that", "from", "into", "for", "it", "on",
    "in", "by", "as", "is", "are", "this", "these", "show", "showing", "about", "some", "one",
    "two", "three", "each", "their", "which", "what", "where", "when", "who", "how", "very",
];

/// The pictures matching `query`, best first: each word found in a picture's name, what it
/// depicts or where it goes counts once, and the words its group is matched on count too (a
/// group's term found whole in the query counts double).
///
/// Only pictures of `looks` (and sourced ones) when given.
pub fn search(query: &str, looks: Option<&[&str]>) -> Vec<&'static Image> {
    let wanted: Vec<String> = words(query)
        .into_iter()
        .filter(|w| !STOP.contains(&w.as_str()))
        .collect();
    if wanted.is_empty() {
        return Vec::new();
    }
    let query_text = format!(" {} ", words(query).join(" "));
    let mut scored: Vec<(usize, usize, &Image)> = Vec::new();
    for (n, image) in catalog().images.iter().enumerate() {
        if looks.is_some_and(|l| image.look != SOURCED && !l.contains(&image.look.as_str())) {
            continue;
        }
        let group = group(image.group_id()).map(|(_, g, _)| g);
        let own = words(&format!(
            "{} {} {} {}",
            image.id.replace('-', " "),
            image.depicts,
            image.placement,
            image.entity.as_deref().unwrap_or_default()
        ));
        let mut shared = Vec::new();
        if let Some(group) = group {
            shared.extend(words(&group.description));
            shared.extend(group.terms.iter().flat_map(|t| words(t)));
        }
        // A word of the picture's own counts double: its group's words fit all its pictures.
        let mut score: usize = wanted
            .iter()
            .map(|w| {
                if own.contains(w) {
                    2
                } else {
                    usize::from(shared.contains(w))
                }
            })
            .sum();
        if score == 0 {
            continue;
        }
        for term in group.map(|g| g.terms.as_slice()).unwrap_or_default() {
            let term = format!(" {} ", words(term).join(" "));
            if term.trim().contains(' ') && query_text.contains(&term) {
                score += 2;
            }
        }
        scored.push((score, n, image));
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, _, i)| i).collect()
}

/// The library's folder of pictures written out to look at (`images/built-in`).
fn dir() -> PathBuf {
    crate::style::store::config()
        .join("images")
        .join("built-in")
}

/// The picture as a file on disk, written there the first time.
pub fn on_disk(image: &Image) -> Result<PathBuf, String> {
    let path = dir().join(&image.file);
    let bytes = image.bytes();
    if fs::metadata(&path).map(|m| m.len()).ok() != Some(bytes.len() as u64) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
        }
        fs::write(&path, bytes).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    }
    Ok(path)
}

/// Pictures in a row per five, each 270 px high over a checkerboard (so the see-through parts
/// show) with its name above, as a JPEG. A vector file shows as a dark square: read its SVG.
pub fn board(images: &[&Image]) -> Result<Vec<u8>, String> {
    const PER_ROW: usize = 5;
    let mut tiles: Vec<(String, Option<RgbaImage>)> = Vec::new();
    for image in images {
        let name = image.id.rsplit('/').next().unwrap_or_default().to_string();
        if image.file.ends_with(".svg") {
            tiles.push((format!("{name} (svg)"), None));
            continue;
        }
        let png = crate::media::ffmpeg::still_png(&on_disk(image)?, 270)?;
        let pixels = image::load_from_memory(&png)
            .map_err(|e| format!("Could not read {name}: {e}"))?
            .to_rgba8();
        tiles.push((name, Some(pixels)));
    }
    let rows: Vec<RgbaImage> = tiles
        .chunks(PER_ROW)
        .map(|row| {
            let row: Vec<(&str, Option<&RgbaImage>)> =
                row.iter().map(|(n, p)| (n.as_str(), p.as_ref())).collect();
            crate::graphics::gate::contact_sheet(&row)
        })
        .collect();
    let width = rows.iter().map(|r| r.width()).max().unwrap_or(1);
    let height = rows.iter().map(|r| r.height()).sum::<u32>().max(1);
    let mut sheet = RgbaImage::from_pixel(width, height, image::Rgba([24, 24, 27, 255]));
    let mut y = 0;
    for row in &rows {
        image::imageops::overlay(&mut sheet, row, 0, y as i64);
        y += row.height();
    }
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, 82)
        .encode(
            sheet.as_raw(),
            width as u16,
            height as u16,
            jpeg_encoder::ColorType::Rgba,
        )
        .map_err(|e| format!("Could not make the board: {e}"))?;
    Ok(jpeg)
}

/// The library pictures in a graphic's folder.
fn library_files_in(graphic_dir: &std::path::Path) -> Vec<&'static Image> {
    let Ok(entries) = fs::read_dir(graphic_dir.join(GRAPHIC_DIR)) else {
        return Vec::new();
    };
    let mut found: Vec<&Image> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| by_file_name(&e.file_name().to_string_lossy()))
        .collect();
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

/// The library pictures in a graphic's folder whose licence asks for credit or isn't known.
pub fn unfree_in(graphic_dir: &std::path::Path) -> Vec<&'static Image> {
    library_files_in(graphic_dir)
        .into_iter()
        .filter(|i| !i.free())
        .collect()
}

/// The library pictures in a graphic's folder drawn for another look than `look`.
pub fn other_looks_in(graphic_dir: &std::path::Path, look: &str) -> Vec<&'static Image> {
    library_files_in(graphic_dir)
        .into_iter()
        .filter(|i| i.look != SOURCED && i.look != look)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_picture_is_built_in_once_and_named_once() {
        let images = &catalog().images;
        assert!(images.len() >= 500, "{} pictures", images.len());
        for image in images {
            assert!(!image.bytes().is_empty(), "{} has no file", image.id);
            assert!(
                image.id.starts_with(&format!("{}/", image.look)),
                "{}",
                image.id
            );
            assert!(look(&image.look).is_some(), "{} has no look", image.id);
            assert_eq!(
                images
                    .iter()
                    .filter(|i| i.file_name() == image.file_name())
                    .count(),
                1,
                "{} shares its file name",
                image.id
            );
            assert!(
                group(image.group_id()).is_some(),
                "{} has no group",
                image.id
            );
        }
        assert_eq!(files::FILES.len(), images.len());
        for topic in &catalog().topics {
            for g in &topic.groups {
                let (_, _, found) = group(&format!("{}/{}", topic.id, g.id)).unwrap();
                assert_eq!(found.len(), g.count, "{}/{}", topic.id, g.id);
            }
        }
        // The style looks carry the same subjects, so any style can show any idea.
        let names = |look: &str| -> Vec<&str> {
            let mut n: Vec<&str> = images
                .iter()
                .filter(|i| i.look == look)
                .map(|i| i.name())
                .collect();
            n.sort();
            n
        };
        for look in ["dark", "editorial", "neon"] {
            assert_eq!(names(look), names("light"), "{look}");
        }
    }

    #[test]
    fn generated_pictures_are_free_and_sourced_ones_say_their_rights() {
        let rack = find("broadsheet/ai-assets/compute/server-rack-open").unwrap();
        assert!(rack.free());
        assert!(rack.bytes().starts_with(b"RIFF"));
        let portrait = find("sam-altman").unwrap();
        assert_eq!(portrait.id, "sourced/people/sam-altman");
        assert!(!portrait.free());
        assert!(portrait.rights.as_deref().unwrap().contains("CC BY-SA 4.0"));
        let wordmark = find("openai-wordmark").unwrap();
        assert!(!wordmark.free() && wordmark.file.ends_with(".svg"));
        let map = find("sourced/countries/germany-map").unwrap();
        assert!(map.free(), "Natural Earth is public domain");
        // A name several looks share needs its id; a graphic's file name finds it too.
        assert!(find("laptop").unwrap_err().contains("neon/devices/laptop"));
        assert_eq!(find("neon-laptop.webp").unwrap().id, "neon/devices/laptop");
    }

    #[test]
    fn search_finds_by_what_a_beat_says() {
        let found = search("the server rack overheats: cooling fans", None);
        assert!(found
            .iter()
            .take(3)
            .any(|i| i.id == "broadsheet/ai-assets/compute/server-cooling-fan"));
        assert!(search("receipts", None)
            .iter()
            .any(|i| i.id.ends_with("receipt-long")));
        let torn = search("a torn map", Some(&["broadsheet"]));
        assert!(torn[0]
            .id
            .starts_with("broadsheet/countries/fictional-maps/torn-map"));
        assert!(search("the of and", None).is_empty());
        // The gap set of 7 Nov: subjects AI-news stories need.
        let ipo = search("the IPO bell rings on Nasdaq", None);
        assert_eq!(ipo[0].id, "broadsheet/markets/stocks/ipo-bell");
        assert!(search("pleads guilty in court", None)
            .iter()
            .take(7)
            .all(|i| i.id.starts_with("broadsheet/law/courts/")));
        assert!(search("humanoid robot walking", None)[0]
            .id
            .starts_with("broadsheet/ai-assets/humanoid/humanoid-walking"));
    }

    #[test]
    fn a_look_shows_its_own_pictures_and_the_sourced_ones() {
        let neon = search("a laptop and a padlock", Some(&["neon"]));
        assert!(!neon.is_empty());
        assert!(neon.iter().all(|i| i.look == "neon"), "{neon:?}");
        assert!(neon.iter().take(2).any(|i| i.id == "neon/devices/laptop"));
        let flags = search("the flag of Germany", Some(&["light"]));
        assert_eq!(flags[0].id, "sourced/countries/germany-flag");
    }

    #[test]
    fn each_style_takes_its_look() {
        let named = |name: &str| crate::style::Style {
            name: name.into(),
            ..crate::style::Style::default()
        };
        assert_eq!(look_for(&named("Light")).id, "light");
        assert_eq!(look_for(&named("Word Sync")).id, "light");
        assert_eq!(look_for(&named("Fancy-Short")).id, "neon");
        let dark = crate::style::builtin()
            .into_iter()
            .find(|s| s.name == "Dark")
            .unwrap();
        assert_eq!(look_for(&dark).id, "dark");
        // A style no look names takes the nearest by colour.
        let mut copy = dark.clone();
        copy.name = "My dark copy".into();
        assert_eq!(look_for(&copy).id, "dark");
        let mut violet = named("Party");
        violet.brand.colors.background = "#1A0838".into();
        violet.brand.colors.accent = "#FF3399".into();
        assert_eq!(look_for(&violet).id, "neon");
    }

    #[test]
    fn every_cast_member_stands_on_one_baseline() {
        use image::GenericImageView;
        for image in &catalog().images {
            if image.width != Some(768) || image.height != Some(1024) {
                continue;
            }
            // Decoded by FFmpeg (the image crate here reads no WebP).
            if !crate::media::ffmpeg::available() {
                return;
            }
            let png = crate::media::ffmpeg::still_png(&on_disk(image).unwrap(), 1024).unwrap();
            let pixels = image::load_from_memory(&png).unwrap();
            let feet = (0..pixels.height())
                .rev()
                .find(|&y| (0..pixels.width()).any(|x| pixels.get_pixel(x, y).0[3] > 20))
                .unwrap();
            assert!(
                (946..=952).contains(&feet),
                "{} stands at y={feet}",
                image.id
            );
        }
    }

    #[test]
    fn a_graphic_carrying_a_portrait_or_another_look_is_found() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(GRAPHIC_DIR)).unwrap();
        for id in ["sam-altman", "server-rack-open", "light/devices/laptop"] {
            let image = find(id).unwrap();
            fs::write(dir.path().join(image.src()), image.bytes()).unwrap();
        }
        let unfree: Vec<&str> = unfree_in(dir.path())
            .iter()
            .map(|i| i.id.as_str())
            .collect();
        assert_eq!(unfree, ["sourced/people/sam-altman"]);
        let strangers: Vec<&str> = other_looks_in(dir.path(), "light")
            .iter()
            .map(|i| i.id.as_str())
            .collect();
        assert_eq!(strangers, ["broadsheet/ai-assets/compute/server-rack-open"]);
    }
}
