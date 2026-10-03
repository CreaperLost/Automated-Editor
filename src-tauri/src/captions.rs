//! Captions burned into preview and export. The words come from a track's transcript, so
//! captions follow every cut and undo; the style lives in the edit document.
//!
//! Cues are groups of a few kept words in edited time. Each cue is rasterized once with the
//! embedded font into a coverage mask, then colored for the word being spoken.
use crate::media::{ColorInfo, PixelFormat, VideoFrame};
use crate::project::layout::parse_hex_rgb;
use crate::timeline::TimelineMapper;
use crate::transcript::{Transcript, WordKind};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/LiberationSans-Bold.ttf");

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
fn default_highlight_color() -> String {
    "#FACC15".into()
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
    #[serde(default = "default_text_color")]
    pub text_color: String,
    /// Color the word being spoken.
    #[serde(default = "default_true")]
    pub highlight_words: bool,
    #[serde(default = "default_highlight_color")]
    pub highlight_color: String,
    /// A dark outline around the letters.
    #[serde(default = "default_true")]
    pub outline: bool,
    /// A box behind the text.
    #[serde(default)]
    pub background: bool,
    #[serde(default = "default_background_color")]
    pub background_color: String,
    #[serde(default = "default_background_opacity")]
    pub background_opacity: f32,
    #[serde(default)]
    pub uppercase: bool,
    /// Most words shown at once.
    #[serde(default = "default_max_words")]
    pub max_words: u32,
    /// Most lines a caption takes; a longer one is drawn smaller to fit.
    #[serde(default = "default_max_lines")]
    pub max_lines: u32,
}

fn default_max_lines() -> u32 {
    MAX_LINES as u32
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
            highlight_words: true,
            highlight_color: default_highlight_color(),
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
        check(
            "Caption background opacity",
            self.background_opacity,
            (0.0, 1.0),
        )?;
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
        if let Some(track) = &self.track_id {
            if track.is_empty()
                || track.len() > 64
                || !track
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err("Invalid caption track id".into());
            }
        }
        parse_hex_rgb(&self.text_color)?;
        parse_hex_rgb(&self.highlight_color)?;
        parse_hex_rgb(&self.background_color)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CueWord {
    /// The transcript word it shows.
    pub id: String,
    pub text: String,
    pub start_us: u64,
    pub end_us: u64,
}

/// One caption on screen, in edited time.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptionCue {
    pub start_us: u64,
    pub end_us: u64,
    pub words: Vec<CueWord>,
}

fn ends_sentence(text: &str) -> bool {
    text.ends_with(['.', '?', '!', '\u{2026}'])
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
        if word.kind != WordKind::Word
            || word.text.trim().is_empty()
            || transcript.caption_mark(&word.id).hidden
        {
            continue;
        }
        let Some((start_us, end_us)) =
            mapper.edited_span_of(word.source_start_us, word.source_end_us)
        else {
            continue;
        };
        let text = if settings.uppercase {
            word.text.trim().to_uppercase()
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

/// The word being spoken at `edited_us`: the last word that has started.
pub fn active_word(cue: &CaptionCue, edited_us: u64) -> Option<usize> {
    cue.words.iter().rposition(|w| w.start_us <= edited_us)
}

fn font() -> &'static fontdue::Font {
    static FONT: OnceLock<fontdue::Font> = OnceLock::new();
    FONT.get_or_init(|| {
        fontdue::Font::from_bytes(FONT_BYTES, fontdue::FontSettings::default())
            .expect("embedded caption font parses")
    })
}

/// A cue drawn once: text and outline coverage plus which word each text pixel belongs to.
#[derive(Clone, Debug)]
pub struct CueRaster {
    pub width: u32,
    pub height: u32,
    fill: Vec<u8>,
    outline: Vec<u8>,
    word_of: Vec<u16>,
    corner_radius: f32,
}

struct PlacedWord {
    index: usize,
    x: f32,
    line: usize,
}

fn word_width(font: &fontdue::Font, text: &str, px: f32) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    for ch in text.chars() {
        if let Some(prev) = previous {
            width += font.horizontal_kern(prev, ch, px).unwrap_or(0.0);
        }
        width += font.metrics(ch, px).advance_width;
        previous = Some(ch);
    }
    width
}

/// Lays out and rasterizes `cue` for a canvas of `canvas_w` x `canvas_h`.
pub fn rasterize_cue(
    cue: &CaptionCue,
    settings: &CaptionSettings,
    canvas_w: u32,
    canvas_h: u32,
) -> Option<CueRaster> {
    if cue.words.is_empty() || canvas_w < 16 || canvas_h < 16 {
        return None;
    }
    let font = font();
    let max_line = canvas_w as f32 * MAX_LINE_WIDTH;
    let max_lines = (settings.max_lines as usize).clamp(1, MAX_LINES);
    // Lines the words need at `px`, wrapping greedily.
    let lines_at = |px: f32| {
        let space = font.metrics(' ', px).advance_width;
        let mut lines = 1;
        let mut current = 0.0f32;
        for word in &cue.words {
            let width = word_width(font, &word.text, px);
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
    let space = font.metrics(' ', px).advance_width;
    let line_metrics = font.horizontal_line_metrics(px)?;
    let ascent = line_metrics.ascent;
    let line_height = (line_metrics.ascent - line_metrics.descent) * 1.12;

    // Greedy wrap; the widths of each line center it later.
    let mut placed = Vec::with_capacity(cue.words.len());
    let mut line_widths = vec![0.0f32];
    for (index, word) in cue.words.iter().enumerate() {
        let width = word_width(font, &word.text, px);
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
        placed.push(PlacedWord { index, x, line });
    }
    let text_w = line_widths.iter().cloned().fold(0.0f32, f32::max);
    let outline_px = if settings.outline {
        (px * 0.085).clamp(1.0, 12.0)
    } else {
        0.0
    };
    let pad_x = if settings.background { px * 0.45 } else { 0.0 } + outline_px + 2.0;
    let pad_y = if settings.background { px * 0.22 } else { 0.0 } + outline_px + 2.0;
    let width = ((text_w + pad_x * 2.0).ceil() as u32).clamp(1, canvas_w);
    let height =
        ((line_height * line_widths.len() as f32 + pad_y * 2.0).ceil() as u32).clamp(1, canvas_h);
    let len = (width * height) as usize;
    let mut fill = vec![0u8; len];
    let mut word_of = vec![u16::MAX; len];

    for word in &placed {
        let line_w = line_widths[word.line];
        let origin_x = (width as f32 - line_w) / 2.0 + word.x;
        let baseline = pad_y + ascent + line_height * word.line as f32;
        let mut pen = origin_x;
        let mut previous = None;
        for ch in cue.words[word.index].text.chars() {
            if let Some(prev) = previous {
                pen += font.horizontal_kern(prev, ch, px).unwrap_or(0.0);
            }
            let (metrics, bitmap) = font.rasterize(ch, px);
            let gx = (pen + metrics.xmin as f32).round() as i64;
            let gy = (baseline - metrics.height as f32 - metrics.ymin as f32).round() as i64;
            for row in 0..metrics.height {
                let y = gy + row as i64;
                if y < 0 || y >= height as i64 {
                    continue;
                }
                for col in 0..metrics.width {
                    let x = gx + col as i64;
                    if x < 0 || x >= width as i64 {
                        continue;
                    }
                    let coverage = bitmap[row * metrics.width + col];
                    if coverage == 0 {
                        continue;
                    }
                    let i = y as usize * width as usize + x as usize;
                    if coverage > fill[i] {
                        fill[i] = coverage;
                    }
                    word_of[i] = word.index as u16;
                }
            }
            pen += metrics.advance_width;
            previous = Some(ch);
        }
    }

    let outline = if outline_px > 0.0 {
        dilate(&fill, width, height, outline_px)
    } else {
        Vec::new()
    };
    Some(CueRaster {
        width,
        height,
        fill,
        outline,
        word_of,
        corner_radius: px * 0.3,
    })
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

/// Colors `raster` into a straight-alpha BGRA frame, highlighting `active` when set.
pub fn colorize(
    raster: &CueRaster,
    settings: &CaptionSettings,
    active: Option<usize>,
) -> VideoFrame {
    let (w, h) = (raster.width, raster.height);
    let text = rgb(&settings.text_color);
    let highlight = rgb(&settings.highlight_color);
    let background = rgb(&settings.background_color);
    let outline_color = [0.0, 0.0, 0.0];
    let active = if settings.highlight_words {
        active
    } else {
        None
    };
    let mut data = vec![0u8; (w * h * 4) as usize];
    let radius = raster.corner_radius.min(w as f32 / 2.0).min(h as f32 / 2.0);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let mut px = [0.0f32; 4];
            if settings.background {
                // Rounded box, anti-aliased at the corners.
                let (half_w, half_h) = (w as f32 / 2.0, h as f32 / 2.0);
                let (qx, qy) = (
                    (x as f32 + 0.5 - half_w).abs() - (half_w - radius),
                    (y as f32 + 0.5 - half_h).abs() - (half_h - radius),
                );
                let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt()
                    + qx.max(qy).min(0.0)
                    - radius;
                let coverage = (0.5 - outside).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    px = over(px, background, settings.background_opacity * coverage);
                }
            }
            if let Some(&o) = raster.outline.get(i) {
                if o > 0 {
                    px = over(px, outline_color, o as f32 / 255.0 * 0.9);
                }
            }
            let f = raster.fill[i];
            if f > 0 {
                let word = raster.word_of[i];
                let color = if active.is_some_and(|a| a as u16 == word) {
                    highlight
                } else {
                    text
                };
                px = over(px, color, f as f32 / 255.0);
            }
            let o = i * 4;
            data[o] = (px[2] * 255.0).round() as u8;
            data[o + 1] = (px[1] * 255.0).round() as u8;
            data[o + 2] = (px[0] * 255.0).round() as u8;
            data[o + 3] = (px[3] * 255.0).round() as u8;
        }
    }
    VideoFrame {
        pts_us: 0,
        width: w,
        height: h,
        stride: w * 4,
        format: PixelFormat::Bgra8888,
        color: ColorInfo::rec709_full(),
        data,
    }
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
        let bad = CaptionSettings {
            text_color: "red".into(),
            ..Default::default()
        };
        assert!(bad.validate().is_err());
        let bad = CaptionSettings {
            track_id: Some("../x".into()),
            ..Default::default()
        };
        assert!(bad.validate().is_err());
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
        assert_eq!(active_word(&cues[0], 600_000), Some(1));
        assert_eq!(cue_at(&cues, 5_000_000), None);
    }

    #[test]
    fn rasterizes_highlight_and_outline() {
        let t = transcript(&[("Hi", 0, 300), ("you", 350, 600)]);
        let cues = build_cues(&t, &mapper(&[(0, 1000)]), &CaptionSettings::default());
        let settings = CaptionSettings {
            highlight_color: "#FF0000".into(),
            text_color: "#00FF00".into(),
            ..Default::default()
        };
        let raster = rasterize_cue(&cues[0], &settings, 1280, 720).unwrap();
        assert!(raster.width > 40 && raster.height > 20);
        let frame = colorize(&raster, &settings, Some(0));
        let pixels: Vec<&[u8]> = frame.data.chunks_exact(4).collect();
        // Opaque red (highlighted "Hi"), opaque green ("you") and outline-dark pixels exist.
        assert!(pixels
            .iter()
            .any(|p| p[3] == 255 && p[2] > 200 && p[1] < 50));
        assert!(pixels
            .iter()
            .any(|p| p[3] == 255 && p[1] > 200 && p[2] < 50));
        assert!(pixels
            .iter()
            .any(|p| p[3] > 100 && p[0] < 10 && p[1] < 10 && p[2] < 10));
        // Corners stay transparent without a background box.
        assert_eq!(pixels[0][3], 0);
        let (x, y) = placement(&settings, 1280, 720, raster.width, raster.height);
        assert!(y + raster.height <= 720 && y > 360);
        assert_eq!(x, (1280 - raster.width) / 2);
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
            ..Default::default()
        };
        let cues = build_cues(&t, &mapper(&[(0, 2000)]), &settings);
        let one_line = rasterize_cue(
            &CaptionCue {
                words: cues[0].words[..1].to_vec(),
                ..cues[0].clone()
            },
            &settings,
            1280,
            720,
        )
        .unwrap();
        let wrapped = rasterize_cue(&cues[0], &settings, 1280, 720).unwrap();
        // Several lines, drawn smaller where three would not hold it, never past the edge.
        assert!(wrapped.height > one_line.height * 3 / 2);
        assert!(wrapped.width <= 1280);

        // One line allowed: the same words on one line, smaller.
        let single = CaptionSettings {
            max_lines: 1,
            ..settings.clone()
        };
        let one = rasterize_cue(&cues[0], &single, 1280, 720).unwrap();
        assert!(one.height < wrapped.height);
    }
}
