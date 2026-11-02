//! Styles: how a video looks. One style holds the brand (colour roles, fonts, a logo), the
//! graphics look (`graphics::look`), how captions look, how the frame is dressed (background,
//! screen corners and shadow, camera border) and what changes on a 9:16 canvas.
//!
//! Styles are kept by the app, not a project: the built-in ones in code ([`builtin`]) and the
//! user's in `<config>/AeroEdits/styles/<name>/style.json` with their font and logo files beside
//! it ([`store`]). A project names one style; a graphic may name another. Nothing of a style is
//! stored in a project: [`dress`] puts the style's frame and caption look onto a document as it
//! is drawn, and graphics take theirs as they render (`graphics::render`).
pub mod brand;
pub mod request;
pub mod store;

use crate::captions::CaptionStyle;
use crate::graphics::look::{Look, ShortsLook};
use crate::project::layout::{BACKGROUND_PRESETS, MAX_BORDER_WIDTH};
use crate::project::revision::EditDocument;
use serde::{Deserialize, Serialize};

/// The style a new project uses, and the one a missing style falls back to.
pub const DEFAULT: &str = "Light";

/// What `style.json` holds. Every part may be left out and takes the Light style's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Style {
    /// Its folder's name (or the built-in's).
    #[serde(skip)]
    pub name: String,
    pub brand: Brand,
    pub graphics: Look,
    pub captions: CaptionsLook,
    pub frame: Frame,
    /// What changes on a 9:16 canvas (a short).
    pub portrait: Portrait,
    /// How its graphics move, in a few rules: how things enter and leave, what moves and what
    /// holds, how one graphic hands over to the next, its signature move. The look's motion
    /// tokens (`ease`, `duration`...) are the numbers; these are the words.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub motion: Vec<String>,
    /// The kinds of graphics it uses, and when.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<GraphicKind>,
    /// Graphics done well in it: items of the graphics library.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<Example>,
    /// Rules about the look ("never an opaque card over the face"). Workflow rules are the
    /// preset's.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<String>,
    /// The images and clips it was made from, files in its folder.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<Reference>,
    /// What was taken from the references, what was left out and why.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub breakdown: String,
    /// What its graphics sound like.
    #[serde(skip_serializing_if = "StyleSound::is_empty")]
    pub sound: StyleSound,
}

impl StyleSound {
    pub fn is_empty(&self) -> bool {
        self.policy.is_empty() && self.cues.is_empty()
    }
}

/// A kind of graphic a style uses.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct GraphicKind {
    /// Like "app mock-up", "quote card", "count-up".
    pub name: String,
    /// When it is used ("a step in the app", "a number the speaker gives").
    pub when: String,
    /// Where it shows in a short.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shorts: Option<crate::shorts::GraphicSlot>,
}

/// A graphic of the library that shows the style done well.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Example {
    /// The library item's id or name.
    pub graphic: String,
    /// The kind it shows, one of the style's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kind: String,
}

/// An image or clip the style was made from.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Reference {
    /// A file in the style's folder (a full path is copied in as it is saved).
    pub file: String,
    /// What it is and what the style takes from it.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// The sounds a style's graphics make.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct StyleSound {
    /// In words: "minimal and subtle, no heavy risers".
    #[serde(skip_serializing_if = "String::is_empty")]
    pub policy: String,
    /// The sound for each graphic moment.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cues: Vec<Cue>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Cue {
    /// The moment it is for: whoosh, click, pop, hit...
    pub role: String,
    /// A sound of the sound library, by id.
    pub sound: String,
}

const MAX_ITEMS: usize = 12;
const MAX_LINE_CHARS: usize = 240;
const MAX_NAME_CHARS: usize = 40;
const MAX_BREAKDOWN_CHARS: usize = 2000;
const MAX_EXAMPLES: usize = 6;

/// `lines` (the style's `what`) within the limits.
fn lines(what: &str, lines: &[String]) -> Result<(), String> {
    if lines.len() > MAX_ITEMS {
        return Err(format!("Keep at most {MAX_ITEMS} {what}"));
    }
    match lines
        .iter()
        .find(|l| l.trim().is_empty() || l.chars().count() > MAX_LINE_CHARS)
    {
        Some(_) => Err(format!(
            "Each of the style's {what} is one line of 1 to {MAX_LINE_CHARS} characters"
        )),
        None => Ok(()),
    }
}

fn short_text(what: &str, value: &str, max: usize) -> Result<(), String> {
    if value.chars().count() > max {
        Err(format!(
            "The style's {what} must be at most {max} characters"
        ))
    } else {
        Ok(())
    }
}

fn kinds_and_examples(
    which: &str,
    kinds: &[GraphicKind],
    examples: &[Example],
    known: &[&GraphicKind],
) -> Result<(), String> {
    if kinds.len() > MAX_ITEMS {
        return Err(format!("Keep at most {MAX_ITEMS} {which}kinds"));
    }
    for kind in kinds {
        if kind.name.trim().is_empty() || kind.name.chars().count() > MAX_NAME_CHARS {
            return Err(format!(
                "Name each {which}kind of graphic in 1 to {MAX_NAME_CHARS} characters"
            ));
        }
        short_text("kind's `when`", &kind.when, MAX_LINE_CHARS)?;
    }
    if examples.len() > MAX_EXAMPLES {
        return Err(format!("Keep at most {MAX_EXAMPLES} {which}examples"));
    }
    for example in examples {
        if example.graphic.trim().is_empty() || example.graphic.chars().count() > 80 {
            return Err("Give each example's `graphic`: a library item's id or name".into());
        }
        let named = |k: &&GraphicKind| k.name.eq_ignore_ascii_case(example.kind.trim());
        if !example.kind.trim().is_empty()
            && !kinds.iter().any(|k| named(&k))
            && !known.iter().any(named)
        {
            return Err(format!(
                "The example {} shows the kind \"{}\", which the style doesn't have",
                example.graphic, example.kind
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Brand {
    pub colors: Colors,
    pub fonts: Fonts,
    /// A logo file in the style's folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
}

/// The colour roles, `#RRGGBB`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Colors {
    pub background: String,
    pub text: String,
    pub accent: String,
    /// Secondary text: subtitles, captions under a title, labels.
    pub muted: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Fonts {
    /// Titles and captions.
    pub title: Font,
    /// Everything else; an empty family takes the title's.
    pub body: Font,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Font {
    /// A font family, like Inter; empty is the system's.
    pub family: String,
    /// A font file in the style's folder for `family`, when it is not installed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// How captions look. Where they sit and how many words they hold is the project's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct CaptionsLook {
    /// How the words come on screen.
    pub style: CaptionStyle,
    /// The font weight (100 to 900); 0 takes the graphics' title weight.
    pub weight: u16,
    /// Draw each caption's emphasis word in the emphasis colour.
    pub emphasis: bool,
    /// The emphasis colour; empty takes the brand's accent.
    pub emphasis_color: String,
    pub text_color: String,
    /// A dark outline around the letters.
    pub outline: bool,
    /// A box behind the text.
    #[serde(rename = "box")]
    pub boxed: CaptionBox,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct CaptionBox {
    pub on: bool,
    pub color: String,
    /// 0 to 1.
    pub opacity: f32,
}

/// How the frame is dressed: what is behind the screen and camera, and their edges.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Frame {
    pub background: Background,
    pub screen: ScreenLook,
    pub camera: CameraLook,
}

/// A background: `solid` (colorStart), `gradient` (colorStart to colorEnd) or `preset` (one
/// of the app's). A project's wallpaper image, when it has one, is shown instead.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Background {
    #[serde(rename = "type")]
    pub kind: String,
    /// aurora, sunset, ocean, forest, candy or graphite.
    pub preset: String,
    pub color_start: String,
    pub color_end: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenLook {
    /// At a 1080 px short side; 0 to 32.
    pub corner_radius_px: u32,
    /// 0 (no shadow) to 40.
    pub shadow_blur_px: u32,
    pub shadow_opacity: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct CameraLook {
    pub border_color: String,
    /// 0 (none) to 8.
    pub border_width: u32,
    pub shadow: bool,
}

/// What a 9:16 canvas changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Portrait {
    /// Multiples of the graphics look's size, energy and speed (bigger type reads on a phone).
    pub graphics: ShortsLook,
    /// Another caption style; absent keeps the captions' own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption_style: Option<CaptionStyle>,
    /// Another background; absent keeps the frame's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,
    /// Motion rules a short adds to the style's.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub motion: Vec<String>,
    /// Kinds of graphics only shorts use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<GraphicKind>,
    /// Graphics done well in a short.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<Example>,
}

impl Default for Portrait {
    fn default() -> Self {
        Self {
            graphics: ShortsLook::default(),
            // Words popping in as they are said suit a phone.
            caption_style: Some(CaptionStyle::Pop),
            background: None,
            motion: Vec::new(),
            kinds: Vec::new(),
            examples: Vec::new(),
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            background: crate::graphics::look::BACKGROUND.into(),
            text: crate::graphics::look::TEXT.into(),
            accent: crate::graphics::look::ACCENT.into(),
            muted: "#6E6E73".into(),
        }
    }
}

impl Default for CaptionsLook {
    fn default() -> Self {
        Self {
            style: CaptionStyle::default(),
            weight: 0,
            emphasis: true,
            emphasis_color: String::new(),
            text_color: "#FFFFFF".into(),
            outline: true,
            boxed: CaptionBox::default(),
        }
    }
}

impl Default for CaptionBox {
    fn default() -> Self {
        Self {
            on: false,
            color: "#000000".into(),
            opacity: 0.55,
        }
    }
}

impl Default for Background {
    fn default() -> Self {
        Self {
            kind: "preset".into(),
            preset: "forest".into(),
            color_start: "#312E81".into(),
            color_end: "#0F172A".into(),
        }
    }
}

impl Default for ScreenLook {
    fn default() -> Self {
        Self {
            corner_radius_px: 17,
            shadow_blur_px: 0,
            shadow_opacity: 0.5,
        }
    }
}

impl Default for CameraLook {
    fn default() -> Self {
        Self {
            border_color: "#6366F1".into(),
            border_width: MAX_BORDER_WIDTH,
            shadow: true,
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            name: DEFAULT.into(),
            brand: Brand::default(),
            graphics: Look::default(),
            captions: CaptionsLook::default(),
            frame: Frame::default(),
            portrait: Portrait::default(),
            motion: Vec::new(),
            kinds: Vec::new(),
            examples: Vec::new(),
            rules: Vec::new(),
            references: Vec::new(),
            breakdown: String::new(),
            sound: StyleSound::default(),
        }
    }
}

/// The styles the app ships. They are read-only: changing one saves a copy.
pub fn builtin() -> Vec<Style> {
    let light = Style::default();
    let dark = Style {
        name: "Dark".into(),
        brand: Brand {
            colors: Colors {
                background: "#111114".into(),
                text: "#F5F5F7".into(),
                accent: "#0A84FF".into(),
                muted: "#98989F".into(),
            },
            ..Brand::default()
        },
        frame: Frame {
            background: Background {
                kind: "preset".into(),
                preset: "graphite".into(),
                ..Background::default()
            },
            screen: ScreenLook {
                shadow_blur_px: 24,
                shadow_opacity: 0.45,
                ..ScreenLook::default()
            },
            camera: CameraLook {
                border_color: "#F5F5F7".into(),
                border_width: 4,
                shadow: true,
            },
        },
        ..Style::default()
    };
    vec![light, dark]
}

pub fn is_builtin(name: &str) -> bool {
    builtin()
        .iter()
        .any(|s| s.name.eq_ignore_ascii_case(name.trim()))
}

fn color(what: &str, value: &str) -> Result<(), String> {
    let hex = value.strip_prefix('#').unwrap_or("");
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!(
            "The style's {what} colour must be like #0F172A, not \"{value}\""
        ))
    }
}

fn family(what: &str, value: &str) -> Result<(), String> {
    if value.chars().count() > 80 || value.contains(['"', '\'', ';', '{', '}', '<', '>', '\\']) {
        Err(format!("Give the {what} font as a family name, like Inter"))
    } else {
        Ok(())
    }
}

impl Background {
    pub fn validate(&self) -> Result<(), String> {
        match self.kind.as_str() {
            "solid" | "gradient" => {}
            "preset" if BACKGROUND_PRESETS.contains(&self.preset.as_str()) => {}
            "preset" => {
                return Err(format!(
                    "There is no background \"{}\" (there are: {})",
                    self.preset,
                    BACKGROUND_PRESETS.join(", ")
                ))
            }
            other => {
                return Err(format!(
                    "A background is solid, gradient or preset, not \"{other}\""
                ))
            }
        }
        color("background start", &self.color_start)?;
        color("background end", &self.color_end)
    }
}

impl Style {
    /// Every part valid, and its files are files in its folder.
    pub fn validate(&self) -> Result<(), String> {
        self.validate_parts()?;
        for file in [
            &self.brand.fonts.title.file,
            &self.brand.fonts.body.file,
            &self.brand.logo,
        ]
        .into_iter()
        .flatten()
        .chain(self.references.iter().map(|r| &r.file))
        {
            if !store::plain_file(file) {
                return Err(format!("\"{file}\" is not a file in the style's folder"));
            }
        }
        Ok(())
    }

    /// Every part valid, its files aside (a full path is taken in as it is saved).
    pub fn validate_parts(&self) -> Result<(), String> {
        let c = &self.brand.colors;
        for (what, value) in [
            ("background", &c.background),
            ("text", &c.text),
            ("accent", &c.accent),
            ("muted", &c.muted),
        ] {
            color(what, value)?;
        }
        family("title", &self.brand.fonts.title.family)?;
        family("body", &self.brand.fonts.body.family)?;
        self.graphics.validate()?;
        let (lo, hi) = crate::graphics::look::SHORTS_RANGE;
        let p = &self.portrait.graphics;
        for (what, value) in [("size", p.size), ("energy", p.energy), ("speed", p.speed)] {
            if !value.is_finite() || !(lo..=hi).contains(&value) {
                return Err(format!("The 9:16 {what} must be {lo} to {hi}"));
            }
        }
        let cap = &self.captions;
        let (lo, hi) = crate::captions::fonts::WEIGHT_RANGE;
        if cap.weight != 0 && !(lo..=hi).contains(&cap.weight) {
            return Err(format!(
                "Caption weight must be {lo} to {hi}, or 0 for the graphics' title weight"
            ));
        }
        color("caption text", &cap.text_color)?;
        if !cap.emphasis_color.is_empty() {
            color("caption emphasis", &cap.emphasis_color)?;
        }
        color("caption box", &cap.boxed.color)?;
        if !cap.boxed.opacity.is_finite() || !(0.0..=1.0).contains(&cap.boxed.opacity) {
            return Err("The caption box's opacity must be 0 to 1".into());
        }
        let f = &self.frame;
        f.background.validate()?;
        if let Some(background) = &self.portrait.background {
            background.validate()?;
        }
        if f.screen.corner_radius_px > crate::project::layout::MAX_CORNER_RADIUS_PX {
            return Err("The screen's corner radius must be 0 to 32".into());
        }
        if f.screen.shadow_blur_px > crate::project::layout::MAX_SHADOW_BLUR_PX {
            return Err("The screen's shadow blur must be 0 to 40".into());
        }
        if !f.screen.shadow_opacity.is_finite() || !(0.0..=1.0).contains(&f.screen.shadow_opacity) {
            return Err("The screen's shadow opacity must be 0 to 1".into());
        }
        if f.camera.border_width > MAX_BORDER_WIDTH {
            return Err(format!(
                "The camera border must be 0 to {MAX_BORDER_WIDTH} wide"
            ));
        }
        color("camera border", &f.camera.border_color)?;
        lines("motion rules", &self.motion)?;
        lines("look rules", &self.rules)?;
        lines("9:16 motion rules", &self.portrait.motion)?;
        kinds_and_examples("", &self.kinds, &self.examples, &[])?;
        let kinds: Vec<&GraphicKind> = self.kinds.iter().collect();
        kinds_and_examples(
            "9:16 ",
            &self.portrait.kinds,
            &self.portrait.examples,
            &kinds,
        )?;
        if self.references.len() > MAX_ITEMS {
            return Err(format!("Keep at most {MAX_ITEMS} references"));
        }
        for reference in &self.references {
            if reference.file.trim().is_empty() {
                return Err("Give each reference its `file`".into());
            }
            short_text("reference's note", &reference.note, MAX_LINE_CHARS)?;
        }
        short_text("breakdown", &self.breakdown, MAX_BREAKDOWN_CHARS)?;
        short_text("sound policy", &self.sound.policy, MAX_LINE_CHARS)?;
        if self.sound.cues.len() > MAX_ITEMS {
            return Err(format!("Keep at most {MAX_ITEMS} sound cues"));
        }
        for cue in &self.sound.cues {
            if cue.role.trim().is_empty() || cue.role.chars().count() > MAX_NAME_CHARS {
                return Err("Give each sound cue its `role` (whoosh, click, pop, hit...)".into());
            }
            if cue.sound.trim().is_empty() {
                return Err(format!(
                    "Give the {} cue its `sound`, a sound library id",
                    cue.role
                ));
            }
        }
        Ok(())
    }

    /// Colours as `#RRGGBB` in capitals.
    pub fn normalized(mut self) -> Self {
        let up = |s: &mut String| *s = s.trim().to_uppercase();
        let c = &mut self.brand.colors;
        for s in [&mut c.background, &mut c.text, &mut c.accent, &mut c.muted] {
            up(s);
        }
        up(&mut self.captions.text_color);
        up(&mut self.captions.emphasis_color);
        up(&mut self.captions.boxed.color);
        up(&mut self.frame.background.color_start);
        up(&mut self.frame.background.color_end);
        up(&mut self.frame.camera.border_color);
        if let Some(b) = &mut self.portrait.background {
            up(&mut b.color_start);
            up(&mut b.color_end);
        }
        self.brand.fonts.title.family = self.brand.fonts.title.family.trim().into();
        self.brand.fonts.body.family = self.brand.fonts.body.family.trim().into();
        self
    }

    /// The graphics look on a canvas of this shape.
    pub fn graphics_for(&self, portrait: bool) -> Look {
        if portrait {
            self.graphics.scaled(&self.portrait.graphics)
        } else {
            self.graphics.clone()
        }
    }

    /// `self` with `patch` merged in (JSON merge patch: objects merge, `null` takes a value
    /// back to the Light style's, anything else replaces).
    pub fn patched(&self, patch: &serde_json::Value) -> Result<Style, String> {
        let mut value = serde_json::to_value(self).map_err(|e| e.to_string())?;
        merge(
            &mut value,
            patch,
            &serde_json::to_value(Style::default()).unwrap_or_default(),
        );
        let mut out: Style =
            serde_json::from_value(value).map_err(|e| format!("Not a style: {e}"))?;
        out.name = self.name.clone();
        let out = out.normalized();
        // Its files may be full paths to take in: they are checked as it is saved.
        out.validate_parts()?;
        Ok(out)
    }

    /// Its files, each its own file name in its folder or a full path to take in.
    pub fn files_mut(&mut self) -> Vec<&mut String> {
        let fonts = &mut self.brand.fonts;
        [
            &mut fonts.title.file,
            &mut fonts.body.file,
            &mut self.brand.logo,
        ]
        .into_iter()
        .flatten()
        .chain(self.references.iter_mut().map(|r| &mut r.file))
        .collect()
    }

    /// The weights each of its fonts is drawn at: the title's (and the captions'), the body's.
    pub fn font_weights(&self) -> [(&'static str, &Font, Vec<u16>); 2] {
        let title: u16 = self.graphics.values()["title-weight"]
            .parse()
            .unwrap_or(700);
        let mut titles = vec![title];
        if self.captions.weight != 0 && self.captions.weight != title {
            titles.push(self.captions.weight);
        }
        [
            ("title", &self.brand.fonts.title, titles),
            ("body", &self.brand.fonts.body, vec![400]),
        ]
    }

    /// Whether each named font is there for a render: `Ok` says how, `Err` why not.
    pub fn fonts_status(&self) -> Vec<(&'static str, Result<String, String>)> {
        let folder = store::folder_of(self);
        self.font_weights()
            .into_iter()
            .filter(|(_, font, _)| !font.family.trim().is_empty())
            .map(|(which, font, weights)| {
                (
                    which,
                    crate::captions::fonts::availability(font, folder.as_deref(), &weights),
                )
            })
            .collect()
    }
}

fn merge(target: &mut serde_json::Value, patch: &serde_json::Value, defaults: &serde_json::Value) {
    use serde_json::Value;
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                let default = defaults.get(key).cloned().unwrap_or(Value::Null);
                if value.is_null() {
                    if default.is_null() {
                        target.remove(key);
                    } else {
                        target.insert(key.clone(), default);
                    }
                    continue;
                }
                let slot = target.entry(key.clone()).or_insert(Value::Null);
                if value.is_object() && slot.is_object() {
                    merge(slot, value, &default);
                } else {
                    *slot = value.clone();
                }
            }
        }
        (target, patch) => *target = patch.clone(),
    }
}

/// Whether `document` draws on a 9:16 canvas: a short, or a tall video.
pub fn is_portrait(document: &EditDocument) -> bool {
    document.short_layout.is_some() || document.layout.aspect_ratio == "9:16"
}

/// The style `document` names (Light when it is missing).
pub fn of(document: &EditDocument) -> Style {
    store::resolve(&document.style)
}

/// Puts `document`'s style onto it, as it is drawn: the frame's look onto its layout (a
/// wallpaper of the project's own wins over the background) and the captions' look onto its
/// captions, the 9:16 variant on a tall canvas.
pub fn dress(document: &mut EditDocument) {
    let style = of(document);
    dress_with(document, &style);
}

pub fn dress_with(document: &mut EditDocument, style: &Style) {
    let portrait = is_portrait(document);
    let frame = &style.frame;
    let layout = &mut document.layout;
    let background = match (&style.portrait.background, portrait) {
        (Some(b), true) => b,
        _ => &frame.background,
    };
    layout.background_type = if layout.wallpaper_asset.is_some() {
        "wallpaper".into()
    } else {
        background.kind.clone()
    };
    layout.background_preset = background.preset.clone();
    layout.color_start = background.color_start.clone();
    layout.color_end = background.color_end.clone();
    layout.corner_radius_px = frame.screen.corner_radius_px;
    layout.shadow_blur_px = frame.screen.shadow_blur_px;
    layout.shadow_opacity = frame.screen.shadow_opacity;
    layout.webcam_border_color = frame.camera.border_color.clone();
    layout.webcam_border_width = frame.camera.border_width;
    layout.webcam_shadow = frame.camera.shadow;
    dress_captions(&mut document.captions, style, portrait);
}

/// Puts `style`'s caption look onto `captions`.
pub fn dress_captions(
    captions: &mut crate::captions::CaptionSettings,
    style: &Style,
    portrait: bool,
) {
    let look = &style.captions;
    captions.style = match (style.portrait.caption_style, portrait) {
        (Some(s), true) => s,
        _ => look.style,
    };
    captions.weight = look.weight;
    captions.emphasis = look.emphasis;
    captions.emphasis_color = look.emphasis_color.clone();
    captions.text_color = look.text_color.clone();
    captions.outline = look.outline;
    captions.background = look.boxed.on;
    captions.background_color = look.boxed.color.clone();
    captions.background_opacity = look.boxed.opacity;
}

/// What the editor and agents see of a style.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleView {
    pub name: String,
    pub builtin: bool,
    pub style: Style,
    /// Its folder (none for a built-in).
    pub folder: Option<String>,
    /// Full paths of its files, for thumbnails and font previews.
    pub logo_path: Option<String>,
}

pub fn view(style: &Style) -> StyleView {
    let folder = store::folder_of(style);
    StyleView {
        name: style.name.clone(),
        builtin: is_builtin(&style.name),
        style: style.clone(),
        logo_path: folder
            .as_ref()
            .zip(style.brand.logo.as_ref())
            .map(|(f, l)| f.join(l).display().to_string()),
        folder: folder.map(|f| f.display().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtins_are_valid_and_light_is_the_default() {
        for style in builtin() {
            style.validate().unwrap();
            assert_eq!(style.clone().normalized(), style, "{}", style.name);
        }
        assert_eq!(Style::default().name, DEFAULT);
        assert!(is_builtin("light") && is_builtin("Dark") && !is_builtin("Mine"));
    }

    #[test]
    fn a_patch_merges_and_null_takes_the_default_back() {
        let style = Style::default();
        let dark = style
            .patched(&serde_json::json!({
                "brand": { "colors": { "accent": "#ff0000" } },
                "captions": { "box": { "on": true } },
                "portrait": { "captionStyle": "pop" }
            }))
            .unwrap();
        assert_eq!(dark.brand.colors.accent, "#FF0000");
        assert_eq!(dark.brand.colors.text, style.brand.colors.text);
        assert!(dark.captions.boxed.on);
        assert_eq!(dark.portrait.caption_style, Some(CaptionStyle::Pop));
        let back = dark
            .patched(&serde_json::json!({
                "brand": { "colors": { "accent": null } },
                "portrait": { "captionStyle": null }
            }))
            .unwrap();
        assert_eq!(back.brand.colors.accent, style.brand.colors.accent);
        assert_eq!(back.portrait.caption_style, style.portrait.caption_style);
        assert!(style
            .patched(&serde_json::json!({ "frame": { "background": { "preset": "mars" } } }))
            .is_err());
    }

    #[test]
    fn dressing_puts_the_frame_and_caption_look_on_and_a_wallpaper_wins() {
        let mut document = EditDocument::default();
        let dark = builtin().remove(1);
        dress_with(&mut document, &dark);
        assert_eq!(document.layout.background_preset, "graphite");
        assert_eq!(document.layout.webcam_border_color, "#F5F5F7");
        assert_eq!(document.layout.shadow_blur_px, 24);
        document.layout.wallpaper_asset = Some("assets/wallpapers/a.png".into());
        dress_with(&mut document, &dark);
        assert_eq!(document.layout.background_type, "wallpaper");
        // A short takes the 9:16 variant.
        let mut style = Style::default();
        style.portrait.caption_style = Some(CaptionStyle::Pop);
        style.portrait.background = Some(Background {
            kind: "solid".into(),
            ..Background::default()
        });
        let mut short = EditDocument::default();
        short.layout.aspect_ratio = "9:16".into();
        dress_with(&mut short, &style);
        assert_eq!(short.captions.style, CaptionStyle::Pop);
        assert_eq!(short.layout.background_type, "solid");
        let mut wide = EditDocument::default();
        dress_with(&mut wide, &style);
        assert_eq!(wide.captions.style, CaptionStyle::Karaoke);
        assert_eq!(wide.layout.background_type, "preset");
    }

    #[test]
    fn motion_kinds_examples_and_rules_are_kept_within_limits() {
        let style = Style::default()
            .patched(&serde_json::json!({
                "motion": ["Enter with ease-out, never linear", "Hold the result two seconds"],
                "kinds": [
                    { "name": "app mock-up", "when": "a step in the app", "shorts": "screen" },
                    { "name": "quote card", "when": "a line worth reading" }
                ],
                "examples": [{ "graphic": "white-chat-reset", "kind": "quote card" }],
                "rules": ["Never an opaque card over the face"],
                "sound": { "policy": "Minimal, no heavy risers", "cues": [{ "role": "pop", "sound": "pop" }] },
                "portrait": { "examples": [{ "graphic": "x", "kind": "app mock-up" }] }
            }))
            .unwrap();
        assert_eq!(
            style.kinds[0].shorts,
            Some(crate::shorts::GraphicSlot::Screen)
        );
        // Saved as written, and an empty part is left out of the file.
        let text = serde_json::to_string(&style).unwrap();
        assert!(text.contains("\"motion\"") && !text.contains("\"references\""));
        assert!(!serde_json::to_string(&Style::default())
            .unwrap()
            .contains("\"kinds\""));
        for bad in [
            serde_json::json!({ "motion": [""] }),
            serde_json::json!({ "rules": ["x".repeat(300)] }),
            serde_json::json!({ "kinds": [{ "name": "", "when": "x" }] }),
            serde_json::json!({ "examples": [{ "graphic": "a", "kind": "no such kind" }] }),
            serde_json::json!({ "sound": { "cues": [{ "role": "pop", "sound": "" }] } }),
            serde_json::json!({ "references": [{ "file": "" }] }),
        ] {
            assert!(style.patched(&bad).is_err(), "{bad}");
        }
        // A reference is a file in the style's folder once saved.
        let mut with_path = style.clone();
        with_path.references = vec![Reference {
            file: r"C:\refs\a.png".into(),
            note: String::new(),
        }];
        assert!(with_path.validate_parts().is_ok() && with_path.validate().is_err());
    }

    #[test]
    fn a_font_that_isnt_there_says_so() {
        let mut style = Style::default();
        assert!(style.fonts_status().is_empty(), "the system's font");
        style.brand.fonts.title.family = "No Such Font Qzx".into();
        let status = style.fonts_status();
        let why = status[0].1.as_ref().unwrap_err();
        assert!(why.contains("not installed"), "{why}");
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("title-font.ttf"), b"font").unwrap();
        style.brand.fonts.title.file = Some("title-font.ttf".into());
        assert!(crate::captions::fonts::availability(
            &style.brand.fonts.title,
            Some(dir.path()),
            &[700]
        )
        .unwrap()
        .contains("the style's file"));
    }

    #[test]
    fn a_tall_canvas_scales_the_graphics_look() {
        let style = Style::default();
        assert_eq!(style.graphics_for(true).size, 1.15);
        assert_eq!(style.graphics_for(false).size, 1.0);
    }
}
