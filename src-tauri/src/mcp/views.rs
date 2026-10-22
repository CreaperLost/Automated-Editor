//! Reading tools beyond single frames: the mouse telemetry, and a picture of a stretch of
//! the timeline (filmstrip, waveform and words on one ruler).
use super::tools::{clock, params, text, with_reader, Ctx, Output, RangeParams};
use crate::proposals::{self, ItemState, Status};
use crate::telemetry::reader::CanonicalKind;
use crate::transcript::{edit as words, store};
use image::{Rgba, RgbaImage};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

/// Lines of telemetry an answer holds at most; ask for a shorter range for the rest.
const MAX_TELEMETRY_LINES: usize = 1500;
/// Scrolls closer together than this read as one.
const SCROLL_JOIN_US: u64 = 500_000;

pub fn telemetry_get(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: RangeParams = params(args)?;
    let (from, to) = (args.from_us.unwrap_or(0), args.to_us.unwrap_or(u64::MAX));
    let (document, recordings) = with_reader(ctx.state, |reader| {
        Ok((
            reader.document().clone(),
            reader.recordings().cloned().collect::<Vec<_>>(),
        ))
    })?;
    let mut out = format!(
        "Mouse telemetry on the edited timeline, revision {}. Positions are 0-1 on the recorded \
         screen (x right, y down).\n",
        document.revision
    );
    let mut lines = 0usize;
    for asset in &recordings {
        out.push_str(&format!("\nRecording {} ({}):\n", asset.name, asset.id));
        let stream =
            match crate::telemetry::reader::read_telemetry(std::path::Path::new(&asset.path)) {
                Ok(stream) => stream,
                Err(error) => {
                    out.push_str(&format!("  no telemetry: {error}\n"));
                    continue;
                }
            };
        let clock_of = document.picture_clock(&asset.id);
        let mut scroll: Option<(u64, u64, usize, f64, f64)> = None;
        let flush_scroll = |scroll: &mut Option<(u64, u64, usize, f64, f64)>, out: &mut String| {
            if let Some((start, end, count, x, y)) = scroll.take() {
                out.push_str(&format!(
                    "  [{}-{}] scroll x{count} at {x:.3},{y:.3}\n",
                    clock(start),
                    clock(end)
                ));
            }
        };
        for event in &stream.events {
            if lines >= MAX_TELEMETRY_LINES {
                break;
            }
            let Some(at) = clock_of.source_to_edited_us(event.t_us) else {
                continue;
            };
            if at < from || at >= to {
                continue;
            }
            match &event.kind {
                CanonicalKind::ButtonDown {
                    button,
                    norm_x,
                    norm_y,
                    ..
                } => {
                    flush_scroll(&mut scroll, &mut out);
                    let button = button.map(|b| format!(" button {b}")).unwrap_or_default();
                    out.push_str(&format!(
                        "  [{}] click{button} at {norm_x:.3},{norm_y:.3}\n",
                        clock(at)
                    ));
                    lines += 1;
                }
                CanonicalKind::Click { norm_x, norm_y, .. } => {
                    flush_scroll(&mut scroll, &mut out);
                    out.push_str(&format!(
                        "  [{}] click at {norm_x:.3},{norm_y:.3}\n",
                        clock(at)
                    ));
                    lines += 1;
                }
                CanonicalKind::Scroll { norm_x, norm_y, .. } => {
                    let (x, y) = (norm_x.unwrap_or(0.5), norm_y.unwrap_or(0.5));
                    match &mut scroll {
                        Some((_, end, count, _, _))
                            if at.saturating_sub(*end) <= SCROLL_JOIN_US =>
                        {
                            *end = at;
                            *count += 1;
                        }
                        _ => {
                            flush_scroll(&mut scroll, &mut out);
                            scroll = Some((at, at, 1, x, y));
                            lines += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        flush_scroll(&mut scroll, &mut out);
        match &stream.keys {
            None => out.push_str("  keyboard: not recorded\n"),
            Some(keys) => {
                out.push_str("  keyboard (typed text is never recorded):\n");
                let mut keyboard: Vec<(u64, String)> = Vec::new();
                for run in &keys.typing {
                    let (Some(a), Some(b)) = (
                        clock_of.source_to_edited_us(run.start_us),
                        clock_of.source_to_edited_us(run.end_us),
                    ) else {
                        continue;
                    };
                    if b < from || a >= to {
                        continue;
                    }
                    let at = run
                        .caret_start
                        .map(|c| format!(" at {:.3},{:.3}", c.norm_x, c.norm_y))
                        .unwrap_or_default();
                    keyboard.push((
                        a,
                        format!(
                            "  [{}-{}] typing, {} keys{at}\n",
                            clock(a),
                            clock(b),
                            run.keys
                        ),
                    ));
                }
                for shortcut in &keys.shortcuts {
                    let Some(at) = clock_of.source_to_edited_us(shortcut.t_us) else {
                        continue;
                    };
                    if at < from || at >= to {
                        continue;
                    }
                    let text = crate::telemetry::keys::badge_keys(shortcut, None).join(" + ");
                    keyboard.push((at, format!("  [{}] shortcut {text}\n", clock(at))));
                }
                keyboard.sort_by_key(|(at, _)| *at);
                for (_, line) in keyboard {
                    if lines >= MAX_TELEMETRY_LINES {
                        break;
                    }
                    out.push_str(&line);
                    lines += 1;
                }
            }
        }
    }
    let zooms: Vec<String> = document
        .zooms
        .iter()
        .flat_map(|zoom| {
            // Cuts inside a zoom leave its pieces back to back on the edited timeline: one line.
            let mut pieces: Vec<(u64, u64)> = Vec::new();
            for (a, b) in document.zoom_edited(zoom) {
                match pieces.last_mut() {
                    Some(last) if a <= last.1 + 1_000 => last.1 = last.1.max(b),
                    _ => pieces.push((a, b)),
                }
            }
            pieces
                .into_iter()
                .filter(|&(a, b)| b > from && a < to)
                .map(move |(a, b)| {
                    format!(
                        "  [{}-{}] zoom {:.2}x at {:.3},{:.3}{}\n",
                        clock(a),
                        clock(b),
                        zoom.scale,
                        zoom.center_x,
                        zoom.center_y,
                        if zoom.fixed {
                            " (fixed)"
                        } else {
                            " (follows the mouse)"
                        }
                    )
                })
        })
        .collect();
    out.push_str(&format!("\nZooms in the edit ({}):\n", zooms.len()));
    for zoom in zooms {
        out.push_str(&zoom);
    }
    if lines >= MAX_TELEMETRY_LINES {
        out.push_str(&format!(
            "\nStopped at {MAX_TELEMETRY_LINES} lines; ask for a shorter range for the rest.\n"
        ));
    }
    text(out)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ViewParams {
    /// Start of the stretch, edited microseconds.
    pub from_us: u64,
    /// End of the stretch, edited microseconds.
    pub to_us: u64,
    /// Image width in pixels, 640 to 2400. Default 1600.
    #[serde(default)]
    pub width: Option<u32>,
    /// The sound to draw and read words from. Default: the first transcribed one.
    #[serde(default)]
    pub stream: Option<String>,
}

const BACKGROUND: Rgba<u8> = Rgba([22, 24, 28, 255]);
const RULER_TEXT: Rgba<u8> = Rgba([150, 156, 168, 255]);
const GRID: Rgba<u8> = Rgba([48, 52, 60, 255]);
const WAVE: Rgba<u8> = Rgba([45, 190, 170, 255]);
const WAVE_RMS: Rgba<u8> = Rgba([120, 230, 210, 255]);
const WORD: Rgba<u8> = Rgba([230, 232, 236, 255]);
const WORD_ID: Rgba<u8> = Rgba([120, 126, 138, 255]);
const CUT_SHADE: Rgba<u8> = Rgba([230, 80, 80, 70]);
const CUT_TEXT: Rgba<u8> = Rgba([255, 130, 130, 255]);
const SPEED_MARK: Rgba<u8> = Rgba([240, 190, 70, 255]);

const RULER_H: u32 = 22;
const WAVE_H: u32 = 84;
const WORD_ROW_H: u32 = 20;
const ID_ROW_H: u32 = 16;

pub fn timeline_view(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ViewParams = params(args)?;
    let width = args.width.unwrap_or(1600).clamp(640, 2400) & !1;
    let gathered = with_reader(ctx.state, |reader| {
        let document = reader.document();
        let duration = document.duration_us();
        if duration == 0 {
            return Err("The timeline is empty".into());
        }
        let from = args.from_us.min(duration - 1);
        let to = args.to_us.min(duration);
        if to <= from {
            return Err("toUs must come after fromUs, inside the timeline".into());
        }
        let stream = super::tools::transcribed_stream(reader, args.stream.clone()).ok();
        let canvas = document.layout.preview_dimensions()?;
        let wave = stream.as_deref().and_then(|key| {
            let context = crate::commands::sound_context(reader, key, true).ok()?;
            crate::project::waveform::query_waveform(
                &context,
                from,
                to,
                (width as usize).min(crate::project::waveform::MAX_QUERY_BUCKETS),
                &|| false,
            )
            .ok()
        });
        let spoken = match &stream {
            Some(key) => match store::load_transcript(reader.root(), key)? {
                Some(transcript) => {
                    let view = words::view(
                        &transcript,
                        &document.transcript_clock(key)?,
                        document.revision,
                    );
                    view.words
                        .into_iter()
                        .filter_map(|w| {
                            let (a, b) = (w.edited_start_us?, w.edited_end_us?);
                            (b > from && a < to).then(|| (a, b, w.word.id, w.word.text))
                        })
                        .collect()
                }
                None => Vec::new(),
            },
            None => Vec::new(),
        };
        let view = proposals::view(reader, &proposals::load(reader.root()).unwrap_or_default());
        let cuts: Vec<(u64, u64, String)> = view
            .batches
            .iter()
            .filter(|b| b.status == Status::Pending)
            .flat_map(|b| {
                b.items
                    .iter()
                    .filter(|i| i.state == ItemState::Pending)
                    .filter_map(|i| {
                        Some((
                            i.edited_start_us?,
                            i.edited_end_us?,
                            format!("{} {}", b.id, i.id),
                        ))
                    })
            })
            .filter(|&(a, b, _)| b > from && a < to)
            .collect();
        let speeds = sped_stretches(&document.sequence, from, to);
        Ok((
            from,
            to,
            canvas,
            stream,
            wave,
            (spoken, speeds),
            cuts,
            document.revision,
        ))
    })?;
    let (from, to, canvas, stream, wave, (spoken, speeds), cuts, revision) = gathered;
    let span = (to - from) as f64;
    let x_of = |t: u64| ((t.saturating_sub(from)) as f64 / span * width as f64).round() as i64;

    // The filmstrip: one frame in the middle of each equal slice.
    let count = (width / 200).clamp(2, 12);
    let thumb_w = (width / count) & !1;
    let thumb_h = ((canvas.1 as u64 * thumb_w as u64 / canvas.0.max(1) as u64) as u32 & !1).max(2);
    let film_y = RULER_H;
    let wave_y = film_y + thumb_h + 6;
    let words_y = wave_y + WAVE_H + 4;
    let ids_y = words_y + 2 * WORD_ROW_H;
    let height = ids_y + ID_ROW_H + 6;
    let mut image = RgbaImage::from_pixel(width, height, BACKGROUND);

    let mut moments = Vec::new();
    for i in 0..count {
        let at = from + ((span * (i as f64 + 0.5)) / count as f64) as u64;
        let (frame, _) = super::frames::render(ctx.state, None, at, thumb_w)?;
        blit_bgra(&mut image, &frame, (i * thumb_w) as i64, film_y as i64);
        moments.push(at);
    }

    // The ruler.
    let step = tick_step(span, width);
    let mut tick = from.div_ceil(step) * step;
    while tick < to {
        let x = x_of(tick);
        fill(&mut image, x, 0, x + 1, RULER_H as i64 - 6, GRID);
        fill(&mut image, x, wave_y as i64, x + 1, height as i64, GRID);
        draw_text(&mut image, x + 3, 3, &clock(tick), 12.0, RULER_TEXT);
        tick += step;
    }

    // The waveform, peak and RMS mirrored around the middle.
    let middle = (wave_y + WAVE_H / 2) as i64;
    if let Some(page) = &wave {
        let buckets = page.buckets.len().max(1);
        for (i, bucket) in page.buckets.iter().enumerate() {
            if bucket.gap {
                continue;
            }
            let x0 = (i * width as usize / buckets) as i64;
            let x1 = (((i + 1) * width as usize / buckets) as i64).max(x0 + 1);
            let half =
                |level: f32| (level.clamp(0.0, 1.0).sqrt() * (WAVE_H as f32 / 2.0 - 2.0)) as i64;
            let (peak, rms) = (half(bucket.peak), half(bucket.rms));
            fill(&mut image, x0, middle - peak, x1, middle + peak + 1, WAVE);
            fill(&mut image, x0, middle - rms, x1, middle + rms + 1, WAVE_RMS);
        }
    } else {
        draw_text(
            &mut image,
            8,
            middle - 8,
            "no sound to draw",
            13.0,
            RULER_TEXT,
        );
    }

    // Stretches played faster or slower: a bar under the ruler, with the speed.
    for (a, b, speed) in &speeds {
        let (x0, x1) = (x_of(*a).max(0), x_of(*b).min(width as i64));
        fill(
            &mut image,
            x0,
            RULER_H as i64 - 4,
            x1.max(x0 + 1),
            RULER_H as i64,
            SPEED_MARK,
        );
        draw_text(
            &mut image,
            x0 + 2,
            film_y as i64 + 2,
            &speed_label(*speed),
            12.0,
            SPEED_MARK,
        );
    }

    // Pending cut proposals, shaded over the waveform and the words.
    for (a, b, label) in &cuts {
        let (x0, x1) = (x_of(*a).max(0), x_of(*b).min(width as i64));
        fill(
            &mut image,
            x0,
            wave_y as i64,
            x1.max(x0 + 1),
            height as i64,
            CUT_SHADE,
        );
        draw_text(&mut image, x0 + 2, wave_y as i64 + 2, label, 11.0, CUT_TEXT);
    }

    // The words, on two rows so neighbours do not overlap, and their ids below.
    let mut row_end = [i64::MIN; 2];
    let mut id_end = i64::MIN;
    for (start, end, id, word) in &spoken {
        let x = x_of(*start).max(0);
        let cut = cuts.iter().any(|(a, b, _)| start < b && end > a);
        let colour = if cut { CUT_TEXT } else { WORD };
        let word = word.trim();
        let w = text_width(word, 14.0) as i64;
        if let Some(row) = (0..2).find(|&r| x >= row_end[r] + 4) {
            draw_text(
                &mut image,
                x,
                (words_y + row as u32 * WORD_ROW_H) as i64,
                word,
                14.0,
                colour,
            );
            row_end[row] = x + w;
        }
        if x >= id_end + 4 {
            draw_text(&mut image, x, ids_y as i64, id, 10.0, WORD_ID);
            id_end = x + text_width(id, 10.0) as i64;
        }
    }

    let mut out = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| format!("Could not encode the view: {e}"))?;
    let moments: Vec<String> = moments.iter().map(|&t| clock(t)).collect();
    Ok(vec![
        Output::Png(out.into_inner()),
        Output::Text(
            format!(
                "Timeline {}-{} at revision {revision}, {width}x{height}. Frames at {}. Waveform \
             and words of {}; {} words, {} pending cut{} shaded red.",
                clock(from),
                clock(to),
                moments.join(", "),
                stream.as_deref().unwrap_or("no transcribed sound"),
                spoken.len(),
                cuts.len(),
                if cuts.len() == 1 { "" } else { "s" },
            ) + &speeds
                .iter()
                .map(|(a, b, speed)| {
                    format!(
                        " Plays at {} {}-{}.",
                        speed_label(*speed),
                        clock(*a),
                        clock(*b)
                    )
                })
                .collect::<String>(),
        ),
    ])
}

/// "2×", "0.5×".
pub(super) fn speed_label(speed: f64) -> String {
    format!("{}×", (speed * 100.0).round() / 100.0)
}

/// Where clips of `sequence` play at other than normal speed within `[from, to)`, by
/// speed, merged where they meet (linked picture and sound are one stretch).
pub(super) fn sped_stretches(
    sequence: &crate::sequence::Sequence,
    from: u64,
    to: u64,
) -> Vec<(u64, u64, f64)> {
    let mut spans: Vec<(u64, u64, f64)> = sequence
        .tracks
        .iter()
        .flat_map(|t| t.clips.iter())
        .filter(|c| c.speed != 1.0 && c.end_us() > from && c.start_us < to)
        .map(|c| (c.start_us.max(from), c.end_us().min(to), c.speed))
        .collect();
    spans.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut out: Vec<(u64, u64, f64)> = Vec::new();
    for (a, b, speed) in spans {
        match out.iter_mut().rev().find(|s| s.2 == speed && a <= s.1) {
            Some(last) => last.1 = last.1.max(b),
            None => out.push((a, b, speed)),
        }
    }
    out
}

/// A ruler step that keeps labels about 110 pixels apart or more.
fn tick_step(span_us: f64, width: u32) -> u64 {
    const STEPS: [u64; 13] = [
        100_000,
        250_000,
        500_000,
        1_000_000,
        2_000_000,
        5_000_000,
        10_000_000,
        15_000_000,
        30_000_000,
        60_000_000,
        120_000_000,
        300_000_000,
        600_000_000,
    ];
    let per_px = span_us / width as f64;
    STEPS
        .into_iter()
        .find(|&step| step as f64 / per_px >= 110.0)
        .unwrap_or(1_800_000_000)
}

/// Blends `colour` over the rectangle `[x0, x1) x [y0, y1)`, clipped to the image.
pub(super) fn fill(image: &mut RgbaImage, x0: i64, y0: i64, x1: i64, y1: i64, colour: Rgba<u8>) {
    let (w, h) = (image.width() as i64, image.height() as i64);
    for y in y0.max(0)..y1.min(h) {
        for x in x0.max(0)..x1.min(w) {
            blend(image.get_pixel_mut(x as u32, y as u32), colour, colour.0[3]);
        }
    }
}

fn blend(pixel: &mut Rgba<u8>, colour: Rgba<u8>, alpha: u8) {
    let a = alpha as u16;
    for c in 0..3 {
        pixel.0[c] = ((colour.0[c] as u16 * a + pixel.0[c] as u16 * (255 - a)) / 255) as u8;
    }
}

pub(super) fn text_width(text: &str, px: f32) -> f32 {
    let font = crate::captions::font();
    text.chars()
        .map(|ch| font.metrics(ch, px).advance_width)
        .sum()
}

/// Draws `text` with its top at `y`.
pub(super) fn draw_text(
    image: &mut RgbaImage,
    x: i64,
    y: i64,
    text: &str,
    px: f32,
    colour: Rgba<u8>,
) {
    let font = crate::captions::font();
    let ascent = font
        .horizontal_line_metrics(px)
        .map(|m| m.ascent)
        .unwrap_or(px * 0.8);
    let (w, h) = (image.width() as i64, image.height() as i64);
    let mut pen = x as f32;
    for ch in text.chars() {
        let (metrics, coverage) = font.rasterize(ch, px);
        let left = pen.round() as i64 + metrics.xmin as i64;
        let top = y + (ascent - metrics.height as f32 - metrics.ymin as f32).round() as i64;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let alpha = coverage[row * metrics.width + column];
                let (px_x, px_y) = (left + column as i64, top + row as i64);
                if alpha == 0 || px_x < 0 || px_y < 0 || px_x >= w || px_y >= h {
                    continue;
                }
                let a = (alpha as u16 * colour.0[3] as u16 / 255) as u8;
                blend(image.get_pixel_mut(px_x as u32, px_y as u32), colour, a);
            }
        }
        pen += metrics.advance_width;
    }
}

pub(super) fn blit_bgra(image: &mut RgbaImage, frame: &crate::media::VideoFrame, x: i64, y: i64) {
    for row in 0..frame.height as i64 {
        let start = row as usize * frame.stride as usize;
        let Some(line) = frame.data.get(start..start + frame.width as usize * 4) else {
            return;
        };
        for (column, px) in line.chunks_exact(4).enumerate() {
            let (px_x, px_y) = (x + column as i64, y + row);
            if px_x < image.width() as i64 && px_y < image.height() as i64 {
                image.put_pixel(px_x as u32, px_y as u32, Rgba([px[2], px[1], px[0], 255]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruler_steps_keep_labels_apart() {
        assert_eq!(tick_step(30_000_000.0, 1600), 5_000_000);
        assert_eq!(tick_step(5_000_000.0, 1600), 500_000);
        assert_eq!(tick_step(3_600_000_000.0, 1600), 300_000_000);
    }

    #[test]
    fn text_is_drawn_inside_the_image() {
        let mut image = RgbaImage::from_pixel(200, 30, BACKGROUND);
        draw_text(&mut image, 2, 2, "w-12 Hello", 14.0, WORD);
        draw_text(&mut image, 190, 25, "clipped at the edge", 14.0, WORD);
        assert!(image.pixels().any(|p| p.0[0] > 150));
    }
}
