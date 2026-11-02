//! The two kinds of vertical video, as the app's two tabs have them. **Clips** are cut from the
//! edited video (the Clips tab); **shorts** are made from a take the user recorded for them,
//! with the video's footage and graphics under it (the Shorts tab). Both are a
//! [`crate::shorts::Short`] (a short has a `take`); each tool here works on its own kind only
//! and names the other's tool when given the wrong one. Making a short is `short_from_take` and
//! what plays under it `short_footage` (in `actions`).
use super::actions::{applied, opened, pick_sound, plural, Want};
use super::tools::{clock, params, schema, text, with_reader, Ctx, Output, ToolDef};
use crate::commands::{self, transcript as tc};
use crate::shorts::Short;
use crate::transcript::store;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

pub static TOOLS: [ToolDef; 2] = [
    ToolDef {
        name: "clips",
        description: "Clips: vertical clips (9:16) cut from the edited video, the Clips tab \
            (shorts made from a take are `shorts`). `op`: `list` (the default, changes \
            nothing) lists them: id, title, where they play in the video, length. `create` \
            with `title`, `fromWord`, `toWord` (word ids from `transcript_get`) and `reason`: \
            a clip of those words, in the look of the clips made so far; its layout sections \
            are picked unless `scenes: false`; 3 s to 3 min, 20-60 s works best. `find`: the \
            app's AI pass picks ideas (a stretch or gathered pieces) and adds them (it \
            needs the user's AI key; when you can read the transcript, pick them yourself \
            with `create`). `range` with `clip`, \
            `fromWord` and `toWord`: it plays those words instead. `rename` with `clip` and \
            `title`; `remove` with `clip`. Each change is one undo step. Idea first: `create` \
            with `idea` and `pieces` (`[{fromWord, toWord}]`, from anywhere, in play order) \
            gathers a clip that serves one idea; `add` with `clip` and `pieces` plays more \
            after it; `arrange` with `clip` and `order` (piece numbers) reorders or drops \
            pieces. A clip keeps its own timeline: after the video's edit changes, `resync` \
            with `clip` takes the video's cuts again (each piece's; its own cuts go).",
        schema: schema::<ClipsArgs>,
        read_only: false,
        run: clips,
    },
    ToolDef {
        name: "shorts",
        description: "Shorts made from a take, the Shorts tab: a new recording of the user \
            retelling part of the video, with the video's footage and graphics under it (clips \
            cut from the video are `clips`). `op`: `list` (the default, changes nothing) lists \
            them: id, title, length, the take's camera and voice, how much plays under it. \
            `rename` with `short` and `title`; `remove` with `short`. One undo step each. A new \
            one is `short_from_take`; what plays under it is `short_footage`.",
        schema: schema::<ShortsArgs>,
        read_only: false,
        run: shorts,
    },
];

/// Which of the two kinds.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// Cut from the video.
    Clip,
    /// Made from a take.
    Short,
}

impl Kind {
    fn of(short: &Short) -> Self {
        if short.take.is_some() {
            Kind::Short
        } else {
            Kind::Clip
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Kind::Clip => "clip",
            Kind::Short => "short",
        }
    }

    fn how(self) -> &'static str {
        match self {
            Kind::Clip => "cut from the video",
            Kind::Short => "made from a take",
        }
    }

    /// The tool for this kind, as the agent knows it.
    fn tool(self) -> &'static str {
        match self {
            Kind::Clip => "`clips`",
            Kind::Short => "`shorts`",
        }
    }
}

/// A short or clip by its id or title, of either kind.
pub(super) fn named(ctx: &Ctx, wanted: &str) -> Result<Short, String> {
    with_reader(ctx.state, |reader| {
        let shorts = &reader.document().shorts;
        shorts
            .iter()
            .find(|s| s.id == wanted)
            .or_else(|| {
                shorts
                    .iter()
                    .find(|s| s.title.eq_ignore_ascii_case(wanted.trim()))
            })
            .cloned()
            .ok_or_else(|| format!("No short or clip {wanted}: `shorts` and `clips` list them"))
    })
}

/// A short or clip of `kind` by its id or title; one of the other kind says which tool has it.
pub(super) fn named_of(ctx: &Ctx, wanted: &str, kind: Kind) -> Result<Short, String> {
    let short = named(ctx, wanted)?;
    let is = Kind::of(&short);
    if is != kind {
        return Err(format!(
            "\"{}\" is a {} ({}), not a {} ({}): {} works on it",
            short.title,
            is.noun(),
            is.how(),
            kind.noun(),
            kind.how(),
            is.tool()
        ));
    }
    Ok(short)
}

/// The project's shorts of `kind`, with where they play and their length filled in.
fn of_kind(ctx: &Ctx, kind: Kind) -> Result<Vec<Short>, String> {
    with_reader(ctx.state, |reader| {
        let document = reader.document();
        let mut shorts: Vec<Short> = document
            .shorts
            .iter()
            .filter(|s| Kind::of(s) == kind)
            .cloned()
            .collect();
        crate::shorts::attach_edited(&mut shorts, document);
        Ok(shorts)
    })
}

fn length(short: &Short) -> String {
    match short.length_us {
        Some(us) => format!("{:.1} s", us as f64 / 1e6),
        None => "its ends were cut from the video".into(),
    }
}

/// Saves `shorts` as the project's, one undo step.
fn save(ctx: &Ctx, before: &super::actions::Open, shorts: Vec<Short>) -> Result<(), String> {
    commands::project_shorts_set_impl(ctx.state, before.handle.clone(), before.revision, shorts)
        .map(|_| ())
}

/// The project's shorts with `id` changed by `change` (which returns its line).
fn change(
    ctx: &Ctx,
    id: &str,
    change: impl FnOnce(&mut Vec<Short>, usize) -> Result<String, String>,
) -> Result<(Vec<Short>, String), String> {
    with_reader(ctx.state, |reader| {
        let mut shorts = reader.document().shorts.clone();
        let index = shorts
            .iter()
            .position(|s| s.id == id)
            .ok_or("That short no longer exists")?;
        let line = change(&mut shorts, index)?;
        Ok((shorts, line))
    })
}

fn new_title(title: Option<&str>) -> Result<String, String> {
    title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| t.chars().take(60).collect())
        .ok_or_else(|| "Give `title` (under 60 characters, also the file name)".into())
}

// ---------------------------------------------------------------------------------------
// Clips

/// The most stretches one clip gathers.
const MAX_PIECES: usize = 12;

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ClipsOp {
    /// List the clips (changes nothing).
    #[default]
    List,
    /// A new clip of the words `fromWord` to `toWord`.
    Create,
    /// The app's AI pass finds moments and adds them as clips.
    Find,
    /// `clip` plays the words `fromWord` to `toWord` instead.
    Range,
    /// `clip` plays `pieces` (or `fromWord` to `toWord`) after what it plays now.
    Add,
    /// `clip` plays its pieces in `order`.
    Arrange,
    /// `clip` takes the video's cuts again.
    Resync,
    /// `clip` gets the title `title`.
    Rename,
    /// Remove `clip`.
    Remove,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClipsArgs {
    /// list (default), create, find, range, add, arrange, resync, rename or remove.
    #[serde(default)]
    pub op: ClipsOp,
    /// range, add, arrange, resync, rename, remove: the clip's id or title.
    #[serde(default)]
    pub clip: Option<String>,
    /// create, rename: the title (under 60 characters, also the file name).
    #[serde(default)]
    pub title: Option<String>,
    /// create, range: the first and last word, ids from `transcript_get`.
    #[serde(default)]
    pub from_word: Option<String>,
    #[serde(default)]
    pub to_word: Option<String>,
    /// create: why it works on its own, in a sentence (shown in the Clips tab).
    #[serde(default)]
    pub reason: Option<String>,
    /// create: what the clip is about, the premise its pieces serve (shown in the Clips tab).
    #[serde(default)]
    pub idea: Option<String>,
    /// create, add: stretches from anywhere in the video, in the order they play, instead of
    /// `fromWord`/`toWord`. Each must still be in the edit.
    #[serde(default)]
    pub pieces: Vec<PieceArg>,
    /// arrange: the pieces' numbers (from 1, as `list` shows them) in the order they play; one
    /// left out is removed.
    #[serde(default)]
    pub order: Vec<usize>,
    /// create: pick its layout sections right away. Default true.
    #[serde(default)]
    pub scenes: Option<bool>,
    /// create, range, find: the sound whose transcript the words are from. Default: the
    /// transcribed speech.
    #[serde(default)]
    pub stream: Option<String>,
}

/// One stretch of a clip gathered from anywhere.
#[derive(Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PieceArg {
    pub from_word: String,
    pub to_word: String,
}

fn clips(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ClipsArgs = params(args)?;
    match args.op {
        ClipsOp::List => list_clips(ctx),
        ClipsOp::Create => create_clip(ctx, args),
        ClipsOp::Find => find_clips(ctx, args),
        ClipsOp::Add => add_to_clip(ctx, args),
        ClipsOp::Arrange => arrange_clip(ctx, args),
        ClipsOp::Range | ClipsOp::Resync | ClipsOp::Rename | ClipsOp::Remove => {
            change_clip(ctx, args)
        }
    }
}

fn list_clips(ctx: &Ctx) -> Result<Vec<Output>, String> {
    let clips = of_kind(ctx, Kind::Clip)?;
    // Where each gathered clip's pieces are in the video now, by number.
    let pieces: Vec<Vec<String>> = with_reader(ctx.state, |reader| {
        let document = reader.document();
        Ok(clips
            .iter()
            .map(|c| {
                let Some(mapper) = document
                    .clock_asset(c.media.as_deref())
                    .map(|a| document.asset_clock(a))
                    .filter(|_| c.pieces.len() > 1)
                else {
                    return Vec::new();
                };
                c.pieces
                    .iter()
                    .enumerate()
                    .map(|(n, p)| {
                        match crate::shorts::edited_span(
                            &mapper,
                            p.source_start_us,
                            p.source_end_us,
                        ) {
                            Some((a, b)) => {
                                format!("  {}. {}-{} of the video", n + 1, clock(a), clock(b))
                            }
                            None => format!("  {}. an end was cut from the video since", n + 1),
                        }
                    })
                    .collect()
            })
            .collect())
    })?;
    if clips.is_empty() {
        return text(
            "No clips yet: `clips` with `op: create` (you pick the words) or `op: find`."
                .to_string(),
        );
    }
    let mut lines = vec![format!("{}:", plural(clips.len(), "clip"))];
    lines.extend(clips.iter().zip(&pieces).flat_map(|(c, pieces)| {
        let place = match (c.edited_start_us, c.edited_end_us, &c.edit) {
            (_, _, Some(_)) => "on its own timeline".to_string(),
            (Some(a), Some(b), None) => format!("{}-{} of the video", clock(a), clock(b)),
            _ => "an end was cut from the video".to_string(),
        };
        let place = match c.pieces.len() {
            0 | 1 => place,
            n => format!("{n} pieces {place}"),
        };
        let idea = if c.idea.is_empty() {
            String::new()
        } else {
            format!(" (idea: {})", c.idea)
        };
        let reason = if c.reason.is_empty() {
            String::new()
        } else {
            format!(": {}", c.reason)
        };
        std::iter::once(format!(
            "{} \"{}\"{idea}, {}, {}{reason}",
            c.id,
            c.title,
            place,
            length(c)
        ))
        .chain(pieces.iter().cloned())
    }));
    text(lines.join("\n"))
}

/// The source span of the words `from`..`to` of transcript `key`.
fn word_span(
    reader: &crate::project::ProjectReader,
    key: &str,
    from: &str,
    to: &str,
) -> Result<(u64, u64), String> {
    let transcript =
        store::load_transcript(reader.root(), key)?.ok_or("That sound is not transcribed")?;
    let word = |id: &str| {
        transcript
            .words
            .iter()
            .find(|w| w.id == id)
            .ok_or_else(|| format!("No word {id} in the transcript of {key}"))
    };
    let (first, last) = (word(from)?, word(to)?);
    if last.source_end_us <= first.source_start_us {
        return Err("`toWord` must come after `fromWord`".into());
    }
    Ok((first.source_start_us, last.source_end_us))
}

fn words(args: &ClipsArgs) -> Result<(String, String), String> {
    match (&args.from_word, &args.to_word) {
        (Some(a), Some(b)) => Ok((a.clone(), b.clone())),
        _ => Err("Give `fromWord` and `toWord`: word ids from `transcript_get`".into()),
    }
}

fn create_clip(ctx: &Ctx, args: ClipsArgs) -> Result<Vec<Output>, String> {
    let before = opened(ctx)?;
    let title = new_title(args.title.as_deref())?;
    let wanted: Vec<(String, String)> = if args.pieces.is_empty() {
        vec![words(&args)?]
    } else {
        args.pieces
            .iter()
            .map(|p| (p.from_word.clone(), p.to_word.clone()))
            .collect()
    };
    if wanted.len() > MAX_PIECES {
        return Err(format!("Gather at most {MAX_PIECES} pieces"));
    }
    let key = pick_sound(ctx, args.stream.clone(), Want::Transcribed)?;
    let (shorts, clip) = with_reader(ctx.state, |reader| {
        let document = reader.document();
        let spans = wanted
            .iter()
            .map(|(from, to)| word_span(reader, &key, from, to))
            .collect::<Result<Vec<_>, _>>()?;
        let (first, rest) = spans.split_first().ok_or("Give the words")?;
        let mut clip = crate::shorts::new_clip(
            document,
            document.stream_ref(&key)?.asset,
            first.0,
            first.1,
            title,
        );
        clip.reason = args.reason.clone().unwrap_or_default().trim().to_string();
        clip.idea = args.idea.clone().unwrap_or_default().trim().to_string();
        let (start, end) = crate::shorts::edited_range_in(&clip, document)
            .ok_or("A word at one end was cut from the video; pick words that are kept")?;
        let mut length =
            crate::shorts::slice_sequence(&document.sequence, &document.assets, start, end)
                .duration_us();
        // Gathered: each further piece plays after the ones before, as the video has it now.
        for (&(a, b), (from, to)) in rest.iter().zip(&wanted[1..]) {
            crate::shorts::add_piece(document, &mut clip, a, b)
                .map_err(|why| format!("{from}..{to}: {why}"))?;
        }
        if let Some(own) = &clip.edit {
            length = own.sequence.duration_us();
        }
        if !(crate::shorts::MIN_SHORT_US..=crate::shorts::MAX_SHORT_US).contains(&length) {
            return Err(format!(
                "That is {:.1} s; a clip is 3 s to 3 min",
                length as f64 / 1e6
            ));
        }
        let mut shorts = document.shorts.clone();
        shorts.push(clip.clone());
        Ok((shorts, clip))
    })?;
    save(ctx, &before, shorts)?;
    let mut lines = vec![if clip.pieces.is_empty() {
        format!("Made clip {} \"{}\".", clip.id, clip.title)
    } else {
        format!(
            "Made clip {} \"{}\" from {} pieces, {:.1} s, on its own timeline.",
            clip.id,
            clip.title,
            clip.pieces.len(),
            clip.edit.as_ref().map_or(0, |e| e.sequence.duration_us()) as f64 / 1e6
        )
    }];
    if args.scenes != Some(false) {
        match super::layouts::run_pick(ctx, Some(clip.id.clone()), None, true) {
            // Its first line; the revision comes once, at the end.
            Ok(picked) => lines.extend(picked.lines().take(1).map(String::from)),
            Err(why) => lines.push(format!("Layouts not picked: {why}.")),
        }
    }
    text(applied(ctx, "clips", &before, lines)?)
}

fn add_to_clip(ctx: &Ctx, args: ClipsArgs) -> Result<Vec<Output>, String> {
    let before = opened(ctx)?;
    let wanted = args
        .clip
        .as_deref()
        .ok_or("Give `clip`: its id or title (`clips` lists them)")?;
    let clip = named_of(ctx, wanted, Kind::Clip)?;
    let pieces: Vec<(String, String)> = if args.pieces.is_empty() {
        vec![words(&args)?]
    } else {
        args.pieces
            .iter()
            .map(|p| (p.from_word.clone(), p.to_word.clone()))
            .collect()
    };
    if clip.pieces.len().max(1) + pieces.len() > MAX_PIECES {
        return Err(format!("A clip gathers at most {MAX_PIECES} pieces"));
    }
    let key = pick_sound(ctx, args.stream.clone(), Want::Transcribed)?;
    let (shorts, line) = with_reader(ctx.state, |reader| {
        let document = reader.document();
        let mut shorts = document.shorts.clone();
        let i = shorts
            .iter()
            .position(|s| s.id == clip.id)
            .ok_or("That clip no longer exists")?;
        for (from, to) in &pieces {
            let (a, b) = word_span(reader, &key, from, to)?;
            crate::shorts::add_piece(document, &mut shorts[i], a, b)
                .map_err(|why| format!("{from}..{to}: {why}"))?;
        }
        let short = &shorts[i];
        let line = format!(
            "Clip \"{}\" now plays {} pieces, {:.1} s, on its own timeline.",
            short.title,
            short.pieces.len(),
            short.edit.as_ref().map_or(0, |e| e.sequence.duration_us()) as f64 / 1e6
        );
        Ok((shorts, line))
    })?;
    save(ctx, &before, shorts)?;
    text(applied(ctx, "clips", &before, vec![line])?)
}

fn arrange_clip(ctx: &Ctx, args: ClipsArgs) -> Result<Vec<Output>, String> {
    let before = opened(ctx)?;
    let wanted = args
        .clip
        .as_deref()
        .ok_or("Give `clip`: its id or title (`clips` lists them)")?;
    let clip = named_of(ctx, wanted, Kind::Clip)?;
    if args.order.contains(&0) {
        return Err("Pieces are numbered from 1, as `clips` lists them".into());
    }
    let order: Vec<usize> = args.order.iter().map(|n| n - 1).collect();
    let (shorts, line) = with_reader(ctx.state, |reader| {
        let document = reader.document();
        let mut shorts = document.shorts.clone();
        let i = shorts
            .iter()
            .position(|s| s.id == clip.id)
            .ok_or("That clip no longer exists")?;
        crate::shorts::arrange_pieces(document, &mut shorts[i], &order)?;
        let short = &shorts[i];
        let line = format!(
            "Clip \"{}\" now plays {} pieces, {:.1} s.",
            short.title,
            short.pieces.len(),
            short.edit.as_ref().map_or(0, |e| e.sequence.duration_us()) as f64 / 1e6
        );
        Ok((shorts, line))
    })?;
    save(ctx, &before, shorts)?;
    text(applied(ctx, "clips", &before, vec![line])?)
}

fn find_clips(ctx: &Ctx, args: ClipsArgs) -> Result<Vec<Output>, String> {
    let before = opened(ctx)?;
    let key = pick_sound(ctx, args.stream, Want::Transcribed)?;
    let had: Vec<String> = with_reader(ctx.state, |reader| {
        Ok(reader
            .document()
            .shorts
            .iter()
            .map(|s| s.id.clone())
            .collect())
    })?;
    ctx.progress(0.1, "Asking the AI pass for clips");
    let project = ctx.transcript_job(|| {
        tc::project_shorts_generate_impl(
            ctx.state,
            ctx.host.transcripts(),
            before.handle.clone(),
            key,
        )
    })?;
    let mut added: Vec<Short> = project
        .shorts
        .into_iter()
        .filter(|s| !had.contains(&s.id))
        .collect();
    let mut lines = vec![format!(
        "Added {} (the clips made before and the shorts stay):",
        plural(added.len(), "clip")
    )];
    for clip in &mut added {
        let pieces = match clip.pieces.len() {
            0 | 1 => String::new(),
            n => format!(", {n} pieces"),
        };
        lines.push(format!(
            "{} \"{}\", {}{pieces}: {}",
            clip.id,
            clip.title,
            length(clip),
            if clip.idea.is_empty() {
                &clip.reason
            } else {
                &clip.idea
            }
        ));
    }
    text(applied(ctx, "clips", &before, lines)?)
}

fn change_clip(ctx: &Ctx, args: ClipsArgs) -> Result<Vec<Output>, String> {
    let before = opened(ctx)?;
    let wanted = args
        .clip
        .as_deref()
        .ok_or("Give `clip`: its id or title (`clips` lists them)")?;
    let clip = named_of(ctx, wanted, Kind::Clip)?;
    let (shorts, line) = match args.op {
        ClipsOp::Remove => change(ctx, &clip.id, |shorts, i| {
            let gone = shorts.remove(i);
            Ok(format!("Removed clip \"{}\".", gone.title))
        })?,
        ClipsOp::Rename => {
            let title = new_title(args.title.as_deref())?;
            change(ctx, &clip.id, |shorts, i| {
                let was = std::mem::replace(&mut shorts[i].title, title);
                Ok(format!(
                    "Renamed clip \"{was}\" to \"{}\".",
                    shorts[i].title
                ))
            })?
        }
        ClipsOp::Resync => with_reader(ctx.state, |reader| {
            let document = reader.document();
            let mut shorts = document.shorts.clone();
            let short = shorts
                .iter_mut()
                .find(|s| s.id == clip.id)
                .ok_or("That clip no longer exists")?;
            // Its stretch, or each piece, as the video has it now; its muted noises stay.
            let sequence = crate::shorts::video_sequence(short, document)?;
            let line = format!(
                "Clip \"{}\" takes the video's cuts again: {:.1} s (its own cuts are gone; \
                 undo brings them back).",
                short.title,
                sequence.duration_us() as f64 / 1e6
            );
            let noise_mutes = short.edit.take().map(|e| e.noise_mutes).unwrap_or_default();
            short.edit = Some(crate::shorts::ShortEdit {
                sequence,
                noise_mutes,
            });
            Ok((shorts, line))
        })?,
        _ => {
            let (from, to) = words(&args)?;
            let key = pick_sound(ctx, args.stream.clone(), Want::Transcribed)?;
            with_reader(ctx.state, |reader| {
                let document = reader.document();
                let (start, end) = word_span(reader, &key, &from, &to)?;
                let mut shorts = document.shorts.clone();
                let i = shorts
                    .iter()
                    .position(|s| s.id == clip.id)
                    .ok_or("That clip no longer exists")?;
                let short = &mut shorts[i];
                // A new stretch follows the video again, as "Take the video's cuts again"
                // does: its own cuts go.
                let dropped = short.edit.take().is_some();
                short.pieces.clear();
                short.source_start_us = start;
                short.source_end_us = end;
                short.media = Some(document.stream_ref(&key)?.asset);
                crate::shorts::edited_range_in(short, document)
                    .ok_or("A word at one end was cut from the video; pick words that are kept")?;
                let line = format!(
                    "Clip \"{}\" now plays {from}..{to}{}.",
                    short.title,
                    if dropped {
                        ", following the video again (its own cuts are gone; undo brings them back)"
                    } else {
                        ""
                    }
                );
                Ok((shorts, line))
            })?
        }
    };
    save(ctx, &before, shorts)?;
    text(applied(ctx, "clips", &before, vec![line])?)
}

// ---------------------------------------------------------------------------------------
// Shorts made from a take

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ShortsOp {
    /// List the shorts made from a take (changes nothing).
    #[default]
    List,
    /// `short` gets the title `title`.
    Rename,
    /// Remove `short`.
    Remove,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShortsArgs {
    /// list (default), rename or remove.
    #[serde(default)]
    pub op: ShortsOp,
    /// rename, remove: the short's id or title.
    #[serde(default)]
    pub short: Option<String>,
    /// rename: the new title (under 60 characters, also the file name).
    #[serde(default)]
    pub title: Option<String>,
}

fn shorts(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ShortsArgs = params(args)?;
    if args.op == ShortsOp::List {
        let shorts = of_kind(ctx, Kind::Short)?;
        if shorts.is_empty() {
            return text(
                "No shorts made from a take yet: `short_from_take` makes one (clips cut from \
                 the video are `clips`)."
                    .to_string(),
            );
        }
        let under = with_reader(ctx.state, |reader| {
            let document = reader.document();
            Ok(shorts
                .iter()
                .map(|s| crate::take_short::placed(document, s).map_or(0, |p| p.len()))
                .collect::<Vec<_>>())
        })?;
        let mut lines = vec![format!(
            "{} made from a take:",
            plural(shorts.len(), "short")
        )];
        lines.extend(shorts.iter().zip(under).map(|(s, under)| {
            let take = s.take.as_ref();
            format!(
                "{} \"{}\", {}; camera {}, voice {}; {} under it",
                s.id,
                s.title,
                length(s),
                take.and_then(|t| t.camera.as_deref()).unwrap_or("none"),
                take.map_or("?", |t| t.voice.as_str()),
                plural(under, "piece")
            )
        }));
        return text(lines.join("\n"));
    }
    let before = opened(ctx)?;
    let wanted = args
        .short
        .as_deref()
        .ok_or("Give `short`: its id or title (`shorts` lists them)")?;
    let short = named_of(ctx, wanted, Kind::Short)?;
    let (shorts, line) = match args.op {
        ShortsOp::Remove => change(ctx, &short.id, |shorts, i| {
            let gone = shorts.remove(i);
            Ok(format!("Removed short \"{}\".", gone.title))
        })?,
        _ => {
            let title = new_title(args.title.as_deref())?;
            change(ctx, &short.id, |shorts, i| {
                let was = std::mem::replace(&mut shorts[i].title, title);
                Ok(format!(
                    "Renamed short \"{was}\" to \"{}\".",
                    shorts[i].title
                ))
            })?
        }
    };
    save(ctx, &before, shorts)?;
    text(applied(ctx, "shorts", &before, vec![line])?)
}
