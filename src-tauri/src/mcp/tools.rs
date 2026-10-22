//! The tools an agent can call. Each one is a plain function over the app state, so the
//! same code answers the MCP server and the tests.
//!
//! Times an agent sees are on the edited timeline (what plays after every cut), in
//! microseconds, with a readable `m:ss.cc` beside them. Rectangles are in canvas pixels at
//! full size (1920x1080 for a 16:9 canvas), the space a HyperFrames composition is drawn in.
use crate::commands::AppState;
use crate::project::ProjectReader;
use crate::render::LayerRole;
use crate::sequence::StreamKind;
use crate::transcript::{edit::TranscriptViewWord, store, WordKind};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

pub const NO_PROJECT_MESSAGE: &str = "No project is open in AeroEdits. Ask the user to open one.";

/// A call bound to a project that isn't the one open now.
pub const OTHER_PROJECT_MESSAGE: &str = "This call was for another project: the project open in \
    AeroEdits changed (or was closed and opened again) since you read it. Nothing was changed. \
    Read project_info or status again, check it is the project the user means, and use its \
    `project` id.";

/// What a tool hands back.
#[derive(Debug)]
pub enum Output {
    Text(String),
    Png(Vec<u8>),
}

/// Where a long tool call reports how far it got: a fraction (0 to 1) and a short message.
pub type Progress<'a> = Box<dyn Fn(f64, &str) + Send + Sync + 'a>;

/// What a tool call knows about its caller, and what it changed.
pub struct Ctx<'a> {
    pub state: &'a AppState,
    /// The app around the server: transcription state, and the windows to tell.
    pub host: &'a dyn super::Host,
    /// The agent's name for itself, from its handshake.
    pub agent: String,
    /// Proposals are accepted as they arrive.
    pub autopilot: bool,
    /// The project the call was meant for (its `project` id), when it said.
    pub project: Option<String>,
    progress: Option<Progress<'a>>,
    project_changed: std::cell::Cell<bool>,
    proposals_changed: std::cell::Cell<bool>,
}

impl<'a> Ctx<'a> {
    pub fn new(host: &'a dyn super::Host, agent: impl Into<String>, autopilot: bool) -> Self {
        Self {
            state: host.state(),
            host,
            agent: agent.into(),
            autopilot,
            project: None,
            progress: None,
            project_changed: Default::default(),
            proposals_changed: Default::default(),
        }
    }

    pub fn with_progress(mut self, progress: Progress<'a>) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Tells the caller how far a long call got.
    pub fn progress(&self, fraction: f64, message: &str) {
        if let Some(progress) = &self.progress {
            progress(fraction.clamp(0.0, 1.0), message);
        }
    }

    /// The edit changed: the windows reload the project.
    pub fn changed_project(&self) {
        self.project_changed.set(true);
    }

    /// The proposals changed: the Agent panel reloads them.
    pub fn changed_proposals(&self) {
        self.proposals_changed.set(true);
    }

    pub fn project_was_changed(&self) -> bool {
        self.project_changed.get()
    }

    pub fn proposals_were_changed(&self) -> bool {
        self.proposals_changed.get()
    }
}

pub type Run = fn(&Ctx, Value) -> Result<Vec<Output>, String>;

#[derive(Clone, Copy)]
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: fn() -> Value,
    /// Changes nothing: not the project, its proposals, or any file (renders and reports
    /// included). Sent to clients as `readOnlyHint`.
    pub read_only: bool,
    pub run: Run,
}

/// Tools that overwrite or delete something the app's undo can't bring back: a file on disk,
/// graphic source, the shared library, saved answers. Sent to clients as `destructiveHint`;
/// every other writing tool only makes changes that are one undo step, or adds new files.
pub const DESTRUCTIVE: &[&str] = &[
    "brand_kit_set",
    "export",
    "graphic_set",
    "graphic_write_file",
    "library_remove",
    "library_save",
    "template_save",
    "transcript_fix",
];

pub fn destructive(name: &str) -> bool {
    DESTRUCTIVE.contains(&name)
}

/// Every tool: the reading and proposing ones here, then the app's actions.
pub fn all() -> &'static [ToolDef] {
    static ALL: std::sync::OnceLock<Vec<ToolDef>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        super::skill::TOOLS
            .iter()
            .chain(TOOLS.iter())
            .chain(super::actions::ACTIONS.iter())
            .copied()
            .collect()
    })
}

pub fn find(name: &str) -> Option<&'static ToolDef> {
    all().iter().find(|tool| tool.name == name)
}

/// Tools an edit rarely needs, left out of the list agents are sent on every connection (each
/// one's schema is paid for on every edit); `more` names them, gives their parameters and runs
/// them. The CLI and direct calls reach them as before. Name each in `more`'s description.
pub const MORE: &[&str] = &[
    "redo",
    "playhead_get",
    "layout_get",
    "telemetry_get",
    "speed",
    "transcript_fix",
    "propose_edit",
    "proposal_withdraw",
    "short_create",
    "template_save",
    "graphics_install",
    "brand_pick",
    "brand_kit_set",
    "graphics_look",
    "graphic_write_file",
    "graphic_templatize",
    "graphic_check",
    "library_save",
    "library_remove",
];

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MoreArgs {
    /// One of the tools named in the description.
    pub tool: String,
    /// Its arguments, as if called directly. Without them: its description and parameters.
    #[serde(default)]
    pub args: Option<Value>,
}

/// `more`: a tool behind it, run as if called directly (a `project` beside `tool` goes to it),
/// or, without `args`, what it does and takes.
fn more(ctx: &mut Ctx, mut args: Value) -> Result<Vec<Output>, String> {
    let project = args.as_object_mut().and_then(|a| a.remove("project"));
    let args: MoreArgs = params(args)?;
    let name = args.tool.trim();
    let tool = MORE
        .iter()
        .find(|n| **n == name)
        .and_then(|n| find(n))
        .ok_or_else(|| {
            format!(
                "No tool {name} behind `more`; they are: {}",
                MORE.join(", ")
            )
        })?;
    let Some(mut inner) = args.args else {
        let entry = super::tool_entry(tool);
        let schema = serde_json::to_string(&entry.input_schema).unwrap_or_default();
        return text(format!(
            "{}: {}\nArguments (JSON schema): {schema}\nCall `more` with `tool: \"{}\"` and \
             `args`.",
            tool.name, tool.description, tool.name
        ));
    };
    if inner.is_null() {
        inner = json!({});
    }
    if let (Some(project), Some(map)) = (project, inner.as_object_mut()) {
        map.entry("project").or_insert(project);
    }
    call(tool, ctx, inner)
}

/// Tools whose change rests on what the agent read before (a revision, a batch id): they must
/// say which project that was, so a project switch can't send them to another one.
pub const NEEDS_PROJECT: &[&str] = &["propose_cuts", "propose_edit", "undo_batch"];

/// Whether a tool takes a `project` argument: every tool that changes something, except the
/// ones that open or make a project.
pub fn takes_project(tool: &ToolDef) -> bool {
    !tool.read_only && !matches!(tool.name, "open" | "create")
}

/// Runs a tool as the server does. A `project` argument (the id project_info and status give)
/// is taken out of `args` and must be the project open now, else nothing runs; tools in
/// [`NEEDS_PROJECT`] refuse to run without one. Proposals check the id again under the
/// command lock, as the UI's commands do.
pub fn call(tool: &ToolDef, ctx: &mut Ctx, mut args: Value) -> Result<Vec<Output>, String> {
    if tool.name == "more" {
        return more(ctx, args);
    }
    let project = match args.as_object_mut().and_then(|args| args.remove("project")) {
        None | Some(Value::Null) => None,
        Some(Value::String(id)) => Some(id),
        Some(_) => return Err("`project` is the id project_info or status gives.".into()),
    };
    match &project {
        Some(id) => {
            let open = with_reader(ctx.state, |reader| {
                Ok(reader.summary.project_handle.clone())
            })?;
            if *id != open {
                return Err(OTHER_PROJECT_MESSAGE.into());
            }
        }
        None if NEEDS_PROJECT.contains(&tool.name) => {
            return Err(format!(
                "{} needs `project`: the id project_info or status gives for the project you \
                 read the revision from.",
                tool.name
            ))
        }
        None => {}
    }
    ctx.project = project;
    (tool.run)(ctx, args)
}

static TOOLS: [ToolDef; 15] = [
    ToolDef {
        name: "more",
        description: "Tools an edit rarely needs, left out of this list to save tokens: `tool` \
            alone gives one's description and arguments; with `args` it runs, as if called \
            directly. redo (the undo taken back); playhead_get (where the user's playhead is); \
            layout_get (a layout's details); telemetry_get (mouse and keys at a time); speed \
            (play a stretch faster or slower, only when asked); transcript_fix (a misheard \
            word); propose_edit (timeline edits as a batch for the user); proposal_withdraw \
            (take back your batch); short_create (a clip from the video's words); \
            template_save (keep the user's answers); graphics_install (the graphics renderer, \
            ask first); brand_pick (the project's brand kit); brand_kit_set (make or change a \
            kit); graphics_look (one style for every graphic: sizes, corners, motion); \
            graphic_write_file, graphic_templatize, graphic_check, library_save, \
            library_remove (writing a graphic of your own and keeping it in the library).",
        schema: schema::<MoreArgs>,
        read_only: false,
        // Runs through `call`, which hands the call to the tool named.
        run: |_, _| Err("`more` runs through `tools::call`".into()),
    },
    ToolDef {
        name: "project_info",
        description: "The open project: name, revision, length, canvas size, tracks (V1 is the \
            bottom video track), and media with their streams. A stream key is \
            `<asset>.<stream>`; streams with `transcribed: true` can be read with \
            transcript_get. Call this first.",
        schema: schema::<NoParams>,
        read_only: true,
        run: project_info,
    },
    ToolDef {
        name: "transcript_get",
        description: "What is said, as lines of phrases on the edited timeline: \
            `[start-end] w-<first>..w-<last> text`. Word ids are stable, so graphics and cuts \
            can refer to them. Gaps longer than a second are shown. Words already cut out are \
            left out. Without `stream`, the first transcribed stream is used.",
        schema: schema::<TranscriptParams>,
        read_only: true,
        run: transcript_get,
    },
    ToolDef {
        name: "timeline_get",
        description: "The clips on every track, on the edited timeline: id, track (V1, V2, A1, \
            ...), start and end, the media and stream it plays, and from where in it.",
        schema: schema::<RangeParams>,
        read_only: true,
        run: timeline_get,
    },
    ToolDef {
        name: "playhead_get",
        description: "Where the user's playhead is on the edited timeline, and whether it is \
            playing. Use it when the user says 'here' or 'now'.",
        schema: schema::<NoParams>,
        read_only: true,
        run: playhead_get,
    },
    ToolDef {
        name: "frame_render",
        description: "A PNG of the finished frame at a moment of the edited timeline, as it \
            will export: layout, zooms, cursor, overlays and captions included. Use it to check \
            what is on screen before and after placing something.",
        schema: schema::<FrameParams>,
        read_only: true,
        run: frame_render,
    },
    ToolDef {
        name: "layout_get",
        description: "Where things are on the canvas at a moment, in canvas pixels: the screen \
            recording, the webcam bubble, overlays and captions. Keep graphics clear of the \
            webcam and caption rectangles unless they are meant to cover them.",
        schema: schema::<AtParams>,
        read_only: true,
        run: layout_get,
    },
    ToolDef {
        name: "telemetry_get",
        description: "What the mouse and keyboard did, on the edited timeline: clicks and \
            scrolls with their positions on the recorded screen (0-1, x right, y down), typing \
            stretches (key count and caret position, never the text) and shortcuts pressed, \
            when the recorder recorded the keyboard, and the zooms in the edit. Clicks and \
            typing show where the tutorial's action is; use them to place callouts and to \
            judge cuts. Shortcut badges are the layout setting `shortcutBadges`.",
        schema: schema::<RangeParams>,
        read_only: true,
        run: super::views::telemetry_get,
    },
    ToolDef {
        name: "timeline_view",
        description: "A PNG of a stretch of the edited timeline: a filmstrip of frames, the \
            voice's waveform and the words with their ids, on one time ruler, with pending \
            cut proposals shaded. Good for judging cuts and pauses; keep the range under a \
            minute for readable words.",
        schema: schema::<super::views::ViewParams>,
        read_only: true,
        run: super::views::timeline_view,
    },
    ToolDef {
        name: "storyboard",
        description: "One small PNG contact sheet of the whole edit (or `fromUs`-`toUs`, or a \
            short with `short`): a tile per scene, split where the camera or a short's scene, \
            a zoom, a graphic, a chapter or a long silence starts, short stretches merged. \
            Each tile is a finished frame with its time, what it shows, zoom, graphic, chapter \
            and words; the JSON lists the same per tile number. Use it to check an edit after \
            your passes and before export, instead of many frame_render calls.",
        schema: schema::<super::storyboard::StoryboardArgs>,
        read_only: true,
        run: super::storyboard::storyboard,
    },
    ToolDef {
        name: "propose_cuts",
        description: "Proposes cutting time out of the video (every track, closing the gap, as \
            word cuts do). Give runs of words by id (`fromWord`..`toWord` from transcript_get) \
            or edited-time ranges, each with a short reason. Nothing changes until the user \
            accepts the batch in the Agent panel (or autopilot is on); word cuts stay valid \
            through later edits. Needs `expectedRevision` from project_info or transcript_get. \
            Returns the batch id and what it would do.",
        schema: schema::<super::propose::CutsParams>,
        read_only: false,
        run: super::propose::propose_cuts,
    },
    ToolDef {
        name: "propose_edit",
        description: "Proposes timeline edits, applied in order as one undo step once the user \
            accepts: addTrack, removeTrack, setTrack, moveTrack, placeAsset, moveClips, \
            trimClip, split, delete, deleteRange, rippleTrim, link, unlink, setMagnetic, \
            setClip, setSpeed, restoreCuts (the schema lists each one's fields). For B-roll, \
            give placeAsset `streams` so unwanted sound isn't placed. They are checked now on \
            a copy and the result's diff is returned; they hold only for this revision. For \
            removing speech prefer propose_cuts.",
        schema: schema::<super::propose::EditParams>,
        read_only: false,
        run: super::propose::propose_edit,
    },
    ToolDef {
        name: "proposals_list",
        description: "The proposal batches and what became of them: pending, accepted (and by \
            autopilot), rejected or withdrawn, item by item for cuts. Check it to learn what \
            the user decided before proposing more.",
        schema: schema::<super::propose::ListParams>,
        read_only: true,
        run: super::propose::proposals_list,
    },
    ToolDef {
        name: "proposal_withdraw",
        description: "Takes back one of your batches that is still waiting for the user.",
        schema: schema::<super::propose::BatchParams>,
        read_only: false,
        run: super::propose::proposal_withdraw,
    },
    ToolDef {
        name: "undo_batch",
        description: "Undoes one of your accepted batches, while nothing has changed the edit \
            after it. The batch is then withdrawn. Needs `expectedRevision`.",
        schema: schema::<super::propose::UndoParams>,
        read_only: false,
        run: super::propose::undo_batch,
    },
];

pub(super) fn schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).unwrap_or_else(|_| json!({"type": "object"}))
}

pub(super) fn params<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, String> {
    let value = if value.is_null() { json!({}) } else { value };
    serde_json::from_value(value).map_err(|e| format!("Invalid arguments: {e}"))
}

pub(super) fn text(value: impl Into<String>) -> Result<Vec<Output>, String> {
    Ok(vec![Output::Text(value.into())])
}

pub(super) fn pretty(value: &Value) -> Result<Vec<Output>, String> {
    text(serde_json::to_string_pretty(value).map_err(|e| e.to_string())?)
}

/// `m:ss.cc`, or `h:mm:ss.cc` from an hour.
pub fn clock(us: u64) -> String {
    let centis = us / 10_000;
    let (h, m, s, c) = (
        centis / 360_000,
        centis / 6_000 % 60,
        centis / 100 % 60,
        centis % 100,
    );
    if h > 0 {
        format!("{h}:{m:02}:{s:02}.{c:02}")
    } else {
        format!("{m}:{s:02}.{c:02}")
    }
}

pub(super) fn with_reader<T>(
    state: &AppState,
    read: impl FnOnce(&ProjectReader) -> Result<T, String>,
) -> Result<T, String> {
    let opened = state.opened_project.lock();
    read(opened.as_ref().ok_or(NO_PROJECT_MESSAGE)?)
}

#[derive(Deserialize, JsonSchema)]
pub struct NoParams {}

fn project_info(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let state = ctx.state;
    params::<NoParams>(args)?;
    let info = with_reader(state, |reader| {
        let document = &reader.history().current;
        let summary = &reader.summary;
        let (width, height) = document.layout.preview_dimensions()?;
        let tracks: Vec<Value> = document
            .sequence
            .tracks
            .iter()
            .map(|track| {
                json!({
                    "id": track.id,
                    "number": document.sequence.number(&track.id),
                    "kind": track.kind,
                    "name": track.name,
                    "hidden": track.hidden,
                    "muted": track.muted,
                    "locked": track.locked,
                    "clips": track.clips.len(),
                })
            })
            .collect();
        let assets: Vec<Value> = summary
            .assets
            .iter()
            .map(|asset| {
                let streams: Vec<Value> = asset
                    .streams
                    .iter()
                    .map(|stream| {
                        let key = format!("{}.{}", asset.id, stream.id);
                        let transcribed = stream.kind == StreamKind::Sound
                            && store::load_transcript(reader.root(), &key)
                                .ok()
                                .flatten()
                                .is_some();
                        json!({
                            "key": key,
                            "kind": stream.kind,
                            "role": stream.role,
                            "name": stream.name,
                            "transcribed": transcribed,
                        })
                    })
                    .collect();
                json!({
                    "id": asset.id,
                    "name": asset.name,
                    "kind": asset.kind,
                    "durationUs": asset.duration_us,
                    "width": asset.width,
                    "height": asset.height,
                    "missing": asset.missing,
                    "streams": streams,
                })
            })
            .collect();
        Ok(json!({
            "project": summary.project_handle,
            "name": summary.name,
            "path": summary.project_path,
            "revision": document.revision,
            "durationUs": document.sequence.duration_us(),
            "duration": clock(document.sequence.duration_us()),
            "fps": summary.fps,
            "canvas": {
                "aspect": document.layout.aspect_ratio,
                "width": width,
                "height": height,
            },
            "captions": document.captions.enabled,
            "tracks": tracks,
            "media": assets,
        }))
    })?;
    pretty(&info)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptParams {
    /// Stream key such as `rec-1234.mic`, from project_info. Default: the first transcribed one.
    #[serde(default)]
    pub stream: Option<String>,
    /// Only words from here on (edited microseconds).
    #[serde(default)]
    pub from_us: Option<u64>,
    /// Only words before this (edited microseconds).
    #[serde(default)]
    pub to_us: Option<u64>,
    /// Times on this short's own timeline instead of the video's. A take's voice is read on
    /// its short's timeline without it.
    #[serde(default)]
    pub short: Option<String>,
}

/// A new line starts after a sentence end, a pause this long, or this many words.
const LINE_PAUSE_US: u64 = 600_000;
const LINE_MAX_WORDS: usize = 18;
/// Pauses this long get a line of their own.
const SHOWN_GAP_US: u64 = 1_000_000;

/// `stream`, or the first transcribed sound of the project.
pub(super) fn transcribed_stream(
    reader: &ProjectReader,
    stream: Option<String>,
) -> Result<String, String> {
    if let Some(key) = stream {
        return Ok(key);
    }
    reader
        .history()
        .current
        .assets
        .iter()
        .flat_map(|asset| {
            asset
                .streams
                .iter()
                .filter(|s| s.kind == StreamKind::Sound)
                .map(move |s| format!("{}.{}", asset.id, s.id))
        })
        .find(|key| {
            store::load_transcript(reader.root(), key)
                .ok()
                .flatten()
                .is_some()
        })
        .ok_or_else(|| {
            "Nothing is transcribed yet. Ask the user to transcribe the microphone.".into()
        })
}

fn transcript_get(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let state = ctx.state;
    let args: TranscriptParams = params(args)?;
    let (key, view, on_short) = with_reader(state, |reader| {
        let document = &reader.history().current;
        let key = transcribed_stream(reader, args.stream)?;
        let transcript = store::load_transcript(reader.root(), &key)?
            .ok_or_else(|| format!("{key} has no transcript"))?;
        // A short's timeline: the one named, else the take short this is the voice of.
        let short = match &args.short {
            Some(id) => Some(
                document
                    .shorts
                    .iter()
                    .find(|s| &s.id == id)
                    .ok_or_else(|| format!("No short {id}"))?,
            ),
            None => document
                .shorts
                .iter()
                .find(|s| s.take.as_ref().is_some_and(|t| t.voice == key)),
        };
        let clock = match short {
            Some(short) => {
                crate::shorts::short_timeline(document, short)?.transcript_clock(&key)?
            }
            None => document.transcript_clock(&key)?,
        };
        let view = crate::transcript::edit::view(&transcript, &clock, document.revision);
        Ok((key, view, short.map(|s| s.id.clone())))
    })?;
    let from = args.from_us.unwrap_or(0);
    let to = args.to_us.unwrap_or(u64::MAX);
    let words: Vec<&TranscriptViewWord> = view
        .words
        .iter()
        .filter(|w| w.edited_start_us.is_some_and(|s| s >= from && s < to))
        .collect();
    let cut = view
        .words
        .iter()
        .filter(|w| w.edited_start_us.is_none())
        .count();
    let mut out = format!(
        "Transcript of {key}{}, revision {}. {} words shown, {cut} cut out. \
         Lines: [start-end] first..last word id, text.\n",
        on_short
            .map(|id| format!(" on short {id}'s timeline"))
            .unwrap_or_default(),
        view.revision,
        words.len()
    );
    out.push_str(&pack_lines(&words));
    text(out)
}

fn pack_lines(words: &[&TranscriptViewWord]) -> String {
    let mut out = String::new();
    let mut line: Vec<&TranscriptViewWord> = Vec::new();
    let flush = |line: &mut Vec<&TranscriptViewWord>, out: &mut String| {
        let (Some(first), Some(last)) = (line.first(), line.last()) else {
            return;
        };
        let text: Vec<String> = line
            .iter()
            .map(|w| match w.word.kind {
                WordKind::Word => w.word.text.trim().to_string(),
                WordKind::AudioEvent => {
                    format!("({})", w.word.text.trim().trim_matches(['(', ')']))
                }
            })
            .collect();
        out.push_str(&format!(
            "[{}-{}] {}..{} {}\n",
            clock(first.edited_start_us.unwrap_or(0)),
            clock(last.edited_end_us.unwrap_or(0)),
            first.word.id,
            last.word.id,
            text.join(" ")
        ));
        line.clear();
    };
    for word in words {
        if let Some(previous) = line.last() {
            let gap = word
                .edited_start_us
                .unwrap_or(0)
                .saturating_sub(previous.edited_end_us.unwrap_or(0));
            let sentence = previous.word.text.trim_end().ends_with(['.', '?', '!']);
            if gap >= LINE_PAUSE_US || sentence || line.len() >= LINE_MAX_WORDS {
                flush(&mut line, &mut out);
                if gap >= SHOWN_GAP_US {
                    out.push_str(&format!("  (pause {:.1}s)\n", gap as f64 / 1e6));
                }
            }
        }
        line.push(word);
    }
    flush(&mut line, &mut out);
    out
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RangeParams {
    /// Only clips that play after this (edited microseconds).
    #[serde(default)]
    pub from_us: Option<u64>,
    /// Only clips that play before this (edited microseconds).
    #[serde(default)]
    pub to_us: Option<u64>,
}

fn timeline_get(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let state = ctx.state;
    let args: RangeParams = params(args)?;
    let from = args.from_us.unwrap_or(0);
    let to = args.to_us.unwrap_or(u64::MAX);
    let value = with_reader(state, |reader| {
        let document = &reader.history().current;
        let sequence = &document.sequence;
        // Top video track first, as the timeline shows them.
        let mut order: Vec<_> = sequence.video_tracks().rev().collect();
        order.extend(sequence.audio_tracks());
        let tracks: Vec<Value> = order
            .into_iter()
            .map(|track| {
                let clips: Vec<Value> = track
                    .clips
                    .iter()
                    .filter(|c| c.end_us() > from && c.start_us < to)
                    .map(|clip| {
                        let asset = document.assets.iter().find(|a| a.id == clip.asset);
                        json!({
                            "id": clip.id,
                            "start": clock(clip.start_us),
                            "end": clock(clip.end_us()),
                            "startUs": clip.start_us,
                            "endUs": clip.end_us(),
                            "stream": format!("{}.{}", clip.asset, clip.stream),
                            "media": asset.map(|a| a.name.as_str()),
                            "inUs": clip.in_us,
                            "outUs": clip.out_us(),
                            "speed": clip.speed,
                            "linkedTo": clip.link,
                        })
                    })
                    .collect();
                json!({
                    "id": track.id,
                    "number": sequence.number(&track.id),
                    "kind": track.kind,
                    "hidden": track.hidden,
                    "muted": track.muted,
                    "locked": track.locked,
                    "clips": clips,
                })
            })
            .collect();
        Ok(json!({
            "revision": document.revision,
            "durationUs": sequence.duration_us(),
            "magnetic": sequence.magnetic,
            "tracks": tracks,
        }))
    })?;
    pretty(&value)
}

fn playhead_get(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let state = ctx.state;
    params::<NoParams>(args)?;
    with_reader(state, |_| Ok(()))?;
    let status = state.playback.lock().status()?;
    let at = status.position_us;
    pretty(&json!({
        "atUs": at,
        "at": clock(at),
        "playing": status.state == crate::playback::PlaybackState::Playing,
        "inShort": status.short_id,
    }))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FrameParams {
    /// The moment, in edited microseconds.
    pub at_us: u64,
    /// Image width in pixels, 240 to 1920. Default 960.
    #[serde(default)]
    pub width: Option<u32>,
    /// A short (id or title): its 9:16 frame at `atUs` on the short's own timeline.
    #[serde(default)]
    pub short: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AtParams {
    /// The moment, in edited microseconds.
    pub at_us: u64,
}

fn frame_render(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let state = ctx.state;
    let args: FrameParams = params(args)?;
    // A 9:16 short is narrow: its default width keeps about the same pixel count.
    let width = args
        .width
        .unwrap_or(if args.short.is_some() { 540 } else { 960 })
        .clamp(240, 1920);
    let (frame, at) = super::frames::render(state, args.short.as_deref(), args.at_us, width)?;
    let png = super::frames::png(&frame)?;
    Ok(vec![
        Output::Png(png),
        Output::Text(format!(
            "Frame at {} ({}x{}).",
            clock(at),
            frame.width,
            frame.height
        )),
    ])
}

fn layout_get(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let state = ctx.state;
    let args: AtParams = params(args)?;
    let (scene, at, canvas) = super::frames::scene(state, args.at_us)?;
    let scale = canvas.0 as f64 / scene.width.max(1) as f64;
    let rect = |l: &crate::render::Layer| {
        let s = |v: u32| (v as f64 * scale).round() as u32;
        json!({ "x": s(l.x), "y": s(l.y), "width": s(l.width), "height": s(l.height) })
    };
    let of = |role: LayerRole| -> Vec<Value> {
        scene
            .layers
            .iter()
            .filter(|l| l.role == role)
            .map(rect)
            .collect()
    };
    pretty(&json!({
        "at": clock(at),
        "atUs": at,
        "canvas": { "width": canvas.0, "height": canvas.1 },
        "screen": of(LayerRole::Screen),
        "webcam": of(LayerRole::Webcam),
        "overlays": of(LayerRole::Overlay),
        "captions": of(LayerRole::Caption),
        "note": "Overlays include the cursor. Captions are listed only while one is shown.",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::TranscriptWord;

    fn word(index: usize, text: &str, start_ms: u64, end_ms: u64) -> TranscriptViewWord {
        TranscriptViewWord {
            word: TranscriptWord {
                id: format!("w-{index}"),
                text: text.into(),
                kind: WordKind::Word,
                source_start_us: start_ms * 1000,
                source_end_us: end_ms * 1000,
                confidence: None,
                speaker: None,
            },
            edited_start_us: Some(start_ms * 1000),
            edited_end_us: Some(end_ms * 1000),
            caption_hidden: false,
            caption_break: false,
            caption_join: false,
        }
    }

    #[test]
    fn a_call_for_another_project_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let (first, folder) = {
            let opened = host.state.opened_project.lock();
            let reader = opened.as_ref().unwrap();
            (
                reader.summary.project_handle.clone(),
                reader.root().to_string_lossy().to_string(),
            )
        };
        let revision = |host: &crate::mcp::tests_support::BareHost| {
            host.state
                .opened_project
                .lock()
                .as_ref()
                .unwrap()
                .document()
                .revision
        };
        let start = revision(&host);
        let edit = |project: Option<&str>| {
            let mut args = json!({
                "expectedRevision": start,
                "reason": "A track for a lower third",
                "edits": [{ "kind": "addTrack", "trackKind": "video" }],
            });
            if let Some(project) = project {
                args["project"] = json!(project);
            }
            args
        };
        let propose = find("propose_edit").unwrap();
        let mut ctx = Ctx::new(&host, "test", false);
        // Reopening gives the same project (same revision, same clip ids) a new id: a proposal
        // read before is refused, and nothing is stored.
        crate::commands::open_project_impl(&host.state, folder.clone()).unwrap();
        assert_eq!(revision(&host), start);
        let error = call(propose, &mut ctx, edit(Some(&first))).unwrap_err();
        assert_eq!(error, OTHER_PROJECT_MESSAGE);
        let proposals = std::path::Path::new(&folder)
            .join("agent")
            .join("proposals.json");
        assert!(!proposals.exists(), "nothing was proposed");
        // So is a direct action meant for the old one.
        let error = call(find("undo").unwrap(), &mut ctx, json!({ "project": first })).unwrap_err();
        assert_eq!(error, OTHER_PROJECT_MESSAGE);
        assert_eq!(revision(&host), start);
        // Without an id a proposal is refused too.
        assert!(call(propose, &mut ctx, edit(None))
            .unwrap_err()
            .contains("needs `project`"));
        // With the open project's id it goes through.
        let now = host
            .state
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .summary
            .project_handle
            .clone();
        assert_ne!(now, first);
        let out = call(propose, &mut ctx, edit(Some(&now))).unwrap();
        let Output::Text(text) = &out[0] else {
            panic!("text expected")
        };
        assert!(text.starts_with("Proposed b-1"), "{text}");
        // project_info reports the id to use.
        let Output::Text(info) =
            &call(find("project_info").unwrap(), &mut ctx, json!({})).unwrap()[0]
        else {
            panic!("text expected")
        };
        assert!(info.contains(&now), "{info}");
    }

    #[test]
    fn destructive_tools_exist_and_write() {
        for name in DESTRUCTIVE {
            let tool = find(name).unwrap_or_else(|| panic!("no tool {name}"));
            assert!(
                !tool.read_only,
                "{name} is destructive, so it can't be read-only"
            );
        }
        // Tools that write files are not read-only.
        for name in [
            "export",
            "graphic_render",
            "graphic_check",
            "graphic_write_file",
        ] {
            assert!(!find(name).unwrap().read_only, "{name} writes files");
        }
    }

    #[test]
    fn propose_edit_publishes_every_edit_kind() {
        let schema = schema::<super::super::propose::EditParams>().to_string();
        for kind in [
            "addTrack",
            "removeTrack",
            "setTrack",
            "moveTrack",
            "placeAsset",
            "moveClips",
            "trimClip",
            "split",
            "delete",
            "deleteRange",
            "rippleTrim",
            "link",
            "unlink",
            "setMagnetic",
            "setClip",
            "setSpeed",
            "restoreCuts",
        ] {
            assert!(schema.contains(&format!("\"{kind}\"")), "{kind} missing");
        }
        // Fields are camelCase, as the edits are parsed.
        assert!(schema.contains("clipIds") && schema.contains("assetId"));
        assert!(!schema.contains("clip_ids"));
    }

    #[test]
    fn clock_reads_minutes_and_hours() {
        assert_eq!(clock(0), "0:00.00");
        assert_eq!(clock(3_250_000), "0:03.25");
        assert_eq!(clock(61_000_000), "1:01.00");
        assert_eq!(clock(3_725_500_000), "1:02:05.50");
    }

    #[test]
    fn lines_break_at_sentences_and_pauses_and_show_long_gaps() {
        let words = [
            word(0, "Hello", 0, 300),
            word(1, "there.", 320, 700),
            word(2, "Today", 800, 1100),
            word(3, "we", 1150, 1300),
            word(4, "start", 2600, 3000),
        ];
        let refs: Vec<&TranscriptViewWord> = words.iter().collect();
        assert_eq!(
            pack_lines(&refs),
            "[0:00.00-0:00.70] w-0..w-1 Hello there.\n\
             [0:00.80-0:01.30] w-2..w-3 Today we\n  (pause 1.3s)\n\
             [0:02.60-0:03.00] w-4..w-4 start\n"
        );
    }

    #[test]
    fn every_tool_has_an_object_schema_and_reports_a_missing_project() {
        let host = super::super::tests_support::BareHost::default();
        let ctx = Ctx::new(&host, "test", false);
        for tool in all() {
            let schema = (tool.schema)();
            assert_eq!(schema["type"], "object", "{}", tool.name);
            assert!(find(tool.name).is_some());
            let args = match tool.name {
                "frame_render" | "layout_get" => json!({ "atUs": 0 }),
                "timeline_view" => json!({ "fromUs": 0, "toUs": 1_000_000 }),
                "propose_cuts" => json!({ "expectedRevision": 0, "reason": "r",
                    "cuts": [{ "fromWord": "w-0" }] }),
                "propose_edit" => json!({ "expectedRevision": 0, "reason": "r",
                    "edits": [{ "kind": "addTrack", "trackKind": "video" }] }),
                "proposal_withdraw" => json!({ "batchId": "b-1" }),
                "undo_batch" => json!({ "batchId": "b-1", "expectedRevision": 0 }),
                "settings" => json!({ "area": "captions" }),
                "short_create" => json!({ "title": "t", "fromWord": "w-0", "toWord": "w-1" }),
                "media_import" => json!({ "paths": ["a.mp4"] }),
                "short_from_take" => json!({ "title": "t", "camera": "m1" }),
                "short_footage" => json!({ "short": "s1" }),
                "graphic_add" => json!({ "template": "title", "atUs": 0 }),
                "graphic_set" => json!({ "graphic": "g", "name": "n" }),
                "brand_pick" => json!({ "brand": "none" }),
                "graphic_templatize" | "graphic_check" => json!({ "graphic": "g" }),
                "library_save" => json!({ "graphic": "g", "useWhen": "always" }),
                "graphic_write_file" => {
                    json!({ "graphic": "g", "path": "index.html", "contents": "" })
                }
                // These work without a project open (and installing would download).
                "transcript_fix" => json!({ "fixes": [{ "word": "w-0", "text": "x" }] }),
                "open" | "create" | "export_status" | "template" | "template_save" | "graphics"
                | "graphics_install" | "brand_kits" | "brand_kit_set" | "library_remove"
                | "more" => continue,
                _ => json!({}),
            };
            let error = (tool.run)(&ctx, args).unwrap_err();
            assert_eq!(error, NO_PROJECT_MESSAGE, "{}", tool.name);
        }
        assert!(params::<FrameParams>(json!({ "atUs": "soon" })).is_err());
    }
}
