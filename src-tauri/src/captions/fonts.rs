//! The captions' typeface: the brand kit's font (its file, or that family installed on this
//! computer), else the bundled Inter Display, at the look's weight. Letters the font lacks (Greek
//! in a Latin-only font) come from Inter Display at the same weight, so every caption draws.
use crate::graphics::brands::BrandKit;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Inter Display (SIL Open Font License, `assets/fonts/LICENSE-Inter.txt`), by weight.
const BUNDLED: [(u16, &[u8]); 4] = [
    (
        500,
        include_bytes!("../../assets/fonts/InterDisplay-Medium.ttf"),
    ),
    (
        600,
        include_bytes!("../../assets/fonts/InterDisplay-SemiBold.ttf"),
    ),
    (
        700,
        include_bytes!("../../assets/fonts/InterDisplay-Bold.ttf"),
    ),
    (
        800,
        include_bytes!("../../assets/fonts/InterDisplay-ExtraBold.ttf"),
    ),
];
pub const BUNDLED_FAMILY: &str = "Inter Display";
pub const WEIGHT_RANGE: (u16, u16) = (100, 900);
/// A font file larger than this is not read.
const MAX_FONT_BYTES: u64 = 40 * 1024 * 1024;

fn weight_name(weight: u16) -> &'static str {
    match weight {
        0..=150 => "Thin",
        151..=250 => "Extra Light",
        251..=350 => "Light",
        351..=450 => "Regular",
        451..=550 => "Medium",
        551..=650 => "Semibold",
        651..=750 => "Bold",
        751..=850 => "Extra Bold",
        _ => "Black",
    }
}

/// The bundled weight nearest `weight`; halfway goes heavier, since captions need weight.
fn nearest_bundled(weight: u16) -> usize {
    BUNDLED
        .iter()
        .enumerate()
        .min_by_key(|(_, (w, _))| ((*w as i32 - weight as i32).abs() * 2, u16::MAX - *w))
        .map(|(i, _)| i)
        .unwrap_or(2)
}

/// Inter Display at the bundled weight nearest `weight`.
pub fn bundled(weight: u16) -> &'static fontdue::Font {
    static FONTS: [OnceLock<fontdue::Font>; 4] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    let i = nearest_bundled(weight);
    FONTS[i].get_or_init(|| {
        fontdue::Font::from_bytes(BUNDLED[i].1, fontdue::FontSettings::default())
            .expect("bundled caption font parses")
    })
}

/// The font captions draw with and the one that fills in letters it lacks.
#[derive(Clone)]
pub struct Typeface {
    /// A font of the kit's; `None` draws everything with the fallback.
    primary: Option<Arc<fontdue::Font>>,
    fallback: &'static fontdue::Font,
    /// Which font it is and why, for the user and agents.
    pub label: String,
}

impl Typeface {
    pub fn bundled(weight: u16) -> Self {
        let i = nearest_bundled(weight);
        Self {
            primary: None,
            fallback: bundled(weight),
            label: format!("{BUNDLED_FAMILY} {}", weight_name(BUNDLED[i].0)),
        }
    }

    /// The font that draws `ch`.
    pub fn font_for(&self, ch: char) -> &fontdue::Font {
        match &self.primary {
            Some(font) if ch.is_whitespace() || font.lookup_glyph_index(ch) != 0 => font,
            _ => self.fallback,
        }
    }

    /// The font whose line height and spacing a caption takes.
    pub fn main(&self) -> &fontdue::Font {
        self.primary.as_deref().unwrap_or(self.fallback)
    }

    /// Letters of `text` the kit's font lacks, drawn with the fallback instead.
    pub fn borrowed(&self, text: &str) -> Vec<char> {
        let Some(font) = &self.primary else {
            return Vec::new();
        };
        let mut out: Vec<char> = text
            .chars()
            .filter(|c| !c.is_whitespace() && font.lookup_glyph_index(*c) == 0)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// The typeface for kit `kit` (kept in `brands`) at `weight`.
pub fn resolve(kit: Option<&BrandKit>, brands: &Path, weight: u16) -> Typeface {
    let plain = Typeface::bundled(weight);
    let Some(kit) = kit.filter(|k| !k.font.trim().is_empty()) else {
        return plain;
    };
    let family = kit.font.trim();
    let with = |font: Arc<fontdue::Font>, how: String| Typeface {
        primary: Some(font),
        label: how,
        ..plain.clone()
    };
    if let Some(file) = &kit.font_file {
        let path = brands.join(&kit.name).join(file);
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let readable = matches!(extension.as_str(), "ttf" | "otf" | "woff" | "woff2");
        if readable {
            if let Some(font) = load(&path) {
                return with(font, format!("{family} (the {} kit's font file)", kit.name));
            }
        }
        if let Some(found) = installed(family, weight).and_then(|p| Some((load(&p)?, p))) {
            return with(found.0, installed_label(family, &found.1, &kit.name));
        }
        let why = if readable {
            format!("the {} kit's font file {file} is missing or damaged", kit.name)
        } else {
            format!(
                "the {} kit's font is a .{extension} file, which captions can't read: give the \
                 kit a .ttf, .otf, .woff or .woff2, or install {family}",
                kit.name
            )
        };
        return Typeface {
            label: format!("{} ({why})", plain.label),
            ..plain
        };
    }
    if normalized(family) == "inter" || normalized(family) == normalized(BUNDLED_FAMILY) {
        return Typeface {
            label: format!("{} (the {} kit's font)", plain.label, kit.name),
            ..plain
        };
    }
    match installed(family, weight).and_then(|p| Some((load(&p)?, p))) {
        Some((font, path)) => with(font, installed_label(family, &path, &kit.name)),
        None => Typeface {
            label: format!(
                "{} ({family}, the {} kit's font, is not installed on this computer)",
                plain.label, kit.name
            ),
            ..plain
        },
    }
}

fn installed_label(family: &str, path: &Path, kit: &str) -> String {
    let file = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    format!("{family} (the {kit} kit's font, installed: {file})")
}

fn normalized(family: &str) -> String {
    family
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// A font file, read once while it stays the same size and age.
fn load(path: &Path) -> Option<Arc<fontdue::Font>> {
    type Cache = HashMap<PathBuf, (u64, Option<std::time::SystemTime>, Arc<fontdue::Font>)>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FONT_BYTES {
        return None;
    }
    let stamp = (meta.len(), meta.modified().ok());
    let cache = CACHE.get_or_init(Default::default);
    if let Some((len, modified, font)) = cache.lock().ok()?.get(path) {
        if (*len, *modified) == stamp {
            return Some(font.clone());
        }
    }
    let bytes = unpacked(std::fs::read(path).ok()?)?;
    let font = Arc::new(fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()?);
    cache
        .lock()
        .ok()?
        .insert(path.to_path_buf(), (stamp.0, stamp.1, font.clone()));
    Some(font)
}

/// A web font (WOFF or WOFF2, known by its first bytes, whatever its name) as the TrueType or
/// OpenType font it packs; any other font as it is.
fn unpacked(bytes: Vec<u8>) -> Option<Vec<u8>> {
    match bytes.get(..4) {
        Some(b"wOFF") => wuff::decompress_woff1(&bytes).ok(),
        Some(b"wOF2") => wuff::decompress_woff2(&bytes).ok(),
        _ => Some(bytes),
    }
    .filter(|font| font.len() as u64 <= MAX_FONT_BYTES)
}

fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(windows) {
        if let Some(windir) = std::env::var_os("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        dirs.push("/System/Library/Fonts".into());
        dirs.push("/Library/Fonts".into());
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }
    } else {
        dirs.push("/usr/share/fonts/truetype".into());
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join(".local/share/fonts"));
        }
    }
    dirs
}

/// The installed upright, static file of `family` nearest `weight`. Only files whose name
/// starts with the family's (`Montserrat-Bold.ttf`, `segoeuib.ttf`) are opened, so a folder of
/// a thousand fonts is not read through.
fn installed(family: &str, weight: u16) -> Option<PathBuf> {
    let key = normalized(family);
    if key.is_empty() {
        return None;
    }
    let mut best: Option<(i32, PathBuf)> = None;
    for dir in font_dirs() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let lower = name.to_ascii_lowercase();
            if !(lower.ends_with(".ttf") || lower.ends_with(".otf"))
                || !normalized(name).starts_with(&key)
                || entry.metadata().map_or(true, |m| m.len() > MAX_FONT_BYTES)
            {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(face) = ttf_parser::Face::parse(&bytes, 0) else {
                continue;
            };
            if face.is_italic() || face.is_oblique() || face.is_variable() {
                continue;
            }
            let named = face.names().into_iter().any(|n| {
                matches!(
                    n.name_id,
                    ttf_parser::name_id::TYPOGRAPHIC_FAMILY | ttf_parser::name_id::FAMILY
                ) && n.to_string().is_some_and(|s| normalized(&s) == key)
            });
            if !named {
                continue;
            }
            let distance = (face.weight().to_number() as i32 - weight as i32).abs();
            if best.as_ref().is_none_or(|(d, _)| distance < *d) {
                best = Some((distance, path));
            }
        }
    }
    best.map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_weight_draws_greek_and_the_nearest_is_picked() {
        for (weight, _) in BUNDLED {
            let font = bundled(weight);
            for c in "αβγδεζηθικλμνξοπρσςτυφχψωάέήίόύώϊϋΐΰΑΩΆΈΉΊΌΎΏΪΫ;·".chars()
            {
                assert_ne!(font.lookup_glyph_index(c), 0, "no {c} at {weight}");
            }
        }
        assert_eq!(BUNDLED[nearest_bundled(650)].0, 700);
        assert_eq!(BUNDLED[nearest_bundled(760)].0, 800);
        assert_eq!(BUNDLED[nearest_bundled(600)].0, 600);
        assert_eq!(BUNDLED[nearest_bundled(300)].0, 500);
        assert_eq!(Typeface::bundled(650).label, "Inter Display Bold");
    }

    fn kit(font: &str, file: Option<&str>) -> BrandKit {
        BrandKit {
            name: "Mine".into(),
            background: "#FFFFFF".into(),
            text: "#000000".into(),
            accent: "#FF0000".into(),
            font: font.into(),
            font_file: file.map(Into::into),
            logo: None,
        }
    }

    /// `sfnt` packed as WOFF 1.0: each table zlib-compressed, in the order the font lists them.
    fn woff1(sfnt: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let u16_at = |at: usize| u16::from_be_bytes([sfnt[at], sfnt[at + 1]]);
        let u32_at = |at: usize| u32::from_be_bytes(sfnt[at..at + 4].try_into().unwrap());
        let count = u16_at(4) as usize;
        let mut directory = Vec::new();
        let mut data = Vec::new();
        let data_start = 44 + 20 * count;
        let mut sfnt_size = 12 + 16 * count as u32;
        for i in 0..count {
            let record = 12 + 16 * i;
            let (offset, length) = (u32_at(record + 8) as usize, u32_at(record + 12) as usize);
            let table = &sfnt[offset..offset + length];
            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
            z.write_all(table).unwrap();
            let packed = z.finish().unwrap();
            let stored = if packed.len() < table.len() { packed } else { table.to_vec() };
            directory.extend_from_slice(&sfnt[record..record + 4]);
            directory.extend_from_slice(&((data_start + data.len()) as u32).to_be_bytes());
            directory.extend_from_slice(&(stored.len() as u32).to_be_bytes());
            directory.extend_from_slice(&(length as u32).to_be_bytes());
            directory.extend_from_slice(&sfnt[record + 4..record + 8]);
            data.extend_from_slice(&stored);
            data.resize(data.len().next_multiple_of(4), 0);
            sfnt_size += (length as u32).next_multiple_of(4);
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"wOFF");
        out.extend_from_slice(&sfnt[0..4]);
        out.extend_from_slice(&((data_start + data.len()) as u32).to_be_bytes());
        out.extend_from_slice(&(count as u16).to_be_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&sfnt_size.to_be_bytes());
        out.extend_from_slice(&[0, 1, 0, 0]);
        out.extend_from_slice(&[0; 20]);
        out.extend_from_slice(&directory);
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn a_kit_file_is_used_and_lends_nothing_it_has() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Mine")).unwrap();
        std::fs::write(dir.path().join("Mine/brand.ttf"), BUNDLED[0].1).unwrap();
        let face = resolve(Some(&kit("Brand Sans", Some("brand.ttf"))), dir.path(), 700);
        assert!(face
            .label
            .starts_with("Brand Sans (the Mine kit's font file)"));
        assert!(face.borrowed("Γειά σου").is_empty());
        // A web font is unpacked, whatever it is named.
        std::fs::write(dir.path().join("Mine/brand.woff"), woff1(BUNDLED[0].1)).unwrap();
        let face = resolve(Some(&kit("Brand Sans", Some("brand.woff"))), dir.path(), 700);
        assert!(face.label.starts_with("Brand Sans (the Mine kit's font file)"));
        assert_ne!(face.main().lookup_glyph_index('Γ'), 0);
        // A file it can't read: the bundled one, saying why.
        std::fs::write(dir.path().join("Mine/broken.woff2"), b"wOF2 not really").unwrap();
        let face = resolve(
            Some(&kit("Nowhere Font Xyz", Some("broken.woff2"))),
            dir.path(),
            700,
        );
        assert!(face.label.contains("broken.woff2 is missing or damaged"), "{}", face.label);
        let face = resolve(
            Some(&kit("Nowhere Font Xyz", Some("brand.eot"))),
            dir.path(),
            700,
        );
        assert!(face.label.contains("can't read"), "{}", face.label);
        // Not installed anywhere: the bundled one.
        let face = resolve(Some(&kit("Nowhere Font Xyz", None)), dir.path(), 700);
        assert!(face.label.contains("is not installed"), "{}", face.label);
        let face = resolve(Some(&kit("Inter", None)), dir.path(), 600);
        assert_eq!(face.label, "Inter Display Semibold (the Mine kit's font)");
    }
}
