//! Review by ear (REV-1). The agent can't hear, so it points: `playhead` shows the user a
//! moment, and `joins` hands them the cuts to listen to, which they mark keep or fix in the
//! Joins panel (see [`crate::joins`]).
use super::tools::{clock, params, schema, text, with_reader, Ctx, Output, ToolDef};
use crate::joins::{self, FIND_DEFAULT, MAX_JOINS};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

pub static TOOLS: [ToolDef; 2] = [
    ToolDef {
        name: "joins",
        description: "Cuts for the user to listen to (you can't hear). `op`: `list` \
            (default): each join's id, time, why, the user's mark (keep, FIX, - not heard) \
            and note. `find`: the app adds the cuts most likely to sound wrong (retakes, words \
            cut into, blips; `max`, default 30). `add` with `joins` [{atUs, reason}]: cuts you \
            doubt (snapped to a cut within 1.5 s). `clear` empties it. Ask the user to \
            listen in the Joins panel, read the marks and fix those marked FIX.",
        schema: schema::<JoinsArgs>,
        read_only: false,
        run: joins_tool,
    },
    ToolDef {
        name: "playhead",
        description: "The user's playhead on the edited timeline. `op`: `get` (the default): \
            where it is and whether it is playing; use it when the user says 'here' or 'now'. \
            `show` with `atUs`: moves their preview there, paused, to show them a moment.",
        schema: schema::<PlayheadArgs>,
        read_only: true,
        run: playhead,
    },
];

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum JoinsOp {
    /// The joins and the user's marks (changes nothing).
    #[default]
    List,
    /// The app adds the cuts most likely to sound wrong.
    Find,
    /// Adds `joins`.
    Add,
    /// Empties the list.
    Clear,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JoinAt {
    /// The cut, in edited microseconds.
    pub at_us: u64,
    /// Why it may sound wrong, in a few words (the user reads it).
    pub reason: String,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct JoinsArgs {
    /// list (default), find, add or clear.
    #[serde(default)]
    pub op: JoinsOp,
    /// add: the cuts.
    #[serde(default)]
    pub joins: Vec<JoinAt>,
    /// find: at most this many. Default 30.
    #[serde(default)]
    pub max: Option<usize>,
}

fn joins_tool(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: JoinsArgs = params(args)?;
    let view = match args.op {
        JoinsOp::List => with_reader(ctx.state, joins::view_for)?,
        JoinsOp::Find => {
            let max = args.max.unwrap_or(FIND_DEFAULT).clamp(1, MAX_JOINS);
            let view = with_reader(ctx.state, |reader| {
                joins::change(reader, |document, list| {
                    let risky = joins::risky(reader.root(), document, max);
                    joins::add(document, list, "app", &risky).map(|_| ())
                })
            })?;
            ctx.host.joins_changed();
            view
        }
        JoinsOp::Add => {
            if args.joins.is_empty() {
                return Err("Give `joins`: [{atUs, reason}].".into());
            }
            let moments: Vec<(u64, String)> = args
                .joins
                .into_iter()
                .map(|j| (j.at_us, j.reason))
                .collect();
            let view = with_reader(ctx.state, |reader| {
                joins::change(reader, |document, list| {
                    joins::add(document, list, &ctx.agent, &moments).map(|_| ())
                })
            })?;
            ctx.host.joins_changed();
            view
        }
        JoinsOp::Clear => {
            let view = with_reader(ctx.state, |reader| {
                joins::change(reader, |_, list| {
                    list.items.clear();
                    Ok(())
                })
            })?;
            ctx.host.joins_changed();
            view
        }
    };
    text(joins::text(&view))
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PlayheadOp {
    /// Where the playhead is.
    #[default]
    Get,
    /// Moves the user's preview to `atUs`, paused.
    Show,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayheadArgs {
    /// get (default) or show.
    #[serde(default)]
    pub op: PlayheadOp,
    /// show: the moment, in edited microseconds.
    #[serde(default)]
    pub at_us: Option<u64>,
}

fn playhead(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let state = ctx.state;
    let args: PlayheadArgs = params(args)?;
    let handle = with_reader(state, |reader| Ok(reader.summary.project_handle.clone()))?;
    if args.op == PlayheadOp::Show {
        let at = args.at_us.ok_or("`show` needs `atUs`.")?;
        // The video, not a short, and still: the user looks at the moment named.
        crate::commands::playback_focus_short_impl(state, handle.clone(), None, 0, false)?;
        let status = crate::commands::playback_seek_impl(state, handle, at)?;
        ctx.host.show_moment(status.position_us);
        return text(format!(
            "The user's preview shows {} (paused).",
            clock(status.position_us)
        ));
    }
    let status = state.playback.lock().status()?;
    let at = status.position_us;
    super::tools::pretty(&json!({
        "atUs": at,
        "at": clock(at),
        "playing": status.state == crate::playback::PlaybackState::Playing,
        "inShort": status.short_id,
    }))
}
