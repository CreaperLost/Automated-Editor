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
            app's AI pass picks moments and adds them (it needs the user's AI key; when you \
            can read the transcript, pick them yourself with `create`). `range` with `clip`, \
            `fromWord` and `toWord`: it plays those words instead. `rename` with `clip` and \
            `title`; `remove` with `clip`. Each change is one undo step.",
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
    /// `clip` gets the title `title`.
    Rename,
    /// Remove `clip`.
    Remove,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClipsArgs {
    /// list (default), create, find, range, rename or remove.
    #[serde(default)]
    pub op: ClipsOp,
    /// range, rename, remove: the clip's id or title.
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
    /// create: pick its layout sections right away. Default true.
    #[serde(default)]
    pub scenes: Option<bool>,
    /// create, range, find: the sound whose transcript the words are from. Default: the
    /// transcribed speech.
    #[serde(default)]
    pub stream: Option<String>,
}

fn clips(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ClipsArgs = params(args)?;
    match args.op {
        ClipsOp::List => list_clips(ctx),
        ClipsOp::Create => create_clip(ctx, args),
        ClipsOp::Find => find_clips(ctx, args),
        ClipsOp::Range | ClipsOp::Rename | ClipsOp::Remove => change_clip(ctx, args),
    }
}

fn list_clips(ctx: &Ctx) -> Result<Vec<Output>, String> {
    let clips = of_kind(ctx, Kind::Clip)?;
    if clips.is_empty() {
        return text(
            "No clips yet: `clips` with `op: create` (you pick the words) or `op: find`."
                .to_string(),
        );
    }
    let mut lines = vec![format!("{}:", plural(clips.len(), "clip"))];
    lines.extend(clips.iter().map(|c| {
        let place = match (c.edited_start_us, c.edited_end_us, &c.edit) {
            (_, _, Some(_)) => "on its own timeline".to_string(),
            (Some(a), Some(b), None) => format!("{}-{} of the video", clock(a), clock(b)),
            _ => "an end was cut from the video".to_string(),
        };
        let reason = if c.reason.is_empty() {
            String::new()
        } else {
            format!(": {}", c.reason)
        };
        format!("{} \"{}\", {}, {}{reason}", c.id, c.title, place, length(c))
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
    let (from, to) = words(&args)?;
    let key = pick_sound(ctx, args.stream.clone(), Want::Transcribed)?;
    let (shorts, clip) = with_reader(ctx.state, |reader| {
        let document = reader.document();
        let (start, end) = word_span(reader, &key, &from, &to)?;
        let (layout, layouts) = crate::shorts::new_look(document, false);
        let base = format!("clip-{}", start / 1000);
        let id = (1..)
            .map(|n| match n {
                1 => base.clone(),
                n => format!("{base}-{n}"),
            })
            .find(|id| !document.shorts.iter().any(|s| &s.id == id))
            .unwrap_or(base);
        let clip = Short {
            id,
            title,
            source_start_us: start,
            source_end_us: end,
            reason: args.reason.unwrap_or_default().trim().to_string(),
            layout,
            layouts,
            media: Some(document.stream_ref(&key)?.asset),
            edit: None,
            take: None,
            caption_marks: Default::default(),
            length_us: None,
            edited_start_us: None,
            edited_end_us: None,
        };
        let (start, end) = crate::shorts::edited_range_in(&clip, document)
            .ok_or("A word at one end was cut from the video; pick words that are kept")?;
        let length =
            crate::shorts::slice_sequence(&document.sequence, &document.assets, start, end)
                .duration_us();
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
    let mut lines = vec![format!("Made clip {} \"{}\".", clip.id, clip.title)];
    if args.scenes != Some(false) {
        match super::layouts::run_pick(ctx, Some(clip.id.clone()), None, true) {
            // Its first line; the revision comes once, at the end.
            Ok(picked) => lines.extend(picked.lines().take(1).map(String::from)),
            Err(why) => lines.push(format!("Layouts not picked: {why}.")),
        }
    }
    text(applied(ctx, "clips", &before, lines)?)
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
        lines.push(format!(
            "{} \"{}\", {}: {}",
            clip.id,
            clip.title,
            length(clip),
            clip.reason
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
