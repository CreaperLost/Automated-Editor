//! The app's own buttons, for an agent: transcription, jump cuts, mouth noises, fillers and
//! retakes, webcam focus, zooms, settings, chapters, shorts and export. Each runs the same
//! built-in pass the UI runs (no model reads the video), applies at once as one undo step like a
//! click, and answers with a line or two the agent can pass on to the user. What changed the
//! edit is listed in the Agent panel's history, with Undo.
//!
//! Every action is an MCP tool and a CLI command (`aeroedits do <action> --key value`, see
//! [`super::cli`]).
use super::tools::{clock, params, schema, text, with_reader, Ctx, Output, ToolDef};
use crate::commands::{self, proposals::with_proposals, transcript as tc};
use crate::project::ProjectReader;
use crate::sequence::{Role, StreamKind};
use crate::transcript::{store, SuggestionSource, TranscriptSuggestionKind};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

pub static ACTIONS: [ToolDef; 17] = [
    ToolDef {
        name: "status",
        description: "Where the open project stands, in a few lines: length, sounds and whether \
            they are transcribed, mouth noises muted, zooms, webcam focus, captions, chapters, \
            shorts, and anything running. Call it first and after a cleanup.",
        schema: schema::<NoArgs>,
        read_only: true,
        run: status,
    },
    ToolDef {
        name: "scan",
        description: "An overview before an edit, changing nothing: microphones, cameras, mouse \
            data, the transcribers and AI key ready, and a first estimate of each pass for a \
            preset (mouth noises, silence, gaps, fillers and retakes, webcam focus, zooms). \
            Each pass changes what the next finds, so before running one, ask with its own dry \
            run (`apply: false`), not these numbers.",
        schema: schema::<ScanArgs>,
        read_only: true,
        run: scan,
    },
    ToolDef {
        name: "open",
        description: "Opens a project folder or a .aero recording folder in the editor window, \
            replacing the one open.",
        schema: schema::<OpenArgs>,
        read_only: false,
        run: open,
    },
    ToolDef {
        name: "create",
        description: "Makes a new project (in the default projects folder unless `location`) and \
            opens it. With `recording`, it edits that .aero recording without writing to it.",
        schema: schema::<CreateArgs>,
        read_only: false,
        run: create,
    },
    ToolDef {
        name: "undo",
        description: "Undoes the last change to the edit, whoever made it.",
        schema: schema::<NoArgs>,
        read_only: false,
        run: undo,
    },
    ToolDef {
        name: "redo",
        description: "Redoes the last undone change.",
        schema: schema::<NoArgs>,
        read_only: false,
        run: redo,
    },
    ToolDef {
        name: "transcribe",
        description: "Transcribes a sound with `provider` (parakeet: local and free; elevenlabs: \
            Scribe, cloud, the user's key), which becomes the saved choice; default the one in \
            the transcription settings. Default sound: the speech on the timeline. Skips a sound \
            already transcribed unless `force`. Takes a while; reports progress.",
        schema: schema::<TranscribeArgs>,
        read_only: false,
        run: transcribe,
    },
    ToolDef {
        name: "mouth_noises",
        description: "Finds clicks, pops, lip smacks and breaths in the speech and mutes those \
            clear of words (one undo step). Run it before silence: muted noises count as \
            silence. `apply: false` only reports what it found.",
        schema: schema::<NoiseArgs>,
        read_only: false,
        run: mouth_noises,
    },
    ToolDef {
        name: "silence",
        description: "Jump cuts: finds pauses and cuts them out of every track (one undo step). \
            `mode: silence` listens to the audio; `mode: gaps` uses the transcript to cut gaps \
            without words, breaths and noises included. Presets gentle, balanced (default), \
            tight, aggressive; any setting can be given on its own. `stream: speech` listens to \
            every microphone and cuts only where all are quiet. `apply: false` only reports.",
        schema: schema::<SilenceArgs>,
        read_only: false,
        run: silence,
    },
    ToolDef {
        name: "fillers",
        description: "Cuts filler words (um, uh, ...) and retakes (a sentence restarted) in the \
            transcript, one undo step: the app's rules plus its OpenAI/OpenRouter review (the \
            user's key, a small model), which catches far more and is on by default whenever a \
            key is set; `ai: false` uses the rules only. `apply: false` only lists.",
        schema: schema::<FillerArgs>,
        read_only: false,
        run: fillers,
    },
    ToolDef {
        name: "webcam_focus",
        description: "Finds where the speaker talks while the mouse rests and makes the webcam \
            fill the frame there (manual stretches stay). Settings as in `settings \
            webcam_focus`; `enabled: false` turns it off; `apply: false` only reports. To hide \
            the camera altogether: `settings` layout {\"webcamEnabled\": false}.",
        schema: schema::<WebcamArgs>,
        read_only: false,
        run: webcam_focus,
    },
    ToolDef {
        name: "zooms",
        description: "Automatic zooms from the recording's clicks. `op`: `add` (default) adds the \
            suggested zooms, `reload` finds them again with the zoom settings (dismissed ones \
            too; manual ones stay), `dismiss` drops the suggestions. `settings` changes the zoom \
            settings first. Reports how many, how long and how far in; `apply: false` with `add` \
            only reports.",
        schema: schema::<ZoomArgs>,
        read_only: false,
        run: zooms,
    },
    ToolDef {
        name: "settings",
        description: "Reads or changes settings. `area`: captions, layout, audio, zoom, \
            webcam_focus, transcription, ai, export, presets. Without `set` it shows the current \
            values; `set` is a JSON object of just the fields to change (nested objects merge). \
            API keys are not reachable here.",
        schema: schema::<SettingsArgs>,
        read_only: false,
        run: settings,
    },
    ToolDef {
        name: "chapters",
        description: "Makes YouTube chapters from the transcript with the app's AI pass (the \
            user's OpenAI/OpenRouter key) and saves them in the project. Run it last, after \
            every cut and right before export: it reads the final edit's words, and cuts made \
            after it can leave chapters titled for talk that is gone. `status` says when they \
            are out of date.",
        schema: schema::<StreamArgs>,
        read_only: false,
        run: chapters,
    },
    ToolDef {
        name: "shorts",
        description: "Finds moments for vertical shorts with the app's AI pass and saves them \
            for the Shorts Studio.",
        schema: schema::<StreamArgs>,
        read_only: false,
        run: shorts,
    },
    ToolDef {
        name: "export",
        description: "Exports the video (the canvas size and frame rate unless given; next to the \
            project unless `destination`). Waits until done and reports progress unless `wait: \
            false`, which returns a job id for export_status.",
        schema: schema::<ExportArgs>,
        read_only: true,
        run: export,
    },
    ToolDef {
        name: "export_status",
        description: "How the export started with `export` is going.",
        schema: schema::<NoArgs>,
        read_only: true,
        run: export_status,
    },
];

// ---------------------------------------------------------------------------------------
// Shared pieces.

#[derive(Deserialize, JsonSchema)]
pub struct NoArgs {}

/// The open project as an action starts.
struct Open {
    handle: String,
    revision: u64,
    duration: u64,
}

const PROJECT_SWITCHED: &str =
    "Another project was opened in AeroEdits while this ran, so it stopped there.";

/// The project open now, whichever it is.
fn current(ctx: &Ctx) -> Result<Open, String> {
    with_reader(ctx.state, |reader| {
        Ok(Open {
            handle: reader.summary.project_handle.clone(),
            revision: reader.document().revision,
            duration: reader.document().duration_us(),
        })
    })
}

/// The project open now.
fn opened(ctx: &Ctx) -> Result<Open, String> {
    current(ctx)
}

/// The answer for an action that may have changed the edit: `lines`, then the length and the
/// revision when it did, and the action listed in the Agent panel's history.
fn applied(
    ctx: &Ctx,
    action: &str,
    before: &Open,
    mut lines: Vec<String>,
) -> Result<String, String> {
    let after = current(ctx)?;
    if after.handle != before.handle {
        // The commands ran on `before`'s project; the one open now has nothing of them.
        lines.push(PROJECT_SWITCHED.into());
        ctx.changed_project();
        return Ok(lines.join("\n"));
    }
    if after.revision != before.revision {
        if after.duration != before.duration {
            lines.push(format!(
                "Length {} → {}.",
                clock(before.duration),
                clock(after.duration)
            ));
        }
        lines.push(format!(
            "Revision {}; `undo` takes it back.",
            after.revision
        ));
        let summary = lines.clone();
        let agent = ctx.agent.clone();
        // The edit is done; failing to list it must not hide that from the agent.
        let _ = with_proposals(ctx.state, Some(&before.handle), |_, proposals| {
            proposals.record_action(&agent, action, summary, before.revision, after.revision);
            Ok(())
        });
        ctx.changed_project();
        ctx.changed_proposals();
    }
    Ok(lines.join("\n"))
}

/// A sound of the project, for choosing what to scan.
struct Sound {
    key: String,
    speech: bool,
    placed: bool,
    transcribed: bool,
}

fn sounds(reader: &ProjectReader) -> Vec<Sound> {
    let document = reader.document();
    let mut out = Vec::new();
    for asset in &document.assets {
        for stream in asset.streams.iter().filter(|s| s.kind == StreamKind::Sound) {
            let key = format!("{}.{}", asset.id, stream.id);
            let placed = document
                .sequence
                .tracks
                .iter()
                .flat_map(|t| &t.clips)
                .any(|c| c.asset == asset.id && c.stream == stream.id);
            let transcribed = store::load_transcript(reader.root(), &key)
                .ok()
                .flatten()
                .is_some();
            out.push(Sound {
                key,
                speech: stream.role == Role::Mic,
                placed,
                transcribed,
            });
        }
    }
    out
}

enum Want {
    /// Speech on the timeline, else any sound there.
    Speech,
    /// A transcribed sound, speech on the timeline first.
    Transcribed,
}

fn pick_sound(ctx: &Ctx, stream: Option<String>, want: Want) -> Result<String, String> {
    with_reader(ctx.state, |reader| {
        let all = sounds(reader);
        if let Some(key) = stream {
            return if all.iter().any(|s| s.key == key) {
                Ok(key)
            } else {
                Err(format!(
                    "No sound {key}. Sounds: {}",
                    all.iter()
                        .map(|s| s.key.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            };
        }
        let rank = |s: &Sound| match want {
            Want::Speech => (!s.placed, !s.speech),
            Want::Transcribed => (!s.speech, !s.placed),
        };
        let mut candidates: Vec<&Sound> = all
            .iter()
            .filter(|s| !matches!(want, Want::Transcribed) || s.transcribed)
            .collect();
        candidates.sort_by_key(|s| rank(s));
        candidates
            .first()
            .map(|s| s.key.clone())
            .ok_or_else(|| match want {
                Want::Transcribed => "Nothing is transcribed yet: run `transcribe` first.".into(),
                Want::Speech => "The project has no sound.".into(),
            })
    })
}

/// Merges JSON `patch` into `base`: objects field by field, anything else replaced.
fn merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(base), Value::Object(patch)) => {
            for (key, value) in patch {
                merge(base.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (base, patch) => *base = patch.clone(),
    }
}

/// `current` with `patch` merged in, checked by the settings' own types. A field the types do
/// not have is refused, since serde would otherwise ignore it (a typo would do nothing); map
/// keys, unset options and empty lists count as the fields they are.
fn patched<T: Serialize + DeserializeOwned>(
    what: &str,
    current: &T,
    patch: &Value,
) -> Result<T, String> {
    let known = serde_json::to_value(current).map_err(|e| e.to_string())?;
    let mut value = known.clone();
    merge(&mut value, patch);
    let mut unknown = Vec::new();
    let next: T = serde_ignored::deserialize(value, |path| unknown.push(path.to_string()))
        .map_err(|e| format!("Invalid {what} settings: {e}"))?;
    if !unknown.is_empty() {
        let fields = match &known {
            Value::Object(map) => map.keys().cloned().collect::<Vec<_>>().join(", "),
            _ => String::new(),
        };
        return Err(format!(
            "Unknown {what} setting {}. The {what} settings are: {fields}",
            unknown.join(", ")
        ));
    }
    Ok(next)
}

fn one_line(value: &impl Serialize) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn plural(n: usize, one: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else if one.ends_with("ch") || one.ends_with('s') {
        format!("{n} {one}es")
    } else {
        format!("{n} {one}s")
    }
}

// ---------------------------------------------------------------------------------------
// Project.

fn status(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    params::<NoArgs>(args)?;
    text(status_text(ctx)?)
}

fn status_text(ctx: &Ctx) -> Result<String, String> {
    let open = opened(ctx)?;
    let mut lines =
        with_reader(ctx.state, |reader| {
            let document = reader.document();
            let summary = &reader.summary;
            let recorded: u64 = reader.recordings().map(|a| a.duration_us).sum();
            let (width, height) = document.layout.preview_dimensions()?;
            let mut lines = vec![format!(
                "Project \"{}\", revision {}, {} long{}, canvas {width}x{height}.",
                summary.name,
                document.revision,
                clock(document.duration_us()),
                if recorded > 0 {
                    format!(" ({} recorded)", clock(recorded))
                } else {
                    String::new()
                },
            )];
            let mut sound_lines = Vec::new();
            for sound in sounds(reader) {
                let mut line = format!(
                    "{} ({}{})",
                    sound.key,
                    if sound.speech { "speech" } else { "background" },
                    if sound.placed {
                        ""
                    } else {
                        ", not on the timeline"
                    }
                );
                if let Some(transcript) = store::load_transcript(reader.root(), &sound.key)? {
                    let clock_of = document.transcript_clock(&sound.key)?;
                    let kept = transcript
                        .words
                        .iter()
                        .filter(|w| crate::transcript::edit::kept(w, &clock_of))
                        .count();
                    line.push_str(&format!(
                        ": transcribed, {kept} of {} words kept",
                        transcript.words.len()
                    ));
                }
                let muted = document.noise_mutes.get(&sound.key).map_or(0, |r| r.len());
                if muted > 0 {
                    line.push_str(&format!(", {} muted", plural(muted, "noise")));
                }
                sound_lines.push(line);
            }
            lines.push(format!("Sounds: {}.", sound_lines.join("; ")));
            let focus = &document.webcam_focus;
            lines.push(format!(
            "Zooms: {} in the edit. Webcam focus: {}, {}. Captions: {}. Chapters: {}. Shorts: {}.",
            document.zooms.len(),
            if focus.enabled { "on" } else { "off" },
            plural(focus.segments.len(), "stretch"),
            if document.captions.enabled { "on" } else { "off" },
            document.chapters.len(),
            document.shorts.len(),
        ));
            Ok(lines)
        })?;
    if let Ok(generation) = commands::project_zoom_suggestions_impl(ctx.state, open.handle, None) {
        if !generation.suggestions.is_empty() {
            lines.push(format!(
                "{} suggested and not added (`zooms`).",
                plural(generation.suggestions.len(), "zoom")
            ));
        }
    }
    if let Some(running) = ctx.host.transcripts().running.lock().clone() {
        lines.push(format!("Busy: {running}."));
    }
    lines.extend(chapters_stale(ctx));
    Ok(lines.join("\n"))
}

/// A warning when an agent made chapters and the edit changed after: chapters read the final
/// edit's words, so they come last, right before export.
fn chapters_stale(ctx: &Ctx) -> Option<String> {
    with_reader(ctx.state, |reader| {
        let document = reader.document();
        if document.chapters.is_empty() {
            return Ok(None);
        }
        let proposals = crate::proposals::load(reader.root())?;
        let made = proposals
            .batches
            .iter()
            .rev()
            .find(|b| {
                b.status == crate::proposals::Status::Accepted
                    && matches!(&b.change, crate::proposals::Change::Action { action } if action == "chapters")
            })
            .and_then(|b| b.applied_revision);
        Ok(made.filter(|&r| r != document.revision).map(|r| {
            format!(
                "Chapters were made at revision {r} and the edit changed since (now {}): run \
                 `chapters` again, as the last step before export.",
                document.revision
            )
        }))
    })
    .ok()
    .flatten()
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanArgs {
    /// The preset to measure the passes with: gentle, balanced (default), tight or aggressive.
    #[serde(default)]
    pub preset: Option<Preset>,
}

/// What the recording has and what each pass would do, for the questions a template asks.
fn scan(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ScanArgs = params(args)?;
    let preset = args.preset.unwrap_or_default();
    let (mut lines, transcribed, cameras, mouse) =
        with_reader(ctx.state, |reader| {
            let document = reader.document();
            let all = sounds(reader);
            let mut lines = vec![format!(
                "Scan of \"{}\", {} long, measured with the {} preset; nothing was changed.",
                reader.summary.name,
                clock(document.duration_us()),
                one_line(&preset).trim_matches('"'),
            )];
            let describe = |s: &Sound| {
                let mut notes = vec![if s.transcribed {
                    "transcribed"
                } else {
                    "not transcribed"
                }];
                if !s.placed {
                    notes.push("not on the timeline");
                }
                format!("{} ({})", s.key, notes.join(", "))
            };
            let mics: Vec<&Sound> = all.iter().filter(|s| s.speech).collect();
            lines.push(match mics.len() {
                0 => "Microphones: none, so nothing to transcribe or cut by speech.".to_string(),
                1 => format!("Microphones: 1, {}.", describe(mics[0])),
                n => format!(
                "Microphones: {n}, {}. Silence can listen to all of them (`stream: speech`, cuts \
                 only where every one is quiet) or one.",
                mics.iter().map(|s| describe(s)).collect::<Vec<_>>().join(", ")
            ),
            });
            let others: Vec<&str> = all
                .iter()
                .filter(|s| !s.speech)
                .map(|s| s.key.as_str())
                .collect();
            if !others.is_empty() {
                lines.push(format!(
                    "Other sound: {} (music, game or desktop; `protect` keeps it from being cut).",
                    others.join(", ")
                ));
            }
            let cameras: Vec<String> = document
                .assets
                .iter()
                .flat_map(|a| {
                    a.streams
                        .iter()
                        .filter(|s| s.role == Role::Webcam)
                        .map(move |s| format!("{}.{}", a.id, s.id))
                })
                .collect();
            lines.push(if cameras.is_empty() {
                "Cameras: none, so no webcam questions.".to_string()
            } else {
                format!(
                    "Cameras: {} ({}), {}.",
                    cameras.len(),
                    cameras.join(", "),
                    if document.layout.webcam_enabled {
                        "shown as a bubble"
                    } else {
                        "hidden (layout webcamEnabled is false)"
                    }
                )
            });
            let mouse: usize = reader
                .recordings()
                .filter_map(|a| {
                    crate::telemetry::reader::read_telemetry(std::path::Path::new(&a.path)).ok()
                })
                .map(|t| t.events.len())
                .sum();
            lines.push(if mouse == 0 {
                "Mouse data: none, so no zooms or webcam focus from it.".to_string()
            } else {
                format!("Mouse data: {mouse} events.")
            });
            Ok((
                lines,
                all.iter().any(|s| s.transcribed),
                !cameras.is_empty(),
                mouse > 0,
            ))
        })?;

    let view = tc::transcript_settings_get_impl();
    let ready = |kind, name: &str, about: &str| match not_ready(&view, kind) {
        None => format!("{name} ready ({about})"),
        Some(reason) => format!("{name} not ready: {reason}"),
    };
    lines.push(format!(
        "Transcribers: {}; {}. Saved choice: {}.",
        ready(
            crate::transcript::ProviderKind::Parakeet,
            "parakeet",
            "local, free"
        ),
        ready(
            crate::transcript::ProviderKind::ElevenLabs,
            "elevenlabs",
            "Scribe, cloud, the user's key"
        ),
        one_line(&view.settings.provider).trim_matches('"'),
    ));
    let ai = tc::ai_settings_get_impl();
    let (provider, key) = match ai.settings.provider {
        crate::ai::AiProvider::OpenAi => ("OpenAI", &ai.openai_key_source),
        crate::ai::AiProvider::OpenRouter => ("OpenRouter", &ai.openrouter_key_source),
    };
    lines.push(if key.is_some() {
        format!("AI key: {provider} key set (chapters, shorts and the AI filler review can run).")
    } else {
        format!("AI key: none for {provider}, so no chapters, shorts or AI filler review.")
    });

    // Each pass as a dry run; one that can't run says why and the rest go on.
    fn pass(lines: &mut Vec<String>, label: &str, outcome: Result<String, String>) {
        lines.push(match outcome {
            Ok(found) => format!("{label}: {}", found.replace('\n', " ")),
            Err(error) => format!("{label}: could not check: {error}"),
        })
    }
    let dry = Some(false);
    pass(
        &mut lines,
        "Mouth noises",
        run_noises(
            ctx,
            NoiseArgs {
                sensitivity: Some(preset.noise_sensitivity()),
                apply: dry,
                ..Default::default()
            },
        ),
    );
    pass(
        &mut lines,
        "Silence from the audio",
        run_silence(
            ctx,
            SilenceArgs {
                preset: Some(preset),
                apply: dry,
                ..Default::default()
            },
        ),
    );
    if transcribed {
        pass(
            &mut lines,
            "Gaps from the transcript",
            run_silence(
                ctx,
                SilenceArgs {
                    mode: SilenceMode::Gaps,
                    preset: Some(preset),
                    apply: dry,
                    ..Default::default()
                },
            ),
        );
        pass(
            &mut lines,
            "Fillers and retakes (the rules; the AI review usually finds more)",
            run_fillers(
                ctx,
                FillerArgs {
                    apply: dry,
                    // A dry run never spends the user's AI key.
                    ai: Some(false),
                    ..Default::default()
                },
            ),
        );
    } else {
        lines.push("Gaps, fillers and retakes: transcribe first.".into());
    }
    if cameras {
        pass(
            &mut lines,
            "Webcam focus",
            run_webcam(
                ctx,
                WebcamArgs {
                    apply: dry,
                    ..Default::default()
                },
            ),
        );
    }
    if mouse {
        let added = with_reader(ctx.state, |reader| Ok(reader.document().zooms.len()))?;
        pass(
            &mut lines,
            "Zooms",
            run_zooms(
                ctx,
                ZoomArgs {
                    apply: dry,
                    ..Default::default()
                },
            )
            .map(|found| format!("{found} {} already in the edit.", plural(added, "zoom"))),
        );
    }
    if let Some(running) = ctx.host.transcripts().running.lock().clone() {
        lines.push(format!("Busy: {running}."));
    }
    text(lines.join("\n"))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenArgs {
    /// A project folder or a .aero recording folder.
    pub path: String,
}

fn open(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: OpenArgs = params(args)?;
    commands::open_project_impl(ctx.state, args.path.clone())?;
    ctx.host.project_opened(&args.path);
    ctx.changed_proposals();
    text(format!("Opened.\n{}", status_text(ctx)?))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateArgs {
    pub name: String,
    /// A .aero recording folder to edit.
    #[serde(default)]
    pub recording: Option<String>,
    /// The folder to make the project in. Default: the app's projects folder.
    #[serde(default)]
    pub location: Option<String>,
}

fn create(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: CreateArgs = params(args)?;
    let parent = args
        .location
        .filter(|p| !p.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(commands::default_projects_dir);
    let folder = crate::project::folder::create_project_folder(
        &parent,
        &args.name,
        args.recording.as_deref().map(std::path::Path::new),
    )?;
    let path = folder.to_string_lossy().into_owned();
    commands::open_project_impl(ctx.state, path.clone())?;
    ctx.host.project_opened(&path);
    ctx.changed_proposals();
    text(format!(
        "Created {path} and opened it.\n{}",
        status_text(ctx)?
    ))
}

fn undo(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    params::<NoArgs>(args)?;
    let before = opened(ctx)?;
    commands::project_undo_impl(ctx.state, before.handle.clone(), before.revision, None)?;
    undone(ctx, &before, "Undid the last change.")
}

fn redo(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    params::<NoArgs>(args)?;
    let before = opened(ctx)?;
    commands::project_redo_impl(ctx.state, before.handle.clone(), before.revision, None)?;
    undone(ctx, &before, "Redid the last undone change.")
}

fn undone(ctx: &Ctx, before: &Open, what: &str) -> Result<Vec<Output>, String> {
    let after = opened(ctx)?;
    ctx.changed_project();
    ctx.changed_proposals();
    text(format!(
        "{what} Length {} → {}, revision {}.",
        clock(before.duration),
        clock(after.duration),
        after.revision
    ))
}

// ---------------------------------------------------------------------------------------
// Cleanup passes.

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeArgs {
    /// Sound key such as `rec-1.mic` (see status). Default: the speech on the timeline.
    #[serde(default)]
    pub stream: Option<String>,
    /// Transcribe again even if it already is.
    #[serde(default)]
    pub force: bool,
    /// parakeet (local, free) or elevenlabs (Scribe, cloud, the user's key). It becomes the
    /// saved choice. Default: the one in the transcription settings.
    #[serde(default)]
    pub provider: Option<Transcriber>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Transcriber {
    Parakeet,
    Elevenlabs,
}

impl Transcriber {
    fn kind(self) -> crate::transcript::ProviderKind {
        match self {
            Transcriber::Parakeet => crate::transcript::ProviderKind::Parakeet,
            Transcriber::Elevenlabs => crate::transcript::ProviderKind::ElevenLabs,
        }
    }
}

/// Why a transcriber can't run now, if it can't.
fn not_ready(
    view: &crate::transcript::settings::TranscriptSettingsView,
    kind: crate::transcript::ProviderKind,
) -> Option<String> {
    use crate::transcript::ProviderKind::*;
    match kind {
        Parakeet if !view.parakeet_available => Some("Parakeet is not part of this build".into()),
        Parakeet if !view.parakeet_model_present => Some(
            "the Parakeet model is not downloaded yet (Transcription settings in the app)".into(),
        ),
        ElevenLabs if view.elevenlabs_key_source.is_none() => {
            Some("no ElevenLabs key is set (Transcription settings in the app)".into())
        }
        _ => None,
    }
}

fn transcribe(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    text(run_transcribe(ctx, params(args)?)?)
}

fn run_transcribe(ctx: &Ctx, args: TranscribeArgs) -> Result<String, String> {
    let open = opened(ctx)?;
    let key = pick_sound(ctx, args.stream, Want::Speech)?;
    let existing = with_reader(ctx.state, |reader| {
        store::load_transcript(reader.root(), &key)
    })?;
    if let (Some(transcript), false) = (&existing, args.force) {
        return Ok(format!(
            "{key} is already transcribed ({}); `force: true` transcribes it again.",
            plural(transcript.words.len(), "word")
        ));
    }
    let view = tc::transcript_settings_get_impl();
    let provider = args
        .provider
        .map_or(view.settings.provider, Transcriber::kind);
    if let Some(reason) = not_ready(&view, provider) {
        return Err(format!("Can't transcribe with {provider:?}: {reason}."));
    }
    if provider != view.settings.provider {
        let mut settings = view.settings.clone();
        settings.provider = provider;
        tc::transcript_settings_set_impl(settings)?;
    }
    let result = tc::transcript_run_impl(
        ctx.state,
        ctx.host.transcripts(),
        open.handle,
        key.clone(),
        &mut |p| ctx.progress(p.fraction, &p.message),
    )?;
    ctx.host.transcript_changed();
    let kept = result
        .view
        .words
        .iter()
        .filter(|w| w.edited_start_us.is_some())
        .count();
    let mut line = format!(
        "Transcribed {key} with {provider:?}: {}, {kept} on the timeline.",
        plural(result.view.words.len(), "word")
    );
    for diagnostic in result.diagnostics {
        line.push_str(&format!(" {diagnostic}"));
    }
    Ok(line)
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NoiseArgs {
    /// Sound key. Default: the speech on the timeline.
    #[serde(default)]
    pub stream: Option<String>,
    /// 0 finds only obvious noises, 1 faint ones too. Default 0.5.
    #[serde(default)]
    pub sensitivity: Option<f32>,
    /// Room muted either side of a noise (never into speech), ms. Default 20.
    #[serde(default)]
    pub padding_ms: Option<u32>,
    /// Also mute noises right next to words (off by default: they can clip speech).
    #[serde(default)]
    pub near_speech: bool,
    /// false: only report what was found. Default true.
    #[serde(default)]
    pub apply: Option<bool>,
}

fn mouth_noises(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    text(run_noises(ctx, params(args)?)?)
}

fn run_noises(ctx: &Ctx, args: NoiseArgs) -> Result<String, String> {
    let before = opened(ctx)?;
    let key = pick_sound(ctx, args.stream, Want::Speech)?;
    ctx.progress(0.1, &format!("Listening for mouth noises in {key}"));
    let scan = commands::noises::detect_mouth_noises_impl(
        ctx.state,
        before.handle.clone(),
        key.clone(),
        crate::dsp::noises::NoiseConfig {
            sensitivity: args.sensitivity.unwrap_or(0.5),
            padding_ms: args.padding_ms.unwrap_or(20),
        },
    )?;
    let mut kinds = std::collections::BTreeMap::<String, usize>::new();
    for noise in &scan.noises {
        *kinds
            .entry(format!("{:?}", noise.kind).to_lowercase())
            .or_default() += 1;
    }
    let found: Vec<String> = kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
    let already = scan.noises.iter().filter(|n| n.muted).count();
    let chosen: Vec<_> = scan
        .noises
        .iter()
        .filter(|n| !n.muted && (args.near_speech || !n.near_speech))
        .collect();
    let near = scan
        .noises
        .iter()
        .filter(|n| !n.muted && n.near_speech)
        .count();
    let mut lines = vec![format!(
        "Found {} in {key}{}{}.",
        plural(scan.noises.len(), "mouth noise"),
        if found.is_empty() {
            String::new()
        } else {
            format!(" ({})", found.join(", "))
        },
        if already > 0 {
            format!(", {already} already muted")
        } else {
            String::new()
        },
    )];
    if args.apply.unwrap_or(true) && !chosen.is_empty() {
        let ranges = chosen
            .iter()
            .map(|n| crate::sequence::SourceRange {
                start_us: n.source_start_us,
                end_us: n.source_end_us,
            })
            .collect();
        commands::noises::project_noise_mutes_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            key,
            ranges,
            false,
        )?;
        lines.push(format!(
            "Muted {}{}.",
            chosen.len(),
            if near > 0 && !args.near_speech {
                format!("; left {near} next to words alone (`nearSpeech: true` mutes them too)")
            } else {
                String::new()
            }
        ));
    } else if !chosen.is_empty() {
        lines.push(format!("{} would be muted.", chosen.len()));
    }
    lines.extend(scan.diagnostics);
    applied(ctx, "mouth_noises", &before, lines)
}

/// How hard jump cuts cut. The same values as the Jump cuts dialog
/// (front-end/src/lib/jumpCutConfig.ts); keep them in step.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    Gentle,
    #[default]
    Balanced,
    Tight,
    Aggressive,
}

impl Preset {
    /// Silence: quieter than (dB), longer than (ms), room kept around words (ms).
    fn silence(self) -> (f32, u32, u32) {
        match self {
            Preset::Gentle => (-48.0, 600, 80),
            Preset::Balanced => (-42.0, 250, 40),
            Preset::Tight => (-36.0, 100, 10),
            Preset::Aggressive => (-30.0, 60, 0),
        }
    }

    /// Gaps without words: longer than (ms), room (ms), refine quiet word edges.
    fn gaps(self) -> (u32, u32, bool) {
        match self {
            Preset::Gentle => (300, 80, false),
            Preset::Balanced => (150, 40, true),
            Preset::Tight => (60, 10, true),
            Preset::Aggressive => (20, 0, true),
        }
    }

    /// Mouth-noise sensitivity.
    fn noise_sensitivity(self) -> f32 {
        match self {
            Preset::Gentle => 0.35,
            Preset::Balanced => 0.5,
            Preset::Tight => 0.6,
            Preset::Aggressive => 0.7,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SilenceMode {
    /// Quiet in the audio.
    #[default]
    Silence,
    /// Gaps between transcribed words (breaths and noises included).
    Gaps,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct SilenceArgs {
    /// `silence` (default, from the audio) or `gaps` (from the transcript).
    #[serde(default)]
    pub mode: SilenceMode,
    /// gentle, balanced (default), tight or aggressive.
    #[serde(default)]
    pub preset: Option<Preset>,
    /// Silence: quieter than this counts as a pause, dB (-60 to -20).
    #[serde(default)]
    pub threshold_db: Option<f32>,
    /// Silence: set the level from each sound instead of thresholdDb, 0 to 1 (higher cuts more).
    #[serde(default)]
    pub auto_level: Option<f32>,
    /// Pauses (or gaps) shorter than this stay, ms.
    #[serde(default)]
    pub min_duration_ms: Option<u32>,
    /// Room kept around words, ms (0 to 300).
    #[serde(default)]
    pub padding_ms: Option<u32>,
    /// Gaps: trim quiet word tails measured in the audio.
    #[serde(default)]
    pub refine_word_edges: Option<bool>,
    /// Gaps: the level of those tails, dB.
    #[serde(default)]
    pub edge_threshold_db: Option<f32>,
    /// Sound key to listen to, or `speech` for every microphone (cut only where all are quiet;
    /// gaps then need each transcribed). Default: all speech on the timeline (silence) or the
    /// transcribed speech (gaps).
    #[serde(default)]
    pub stream: Option<String>,
    /// A second sound to keep: cut only where it is quiet too (music, game or video sound).
    #[serde(default)]
    pub protect: Option<String>,
    /// false: only report what was found. Default true.
    #[serde(default)]
    pub apply: Option<bool>,
}

fn silence(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    text(run_silence(ctx, params(args)?)?)
}

/// Every sound the timeline plays as speech, scanned together (the backend's `ALL_SPEECH`).
const ALL_SPEECH: &str = "speech";

fn run_silence(ctx: &Ctx, args: SilenceArgs) -> Result<String, String> {
    let before = opened(ctx)?;
    let preset = args.preset.unwrap_or_default();
    let (result, described) = match args.mode {
        SilenceMode::Silence => {
            let (threshold, min, padding) = preset.silence();
            let config = crate::dsp::SilenceConfig {
                threshold_db: args.threshold_db.unwrap_or(threshold),
                min_duration_ms: args.min_duration_ms.unwrap_or(min),
                padding_ms: args.padding_ms.unwrap_or(padding),
                auto_level: args.auto_level,
                window_ms: None,
                step_ms: None,
                channel_policy: None,
            };
            let key = match args.stream {
                Some(key) if key == ALL_SPEECH => key,
                Some(key) => pick_sound(ctx, Some(key), Want::Speech)?,
                None => with_reader(ctx.state, |reader| {
                    let speech = sounds(reader)
                        .into_iter()
                        .filter(|s| s.placed && s.speech)
                        .count();
                    Ok(if speech >= 2 {
                        Some(ALL_SPEECH.to_string())
                    } else {
                        None
                    })
                })?
                .map_or_else(|| pick_sound(ctx, None, Want::Speech), Ok)?,
            };
            let described = format!(
                "{} below {} dB, at least {:.2} s, {} ms room",
                key,
                config
                    .auto_level
                    .map_or(format!("{}", config.threshold_db), |l| format!(
                        "auto {l:.2}"
                    )),
                config.min_duration_ms as f64 / 1000.0,
                config.padding_ms
            );
            ctx.progress(0.1, &format!("Finding silence in {key}"));
            let result = match args.protect {
                Some(second) => commands::detect_silence_sources_impl(
                    ctx.state,
                    before.handle.clone(),
                    vec![key, second],
                    config,
                )?,
                None => {
                    commands::detect_silence_impl(ctx.state, before.handle.clone(), key, config)?
                }
            };
            (result, described)
        }
        SilenceMode::Gaps => {
            let (min, padding, refine) = preset.gaps();
            let config = crate::transcript::pauses::TranscriptGapConfig {
                min_duration_ms: args.min_duration_ms.unwrap_or(min),
                padding_ms: args.padding_ms.unwrap_or(padding),
                refine_word_edges: args.refine_word_edges.unwrap_or(refine),
                edge_threshold_db: args.edge_threshold_db.unwrap_or(-42.0),
            };
            // Every microphone: each needs its transcript, which the pass checks.
            let key = match args.stream {
                Some(key) if key == ALL_SPEECH => key,
                stream => pick_sound(ctx, stream, Want::Transcribed)?,
            };
            let described = format!(
                "{key} gaps without words of at least {:.2} s, {} ms room",
                config.min_duration_ms as f64 / 1000.0,
                config.padding_ms
            );
            ctx.progress(0.1, &format!("Finding gaps without words in {key}"));
            let ids = args.protect.map(|second| vec![key.clone(), second]);
            let result = commands::transcript_pauses::detect_non_speech_gaps_impl(
                ctx.state,
                before.handle.clone(),
                key,
                ids,
                config,
            )?;
            (result, described)
        }
    };
    let chosen: Vec<_> = result.suggestions.iter().filter(|s| s.selected).collect();
    let total: u64 = chosen
        .iter()
        .map(|s| s.end_us.saturating_sub(s.start_us))
        .sum();
    let noun = match args.mode {
        SilenceMode::Silence => "pause",
        SilenceMode::Gaps => "gap",
    };
    let mut lines = vec![format!(
        "Found {} ({described}), {:.1} s in all.",
        plural(chosen.len(), noun),
        total as f64 / 1e6
    )];
    if args.apply.unwrap_or(true) && !chosen.is_empty() {
        let ranges = chosen
            .iter()
            .map(|s| crate::zoom::EditedRange {
                start_us: s.start_us,
                end_us: s.end_us,
            })
            .collect();
        commands::transcript_pauses::apply_jump_cuts_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            ranges,
            result.transcript_dependencies.clone(),
        )?;
        lines.push("Cut them from every track.".into());
    }
    lines.extend(result.diagnostics);
    applied(
        ctx,
        match args.mode {
            SilenceMode::Silence => "silence",
            SilenceMode::Gaps => "gaps",
        },
        &before,
        lines,
    )
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct FillerArgs {
    /// Sound key of a transcript. Default: the transcribed speech.
    #[serde(default)]
    pub stream: Option<String>,
    /// Which to cut: `filler`, `retake`. Default both.
    #[serde(default)]
    pub kinds: Option<Vec<TranscriptSuggestionKind>>,
    /// Also run the app's AI review (OpenAI or OpenRouter, the user's key), which catches far
    /// more than the rules alone. Default: on whenever an AI key is set.
    #[serde(default)]
    pub ai: Option<bool>,
    /// false: only list them. Default true.
    #[serde(default)]
    pub apply: Option<bool>,
}

/// An AI key is set for the AI provider chosen in the settings.
pub(super) fn ai_key_ready() -> bool {
    let ai = tc::ai_settings_get_impl();
    match ai.settings.provider {
        crate::ai::AiProvider::OpenAi => ai.openai_key_source.is_some(),
        crate::ai::AiProvider::OpenRouter => ai.openrouter_key_source.is_some(),
    }
}

fn fillers(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    text(run_fillers(ctx, params(args)?)?)
}

fn run_fillers(ctx: &Ctx, args: FillerArgs) -> Result<String, String> {
    let before = opened(ctx)?;
    let key = pick_sound(ctx, args.stream, Want::Transcribed)?;
    let kinds = args.kinds.unwrap_or_else(|| {
        vec![
            TranscriptSuggestionKind::Filler,
            TranscriptSuggestionKind::Retake,
        ]
    });
    let use_ai = args.ai.unwrap_or_else(|| ctx.host.ai_review_ready());
    let found = if use_ai {
        ctx.progress(0.1, "Asking the AI review for fillers and retakes");
        tc::transcript_ai_suggest_impl(
            ctx.state,
            ctx.host.transcripts(),
            before.handle.clone(),
            key.clone(),
            &mut |p| ctx.progress(p.fraction, &p.message),
        )?
    } else {
        tc::transcript_suggestions_impl(ctx.state, before.handle.clone(), key.clone())?
    };
    // Rejected ones stay rejected, and agents' own proposals are decided in the Agent panel.
    let chosen: Vec<_> = found
        .iter()
        .filter(|s| !s.dismissed && s.source != SuggestionSource::Agent && kinds.contains(&s.kind))
        .collect();
    let count = |kind| chosen.iter().filter(|s| s.kind == kind).count();
    let (filler_count, retake_count) = (
        count(TranscriptSuggestionKind::Filler),
        count(TranscriptSuggestionKind::Retake),
    );
    let from_ai = chosen
        .iter()
        .filter(|s| s.source == SuggestionSource::Ai)
        .count();
    let mut lines = vec![format!(
        "Found {} and {} in {key}{}.",
        plural(filler_count, "filler"),
        plural(retake_count, "retake"),
        if use_ai {
            format!(" (rules and the AI review; {from_ai} from the AI review)")
        } else if from_ai > 0 {
            // The review's earlier finds are kept with the transcript.
            format!(" (rules, and {from_ai} the AI review found before)")
        } else {
            " (the rules only)".to_string()
        },
    )];
    if args.ai.is_none() && !use_ai && from_ai == 0 {
        lines.push(
            "No AI key is set, so the AI review (which finds far more) did not run; the user \
             can add one under Transcription and AI settings."
                .into(),
        );
    }
    let examples: Vec<String> = chosen
        .iter()
        .take(6)
        .map(|s| format!("{} \"{}\"", clock(s.edited_start_us), s.text))
        .collect();
    if !examples.is_empty() {
        lines.push(format!("For example: {}.", examples.join("; ")));
    }
    if args.apply.unwrap_or(true) && !chosen.is_empty() {
        let mut ids: Vec<String> = chosen.iter().flat_map(|s| s.word_ids.clone()).collect();
        ids.sort();
        ids.dedup();
        tc::transcript_cut_words_checked_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            key,
            ids,
            None,
        )?;
        lines.push("Cut them all.".into());
    }
    applied(ctx, "fillers", &before, lines)
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct WebcamArgs {
    /// false turns webcam focus off (its stretches are kept for turning it back on).
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Detection settings to change first, as in `settings webcam_focus` (`settings` part),
    /// such as {"idleMs": 2500, "requireSpeech": true}.
    #[serde(default)]
    pub settings: Option<Value>,
    /// false: only report what it would do. Default true.
    #[serde(default)]
    pub apply: Option<bool>,
}

fn webcam_focus(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    text(run_webcam(ctx, params(args)?)?)
}

/// "9 stretches, 1:10 in all" for source ranges.
fn stretches(ranges: &[(u64, u64)]) -> String {
    let total: u64 = ranges.iter().map(|(a, b)| b.saturating_sub(*a)).sum();
    format!(
        "{}, {} in all",
        plural(ranges.len(), "stretch"),
        clock(total)
    )
}

fn run_webcam(ctx: &Ctx, args: WebcamArgs) -> Result<String, String> {
    let before = opened(ctx)?;
    let focus = with_reader(ctx.state, |reader| {
        Ok(reader.document().webcam_focus.clone())
    })?;
    if args.enabled == Some(false) {
        let mut off = focus;
        off.enabled = false;
        commands::project_webcam_focus_update_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            off,
        )?;
        return applied(
            ctx,
            "webcam_focus",
            &before,
            vec!["Webcam focus is off.".into()],
        );
    }
    let settings = match &args.settings {
        Some(patch) => patched("webcam focus", &focus.settings, patch)?,
        None => focus.settings.clone(),
    };
    ctx.progress(0.1, "Finding where the speaker talks while the mouse rests");
    let found = commands::webcam_focus_find_impl(ctx.state, &before.handle, &settings)?;
    let mut lines = vec![if found.ranges.is_empty() {
        "No long enough idle-mouse stretches while talking were found.".to_string()
    } else if args.apply == Some(false) {
        format!(
            "The webcam would fill the frame in {} (talking while the mouse rests).",
            stretches(&found.ranges)
        )
    } else {
        format!(
            "The webcam fills the frame in {} (talking while the mouse rests).",
            stretches(&found.ranges)
        )
    }];
    lines.extend(found.diagnostics.iter().cloned());
    if args.apply != Some(false) {
        commands::webcam_focus_store_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            settings,
            found,
        )?;
    }
    applied(ctx, "webcam_focus", &before, lines)
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ZoomOp {
    /// Add every suggested zoom.
    #[default]
    Add,
    /// Find the recording's zooms again with the zoom settings.
    Reload,
    /// Drop the suggestions.
    Dismiss,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomArgs {
    /// add (default), reload or dismiss.
    #[serde(default)]
    pub op: ZoomOp,
    /// Zoom settings to change first, as in `settings zoom`, such as {"clickScale": 1.8}.
    #[serde(default)]
    pub settings: Option<Value>,
    /// false with `add`: only report the suggestions. Default true.
    #[serde(default)]
    pub apply: Option<bool>,
}

fn zooms(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    text(run_zooms(ctx, params(args)?)?)
}

/// "18 zooms, 3.2 s on average (1.5 to 8.0 s), 1.6x in" for suggestions.
fn describe_zooms(suggestions: &[crate::zoom::ZoomSuggestion]) -> String {
    let seconds: Vec<f64> = suggestions
        .iter()
        .map(|s| s.source_end_us.saturating_sub(s.source_start_us) as f64 / 1e6)
        .collect();
    if seconds.is_empty() {
        return "no zooms".into();
    }
    let (shortest, longest) = seconds
        .iter()
        .fold((f64::MAX, 0f64), |(lo, hi), s| (lo.min(*s), hi.max(*s)));
    let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len() as f64;
    let scales: Vec<f64> = suggestions.iter().map(|s| s.scale).collect();
    format!(
        "{}, {:.1} s on average ({:.1} to {:.1} s), {:.1}x in",
        plural(suggestions.len(), "zoom"),
        mean(&seconds),
        shortest,
        longest,
        mean(&scales)
    )
}

fn run_zooms(ctx: &Ctx, args: ZoomArgs) -> Result<String, String> {
    let before = opened(ctx)?;
    let mut lines = Vec::new();
    if args.apply == Some(false) {
        if args.op != ZoomOp::Add || args.settings.is_some() {
            return Err("`apply: false` only reports the current suggestions: leave out `op` and `settings`".into());
        }
        let generation =
            commands::project_zoom_suggestions_impl(ctx.state, before.handle.clone(), None)?;
        lines.push(if generation.suggestions.is_empty() {
            "No zoom suggestions to add.".to_string()
        } else {
            format!("Would add {}.", describe_zooms(&generation.suggestions))
        });
        lines.extend(generation.diagnostics);
        return Ok(lines.join("\n"));
    }
    if let Some(patch) = &args.settings {
        let current = with_reader(ctx.state, |reader| {
            Ok(reader.document().zoom_settings.clone())
        })?;
        let next = patched("zoom", &current, patch)?;
        commands::project_zoom_settings_set_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            next,
        )?;
        lines.push("Changed the zoom settings.".into());
    }
    let revision = opened(ctx)?.revision;
    match args.op {
        ZoomOp::Reload => {
            commands::project_zoom_reload_impl(ctx.state, before.handle.clone(), revision, None)?;
            lines.push("Found the recording's zooms again.".into());
        }
        ZoomOp::Add | ZoomOp::Dismiss => {
            let generation =
                commands::project_zoom_suggestions_impl(ctx.state, before.handle.clone(), None)?;
            let ids: Vec<String> = generation
                .suggestions
                .iter()
                .map(|s| s.id.clone())
                .collect();
            if ids.is_empty() {
                lines.push("No zoom suggestions left to add.".into());
                lines.extend(generation.diagnostics);
            } else if args.op == ZoomOp::Add {
                commands::project_zoom_accept_impl(
                    ctx.state,
                    before.handle.clone(),
                    revision,
                    ids.clone(),
                    None,
                )?;
                lines.push(format!(
                    "Added {}.",
                    describe_zooms(&generation.suggestions)
                ));
            } else {
                commands::project_zoom_dismiss_impl(
                    ctx.state,
                    before.handle.clone(),
                    revision,
                    ids.clone(),
                )?;
                lines.push(format!(
                    "Dismissed {}.",
                    plural(ids.len(), "zoom suggestion")
                ));
            }
        }
    }
    let count = with_reader(ctx.state, |reader| Ok(reader.document().zooms.len()))?;
    lines.push(format!("{} in the edit.", plural(count, "zoom")));
    applied(ctx, "zooms", &before, lines)
}

// ---------------------------------------------------------------------------------------
// Settings.

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    Captions,
    Layout,
    Audio,
    Zoom,
    WebcamFocus,
    Transcription,
    Ai,
    Export,
    Presets,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SettingsArgs {
    /// captions, layout, audio, zoom, webcam_focus, transcription, ai, export or presets.
    pub area: Area,
    /// Only the fields to change, such as {"enabled": true, "fontSizePct": 6}.
    #[serde(default)]
    pub set: Option<Value>,
}

fn settings(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: SettingsArgs = params(args)?;
    let name = serde_json::to_value(args.area)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let Some(patch) = args.set else {
        return text(format!(
            "{name}: {}",
            one_line(&current_settings(ctx, args.area)?)
        ));
    };
    let before = opened(ctx);
    match args.area {
        Area::Presets | Area::Export => {
            return Err(format!(
                "{name} cannot be set here: pass these values to the actions instead"
            ))
        }
        Area::Transcription => {
            let next = patched(&name, &tc::transcript_settings_get_impl().settings, &patch)?;
            tc::transcript_settings_set_impl(next)?;
        }
        Area::Ai => {
            let next = patched(&name, &tc::ai_settings_get_impl().settings, &patch)?;
            tc::ai_settings_set_impl(next)?;
        }
        area => {
            let before = before.as_ref().map_err(Clone::clone)?;
            let (handle, revision) = (before.handle.clone(), before.revision);
            let document = with_reader(ctx.state, |reader| Ok(reader.document().clone()))?;
            match area {
                Area::Captions => {
                    let next = patched(&name, &document.captions, &patch)?;
                    commands::project_captions_update_impl(ctx.state, handle, revision, next)?;
                }
                Area::Layout => {
                    let next = patched(&name, &document.layout, &patch)?;
                    commands::project_layout_update_impl(ctx.state, handle, revision, next, None)?;
                }
                Area::Audio => {
                    let next = patched(&name, &document.audio, &patch)?;
                    commands::project_audio_update_impl(ctx.state, handle, revision, next)?;
                }
                Area::Zoom => {
                    let next = patched(&name, &document.zoom_settings, &patch)?;
                    commands::project_zoom_settings_set_impl(ctx.state, handle, revision, next)?;
                }
                Area::WebcamFocus => {
                    let next = patched(&name, &document.webcam_focus, &patch)?;
                    commands::project_webcam_focus_update_impl(ctx.state, handle, revision, next)?;
                }
                _ => unreachable!("handled above"),
            }
        }
    }
    let now = one_line(&current_settings(ctx, args.area)?);
    match before {
        Ok(before) => text(applied(
            ctx,
            "settings",
            &before,
            vec![format!("{name} now: {now}")],
        )?),
        Err(_) => text(format!("{name} now: {now}")),
    }
}

fn value<T: Serialize>(settings: &T) -> Value {
    serde_json::to_value(settings).unwrap_or_default()
}

fn current_settings(ctx: &Ctx, area: Area) -> Result<Value, String> {
    Ok(match area {
        Area::Transcription => value(&tc::transcript_settings_get_impl().settings),
        Area::Ai => value(&tc::ai_settings_get_impl().settings),
        Area::Presets => {
            let presets: Vec<Value> = [Preset::Gentle, Preset::Balanced, Preset::Tight, Preset::Aggressive]
                .iter()
                .map(|preset| {
                    let (db, min, room) = preset.silence();
                    let (gap, gap_room, refine) = preset.gaps();
                    json!({
                        "preset": preset,
                        "silence": { "thresholdDb": db, "minDurationMs": min, "paddingMs": room },
                        "gaps": { "minDurationMs": gap, "paddingMs": gap_room, "refineWordEdges": refine },
                        "mouthNoiseSensitivity": preset.noise_sensitivity(),
                    })
                })
                .collect();
            Value::Array(presets)
        }
        Area::Export => {
            let defaults = export_defaults(ctx)?;
            value(&defaults)
        }
        area => with_reader(ctx.state, |reader| {
            let document = reader.document();
            Ok(match area {
                Area::Captions => value(&document.captions),
                Area::Layout => value(&document.layout),
                Area::Audio => value(&document.audio),
                Area::Zoom => value(&document.zoom_settings),
                Area::WebcamFocus => {
                    // The stretches themselves are long and are the action's business.
                    let mut focus = value(&document.webcam_focus);
                    if let Value::Object(map) = &mut focus {
                        let count = document.webcam_focus.segments.len();
                        map.insert("segments".into(), json!(format!("{count} stretches")));
                        map.remove("normalView");
                    }
                    focus
                }
                _ => Value::Null,
            })
        })?,
    })
}

// ---------------------------------------------------------------------------------------
// Chapters, shorts, export.

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct StreamArgs {
    /// Sound key of a transcript. Default: the transcribed speech.
    #[serde(default)]
    pub stream: Option<String>,
}

fn chapters(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: StreamArgs = params(args)?;
    let before = opened(ctx)?;
    let key = pick_sound(ctx, args.stream, Want::Transcribed)?;
    ctx.progress(0.1, "Asking the AI pass for chapters");
    let project = tc::project_chapters_generate_impl(
        ctx.state,
        ctx.host.transcripts(),
        before.handle.clone(),
        key,
    )?;
    let mut lines = vec![format!(
        "Made {}:",
        plural(project.chapters.len(), "chapter")
    )];
    lines.extend(project.chapters.iter().map(|c| {
        format!(
            "{} {}",
            c.edited_us.map_or("(cut)".to_string(), clock),
            c.title
        )
    }));
    text(applied(ctx, "chapters", &before, lines)?)
}

fn shorts(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: StreamArgs = params(args)?;
    let before = opened(ctx)?;
    let key = pick_sound(ctx, args.stream, Want::Transcribed)?;
    ctx.progress(0.1, "Asking the AI pass for shorts");
    let project = tc::project_shorts_generate_impl(
        ctx.state,
        ctx.host.transcripts(),
        before.handle.clone(),
        key,
    )?;
    let mut lines = vec![format!(
        "Found {} for the Shorts Studio:",
        plural(project.shorts.len(), "short")
    )];
    lines.extend(project.shorts.iter().map(|s| {
        format!(
            "\"{}\" ({:.0} s): {}",
            s.title,
            s.source_end_us.saturating_sub(s.source_start_us) as f64 / 1e6,
            s.reason
        )
    }));
    text(applied(ctx, "shorts", &before, lines)?)
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExportArgs {
    /// The file to write. Default: next to the project, named after it.
    #[serde(default)]
    pub destination: Option<String>,
    /// Export settings to change, as in `settings export`, such as {"quality": "max"}.
    #[serde(default)]
    pub settings: Option<Value>,
    /// false: start it and return; check with export_status. Default true.
    #[serde(default)]
    pub wait: Option<bool>,
}

fn export_defaults(ctx: &Ctx) -> Result<crate::export::ExportSettings, String> {
    with_reader(ctx.state, |reader| {
        let (width, height) = reader.document().layout.preview_dimensions()?;
        Ok(crate::export::ExportSettings {
            width,
            height,
            fps: reader.summary.fps,
            ..Default::default()
        })
    })
}

fn export(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ExportArgs = params(args)?;
    let open = opened(ctx)?;
    let defaults = export_defaults(ctx)?;
    let mut settings = match &args.settings {
        Some(patch) => patched("export", &defaults, patch)?,
        None => defaults,
    };
    if args.destination.is_some() {
        settings.destination = args.destination;
    }
    let stale = chapters_stale(ctx)
        .map(|w| format!("\n{w}"))
        .unwrap_or_default();
    let started = commands::export_start_impl(ctx.state, open.handle, settings)?;
    if args.wait == Some(false) {
        return text(format!(
            "Export {} started at revision {}; check it with export_status.{stale}",
            started.job_id, started.captured_revision
        ));
    }
    let begun = std::time::Instant::now();
    loop {
        let status = commands::export_status_impl(ctx.state, Some(started.job_id.clone()))?;
        use crate::export::ExportState::*;
        match status.state {
            Completed => {
                return text(format!(
                    "Exported {}x{} {} to {} in {:.0} s.{stale}",
                    status.width,
                    status.height,
                    status.video_codec,
                    status.output_path.unwrap_or_default(),
                    begun.elapsed().as_secs_f64()
                ))
            }
            Failed | Cancelled => return Err(export_failure(&status)),
            _ => {
                let fraction = if status.progress_denominator > 0 {
                    status.progress_numerator as f64 / status.progress_denominator as f64
                } else {
                    0.0
                };
                ctx.progress(fraction, &format!("Exporting, {:.0}%", fraction * 100.0));
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }
}

fn export_failure(status: &crate::export::ExportStatus) -> String {
    let message = status
        .failure
        .as_ref()
        .and_then(|f| serde_json::to_value(f).ok())
        .and_then(|v| {
            v.get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| format!("{:?}", status.state));
    format!("The export stopped: {message}")
}

fn export_status(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    params::<NoArgs>(args)?;
    let status = commands::export_status_impl(ctx.state, None)?;
    use crate::export::ExportState::*;
    text(match status.state {
        Idle => "No export has run since the app started.".to_string(),
        Queued | Running => format!(
            "Export {} running: {}/{} frames.",
            status.job_id, status.progress_numerator, status.progress_denominator
        ),
        Completed => format!(
            "Export {} done: {}.",
            status.job_id,
            status.output_path.clone().unwrap_or_default()
        ),
        Failed | Cancelled => export_failure(&status),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tests_support::lesson;

    fn call(ctx: &Ctx, name: &str, args: Value) -> Result<String, String> {
        let tool = crate::mcp::tools::find(name).unwrap();
        (tool.run)(ctx, args).map(|outputs| {
            outputs
                .into_iter()
                .filter_map(|o| match o {
                    Output::Text(text) => Some(text),
                    Output::Png(_) => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    fn length(ctx: &Ctx) -> u64 {
        opened(ctx).unwrap().duration
    }

    fn history(ctx: &Ctx) -> Vec<crate::proposals::BatchView> {
        with_reader(ctx.state, |reader| {
            Ok(crate::proposals::view(reader, &crate::proposals::load(reader.root())?).batches)
        })
        .unwrap()
    }

    #[test]
    fn fillers_are_cut_listed_in_the_panel_and_undone() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, key) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);
        let status = call(&ctx, "status", json!({})).unwrap();
        assert!(
            status.contains(&format!("{key} (speech): transcribed, 7 of 7 words kept")),
            "{status}"
        );

        let dry = call(&ctx, "fillers", json!({ "apply": false })).unwrap();
        assert!(
            dry.starts_with(&format!("Found 1 filler and 0 retakes in {key}")),
            "{dry}"
        );
        assert!(dry.contains("\"um\""), "{dry}");
        // The AI review is the default, but there is no key here: the rules only, and why.
        assert!(
            dry.contains("(the rules only)") && dry.contains("No AI key is set"),
            "{dry}"
        );
        let rules = call(&ctx, "fillers", json!({ "apply": false, "ai": false })).unwrap();
        assert!(
            !rules.contains("No AI key"),
            "asked for the rules only: {rules}"
        );
        assert!(history(&ctx).is_empty(), "a dry run changes nothing");

        let before = length(&ctx);
        let done = call(&ctx, "fillers", json!({})).unwrap();
        assert!(
            done.contains("Cut them all.") && done.contains("Length 0:04.00 →"),
            "{done}"
        );
        assert!(length(&ctx) < before);
        assert!(ctx.project_was_changed() && ctx.proposals_were_changed());
        let listed = history(&ctx);
        assert_eq!(listed[0].kind, "action");
        assert_eq!(listed[0].action.as_deref(), Some("fillers"));
        assert_eq!(listed[0].agent, "claude-code");
        assert!(listed[0].can_undo);

        let undone = call(&ctx, "undo", json!({})).unwrap();
        assert!(undone.starts_with("Undid the last change."), "{undone}");
        assert_eq!(length(&ctx), before);
    }

    #[test]
    fn silence_and_gaps_cut_the_pauses_with_presets_and_overrides() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "codex", false);
        let before = length(&ctx);
        // The click at 1.2 s splits the first pause in two...
        let dry = call(&ctx, "silence", json!({ "apply": false })).unwrap();
        assert!(dry.starts_with("Found 3 pauses"), "{dry}");
        assert!(
            dry.contains("below -42 dB, at least 0.25 s, 40 ms room"),
            "{dry}"
        );
        assert_eq!(length(&ctx), before);
        // ...until it is muted, as the mouth-noise pass is meant to run first.
        let muted = call(&ctx, "mouth_noises", json!({})).unwrap();
        assert!(
            muted.starts_with("Found 1 mouth noise in") && muted.contains("(1 click)"),
            "{muted}"
        );
        assert!(muted.contains("Muted 1."), "{muted}");
        assert_eq!(length(&ctx), before, "muting keeps the length");
        let dry = call(&ctx, "silence", json!({ "apply": false })).unwrap();
        assert!(dry.starts_with("Found 2 pauses"), "{dry}");
        // Only pauses of a second or more: neither is.
        let none = call(&ctx, "silence", json!({ "minDurationMs": 1000 })).unwrap();
        assert!(none.starts_with("Found 0 pauses"), "{none}");
        assert_eq!(length(&ctx), before);

        let cut = call(&ctx, "silence", json!({ "preset": "tight" })).unwrap();
        assert!(cut.contains("Cut them from every track."), "{cut}");
        let after_silence = length(&ctx);
        assert!(after_silence + 1_000_000 < before, "{cut}");

        let gaps = call(&ctx, "silence", json!({ "mode": "gaps", "apply": false })).unwrap();
        assert!(
            gaps.contains("gaps without words of at least 0.15 s"),
            "{gaps}"
        );
        assert!(call(&ctx, "silence", json!({ "preset": "fierce" }))
            .unwrap_err()
            .contains("Invalid arguments"));
    }

    #[test]
    fn scan_reports_what_each_pass_would_do_and_changes_nothing() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, key) = lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        let start = opened(&ctx).unwrap();
        let report = call(&ctx, "scan", json!({})).unwrap();
        let has = |part: &str| assert!(report.contains(part), "{part} in:\n{report}");
        assert!(
            report
                .starts_with("Scan of \"Lesson\", 0:04.00 long, measured with the balanced preset"),
            "{report}"
        );
        has(&format!("Microphones: 1, {key} (transcribed)."));
        has("Cameras: none, so no webcam questions.");
        has("Mouse data: none");
        has("Transcribers: parakeet ");
        has("AI key: ");
        has("Mouth noises: Found 1 mouth noise");
        has("1 would be muted.");
        // The click is not muted yet, so it splits the first pause.
        has("Silence from the audio: Found 3 pauses");
        has("Gaps from the transcript: Found ");
        // The scan's dry run never spends the AI key, and says the review finds more.
        has("Fillers and retakes (the rules; the AI review usually finds more): Found 1 filler and 0 retakes");
        has("(the rules only)");
        assert!(
            !report.contains("Webcam focus:") && !report.contains("Zooms:"),
            "{report}"
        );
        let end = opened(&ctx).unwrap();
        assert_eq!(
            (end.revision, end.duration),
            (start.revision, start.duration)
        );
        assert!(
            history(&ctx).is_empty(),
            "a scan lists nothing in the panel"
        );

        // Every microphone at once, which with one is that one.
        let all = call(
            &ctx,
            "silence",
            json!({ "stream": "speech", "apply": false }),
        )
        .unwrap();
        assert!(
            all.starts_with("Found 3 pauses (speech below -42 dB"),
            "{all}"
        );
        let zooms = call(&ctx, "zooms", json!({ "apply": false })).unwrap();
        assert!(zooms.starts_with("No zoom suggestions to add."), "{zooms}");
        assert!(call(&ctx, "zooms", json!({ "apply": false, "op": "reload" })).is_err());
    }

    #[test]
    fn settings_read_and_patch_and_refuse_unknown_fields() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        let captions = call(&ctx, "settings", json!({ "area": "captions" })).unwrap();
        assert!(
            captions.starts_with("captions: {") && captions.contains("\"enabled\":false"),
            "{captions}"
        );
        let revision = opened(&ctx).unwrap().revision;
        let set = call(
            &ctx,
            "settings",
            json!({ "area": "captions", "set": { "enabled": true } }),
        )
        .unwrap();
        assert!(
            set.contains("\"enabled\":true") && set.contains(&format!("Revision {}", revision + 1)),
            "{set}"
        );
        let error = call(
            &ctx,
            "settings",
            json!({ "area": "captions", "set": { "enabeld": true } }),
        )
        .unwrap_err();
        assert!(
            error.contains("Unknown captions setting enabeld") && error.contains("enabled"),
            "{error}"
        );
        let presets = call(&ctx, "settings", json!({ "area": "presets" })).unwrap();
        assert!(
            presets.contains("\"preset\":\"tight\"") && presets.contains("\"thresholdDb\":-36.0"),
            "{presets}"
        );
        let export = call(&ctx, "settings", json!({ "area": "export" })).unwrap();
        assert!(export.contains("\"width\":1920"), "{export}");
        assert!(call(&ctx, "settings", json!({ "area": "presets", "set": {} })).is_err());
    }

    #[test]
    fn settings_take_fields_the_current_values_leave_out() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, key) = lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        let track = with_reader(ctx.state, |reader| {
            let sequence = &reader.document().sequence;
            Ok(sequence
                .tracks
                .iter()
                .find(|t| t.kind == crate::sequence::TrackKind::Audio)
                .unwrap()
                .id
                .clone())
        })
        .unwrap();
        let mix =
            |mix: Value| json!({ "area": "audio", "set": { "tracks": { track.as_str(): mix } } });
        // No track has a mix of its own yet, so `tracks` is left out of the current settings.
        let audio = call(&ctx, "settings", json!({ "area": "audio" })).unwrap();
        assert!(!audio.contains("\"tracks\""), "{audio}");
        let set = call(&ctx, "settings", mix(json!({ "volumeDb": -6.0 }))).unwrap();
        assert!(set.contains("\"volumeDb\":-6.0"), "{set}");
        // An option set, then cleared back to unset.
        call(
            &ctx,
            "settings",
            json!({ "area": "captions", "set": { "trackId": key } }),
        )
        .unwrap();
        let cleared = call(
            &ctx,
            "settings",
            json!({ "area": "captions", "set": { "trackId": null } }),
        )
        .unwrap();
        assert!(!cleared.contains("\"trackId\""), "{cleared}");
        // Typos are still caught, inside map entries too.
        let error = call(&ctx, "settings", mix(json!({ "volumDb": -3.0 }))).unwrap_err();
        assert!(error.contains("Unknown audio setting tracks."), "{error}");
        assert!(error.contains("volumDb"), "{error}");
    }

    #[test]
    fn chapters_made_before_a_later_cut_are_reported_out_of_date() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        // Chapters as the `chapters` action leaves them (its AI pass needs a key).
        let before = opened(&ctx).unwrap();
        commands::project_chapters_set_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            vec![crate::chapters::Chapter {
                id: "c1".into(),
                source_us: 0,
                title: "Intro".into(),
                media: None,
                edited_us: None,
            }],
        )
        .unwrap();
        applied(&ctx, "chapters", &before, vec!["Made 1 chapter.".into()]).unwrap();
        let status = call(&ctx, "status", json!({})).unwrap();
        assert!(!status.contains("Chapters were made"), "{status}");

        call(&ctx, "silence", json!({})).unwrap();
        let status = call(&ctx, "status", json!({})).unwrap();
        assert!(
            status.contains("Chapters were made at revision")
                && status.contains("run `chapters` again, as the last step before export"),
            "{status}"
        );
    }

    #[test]
    fn a_project_is_created_and_exported() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        let out = dir.path().join("lesson.mp4");
        let exported = call(
            &ctx,
            "export",
            json!({ "destination": out.to_string_lossy(), "settings": { "width": 640, "height": 360 } }),
        )
        .unwrap();
        assert!(
            exported.starts_with("Exported 640x360 h264 to"),
            "{exported}"
        );
        assert!(out.metadata().unwrap().len() > 0);
        assert!(call(&ctx, "export_status", json!({}))
            .unwrap()
            .contains("done"));
        assert!(
            call(&ctx, "export", json!({ "settings": { "widht": 640 } }))
                .unwrap_err()
                .contains("Unknown export setting widht")
        );

        let made = call(
            &ctx,
            "create",
            json!({ "name": "Second", "location": dir.path().to_string_lossy() }),
        )
        .unwrap();
        assert!(made.contains("Project \"Second\", revision"), "{made}");
        assert!(call(&ctx, "fillers", json!({}))
            .unwrap_err()
            .contains("Nothing is transcribed yet"));
    }
}
