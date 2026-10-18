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
    /// Changes nothing in the project or its proposals.
    pub read_only: bool,
    pub run: Run,
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

static TOOLS: [ToolDef; 14] = [
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
            accepts: addTrack, placeAsset, moveClips, trimClip, split, delete, deleteRange, \
            setClip, link, unlink, setTrack, moveTrack, removeTrack. They are checked now on \
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
    let width = args.width.unwrap_or(960).clamp(240, 1920);
    let (frame, at) = super::frames::render(state, args.at_us, width)?;
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
                "open" | "create" | "export_status" | "template" | "template_save" | "graphics"
                | "graphics_install" | "brand_kits" => continue,
                _ => json!({}),
            };
            let error = (tool.run)(&ctx, args).unwrap_err();
            assert_eq!(error, NO_PROJECT_MESSAGE, "{}", tool.name);
        }
        assert!(params::<FrameParams>(json!({ "atUs": "soon" })).is_err());
    }
}
