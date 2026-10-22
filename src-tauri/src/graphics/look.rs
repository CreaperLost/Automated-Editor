//! The video's look: one set of design tokens every graphic in it takes (type, shape, layout,
//! colour roles and motion), beside the brand kit's colours, font and logo. A project has one
//! look in its edit document, so choosing or adjusting it is one undo step, and it is part of
//! every render's hash, so a change restyles every graphic that uses it.
//!
//! A look is a preset, five global controls that scale it (size, roundness, speed, energy,
//! density) and any tokens set by hand. At render time it becomes `--look-*` custom properties
//! on the composition element (where HyperFrames puts the colour variables the roles mix from),
//! in a style that comes after the composition's own, so it wins over the defaults each
//! template carries for the same tokens. A graphic may override tokens of its own.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_PRESET: &str = "app";
/// The style block's id in a staged composition.
pub const STYLE_ID: &str = "aeroedits-look";

/// The ranges of the global controls (1 leaves the preset as it is).
pub const SIZE_RANGE: (f32, f32) = (0.75, 1.35);
pub const ROUNDNESS_RANGE: (f32, f32) = (0.0, 2.5);
pub const SPEED_RANGE: (f32, f32) = (0.5, 2.0);
pub const ENERGY_RANGE: (f32, f32) = (0.0, 2.0);
pub const DENSITY_RANGE: (f32, f32) = (0.7, 1.4);
const MAX_TOKEN_CHARS: usize = 160;

fn one() -> f32 {
    1.0
}

fn default_preset() -> String {
    DEFAULT_PRESET.into()
}

/// What the edit document keeps of the video's look.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Look {
    /// The preset it starts from: app, calm or bold.
    #[serde(default = "default_preset")]
    pub preset: String,
    /// Type sizes, as a multiple of the preset's.
    #[serde(default = "one")]
    pub size: f32,
    /// Corner radii, as a multiple of the preset's (0 is square).
    #[serde(default = "one")]
    pub roundness: f32,
    /// How fast things move: 2 plays every entrance in half the time.
    #[serde(default = "one")]
    pub speed: f32,
    /// How far things travel as they come in (0 only fades them).
    #[serde(default = "one")]
    pub energy: f32,
    /// Padding, gaps and margins, as a multiple of the preset's.
    #[serde(default = "one")]
    pub density: f32,
    /// Tokens set by hand, by name without `--look-` (`radius`, `ease`...), over the rest.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tokens: BTreeMap<String, String>,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            preset: default_preset(),
            size: 1.0,
            roundness: 1.0,
            speed: 1.0,
            energy: 1.0,
            density: 1.0,
            tokens: BTreeMap::new(),
        }
    }
}

impl Look {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        preset(&self.preset)?;
        for (name, value, (lo, hi)) in [
            ("size", self.size, SIZE_RANGE),
            ("roundness", self.roundness, ROUNDNESS_RANGE),
            ("speed", self.speed, SPEED_RANGE),
            ("energy", self.energy, ENERGY_RANGE),
            ("density", self.density, DENSITY_RANGE),
        ] {
            if !(lo..=hi).contains(&value) {
                return Err(format!("The look's {name} must be {lo} to {hi}"));
            }
        }
        check_tokens(&self.tokens)
    }

    /// Every token's value: the preset's, scaled by the global controls, then the tokens set
    /// by hand.
    pub fn values(&self) -> BTreeMap<&'static str, String> {
        let p = preset(&self.preset).unwrap_or(&PRESETS[0]);
        let vmin = |v: f32| format!("{}vmin", round(v));
        let ms = |v: f32| format!("{}", (v / self.speed).round());
        let mut out: BTreeMap<&'static str, String> = BTreeMap::from([
            ("font", FONT.to_string()),
            ("title", vmin(p.title * self.size)),
            ("heading", vmin(p.heading * self.size)),
            ("body", vmin(p.body * self.size)),
            ("label", vmin(p.label_size * self.size)),
            ("title-weight", p.title_weight.to_string()),
            ("title-tracking", format!("{}em", p.title_tracking)),
            ("label-tracking", format!("{}em", p.label_tracking)),
            ("label-case", p.label_case.to_string()),
            ("leading", p.leading.to_string()),
            ("radius", vmin(p.radius * self.roundness)),
            ("radius-small", vmin(p.radius_small * self.roundness)),
            ("border", vmin(p.border)),
            ("pad", vmin(p.pad * self.density)),
            ("gap", vmin(p.gap * self.density)),
            ("margin", format!("{}vw", round(p.margin * self.density))),
            ("align", p.align.to_string()),
            ("muted-mix", format!("{}%", p.muted)),
            ("line-mix", format!("{}%", p.line)),
            ("tint-mix", format!("{}%", p.tint)),
            ("wash-mix", format!("{}%", p.wash)),
            ("shadow-mix", format!("{}%", p.shadow)),
            ("duration", ms(p.duration)),
            ("stagger", ms(p.stagger)),
            ("out", ms(p.out)),
            ("ease", p.ease.to_string()),
            ("gsap-ease", p.gsap_ease.to_string()),
            ("distance", vmin(p.distance * self.energy)),
            ("entry", p.entry.to_string()),
        ]);
        for (name, value) in &self.tokens {
            if let Some((&key, _)) = out.get_key_value(name.as_str()) {
                out.insert(key, value.clone());
            }
        }
        out
    }

    /// The style a render adds for this look, with a graphic's own `overrides` after it.
    pub fn css(&self, overrides: &BTreeMap<String, String>) -> String {
        let mut values = self.values();
        for (name, value) in overrides {
            if let Some((&key, _)) = values.get_key_value(name.as_str()) {
                values.insert(key, value.clone());
            }
        }
        let mut css = String::from("[data-composition-id] {\n");
        for (name, value) in &values {
            css.push_str(&format!("  --look-{name}: {value};\n"));
        }
        // Colour roles, mixed from the composition's own colour variables (the kit's).
        css.push_str(&format!(
            "  --look-muted: color-mix(in srgb, var(--text, {TEXT}) var(--look-muted-mix), var(--background, {BACKGROUND}));\n\
             \x20 --look-line: color-mix(in srgb, var(--text, {TEXT}) var(--look-line-mix), var(--background, {BACKGROUND}));\n\
             \x20 --look-tint: color-mix(in srgb, var(--accent, {ACCENT}) var(--look-tint-mix), var(--background, {BACKGROUND}));\n\
             \x20 --look-wash: color-mix(in srgb, var(--accent, {ACCENT}) var(--look-wash-mix), transparent);\n\
             \x20 --look-shadow: 0 1vmin 3vmin color-mix(in srgb, var(--text, {TEXT}) var(--look-shadow-mix), transparent);\n"
        ));
        css.push_str("}\n");
        css
    }
}

/// The colours the templates and the look fall back to without a brand kit: the app look.
pub const BACKGROUND: &str = "#FFFFFF";
pub const TEXT: &str = "#1D1D1F";
pub const ACCENT: &str = "#0071E3";
/// The kit's font when it has one, else the system's (San Francisco, Segoe UI Variable).
const FONT: &str = "var(--brand-font, -apple-system, BlinkMacSystemFont, \"SF Pro Display\", \
     \"Segoe UI Variable Display\", \"Segoe UI\", system-ui, sans-serif)";

fn round(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}

/// One preset: every token before the global controls scale it.
pub struct Preset {
    pub name: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    title: f32,
    heading: f32,
    body: f32,
    label_size: f32,
    title_weight: u16,
    title_tracking: f32,
    label_tracking: f32,
    label_case: &'static str,
    leading: f32,
    radius: f32,
    radius_small: f32,
    border: f32,
    pad: f32,
    gap: f32,
    margin: f32,
    align: &'static str,
    muted: u8,
    line: u8,
    tint: u8,
    wash: u8,
    shadow: u8,
    duration: f32,
    stagger: f32,
    out: f32,
    ease: &'static str,
    /// The same easing by GSAP's name, for graphics animated with it.
    gsap_ease: &'static str,
    distance: f32,
    entry: &'static str,
}

pub static PRESETS: [Preset; 3] = [
    Preset {
        name: "app",
        label: "App",
        description: "Clean and light, like a well-made app: rounded cards, hairlines, soft \
                      shadows, quick gentle motion.",
        title: 8.0,
        heading: 5.2,
        body: 3.8,
        label_size: 3.0,
        title_weight: 650,
        title_tracking: -0.035,
        label_tracking: 0.0,
        label_case: "none",
        leading: 1.15,
        radius: 2.6,
        radius_small: 1.4,
        border: 0.12,
        pad: 2.6,
        gap: 2.2,
        margin: 8.0,
        align: "center",
        muted: 62,
        line: 8,
        tint: 10,
        wash: 5,
        shadow: 5,
        duration: 700.0,
        stagger: 90.0,
        out: 350.0,
        ease: "cubic-bezier(0.22, 1, 0.36, 1)",
        gsap_ease: "expo.out",
        distance: 1.8,
        entry: "rise",
    },
    Preset {
        name: "calm",
        label: "Calm",
        description: "Softer and slower: rounder cards, lighter titles, things fade in with \
                      little travel.",
        title: 7.4,
        heading: 5.0,
        body: 3.6,
        label_size: 2.9,
        title_weight: 600,
        title_tracking: -0.025,
        label_tracking: 0.0,
        label_case: "none",
        leading: 1.2,
        radius: 3.6,
        radius_small: 2.0,
        border: 0.12,
        pad: 3.0,
        gap: 2.6,
        margin: 9.0,
        align: "center",
        muted: 58,
        line: 6,
        tint: 8,
        wash: 4,
        shadow: 4,
        duration: 1000.0,
        stagger: 140.0,
        out: 450.0,
        ease: "cubic-bezier(0.33, 1, 0.68, 1)",
        gsap_ease: "sine.out",
        distance: 1.0,
        entry: "fade",
    },
    Preset {
        name: "bold",
        label: "Bold",
        description: "Big, heavy titles set left, tighter corners and snappy motion with a \
                      little overshoot.",
        title: 9.2,
        heading: 5.8,
        body: 3.8,
        label_size: 2.8,
        title_weight: 760,
        title_tracking: -0.045,
        label_tracking: 0.12,
        label_case: "uppercase",
        leading: 1.05,
        radius: 1.6,
        radius_small: 0.8,
        border: 0.14,
        pad: 2.4,
        gap: 2.0,
        margin: 7.0,
        align: "left",
        muted: 64,
        line: 10,
        tint: 14,
        wash: 7,
        shadow: 7,
        duration: 520.0,
        stagger: 70.0,
        out: 280.0,
        ease: "cubic-bezier(0.34, 1.4, 0.64, 1)",
        gsap_ease: "back.out(1.6)",
        distance: 3.0,
        entry: "scale",
    },
];

pub fn preset(name: &str) -> Result<&'static Preset, String> {
    PRESETS.iter().find(|p| p.name == name).ok_or_else(|| {
        format!(
            "There is no look \"{name}\" (there are: {})",
            PRESETS
                .iter()
                .map(|p| p.name)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// Every token name a look has, for the editor and agents.
pub fn token_names() -> Vec<&'static str> {
    Look::default().values().into_keys().collect()
}

/// Tokens set by hand must be ones a look has, with plain CSS values.
pub fn check_tokens(tokens: &BTreeMap<String, String>) -> Result<(), String> {
    let names = token_names();
    for (name, value) in tokens {
        if !names.contains(&name.as_str()) {
            return Err(format!(
                "A look has no token \"{name}\" (it has: {})",
                names.join(", ")
            ));
        }
        if value.trim().is_empty()
            || value.chars().count() > MAX_TOKEN_CHARS
            || value.contains([';', '{', '}', '<', '>', '"', '\\'])
        {
            return Err(format!("\"{value}\" is not a value for the look's {name}"));
        }
    }
    Ok(())
}

/// `html` with `css` as the last style in its head, so it wins over the composition's own (and
/// before the brand kit's, which only sets `--brand-*` and the page font).
pub fn inject_css(html: &str, css: &str) -> String {
    let block = format!("<style id=\"{STYLE_ID}\">\n{css}</style>\n");
    match html.find("</head>") {
        Some(at) => format!("{}{block}{}", &html[..at], &html[at..]),
        None => format!("{block}{html}"),
    }
}

/// What the editor and agents see of the presets.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetView {
    pub name: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

pub fn presets() -> Vec<PresetView> {
    PRESETS
        .iter()
        .map(|p| PresetView {
            name: p.name,
            label: p.label,
            description: p.description,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_controls_scale_the_preset_and_tokens_win() {
        let mut look = Look::default();
        assert_eq!(look.values()["radius"], "2.6vmin");
        assert_eq!(look.values()["duration"], "700");
        look.roundness = 2.0;
        look.speed = 2.0;
        look.energy = 0.0;
        let values = look.values();
        assert_eq!(values["radius"], "5.2vmin");
        assert_eq!(values["duration"], "350");
        assert_eq!(values["distance"], "0vmin");
        look.tokens.insert("radius".into(), "0".into());
        assert_eq!(look.values()["radius"], "0");
        assert!(look.validate().is_ok());
    }

    #[test]
    fn bad_looks_are_refused() {
        let bad = |change: fn(&mut Look)| {
            let mut look = Look::default();
            change(&mut look);
            look.validate().is_err()
        };
        assert!(bad(|l| l.preset = "neon".into()));
        assert!(bad(|l| l.speed = 3.0));
        assert!(bad(|l| {
            l.tokens.insert("colour".into(), "red".into());
        }));
        assert!(bad(|l| {
            l.tokens
                .insert("radius".into(), "1px; } body { display: none".into());
        }));
    }

    #[test]
    fn the_css_sets_every_token_on_the_composition_and_a_graphic_can_override() {
        let css = Look::default().css(&BTreeMap::from([("radius".into(), "0".into())]));
        assert!(css.starts_with("[data-composition-id] {"));
        assert!(css.contains("--look-radius: 0;"));
        assert!(css.contains("--look-title: 8vmin;"));
        assert!(css.contains("--look-muted: color-mix("));
        for name in token_names() {
            assert!(css.contains(&format!("--look-{name}:")), "{name}");
        }
        let html = inject_css("<html><head><style>a{}</style></head></html>", &css);
        assert!(html.find(STYLE_ID).unwrap() > html.find("a{}").unwrap());
    }

    /// Writes each preset's style to `<AEROEDITS_LOOK_SAMPLES>/<preset>.css`, to stage templates
    /// by hand and look at them.
    #[test]
    #[ignore = "set AEROEDITS_LOOK_SAMPLES to a folder"]
    fn look_samples() {
        let dir = std::path::PathBuf::from(std::env::var("AEROEDITS_LOOK_SAMPLES").unwrap());
        for preset in &PRESETS {
            let look = Look {
                preset: preset.name.into(),
                ..Default::default()
            };
            std::fs::write(
                dir.join(format!("{}.css", preset.name)),
                look.css(&BTreeMap::new()),
            )
            .unwrap();
        }
    }

    #[test]
    fn every_template_carries_the_default_look() {
        let css = Look::default().css(&BTreeMap::new());
        for template in super::super::templates::all() {
            let html = template.html();
            for line in css
                .lines()
                .map(str::trim)
                .filter(|l| l.starts_with("--look-"))
            {
                assert!(html.contains(line), "{}: {line}", template.name);
            }
        }
    }
}
