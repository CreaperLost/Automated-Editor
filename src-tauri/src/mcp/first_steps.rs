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

/// The plan for the project open now.
pub fn plan_open(host: &dyn super::Host, online: bool) -> Result<Plan, String> {
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

/// The reason in a guarded pass's answer when it held back ("Not applied: ...").
fn held(answer: &str) -> Option<String> {
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
