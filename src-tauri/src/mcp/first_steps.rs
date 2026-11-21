//! A preset's first steps, run by the app itself (AUTO-1): after opening a project the user
//! picks Run, and the steps of the preset's method that need no agent run as an agent would
//! call them (the same tools, `guard: true`, one undo step each), up to where the method asks
//! the user or needs the agent. Only free steps on this computer run unless the user allows
//! online ones: transcription and the fillers' review then use their keys.
use super::tools::{self, with_reader, Ctx, Output, Stop};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::sync::Arc;

/// The name the history and the Agent panel give these steps.
pub const AUTHOR: &str = "AeroEdits";

/// One step the app runs: a tool call.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub label: String,
    /// It sends something off the computer, with the user's key.
    pub online: bool,
    #[serde(skip)]
    pub tool: &'static str,
    #[serde(skip)]
    pub args: Value,
}

/// What a run of the project's preset would do.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub preset: String,
    pub steps: Vec<Step>,
    /// What is left after them, for the agent or the user.
    pub left: Vec<String>,
}

/// What the run can use on this computer.
#[derive(Clone, Debug, Default)]
pub struct Ready {
    /// The user allowed online steps for this run.
    pub online: bool,
    /// An AI key is set for the fillers' review.
    pub ai_key: bool,
    /// The local review model is downloaded.
    pub local_model: bool,
    /// The recording has mouse data, for zooms.
    pub mouse: bool,
    /// The speech is transcribed already.
    pub transcribed: bool,
    /// Why the transcription can't run here, when the speech isn't transcribed yet.
    pub transcriber: Option<String>,
}

/// How each step went, as the run goes.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Event {
    Started {
        index: usize,
    },
    Progress {
        index: usize,
        fraction: f64,
        message: String,
    },
    /// The step ran: its answer, in a line or two.
    Done {
        index: usize,
        line: String,
    },
    /// The step waits for the user (its check found something that looks wrong) or failed:
    /// the run stops here.
    Stopped {
        index: usize,
        line: String,
    },
    /// Every step ran, or the run stopped.
    Finished {
        stopped: bool,
    },
}

/// The steps for a preset's `choices`: the scan and transcription always, then each cleanup
/// pass the choices turn on, in the method's order. A preset without choices stops after the
/// transcription, where its method asks its questions.
pub fn plan(preset: &str, choices: &Map<String, Value>, ready: Ready) -> Plan {
    let mut steps = vec![Step {
        label: "Measure the recording".into(),
        online: false,
        tool: "scan",
        args: json!({}),
    }];
    // Every pass after it needs the words.
    if let Some(why) = &ready.transcriber {
        return Plan {
            preset: preset.into(),
            steps,
            left: vec![format!(
                "Transcription: {why}{}. The cleanup waits for it.",
                match ready.online {
                    true => "",
                    false => "; allow online steps to transcribe with your key",
                }
            )],
        };
    }
    match (ready.transcribed, ready.online) {
        (true, _) => {}
        (false, true) => steps.push(Step {
            label: "Transcribe online (ElevenLabs or OpenRouter, your key)".into(),
            online: true,
            tool: "transcribe",
            args: json!({}),
        }),
        (false, false) => steps.push(Step {
            label: "Transcribe on this computer (Parakeet, English)".into(),
            online: false,
            tool: "transcribe",
            args: json!({ "provider": "parakeet" }),
        }),
    }
    let mut left = Vec::new();
    if choices.is_empty() {
        left.push(format!(
            "{preset} has no saved choices yet: the agent asks its questions first."
        ));
        return Plan {
            preset: preset.into(),
            steps,
            left,
        };
    }
    let on = |part: &str| {
        choices
            .get(part)
            .and_then(|c| c.get("apply"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    let strength = |part: &str| {
        choices
            .get(part)
            .and_then(|c| c.get("preset"))
            .and_then(Value::as_str)
            .unwrap_or("balanced")
            .to_string()
    };
    if on("mouthNoises") {
        let sensitivity = match strength("mouthNoises").as_str() {
            "gentle" => 0.35,
            "tight" => 0.6,
            _ => 0.5,
        };
        steps.push(Step {
            label: format!("Mute mouth noises ({})", strength("mouthNoises")),
            online: false,
            tool: "mouth_noises",
            args: json!({ "sensitivity": sensitivity, "guard": true }),
        });
    }
    if on("silence") {
        steps.push(Step {
            label: format!("Cut pauses ({})", strength("silence")),
            online: false,
            tool: "silence",
            args: json!({ "preset": strength("silence"), "guard": true }),
        });
    }
    if on("gaps") {
        steps.push(Step {
            label: format!("Cut gaps between words ({})", strength("gaps")),
            online: false,
            tool: "silence",
            args: json!({ "mode": "gaps", "preset": strength("gaps"), "guard": true }),
        });
    }
    if on("fillers") {
        // Retakes are a question in the method, so only the fillers are cut here.
        let review = match (ready.online && ready.ai_key, ready.local_model) {
            (true, _) => Some(("the AI review, your key", true, None)),
            (false, true) => Some(("the local review", false, Some("local"))),
            (false, false) => None,
        };
        match review {
            Some((who, online, review)) => {
                let mut args = json!({ "kinds": ["filler", "drink"], "guard": true });
                if let Some(review) = review {
                    args["review"] = json!(review);
                }
                steps.push(Step {
                    label: format!("Cut fillers ({who})"),
                    online,
                    tool: "fillers",
                    args,
                });
                left.push("Retakes: the agent lists them and you pick.".into());
            }
            None => left.push(
                "Fillers and retakes: no review can run on this computer (download the local \
                 model in the AI settings, or allow online steps), so the agent reviews them."
                    .into(),
            ),
        }
    }
    if on("zooms") && ready.mouse {
        let settings = choices
            .get("zooms")
            .and_then(|z| z.get("settings"))
            .cloned()
            .unwrap_or(Value::Null);
        let mut args = json!({ "op": "add" });
        if settings.is_object() {
            args["settings"] = settings;
        }
        steps.push(Step {
            label: "Add zooms".into(),
            online: false,
            tool: "zooms",
            args,
        });
    }
    if on("graphics") {
        left.push("Graphics: the agent proposes them as a table first.".into());
    }
    left.push("Chapters and the export (only when you say go): the agent.".into());
    Plan {
        preset: preset.into(),
        steps,
        left,
    }
}

/// Where a faceless production stands, as its run needs it (FCL-10).
#[derive(Clone, Debug, Default)]
pub struct Production {
    /// The first stage not done (research, script, review, voice, visuals, package,
    /// publish); none when every stage is done.
    pub next: Option<String>,
    /// The narration is accepted: the video's length is set.
    pub voiced: bool,
    /// Graphics on the timeline.
    pub graphics: usize,
    /// The story's links not read yet.
    pub unread: Vec<String>,
    /// A music bed is on the timeline.
    pub music_placed: bool,
    /// A sound effect starts on the first frame.
    pub opening_placed: bool,
    /// The bed the choices' `music` `query` finds first: its id, and whether it is on this
    /// computer already.
    pub music: Option<(String, bool)>,
}

/// The steps for a faceless production: what the app can do without the agent or a paid call
/// at the stage it is at, then what is left. Writing, the paid narration, the graphics and the
/// package stay with the agent and the user.
pub fn plan_faceless(
    preset: &str,
    choices: &Map<String, Value>,
    production: &Production,
    online: bool,
) -> Plan {
    let mut steps = Vec::new();
    let mut left = Vec::new();
    let next = production.next.as_deref();
    let on = |part: &str| {
        choices
            .get(part)
            .and_then(|c| c.get("apply"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    if next == Some("research") && !production.unread.is_empty() {
        if online {
            steps.push(Step {
                label: format!(
                    "Read the story's {} (fetched from the web, free)",
                    plural(production.unread.len(), "report")
                ),
                online: true,
                tool: "faceless_research",
                args: json!({ "op": "fetch", "urls": production.unread }),
            });
        } else {
            left.push("The story's reports: allow online steps to read them.".into());
        }
    }
    if production.voiced {
        if production.graphics > 0 && next == Some("visuals") {
            steps.push(Step {
                label: "Check the visuals, each graphic stretched to meet the next".into(),
                online: false,
                tool: "faceless",
                args: json!({ "op": "visuals", "fill": true }),
            });
        }
        if on("opening") && !production.opening_placed {
            if let Some(sound) = choices["opening"].get("sound").and_then(Value::as_str) {
                steps.push(Step {
                    label: format!("Opening sound ({sound}) on the first frame"),
                    online: false,
                    tool: "library",
                    args: json!({ "kind": "sound", "op": "use", "name": sound, "atUs": 0 }),
                });
            }
        }
        if on("music") && !production.music_placed {
            match &production.music {
                Some((id, here)) if *here || online => steps.push(Step {
                    label: format!(
                        "Music bed under the narration ({id}{})",
                        if *here { "" } else { ", downloaded once" }
                    ),
                    online: !here,
                    tool: "library",
                    args: json!({ "kind": "music", "op": "use", "name": id, "atUs": 0 }),
                }),
                Some(_) => left.push(
                    "Music: the bed isn't on this computer yet; allow online steps to download it."
                        .into(),
                ),
                None => left.push("Music: the agent picks a bed that suits the story.".into()),
            }
        }
    }
    left.push(
        match next {
            Some("research") => {
                "Research and the brief: the agent, from the pages read, with facts quoted exactly."
            }
            Some("script") | Some("review") => "The script and its three reviews: the agent.",
            Some("voice") => {
                "Narration: paid (your OpenRouter key, a few cents a short, about 30 cents a \
                 long); the agent voices it when you say, then you listen and accept it."
            }
            Some("visuals") if production.graphics == 0 => {
                "Graphics: the agent plans them as a table first."
            }
            Some("visuals") => "What the visuals check finds: the agent fixes it.",
            Some("package") => {
                "The package (title, description with sources, tags): the agent; you see it on \
                 the Export page."
            }
            _ => "Nothing else before the export.",
        }
        .to_string(),
    );
    left.push("Export and upload: only when you say go.".into());
    Plan {
        preset: preset.into(),
        steps,
        left,
    }
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

/// The plan for the project open now.
pub fn plan_open(host: &dyn super::Host, online: bool) -> Result<Plan, String> {
    if let Some(plan) = faceless_plan_open(host, online)? {
        return Ok(plan);
    }
    let (preset, mouse, transcribed) = with_reader(host.state(), |reader| {
        let mouse = reader.recordings().any(|a| {
            crate::telemetry::reader::read_telemetry(std::path::Path::new(&a.path))
                .is_ok_and(|t| !t.events.is_empty())
        });
        let transcribed = crate::commands::has_speech_transcript(reader);
        Ok((crate::preset::of(reader.document()), mouse, transcribed))
    })?;
    let mut choices = preset.choices.clone();
    super::skill::forget_review(&mut choices);
    let transcriber = match transcribed {
        true => None,
        false => super::actions::not_ready(
            &crate::commands::transcript::transcript_settings_get_impl(),
            match online {
                true => crate::transcript::ProviderKind::ElevenLabs,
                false => crate::transcript::ProviderKind::Parakeet,
            },
        ),
    };
    let ready = Ready {
        online,
        ai_key: host.ai_review_ready(),
        local_model: host.local_review_ready(),
        mouse,
        transcribed,
        transcriber,
    };
    Ok(plan(&preset.name, &choices, ready))
}

/// The plan for the open project when it is a faceless production.
fn faceless_plan_open(host: &dyn super::Host, online: bool) -> Result<Option<Plan>, String> {
    use crate::sequence::Role;
    let found = with_reader(host.state(), |reader| {
        let root = reader.root();
        let Some(summary) = crate::faceless::summary(root) else {
            return Ok(None);
        };
        let document = reader.document();
        let voiced = summary
            .stages
            .iter()
            .any(|s| s.id == "voice" && s.state == crate::faceless::StageState::Done);
        let read: Vec<String> = crate::faceless::evidence::load(root)
            .map(|d| d.sources.into_iter().map(|s| s.url).collect())
            .unwrap_or_default();
        let unread = summary
            .links
            .iter()
            .filter(|l| !read.contains(l))
            .cloned()
            .collect();
        let mut graphics = 0;
        let mut music_placed = false;
        let mut opening_placed = false;
        for (_, clip) in document.sequence.clips() {
            let Some(asset) = document.asset(&clip.asset) else {
                continue;
            };
            if asset.is_graphic() {
                graphics += 1;
            }
            let role = |r: Role| asset.streams.iter().any(|s| s.role == r);
            music_placed |= role(Role::Music);
            opening_placed |=
                role(Role::Effects) && clip.start_us <= crate::faceless::voice::LEAD_US;
        }
        Ok(Some((
            crate::preset::of(document),
            Production {
                next: summary.next.map(String::from),
                voiced,
                graphics,
                unread,
                music_placed,
                opening_placed,
                music: None,
            },
        )))
    })?;
    let Some((preset, mut production)) = found else {
        return Ok(None);
    };
    let query = preset
        .choices
        .get("music")
        .and_then(|m| m.get("query"))
        .and_then(Value::as_str)
        .map(String::from);
    production.music = crate::commands::music::music_library_list_impl(query)
        .ok()
        .and_then(|tracks| tracks.into_iter().next())
        .map(|t| (t.id, t.downloaded));
    Ok(Some(plan_faceless(
        &preset.name,
        &preset.choices,
        &production,
        online,
    )))
}

/// The reason in a guarded pass's answer when it held back ("Not applied: ..."), or the
/// visuals check's when something blocks them.
fn held(answer: &str) -> Option<String> {
    if let Some(at) = answer.find("\nMust fix:") {
        let fixes = answer[at + 1..]
            .lines()
            .skip(1)
            .take_while(|l| l.starts_with("- "))
            .count();
        return Some(format!(
            "Waits for the agent: the visuals check found {} to fix.",
            plural(fixes, "thing")
        ));
    }
    let start = answer.find("Not applied: ")? + "Not applied: ".len();
    let reason = &answer[start..];
    let end = reason.find(". Ask the user").unwrap_or(reason.len());
    Some(format!("Waits for you: {}.", &reason[..end]))
}

/// Runs the plan's steps one after another on the project open now, telling `report` how each
/// goes; stops at the first that waits for the user or fails, or when `stop` is set.
pub fn run(host: &dyn super::Host, plan: &Plan, stop: Arc<Stop>, report: &(dyn Fn(Event) + Sync)) {
    let mut stopped = false;
    for (index, step) in plan.steps.iter().enumerate() {
        if stop.stopped() {
            stopped = true;
            break;
        }
        report(Event::Started { index });
        let outcome = with_reader(host.state(), |reader| {
            Ok(reader.summary.project_handle.clone())
        })
        .and_then(|project| {
            let tool = tools::find(step.tool).ok_or("No such step")?;
            let mut args = step.args.clone();
            if tools::takes_project(tool) {
                args["project"] = json!(project);
            }
            let progress = move |fraction: f64, message: &str| {
                report(Event::Progress {
                    index,
                    fraction,
                    message: message.to_string(),
                })
            };
            let mut ctx = Ctx::new(host, AUTHOR, false)
                .with_progress(Box::new(progress))
                .with_stop(stop.clone());
            let out = tools::call(tool, &mut ctx, args);
            if ctx.project_was_changed() {
                host.project_changed();
            }
            out
        });
        let answer = outcome.map(|outputs| {
            outputs
                .into_iter()
                .filter_map(|o| match o {
                    Output::Text(text) => Some(text),
                    Output::Png(_) | Output::Jpeg(_) => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        });
        match answer {
            Ok(line) => match held(&line) {
                Some(reason) => {
                    report(Event::Stopped {
                        index,
                        line: reason,
                    });
                    stopped = true;
                    break;
                }
                None => report(Event::Done { index, line }),
            },
            Err(error) => {
                report(Event::Stopped { index, line: error });
                stopped = true;
                break;
            }
        }
    }
    report(Event::Finished { stopped });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tutorial() -> Map<String, Value> {
        crate::preset::builtin()
            .into_iter()
            .find(|p| p.name == crate::preset::DEFAULT)
            .unwrap()
            .choices
    }

    fn tools_of(plan: &Plan) -> Vec<&str> {
        plan.steps.iter().map(|s| s.tool).collect()
    }

    fn faceless() -> Map<String, Value> {
        crate::preset::builtin()
            .into_iter()
            .find(|p| p.name == crate::preset::FACELESS_SHORT)
            .unwrap()
            .choices
    }

    #[test]
    fn a_faceless_run_does_the_app_s_steps_at_each_stage_and_never_a_paid_one() {
        // Just made from an idea: read its reports (online), then the agent researches.
        let fresh = Production {
            next: Some("research".into()),
            unread: vec!["https://a.example/1".into(), "https://b.example/2".into()],
            ..Production::default()
        };
        let plan = plan_faceless("Faceless short", &faceless(), &fresh, true);
        assert_eq!(tools_of(&plan), ["faceless_research"]);
        assert_eq!(plan.steps[0].args["op"], "fetch");
        assert_eq!(plan.steps[0].args["urls"].as_array().unwrap().len(), 2);
        assert!(plan.steps[0].online);
        assert!(plan.left[0].starts_with("Research"));
        let offline = plan_faceless("Faceless short", &faceless(), &fresh, false);
        assert!(offline.steps.is_empty());
        assert!(offline.left[0].contains("allow online steps"));

        // Waiting for the narration: paid, so it is left to the agent at the user's word.
        let script_done = Production {
            next: Some("voice".into()),
            ..Production::default()
        };
        let plan = plan_faceless("Faceless short", &faceless(), &script_done, true);
        assert!(plan.steps.is_empty());
        assert!(plan.left[0].contains("paid"), "{:?}", plan.left);

        // Voiced, graphics placed: the visuals check, the opening hit and the bed.
        let voiced = Production {
            next: Some("visuals".into()),
            voiced: true,
            graphics: 7,
            music: Some(("calm-bed".into(), true)),
            ..Production::default()
        };
        let plan = plan_faceless("Faceless short", &faceless(), &voiced, false);
        assert_eq!(tools_of(&plan), ["faceless", "library", "library"]);
        assert_eq!(plan.steps[0].args, json!({ "op": "visuals", "fill": true }));
        assert_eq!(plan.steps[1].args["name"], "hit-soft");
        assert_eq!(plan.steps[1].args["atUs"], 0);
        assert_eq!(plan.steps[2].args["name"], "calm-bed");
        assert!(plan.steps.iter().all(|s| !s.online));
        assert!(plan.left.iter().any(|l| l.contains("only when you say go")));

        // Placed already, the sound steps are not repeated; a bed still to download needs
        // online steps.
        let placed = Production {
            next: Some("package".into()),
            voiced: true,
            graphics: 7,
            opening_placed: true,
            music: Some(("calm-bed".into(), false)),
            ..Production::default()
        };
        let plan = plan_faceless("Faceless short", &faceless(), &placed, false);
        assert!(plan.steps.is_empty());
        assert!(plan.left[0].contains("download"));
        let plan = plan_faceless("Faceless short", &faceless(), &placed, true);
        assert_eq!(tools_of(&plan), ["library"]);
        assert!(plan.steps[0].online);
        let done = Production {
            music_placed: true,
            ..placed
        };
        assert!(plan_faceless("Faceless short", &faceless(), &done, true)
            .steps
            .is_empty());
    }

    #[test]
    fn an_open_faceless_production_gets_its_own_plan() {
        let dir = tempfile::tempdir().unwrap();
        let root = crate::faceless::create(
            dir.path(),
            "Robot",
            crate::faceless::Format::Short,
            &["https://a.example/report".into()],
        )
        .unwrap();
        let host = crate::mcp::tests_support::BareHost::default();
        let opened =
            crate::commands::open_project_impl(&host.state, root.to_string_lossy().into()).unwrap();
        crate::commands::preset::project_preset_set_impl(
            &host.state,
            opened.project_handle,
            opened.revision,
            crate::preset::FACELESS_SHORT.into(),
            crate::commands::preset::PresetTarget::Video,
        )
        .unwrap();
        let online = plan_open(&host, true).unwrap();
        assert_eq!(online.preset, crate::preset::FACELESS_SHORT);
        assert_eq!(
            tools_of(&online),
            ["faceless_research"],
            "no scan or transcription"
        );
        assert_eq!(
            online.steps[0].args["urls"],
            json!(["https://a.example/report"])
        );
        assert!(plan_open(&host, false).unwrap().steps.is_empty());
    }

    #[test]
    fn the_visuals_check_finding_something_stops_the_run() {
        let answer = "7 graphics over 40 s.\nMust fix:\n- a gap\n- another\nWorth a look:\n- x";
        assert_eq!(
            held(answer).as_deref(),
            Some("Waits for the agent: the visuals check found 2 things to fix.")
        );
        assert_eq!(held("7 graphics over 40 s.\nVisuals done."), None);
    }

    #[test]
    fn the_tutorial_runs_its_free_passes_and_leaves_the_rest_to_the_agent() {
        let free = plan(
            "Tutorial",
            &tutorial(),
            Ready {
                local_model: true,
                mouse: true,
                ..Ready::default()
            },
        );
        assert_eq!(
            tools_of(&free),
            [
                "scan",
                "transcribe",
                "mouth_noises",
                "silence",
                "fillers",
                "zooms"
            ]
        );
        assert!(free.steps.iter().all(|s| !s.online));
        assert_eq!(free.steps[1].args["provider"], "parakeet");
        assert_eq!(free.steps[4].args["review"], "local");
        assert_eq!(free.steps[4].args["kinds"], json!(["filler", "drink"]));
        assert!(free.steps[2..5].iter().all(|s| s.args["guard"] == true));
        assert!(free.left.iter().any(|l| l.starts_with("Retakes")));
        assert!(free.left.iter().any(|l| l.contains("export")));

        // Allowed online with a key: the online transcription and review, nothing else.
        let online = plan(
            "Tutorial",
            &tutorial(),
            Ready {
                online: true,
                ai_key: true,
                ..Ready::default()
            },
        );
        assert_eq!(
            tools_of(&online),
            ["scan", "transcribe", "mouth_noises", "silence", "fillers"],
            "no zooms without mouse data"
        );
        assert!(online.steps[1].online && online.steps[4].online);
        assert!(online.steps[4].args.get("review").is_none());
    }

    #[test]
    fn without_a_review_or_choices_the_run_stops_sooner() {
        let no_review = plan("Tutorial", &tutorial(), Ready::default());
        assert!(!tools_of(&no_review).contains(&"fillers"));
        assert!(no_review.left[0].starts_with("Fillers and retakes"));

        let fresh = plan("Mine", &Map::new(), Ready::default());
        assert_eq!(tools_of(&fresh), ["scan", "transcribe"]);
        assert!(fresh.left[0].contains("no saved choices"));

        // Nothing can transcribe here: the passes that need the words wait.
        let no_words = plan(
            "Tutorial",
            &tutorial(),
            Ready {
                transcriber: Some("Parakeet is not part of this build".into()),
                ..Ready::default()
            },
        );
        assert_eq!(tools_of(&no_words), ["scan"]);
        assert!(
            no_words.left[0].contains("allow online steps"),
            "{:?}",
            no_words.left
        );
    }

    #[test]
    fn a_guarded_pass_that_held_back_stops_the_run_with_its_reason() {
        let answer = "Silence: 40 pauses.\nNot applied: it would cut 31% of the video. Ask the \
            user with these numbers; on their go, run it again without `guard`.";
        assert_eq!(
            held(answer).unwrap(),
            "Waits for you: it would cut 31% of the video."
        );
        assert_eq!(held("Cut 40 pauses."), None);
    }

    #[test]
    fn the_steps_run_through_the_tools_and_stop_where_a_check_holds_back() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let mut choices = Map::new();
        choices.insert("mouthNoises".into(), json!({ "apply": true }));
        choices.insert(
            "silence".into(),
            json!({ "apply": true, "preset": "balanced" }),
        );
        // The lesson is transcribed already: no transcription step.
        let plan = plan(
            "Tutorial",
            &choices,
            Ready {
                transcribed: true,
                ..Ready::default()
            },
        );
        assert_eq!(tools_of(&plan), ["scan", "mouth_noises", "silence"]);
        let events = std::sync::Mutex::new(Vec::new());
        run(&host, &plan, Default::default(), &|e| {
            if !matches!(e, Event::Progress { .. }) {
                events.lock().unwrap().push(e)
            }
        });
        let events = events.into_inner().unwrap();
        let ended: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::Done { index, .. } => Some((*index, "done")),
                Event::Stopped { index, .. } => Some((*index, "stopped")),
                _ => None,
            })
            .collect();
        // The lesson's pauses are over a third of it, so the silence pass waits for the user.
        assert_eq!(
            ended,
            [(0, "done"), (1, "done"), (2, "stopped")],
            "{events:?}"
        );
        assert!(events.iter().any(
            |e| matches!(e, Event::Stopped { line, .. } if line.starts_with("Waits for you"))
        ));
        assert_eq!(events.last(), Some(&Event::Finished { stopped: true }));
        // An agent that comes after is told what ran, so it doesn't run it again.
        let mut ctx = Ctx::new(&host, "claude-code", false);
        let status = tools::call(tools::find("status").unwrap(), &mut ctx, json!({})).unwrap();
        let Output::Text(status) = &status[0] else {
            panic!("status is text")
        };
        assert!(
            status.contains("The app already ran the preset's mouth_noises here"),
            "{status}"
        );
    }
}
