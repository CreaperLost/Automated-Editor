//! A style's brand as a graphic renders it: its colours fill the composition's `background`,
//! `text` and `accent` variables where the graphic has not set them, and a style block sets the
//! fonts and exposes `--brand-*` custom properties. The style's files are part of the render's
//! hash, so changing the style renders the graphics that use it again.
use super::Style;
use serde_json::{Map, Value};
use std::path::Path;

/// Where a render's copy of the style's files goes inside the staged composition.
pub const STAGE_DIR: &str = "aeroedits-brand";
const SYSTEM_FONTS: &str = "\"Segoe UI\", system-ui, -apple-system, sans-serif";

/// The variables the brand fills, by the ids the bundled templates use.
pub fn variables(style: &Style) -> [(&'static str, &str); 3] {
    let c = &style.brand.colors;
    [
        ("background", c.background.as_str()),
        ("text", c.text.as_str()),
        ("accent", c.accent.as_str()),
    ]
}

/// `variables` with the brand's colours where the composition declares them and the graphic
/// has not set them.
pub fn fill_variables(
    variables: &Map<String, Value>,
    declared: &[crate::graphics::VariableDecl],
    style: &Style,
) -> Map<String, Value> {
    let mut out = variables.clone();
    for (id, value) in self::variables(style) {
        if !out.contains_key(id) && declared.iter().any(|d| d.id == id) {
            out.insert(id.to_string(), Value::String(value.to_string()));
        }
    }
    out
}

/// The `<style>` a render adds for `style`. `own_colors` is whether the graphic set its own
/// text or background, when the brand's muted colour would not fit them (the look mixes one).
pub fn css(style: &Style, own_colors: bool) -> String {
    let fonts = &style.brand.fonts;
    let mut css = String::new();
    for (font, which) in [(&fonts.title, "title"), (&fonts.body, "body")] {
        if let (Some(file), false) = (&font.file, font.family.is_empty()) {
            css.push_str(&format!(
                "@font-face {{ font-family: \"{}\"; src: url(\"{STAGE_DIR}/{file}\"); }} /* {which} */\n",
                font.family
            ));
        }
    }
    let c = &style.brand.colors;
    css.push_str(&format!(
        ":root {{ --brand-background: {}; --brand-text: {}; --brand-accent: {}; --brand-muted: {};",
        c.background, c.text, c.accent, c.muted
    ));
    if !fonts.title.family.is_empty() {
        css.push_str(&format!(" --brand-font: \"{}\";", fonts.title.family));
    }
    let body = (!fonts.body.family.is_empty()).then_some(&fonts.body.family);
    if let Some(body) = body {
        css.push_str(&format!(" --brand-body-font: \"{body}\";"));
    }
    if let Some(logo) = &style.brand.logo {
        css.push_str(&format!(" --brand-logo: url(\"{STAGE_DIR}/{logo}\");"));
    }
    css.push_str(" }\n");
    if !own_colors {
        css.push_str(&format!(
            "[data-composition-id] {{ --look-muted: {}; }}\n",
            c.muted
        ));
    }
    if !fonts.title.family.is_empty() {
        css.push_str(&format!(
            "body {{ font-family: \"{}\", {SYSTEM_FONTS}; }}\n",
            fonts.title.family
        ));
    }
    // Body text in the body font, titles kept in the title font.
    if let Some(body) = body.filter(|b| **b != fonts.title.family) {
        css.push_str(&format!(
            "#root, body {{ font-family: \"{body}\", {SYSTEM_FONTS}; }}\n\
             #root .title, #root .count, #root h1, #root h2 {{ font-family: var(--look-font); }}\n"
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

fn files(style: &Style) -> impl Iterator<Item = &String> {
    let f = &style.brand.fonts;
    [&f.title.file, &f.body.file, &style.brand.logo]
        .into_iter()
        .flatten()
}

/// Copies the style's files into a render's staged composition and adds its style block.
pub fn stage(style: &Style, stage: &Path, own_colors: bool) -> Result<(), String> {
    if let Some(from) = super::store::folder_of(style) {
        let to = stage.join(STAGE_DIR);
        for file in files(style) {
            std::fs::create_dir_all(&to).map_err(|e| e.to_string())?;
            std::fs::copy(from.join(file), to.join(file))
                .map_err(|e| format!("Style \"{}\": {file}: {e}", style.name))?;
        }
    }
    let entry = stage.join(crate::graphics::ENTRY);
    let html = std::fs::read_to_string(&entry).map_err(|e| format!("{}: {e}", entry.display()))?;
    std::fs::write(&entry, inject_css(&html, &css(style, own_colors))).map_err(|e| e.to_string())
}

/// The bytes that name a style in a render's hash: its name, settings and files.
pub fn fingerprint(style: &Style) -> Vec<u8> {
    let mut out = style.name.as_bytes().to_vec();
    out.push(0);
    out.extend(serde_json::to_vec(style).unwrap_or_default());
    if let Some(folder) = super::store::folder_of(style) {
        for file in files(style) {
            out.extend_from_slice(file.as_bytes());
            out.push(0);
            out.extend(std::fs::read(folder.join(file)).unwrap_or_default());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_brand_fills_only_declared_variables_the_graphic_left_unset() {
        let declared: Vec<crate::graphics::VariableDecl> =
            serde_json::from_value(serde_json::json!([
                {"id":"title","type":"string","default":""},
                {"id":"text","type":"color","default":"#fff"},
                {"id":"accent","type":"color","default":"#fc0"}
            ]))
            .unwrap();
        let style = Style::default();
        let own = serde_json::json!({"accent": "#ABCDEF"});
        let filled = fill_variables(own.as_object().unwrap(), &declared, &style);
        assert_eq!(filled["accent"], "#ABCDEF", "the graphic's own wins");
        assert_eq!(filled["text"], style.brand.colors.text.as_str());
        assert!(!filled.contains_key("background"), "not declared");
    }

    #[test]
    fn the_block_sets_fonts_colours_and_goes_last_in_the_head() {
        let mut style = Style::default();
        style.brand.fonts.title.family = "Inter".into();
        style.brand.fonts.title.file = Some("title-font.woff2".into());
        style.brand.fonts.body.family = "Source Serif".into();
        style.brand.logo = Some("logo.png".into());
        let html = inject_css("<head><style>a{}</style></head><body>", &css(&style, false));
        let block = html.find("aeroedits-brand").unwrap();
        assert!(block > html.find("a{}").unwrap() && block < html.find("</head>").unwrap());
        assert!(html.contains("url(\"aeroedits-brand/title-font.woff2\")"));
        assert!(html.contains("--brand-logo"));
        assert!(html.contains("--look-muted: #6E6E73"));
        assert!(html.contains("#root, body { font-family: \"Source Serif\""));
        assert!(!css(&style, true).contains("--look-muted"));
    }
}
