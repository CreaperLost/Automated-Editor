//! `source_search`: what was said, in every transcript, cut or not (see
//! [`crate::source_search`]), and what footage shows, from its notes ([`crate::source_notes`]);
//! a sparse board of one media for writing them.
use super::tools::{clock, params, schema, text, with_reader, Ctx, Output, ToolDef};
use super::views::{draw_text, fill};
use crate::source_search::{self, LIMIT_DEFAULT};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

pub static TOOLS: [ToolDef; 1] = [ToolDef {
    name: "source_search",
    description: "Finds a word or phrase in every transcript: the recording, imported takes and \
        videos, including what the edit cut out. Each hit: stream, recording time, where it \
        plays in the edit (or `cut`, or not on the timeline), word ids and the words around \
        it. Whole words in order; the last may be the start of one; long words match one \
        letter off. Use it to find a moment by what was said, or bring back a cut line \
        (`propose_edit` `restoreCuts`). It also finds footage by its notes (what it shows). \
        `media` alone: a board of 12 frames spread over it and its notes; footage without \
        notes gets them once with `media_import` `notes`, kept by its bytes for every project.",
    schema: schema::<SearchArgs>,
    read_only: true,
    run: source_search_tool,
}];

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchArgs {
    /// A word or phrase.
    #[serde(default)]
    pub query: String,
    /// A media id: its board and notes.
    #[serde(default)]
    pub media: Option<String>,
    /// Only this stream (`<asset>.<stream>`). Default: every transcribed one.
    #[serde(default)]
    pub stream: Option<String>,
    /// At most this many hits. Default 20, at most 100.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Frames on a media's board.
const BOARD_TILES: u64 = 12;
const BOARD_COLUMNS: u32 = 4;
const TILE_HEIGHT: u32 = 180;

fn source_search_tool(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: SearchArgs = params(args)?;
    let notes_dir = crate::source_notes::dir(&ctx.host.config_dir());
    if let Some(media) = &args.media {
        return media_board(ctx, media, &notes_dir);
    }
    if args.query.trim().is_empty() {
        return Err("Give a `query`, or a `media` id for its board".into());
    }
    let (results, assets, root) = with_reader(ctx.state, |reader| {
        Ok((
            source_search::search(
                reader.root(),
                &reader.history().current,
                &args.query,
                args.stream.as_deref(),
                args.limit.unwrap_or(LIMIT_DEFAULT),
            )?,
            reader.document().assets.clone(),
            reader.root().to_path_buf(),
        ))
    })?;
    let mut out = source_search::text(&results, args.query.trim());
    let mut shown = Vec::new();
    for asset in assets.iter().filter(|a| !a.is_graphic() && !a.missing) {
        let Ok(print) = crate::source_notes::fingerprint(&root, asset) else {
            continue;
        };
        let Some(notes) = crate::source_notes::load(&notes_dir, &print) else {
            continue;
        };
        for (at, line) in notes.matching(&args.query) {
            shown.push(match at {
                Some(at) => format!("{} ({}) at {}: {line}", asset.id, asset.name, clock(at)),
                None => format!("{} ({}) shows: {line}", asset.id, asset.name),
            });
        }
    }
    if !shown.is_empty() {
        out.push_str(&format!(
            "\nIn the footage's notes (times on the media's own clock):\n{}",
            shown.join("\n")
        ));
    }
    text(out)
}

/// A board of [`BOARD_TILES`] frames spread over media `id`'s picture, and its notes.
fn media_board(ctx: &Ctx, id: &str, notes_dir: &std::path::Path) -> Result<Vec<Output>, String> {
    use crate::sequence::StreamKind;
    let (asset, root) = with_reader(ctx.state, |reader| {
        let asset = reader
            .document()
            .asset(id)
            .cloned()
            .ok_or_else(|| format!("No media {id}"))?;
        Ok((asset, reader.root().to_path_buf()))
    })?;
    let notes = crate::source_notes::fingerprint(&root, &asset)
        .ok()
        .and_then(|print| crate::source_notes::load(notes_dir, &print));
    let mut lines = vec![match &notes {
        Some(n) => format!(
            "{} ({}) has notes; don't describe it again. {}{}",
            asset.id,
            asset.name,
            n.description,
            if n.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", n.tags.join(", "))
            }
        ),
        None => format!(
            "{} ({}, {}) has no notes yet: from the board, give `media_import` `notes` once \
             (description, tags, moments at the times shown).",
            asset.id,
            asset.name,
            clock(asset.duration_us)
        ),
    }];
    if let Some(n) = &notes {
        lines.extend(
            n.moments
                .iter()
                .map(|m| format!("{}: {}", clock(m.at_us), m.text)),
        );
    }
    let Some(stream) = asset.streams.iter().find(|s| s.kind == StreamKind::Picture) else {
        lines.push("It has no picture to show.".into());
        return text(lines.join("\n"));
    };
    let source = crate::sequence::sources::stream_source(&root, &asset, &stream.id)?;
    let tiles = if asset.kind == crate::sequence::AssetKind::Image {
        1
    } else {
        BOARD_TILES
    };
    let mut frames = Vec::new();
    for i in 0..tiles {
        let at = asset.duration_us * (2 * i + 1) / (2 * tiles);
        let jpeg = crate::commands::thumbs::thumbnail(&root, &asset.id, &source, at, TILE_HEIGHT)?;
        if let Some(jpeg) = jpeg {
            let picture = image::load_from_memory(&jpeg)
                .map_err(|e| format!("Could not read a frame: {e}"))?
                .to_rgba8();
            frames.push((at, picture));
        }
    }
    if frames.is_empty() {
        lines.push("No frame could be read.".into());
        return text(lines.join("\n"));
    }
    let width = frames.iter().map(|(_, f)| f.width()).max().unwrap_or(1);
    let columns = BOARD_COLUMNS.min(frames.len() as u32);
    let rows = (frames.len() as u32).div_ceil(columns);
    let mut board = image::RgbaImage::from_pixel(
        width * columns,
        TILE_HEIGHT * rows,
        image::Rgba([16, 16, 16, 255]),
    );
    for (i, (at, frame)) in frames.iter().enumerate() {
        let (x, y) = (
            (i as u32 % columns) * width,
            (i as u32 / columns) * TILE_HEIGHT,
        );
        image::imageops::overlay(&mut board, frame, x as i64, y as i64);
        let label_top = (y + TILE_HEIGHT) as i64 - 20;
        fill(
            &mut board,
            x as i64,
            label_top,
            x as i64 + 64,
            (y + TILE_HEIGHT) as i64,
            image::Rgba([0, 0, 0, 200]),
        );
        draw_text(
            &mut board,
            x as i64 + 4,
            (y + TILE_HEIGHT) as i64 - 18,
            &clock(*at),
            14.0,
            image::Rgba([255, 255, 255, 255]),
        );
    }
    let rgb = image::DynamicImage::ImageRgba8(board).to_rgb8();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 75)
        .encode_image(&rgb)
        .map_err(|e| format!("Could not encode the board: {e}"))?;
    Ok(vec![Output::Text(lines.join("\n")), Output::Jpeg(jpeg)])
}
