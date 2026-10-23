//! Captions burned into preview and export. The words come from a track's transcript, so
//! captions follow every cut and undo; where they sit and how many words they hold live in the
//! edit document, how they look (colours, outline, box, pop / karaoke / static, the typeface and
//! accent) in the project's style ([`crate::style`], [`CaptionLook`]).
//!
//! Cues are groups of a few kept words in edited time. Each word of a cue is rasterized once
//! into coverage masks, then composed for the moment shown: popping in, lit as it is said, or
//! still ([`CaptionStyle`]). Everything depends on the edited time alone, so the preview and
//! the export draw the same frame.
pub mod fonts;

use crate::media::{ColorInfo, PixelFormat, VideoFrame};
use crate::project::layout::parse_hex_rgb;
use crate::timeline::TimelineMapper;
use crate::transcript::{Transcript, WordKind};
pub use fonts::Typeface;
use serde::{Deserialize, Serialize};

pub const FONT_SIZE_PCT_RANGE: (f32, f32) = (2.0, 15.0);
pub const OFFSET_PCT_RANGE: (f32, f32) = (0.0, 45.0);
pub const MAX_WORDS_RANGE: (u32, u32) = (1, 12);
/// A pause this long between words starts a new caption.
const CUE_PAUSE_US: u64 = 700_000;
/// No caption stays up longer than this, however few words it has.
const MAX_CUE_US: u64 = 5_000_000;
/// A caption lingers this long after its last word unless the next one starts sooner.
const LINGER_US: u64 = 400_000;
/// Lines wrap at this fraction of the canvas width.
const MAX_LINE_WIDTH: f32 = 0.86;
const MAX_LINES: usize = 3;
/// How long a word takes to pop in at the look's speed 1.
const POP_US: f32 = 240_000.0;
/// How far a popping word grows past its size before it settles (back-out easing).
const POP_OVERSHOOT: f32 = 1.70158;
/// How bright a word not yet said is in karaoke, as a share of its full opacity.
const UNSAID: f32 = 0.45;

/// How a caption's words come on screen.
#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum CaptionStyle {
    /// Each word pops in as it is said: it grows from a little smaller and fades in.
    Pop,
    /// The whole caption shows at once, dim, and each word lights up as it is said.
    #[default]
    Karaoke,
    /// The whole caption shows at once and stays still.
    Static,
}

impl CaptionStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pop => "pop",
            Self::Karaoke => "karaoke",
            Self::Static => "static",
        }
    }
}

fn default_position() -> String {
    "bottom".into()
}
fn default_offset_pct() -> f32 {
    8.0
}
fn default_font_size_pct() -> f32 {
    5.5
}
fn default_text_color() -> String {
    "#FFFFFF".into()
}
fn default_true() -> bool {
    true
}
fn default_background_color() -> String {
    "#000000".into()
}
fn default_background_opacity() -> f32 {
    0.55
}
fn default_max_words() -> u32 {
    6
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptionSettings {
    #[serde(default)]
    pub enabled: bool,
    /// Transcript to caption. `None` picks the microphone, then system audio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
    /// `bottom`, `middle` or `top`.
    #[serde(default = "default_position")]
    pub position: String,
    /// Distance from the top or bottom edge, as a percentage of the canvas height.
    #[serde(default = "default_offset_pct")]
    pub offset_pct: f32,
    /// Font size as a percentage of the canvas height.
    #[serde(default = "default_font_size_pct")]
    pub font_size_pct: f32,
    #[serde(default)]
    pub uppercase: bool,
    /// Most words shown at once.
    #[serde(default = "default_max_words")]
    pub max_words: u32,
    /// Most lines a caption takes; a longer one is drawn smaller to fit.
    #[serde(default = "default_max_lines")]
    pub max_lines: u32,
    // How they look: never stored, put on from the project's style as the document is drawn
    // (`style::dress`).
    #[serde(skip, default = "default_text_color")]
    pub text_color: String,
    /// How the words come on screen.
    #[serde(skip)]
    pub style: CaptionStyle,
    /// Draw the emphasis words (picked per caption, kept on the transcript) in the accent.
    #[serde(skip, default = "default_true")]
    pub emphasis: bool,
    /// The emphasis colour; empty takes the style's accent.
    #[serde(skip)]
    pub emphasis_color: String,
    /// The font weight (100 to 900); 0 takes the graphics' title weight.
    #[serde(skip)]
    pub weight: u16,
    /// A dark outline around the letters.
    #[serde(skip, default = "default_true")]
    pub outline: bool,
    /// A box behind the text.
    #[serde(skip)]
    pub background: bool,
    #[serde(skip, default = "default_background_color")]
    pub background_color: String,
    #[serde(skip, default = "default_background_opacity")]
    pub background_opacity: f32,
}

/// One line: easier to read, and what a caption turned on starts with.
fn default_max_lines() -> u32 {
    1
}

impl Default for CaptionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            track_id: None,
            position: default_position(),
            offset_pct: default_offset_pct(),
            font_size_pct: default_font_size_pct(),
            text_color: default_text_color(),
            style: CaptionStyle::default(),
            emphasis: true,
            emphasis_color: String::new(),
            weight: 0,
            outline: true,
            background: false,
            background_color: default_background_color(),
            background_opacity: default_background_opacity(),
            uppercase: false,
            max_words: default_max_words(),
            max_lines: default_max_lines(),
        }
    }
}

impl CaptionSettings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        let check = |name: &str, value: f32, (lo, hi): (f32, f32)| {
            if value.is_finite() && (lo..=hi).contains(&value) {
                Ok(())
            } else {
                Err(format!("{name} must be between {lo} and {hi}"))
            }
        };
        check("Caption size", self.font_size_pct, FONT_SIZE_PCT_RANGE)?;
        check("Caption offset", self.offset_pct, OFFSET_PCT_RANGE)?;
        if !(MAX_WORDS_RANGE.0..=MAX_WORDS_RANGE.1).contains(&self.max_words) {
            return Err(format!(
                "Words per caption must be between {} and {}",
                MAX_WORDS_RANGE.0, MAX_WORDS_RANGE.1
            ));
        }
        if !(1..=MAX_LINES as u32).contains(&self.max_lines) {
            return Err(format!("A caption takes 1 to {MAX_LINES} lines"));
        }
        if !matches!(self.position.as_str(), "bottom" | "middle" | "top") {
            return Err("Caption position must be bottom, middle or top".into());
        }
        // A sound's key, `<asset>.<stream>`; that it names a sound of the project is checked
        // where the assets are known.
        if let Some(track) = &self.track_id {
            if track.len() > 160
                || track.contains("..")
                || crate::sequence::StreamRef::parse(track).is_none()
                || !track
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            {
                return Err("Invalid caption track id".into());
            }
        }
        Ok(())
    }
}

/// What captions take from the style beyond their settings: the typeface (the brand's title
/// font at the caption weight), the accent the emphasis words are drawn in, and how fast and
/// far a word pops (the graphics look's speed and energy).
#[derive(Clone)]
pub struct CaptionLook {
    pub face: Typeface,
    /// The emphasis colour, `#RRGGBB`.
    pub accent: String,
    pub weight: u16,
    pop_us: f32,
    /// The size a popping word starts at, as a share of its own.
    pop_from: f32,
}

impl CaptionLook {
    /// `settings` as dressed by `style` (its weight and emphasis colour), with `look`'s motion.
    pub fn new(
        settings: &CaptionSettings,
        style: &crate::style::Style,
        look: &crate::graphics::look::Look,
    ) -> Self {
        let weight = if settings.weight > 0 {
            settings.weight
        } else {
            look.values()
                .get("title-weight")
                .and_then(|w| w.trim().parse::<u16>().ok())
                .unwrap_or(700)
        }
        .clamp(fonts::WEIGHT_RANGE.0, fonts::WEIGHT_RANGE.1);
        let accent = if !settings.emphasis_color.is_empty() {
            settings.emphasis_color.clone()
        } else {
            Some(style.brand.colors.accent.clone())
                .filter(|a| parse_hex_rgb(a).is_ok())
                .unwrap_or_else(|| crate::graphics::look::ACCENT.into())
        };
        Self {
            face: fonts::resolve(style, weight),
            accent,
            weight,
            pop_us: POP_US / look.speed.max(0.1),
            pop_from: (1.0 - 0.15 * look.energy).clamp(0.6, 1.0),
        }
    }

    /// The look of `document`'s captions, from its style.
    pub fn of(document: &crate::project::revision::EditDocument) -> Self {
        let style = crate::style::of(document);
        let portrait = crate::style::is_portrait(document);
        let mut settings = document.captions.clone();
        crate::style::dress_captions(&mut settings, &style, portrait);
        Self::new(&settings, &style, &style.graphics_for(portrait))
    }

    /// The Light style's look, for `settings` as they are.
    pub fn plain(settings: &CaptionSettings) -> Self {
        Self::new(
            settings,
            &crate::style::Style::default(),
            &crate::graphics::look::Look::default(),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CueWord {
    /// The transcript word it shows.
    pub id: String,
    pub text: String,
    pub start_us: u64,
    pub end_us: u64,
    /// Drawn in the accent: the word picked to stand out in its caption.
    pub emphasis: bool,
}

/// One caption on screen, in edited time.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptionCue {
    pub start_us: u64,
    pub end_us: u64,
    pub words: Vec<CueWord>,
}

fn ends_sentence(text: &str) -> bool {
    // `;` and U+037E are the Greek question mark.
    text.ends_with(['.', '?', '!', '\u{2026}', ';', '\u{37E}'])
}

/// Groups the transcript's kept words into cues. Cut words never appear; a cue breaks at a
/// sentence end, a pause, `max_words`, or after [`MAX_CUE_US`].
pub fn build_cues(
    transcript: &Transcript,
    mapper: &TimelineMapper,
    settings: &CaptionSettings,
) -> Vec<CaptionCue> {
    let max_words = settings
        .max_words
        .clamp(MAX_WORDS_RANGE.0, MAX_WORDS_RANGE.1) as usize;
    // (word, source start, source end), in playback order once clips are reordered.
    let mut words = Vec::new();
    for word in &transcript.words {
        let mark = transcript.caption_mark(&word.id);
        if word.kind != WordKind::Word || word.text.trim().is_empty() || mark.hidden {
            continue;
        }
        let Some((start_us, end_us)) =
            mapper.edited_span_of(word.source_start_us, word.source_end_us)
        else {
            continue;
        };
        let text = if settings.uppercase {
            crate::transcript::language::to_caps(word.text.trim())
        } else {
            word.text.trim().to_string()
        };
        // Punctuation given as a word of its own joins the word before ("video" "." → "video.").
        if text.chars().all(|c| !c.is_alphanumeric()) {
            if let Some((last, _, _)) = words.last_mut() {
                let last: &mut CueWord = last;
                last.text.push_str(&text);
                continue;
            }
        }
        words.push((
            CueWord {
                id: word.id.clone(),
                text,
                start_us,
                end_us: end_us.max(start_us + 1),
                emphasis: mark.emphasis,
            },
            word.source_start_us,
            word.source_end_us,
        ));
    }
    words.sort_by_key(|(word, _, _)| word.start_us);

    let mut cues: Vec<CaptionCue> = Vec::new();
    let mut current: Vec<CueWord> = Vec::new();
    let mut last_source_end = 0u64;
    for (word, source_start, source_end) in words {
        if let Some(last) = current.last() {
            let first_start = current[0].start_us;
            // Playing earlier recording time next means a clip was reordered here: never
            // join across it. (A cut jumps forward and keeps the cue together.)
            let jumped = source_start < last_source_end;
            let mark = transcript.caption_mark(&word.id);
            // A user's break or join (from the caption track) wins over the automatic rules.
            let breaks = mark.cue_break
                || jumped
                || (!mark.cue_join
                    && (current.len() >= max_words
                        || ends_sentence(&last.text)
                        || word.start_us.saturating_sub(last.end_us) >= CUE_PAUSE_US
                        || word.end_us.saturating_sub(first_start) > MAX_CUE_US));
            if breaks {
                cues.push(cue_from(std::mem::take(&mut current)));
            }
        }
        last_source_end = source_end;
        current.push(word);
    }
    if !current.is_empty() {
        cues.push(cue_from(current));
    }
    // Each cue lingers briefly but never overlaps the next one.
    for i in 0..cues.len() {
        let next_start = cues.get(i + 1).map(|c| c.start_us).unwrap_or(u64::MAX);
        let cue = &mut cues[i];
        cue.end_us = (cue.end_us + LINGER_US)
            .min(next_start)
            .max(cue.start_us + 1);
    }
    cues
}

fn cue_from(words: Vec<CueWord>) -> CaptionCue {
    CaptionCue {
        start_us: words[0].start_us,
        end_us: words.last().map(|w| w.end_us).unwrap_or(words[0].end_us),
        words,
    }
}

/// Which transcript captions read from: the chosen one, else the first transcribed of the
/// recording's microphones, then its system audio, then imported speech. `recorded` lists
/// the recording's audio track ids, microphones first; `imported` the imported speech ids.
pub fn caption_source(
    settings: &CaptionSettings,
    recorded: impl IntoIterator<Item = String>,
    imported: impl IntoIterator<Item = String>,
    load: impl Fn(&str) -> Option<Transcript>,
) -> Option<Transcript> {
    if let Some(id) = &settings.track_id {
        return load(id);
    }
    recorded
        .into_iter()
        .chain(imported)
        .find_map(|id| load(&id))
}

/// The cue on screen at `edited_us`, by index.
pub fn cue_at(cues: &[CaptionCue], edited_us: u64) -> Option<usize> {
    let i = cues.partition_point(|c| c.start_us <= edited_us);
    let index = i.checked_sub(1)?;
    (edited_us < cues[index].end_us).then_some(index)
}

/// How one word of a caption is drawn at a moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WordState {
    /// 0 is not drawn.
    pub alpha: f32,
    /// Its size as a share of its own, around its middle.
    pub scale: f32,
    /// How much of it, from the left, is lit (karaoke): 0 to 1.
    pub lit: f32,
}

impl WordState {
    const SHOWN: Self = Self {
        alpha: 1.0,
        scale: 1.0,
        lit: 1.0,
    };

    /// Rounded, so frames a hair apart compare equal and a still caption is drawn once.
    fn quantized(self) -> Self {
        Self {
            alpha: (self.alpha * 255.0).round() / 255.0,
            scale: (self.scale * 1000.0).round() / 1000.0,
            lit: (self.lit * 1000.0).round() / 1000.0,
        }
    }
}

/// Back-out easing: past 1 a little, then settles at 1.
fn back_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0) - 1.0;
    1.0 + (POP_OVERSHOOT + 1.0) * t * t * t + POP_OVERSHOOT * t * t
}

/// How each word of `cue` is drawn at `edited_us`.
pub fn word_states(
    cue: &CaptionCue,
    style: CaptionStyle,
    look: &CaptionLook,
    edited_us: u64,
) -> Vec<WordState> {
    cue.words
        .iter()
        .map(|word| {
            let since = edited_us as f32 - word.start_us as f32;
            match style {
                CaptionStyle::Static => WordState::SHOWN,
                CaptionStyle::Karaoke => WordState {
                    lit: (since / (word.end_us - word.start_us).max(1) as f32).clamp(0.0, 1.0),
                    ..WordState::SHOWN
                },
                CaptionStyle::Pop => {
                    let t = (since / look.pop_us).clamp(0.0, 1.0);
                    WordState {
                        alpha: if since < 0.0 { 0.0 } else { (t * 3.0).min(1.0) },
                        scale: look.pop_from + (1.0 - look.pop_from) * back_out(t),
                        lit: 1.0,
                    }
                }
            }
            .quantized()
        })
        .collect()
}

/// The bundled caption font at its usual weight, for other text the app draws.
pub(crate) fn font() -> &'static fontdue::Font {
    fonts::bundled(700)
}

/// One word drawn once: its text and outline coverage in a box of the cue's.
#[derive(Clone, Debug)]
struct WordRaster {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    fill: Vec<u8>,
    outline: Vec<u8>,
    /// Where its letters start and how wide they are, in its box (for karaoke's wipe).
    ink_x: f32,
    ink_w: f32,
    emphasis: bool,
    /// Its line, where it starts on it and the line's whole width: a line shown in part is
    /// centred on the words shown so far.
    line: usize,
    line_x: f32,
    line_w: f32,
}

/// A cue drawn once: each word's coverage and the background box's.
#[derive(Clone, Debug)]
pub struct CueRaster {
    pub width: u32,
    pub height: u32,
    /// The background box's coverage; empty without one.
    boxed: Vec<u8>,
    words: Vec<WordRaster>,
}

struct PlacedWord {
    index: usize,
    x: f32,
    width: f32,
    line: usize,
}

fn word_width(face: &Typeface, text: &str, px: f32) -> f32 {
    let mut width = 0.0;
    let mut previous: Option<(char, *const fontdue::Font)> = None;
    for ch in text.chars() {
        let font = face.font_for(ch);
        if let Some((prev, prev_font)) = previous {
            if std::ptr::eq(prev_font, font) {
                width += font.horizontal_kern(prev, ch, px).unwrap_or(0.0);
            }
        }
        width += font.metrics(ch, px).advance_width;
        previous = Some((ch, font));
    }
    width
}

/// A cue laid out for a canvas: its text size, where each word goes and the box it takes.
struct CueFit {
    px: f32,
    placed: Vec<PlacedWord>,
    line_widths: Vec<f32>,
    ascent: f32,
    line_height: f32,
    outline_px: f32,
    pad_y: f32,
    width: u32,
    height: u32,
}

/// The size `cue` takes on a canvas of `canvas_w` x `canvas_h`, without drawing it.
pub fn cue_size(
    cue: &CaptionCue,
    settings: &CaptionSettings,
    look: &CaptionLook,
    canvas_w: u32,
    canvas_h: u32,
) -> Option<(u32, u32)> {
    fit_cue(cue, settings, look, canvas_w, canvas_h).map(|fit| (fit.width, fit.height))
}

fn fit_cue(
    cue: &CaptionCue,
    settings: &CaptionSettings,
    look: &CaptionLook,
    canvas_w: u32,
    canvas_h: u32,
) -> Option<CueFit> {
    if cue.words.is_empty() || canvas_w < 16 || canvas_h < 16 {
        return None;
    }
    let face = &look.face;
    let main = face.main();
    let max_line = canvas_w as f32 * MAX_LINE_WIDTH;
    let max_lines = (settings.max_lines as usize).clamp(1, MAX_LINES);
    // Lines the words need at `px`, wrapping greedily.
    let lines_at = |px: f32| {
        let space = main.metrics(' ', px).advance_width;
        let mut lines = 1;
        let mut current = 0.0f32;
        for word in &cue.words {
            let width = word_width(face, &word.text, px);
            if current > 0.0 && current + space + width > max_line {
                lines += 1;
                current = width;
            } else {
                current += if current > 0.0 { space + width } else { width };
            }
        }
        lines
    };
    let wanted = (settings.font_size_pct / 100.0 * canvas_h as f32).clamp(6.0, 400.0);
    let mut px = wanted;
    // Too many lines for the limit: smaller text, down to half size.
    while lines_at(px) > max_lines && px > (wanted * 0.5).max(6.0) {
        px *= 0.92;
    }
    let space = main.metrics(' ', px).advance_width;
    let line_metrics = main.horizontal_line_metrics(px)?;
    let ascent = line_metrics.ascent;
    let line_height = (line_metrics.ascent - line_metrics.descent) * 1.12;

    // Greedy wrap; the widths of each line center it later.
    let mut placed = Vec::with_capacity(cue.words.len());
    let mut line_widths = vec![0.0f32];
    for (index, word) in cue.words.iter().enumerate() {
        let width = word_width(face, &word.text, px);
        let line = line_widths.len() - 1;
        let current = line_widths[line];
        let x = if current == 0.0 {
            0.0
        } else if current + space + width <= max_line || line_widths.len() >= max_lines {
            current + space
        } else {
            line_widths.push(0.0);
            0.0
        };
        let line = line_widths.len() - 1;
        line_widths[line] = x + width;
        placed.push(PlacedWord {
            index,
            x,
            width,
            line,
        });
    }
    let text_w = line_widths.iter().cloned().fold(0.0f32, f32::max);
    let outline_px = if settings.outline {
        (px * 0.085).clamp(1.0, 12.0)
    } else {
        0.0
    };
    // Room for a popping word to grow past its size.
    let pop = if settings.style == CaptionStyle::Pop {
        let widest = placed.iter().map(|p| p.width).fold(0.0f32, f32::max);
        (widest * 0.04).max(px * 0.1)
    } else {
        0.0
    };
    let pad_x = if settings.background { px * 0.45 } else { 0.0 } + outline_px + 2.0 + pop;
    let pad_y = if settings.background { px * 0.22 } else { 0.0 } + outline_px + 2.0 + pop;
    let width = ((text_w + pad_x * 2.0).ceil() as u32).clamp(1, canvas_w);
    let height =
        ((line_height * line_widths.len() as f32 + pad_y * 2.0).ceil() as u32).clamp(1, canvas_h);
    Some(CueFit {
        px,
        placed,
        line_widths,
        ascent,
        line_height,
        outline_px,
        pad_y,
        width,
        height,
    })
}

/// Lays out and rasterizes `cue` for a canvas of `canvas_w` x `canvas_h`.
pub fn rasterize_cue(
    cue: &CaptionCue,
    settings: &CaptionSettings,
    look: &CaptionLook,
    canvas_w: u32,
    canvas_h: u32,
) -> Option<CueRaster> {
    let CueFit {
        px,
        placed,
        line_widths,
        ascent,
        line_height,
        outline_px,
        pad_y,
        width,
        height,
    } = fit_cue(cue, settings, look, canvas_w, canvas_h)?;
    let face = &look.face;
    // Each word's box holds its outline and a little more.
    let margin = outline_px.ceil() + 2.0;
    let mut words = Vec::with_capacity(placed.len());
    for word in &placed {
        let line_w = line_widths[word.line];
        let origin_x = (width as f32 - line_w) / 2.0 + word.x;
        let top = pad_y + line_height * word.line as f32;
        let box_x = (origin_x - margin).floor();
        let box_y = (top - margin).floor();
        let (bw, bh) = (
            (word.width + margin * 2.0 + 2.0).ceil() as u32,
            (line_height + margin * 2.0 + 2.0).ceil() as u32,
        );
        let ink_x = origin_x - box_x;
        let baseline = top - box_y + ascent;
        let mut fill = vec![0u8; (bw * bh) as usize];
        let mut pen = ink_x;
        let mut previous: Option<(char, *const fontdue::Font)> = None;
        for ch in cue.words[word.index].text.chars() {
            let font = face.font_for(ch);
            if let Some((prev, prev_font)) = previous {
                if std::ptr::eq(prev_font, font) {
                    pen += font.horizontal_kern(prev, ch, px).unwrap_or(0.0);
                }
            }
            let (metrics, bitmap) = font.rasterize(ch, px);
            let gx = (pen + metrics.xmin as f32).round() as i64;
            let gy = (baseline - metrics.height as f32 - metrics.ymin as f32).round() as i64;
            for row in 0..metrics.height {
                let y = gy + row as i64;
                if y < 0 || y >= bh as i64 {
                    continue;
                }
                for col in 0..metrics.width {
                    let x = gx + col as i64;
                    if x < 0 || x >= bw as i64 {
                        continue;
                    }
                    let coverage = bitmap[row * metrics.width + col];
                    let i = y as usize * bw as usize + x as usize;
                    if coverage > fill[i] {
                        fill[i] = coverage;
                    }
                }
            }
            pen += metrics.advance_width;
            previous = Some((ch, font));
        }
        let outline = if outline_px > 0.0 {
            dilate(&fill, bw, bh, outline_px)
        } else {
            Vec::new()
        };
        words.push(WordRaster {
            x: box_x as i32,
            y: box_y as i32,
            width: bw,
            height: bh,
            fill,
            outline,
            ink_x,
            ink_w: word.width.max(1.0),
            emphasis: cue.words[word.index].emphasis,
            line: word.line,
            line_x: word.x,
            line_w,
        });
    }
    let boxed = if settings.background {
        rounded_box(width, height, px * 0.3)
    } else {
        Vec::new()
    };
    Some(CueRaster {
        width,
        height,
        boxed,
        words,
    })
}

/// Coverage of a rounded box filling `width` x `height`, anti-aliased at the corners.
fn rounded_box(width: u32, height: u32, radius: f32) -> Vec<u8> {
    let (w, h) = (width as f32, height as f32);
    let radius = radius.min(w / 2.0).min(h / 2.0);
    let (half_w, half_h) = (w / 2.0, h / 2.0);
    let mut out = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let (qx, qy) = (
                (x as f32 + 0.5 - half_w).abs() - (half_w - radius),
                (y as f32 + 0.5 - half_h).abs() - (half_h - radius),
            );
            let outside =
                (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - radius;
            out[(y * width + x) as usize] = ((0.5 - outside).clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    out
}

/// Coverage grown by `radius` pixels in every direction (a disc), for the outline.
fn dilate(mask: &[u8], width: u32, height: u32, radius: f32) -> Vec<u8> {
    let (w, h) = (width as i64, height as i64);
    let r = radius.ceil() as i64;
    let offsets: Vec<(i64, i64, f32)> = (-r..=r)
        .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
        .filter_map(|(dx, dy)| {
            let distance = ((dx * dx + dy * dy) as f32).sqrt();
            // Soft edge over the last pixel keeps the outline anti-aliased.
            let weight = (radius + 0.5 - distance).clamp(0.0, 1.0);
            (weight > 0.0).then_some((dx, dy, weight))
        })
        .collect();
    let mut out = vec![0u8; mask.len()];
    for y in 0..h {
        for x in 0..w {
            let source = mask[(y * w + x) as usize];
            if source == 0 {
                continue;
            }
            for &(dx, dy, weight) in &offsets {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let value = (source as f32 * weight).round() as u8;
                let i = (ny * w + nx) as usize;
                if value > out[i] {
                    out[i] = value;
                }
            }
        }
    }
    out
}

fn over(dst: [f32; 4], src: [f32; 3], alpha: f32) -> [f32; 4] {
    // Straight-alpha "source over destination".
    let out_a = alpha + dst[3] * (1.0 - alpha);
    if out_a <= 0.0 {
        return [0.0; 4];
    }
    let mix = |s: f32, d: f32| (s * alpha + d * dst[3] * (1.0 - alpha)) / out_a;
    [
        mix(src[0], dst[0]),
        mix(src[1], dst[1]),
        mix(src[2], dst[2]),
        out_a,
    ]
}

fn rgb(hex: &str) -> [f32; 3] {
    let [r, g, b] = parse_hex_rgb(hex).unwrap_or([255, 255, 255]);
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

/// Coverage of `mask` (`w` x `h`) at a point, read between pixels.
fn sample(mask: &[u8], w: u32, h: u32, x: f32, y: f32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |xi: f32, yi: f32| -> f32 {
        if xi < 0.0 || yi < 0.0 || xi >= w as f32 || yi >= h as f32 {
            0.0
        } else {
            mask[yi as usize * w as usize + xi as usize] as f32
        }
    };
    if fx == 0.0 && fy == 0.0 {
        return at(x0, y0) / 255.0;
    }
    let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1.0, y0) * fx;
    let bottom = at(x0, y0 + 1.0) * (1.0 - fx) + at(x0 + 1.0, y0 + 1.0) * fx;
    (top * (1.0 - fy) + bottom * fy) / 255.0
}

/// Lays `mask` of `word` over `out` (`width` x `height`), grown by `scale` around its middle;
/// `shade` gives the colour and opacity at a point of the word's own box (its x).
fn paint(
    out: &mut [[f32; 4]],
    width: u32,
    height: u32,
    word: &WordRaster,
    mask: &[u8],
    scale: f32,
    dx: f32,
    shade: impl Fn(f32) -> ([f32; 3], f32),
) {
    if mask.is_empty() || scale <= 0.01 {
        return;
    }
    let (half_w, half_h) = (word.width as f32 / 2.0, word.height as f32 / 2.0);
    let (cx, cy) = (word.x as f32 + dx + half_w, word.y as f32 + half_h);
    let x0 = ((cx - half_w * scale).floor() as i64).max(0);
    let x1 = ((cx + half_w * scale).ceil() as i64).min(width as i64);
    let y0 = ((cy - half_h * scale).floor() as i64).max(0);
    let y1 = ((cy + half_h * scale).ceil() as i64).min(height as i64);
    for y in y0..y1 {
        let sy = (y as f32 + 0.5 - cy) / scale + half_h - 0.5;
        for x in x0..x1 {
            let sx = (x as f32 + 0.5 - cx) / scale + half_w - 0.5;
            let coverage = sample(mask, word.width, word.height, sx, sy);
            if coverage <= 0.0 {
                continue;
            }
            let (color, alpha) = shade(sx);
            let i = y as usize * width as usize + x as usize;
            out[i] = over(out[i], color, coverage * alpha);
        }
    }
}

/// Colors `raster` into a straight-alpha BGRA frame with each word as `states` says.
pub fn colorize(
    raster: &CueRaster,
    settings: &CaptionSettings,
    look: &CaptionLook,
    states: &[WordState],
) -> VideoFrame {
    let (w, h) = (raster.width, raster.height);
    let text = rgb(&settings.text_color);
    let accent = rgb(&look.accent);
    let background = rgb(&settings.background_color);
    let outline_color = [0.0, 0.0, 0.0];
    let mut px = vec![[0.0f32; 4]; (w * h) as usize];
    if !raster.boxed.is_empty() {
        for (p, &c) in px.iter_mut().zip(&raster.boxed) {
            if c > 0 {
                *p = over(
                    *p,
                    background,
                    settings.background_opacity * c as f32 / 255.0,
                );
            }
        }
    }
    let shown = |i: usize| states.get(i).copied().unwrap_or(WordState::SHOWN);
    // A line whose later words are still to come is centred on the words shown so far (as
    // popping captions do), moving by whole pixels so still words stay sharp.
    let mut shown_end: Vec<f32> = Vec::new();
    for (i, word) in raster.words.iter().enumerate() {
        if shown_end.len() <= word.line {
            shown_end.resize(word.line + 1, 0.0);
        }
        if shown(i).alpha > 0.0 {
            shown_end[word.line] = shown_end[word.line].max(word.line_x + word.ink_w);
        }
    }
    let shift = |word: &WordRaster| {
        ((word.line_w - shown_end[word.line]) / 2.0)
            .max(0.0)
            .round()
    };
    // Every outline goes under every word's letters.
    for (i, word) in raster.words.iter().enumerate() {
        let state = shown(i);
        if state.alpha > 0.0 {
            paint(
                &mut px,
                w,
                h,
                word,
                &word.outline,
                state.scale,
                shift(word),
                |_| (outline_color, 0.9 * state.alpha),
            );
        }
    }
    for (i, word) in raster.words.iter().enumerate() {
        let state = shown(i);
        if state.alpha <= 0.0 {
            continue;
        }
        let color = if settings.emphasis && word.emphasis {
            accent
        } else {
            text
        };
        // Lit up to here, with a soft edge of a pixel and a half. The emphasis word is bright
        // before it is said, so its accent reads on a dark screen.
        let edge = word.ink_x + state.lit * word.ink_w;
        let bright = settings.emphasis && word.emphasis;
        paint(
            &mut px,
            w,
            h,
            word,
            &word.fill,
            state.scale,
            shift(word),
            |sx| {
                let lit = if state.lit >= 1.0 || bright {
                    1.0
                } else if state.lit <= 0.0 {
                    0.0
                } else {
                    ((edge - sx) / 1.5 + 0.5).clamp(0.0, 1.0)
                };
                (color, state.alpha * (UNSAID + (1.0 - UNSAID) * lit))
            },
        );
    }
    let mut data = vec![0u8; (w * h * 4) as usize];
    for (p, o) in px.iter().zip(data.chunks_exact_mut(4)) {
        o[0] = (p[2] * 255.0).round() as u8;
        o[1] = (p[1] * 255.0).round() as u8;
        o[2] = (p[0] * 255.0).round() as u8;
        o[3] = (p[3] * 255.0).round() as u8;
    }
    VideoFrame {
        pts_us: 0,
        width: w,
        height: h,
        stride: w * 4,
        format: PixelFormat::Bgra8888,
        color: ColorInfo::rec709_full(),
        data: data.into(),
    }
}

/// A keyboard shortcut badge ("Ctrl + Shift + P") for a canvas, drawn like a caption in a
/// dark rounded box.
pub fn rasterize_badge(keys: &[String], canvas_w: u32, canvas_h: u32) -> Option<VideoFrame> {
    let mut words = Vec::new();
    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            words.push("+".to_string());
        }
        words.push(key.clone());
    }
    let cue = CaptionCue {
        start_us: 0,
        end_us: 1,
        words: words
            .into_iter()
            .enumerate()
            .map(|(i, text)| CueWord {
                id: i.to_string(),
                text,
                start_us: 0,
                end_us: 1,
                emphasis: false,
            })
            .collect(),
    };
    let settings = CaptionSettings {
        enabled: true,
        font_size_pct: 3.4,
        text_color: "#f8fafc".into(),
        style: CaptionStyle::Static,
        outline: false,
        background: true,
        background_color: "#0f172a".into(),
        background_opacity: 0.86,
        max_lines: 1,
        weight: 700,
        ..CaptionSettings::default()
    };
    let look = CaptionLook::plain(&settings);
    // The short side sets the size, so a 9:16 short's badge is as big as a video's.
    let short = canvas_w.min(canvas_h);
    let raster = rasterize_cue(&cue, &settings, &look, canvas_w, short)?;
    Some(colorize(&raster, &settings, &look, &[]))
}

/// Where a caption of `w` x `h` sits on the canvas.
pub fn placement(
    settings: &CaptionSettings,
    canvas_w: u32,
    canvas_h: u32,
    w: u32,
    h: u32,
) -> (u32, u32) {
    let x = canvas_w.saturating_sub(w) / 2;
    let offset = (settings.offset_pct / 100.0 * canvas_h as f32).round() as u32;
    let free = canvas_h.saturating_sub(h);
    let y = match settings.position.as_str() {
        "top" => offset.min(free),
        "middle" => free / 2,
        _ => free.saturating_sub(offset),
    };
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::SourceInterval;
    use crate::transcript::{test_word, ProviderKind};

    #[test]
    fn the_caption_font_draws_greek() {
        let font = font();
        for c in "αβγδεζηθικλμνξοπρσςτυφχψωάέήίόύώϊϋΐΰΑΩΆΈΉΊΌΎΏ;·".chars()
        {
            assert_ne!(font.lookup_glyph_index(c), 0, "no glyph for {c}");
        }
    }

    fn transcript(words: &[(&str, u64, u64)]) -> Transcript {
        Transcript::new(
            "mic".into(),
            ProviderKind::ElevenLabs,
            "scribe_v2".into(),
            None,
            words.iter().map(|(t, s, e)| test_word(t, *s, *e)).collect(),
        )
    }

    fn mapper(ranges: &[(u64, u64)]) -> TimelineMapper {
        TimelineMapper::try_new(
            ranges
                .iter()
                .enumerate()
                .map(|(i, (s, e))| SourceInterval::new(format!("r{i}"), s * 1000, e * 1000))
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn caption_track_edits_split_merge_hide_retext_and_retime() {
        // Times in ms: "one two three four" in 1 s, then a pause and "five".
        let mut t = transcript(&[
            ("one", 0, 200),
            ("two", 250, 450),
            ("three", 500, 700),
            ("four", 750, 950),
            ("five", 2000, 2200),
        ]);
        let map = mapper(&[(0, 3000)]);
        let settings = CaptionSettings::default();
        let texts = |t: &Transcript| -> Vec<String> {
            build_cues(t, &map, &settings)
                .iter()
                .map(|c| {
                    c.words
                        .iter()
                        .map(|w| w.text.clone())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect()
        };
        assert_eq!(texts(&t), ["one two three four", "five"]);

        // Split before "three"; merge "five" back over the pause.
        t.set_caption_mark("w-2", |m| m.cue_break = true).unwrap();
        t.set_caption_mark("w-4", |m| m.cue_join = true).unwrap();
        assert_eq!(texts(&t), ["one two", "three four five"]);
        // Hide "two": still heard, not shown.
        t.set_caption_mark("w-1", |m| m.hidden = true).unwrap();
        assert_eq!(texts(&t), ["one", "three four five"]);
        t.set_caption_mark("w-1", |m| m.hidden = false).unwrap();
        assert!(
            !t.caption_marks.contains_key("w-1"),
            "an all-off mark is dropped"
        );
        // Emphasis reaches the cue's word.
        t.set_caption_mark("w-3", |m| m.emphasis = true).unwrap();
        let cues = build_cues(&t, &map, &settings);
        assert!(cues[1].words[1].emphasis && !cues[1].words[0].emphasis);

        // Same word count keeps the timing; a different count shares the old span.
        t.replace_words(&["w-0".into(), "w-1".into()], "One, two")
            .unwrap();
        assert_eq!(texts(&t)[0], "One, two");
        t.replace_words(&["w-0".into(), "w-1".into()], "Hello there everyone")
            .unwrap();
        assert_eq!(texts(&t)[0], "Hello there everyone");
        let words: Vec<_> = t
            .words
            .iter()
            .take(3)
            .map(|w| (w.source_start_us, w.source_end_us))
            .collect();
        assert_eq!(words.first().unwrap().0, 0);
        assert_eq!(words.last().unwrap().1, 450_000);
        t.validate().unwrap();

        // Retime "five" later; it cannot jump back over "four".
        let five = t.words.last().unwrap().id.clone();
        t.retime_words(&[five.clone()], 2_400_000, 2_800_000)
            .unwrap();
        assert_eq!(
            (
                t.words.last().unwrap().source_start_us,
                t.words.last().unwrap().source_end_us
            ),
            (2_400_000, 2_800_000)
        );
        assert!(t.retime_words(&[five], 100_000, 300_000).is_err());
    }

    #[test]
    fn defaults_validate_and_bad_values_fail() {
        CaptionSettings::default().validate().unwrap();
        let bad = CaptionSettings {
            font_size_pct: 40.0,
            ..Default::default()
        };
        assert!(bad.validate().is_err());
        for bad_key in ["../x", "a..b", "nodot", ".sound0", "a.", "a/b.sound0"] {
            let bad = CaptionSettings {
                track_id: Some(bad_key.into()),
                ..Default::default()
            };
            assert!(bad.validate().is_err(), "{bad_key}");
        }
        // Sounds are named `<asset>.<stream>`, as the captions inspector sends them.
        CaptionSettings {
            track_id: Some("rec-1.mic_0".into()),
            ..Default::default()
        }
        .validate()
        .unwrap();
        // How they look is the style's, never stored with them.
        let stored = serde_json::to_value(CaptionSettings::default()).unwrap();
        assert!(stored.get("style").is_none() && stored.get("textColor").is_none());
    }

    #[test]
    fn reordered_clips_give_sorted_cues_that_never_span_the_jump() {
        let t = transcript(&[
            ("early", 0, 300),
            ("words", 350, 600),
            ("late", 1000, 1300),
            ("words", 1350, 1600),
        ]);
        // Source 1000-2000 plays first, then 0-1000.
        let m = mapper(&[(1000, 2000), (0, 1000)]);
        let cues = build_cues(&t, &m, &CaptionSettings::default());
        assert!(cues.windows(2).all(|w| w[0].start_us < w[1].start_us));
        let texts: Vec<Vec<&str>> = cues
            .iter()
            .map(|c| c.words.iter().map(|w| w.text.as_str()).collect())
            .collect();
        assert_eq!(texts, vec![vec!["late", "words"], vec!["early", "words"]]);
        assert_eq!(cue_at(&cues, 100_000), Some(0));
        assert_eq!(cue_at(&cues, 1_100_000), Some(1));
    }

    #[test]
    fn cues_break_on_sentences_pauses_and_word_count() {
        let t = transcript(&[
            ("Hello", 0, 300),
            ("there.", 350, 600),
            ("This", 700, 900),
            ("is", 950, 1000),
            ("a", 1050, 1100),
            ("test", 1150, 1400),
            // long pause
            ("Again", 3000, 3300),
        ]);
        let settings = CaptionSettings {
            max_words: 3,
            ..Default::default()
        };
        let cues = build_cues(&t, &mapper(&[(0, 4000)]), &settings);
        let texts: Vec<Vec<&str>> = cues
            .iter()
            .map(|c| c.words.iter().map(|w| w.text.as_str()).collect())
            .collect();
        assert_eq!(
            texts,
            vec![
                vec!["Hello", "there."],
                vec!["This", "is", "a"],
                vec!["test"],
                vec!["Again"]
            ]
        );
        // Lingering stops where the next cue starts.
        assert_eq!(cues[0].end_us, 700_000);
        assert_eq!(cues[2].end_us, 1_800_000);
    }

    #[test]
    fn a_greek_question_mark_ends_a_caption() {
        let t = transcript(&[("Τι", 0, 200), ("κάνεις;", 250, 600), ("Καλά", 700, 900)]);
        let cues = build_cues(&t, &mapper(&[(0, 2000)]), &CaptionSettings::default());
        assert_eq!(cues.len(), 2);
    }

    #[test]
    fn cut_words_leave_the_captions_and_times_follow_the_edit() {
        let t = transcript(&[("one", 0, 400), ("um", 500, 700), ("two", 800, 1200)]);
        // "um" cut: source 450-750 removed.
        let cues = build_cues(
            &t,
            &mapper(&[(0, 450), (750, 2000)]),
            &CaptionSettings::default(),
        );
        assert_eq!(cues.len(), 1);
        let words: Vec<_> = cues[0].words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(words, vec!["one", "two"]);
        assert_eq!(cues[0].words[1].start_us, 500_000);
        assert_eq!(cue_at(&cues, 600_000), Some(0));
        assert_eq!(cue_at(&cues, 5_000_000), None);
    }

    /// Opaque pixels of `frame` close to `rgb`.
    fn count(frame: &VideoFrame, rgb: [u8; 3]) -> usize {
        frame
            .data
            .chunks_exact(4)
            .filter(|p| {
                p[3] == 255
                    && (p[2] as i32 - rgb[0] as i32).abs() < 40
                    && (p[1] as i32 - rgb[1] as i32).abs() < 40
                    && (p[0] as i32 - rgb[2] as i32).abs() < 40
            })
            .count()
    }

    #[test]
    fn the_emphasis_word_takes_the_accent_and_the_outline_stays_dark() {
        let mut t = transcript(&[("Hi", 0, 300), ("you", 350, 600)]);
        t.set_caption_mark("w-1", |m| m.emphasis = true).unwrap();
        let settings = CaptionSettings {
            text_color: "#00FF00".into(),
            emphasis_color: "#FF0000".into(),
            style: CaptionStyle::Static,
            ..Default::default()
        };
        let cues = build_cues(&t, &mapper(&[(0, 1000)]), &settings);
        let look = CaptionLook::plain(&settings);
        let raster = rasterize_cue(&cues[0], &settings, &look, 1280, 720).unwrap();
        assert!(raster.width > 40 && raster.height > 20);
        let states = word_states(&cues[0], settings.style, &look, 0);
        let frame = colorize(&raster, &settings, &look, &states);
        assert!(count(&frame, [255, 0, 0]) > 50, "the emphasis word is red");
        assert!(count(&frame, [0, 255, 0]) > 50, "the other is green");
        let pixels: Vec<&[u8]> = frame.data.chunks_exact(4).collect();
        assert!(pixels
            .iter()
            .any(|p| p[3] > 100 && p[0] < 10 && p[1] < 10 && p[2] < 10));
        // Corners stay transparent without a background box.
        assert_eq!(pixels[0][3], 0);
        let (x, y) = placement(&settings, 1280, 720, raster.width, raster.height);
        assert!(y + raster.height <= 720 && y > 360);
        assert_eq!(x, (1280 - raster.width) / 2);
        // Emphasis off: both words in the text colour.
        let plain = CaptionSettings {
            emphasis: false,
            ..settings.clone()
        };
        let frame = colorize(&raster, &plain, &look, &states);
        assert_eq!(count(&frame, [255, 0, 0]), 0);
        // Without its own colour, the emphasis is the style's accent.
        let mut style = crate::style::Style::default();
        style.brand.colors.accent = "#123456".into();
        let look = CaptionLook::new(&CaptionSettings::default(), &style, &Default::default());
        assert_eq!(look.accent, "#123456");
    }

    #[test]
    fn a_popping_line_is_centred_and_the_emphasis_word_is_bright_before_it_is_said() {
        let mut t = transcript(&[
            ("one", 0, 400),
            ("wonderful", 500, 900),
            ("two", 1000, 1300),
        ]);
        t.set_caption_mark("w-2", |m| m.emphasis = true).unwrap();
        let settings = CaptionSettings {
            style: CaptionStyle::Pop,
            emphasis_color: "#FF0000".into(),
            ..Default::default()
        };
        let cues = build_cues(&t, &mapper(&[(0, 2000)]), &settings);
        let look = CaptionLook::plain(&settings);
        let raster = rasterize_cue(&cues[0], &settings, &look, 1280, 720).unwrap();
        // Ink across the frame's columns: its middle sits at the caption's middle.
        let middle = |frame: &VideoFrame| {
            let columns: Vec<u32> = (0..frame.width)
                .filter(|&x| {
                    (0..frame.height)
                        .any(|y| frame.data[((y * frame.width + x) * 4 + 3) as usize] > 128)
                })
                .collect();
            (columns[0] + columns[columns.len() - 1]) as f32 / 2.0
        };
        for at in [450_000, 950_000, 1_500_000] {
            let states = word_states(&cues[0], CaptionStyle::Pop, &look, at);
            let frame = colorize(&raster, &settings, &look, &states);
            assert!(
                (middle(&frame) - raster.width as f32 / 2.0).abs() < 3.0,
                "centred at {at}: {} of {}",
                middle(&frame),
                raster.width
            );
        }

        // Karaoke before "two" is said: the other words dim, "two" in its full accent.
        let karaoke = CaptionSettings {
            style: CaptionStyle::Karaoke,
            ..settings
        };
        let raster = rasterize_cue(&cues[0], &karaoke, &look, 1280, 720).unwrap();
        let states = word_states(&cues[0], CaptionStyle::Karaoke, &look, 0);
        let frame = colorize(&raster, &karaoke, &look, &states);
        assert!(
            count(&frame, [255, 0, 0]) > 50,
            "the emphasis word is bright red"
        );
        assert_eq!(count(&frame, [255, 255, 255]), 0, "the others are dim");
    }

    #[test]
    fn the_three_styles_over_time() {
        let t = transcript(&[("one", 0, 400), ("two", 500, 900)]);
        let settings = CaptionSettings::default();
        let cues = build_cues(&t, &mapper(&[(0, 2000)]), &settings);
        let look = CaptionLook::plain(&settings);
        let at = |style, us| word_states(&cues[0], style, &look, us);

        // Static: everything, all the time.
        assert!(at(CaptionStyle::Static, 0)
            .iter()
            .all(|s| *s == WordState::SHOWN));
        // Karaoke: all shown; "one" half lit at 200 ms, "two" not yet.
        let k = at(CaptionStyle::Karaoke, 200_000);
        assert_eq!((k[0].alpha, k[0].lit, k[1].lit), (1.0, 0.5, 0.0));
        assert_eq!(at(CaptionStyle::Karaoke, 950_000)[1].lit, 1.0);
        // Pop: "two" hidden until said, small as it starts, past its size, then settled.
        let p = at(CaptionStyle::Pop, 30_000);
        assert_eq!(at(CaptionStyle::Pop, 450_000)[1].alpha, 0.0);
        assert!(p[0].alpha > 0.0 && p[0].scale < 1.0);
        let peak = (0..24)
            .map(|i| at(CaptionStyle::Pop, i * 10_000)[0].scale)
            .fold(0.0f32, f32::max);
        assert!(peak > 1.0 && peak < 1.05, "{peak}");
        assert_eq!(at(CaptionStyle::Pop, 400_000)[0], WordState::SHOWN);
        // The look's speed makes it quicker.
        let quick = CaptionLook::new(
            &settings,
            &crate::style::Style::default(),
            &crate::graphics::look::Look {
                speed: 2.0,
                ..Default::default()
            },
        );
        assert_eq!(
            word_states(&cues[0], CaptionStyle::Pop, &quick, 130_000)[0],
            WordState::SHOWN
        );

        // Drawn: a karaoke caption half way is dimmer on the right of "one" than its left.
        let raster = rasterize_cue(&cues[0], &settings, &look, 1280, 720).unwrap();
        let frame = colorize(&raster, &settings, &look, &k);
        let word = &raster.words[0];
        let brightest = |x0: i32, x1: i32| {
            let mut best = 0u8;
            for y in word.y.max(0)..(word.y + word.height as i32).min(raster.height as i32) {
                for x in x0.max(0)..x1.min(raster.width as i32) {
                    let i = (y as usize * raster.width as usize + x as usize) * 4;
                    best = best.max(frame.data[i + 1]);
                }
            }
            best
        };
        let left = word.x + word.ink_x as i32;
        let mid = left + (word.ink_w / 2.0) as i32;
        let right = left + word.ink_w as i32;
        assert!(brightest(left, mid - 2) > 240);
        assert!(brightest(mid + 3, right) < 200);

        // A popping word grows inside the caption's box and never past it.
        let pop = CaptionSettings {
            style: CaptionStyle::Pop,
            ..settings.clone()
        };
        let raster = rasterize_cue(&cues[0], &pop, &look, 1280, 720).unwrap();
        let frame = colorize(&raster, &pop, &look, &at(CaptionStyle::Pop, 120_000));
        assert!(frame.data.chunks_exact(4).any(|p| p[3] > 0));
    }

    #[test]
    fn greek_with_tonos_and_final_sigma_draws_in_every_style() {
        let t = transcript(&[
            ("Καλημέρα", 0, 400),
            ("φίλοι", 450, 800),
            ("μας;", 850, 1100),
        ]);
        for style in [
            CaptionStyle::Pop,
            CaptionStyle::Karaoke,
            CaptionStyle::Static,
        ] {
            let settings = CaptionSettings {
                style,
                uppercase: style == CaptionStyle::Static,
                ..Default::default()
            };
            let cues = build_cues(&t, &mapper(&[(0, 2000)]), &settings);
            let look = CaptionLook::plain(&settings);
            for (w, h) in [(1920, 1080), (1080, 1920)] {
                let raster = rasterize_cue(&cues[0], &settings, &look, w, h).unwrap();
                assert!(raster.width <= w);
                let frame = colorize(
                    &raster,
                    &settings,
                    &look,
                    &word_states(&cues[0], style, &look, 1_000_000),
                );
                assert!(count(&frame, [255, 255, 255]) > 200, "{style:?} {w}x{h}");
            }
        }
    }

    /// `AEROEDITS_CAPTION_PROBE=<folder> cargo test --lib caption_sheet_probe -- --ignored`:
    /// writes a sheet per canvas (16:9, 9:16) with each style mid-word, English and Greek, an
    /// emphasis word in each, over a screen-like background, to look at by eye.
    #[test]
    #[ignore]
    fn caption_sheet_probe() {
        let Some(out) = std::env::var_os("AEROEDITS_CAPTION_PROBE") else {
            return;
        };
        let lines: [(&str, &[(&str, u64, u64)], &str); 2] = [
            (
                "en",
                &[
                    ("This", 0, 200),
                    ("one", 250, 450),
                    ("setting", 500, 900),
                    ("doubles", 950, 1400),
                    ("your", 1450, 1600),
                    ("speed.", 1650, 2100),
                ],
                "w-3",
            ),
            (
                "el",
                &[
                    ("Αυτή", 0, 250),
                    ("η", 300, 350),
                    ("ρύθμιση", 400, 900),
                    ("διπλασιάζει", 950, 1600),
                    ("την", 1650, 1750),
                    ("ταχύτητά", 1800, 2300),
                    ("σας;", 2350, 2700),
                ],
                "w-3",
            ),
        ];
        let styles = [
            (CaptionStyle::Pop, 1_050_000u64),
            (CaptionStyle::Karaoke, 1_200_000),
            (CaptionStyle::Static, 1_200_000),
        ];
        for (canvas_w, canvas_h, name) in [(1920u32, 1080u32, "16x9"), (1080, 1920, "9x16")] {
            let (tile_w, tile_h) = (canvas_w / 2, canvas_h / 2);
            let mut sheet = image::RgbaImage::new(tile_w * 2, tile_h * 3);
            for (row, (style, at)) in styles.iter().enumerate() {
                for (col, (_, words, emphasis)) in lines.iter().enumerate() {
                    let mut t = transcript(words);
                    t.set_caption_mark(emphasis, |m| m.emphasis = true).unwrap();
                    let settings = CaptionSettings {
                        style: *style,
                        max_words: if canvas_w < canvas_h { 4 } else { 7 },
                        max_lines: 2,
                        font_size_pct: if canvas_w < canvas_h { 4.2 } else { 5.5 },
                        ..Default::default()
                    };
                    let look = CaptionLook::plain(&settings);
                    let cues = build_cues(&t, &mapper(&[(0, 4000)]), &settings);
                    let index = cue_at(&cues, *at).unwrap();
                    let raster =
                        rasterize_cue(&cues[index], &settings, &look, canvas_w, canvas_h).unwrap();
                    let states = word_states(&cues[index], *style, &look, *at);
                    let frame = colorize(&raster, &settings, &look, &states);
                    let (x, y) =
                        placement(&settings, canvas_w, canvas_h, frame.width, frame.height);
                    // A light app window on a darker desktop, like a screen recording.
                    let mut canvas = image::RgbaImage::from_fn(canvas_w, canvas_h, |px, py| {
                        let inside = px > canvas_w / 12
                            && px < canvas_w * 11 / 12
                            && py > canvas_h / 10
                            && py < canvas_h * 8 / 10;
                        if inside {
                            image::Rgba([236, 238, 242, 255])
                        } else {
                            let v = 40 + (py * 30 / canvas_h) as u8;
                            image::Rgba([v, v + 6, v + 18, 255])
                        }
                    });
                    for (i, p) in frame.data.chunks_exact(4).enumerate() {
                        let (fx, fy) = (i as u32 % frame.width, i as u32 / frame.width);
                        let dst = canvas.get_pixel_mut(x + fx, y + fy);
                        let a = p[3] as u32;
                        for (c, s) in [(0, p[2]), (1, p[1]), (2, p[0])] {
                            dst.0[c] = ((s as u32 * a + dst.0[c] as u32 * (255 - a)) / 255) as u8;
                        }
                    }
                    let small = image::imageops::resize(
                        &canvas,
                        tile_w,
                        tile_h,
                        image::imageops::FilterType::Triangle,
                    );
                    image::imageops::overlay(
                        &mut sheet,
                        &small,
                        (col as u32 * tile_w) as i64,
                        (row as u32 * tile_h) as i64,
                    );
                }
            }
            let path = std::path::Path::new(&out).join(format!("captions-{name}.png"));
            sheet.save(&path).unwrap();
            eprintln!("wrote {}", path.display());
        }
    }

    #[test]
    fn long_captions_wrap_onto_more_lines() {
        let words: Vec<(&str, u64, u64)> = (0..10)
            .map(|i| ("wonderful", i * 100, i * 100 + 90))
            .collect();
        let t = transcript(&words);
        let settings = CaptionSettings {
            max_words: 10,
            font_size_pct: 10.0,
            max_lines: 3,
            ..Default::default()
        };
        let look = CaptionLook::plain(&settings);
        let cues = build_cues(&t, &mapper(&[(0, 2000)]), &settings);
        let one_line = rasterize_cue(
            &CaptionCue {
                words: cues[0].words[..1].to_vec(),
                ..cues[0].clone()
            },
            &settings,
            &look,
            1280,
            720,
        )
        .unwrap();
        let wrapped = rasterize_cue(&cues[0], &settings, &look, 1280, 720).unwrap();
        // Several lines, drawn smaller where three would not hold it, never past the edge.
        assert!(wrapped.height > one_line.height * 3 / 2);
        assert!(wrapped.width <= 1280);

        // One line allowed: the same words on one line, smaller.
        let single = CaptionSettings {
            max_lines: 1,
            ..settings.clone()
        };
        let one = rasterize_cue(&cues[0], &single, &look, 1280, 720).unwrap();
        assert!(one.height < wrapped.height);
    }
}
