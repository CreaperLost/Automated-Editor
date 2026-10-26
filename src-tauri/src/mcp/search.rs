//! `source_search`: what was said, in every transcript, cut or not (see
//! [`crate::source_search`]).
use super::tools::{params, schema, text, with_reader, Ctx, Output, ToolDef};
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
        (`propose_edit` `restoreCuts`).",
    schema: schema::<SearchArgs>,
    read_only: true,
    run: source_search_tool,
}];

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchArgs {
    /// A word or phrase.
    pub query: String,
    /// Only this stream (`<asset>.<stream>`). Default: every transcribed one.
    #[serde(default)]
    pub stream: Option<String>,
    /// At most this many hits. Default 20, at most 100.
    #[serde(default)]
    pub limit: Option<usize>,
}

fn source_search_tool(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: SearchArgs = params(args)?;
    let results = with_reader(ctx.state, |reader| {
        source_search::search(
            reader.root(),
            &reader.history().current,
            &args.query,
            args.stream.as_deref(),
            args.limit.unwrap_or(LIMIT_DEFAULT),
        )
    })?;
    text(source_search::text(&results, args.query.trim()))
}
