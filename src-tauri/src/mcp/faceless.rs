//! `faceless`: faceless videos (FCL, docs/ROADMAP.md#fcl). A production is a project whose
//! picture is graphics and whose sound is generated narration, long (16:9) or short (9:16).
//! Find stories (discover, ideas, setAside), make a production (create), say where it stands.
use super::tools::{params, schema, text, Ctx, Output, ToolDef};
use crate::commands::faceless as fc;
use crate::faceless::evidence::{self, Brief, Dossier, FactInput, SourceStatus};
use crate::faceless::ideas::{Idea, Lane};
use crate::faceless::{Format, ProductionSummary, StageState};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

pub static TOOLS: [ToolDef; 4] = [
    ToolDef {
        name: "faceless_sources",
        description: "The research watchlist: X accounts tracked in groups (labs, leaders, \
            research, news, security, creators; each with a likes floor), YouTube channels \
            watched for what they cover, TikTok and Instagram creators. `op`: `read` (the \
            default) lists them with how many posts passed the filters and how often one made \
            the top 10. `add` with `platform` (x, youtube, tiktok, instagram), `handle` and, on \
            X, `group`; `remove` with `platform` and `handle`. `suggestX` proposes X accounts \
            the tracked ones follow (about half a cent); `findCreators` with `platform` (tiktok \
            or instagram) proposes who posts the most-watched videos on the channel's topics \
            (about a cent). Suggestions are proposals: add only what the user approves.",
        schema: schema::<SourcesArgs>,
        read_only: false,
        run: sources,
    },
    ToolDef {
        name: "faceless_script",
        description: "The open faceless production's script and its three reviews, checked by \
            the app. Follow the method (`preset`) for how to write and review. `op`: `read` (the \
            default): the script, the app's check and the review. `check` with `script` only \
            checks a draft; `write` with `script` checks and keeps it when nothing blocks it. \
            The script: {title, titleOptions, titleFactIds, hooks, spine, firstFrame, scenes: \
            [{narration, factIds, analysis, onScreen, source}], leftOut}. Blocking: a scene \
            citing no fact unless `analysis`, unknown fact ids, a number in a scene or the \
            title that none of its cited quotes has (write measured numbers in digits), a \
            sentence said twice, a length outside the format (long 875-1400 words, short \
            87-169 at the 1.2x voice). `review` with `findings` ([{review: attention | \
            coldReader | fairness, claim (exact script words), problem, severity: blocker | \
            minor, why, change}]) and `coldReader` (the cold reader's answers): a blocker holds \
            the video until the script no longer says it and `resolve` with `indexes` marks it \
            fixed.",
        schema: schema::<ScriptArgs>,
        read_only: false,
        run: script_tool,
    },
    ToolDef {
        name: "faceless_research",
        description: "Research for the open faceless production, with every fact checked by the \
            app. You search the web yourself; the app fetches each page and keeps a fact only \
            when its `quote` is found word for word on the page it fetched and every number in \
            the `statement` is in the quote. `op`: `read` (the default): sources, facts, refused \
            facts and the grade (`ready` needs 6 facts from 3 independent origins, or a primary \
            source and one more). `search` with `query` (and `limit`) searches the web through \
            Treg (a fraction of a cent; or search with your own tools). `youtube` with `query` \
            lists the videos already made on it, with views and age (a tenth of a cent): see \
            what was said and find the angle they missed. `fetch` with `urls` (up \
            to 20; start with the production's links): the app reads each page; with a Treg key, \
            Treg reads PDFs, Google News links and script-built pages; paywalls and sites that \
            refuse readers stay unreadable, so find the words elsewhere. `source` \
            with `url` (and `find`, a phrase, or `offset`) gives the page's text as the app sees \
            it, so you copy quotes exactly. `add` with `facts` ([{statement, quote, url, kind, \
            scope, uncertainty, counterpoint, numberContext}]). `remove` with `ids`. `brief` with \
            `brief` ({viewerQuestion, whyItMatters, beats citing [f1], counterpoints, \
            openQuestions, visualIdeas, formatFit}). Mark your own reasoning as analysis in the \
            script, never as a fact.",
        schema: schema::<ResearchArgs>,
        read_only: false,
        run: research,
    },
    ToolDef {
        name: "faceless",
        description: "Faceless videos: a long (5-8 min, 16:9) or short (30-58 s, 9:16) video made \
        from a story, with generated narration and graphics, no camera. `op`: `status` (the \
        default) says where the open production stands, stage by stage (research, script, \
        review, voice, visuals, package, publish: todo, done or stale); `discover` searches the \
        web for stories with the channel profile (about a minute; X and Reddit through Treg \
        cost about a cent); `ideas` ranks what was found for `format` (long or short: a table \
        of the top 10 with each score's parts and how it moved since the last search, then \
        repositories and papers; `days`, `limit`); `demand` checks the top `limit` (default \
        10) through Treg, a few cents: YouTube videos already on each story and their views, \
        Google searches for its subject and their trend, reels for shorts; it adds to the \
        score and shows in the table; `triage` with `reads` gives your editorial \
        read of the top stories (importance, longFit, shortFit 1 to 5, angle, reason; verdict \
        reject sets it aside), which counts for 40% of a story's score; `setAside` with `ids` \
        drops stories that aren't for the channel (`aside: false` brings them back); `pin` \
        with `id` puts one first; `separate` with `id` takes a report out of a group it doesn't \
        belong in; `join` with `ids` and `id` puts reports of one event worded differently into \
        that story's group; `create` with `idea` (an idea's id) or `topic`, and `format` (long \
        or short), \
        makes a production and opens it; `list` the productions; `made` the videos already \
        made. You pick the story with the user: never one marked made. `voice` (after the \
        review) reads the script with the channel's voice through OpenRouter on the user's \
        key, puts the narration on the timeline as speech, transcribes it and lists names and \
        numbers the transcription heard differently; only changed scenes are paid for again, \
        `scenes` redoes others. Ask the user to listen to those; `acceptVoice` only when they \
        say it is right. `visuals` checks the graphics you placed (`graphic_add` at the \
        narration's words, `actionWord` for each action) and records the stage when nothing \
        blocks it: nothing on screen anywhere blocks; more than 12 s without a change, too \
        many words to read, the same kind twice, a start between words and too few kinds are \
        noted; `fill: true` first stretches each graphic to meet the next.",
        schema: schema::<FacelessArgs>,
        read_only: false,
        run: faceless,
    },
];

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResearchOp {
    #[default]
    Read,
    Fetch,
    Source,
    Add,
    Remove,
    Brief,
    Search,
    Youtube,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResearchArgs {
    /// read (default), fetch, source, add, remove or brief.
    #[serde(default)]
    pub op: ResearchOp,
    /// fetch: pages to read.
    #[serde(default)]
    pub urls: Vec<String>,
    /// fetch: read pages already fetched again.
    #[serde(default)]
    pub again: bool,
    /// source: the page.
    #[serde(default)]
    pub url: Option<String>,
    /// source: show the text around this phrase.
    #[serde(default)]
    pub find: Option<String>,
    /// source: start at this character.
    #[serde(default)]
    pub offset: Option<usize>,
    /// add: facts to check and keep.
    #[serde(default)]
    pub facts: Vec<FactInput>,
    /// remove: fact ids.
    #[serde(default)]
    pub ids: Vec<String>,
    /// brief: your reading of the evidence.
    #[serde(default)]
    pub brief: Option<Brief>,
    /// search: what to look for.
    #[serde(default)]
    pub query: Option<String>,
    /// search: at most this many results (default 8).
    #[serde(default)]
    pub limit: Option<usize>,
}

const SOURCE_CHUNK: usize = 12_000;

fn dossier_text(d: &Dossier) -> String {
    let mut out = format!(
        "{}: {:?}, {} facts from {} independent origins.\n",
        d.topic,
        d.grade,
        d.facts.len(),
        d.origins.len()
    );
    out.push_str("Sources:\n");
    for s in &d.sources {
        let state = match s.status {
            SourceStatus::Fetched => format!(
                "read, {} characters{}",
                s.chars,
                s.via
                    .as_deref()
                    .map_or(String::new(), |v| format!(", via {v}"))
            ),
            SourceStatus::Unreadable => {
                format!("unreadable: {}", s.reason.as_deref().unwrap_or(""))
            }
            SourceStatus::Paywalled => format!("paywalled: {}", s.reason.as_deref().unwrap_or("")),
        };
        let date = s
            .published_on
            .as_deref()
            .map_or(String::new(), |d| format!(", published {d}"));
        out.push_str(&format!("- {} ({state}{date})\n", s.url));
    }
    if !d.facts.is_empty() {
        out.push_str("Facts:\n");
        for f in &d.facts {
            let also = d.corroboration.get(&f.id).copied().unwrap_or(0);
            out.push_str(&format!(
                "[{}] {}{}{} | \"{}\" | {}{}\n",
                f.id,
                f.input.statement,
                if f.primary { " (primary)" } else { "" },
                if also > 0 {
                    format!(" (also in {also} other origins)")
                } else {
                    String::new()
                },
                f.input.quote,
                f.domain,
                f.input
                    .scope
                    .as_deref()
                    .map_or(String::new(), |s| format!(" | scope: {s}"))
            ));
        }
    }
    let recent: Vec<_> = d.rejected.iter().rev().take(5).collect();
    if !recent.is_empty() {
        out.push_str("Recently refused:\n");
        for r in recent {
            out.push_str(&format!("- {} ({}): {}\n", r.statement, r.url, r.reason));
        }
    }
    if !d.brief.beats.is_empty() || !d.brief.why_it_matters.is_empty() {
        out.push_str(&format!(
            "Brief: {} {}\nBeats:\n{}",
            d.brief.viewer_question,
            d.brief.why_it_matters,
            d.brief
                .beats
                .iter()
                .map(|b| format!("- {b}\n"))
                .collect::<String>()
        ));
    }
    out
}

fn research(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ResearchArgs = params(args)?;
    let root = fc::production_root(ctx.state)?;
    match args.op {
        ResearchOp::Read => match evidence::load(&root) {
            Some(d) => text(dossier_text(&d)),
            None => {
                let links = crate::faceless::load(&root)
                    .map(|p| p.links)
                    .unwrap_or_default();
                text(format!(
                    "No research yet. Fetch the story's pages first ({}), then search for the \
                     primary source and independent reporting.",
                    if links.is_empty() {
                        "no links saved with it".to_string()
                    } else {
                        links.join(" ")
                    }
                ))
            }
        },
        ResearchOp::Youtube => {
            let query = args.query.ok_or("Give `query`")?;
            let (videos, cost) = fc::faceless_youtube_impl(&query)?;
            let lines = videos
                .iter()
                .take(args.limit.unwrap_or(12))
                .map(|v| {
                    format!(
                        "- {} | {} | {:.0} views{} | {}",
                        v.title,
                        v.channel,
                        v.views,
                        v.hours.map_or(String::new(), |h| if h < 48.0 {
                            format!(", {h:.0} h ago")
                        } else {
                            format!(", {:.0} days ago", h / 24.0)
                        }),
                        v.url
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            text(format!(
                "{lines}\n({} through Treg.) Use it to see what is already said and how it \
                 does; find an angle they missed, never copy one.",
                crate::faceless::treg::dollars(cost)
            ))
        }
        ResearchOp::Search => {
            let query = args.query.ok_or("Give `query`")?;
            let (results, cost) =
                fc::faceless_web_search_impl(ctx.state, &query, args.limit.unwrap_or(8))?;
            if results.is_empty() {
                return text(format!(
                    "Nothing found ({}).",
                    crate::faceless::treg::dollars(cost)
                ));
            }
            let lines = results
                .iter()
                .map(|r| format!("- {} | {}\n  {}", r.title, r.url, r.snippet))
                .collect::<Vec<_>>()
                .join("\n");
            text(format!(
                "{lines}\n({} through Treg.) Fetch the pages worth reading.",
                crate::faceless::treg::dollars(cost)
            ))
        }
        ResearchOp::Fetch => {
            if args.urls.is_empty() {
                return Err("Give `urls`".into());
            }
            ctx.progress(0.1, "Reading pages");
            let sources = fc::faceless_fetch_impl(ctx.state, &args.urls, args.again)?;
            let lines = sources
                .iter()
                .map(|s| match s.status {
                    SourceStatus::Fetched => format!(
                        "- {}: read, {} characters{}{}",
                        s.url,
                        s.chars,
                        s.via
                            .as_deref()
                            .map_or(String::new(), |v| format!(" (via {v})")),
                        if s.notices.is_empty() {
                            String::new()
                        } else {
                            format!(", notices: {}", s.notices.join(" / "))
                        }
                    ),
                    _ => format!(
                        "- {}: {:?}, {}",
                        s.url,
                        s.status,
                        s.reason.as_deref().unwrap_or("")
                    ),
                })
                .collect::<Vec<_>>()
                .join("\n");
            text(if lines.is_empty() {
                "Those pages were already read (`again: true` reads them again).".into()
            } else {
                format!("{lines}\nRead a page with `op: source` to copy quotes exactly.")
            })
        }
        ResearchOp::Source => {
            let url = args.url.ok_or("Give `url`")?;
            let page = evidence::source_text(&root, &url)
                .ok_or("That page was not read: `op: fetch` it first")?;
            let chars: Vec<char> = page.chars().collect();
            let start = match (&args.find, args.offset) {
                (Some(phrase), _) => {
                    let lower = page.to_lowercase();
                    let at = lower
                        .find(&phrase.to_lowercase())
                        .ok_or_else(|| format!("\"{phrase}\" is not on the page"))?;
                    lower[..at].chars().count().saturating_sub(1500)
                }
                (None, Some(offset)) => offset.min(chars.len()),
                (None, None) => 0,
            };
            let end = (start + SOURCE_CHUNK).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            text(format!(
                "{url}, characters {start}-{end} of {}:\n{chunk}{}",
                chars.len(),
                if end < chars.len() {
                    format!("\n[more: `offset: {end}`]")
                } else {
                    String::new()
                }
            ))
        }
        ResearchOp::Add => {
            if args.facts.is_empty() {
                return Err("Give `facts`".into());
            }
            let added = fc::faceless_add_facts_impl(ctx.state, args.facts)?;
            let refused = added
                .refused
                .iter()
                .map(|r| format!("- refused \"{}\": {}", r.statement, r.reason))
                .collect::<Vec<_>>()
                .join("\n");
            text(format!(
                "Kept {} ({}). The story is {:?}: {} facts from {} independent origins.{}{refused}",
                added.kept.len(),
                added.kept.join(", "),
                added.grade,
                added.facts,
                added.origins,
                if refused.is_empty() { "" } else { "\n" }
            ))
        }
        ResearchOp::Remove => {
            let d = evidence::remove_facts(&root, &args.ids)?;
            text(format!("Removed. {:?}, {} facts.", d.grade, d.facts.len()))
        }
        ResearchOp::Brief => {
            let brief = args.brief.ok_or("Give `brief`")?;
            let d = evidence::set_brief(&root, brief)?;
            text(format!(
                "Saved the brief. The story is {:?} with {} facts.",
                d.grade,
                d.facts.len()
            ))
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FacelessOp {
    #[default]
    Status,
    Discover,
    Ideas,
    Demand,
    Triage,
    SetAside,
    Pin,
    Separate,
    Join,
    Create,
    List,
    Made,
    Voice,
    AcceptVoice,
    Visuals,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct FacelessArgs {
    /// status (default), discover, ideas, setAside, create, list or made.
    #[serde(default)]
    pub op: FacelessOp,
    /// create: an idea's id from `ideas`; its title becomes the topic and its reports the links.
    #[serde(default)]
    pub idea: Option<u64>,
    /// create: what the video is about, when not from an idea.
    #[serde(default)]
    pub topic: Option<String>,
    /// create: the story's sources, when not from an idea.
    #[serde(default)]
    pub links: Vec<String>,
    /// create: long (default) or short.
    #[serde(default)]
    pub format: Format,
    /// ideas: how many days back (default 3, at most 7).
    #[serde(default)]
    pub days: Option<i64>,
    /// ideas: at most this many per lane (default 15).
    #[serde(default)]
    pub limit: Option<usize>,
    /// setAside: story ids (an idea's `ids`).
    #[serde(default)]
    pub ids: Vec<u64>,
    /// setAside: false brings them back.
    #[serde(default)]
    pub aside: Option<bool>,
    /// triage: your read of each story: [{id, verdict (keep or reject), importance, longFit,
    /// shortFit (1 to 5), angle, reason}].
    #[serde(default)]
    pub reads: Vec<fc::Triage>,
    /// ideas: also what the filters left out.
    #[serde(default)]
    pub all: bool,
    /// pin, separate: the story id; join: the story the `ids` join.
    #[serde(default)]
    pub id: Option<u64>,
    /// pin, separate: false undoes it.
    #[serde(default)]
    pub on: Option<bool>,
    /// voice: scenes (1-based) to voice again although their words didn't change.
    #[serde(default)]
    pub scenes: Vec<usize>,
    /// visuals: first stretch each graphic to meet the next (one undo step).
    #[serde(default)]
    pub fill: bool,
}

fn visuals_text(result: &fc::VisualsResult) -> String {
    let v = &result.visuals;
    let mut out = String::new();
    if result.filled > 0 {
        out.push_str(&format!(
            "Stretched {} graphic(s) to meet the next.\n",
            result.filled
        ));
    }
    out.push_str(&format!(
        "{} graphics over {:.0} s.",
        v.beats.len(),
        v.duration_us as f64 / 1e6
    ));
    if !v.errors.is_empty() {
        out.push_str("\nMust fix:");
        for e in &v.errors {
            out.push_str(&format!("\n- {e}"));
        }
    }
    if !v.notes.is_empty() {
        out.push_str("\nWorth a look:");
        for n in &v.notes {
            out.push_str(&format!("\n- {n}"));
        }
    }
    out.push_str(if result.done {
        "\nVisuals done. Check them with `storyboard`, then the package (`publish`)."
    } else {
        "\nVisuals not done until nothing blocks."
    });
    out
}

fn voice_text(result: &fc::VoiceResult) -> String {
    let v = &result.voice;
    let mut out = format!(
        "Narration: {} scenes, {:.1} s ({} voiced now{}), {} words heard, on the timeline as {}.",
        v.scenes.len(),
        v.duration_us as f64 / 1e6,
        v.generated,
        v.cost_usd
            .map(|c| format!(", about ${c:.2}"))
            .unwrap_or_default(),
        v.heard_words,
        v.sound.as_deref().unwrap_or("speech"),
    );
    for d in &result.diagnostics {
        out.push_str(&format!(" {d}"));
    }
    if let Some(problem) = &v.length_problem {
        out.push_str(&format!("\nLength: {problem}"));
    }
    if !v.issues.is_empty() {
        out.push_str(
            "\nHeard differently (ask the user to listen; fix the script, or `scenes` to voice \
             one again):",
        );
        for i in &v.issues {
            out.push_str(&format!("\n- scene {} {}: {}", i.scene, i.kind, i.expected));
        }
    }
    out.push_str(if result.done {
        "\nVoice done. Next: the visuals, timed on the narration's words."
    } else {
        "\nVoice not done until the user listens and accepts (`acceptVoice`) or the script is \
         fixed."
    });
    out
}

fn line(p: &ProductionSummary) -> String {
    let stages = p
        .stages
        .iter()
        .map(|s| {
            let mark = match s.state {
                StageState::Todo => "todo",
                StageState::Done => "done",
                StageState::Stale => "stale",
            };
            format!("{} {mark}", s.id)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let next = p
        .next
        .map_or("all done".to_string(), |n| format!("next: {n}"));
    let links = if p.links.is_empty() {
        String::new()
    } else {
        format!(" Sources to start from: {}.", p.links.join(" "))
    };
    format!(
        "{} ({}, {}): {stages}; {next}. {}{links}",
        p.topic,
        p.format.label(),
        p.created_at.get(..10).unwrap_or(&p.created_at),
        p.path
    )
}

fn idea_line(i: &Idea) -> String {
    let made = if i.made { " MADE ALREADY" } else { "" };
    let ids = if i.ids.len() > 1 {
        format!(" ids {:?}", i.ids)
    } else {
        String::new()
    };
    let editor = i.editorial.as_ref().map_or(String::new(), |e| {
        format!(
            " | read by {}: importance {}, long {}, short {}; {}",
            e.by, e.importance, e.long_fit, e.short_fit, e.angle
        )
    });
    format!(
        "{}. #{} {:.2} {}{made} | {} outlets, attention {:.0}{}, hooks: {} | {}{ids}{editor}",
        i.rank,
        i.id,
        i.score,
        i.title,
        i.outlets.len(),
        i.attention,
        if i.growth > 0.0 {
            format!(" (+{:.0})", i.growth)
        } else {
            String::new()
        },
        if i.hooks.is_empty() {
            "none".to_string()
        } else {
            i.hooks.join(", ")
        },
        i.urls.first().map(String::as_str).unwrap_or_default()
    )
}

fn faceless(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: FacelessArgs = params(args)?;
    match args.op {
        FacelessOp::Voice => {
            let result = ctx.transcript_job(|| {
                fc::faceless_voice_impl(
                    ctx.state,
                    ctx.host.transcripts(),
                    &args.scenes,
                    &mut |f, m| ctx.progress(f, m),
                )
            })?;
            if result.opened.is_some() {
                ctx.changed_project();
            }
            ctx.host.transcript_changed();
            text(voice_text(&result))
        }
        FacelessOp::Visuals => {
            let result = fc::faceless_visuals_impl(ctx.state, args.fill)?;
            if result.opened.is_some() {
                ctx.changed_project();
            }
            text(visuals_text(&result))
        }
        FacelessOp::AcceptVoice => text(voice_text(&fc::faceless_voice_accept_impl(ctx.state)?)),
        FacelessOp::Status => match fc::faceless_status_impl(ctx.state) {
            Some(p) => text(line(&p)),
            None => text(
                "The open project is not a faceless video (or none is open). `op: ideas` shows \
                 the stories found; `op: create` makes a production.",
            ),
        },
        FacelessOp::Discover => {
            let result = fc::faceless_discover_impl(&|fraction, name| {
                ctx.progress(fraction, &format!("Searching {name}"))
            })?;
            let funnels = result
                .funnels
                .iter()
                .map(|f| f.line())
                .collect::<Vec<_>>()
                .join("\n");
            text(format!(
                "Found {} new stories ({} kept from the last week){}.\n{funnels}\nNext: `op: ideas`.",
                result.new_stories,
                result.stories,
                if result.cost_micro > 0 {
                    format!(", Treg cost {}", crate::faceless::treg::dollars(result.cost_micro))
                } else {
                    String::new()
                }
            ))
        }
        FacelessOp::Ideas => {
            let mut view = fc::faceless_ideas_impl(
                args.days,
                Some(args.limit.unwrap_or(if args.all { 40 } else { 15 })),
                args.format,
            )?;
            if view.ideas.is_empty() {
                return text(format!(
                    "No stories yet{}. Run `op: discover` first.",
                    view.last_discover_at
                        .map_or(String::new(), |at| format!(" (last search {at})"))
                ));
            }
            // What was filtered out and why, counted, then only the shortlist unless `all`.
            let mut reasons: std::collections::BTreeMap<String, usize> = Default::default();
            for idea in view.ideas.iter().filter(|i| !i.shortlisted) {
                *reasons
                    .entry(idea.left_out.clone().unwrap_or_default())
                    .or_default() += 1;
            }
            let left_out = reasons
                .iter()
                .map(|(why, n)| format!("{n} {why}"))
                .collect::<Vec<_>>()
                .join(", ");
            if !args.all {
                view.ideas.retain(|i| i.shortlisted);
            }
            let mut out = format!(
                "Profile {}: {}\nLast search {}. The {} ranking: coverage (independent outlets), \
                 attention, fit (the channel's hooks), freshness, demand and an editor's read \
                 when given. {}`triage` the top ones with your read.\n\n{}",
                view.profile,
                view.about,
                view.last_discover_at.map_or("never".to_string(), |at| at
                    .format("%Y-%m-%d %H:%M UTC")
                    .to_string()),
                args.format.label(),
                if args.all || left_out.is_empty() {
                    String::new()
                } else {
                    format!("Shortlist only; left out by the filters: {left_out} (`all: true` shows them). ")
                },
                crate::faceless::ideas::review_table(&view.ideas, 10)
            );
            for (lane, heading) in [
                (Lane::Stories, "Stories"),
                (Lane::Repositories, "Viral repositories"),
                (Lane::Papers, "Papers"),
            ] {
                let members: Vec<&Idea> = view.ideas.iter().filter(|i| i.lane == lane).collect();
                if members.is_empty() {
                    continue;
                }
                out.push_str(&format!("\n\n{heading}:"));
                for idea in members {
                    out.push('\n');
                    out.push_str(&idea_line(idea));
                }
            }
            text(out)
        }
        FacelessOp::Demand => {
            ctx.progress(0.1, "Checking YouTube, Google searches and reels");
            let result = fc::faceless_demand_impl(args.format, args.limit.unwrap_or(10))?;
            let view = fc::faceless_ideas_impl(args.days, Some(10), args.format)?;
            text(format!(
                "Checked demand for {} stories through Treg ({}).{}\nDemand is who watches \
                 videos on the story (or its subject) and how many search for the subject; \
                 recent videos on it lower the score a little.\n\n{}",
                result.checked,
                crate::faceless::treg::dollars(result.cost_micro),
                if result.problems.is_empty() {
                    String::new()
                } else {
                    format!(" Problems: {}.", result.problems.join("; "))
                },
                crate::faceless::ideas::review_table(&view.ideas, 10)
            ))
        }
        FacelessOp::Triage => {
            if args.reads.is_empty() {
                return Err(
                    "Give `reads`: [{id, verdict, importance, longFit, shortFit, angle, reason}]"
                        .into(),
                );
            }
            let n = fc::faceless_triage_impl(args.reads, &ctx.agent)?;
            text(format!(
                "Saved {n} reads; they now count in the ranking (`op: ideas` shows the new order)."
            ))
        }
        FacelessOp::Join => {
            let to = args.id.ok_or("Give `id`: the story to join")?;
            if args.ids.is_empty() {
                return Err("Give `ids`: the reports to join to it".into());
            }
            fc::faceless_join_impl(&args.ids, to)?;
            text(format!(
                "Joined {} report(s) to story {to}; they count as one event now.",
                args.ids.len()
            ))
        }
        FacelessOp::Pin | FacelessOp::Separate => {
            let id = args.id.ok_or("Give `id`")?;
            let on = args.on.unwrap_or(true);
            if args.op == FacelessOp::Pin {
                fc::faceless_pin_impl(id, on)?;
            } else {
                fc::faceless_separate_impl(id, on)?;
            }
            text(format!(
                "Story {id} {}.",
                match (args.op, on) {
                    (FacelessOp::Pin, true) => "is pinned first",
                    (FacelessOp::Pin, false) => "is unpinned",
                    (_, true) => "no longer groups with other reports",
                    (_, false) => "may group again",
                }
            ))
        }
        FacelessOp::SetAside => {
            if args.ids.is_empty() {
                return Err("Give `ids`: story ids from `ideas`".into());
            }
            let aside = args.aside.unwrap_or(true);
            fc::faceless_set_aside_impl(&args.ids, aside)?;
            text(format!(
                "{} {} {}.",
                if aside { "Set aside" } else { "Brought back" },
                args.ids.len(),
                if args.ids.len() == 1 {
                    "story"
                } else {
                    "stories"
                }
            ))
        }
        FacelessOp::Create => {
            let (topic, links) = match args.idea {
                Some(id) => {
                    let view = fc::faceless_ideas_impl(Some(7), Some(100), args.format)?;
                    let idea = view
                        .ideas
                        .into_iter()
                        .find(|i| i.id == id || i.ids.contains(&id))
                        .ok_or_else(|| format!("No idea #{id}; read `op: ideas` again"))?;
                    (args.topic.unwrap_or(idea.title), idea.urls)
                }
                None => (
                    args.topic.ok_or("Give the video's `topic`, or an `idea`")?,
                    args.links,
                ),
            };
            let created = fc::faceless_create_impl(ctx.state, topic, args.format, links)?;
            ctx.host.project_opened(&created.production.path);
            ctx.changed_proposals();
            let warning = created.already_made.map_or(String::new(), |m| {
                format!(
                    "\nWarning: a {} video on \"{}\" was already made ({}). Ask the user before \
                     going on.",
                    m.format.label(),
                    m.topic,
                    m.kind
                )
            });
            text(format!(
                "Made and opened: {}{warning}",
                line(&created.production)
            ))
        }
        FacelessOp::List => {
            let all = fc::faceless_list_impl();
            if all.is_empty() {
                return text(format!(
                    "No faceless productions yet in {}.",
                    crate::faceless::home().display()
                ));
            }
            text(all.iter().map(line).collect::<Vec<_>>().join("\n"))
        }
        FacelessOp::Made => {
            let made = fc::faceless_made_impl();
            if made.is_empty() {
                return text("No faceless videos made yet.");
            }
            text(
                made.iter()
                    .map(|m| {
                        format!(
                            "{} ({}, {}{}){}",
                            m.topic,
                            m.format.label(),
                            m.kind,
                            m.youtube
                                .as_deref()
                                .map_or(String::new(), |id| format!(", youtube {id}")),
                            m.link.as_deref().map_or(String::new(), |l| format!(" {l}"))
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ScriptOp {
    #[default]
    Read,
    Check,
    Write,
    Review,
    Resolve,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScriptArgs {
    /// read (default), check, write, review or resolve.
    #[serde(default)]
    pub op: ScriptOp,
    /// check, write: the script.
    #[serde(default)]
    pub script: Option<crate::faceless::script::Script>,
    /// review: the findings of the three reviews.
    #[serde(default)]
    pub findings: Vec<crate::faceless::script::Finding>,
    /// review: the cold reader's answers.
    #[serde(default)]
    pub cold_reader: Option<Value>,
    /// resolve: indexes of findings now fixed in the script.
    #[serde(default)]
    pub indexes: Vec<usize>,
}

fn check_text(check: &crate::faceless::script::Check) -> String {
    let mut out = format!(
        "{} words, about {:.0} s once voiced.",
        check.words, check.estimated_seconds
    );
    if !check.errors.is_empty() {
        out.push_str("\nMust fix:");
        for e in &check.errors {
            out.push_str(&format!("\n- {e}"));
        }
    }
    if !check.notes.is_empty() {
        out.push_str("\nWorth a look:");
        for n in &check.notes {
            out.push_str(&format!("\n- {n}"));
        }
    }
    out
}

fn review_text(review: &crate::faceless::script::Review) -> String {
    let mut out = format!("Review: {} findings", review.findings.len());
    let open = review.open_blockers().len();
    if open > 0 {
        out.push_str(&format!(
            ", {open} blocker(s) open: the video holds until fixed"
        ));
    }
    for (i, f) in review.findings.iter().enumerate() {
        out.push_str(&format!(
            "\n{i}. [{} {}{}] \"{}\": {} → {}",
            f.review,
            f.severity,
            if f.resolved { ", fixed" } else { "" },
            f.claim,
            f.why,
            f.change
        ));
    }
    out
}

fn script_tool(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    use crate::faceless::script as fs;
    let args: ScriptArgs = params(args)?;
    let root = fc::production_root(ctx.state)?;
    let production = crate::faceless::load(&root).ok_or("Not a faceless video")?;
    match args.op {
        ScriptOp::Read => {
            let Some(script) = fs::load_script(&root) else {
                return text("No script yet. Read the research (`faceless_research`), then write one with `op: write`.");
            };
            let check = evidence::load(&root)
                .map(|d| fs::check(&script, &d, production.format, fs::DEFAULT_SPEED))
                .unwrap_or_default();
            let mut out = format!("Title: {}\n{}\n", script.title, check_text(&check));
            for (i, s) in script.scenes.iter().enumerate() {
                out.push_str(&format!(
                    "\n{}. {}{} [{}]",
                    i + 1,
                    if s.analysis { "(analysis) " } else { "" },
                    s.narration,
                    s.fact_ids.join(", ")
                ));
            }
            if let Some(review) = fs::load_review(&root) {
                out.push_str("\n\n");
                out.push_str(&review_text(&review));
            }
            text(out)
        }
        ScriptOp::Check | ScriptOp::Write => {
            let script = args.script.ok_or("Give `script`")?;
            if args.op == ScriptOp::Check {
                let dossier = evidence::load(&root).ok_or("Research the story first")?;
                let check = fs::check(&script, &dossier, production.format, fs::DEFAULT_SPEED);
                return text(check_text(&check));
            }
            let check = fs::write(&root, script, fs::DEFAULT_SPEED)?;
            text(format!(
                "{}\n{}",
                if check.errors.is_empty() {
                    "Kept the script. Next: the three reviews (`op: review`)."
                } else {
                    "Not kept: fix what must be fixed and write it again."
                },
                check_text(&check)
            ))
        }
        ScriptOp::Review => {
            let review = fs::save_review(
                &root,
                args.findings,
                args.cold_reader.unwrap_or(Value::Null),
            )?;
            text(review_text(&review))
        }
        ScriptOp::Resolve => {
            let review = fs::resolve(&root, &args.indexes)?;
            text(review_text(&review))
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SourcesOp {
    #[default]
    Read,
    Add,
    Remove,
    SuggestX,
    FindCreators,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourcesArgs {
    /// read (default), add, remove, suggestX or findCreators.
    #[serde(default)]
    pub op: SourcesOp,
    /// add, remove, findCreators: x, youtube, tiktok or instagram.
    #[serde(default)]
    pub platform: Option<String>,
    /// add, remove: the account's handle.
    #[serde(default)]
    pub handle: Option<String>,
    /// add on X: labs, leaders, research, news, security or creators (default creators).
    #[serde(default)]
    pub group: Option<String>,
    /// findCreators: what to search for (default: AI news, ChatGPT, Claude AI, OpenAI, ...).
    #[serde(default)]
    pub queries: Vec<String>,
}

fn candidate_line(c: &crate::faceless::watch::Candidate) -> String {
    let who = if c.followed_by.is_empty() {
        format!(
            "{} posts, {:.0} views together, e.g. {}",
            c.posts, c.views, c.example
        )
    } else {
        format!(
            "followed by {} tracked: {}",
            c.followed_by.len(),
            c.followed_by.join(", ")
        )
    };
    format!(
        "- @{} {} ({:.0} followers) | {who}{}",
        c.handle,
        c.name,
        c.followers,
        if c.bio.is_empty() {
            String::new()
        } else {
            format!(" | {}", c.bio.replace('\n', " "))
        }
    )
}

fn sources(_ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: SourcesArgs = params(args)?;
    match args.op {
        SourcesOp::Read => {
            let list = fc::faceless_watchlist_impl()?;
            let mut out = String::new();
            for platform in ["x", "youtube", "tiktok", "instagram"] {
                let rows: Vec<_> = list.iter().filter(|w| w.platform == platform).collect();
                out.push_str(&format!("\n{platform} ({}):", rows.len()));
                for w in rows {
                    out.push_str(&format!(
                        "\n- @{}{}{} | kept {}, top 10 {}{}",
                        w.handle.trim_start_matches('@'),
                        if w.group.is_empty() {
                            String::new()
                        } else {
                            format!(" [{}]", w.group)
                        },
                        if w.name.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", w.name)
                        },
                        w.kept,
                        w.top10,
                        w.usual
                            .map_or(String::new(), |u| format!(", usually {u:.0}"))
                    ));
                }
            }
            text(out.trim_start().to_string())
        }
        SourcesOp::Add | SourcesOp::Remove => {
            let platform = args.platform.ok_or("Give `platform`")?;
            let handle = args.handle.ok_or("Give `handle`")?;
            if args.op == SourcesOp::Add {
                fc::faceless_watch_add_impl(&platform, &handle, args.group.as_deref(), "", 0.0)?;
                text(format!(
                    "Now watching @{} on {platform}.",
                    handle.trim_start_matches('@')
                ))
            } else {
                fc::faceless_watch_remove_impl(&platform, &handle)?;
                text(format!(
                    "No longer watching @{} on {platform}.",
                    handle.trim_start_matches('@')
                ))
            }
        }
        SourcesOp::SuggestX => {
            let (found, cost) = fc::faceless_suggest_x_impl()?;
            if found.is_empty() {
                return text(format!(
                    "No new accounts followed by several tracked ones ({}).",
                    crate::faceless::treg::dollars(cost)
                ));
            }
            text(format!(
                "{}\n({} through Treg.) Propose the ones that fit; add only what the user approves.",
                found.iter().map(candidate_line).collect::<Vec<_>>().join("\n"),
                crate::faceless::treg::dollars(cost)
            ))
        }
        SourcesOp::FindCreators => {
            let platform = args
                .platform
                .ok_or("Give `platform`: tiktok or instagram")?;
            let (found, cost) = fc::faceless_find_creators_impl(&platform, Some(args.queries))?;
            if found.is_empty() {
                return text(format!(
                    "No creators found ({}).",
                    crate::faceless::treg::dollars(cost)
                ));
            }
            text(format!(
                "{}\n({} through Treg.) Propose the ones that fit; add only what the user approves.",
                found.iter().map(candidate_line).collect::<Vec<_>>().join("\n"),
                crate::faceless::treg::dollars(cost)
            ))
        }
    }
}
