//! Brand kits: colours, a font and a logo the graphics take, kept by the app (not a project)
//! in `<config>/AeroEdits/brands/<name>/brand.json` with the logo and font files beside it.
//!
//! A project has a default kit and each graphic may pick another (or none). At render time the
//! kit fills the composition's shared `background`, `text` and `accent` variables where the
//! graphic has not set them, and a `brand.css` block sets the font and exposes `--brand-*`
//! custom properties. The kit's files are part of the render's hash, so changing a kit renders
//! the graphics that use it again.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const KIT_FILE: &str = "brand.json";
/// A graphic that says this uses no kit, whatever the project's default.
pub const NONE: &str = "none";
/// What the editor sends for "the project's default" (never a kit's name).
pub const DEFAULT: &str = "default";
/// Where a render's copy of the kit's files goes inside the staged composition.
pub const STAGE_DIR: &str = "aeroedits-brand";
const MAX_KITS: usize = 100;
const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
const LOGO_EXTENSIONS: &[&str] = &["png", "svg", "jpg", "jpeg", "webp"];
const FONT_EXTENSIONS: &[&str] = &["woff2", "woff", "ttf", "otf"];

/// What `brand.json` holds. The kit's name is its folder's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandKit {
    #[serde(default, skip_serializing)]
    pub name: String,
    pub background: String,
    pub text: String,
    pub accent: String,
    /// A font family; empty keeps each composition's own.
    #[serde(default)]
    pub font: String,
    /// A font file in the kit's folder for `font`, when it is not installed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_file: Option<String>,
    /// A logo file in the kit's folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
}

impl BrandKit {
    /// The variables the kit fills, by the ids the bundled templates use.
    pub fn variables(&self) -> [(&'static str, &str); 3] {
        [
            ("background", self.background.as_str()),
            ("text", self.text.as_str()),
            ("accent", self.accent.as_str()),
        ]
    }
}

/// The app's brand kits folder (`AEROEDITS_BRANDS_DIR` overrides it).
pub fn dir() -> PathBuf {
    std::env::var_os("AEROEDITS_BRANDS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::transcript::settings::config_dir().join("brands"))
}

/// A kit name is also a folder name: letters, digits, spaces, `-` and `_`, 1 to 40 of them.
pub fn check_name(name: &str) -> Result<(), String> {
    let ok = !name.trim().is_empty()
        && name == name.trim()
        && name.chars().count() <= 40
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        && !name.eq_ignore_ascii_case(NONE)
        && !name.eq_ignore_ascii_case(DEFAULT);
    if ok {
        Ok(())
    } else {
        Err("Name a brand kit with letters, digits, spaces, - or _ (at most 40)".into())
    }
}

fn is_color(s: &str) -> bool {
    let hex = s.strip_prefix('#').unwrap_or("");
    matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// A file name in the kit's folder (no paths).
fn plain_file(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 120
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':'])
        && name != KIT_FILE
}

fn check(kit: &BrandKit) -> Result<(), String> {
    check_name(&kit.name)?;
    for (id, value) in kit.variables() {
        if !is_color(value) {
            return Err(format!("The kit's {id} colour must be like #0F172A"));
        }
    }
    if kit.font.chars().count() > 80 || kit.font.contains(['"', '\'', ';', '{', '}', '<', '>']) {
        return Err("Give the font as a family name, like Inter".into());
    }
    for file in kit.font_file.iter().chain(kit.logo.iter()) {
        if !plain_file(file) {
            return Err(format!("\"{file}\" is not a file in the kit"));
        }
    }
    Ok(())
}

pub fn load(brands: &Path, name: &str) -> Result<BrandKit, String> {
    check_name(name).map_err(|_| format!("There is no brand kit \"{name}\""))?;
    let path = brands.join(name).join(KIT_FILE);
    let text =
        std::fs::read_to_string(&path).map_err(|_| format!("There is no brand kit \"{name}\""))?;
    let mut kit: BrandKit =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    kit.name = name.to_string();
    check(&kit)?;
    Ok(kit)
}

pub fn list(brands: &Path) -> Vec<BrandKit> {
    let mut kits: Vec<BrandKit> = std::fs::read_dir(brands)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(KIT_FILE).is_file())
        .filter_map(|e| load(brands, &e.file_name().to_string_lossy()).ok())
        .collect();
    kits.sort_by_key(|k| k.name.to_lowercase());
    kits
}

/// What the editor and agents see of a kit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandKitView {
    pub name: String,
    pub background: String,
    pub text: String,
    pub accent: String,
    pub font: String,
    pub font_file: Option<String>,
    pub logo: Option<String>,
    /// The logo's full path, for a thumbnail.
    pub logo_path: Option<String>,
    pub folder: String,
}

pub fn view(brands: &Path, kit: &BrandKit) -> BrandKitView {
    let folder = brands.join(&kit.name);
    BrandKitView {
        name: kit.name.clone(),
        background: kit.background.clone(),
        text: kit.text.clone(),
        accent: kit.accent.clone(),
        font: kit.font.clone(),
        font_file: kit.font_file.clone(),
        logo: kit.logo.clone(),
        logo_path: kit
            .logo
            .as_ref()
            .map(|l| folder.join(l).display().to_string()),
        folder: folder.display().to_string(),
    }
}

/// A kit as the editor sends it to save. `logo` and `font_file` are either the kit's own file
/// (kept), a full path to copy in, or absent (removed). `previous` renames a kit.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandKitInput {
    pub name: String,
    #[serde(default)]
    pub previous: Option<String>,
    pub background: String,
    pub text: String,
    pub accent: String,
    #[serde(default)]
    pub font: String,
    #[serde(default)]
    pub font_file: Option<String>,
    #[serde(default)]
    pub logo: Option<String>,
}

/// Saves a kit (new, changed or renamed) and returns it.
pub fn save(brands: &Path, input: &BrandKitInput) -> Result<BrandKit, String> {
    let name = input.name.trim().to_string();
    check_name(&name)?;
    let previous = input.previous.as_deref().filter(|p| !p.is_empty());
    let renamed = previous.is_some_and(|p| p != name);
    let target = brands.join(&name);
    let exists = target.join(KIT_FILE).is_file();
    if exists && (previous.is_none() || renamed) {
        return Err(format!("There is already a brand kit \"{name}\""));
    }
    if previous.is_none() && list(brands).len() >= MAX_KITS {
        return Err(format!("Keep at most {MAX_KITS} brand kits"));
    }
    let source = match previous {
        Some(p) => {
            check_name(p)?;
            let dir = brands.join(p);
            if !dir.join(KIT_FILE).is_file() {
                return Err(format!("There is no brand kit \"{p}\""));
            }
            Some(dir)
        }
        None => None,
    };
    let mut kit = BrandKit {
        name: name.clone(),
        background: input.background.trim().to_uppercase(),
        text: input.text.trim().to_uppercase(),
        accent: input.accent.trim().to_uppercase(),
        font: input.font.trim().to_string(),
        font_file: None,
        logo: None,
    };
    check(&BrandKit {
        font_file: None,
        logo: None,
        ..kit.clone()
    })?;
    // Work in a new folder, then swap it in, so a failed save leaves the kit as it was.
    std::fs::create_dir_all(brands).map_err(|e| format!("{}: {e}", brands.display()))?;
    let stage = brands.join(format!(".saving-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&stage).map_err(|e| format!("{}: {e}", stage.display()))?;
    let made = (|| {
        kit.logo = take_file(
            input.logo.as_deref(),
            source.as_deref(),
            &stage,
            "logo",
            LOGO_EXTENSIONS,
        )?;
        kit.font_file = take_file(
            input.font_file.as_deref(),
            source.as_deref(),
            &stage,
            "font",
            FONT_EXTENSIONS,
        )?;
        let text = serde_json::to_string_pretty(&kit).map_err(|e| e.to_string())?;
        std::fs::write(stage.join(KIT_FILE), text + "\n").map_err(|e| e.to_string())
    })();
    if let Err(error) = made {
        let _ = std::fs::remove_dir_all(&stage);
        return Err(error);
    }
    if let Some(old) = &source {
        let _ = std::fs::remove_dir_all(old);
    }
    if target.exists() {
        let _ = std::fs::remove_dir_all(&target);
    }
    std::fs::rename(&stage, &target).map_err(|e| format!("{}: {e}", target.display()))?;
    super::render::sources_changed();
    Ok(kit)
}

/// The kit's file for `given`: its own file copied from `kit` (kept), a full path copied in as
/// `<stem>.<ext>`, or none.
fn take_file(
    given: Option<&str>,
    kit: Option<&Path>,
    stage: &Path,
    stem: &str,
    extensions: &[&str],
) -> Result<Option<String>, String> {
    let Some(given) = given.map(str::trim).filter(|g| !g.is_empty()) else {
        return Ok(None);
    };
    let from = if plain_file(given) && !Path::new(given).is_absolute() {
        kit.map(|k| k.join(given))
            .filter(|p| p.is_file())
            .ok_or_else(|| format!("The kit has no file \"{given}\""))?
    } else {
        PathBuf::from(given)
    };
    let ext = from
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| extensions.contains(&e.as_str()))
        .ok_or_else(|| format!("A {stem} is a {} file", extensions.join(", ")))?;
    let size = std::fs::metadata(&from)
        .map_err(|e| format!("{}: {e}", from.display()))?
        .len();
    if size > MAX_FILE_BYTES {
        return Err(format!("That {stem} file is too big"));
    }
    let name = format!("{stem}.{ext}");
    std::fs::copy(&from, stage.join(&name)).map_err(|e| format!("{}: {e}", from.display()))?;
    Ok(Some(name))
}

pub fn remove(brands: &Path, name: &str) -> Result<(), String> {
    check_name(name)?;
    let dir = brands.join(name);
    if !dir.join(KIT_FILE).is_file() {
        return Err(format!("There is no brand kit \"{name}\""));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    super::render::sources_changed();
    Ok(())
}

/// The kit a graphic uses: its own choice (`none` for no kit), else the project's default.
pub fn effective<'a>(own: Option<&'a str>, project: Option<&'a str>) -> Option<&'a str> {
    match own {
        Some(NONE) => None,
        Some(name) => Some(name),
        None => project,
    }
}

/// `variables` with the kit's colours where the composition declares them and the graphic
/// has not set them.
pub fn fill_variables(
    variables: &Map<String, Value>,
    declared: &[super::VariableDecl],
    kit: &BrandKit,
) -> Map<String, Value> {
    let mut out = variables.clone();
    for (id, value) in kit.variables() {
        if !out.contains_key(id) && declared.iter().any(|d| d.id == id) {
            out.insert(id.to_string(), Value::String(value.to_string()));
        }
    }
    out
}

/// The `<style>` a render adds to the composition for `kit`: the font (from the kit's file if it
/// has one) on the page, and `--brand-*` properties for compositions of one's own.
pub fn css(kit: &BrandKit) -> String {
    let mut css = String::new();
    if let (Some(file), false) = (&kit.font_file, kit.font.is_empty()) {
        css.push_str(&format!(
            "@font-face {{ font-family: \"{}\"; src: url(\"{STAGE_DIR}/{file}\"); }}\n",
            kit.font
        ));
    }
    css.push_str(&format!(
        ":root {{ --brand-background: {}; --brand-text: {}; --brand-accent: {};",
        kit.background, kit.text, kit.accent
    ));
    if !kit.font.is_empty() {
        css.push_str(&format!(" --brand-font: \"{}\";", kit.font));
    }
    if let Some(logo) = &kit.logo {
        css.push_str(&format!(" --brand-logo: url(\"{STAGE_DIR}/{logo}\");"));
    }
    css.push_str(" }\n");
    if !kit.font.is_empty() {
        css.push_str(&format!(
            "body {{ font-family: \"{}\", \"Segoe UI\", system-ui, -apple-system, sans-serif; }}\n",
            kit.font
        ));
    }
    css
}

/// `html` with `css` as the last style in its head, so it wins over the composition's own.
pub fn inject_css(html: &str, css: &str) -> String {
    let block = format!("<style id=\"aeroedits-brand\">\n{css}</style>\n");
    match html.find("</head>") {
        Some(at) => format!("{}{block}{}", &html[..at], &html[at..]),
        None => format!("{block}{html}"),
    }
}

/// Copies the kit's files into a render's staged composition and adds its style.
pub fn stage(brands: &Path, kit: &BrandKit, stage: &Path) -> Result<(), String> {
    let from = brands.join(&kit.name);
    let to = stage.join(STAGE_DIR);
    for file in kit.font_file.iter().chain(kit.logo.iter()) {
        std::fs::create_dir_all(&to).map_err(|e| e.to_string())?;
        std::fs::copy(from.join(file), to.join(file))
            .map_err(|e| format!("Brand kit \"{}\": {file}: {e}", kit.name))?;
    }
    let entry = stage.join(super::ENTRY);
    let html = std::fs::read_to_string(&entry).map_err(|e| format!("{}: {e}", entry.display()))?;
    std::fs::write(&entry, inject_css(&html, &css(kit))).map_err(|e| e.to_string())
}

/// The bytes that name a kit in a render's hash: its settings and its files.
pub fn fingerprint(brands: &Path, kit: &BrandKit) -> Vec<u8> {
    let mut out = serde_json::to_vec(kit).unwrap_or_default();
    for file in kit.font_file.iter().chain(kit.logo.iter()) {
        out.extend_from_slice(file.as_bytes());
        out.push(0);
        out.extend(std::fs::read(brands.join(&kit.name).join(file)).unwrap_or_default());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> BrandKitInput {
        BrandKitInput {
            name: name.into(),
            previous: None,
            background: "#101010".into(),
            text: "#fafafa".into(),
            accent: "#ff5500".into(),
            font: "Inter".into(),
            font_file: None,
            logo: None,
        }
    }

    #[test]
    fn kits_are_saved_listed_renamed_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let brands = dir.path();
        let logo = dir.path().join("mark.svg");
        std::fs::write(&logo, "<svg/>").unwrap();
        let saved = save(
            brands,
            &BrandKitInput {
                logo: Some(logo.display().to_string()),
                ..input("Acme")
            },
        )
        .unwrap();
        assert_eq!(saved.logo.as_deref(), Some("logo.svg"));
        assert_eq!(saved.accent, "#FF5500");
        assert!(brands.join("Acme").join("logo.svg").is_file());
        assert!(save(brands, &input("Acme"))
            .unwrap_err()
            .contains("already"));
        // Renamed, keeping its logo by the kit's own file name.
        let renamed = save(
            brands,
            &BrandKitInput {
                previous: Some("Acme".into()),
                logo: Some("logo.svg".into()),
                ..input("Acme Dark")
            },
        )
        .unwrap();
        assert_eq!(renamed.logo.as_deref(), Some("logo.svg"));
        assert!(!brands.join("Acme").exists());
        let listed = list(brands);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Acme Dark");
        // Changed in place, dropping the logo.
        let changed = save(
            brands,
            &BrandKitInput {
                previous: Some("Acme Dark".into()),
                ..input("Acme Dark")
            },
        )
        .unwrap();
        assert!(changed.logo.is_none());
        for bad in ["", "../x", "none", "a/b", "Default"] {
            assert!(save(brands, &input(bad)).is_err(), "{bad:?}");
        }
        assert!(save(
            brands,
            &BrandKitInput {
                accent: "orange".into(),
                ..input("B")
            }
        )
        .is_err());
        remove(brands, "Acme Dark").unwrap();
        assert!(list(brands).is_empty());
    }

    #[test]
    fn a_kit_fills_only_declared_variables_the_graphic_left_unset() {
        let declared: Vec<super::super::VariableDecl> = serde_json::from_value(serde_json::json!([
            {"id":"title","type":"string","default":""},
            {"id":"text","type":"color","default":"#fff"},
            {"id":"accent","type":"color","default":"#fc0"}
        ]))
        .unwrap();
        let kit = BrandKit {
            name: "K".into(),
            background: "#000000".into(),
            text: "#111111".into(),
            accent: "#222222".into(),
            font: String::new(),
            font_file: None,
            logo: None,
        };
        let own = serde_json::json!({"accent": "#ABCDEF"});
        let filled = fill_variables(own.as_object().unwrap(), &declared, &kit);
        assert_eq!(filled["accent"], "#ABCDEF", "the graphic's own wins");
        assert_eq!(filled["text"], "#111111");
        assert!(!filled.contains_key("background"), "not declared");
        assert_eq!(effective(Some(NONE), Some("K")), None);
        assert_eq!(effective(None, Some("K")), Some("K"));
        assert_eq!(effective(Some("J"), Some("K")), Some("J"));
    }

    #[test]
    fn the_style_goes_last_in_the_head() {
        let kit = BrandKit {
            name: "K".into(),
            background: "#000000".into(),
            text: "#111111".into(),
            accent: "#222222".into(),
            font: "Inter".into(),
            font_file: Some("font.woff2".into()),
            logo: Some("logo.png".into()),
        };
        let html = inject_css("<head><style>a{}</style></head><body>", &css(&kit));
        let style = html.find("aeroedits-brand").unwrap();
        assert!(style > html.find("a{}").unwrap() && style < html.find("</head>").unwrap());
        assert!(html.contains("url(\"aeroedits-brand/font.woff2\")"));
        assert!(html.contains("--brand-logo"));
        assert!(html.contains("font-family: \"Inter\""));
    }
}
