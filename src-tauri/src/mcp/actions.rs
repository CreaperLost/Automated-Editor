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

pub static ACTIONS: [ToolDef; 36] = [
    ToolDef {
        name: "status",
        description: "Where the open project stands, in a few lines: length, sounds and whether \
            they are transcribed, mouth noises muted, zooms, webcam focus, captions, chapters, \
            shorts, whether each camera has a face track for framing shorts, and anything \
            running. Call it first and after a cleanup.",
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
        name: "align",
        description: "Aligns a transcript's word times to the audio (20 ms, local model), so \
            cuts at word edges stop clipping words or keeping pauses inside them. Transcribing \
            already does this when the model is downloaded; use it for transcripts made before. \
            Text and word ids stay; cuts already made stay; suggestions worked out from the old \
            times go stale. Skips a transcript already aligned unless `force`.",
        schema: schema::<AlignArgs>,
        read_only: false,
        run: align,
    },
    ToolDef {
        name: "mouth_noises",
        description: "Finds clicks, pops, lip smacks and breaths in the speech and mutes those \
            clear of words (one undo step). Transcribe first: the transcript's words are never \
            muted. Noises close together count as one place; the user sees each place and can \
            unmute it in the app's Mouth noises panel. Run it before silence: muted noises \
            count as silence. `apply: false` only reports what it found.",
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
            every microphone and cuts only where all are quiet. `short` cleans a take short's \
            own timeline instead of the video's. `apply: false` only reports.",
        schema: schema::<SilenceArgs>,
        read_only: false,
        run: silence,
    },
    ToolDef {
        name: "fillers",
        description: "Cuts filler words (um, uh, ...) and retakes (a sentence restarted) in the \
            transcript, one undo step: the app's rules plus the review saved with \
            `fillers_review` (in-house: you read the transcript, the default). `review: \
            openrouter` instead has the app send it to OpenRouter/OpenAI now (the user's key, \
            paid). `apply: false` only lists.",
        schema: schema::<FillerArgs>,
        read_only: false,
        run: fillers,
    },
    ToolDef {
        name: "fillers_review",
        description: "In-house filler and retake review: you read the transcript yourself, so \
            nothing goes to OpenRouter. Without `cuts`: page `page` (from 0) as `[index]word` \
            with pauses, and what counts as a filler or a retake. `review: candidates` \
            (default) shows only windows of about 15 words either side of what the app's local \
            pass flagged (\"like\", \"so\", a phrase said twice); `thorough` shows every kept \
            word. Read every page, then call once with all your `cuts` and the pages' `stamp`: \
            that replaces the last review. Then `fillers` cuts them with the rules' finds.",
        schema: schema::<ReviewArgs>,
        read_only: false,
        run: fillers_review,
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
            Telemetry polish: layout `webcamAutoHide` (the bubble fades while work is under it), \
            `windowCrop` with `windowCropPaddingPct` and `windowCropOff` (recording stretches \
            `{media, sourceStartUs, sourceEndUs}` shown uncropped), and audio `soundEffects` \
            (`clicks`, `whooshes`, `muted`, `volumeDb`). API keys are not reachable here.",
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
        name: "short_create",
        description: "Makes a vertical short from the words `fromWord` to `toWord` (ids from \
            `transcript_get`) with your `title`, for when you pick the moments yourself rather \
            than with `shorts`. It takes the look of the shorts already made and picks its \
            scenes (`short_scenes`) unless `scenes: false`. 3 s to 3 min; 20-60 s works best.",
        schema: schema::<ShortCreateArgs>,
        read_only: false,
        run: short_create,
    },
    ToolDef {
        name: "short_scenes",
        description: "Scenes of a vertical short: stretches showing only the camera or only the \
            screen instead of the split (camera above or below the screen), easing between \
            them. Without `short` it lists the shorts and their scenes. `op: pick` (default \
            with a short) picks them from the recording, no AI: the camera for the opening \
            line and while the mouse rests as the speaker talks, the screen while clicks come \
            thick and fast; scenes set by hand stay. `op: set` replaces them with `scenes` \
            (seconds on the short's own timeline), `op: clear` makes it split throughout.",
        schema: schema::<ShortScenesArgs>,
        read_only: false,
        run: short_scenes,
    },
    ToolDef {
        name: "media_import",
        description: "Imports files (videos, sounds, pictures, or AeroEdits recordings) into \
            the project's Media, as the Media panel's Import does. Use it for a take the user \
            recorded for a short: a camera video (often with its own microphone) and, if they \
            used one, a separate microphone file. Answers each new media id and its streams.",
        schema: schema::<MediaImportArgs>,
        read_only: false,
        run: media_import,
    },
    ToolDef {
        name: "short_from_take",
        description: "Makes a vertical short from a take: a new recording made for the short \
            (the user retelling one part of the video to camera). `media` lists the take's \
            files and the app tells them apart (the picture is the camera; a sound-only file \
            is the voice, else a recording's microphone, else the camera's own sound); or name \
            them with `camera` and `voice`. A separate voice is lined up with \
            the camera's sound automatically. The short plays the take's camera and voice \
            with captions from the take's words; footage from the video goes under it with \
            `short_footage`. Transcribe the voice next (`transcribe` with its stream).",
        schema: schema::<ShortFromTakeArgs>,
        read_only: false,
        run: short_from_take,
    },
    ToolDef {
        name: "short_footage",
        description: "Footage under a short made from a take: what plays in the screen part \
            while the take talks. `op: add` places `items`: each under the take's words \
            `fromWord`..`toWord` (ids from `transcript_get` with the take's voice as `stream`) \
            or seconds `atS`..`toS` on the short, showing the video as edited (only what it \
            keeps, its cuts included) from its word `sourceWord` (an id from the video's \
            transcript) or from `videoS` seconds on its timeline; the footage's game or \
            desktop sound plays ducked under the voice (`sound: false` mutes it). Where no \
            footage plays, the camera fills the frame. `op: remove` takes it off \
            `fromS`..`toS` (all without them); `op: list` (default) lists it.",
        schema: schema::<ShortFootageArgs>,
        read_only: false,
        run: short_footage,
    },
    ToolDef {
        name: "check",
        description: "The check before export, the one the export dialog shows: black frames, \
            blips and words cut partway, choppy stretches, zooms that flash or jump, captions \
            over the webcam, chapters YouTube would drop, loudness, missing files, proposals \
            waiting. Each finding has an id, a time and, when one is safe, a fix. Without `fix` \
            it changes nothing; `fix` applies the fixes of those ids (or [\"all\"]), each as its \
            own undo step. Run it after the last edit, fix, and only then ask about export.",
        schema: schema::<CheckArgs>,
        read_only: false,
        run: check,
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
    ToolDef {
        name: "graphics",
        description: "Motion graphics: whether the renderer is installed, the templates (with \
            their variables, types and defaults), the user's graphics library (templates saved \
            from earlier projects, each with when to use it: look there before writing a new \
            graphic) and the graphics in the project (id, where they play, their variables, \
            brand kit, how their render is going). Graphics play full screen over everything \
            (screen, camera, captions), and cuts never go through one, so add them as one of \
            the last passes, after the cuts are checked and before chapters.",
        schema: schema::<NoArgs>,
        read_only: true,
        run: graphics_list,
    },
    ToolDef {
        name: "graphic_add",
        description: "Adds a graphic from a template (or a copy of a library item, `library`) \
            at a time on the edited timeline (`atUs`) or at a transcript word (`atWord`, an id \
            from transcript_get), on a Graphics track above the video, one undo step. It plays \
            for `lengthUs` (the template's or item's own by default) and renders in the \
            background; `graphic_render` waits for it. `blank` starts an empty composition to \
            write with graphic_write_file.",
        schema: schema::<GraphicAddArgs>,
        read_only: false,
        run: graphic_add,
    },
    ToolDef {
        name: "graphic_set",
        description: "Changes a graphic: its variables (checked against what it declares; \
            null puts one back to its default), its name, its length on the timeline, its \
            brand kit (`default` for the project's, `none`, or a kit from brand_kits), and with \
            `short` the slot it plays in that short (`full` frame, the `camera` or `screen` \
            part of the split, or `overScreen`: over the screen part, under the captions, its \
            see-through parts showing the screen). It renders again by itself.",
        schema: schema::<GraphicSetArgs>,
        read_only: false,
        run: graphic_set,
    },
    ToolDef {
        name: "graphic_write_file",
        description: "Writes a file into a graphic's folder (index.html, or an image or font it \
            uses), for a composition of your own; answers with HyperFrames' lint of it. Its \
            folder is in `graphics`; with a shell you can write there directly. Rules: a \
            HyperFrames composition, full-screen over the video, laid out with vw/vh/vmin so it \
            fits 16:9 and 9:16; a transparent page with a background you fade in and out; no \
            network (no CDN scripts or web fonts); Web Animations or CSS, finite and paused; \
            editable text as data-composition-variables.",
        schema: schema::<GraphicWriteArgs>,
        read_only: false,
        run: graphic_write_file,
    },
    ToolDef {
        name: "graphic_render",
        description: "Renders the graphics (or one) for the video's canvas now, waiting, and \
            reports each one's lint and render. Look at the result with frame_render at a time \
            inside the graphic.",
        schema: schema::<GraphicRenderArgs>,
        read_only: true,
        run: graphic_render,
    },
    ToolDef {
        name: "graphic_templatize",
        description: "Turns a graphic you wrote (one the user likes) toward a general template \
            for the user's library: returns its source, what the app already sees in it (network \
            references, missing brand colours, fixed text) and the rules to rewrite it by (text \
            as variables, brand kit colours, timing from data-duration, viewport layout with a \
            portrait block, a fading card, no network). Rewrite its files, then graphic_check.",
        schema: schema::<GraphicOnlyArgs>,
        read_only: true,
        run: graphic_templatize,
    },
    ToolDef {
        name: "graphic_check",
        description: "The library's gate for a graphic, run by the app (no model): HyperFrames' \
            check (lint, runtime, overflow, contrast) at 16:9 and 9:16 with its text and with \
            long text, stills at 3 s and 8 s (see-through at both ends, so its timing follows \
            the clip), each text variable changing the picture, no network, brand kit colours. \
            Answers pass or fail per check with why, and a contact sheet (both shapes, its text \
            and long text). Takes 30-90 s; an unchanged graphic answers at once.",
        schema: schema::<GraphicOnlyArgs>,
        read_only: true,
        run: graphic_check,
    },
    ToolDef {
        name: "library_save",
        description: "Saves a graphic to the user's graphics library as a template every \
            project can reuse with new text, once it passes the gate (graphic_check, run again \
            here). Give a `label`, a one-line `useWhen` (when an agent should pick it) and \
            optionally `lengthUs`. If a library item or bundled template is close to it (label \
            or variables), nothing is saved and the answer names it: ask the user, then \
            `update` that item (its next version) or `asNew`. Ask the user before saving; \
            `asIs` (saving a graphic that fails, marked not general) only at their word.",
        schema: schema::<LibrarySaveArgs>,
        read_only: false,
        run: library_save,
    },
    ToolDef {
        name: "graphics_install",
        description: "Installs the graphics renderer (HyperFrames with its own Node and Chrome: \
            a 200 MB download, about 500 MB on disk, in the app's data folder). Ask the user \
            before calling it.",
        schema: schema::<NoArgs>,
        read_only: false,
        run: graphics_install,
    },
    ToolDef {
        name: "brand_kits",
        description: "The user's brand kits (background, text and accent colours, a font, a \
            logo) and the project's default kit. A kit fills a graphic's `background`, `text` \
            and `accent` variables where the graphic has not set them, and sets the font; a \
            composition of your own can use var(--brand-accent), var(--brand-font), \
            var(--brand-logo) and the others. Kits are made by the user in the Graphics tab.",
        schema: schema::<NoArgs>,
        read_only: true,
        run: brand_kits,
    },
    ToolDef {
        name: "brand_pick",
        description: "Picks the brand kit the project's graphics use (`brand`: a kit's name, or \
            `none`), one undo step; with `graphic`, picks that graphic's own instead \
            (`default` follows the project's). Graphics render again.",
        schema: schema::<BrandPickArgs>,
        read_only: false,
        run: brand_pick,
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
            let faces = crate::face::status(reader.root(), document);
            if !faces.is_empty() {
                let each: Vec<String> = faces
                    .iter()
                    .map(|f| {
                        let key = format!("{}.{}", f.asset, f.stream);
                        match (f.state.as_str(), f.face_share) {
                            ("ready", Some(share)) => format!(
                                "{key} has a face track (a face in {:.0}% of it)",
                                share * 100.0
                            ),
                            ("detecting", _) => {
                                format!("{key} is being looked at ({:.0}%)", f.progress * 100.0)
                            }
                            ("failed", _) => format!(
                                "{key} has no face track: {}",
                                f.error.as_deref().unwrap_or("detection failed")
                            ),
                            ("unavailable", _) => {
                                format!("{key} has no face track (not in this build)")
                            }
                            _ => format!("{key} has no face track yet"),
                        }
                    })
                    .collect();
                lines.push(format!(
                    "Camera framing in shorts (each camera clip centred on the face unless the \
                     short's layout lists it in cameraClips): {}.",
                    each.join("; ")
                ));
            }
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
        format!("AI key: {provider} key set (chapters and shorts can run). Fillers and retakes are reviewed in-house by you (`fillers_review`) unless the user asks for the OpenRouter review.")
    } else {
        format!("AI key: none for {provider}, so no chapters or shorts. Fillers and retakes are reviewed in-house by you (`fillers_review`).")
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
            "Fillers and retakes (the rules, plus a saved in-house review)",
            run_fillers(
                ctx,
                FillerArgs {
                    apply: dry,
                    // A dry run never spends the user's AI key.
                    review: Some(FillerReview::InHouse),
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
pub struct AlignArgs {
    /// Sound key such as `rec-1.mic` (see status). Default: the speech on the timeline.
    #[serde(default)]
    pub stream: Option<String>,
    /// Align again even if it already is.
    #[serde(default)]
    pub force: bool,
}

fn align(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: AlignArgs = params(args)?;
    let open = opened(ctx)?;
    let key = pick_sound(ctx, args.stream, Want::Speech)?;
    let existing = with_reader(ctx.state, |reader| {
        store::load_transcript(reader.root(), &key)
    })?
    .ok_or_else(|| format!("{key} is not transcribed yet: `transcribe` it first."))?;
    if let (Some(info), false) = (&existing.alignment, args.force) {
        return text(format!(
            "{key} is already aligned ({} aligned, {} kept their times); `force: true` aligns \
             it again.",
            info.aligned_words, info.kept_words
        ));
    }
    let result = tc::transcript_align_impl(
        ctx.state,
        ctx.host.transcripts(),
        open.handle,
        key.clone(),
        &mut |p| ctx.progress(p.fraction, &p.message),
    )?;
    ctx.host.transcript_changed();
    text(format!("{key}: {}", result.diagnostics.join(" ")))
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
    // Places the scan found; muted moments it does not list (kind None) are not new finds.
    let places: Vec<_> = scan.noises.iter().filter(|n| n.kind.is_some()).collect();
    let mut kinds = std::collections::BTreeMap::<String, usize>::new();
    for noise in &places {
        if let Some(kind) = noise.kind {
            *kinds.entry(format!("{kind:?}").to_lowercase()).or_default() += 1;
        }
    }
    let found: Vec<String> = kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
    let already = places.iter().filter(|n| n.muted).count();
    let chosen: Vec<_> = places
        .iter()
        .filter(|n| !n.muted && (args.near_speech || !n.near_speech))
        .collect();
    let near = places.iter().filter(|n| !n.muted && n.near_speech).count();
    let noises: u32 = places.iter().map(|n| n.count).sum();
    let mut lines = vec![format!(
        "Found {} in {key}, in {}{}{}.",
        plural(noises as usize, "mouth noise"),
        plural(places.len(), "place"),
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
    if !scan.guarded {
        lines.push(
            "There is no transcript for this sound, so nothing guards the words: transcribe first."
                .into(),
        );
    }
    if args.apply.unwrap_or(true) && !chosen.is_empty() {
        let ranges = chosen.iter().flat_map(|n| n.parts.clone()).collect();
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
    /// Gaps: keep the sounds the transcriber tagged between words ("[keyboard clacking]",
    /// "[laughs]"). Default false: a gap without words is cut, typing and all.
    #[serde(default)]
    pub keep_sound_events: Option<bool>,
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
    /// A short made from a take (its id): clean its own timeline, the take, instead of the
    /// video's. Its speech is listened to unless `stream` says otherwise.
    #[serde(default)]
    pub short: Option<String>,
}

fn silence(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    text(run_silence(ctx, params(args)?)?)
}

/// Every sound the timeline plays as speech, scanned together (the backend's `ALL_SPEECH`).
const ALL_SPEECH: &str = "speech";

fn run_silence(ctx: &Ctx, args: SilenceArgs) -> Result<String, String> {
    let before = opened(ctx)?;
    let preset = args.preset.unwrap_or_default();
    let short = args
        .short
        .as_deref()
        .map(|s| take_short_id(ctx, s))
        .transpose()?;
    let short = short.as_deref();
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
                None if short.is_some() => ALL_SPEECH.to_string(),
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
                Some(second) => commands::detect_silence_sources_for(
                    ctx.state,
                    before.handle.clone(),
                    vec![key, second],
                    config,
                    short,
                )?,
                None => commands::detect_silence_for(
                    ctx.state,
                    before.handle.clone(),
                    key,
                    config,
                    short,
                )?,
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
                keep_sound_events: args.keep_sound_events.unwrap_or(false),
            };
            // Every microphone: each needs its transcript, which the pass checks.
            let key = match args.stream {
                Some(key) if key == ALL_SPEECH => key,
                None if short.is_some() => ALL_SPEECH.to_string(),
                stream => pick_sound(ctx, stream, Want::Transcribed)?,
            };
            let described = format!(
                "{key} gaps without words of at least {:.2} s, {} ms room",
                config.min_duration_ms as f64 / 1000.0,
                config.padding_ms
            );
            ctx.progress(0.1, &format!("Finding gaps without words in {key}"));
            let ids = args.protect.map(|second| vec![key.clone(), second]);
            let result = commands::transcript_pauses::detect_non_speech_gaps_for(
                ctx.state,
                before.handle.clone(),
                key,
                ids,
                config,
                short,
            )?;
            (result, described)
        }
    };
    // Transcript gaps come back unticked (the UI asks the user to listen first); asking for
    // the pass means all of them, so only the audio pass's own selection counts.
    let chosen: Vec<_> = result
        .suggestions
        .iter()
        .filter(|s| s.selected || matches!(args.mode, SilenceMode::Gaps))
        .collect();
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
        commands::transcript_pauses::apply_jump_cuts_for(
            ctx.state,
            before.handle.clone(),
            before.revision,
            ranges,
            result.transcript_dependencies.clone(),
            short,
        )?;
        lines.push(match short {
            Some(id) => format!("Cut them from every track of short {id}."),
            None => "Cut them from every track.".into(),
        });
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
    /// Whose review adds to the rules. `in-house` (default): the one you saved with
    /// `fillers_review` (nothing leaves the computer). `openrouter`: the app sends the
    /// transcript to OpenRouter or OpenAI now (the user's key, paid).
    #[serde(default)]
    pub review: Option<FillerReview>,
    /// false: only list them. Default true.
    #[serde(default)]
    pub apply: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
pub enum FillerReview {
    /// The connected agent's own review, saved with `fillers_review`.
    #[default]
    #[serde(rename = "in-house")]
    InHouse,
    /// The app's OpenRouter (or OpenAI) review, run now with the user's key.
    #[serde(rename = "openrouter")]
    OpenRouter,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReviewArgs {
    /// Sound key of a transcript. Default: the transcribed speech.
    #[serde(default)]
    pub stream: Option<String>,
    /// What the pages show. `candidates` (default): windows around what the app's local pass
    /// flagged as a possible filler or retake. `thorough`: every kept word.
    #[serde(default)]
    pub review: Option<ReviewRead>,
    /// The page to read, from 0. Default 0.
    #[serde(default)]
    pub page: Option<usize>,
    /// Your cuts over all the pages, to save as the review. Leave out to read a page.
    #[serde(default)]
    pub cuts: Option<Vec<ReviewCut>>,
    /// The `stamp` the pages gave; required with `cuts`.
    #[serde(default)]
    pub stamp: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
pub enum ReviewRead {
    /// Only the words around the local pass's flags.
    #[default]
    #[serde(rename = "candidates")]
    Candidates,
    /// Every kept word.
    #[serde(rename = "thorough")]
    Thorough,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ReviewCut {
    /// `filler` or `retake`.
    #[serde(rename = "type")]
    pub kind: String,
    /// The first word's index, as the pages number them.
    pub from: usize,
    /// The last word's index (inclusive).
    pub to: usize,
    /// A few words on why, shown to the user.
    #[serde(default)]
    pub why: String,
}

fn fillers_review(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ReviewArgs = params(args)?;
    let handle = opened(ctx)?.handle;
    let key = pick_sound(ctx, args.stream, Want::Transcribed)?;
    let Some(cuts) = args.cuts else {
        let read = args.review.unwrap_or_default();
        let page = args.page.unwrap_or(0);
        // What to call after page `at` of `pages`.
        let next = |at: usize, pages: usize, stamp: &str| {
            if at + 1 < pages {
                let review = match read {
                    ReviewRead::Candidates => "",
                    ReviewRead::Thorough => "`review: \"thorough\"`, ",
                };
                format!("Next: `fillers_review` with {review}`page: {}`.", at + 1)
            } else {
                format!(
                    "That was the last page. Now call `fillers_review` once with all your cuts \
                     (`[{{\"type\": \"filler\"|\"retake\", \"from\": i, \"to\": j, \"why\": \
                     \"...\"}}]`) and `stamp: \"{stamp}\"`; then `fillers` (`apply: false` \
                     first) lists and cuts them."
                )
            }
        };
        if read == ReviewRead::Thorough {
            let page = tc::transcript_review_page_impl(ctx.state, handle, key.clone(), page)?;
            return text(format!(
                "In-house review of {key}: page {} of {} ({} kept words, stamp {}). Pages \
                 overlap a little; indexes count over all of them.\n\n{}\n{}\n\n{}",
                page.page,
                page.pages,
                page.words,
                page.stamp,
                crate::ai::fillers::CRITERIA,
                page.text,
                next(page.page, page.pages, &page.stamp),
            ));
        }
        let page = tc::transcript_review_candidates_impl(ctx.state, handle, key.clone(), page)?;
        let settled = format!(
            "The rules already cut {} (um, uh, ...) and {}: leave those.",
            plural(page.rule_fillers, "filler sound"),
            plural(page.rule_retakes, "retake")
        );
        if page.pages == 0 {
            return text(format!(
                "In-house review of {key}, candidates only (stamp {}): the app's local pass \
                 flagged nothing in the {} kept words. {settled} Nothing to read: go on to \
                 `fillers` (`apply: false` first), or read every word with `review: \
                 \"thorough\"`.",
                page.stamp, page.words
            ));
        }
        let head = if page.page == 0 {
            format!(
                "The app's local pass flagged {} and {} in {}: {} words to read instead of {} \
                 for a full read ({} with `review: \"thorough\"`), {}% of it. {settled} \
                 Each window lists its flags: cut a flag only when the criteria say so (\"like\" \
                 or \"so\" that carry meaning stay, so does a phrase said twice for emphasis), \
                 and mark anything else in a window that should go.\n\n{}\n",
                plural(page.fillers, "possible filler"),
                plural(page.retakes, "possible retake"),
                plural(page.windows, "window"),
                page.shown_words,
                page.full_words,
                plural(page.full_pages, "page"),
                (page.shown_words * 100).div_ceil(page.full_words.max(1)),
                crate::ai::fillers::CRITERIA,
            )
        } else {
            String::new()
        };
        return text(format!(
            "In-house review of {key}, candidates only: page {} of {} (stamp {}). Indexes count \
             over the whole transcript.\n{head}\n{}\n\n{}",
            page.page,
            page.pages,
            page.stamp,
            page.text,
            next(page.page, page.pages, &page.stamp),
        ));
    };
    let stamp = args
        .stamp
        .ok_or("Give the `stamp` the pages showed with your cuts")?;
    let given = cuts.len();
    let cuts = serde_json::to_value(cuts).map_err(|e| e.to_string())?;
    let (kept, suggestions) =
        tc::transcript_review_save_impl(ctx.state, handle, key.clone(), &stamp, cuts, &ctx.agent)?;
    let listed = suggestions
        .iter()
        .filter(|s| s.source == SuggestionSource::Ai && !s.dismissed)
        .count();
    let mut lines = vec![format!(
        "Saved {kept} of {given} cuts as the in-house review of {key}; {listed} show in the \
         Transcript panel's review list (marked In-house) besides the rules' own."
    )];
    if kept < given {
        lines.push(
            "The rest were skipped: out of range, reversed, too long (a filler at most 4 words, \
             a retake at most 60) or of an unknown type."
                .into(),
        );
    }
    lines.push(
        "Next: `fillers` with `apply: false` lists everything to cut; then `fillers`.".into(),
    );
    text(lines.join("\n"))
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
    let use_ai = args.review == Some(FillerReview::OpenRouter);
    let found = if use_ai {
        ctx.progress(0.1, "Asking the OpenRouter review for fillers and retakes");
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
    // Who made the saved review: the connected agent (in-house) or the OpenRouter model.
    let reviewer = chosen
        .iter()
        .find(|s| s.source == SuggestionSource::Ai)
        .and_then(|s| s.reviewer.as_deref())
        .map(|r| {
            if r.starts_with(tc::IN_HOUSE) {
                "the in-house review".to_string()
            } else {
                format!("the OpenRouter review ({r})")
            }
        })
        .unwrap_or_else(|| "the review".into());
    let mut lines = vec![format!(
        "Found {} and {} in {key}{}.",
        plural(filler_count, "filler"),
        plural(retake_count, "retake"),
        if use_ai {
            format!(" (rules and the OpenRouter review run now; {from_ai} from it)")
        } else if from_ai > 0 {
            // The review's finds are kept with the transcript.
            format!(" (rules, and {from_ai} from {reviewer})")
        } else {
            " (the rules only)".to_string()
        },
    )];
    if !use_ai && from_ai == 0 {
        lines.push(
            "The rules alone miss most fillers and retakes. Review the transcript in-house \
             first, unless it flagged nothing: `fillers_review` pages you through the words \
             around what the app flagged (every word with `review: \"thorough\"`) and saves \
             your cuts (nothing is sent to OpenRouter)."
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

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShortCreateArgs {
    pub title: String,
    /// The first and last word, ids from `transcript_get`.
    pub from_word: String,
    pub to_word: String,
    /// The sound whose transcript the ids are from; default the speech that is transcribed.
    #[serde(default)]
    pub stream: Option<String>,
    /// Why it works on its own, in a sentence (shown in the Shorts Studio).
    #[serde(default)]
    pub reason: Option<String>,
    /// Pick its scenes right away. Default true.
    #[serde(default)]
    pub scenes: Option<bool>,
}

fn short_create(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ShortCreateArgs = params(args)?;
    let before = opened(ctx)?;
    let key = pick_sound(ctx, args.stream, Want::Transcribed)?;
    let (shorts, short) = with_reader(ctx.state, |reader| {
        let document = reader.document();
        let transcript =
            store::load_transcript(reader.root(), &key)?.ok_or("That sound is not transcribed")?;
        let word = |id: &str| {
            transcript
                .words
                .iter()
                .find(|w| w.id == id)
                .ok_or_else(|| format!("No word {id} in the transcript of {key}"))
        };
        let (first, last) = (word(&args.from_word)?, word(&args.to_word)?);
        if last.source_end_us <= first.source_start_us {
            return Err("`toWord` must come after `fromWord`".into());
        }
        let source = document.stream_ref(&key)?;
        // The look of the shorts made so far, so they match; each short has its own scenes.
        let mut layout = document
            .shorts
            .first()
            .map(|s| s.layout.clone())
            .unwrap_or_default();
        layout.scenes.clear();
        layout.graphic_slots.clear();
        let base = format!("short-{}", first.source_start_us / 1000);
        let id = (1..)
            .map(|n| {
                if n == 1 {
                    base.clone()
                } else {
                    format!("{base}-{n}")
                }
            })
            .find(|id| !document.shorts.iter().any(|s| &s.id == id))
            .unwrap_or(base);
        let short = crate::shorts::Short {
            id,
            title: args.title.trim().to_string(),
            source_start_us: first.source_start_us,
            source_end_us: last.source_end_us,
            reason: args.reason.unwrap_or_default().trim().to_string(),
            layout,
            media: Some(source.asset),
            edit: None,
            take: None,
            length_us: None,
            edited_start_us: None,
            edited_end_us: None,
        };
        let (start, end) = crate::shorts::edited_range_in(&short, document)
            .ok_or("A word at one end was cut from the video; pick words that are kept")?;
        let length =
            crate::shorts::slice_sequence(&document.sequence, &document.assets, start, end)
                .duration_us();
        if !(crate::shorts::MIN_SHORT_US..=crate::shorts::MAX_SHORT_US).contains(&length) {
            return Err(format!(
                "That is {:.1} s; a short is 3 s to 3 min",
                length as f64 / 1e6
            ));
        }
        let mut shorts = document.shorts.clone();
        shorts.push(short.clone());
        Ok((shorts, short))
    })?;
    let saved = commands::project_shorts_set_impl(
        ctx.state,
        before.handle.clone(),
        before.revision,
        shorts,
    )?;
    let mut lines = vec![format!("Made short {} \"{}\".", short.id, short.title)];
    if args.scenes != Some(false) {
        match commands::project_short_scenes_pick_impl(
            ctx.state,
            before.handle.clone(),
            saved.revision,
            short.id.clone(),
        ) {
            Ok(_) => {
                lines.push("Scenes:".into());
                lines.extend(with_reader(ctx.state, |reader| {
                    let document = reader.document();
                    let now = document
                        .shorts
                        .iter()
                        .find(|s| s.id == short.id)
                        .unwrap_or(&short);
                    Ok(scene_lines(document, now))
                })?);
            }
            Err(why) => lines.push(format!("Scenes not picked: {why}.")),
        }
    }
    text(applied(ctx, "short_create", &before, lines)?)
}

#[derive(Deserialize, JsonSchema, Default, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ScenesOp {
    #[default]
    Pick,
    Set,
    Clear,
    List,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SceneInput {
    /// Seconds on the short's own timeline.
    pub from_s: f64,
    pub to_s: f64,
    pub mode: crate::short_scenes::SceneMode,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShortScenesArgs {
    /// The short, by id or title. Without it, the shorts are listed.
    #[serde(default)]
    pub short: Option<String>,
    #[serde(default)]
    pub op: Option<ScenesOp>,
    /// For `set`: the scenes, e.g. [{"fromS": 0, "toS": 2.5, "mode": "camera"}].
    #[serde(default)]
    pub scenes: Option<Vec<SceneInput>>,
    /// How long the frame takes to change, in ms (0 is a hard cut, at most 2000).
    #[serde(default)]
    pub transition_ms: Option<u32>,
}

/// A short's scenes as lines, in seconds on its own timeline.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaImportArgs {
    /// Full paths of the files (or recording folders) to import.
    pub paths: Vec<String>,
}

fn media_import(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: MediaImportArgs = params(args)?;
    let before = opened(ctx)?;
    let known: std::collections::HashSet<String> = with_reader(ctx.state, |reader| {
        Ok(reader
            .document()
            .assets
            .iter()
            .map(|a| a.id.clone())
            .collect())
    })?;
    ctx.progress(0.1, "Importing");
    let project = commands::project_media_import_impl(
        ctx.state,
        before.handle.clone(),
        before.revision,
        args.paths,
    )?;
    let mut lines = Vec::new();
    for asset in project.assets.iter().filter(|a| !known.contains(&a.id)) {
        let streams: Vec<String> = asset
            .streams
            .iter()
            .map(|s| format!("{}.{} ({:?}, {:?})", asset.id, s.id, s.kind, s.role))
            .collect();
        lines.push(format!(
            "Imported {} as {} ({}): {}.",
            asset.name,
            asset.id,
            clock(asset.duration_us),
            streams.join(", ")
        ));
    }
    text(applied(ctx, "media_import", &before, lines)?)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShortFromTakeArgs {
    pub title: String,
    /// The take's files (media ids); the app tells the camera from the microphone. Or name
    /// them with `camera` and `voice`.
    #[serde(default)]
    pub media: Vec<String>,
    /// The take's camera: a media id (its camera, else its picture) or `<id>.<stream>`.
    #[serde(default)]
    pub camera: Option<String>,
    /// A separate microphone: a media id or `<id>.<stream>`. Default: the camera's own sound.
    #[serde(default)]
    pub voice: Option<String>,
    /// What the short is about, in a sentence (shown in the Shorts list).
    #[serde(default)]
    pub idea: Option<String>,
}

fn short_from_take(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ShortFromTakeArgs = params(args)?;
    let before = opened(ctx)?;
    ctx.progress(0.1, "Lining up the take");
    let made = commands::project_short_from_take_impl(
        ctx.state,
        before.handle.clone(),
        before.revision,
        args.title,
        args.camera,
        args.voice,
        args.media,
        args.idea,
    )?;
    let short = made
        .project
        .shorts
        .iter()
        .find(|s| s.id == made.short_id)
        .ok_or("The short was not saved")?;
    let take = short.take.as_ref().ok_or("The short has no take")?;
    let mut lines = vec![format!(
        "Made short {} \"{}\" from the take: {} long, voice {}{}.",
        short.id,
        short.title,
        clock(short.length_us.unwrap_or(0)),
        take.voice,
        take.camera
            .as_ref()
            .map(|c| format!(", camera {c}"))
            .unwrap_or_default()
    )];
    match made.sync.score {
        Some(score) if made.sync.offset_us != 0 => lines.push(format!(
            "Lined the camera up with the voice: the camera started {:.2} s {} (match {:.2}).",
            (made.sync.offset_us.unsigned_abs()) as f64 / 1e6,
            if made.sync.offset_us > 0 { "earlier" } else { "later" },
            score
        )),
        Some(score) => lines.push(format!(
            "The camera's sound did not clearly match the voice (match {score:.2}), so both start together; check the lip sync in the preview."
        )),
        None => {}
    }
    let transcribed = with_reader(ctx.state, |reader| {
        Ok(store::load_transcript(reader.root(), &take.voice)?.is_some())
    })?;
    lines.push(if transcribed {
        format!(
            "Its voice is transcribed: `transcript_get` with stream {}.",
            take.voice
        )
    } else {
        format!("Next: `transcribe` with stream {}.", take.voice)
    });
    text(applied(ctx, "short_from_take", &before, lines)?)
}

#[derive(Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FootageOp {
    #[default]
    List,
    Add,
    Remove,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FootageItemArgs {
    /// Under the take's words `fromWord`..`toWord` (ids from the take's transcript)...
    #[serde(default)]
    pub from_word: Option<String>,
    #[serde(default)]
    pub to_word: Option<String>,
    /// ...or seconds `atS`..`toS` on the short.
    #[serde(default)]
    pub at_s: Option<f64>,
    #[serde(default)]
    pub to_s: Option<f64>,
    /// What shows: the edited video from this word on (an id from the video's transcript; it
    /// must be one the video keeps)...
    #[serde(default)]
    pub source_word: Option<String>,
    /// The sound whose transcript `sourceWord` is from; default the video's speech.
    #[serde(default)]
    pub source_stream: Option<String>,
    /// ...or from `videoS` seconds on the video's timeline (the times `transcript_get` shows).
    #[serde(default)]
    pub video_s: Option<f64>,
    /// Play the footage's game or desktop sound, ducked under the voice. Default true.
    #[serde(default)]
    pub sound: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShortFootageArgs {
    /// The short's id (or title).
    pub short: String,
    #[serde(default)]
    pub op: Option<FootageOp>,
    #[serde(default)]
    pub items: Vec<FootageItemArgs>,
    /// For `remove`: seconds on the short; without them, all footage.
    #[serde(default)]
    pub from_s: Option<f64>,
    #[serde(default)]
    pub to_s: Option<f64>,
}

fn to_us(seconds: f64) -> Result<u64, String> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(format!("{seconds} is not a time in seconds"));
    }
    Ok((seconds * 1e6).round() as u64)
}

fn short_footage(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ShortFootageArgs = params(args)?;
    let before = opened(ctx)?;
    let short = with_reader(ctx.state, |reader| {
        let shorts = &reader.document().shorts;
        shorts
            .iter()
            .find(|s| s.id == args.short)
            .or_else(|| {
                shorts
                    .iter()
                    .find(|s| s.title.eq_ignore_ascii_case(args.short.trim()))
            })
            .cloned()
            .ok_or_else(|| format!("No short {}", args.short))
    })?;
    if short.take.is_none() {
        return Err(format!(
            "{} is cut from the video; footage goes under shorts made from a take (`short_from_take`)",
            short.id
        ));
    }
    let list = |short: &crate::shorts::Short| -> Result<Vec<String>, String> {
        let placed = with_reader(ctx.state, |reader| {
            crate::take_short::placed(reader.document(), short)
        })?;
        if placed.is_empty() {
            return Ok(vec![format!(
                "No footage under {} yet: the camera fills the frame.",
                short.id
            )]);
        }
        Ok(placed
            .iter()
            .map(|f| {
                let from = f
                    .video_us
                    .map(|v| format!("the video at {}", clock(v)))
                    .unwrap_or_else(|| "a part of the video cut since".into());
                format!(
                    "{}-{}: {from}{}",
                    clock(f.at_us),
                    clock(f.to_us),
                    if f.sound { ", with its sound" } else { "" }
                )
            })
            .collect())
    };
    let change = match args.op.unwrap_or_default() {
        FootageOp::List => return text(list(&short)?.join("\n")),
        FootageOp::Remove => commands::FootageChange::Remove {
            from_us: args.from_s.map(to_us).transpose()?,
            to_us: args.to_s.map(to_us).transpose()?,
        },
        FootageOp::Add => {
            let items = with_reader(ctx.state, |reader| {
                let take = short.take.as_ref().ok_or("No take")?;
                let take_words = store::load_transcript(reader.root(), &take.voice)?;
                let mut out = Vec::new();
                for item in &args.items {
                    let (at_us, to) = match (&item.from_word, &item.to_word, item.at_s, item.to_s) {
                        (Some(from), Some(to), _, _) => {
                            let transcript = take_words.as_ref().ok_or_else(|| {
                                format!(
                                    "The take is not transcribed: `transcribe` with stream {}",
                                    take.voice
                                )
                            })?;
                            crate::take_short::words_at(&short, transcript, from, to)?
                        }
                        (_, _, Some(a), Some(b)) => (to_us(a)?, to_us(b)?),
                        _ => {
                            return Err("Each item needs `fromWord`/`toWord` or `atS`/`toS`".into())
                        }
                    };
                    let video_us = match (&item.source_word, item.video_s) {
                        (Some(word), _) => {
                            let key = match &item.source_stream {
                                Some(key) => key.clone(),
                                None => video_speech(reader, &take.voice)?,
                            };
                            let transcript = store::load_transcript(reader.root(), &key)?
                                .ok_or_else(|| format!("{key} is not transcribed"))?;
                            let source = transcript
                                .words
                                .iter()
                                .find(|w| &w.id == word)
                                .ok_or_else(|| format!("No word {word} in {key}"))?;
                            // Only what the video keeps: the word must be on its timeline.
                            reader
                                .document()
                                .transcript_clock(&key)?
                                .source_to_edited_us(source.source_start_us)
                                .ok_or_else(|| {
                                    format!("Word {word} was cut from the video; pick one it keeps")
                                })?
                        }
                        (None, Some(seconds)) => to_us(seconds)?,
                        _ => return Err("Each item needs `sourceWord` or `videoS`".into()),
                    };
                    out.push(crate::take_short::Footage {
                        at_us,
                        to_us: to,
                        video_us,
                        sound: item.sound.unwrap_or(true),
                    });
                }
                Ok(out)
            })?;
            commands::FootageChange::Add { items }
        }
    };
    let project = commands::project_short_footage_impl(
        ctx.state,
        before.handle.clone(),
        before.revision,
        short.id.clone(),
        change,
    )?;
    let after = project
        .shorts
        .iter()
        .find(|s| s.id == short.id)
        .ok_or("The short was removed")?;
    let mut lines = vec![format!("Footage under {} now:", short.id)];
    lines.extend(list(after)?);
    text(applied(ctx, "short_footage", &before, lines)?)
}

/// The id of the take short `wanted` names (its id or title).
fn take_short_id(ctx: &Ctx, wanted: &str) -> Result<String, String> {
    with_reader(ctx.state, |reader| {
        let shorts = &reader.document().shorts;
        let short = shorts
            .iter()
            .find(|s| s.id == wanted)
            .or_else(|| {
                shorts
                    .iter()
                    .find(|s| s.title.eq_ignore_ascii_case(wanted.trim()))
            })
            .ok_or_else(|| format!("No short {wanted}"))?;
        if short.take.is_none() {
            return Err(format!(
                "{} is cut from the video: clean the video itself, and the short follows",
                short.id
            ));
        }
        Ok(short.id.clone())
    })
}

/// The video's transcribed speech: the first transcribed sound on the main timeline that is
/// not the take's voice.
fn video_speech(reader: &ProjectReader, take_voice: &str) -> Result<String, String> {
    sounds(reader)
        .into_iter()
        .filter(|s| s.key != take_voice && s.transcribed)
        .min_by_key(|s| (!s.placed, !s.speech))
        .map(|s| s.key)
        .ok_or_else(|| "The video is not transcribed: `transcribe` it first".into())
}

fn scene_lines(
    document: &crate::project::revision::EditDocument,
    short: &crate::shorts::Short,
) -> Vec<String> {
    let Ok(timeline) = crate::shorts::short_timeline(document, short) else {
        return vec!["Part of it was cut from the video, so its scenes can't be placed.".into()];
    };
    let ranges = crate::short_scenes::edited_ranges(&short.layout.scenes, |media| {
        timeline
            .clock_asset(media)
            .map(|asset| timeline.asset_clock(asset))
    });
    if ranges.is_empty() {
        return vec!["Split throughout.".into()];
    }
    ranges
        .iter()
        .map(|&(a, b, mode)| {
            format!(
                "{:.1}-{:.1} s: {}",
                a as f64 / 1e6,
                b as f64 / 1e6,
                match mode {
                    crate::short_scenes::SceneMode::Camera => "camera only",
                    crate::short_scenes::SceneMode::Screen => "screen only",
                }
            )
        })
        .collect()
}

fn short_scenes(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ShortScenesArgs = params(args)?;
    let before = opened(ctx)?;
    let Some(wanted) = args.short else {
        return text(with_reader(ctx.state, |reader| {
            let document = reader.document();
            if document.shorts.is_empty() {
                return Ok(
                    "No shorts yet: `shorts` finds some, or make one in the Shorts Studio.".into(),
                );
            }
            let mut lines = Vec::new();
            for short in &document.shorts {
                lines.push(format!("{} \"{}\":", short.id, short.title));
                lines.extend(
                    scene_lines(document, short)
                        .into_iter()
                        .map(|l| format!("  {l}")),
                );
            }
            Ok(lines.join("\n"))
        })?);
    };
    let (short, shorts) = with_reader(ctx.state, |reader| {
        let shorts = reader.document().shorts.clone();
        let short = shorts
            .iter()
            .find(|s| s.id == wanted)
            .or_else(|| {
                shorts
                    .iter()
                    .find(|s| s.title.eq_ignore_ascii_case(wanted.trim()))
            })
            .cloned()
            .ok_or_else(|| {
                format!(
                    "No short {wanted}. Shorts: {}",
                    shorts
                        .iter()
                        .map(|s| format!("{} \"{}\"", s.id, s.title))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        Ok((short, shorts))
    })?;
    let op = args.op.unwrap_or_default();
    if op == ScenesOp::List {
        let lines = with_reader(ctx.state, |reader| {
            Ok(scene_lines(reader.document(), &short))
        })?;
        return text(lines.join("\n"));
    }
    let mut revision = before.revision;
    if op == ScenesOp::Pick {
        revision = commands::project_short_scenes_pick_impl(
            ctx.state,
            before.handle.clone(),
            revision,
            short.id.clone(),
        )?
        .revision;
    }
    let set: Option<Vec<crate::short_scenes::Scene>> = match op {
        ScenesOp::Clear => Some(Vec::new()),
        ScenesOp::Set => {
            let wanted = args.scenes.ok_or("`set` needs `scenes`")?;
            let mut ranges = Vec::new();
            for scene in wanted {
                if !(scene.from_s.is_finite() && scene.from_s >= 0.0 && scene.to_s > scene.from_s) {
                    return Err(format!(
                        "A scene runs from a time to a later one: {}-{}",
                        scene.from_s, scene.to_s
                    ));
                }
                ranges.push((
                    (scene.from_s * 1e6) as u64,
                    (scene.to_s * 1e6) as u64,
                    scene.mode,
                ));
            }
            Some(with_reader(ctx.state, |reader| {
                crate::shorts::scenes_at(reader.document(), &short, &ranges)
            })?)
        }
        _ => None,
    };
    if set.is_some() || args.transition_ms.is_some() {
        let mut shorts =
            with_reader(ctx.state, |reader| Ok(reader.document().shorts.clone())).unwrap_or(shorts);
        for s in shorts.iter_mut().filter(|s| s.id == short.id) {
            if let Some(scenes) = set.clone() {
                s.layout.scenes = scenes;
            }
            if let Some(ms) = args.transition_ms {
                s.layout.scene_transition_ms = ms;
            }
        }
        commands::project_shorts_set_impl(ctx.state, before.handle.clone(), revision, shorts)?;
    }
    let mut lines = vec![format!("Scenes of \"{}\":", short.title)];
    lines.extend(with_reader(ctx.state, |reader| {
        let document = reader.document();
        let now = document
            .shorts
            .iter()
            .find(|s| s.id == short.id)
            .unwrap_or(&short);
        Ok(scene_lines(document, now))
    })?);
    text(applied(ctx, "short_scenes", &before, lines)?)
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CheckArgs {
    /// Ids of findings to fix, from a check of the edit as it is now, or ["all"] for every
    /// finding that has a fix. Without it nothing changes.
    #[serde(default)]
    pub fix: Option<Vec<String>>,
}

fn check(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: CheckArgs = params(args)?;
    let before = opened(ctx)?;
    let report = commands::project_check_impl(ctx.state, before.handle.clone())?;
    let Some(ids) = args.fix.filter(|ids| !ids.is_empty()) else {
        return text(crate::check::report_text(&report));
    };
    let all = ids.iter().any(|id| id == "all");
    let fixed: Vec<String> = report
        .findings
        .iter()
        .filter(|f| f.fix.is_some() && (all || ids.contains(&f.id)))
        .filter_map(|f| {
            f.fix_label
                .as_ref()
                .map(|label| format!("{} ({label})", f.id))
        })
        .collect();
    commands::project_check_fix_impl(ctx.state, before.handle.clone(), before.revision, ids)?;
    let after = commands::project_check_impl(ctx.state, before.handle.clone())?;
    let lines = vec![
        format!(
            "Fixed {}, each its own undo step: {}.",
            plural(fixed.len(), "finding"),
            fixed.join("; ")
        ),
        crate::check::report_text(&after),
    ];
    text(applied(ctx, "check", &before, lines)?)
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
    /// Export vertical shorts instead of the video: a short's id or title, or "all". They go
    /// next to the project, named after it and the short (1080x1920), one after another.
    #[serde(default)]
    pub short: Option<String>,
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
    if let Some(wanted) = args.short {
        return export_shorts(ctx, &open.handle, &wanted, settings);
    }
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
    let status = wait_for_export(ctx, &started.job_id, "Exporting")?;
    text(format!(
        "Exported {}x{} {} to {} in {:.0} s.{stale}",
        status.width,
        status.height,
        status.video_codec,
        status.output_path.unwrap_or_default(),
        begun.elapsed().as_secs_f64()
    ))
}

/// Waits for export `job_id` to finish, reporting progress as `doing`.
fn wait_for_export(
    ctx: &Ctx,
    job_id: &str,
    doing: &str,
) -> Result<crate::export::ExportStatus, String> {
    loop {
        let status = commands::export_status_impl(ctx.state, Some(job_id.to_string()))?;
        use crate::export::ExportState::*;
        match status.state {
            Completed => return Ok(status),
            Failed | Cancelled => return Err(export_failure(&status)),
            _ => {
                let fraction = if status.progress_denominator > 0 {
                    status.progress_numerator as f64 / status.progress_denominator as f64
                } else {
                    0.0
                };
                ctx.progress(fraction, &format!("{doing}, {:.0}%", fraction * 100.0));
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }
}

/// Exports the short `wanted` (id or title) or "all", one after another.
fn export_shorts(
    ctx: &Ctx,
    handle: &str,
    wanted: &str,
    mut settings: crate::export::ExportSettings,
) -> Result<Vec<Output>, String> {
    let shorts: Vec<(String, String)> = with_reader(ctx.state, |reader| {
        Ok(reader
            .document()
            .shorts
            .iter()
            .map(|s| (s.id.clone(), s.title.clone()))
            .collect())
    })?;
    let chosen: Vec<&(String, String)> = if wanted.trim().eq_ignore_ascii_case("all") {
        shorts.iter().collect()
    } else {
        shorts
            .iter()
            .filter(|(id, title)| id == wanted || title.eq_ignore_ascii_case(wanted.trim()))
            .take(1)
            .collect()
    };
    if chosen.is_empty() {
        return Err(if shorts.is_empty() {
            "No shorts yet: `short_create` or `shorts` makes them.".into()
        } else {
            format!(
                "No short {wanted}. Shorts: {}",
                shorts
                    .iter()
                    .map(|(id, title)| format!("{id} \"{title}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });
    }
    // Each short is named after itself next to the project; one file name can't hold them.
    settings.destination = None;
    let mut lines = Vec::new();
    for (i, (id, title)) in chosen.iter().enumerate() {
        let started = commands::project_short_export_impl(
            ctx.state,
            handle.to_string(),
            id.clone(),
            settings.clone(),
        )?;
        let doing = format!("Exporting short {} of {} ({title})", i + 1, chosen.len());
        let status = wait_for_export(ctx, &started.job_id, &doing)?;
        lines.push(format!(
            "\"{title}\": {}x{} to {}",
            status.width,
            status.height,
            status.output_path.unwrap_or_default()
        ));
    }
    text(lines.join("\n"))
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

// ---------------------------------------------------------------------------------------
// Graphics.

fn render_line(render: &crate::graphics::render::RenderState) -> String {
    match render.state.as_str() {
        "rendering" => format!("rendering {:.0}%", render.progress * 100.0),
        "queued" => "waiting to render".into(),
        "failed" => format!(
            "render failed: {}",
            render
                .error
                .clone()
                .unwrap_or_default()
                .replace('\n', " / ")
        ),
        _ => "rendered or idle".into(),
    }
}

fn graphics_list(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    params::<NoArgs>(args)?;
    let runtime = crate::graphics::runtime::status();
    let templates = crate::graphics::templates::views();
    let in_project = match current(ctx) {
        Ok(open) => commands::graphics::project_graphics_list_impl(ctx.state, open.handle)?,
        Err(_) => Vec::new(),
    };
    let mut out = json!({
        "renderer": if runtime.installed {
            format!("installed (HyperFrames {})", runtime.hyperframes)
        } else {
            "not installed: ask the user, then call graphics_install".to_string()
        },
        "templates": templates.iter().map(|t| json!({
            "name": t.name,
            "label": t.label,
            "description": t.description,
            "lengthUs": t.length_us,
            "variables": t.variables.iter().map(|v| {
                let mut decl = json!({"id": v.id, "type": v.kind, "label": v.label, "default": v.default});
                if !v.options.is_empty() {
                    decl["options"] = json!(v.options.iter().map(|o| o.value.clone()).collect::<Vec<_>>());
                }
                decl
            }).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    });
    out["library"] = json!(
        crate::graphics::library::list(&crate::graphics::library::dir())
            .iter()
            .map(|item| json!({
                "id": item.id,
                "name": item.item.name,
                "label": item.item.label,
                "useWhen": item.item.use_when,
                "kind": if item.item.general { "template" } else { "as is (not general)" },
                "version": item.item.version,
                "lengthUs": item.item.length_us,
                "variables": item.variables.iter().map(|v| {
                    json!({"id": v.id, "type": v.kind, "default": v.default})
                }).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>()
    );
    out["graphics"] = json!(in_project
        .iter()
        .map(|g| json!({
            "id": g.graphic.asset_id,
            "name": g.graphic.name,
            "template": g.graphic.template,
            "folder": g.graphic.folder,
            "variables": g.graphic.variables,
            "brand": g.graphic.brand.clone().unwrap_or_else(|| "default".into()),
            "plays": g.clips.iter().map(|(id, start, length)| json!({
                "clip": id,
                "at": clock(*start),
                "atUs": start,
                "lengthUs": length,
            })).collect::<Vec<_>>(),
            "render": render_line(&g.graphic.render),
        }))
        .collect::<Vec<_>>());
    super::tools::pretty(&out)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GraphicAddArgs {
    /// A template name from `graphics` (title, step, shortcut, key-point, recap, end-card, blank).
    #[serde(default)]
    pub template: Option<String>,
    /// Or: a library item's id or name from `graphics` (copied into the project).
    #[serde(default)]
    pub library: Option<String>,
    /// Where it starts on the edited timeline, in microseconds.
    #[serde(default)]
    pub at_us: Option<u64>,
    /// Or: the transcript word it starts at (an id from transcript_get).
    #[serde(default)]
    pub at_word: Option<String>,
    /// The sound whose transcript `atWord` is from; default the transcribed speech.
    #[serde(default)]
    pub stream: Option<String>,
    /// How long it plays; default the template's.
    #[serde(default)]
    pub length_us: Option<u64>,
    #[serde(default)]
    pub name: Option<String>,
    /// Values for its variables, by id.
    #[serde(default)]
    pub variables: serde_json::Map<String, Value>,
}

fn graphic_add(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: GraphicAddArgs = params(args)?;
    let before = opened(ctx)?;
    let at = match (args.at_us, &args.at_word) {
        (Some(at), None) => at,
        (None, Some(word)) => {
            let key = pick_sound(ctx, args.stream.clone(), Want::Transcribed)?;
            with_reader(ctx.state, |reader| {
                let transcript = store::load_transcript(reader.root(), &key)?
                    .ok_or("That sound is not transcribed")?;
                let found = transcript
                    .words
                    .iter()
                    .find(|w| &w.id == word)
                    .ok_or_else(|| format!("No word {word} in the transcript of {key}"))?;
                reader
                    .document()
                    .transcript_clock(&key)?
                    .source_to_edited_us(found.source_start_us)
                    .ok_or_else(|| format!("Word {word} was cut from the video"))
            })?
        }
        _ => return Err("Give one of `atUs` or `atWord`".into()),
    };
    let (created, from) = match (&args.template, &args.library) {
        (Some(template), None) => (
            commands::graphics::project_graphic_create_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                template.clone(),
                args.name,
                args.variables,
                at,
                args.length_us,
            )?,
            template.clone(),
        ),
        (None, Some(item)) => {
            let created = commands::graphics::project_graphic_from_library_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                item.clone(),
                at,
                args.length_us,
            )?;
            if args.name.is_some() || !args.variables.is_empty() {
                commands::graphics::project_graphic_set_impl(
                    ctx.state,
                    before.handle.clone(),
                    created.project.revision,
                    created.asset_id.clone(),
                    args.variables,
                    args.name,
                    None,
                )?;
            }
            (created, format!("library {item}"))
        }
        _ => return Err("Give one of `template` or `library`".into()),
    };
    let (name, length) = with_reader(ctx.state, |reader| {
        let document = reader.document();
        Ok((
            document
                .asset(&created.asset_id)
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            document
                .sequence
                .clip(&created.clip_id)
                .map_or(0, |(_, c)| c.duration_us),
        ))
    })?;
    let mut lines = vec![format!(
        "Added graphic {} \"{name}\" ({}) at {} for {:.1} s, clip {}.",
        created.asset_id,
        from,
        clock(at),
        length as f64 / 1e6,
        created.clip_id
    )];
    lines.push(if crate::graphics::runtime::status().installed {
        "It renders in the background; graphic_render waits for it.".into()
    } else {
        "The renderer is not installed, so it shows as a placeholder: ask the user, then graphics_install.".into()
    });
    text(applied(ctx, "graphic_add", &before, lines)?)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GraphicSetArgs {
    /// The graphic's id (from `graphics`).
    pub graphic: String,
    /// Variable values to change, by id; null puts one back to its default.
    #[serde(default)]
    pub variables: serde_json::Map<String, Value>,
    #[serde(default)]
    pub name: Option<String>,
    /// How long its (first) clip plays.
    #[serde(default)]
    pub length_us: Option<u64>,
    /// Its brand kit: `default` (the project's), `none`, or a kit's name from brand_kits.
    #[serde(default)]
    pub brand: Option<String>,
    /// A short (id or title) to set the graphic's `slot` in.
    #[serde(default)]
    pub short: Option<String>,
    /// Where it plays in that short.
    #[serde(default)]
    pub slot: Option<crate::shorts::GraphicSlot>,
}

fn graphic_set(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: GraphicSetArgs = params(args)?;
    let before = opened(ctx)?;
    let mut lines = Vec::new();
    if !args.variables.is_empty() || args.name.is_some() || args.brand.is_some() {
        let updated = commands::graphics::project_graphic_set_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            args.graphic.clone(),
            args.variables.clone(),
            args.name.clone(),
            args.brand.clone(),
        )?;
        lines.push(format!(
            "Graphic {} \"{}\": {}{}.",
            args.graphic,
            updated.graphic.name,
            serde_json::to_string(&updated.graphic.variables).unwrap_or_default(),
            match &updated.graphic.brand {
                Some(kit) => format!(", brand kit {kit}"),
                None => ", the project's brand kit".into(),
            }
        ));
    }
    match (&args.short, args.slot) {
        (Some(wanted), Some(slot)) => {
            let now = current(ctx)?;
            let (title, shorts) = with_reader(ctx.state, |reader| {
                let document = reader.document();
                let mut shorts = document.shorts.clone();
                let short = shorts
                    .iter_mut()
                    .find(|s| s.id == *wanted || s.title.eq_ignore_ascii_case(wanted.trim()))
                    .ok_or_else(|| format!("No short {wanted} (see shorts)"))?;
                let timeline = crate::shorts::short_timeline(document, short)?;
                let clips: Vec<String> = timeline
                    .sequence
                    .clips()
                    .filter(|(_, c)| c.asset == args.graphic)
                    .map(|(_, c)| c.id.clone())
                    .collect();
                if clips.is_empty() {
                    return Err(format!(
                        "Graphic {} does not play in short \"{}\"",
                        args.graphic, short.title
                    ));
                }
                for clip in clips {
                    if slot == crate::shorts::GraphicSlot::Full {
                        short.layout.graphic_slots.remove(&clip);
                    } else {
                        short.layout.graphic_slots.insert(clip, slot);
                    }
                }
                Ok((short.title.clone(), shorts))
            })?;
            commands::project_shorts_set_impl(ctx.state, now.handle, now.revision, shorts)?;
            lines.push(format!(
                "In short \"{title}\" it plays {}.",
                match slot {
                    crate::shorts::GraphicSlot::Full => "over the whole frame",
                    crate::shorts::GraphicSlot::Camera => "in the camera's part",
                    crate::shorts::GraphicSlot::Screen => "in the screen's part",
                    crate::shorts::GraphicSlot::OverScreen => "over the screen, under the captions",
                }
            ));
        }
        (None, None) => {}
        _ => return Err("Give `short` and `slot` together".into()),
    }
    if let Some(length) = args.length_us {
        let now = current(ctx)?;
        let clip = with_reader(ctx.state, |reader| {
            reader
                .document()
                .sequence
                .clips()
                .find(|(_, c)| c.asset == args.graphic)
                .map(|(_, c)| (c.id.clone(), c.start_us))
                .ok_or_else(|| "That graphic is not on the timeline".to_string())
        })?;
        let length = length.clamp(
            crate::graphics::MIN_LENGTH_US,
            crate::graphics::MAX_LENGTH_US,
        );
        commands::project_sequence_edit_impl(
            ctx.state,
            now.handle,
            now.revision,
            crate::sequence::edit::SequenceEdit::TrimClip {
                clip_id: clip.0,
                edge: crate::sequence::edit::Edge::End,
                to_us: clip.1 + length,
                ripple: None,
            },
            None,
        )?;
        lines.push(format!("It plays for {:.1} s.", length as f64 / 1e6));
    }
    if lines.is_empty() {
        return Err("Give `variables`, `name`, `lengthUs`, `brand` or `short` and `slot`".into());
    }
    lines.push("It renders again by itself.".into());
    text(applied(ctx, "graphic_set", &before, lines)?)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GraphicWriteArgs {
    pub graphic: String,
    /// A path inside its folder, like index.html or assets/logo.svg.
    pub path: String,
    /// The file's text (for a binary file, base64 with `base64: true`).
    pub contents: String,
    #[serde(default)]
    pub base64: bool,
}

fn graphic_folder(ctx: &Ctx, graphic: &str) -> Result<std::path::PathBuf, String> {
    with_reader(ctx.state, |reader| {
        reader
            .document()
            .asset(graphic)
            .filter(|a| a.is_graphic())
            .ok_or_else(|| format!("No graphic {graphic} in the project (see `graphics`)"))?;
        Ok(crate::graphics::folder(reader.root(), graphic))
    })
}

fn graphic_write_file(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: GraphicWriteArgs = params(args)?;
    let dir = graphic_folder(ctx, &args.graphic)?;
    let bytes = if args.base64 {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(args.contents.trim())
            .map_err(|e| format!("Not base64: {e}"))?
    } else {
        args.contents.into_bytes()
    };
    crate::graphics::write_file(&dir, &args.path, &bytes)?;
    let mut lines = vec![format!("Wrote {} ({} bytes).", args.path, bytes.len())];
    if args.path.ends_with(".html") {
        match crate::graphics::render::lint(&dir) {
            Ok(lint) => lines.push(format!("Lint: {lint}")),
            Err(why) => lines.push(format!("Not linted: {why}")),
        }
    }
    text(lines.join("\n"))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GraphicRenderArgs {
    /// One graphic's id; default every graphic on the timeline.
    #[serde(default)]
    pub graphic: Option<String>,
}

fn graphic_render(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: GraphicRenderArgs = params(args)?;
    let (root, specs) = with_reader(ctx.state, |reader| {
        let document = reader.document();
        let canvas = document.layout.preview_dimensions()?;
        let specs: Vec<_> = crate::graphics::render::specs_for(reader.root(), document, canvas)
            .into_iter()
            .filter(|s| args.graphic.as_ref().is_none_or(|g| &s.asset_id == g))
            .collect();
        Ok((reader.root().to_path_buf(), specs))
    })?;
    if specs.is_empty() {
        return Err(match args.graphic {
            Some(id) => format!("Graphic {id} is not on the timeline"),
            None => "There are no graphics on the timeline".into(),
        });
    }
    let mut lines = Vec::new();
    let mut linted = std::collections::BTreeSet::new();
    for spec in &specs {
        if linted.insert(spec.asset_id.clone()) {
            let dir = crate::graphics::folder(&root, &spec.asset_id);
            if let Ok(lint) = crate::graphics::render::lint(&dir) {
                lines.push(format!("{} \"{}\" lint: {lint}", spec.asset_id, spec.name));
            }
        }
    }
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let started = std::time::Instant::now();
    crate::graphics::render::ensure(&specs, &cancel, &mut |fraction, label| {
        ctx.progress(fraction, label)
    })?;
    lines.push(format!(
        "{} rendered for the {}x{} canvas in {:.0} s; check them with frame_render inside each.",
        plural(specs.len(), "graphic"),
        specs[0].width,
        specs[0].height,
        started.elapsed().as_secs_f64()
    ));
    text(lines.join("\n"))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GraphicOnlyArgs {
    /// The graphic's id (from `graphics`).
    pub graphic: String,
}

/// How to turn a graphic into a template for the library. The gate (graphic_check) tests each
/// point without a model.
const TEMPLATE_RULES: &str = "Rewrite the files (graphic_write_file, or the folder directly), \
keeping its look and motion, so it takes any text and fits anywhere:
1. Text: every visible text and number becomes a variable in data-composition-variables on \
<html> (type string, or number for counts), shown with data-var-text or set by the script from \
window.__hyperframes.getVariables(). Defaults are good example text; a list is one string with \
an item per line. No text written into the markup that no variable fills.
2. Colours: only the shared color variables background, text and accent (declare all three and \
use var(--background), var(--text), var(--accent)), so the user's brand kits apply. Tints come \
from color-mix() or opacity, not new hex colours.
3. Timing: read the length from the root's data-duration (parseFloat(root.dataset.duration) * \
1000) and place every animation as a share of it: in over about 300 ms, out over the last \
450 ms, nothing at fixed end times, so it fits any clip length.
4. Layout: vw/vh/vmin with flex or grid, no px sizes; text wraps (max-width, never nowrap) and \
long text still fits; an @media (orientation: portrait) block re-flows it for 9:16 shorts.
5. Page: transparent, with a card (var(--background)) that fades in and out, so the first and \
last frames are see-through. A graphic meant to hold to the end of its clip (an end card) puts \
data-holds-end on the root element and fades in only; its first frame is still see-through.
6. No network: no CDN scripts, web fonts or http(s) URLs; images and fonts go beside index.html.
Then graphic_check, which renders it at 16:9 and 9:16, 3 s and 8 s, with long text and with \
each text variable changed, and says what fails. Fix and check again until it passes; then ask \
the user to keep it and call library_save with a label, a one-line useWhen and lengthUs.";

fn graphic_templatize(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: GraphicOnlyArgs = params(args)?;
    let dir = graphic_folder(ctx, &args.graphic)?;
    let source = crate::graphics::read_source(&dir)?;
    let entry = dir.join(crate::graphics::ENTRY);
    let html = std::fs::read_to_string(&entry).map_err(|e| format!("{}: {e}", entry.display()))?;
    let declared = crate::graphics::parse_declarations(&html)?;
    let mut files = Vec::new();
    crate::graphics::render::list_files(&dir, &dir, &mut files)?;
    let texts: Vec<(String, String)> = files
        .iter()
        .filter(|(relative, _)| relative != crate::graphics::SOURCE_FILE)
        .filter_map(|(relative, path)| {
            Some((relative.clone(), std::fs::read_to_string(path).ok()?))
        })
        .collect();
    let mut lines = vec![
        format!(
            "Graphic {} \"{}\" in {}{}.",
            args.graphic,
            source.name,
            dir.display(),
            source
                .template
                .as_deref()
                .map(|t| format!(" (from template {t})"))
                .unwrap_or_default()
        ),
        format!(
            "Files: {}.",
            files
                .iter()
                .map(|(relative, _)| relative.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        format!(
            "Variables now: {}.",
            if declared.is_empty() {
                "none".to_string()
            } else {
                declared
                    .iter()
                    .map(|d| format!("{} ({})", d.id, d.kind))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ),
    ];
    let seen: Vec<String> = crate::graphics::gate::static_checks(&html, &declared, &texts)
        .into_iter()
        .filter(|c| c.status != crate::graphics::gate::Status::Pass)
        .map(|c| format!("- {}: {}", c.label, c.reason))
        .collect();
    if !seen.is_empty() {
        lines.push("What the app sees already:".into());
        lines.extend(seen);
    }
    let ids: Vec<String> = declared.iter().map(|d| d.id.clone()).collect();
    if let Some(close) = crate::graphics::library::closest(
        &crate::graphics::library::dir(),
        &source.name,
        &ids,
        None,
    ) {
        lines.push(close_line(&close));
    }
    lines.push(String::new());
    lines.push(TEMPLATE_RULES.into());
    lines.push(String::new());
    lines.push("index.html:".into());
    lines.push(html);
    text(lines.join("\n"))
}

fn close_line(close: &crate::graphics::library::CloseMatch) -> String {
    match close.kind.as_str() {
        "template" => format!(
            "Close to the bundled template {} (\"{}\"): {}. Ask the user whether a library item \
             is worth it (library_save asNew: true) or the template does the job.",
            close.id, close.label, close.why
        ),
        _ => format!(
            "Close to library item {} \"{}\" (version {}): {}. Ask the user: update it as its \
             next version (library_save update: {}) or keep both (asNew: true).",
            close.id,
            close.label,
            close.version.unwrap_or(1),
            close.why,
            close.id
        ),
    }
}

/// The gate's answer as lines, and its contact sheet.
fn gate_outputs(
    report: &crate::graphics::gate::Report,
    mut lines: Vec<String>,
) -> Result<Vec<Output>, String> {
    lines.extend(report.lines());
    let mut out = Vec::new();
    if let Some(png) = report.sheet.as_ref().and_then(|s| std::fs::read(s).ok()) {
        out.push(Output::Png(png));
        lines.push(
            "The contact sheet: 16:9 with its text, then with long text; 9:16 the same.".into(),
        );
    }
    out.push(Output::Text(lines.join("\n")));
    Ok(out)
}

fn graphic_check(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: GraphicOnlyArgs = params(args)?;
    let open = opened(ctx)?;
    graphic_folder(ctx, &args.graphic)?;
    let report = commands::graphics::project_graphic_check_impl(
        ctx.state,
        open.handle,
        args.graphic.clone(),
        &mut |fraction, message| ctx.progress(fraction, message),
    )?;
    let head = vec![
        format!(
            "Gate for graphic {}: {} ({} s).",
            args.graphic,
            if report.passed { "passed" } else { "failed" },
            report.seconds
        ),
        if report.passed {
            "It can go to the library as a template: ask the user, then library_save.".into()
        } else {
            "Fix what failed (graphic_templatize has the rules) and check again.".into()
        },
    ];
    gate_outputs(&report, head)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySaveArgs {
    /// The graphic's id (from `graphics`).
    pub graphic: String,
    /// What the library shows, like "Quote card"; default the graphic's name.
    #[serde(default)]
    pub label: Option<String>,
    /// One line on when to use it, for agents picking from the library.
    pub use_when: String,
    /// Its short name for graphic_add (lowercase words joined with -); default from the label.
    #[serde(default)]
    pub name: Option<String>,
    /// How long it plays when used; default its clip's length here.
    #[serde(default)]
    pub length_us: Option<u64>,
    /// A library item (id or name) to replace as its next version.
    #[serde(default)]
    pub update: Option<String>,
    /// Save a new item although a close one exists.
    #[serde(default)]
    pub as_new: bool,
    /// Save it although it fails the gate, marked not general. Only at the user's word.
    #[serde(default)]
    pub as_is: bool,
}

fn library_save(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: LibrarySaveArgs = params(args)?;
    let open = opened(ctx)?;
    graphic_folder(ctx, &args.graphic)?;
    if args.use_when.trim().is_empty() {
        return Err("Give `useWhen`: one line on when to use it".into());
    }
    let outcome = commands::graphics::project_graphic_library_save_impl(
        ctx.state,
        open.handle,
        commands::graphics::LibrarySave {
            asset_id: args.graphic.clone(),
            label: args.label,
            name: args.name,
            use_when: Some(args.use_when),
            length_us: args.length_us,
            as_is: args.as_is,
            update: args.update,
            as_new: args.as_new,
        },
        &mut |fraction, message| ctx.progress(fraction, message),
    )?;
    let report = &outcome.check.report;
    let head = match (&outcome.saved, &outcome.close) {
        (Some(saved), _) => vec![
            format!(
                "Saved to the library as \"{}\" ({}, {}), version {}, {}.",
                saved.item.label,
                saved.item.name,
                saved.id,
                saved.item.version,
                if saved.item.general {
                    "a general template"
                } else {
                    "as is, not general"
                }
            ),
            format!(
                "Any project can use it: graphic_add with library: {}.",
                saved.item.name
            ),
        ],
        (None, Some(close)) => vec![format!("Not saved yet. {}", close_line(close))],
        (None, None) => vec![
            "Not saved: it fails the gate. Fix what failed and try again; only if the user \
             says so, save it as is (asIs: true), marked not general."
                .into(),
        ],
    };
    gate_outputs(report, head)
}

fn graphics_install(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    params::<NoArgs>(args)?;
    if crate::graphics::runtime::status().installed {
        return text("The graphics renderer is already installed.");
    }
    let status = commands::graphics::graphics_runtime_install_impl(&mut |p| {
        let fraction = p
            .bytes_total
            .map_or(0.0, |total| p.bytes_done as f64 / total.max(1) as f64);
        ctx.progress(fraction, &p.step)
    })?;
    text(format!(
        "Installed the graphics renderer (HyperFrames {}) in {}.",
        status.hyperframes, status.dir
    ))
}

fn brand_kits(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    params::<NoArgs>(args)?;
    let handle = current(ctx).ok().map(|open| open.handle);
    let has_project = handle.is_some();
    let view = commands::graphics::graphics_brands_impl(ctx.state, handle)?;
    let out = json!({
        "kits": view.kits.iter().map(|kit| json!({
            "name": kit.name,
            "background": kit.background,
            "text": kit.text,
            "accent": kit.accent,
            "font": kit.font,
            "logo": kit.logo.is_some(),
        })).collect::<Vec<_>>(),
        "projectDefault": if has_project {
            json!(view.project_default.unwrap_or_else(|| "none".into()))
        } else {
            json!("no project open")
        },
        "folder": view.folder,
    });
    super::tools::pretty(&out)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrandPickArgs {
    /// A kit's name from brand_kits, or `none`; with `graphic` also `default` (the project's).
    pub brand: String,
    /// One graphic's id; without it the project's default kit is set.
    #[serde(default)]
    pub graphic: Option<String>,
}

fn brand_pick(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: BrandPickArgs = params(args)?;
    let before = opened(ctx)?;
    let line = match &args.graphic {
        Some(graphic) => {
            let updated = commands::graphics::project_graphic_set_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                graphic.clone(),
                Default::default(),
                None,
                Some(args.brand.clone()),
            )?;
            format!(
                "Graphic {graphic} \"{}\" uses {}.",
                updated.graphic.name,
                match &updated.graphic.brand {
                    Some(kit) if kit == crate::graphics::brands::NONE => "no brand kit".into(),
                    Some(kit) => format!("brand kit {kit}"),
                    None => "the project's brand kit".into(),
                }
            )
        }
        None => {
            let brand = match args.brand.trim() {
                crate::graphics::brands::NONE | "" => None,
                crate::graphics::brands::DEFAULT => {
                    return Err("Give a kit's name or `none` for the project".into())
                }
                name => Some(name.to_string()),
            };
            commands::graphics::project_graphics_brand_set_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                brand.clone(),
            )?;
            match brand {
                Some(kit) => format!("The project's graphics use brand kit {kit} by default."),
                None => "The project's graphics use no brand kit by default.".into(),
            }
        }
    };
    text(applied(
        ctx,
        "brand_pick",
        &before,
        vec![line, "Graphics render again by themselves.".into()],
    )?)
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

    /// The `stamp` a review page asks to save cuts with.
    fn review_stamp(page: &str) -> String {
        page.split("stamp: \"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap()
            .to_string()
    }

    #[test]
    fn candidate_review_shows_only_the_windows() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, key) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);
        // 120 words in the lesson's 4 s: "and like" at 59-60, a filler sound at 100.
        let root = with_reader(ctx.state, |reader| Ok(reader.root().to_path_buf())).unwrap();
        let mut transcript = crate::transcript::store::load_transcript(&root, &key)
            .unwrap()
            .unwrap();
        transcript.words = (0..120u64)
            .map(|i| crate::transcript::TranscriptWord {
                id: format!("w-{i}"),
                text: match i {
                    59 => "and".into(),
                    60 => "like".into(),
                    100 => "um".into(),
                    _ => format!("w{i}"),
                },
                kind: crate::transcript::WordKind::Word,
                source_start_us: i * 32_000,
                source_end_us: i * 32_000 + 30_000,
                confidence: None,
                speaker: None,
            })
            .collect();
        crate::transcript::store::save_transcript(&root, &transcript).unwrap();

        let page = call(&ctx, "fillers_review", json!({})).unwrap();
        assert!(
            page.contains("candidates only: page 0 of 1")
                && page.contains("1 possible filler and 0 possible retakes in 1 window")
                && page.contains("31 words to read instead of 120 for a full read (1 page with")
                && page.contains("26% of it")
                && page.contains("The rules already cut 1 filler sound")
                && page.contains("[60] \"like\" (filler?)\n[45]w45 ")
                && page.contains("[75]w75\n"),
            "{page}"
        );
        assert!(!page.contains("[44]") && !page.contains("[76]"), "{page}");
        assert!(call(&ctx, "fillers_review", json!({ "page": 1 }))
            .unwrap_err()
            .contains("There are 1 pages"));
        // Cuts are saved as from a full read.
        let saved = call(
            &ctx,
            "fillers_review",
            json!({ "cuts": [{ "type": "filler", "from": 60, "to": 60, "why": "filler" }],
                    "stamp": review_stamp(&page) }),
        )
        .unwrap();
        assert!(saved.starts_with("Saved 1 of 1 cuts"), "{saved}");
        let listed = call(&ctx, "fillers", json!({ "apply": false })).unwrap();
        assert!(
            listed.starts_with(&format!("Found 2 fillers and 0 retakes in {key}")),
            "{listed}"
        );
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
        // No review saved yet: the rules only, and how to review in-house.
        assert!(
            dry.contains("(the rules only)") && dry.contains("`fillers_review`"),
            "{dry}"
        );
        assert!(history(&ctx).is_empty(), "a dry run changes nothing");

        // The in-house review: read the page, save cuts against its stamp.
        let page = call(&ctx, "fillers_review", json!({ "review": "thorough" })).unwrap();
        assert!(
            page.contains("page 0 of 1") && page.contains("\"retake\""),
            "{page}"
        );
        let stamp = review_stamp(&page);
        assert!(
            page.contains("(7 kept words") && page.contains("Pages overlap"),
            "{page}"
        );
        // By default only the flags' windows, here all of it: "So" opens a sentence, and the
        // rules cut "um" already.
        let flagged = call(&ctx, "fillers_review", json!({})).unwrap();
        assert!(
            flagged.contains("candidates only: page 0 of 1")
                && flagged.contains("[3] \"So\" (filler?)")
                && flagged.contains("The rules already cut 1 filler sound")
                && flagged.contains("7 words to read instead of 7"),
            "{flagged}"
        );
        assert_eq!(review_stamp(&flagged), stamp);
        let last = 6;
        let saved = call(
            &ctx,
            "fillers_review",
            json!({ "cuts": [
                { "type": "filler", "from": last, "to": last, "why": "trailing" },
                { "type": "joke", "from": 0, "to": 0 },
            ], "stamp": stamp }),
        )
        .unwrap();
        assert!(
            saved.starts_with("Saved 1 of 2 cuts as the in-house review"),
            "{saved}"
        );
        assert!(call(
            &ctx,
            "fillers_review",
            json!({ "cuts": [], "stamp": "nope" })
        )
        .unwrap_err()
        .contains("read the pages again"));
        let reviewed = call(&ctx, "fillers", json!({ "apply": false })).unwrap();
        assert!(
            reviewed.contains("from the in-house review") && !reviewed.contains("`fillers_review`"),
            "{reviewed}"
        );
        assert!(history(&ctx).is_empty(), "saving a review cuts nothing");

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
    fn graphics_are_added_at_a_word_changed_and_skipped_by_cuts() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);
        let added = call(
            &ctx,
            "graphic_add",
            json!({ "template": "step", "atWord": "w-0", "lengthUs": 1_000_000,
                    "variables": { "title": "Install" } }),
        )
        .unwrap();
        assert!(added.contains("(step) at 0:00.00 for 1.0 s"), "{added}");
        assert_eq!(history(&ctx)[0].action.as_deref(), Some("graphic_add"));
        let id = with_reader(ctx.state, |reader| {
            Ok(reader
                .document()
                .assets
                .iter()
                .find(|a| a.is_graphic())
                .unwrap()
                .id
                .clone())
        })
        .unwrap();
        let listed = call(&ctx, "graphics", json!({})).unwrap();
        assert!(
            listed.contains("\"Install\"") && listed.contains(&id),
            "{listed}"
        );

        let bad = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "variables": { "titel": "x" } }),
        )
        .unwrap_err();
        assert!(bad.contains("\"titel\"") && bad.contains("title"), "{bad}");
        let set = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "variables": { "step": 4 }, "lengthUs": 1_500_000 }),
        )
        .unwrap();
        assert!(set.contains("\"step\":4") && set.contains("1.5 s"), "{set}");

        // Brand kits: the graphic's own choice and the project's default.
        let own = call(
            &ctx,
            "brand_pick",
            json!({ "graphic": id, "brand": "none" }),
        )
        .unwrap();
        assert!(own.contains("uses no brand kit"), "{own}");
        let missing = call(&ctx, "brand_pick", json!({ "brand": "No Such Kit" })).unwrap_err();
        assert!(missing.contains("There is no brand kit"), "{missing}");
        let project = call(&ctx, "brand_pick", json!({ "brand": "none" })).unwrap();
        assert!(project.contains("use no brand kit by default"), "{project}");
        let listed = call(&ctx, "graphics", json!({})).unwrap();
        assert!(listed.contains("\"brand\": \"none\""), "{listed}");
        let slot = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "short": "nope", "slot": "camera" }),
        )
        .unwrap_err();
        assert!(slot.contains("No short nope"), "{slot}");
        assert!(call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "slot": "camera" })
        )
        .is_err());

        // The filler "um" (0.35-0.6 s) is under the graphic: the cut is refused, not made.
        let before = length(&ctx);
        let refused = call(&ctx, "fillers", json!({})).unwrap_err();
        assert!(refused.contains("A graphic plays here"), "{refused}");
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

        // Transcript gaps come back unticked for the UI; the action still counts and cuts them.
        let gaps = call(&ctx, "silence", json!({ "mode": "gaps", "apply": false })).unwrap();
        assert!(!gaps.starts_with("Found 0 gaps"), "{gaps}");
        let cut = call(&ctx, "silence", json!({ "mode": "gaps" })).unwrap();
        assert!(cut.contains("Cut them from every track."), "{cut}");
        assert!(length(&ctx) < before, "{cut}");
        call(&ctx, "undo", json!({})).unwrap();
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
        has("Fillers and retakes (the rules, plus a saved in-house review): Found 1 filler and 0 retakes");
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
    fn an_agent_makes_a_short_and_sets_its_scenes() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        let made = call(
            &ctx,
            "short_create",
            json!({ "title": "Next step", "fromWord": "w-0", "toWord": "w-6", "reason": "A tip" }),
        )
        .unwrap();
        // The voice is a file, not a recording: there is no mouse data to pick scenes from.
        assert!(made.contains("Made short short-0 \"Next step\""), "{made}");
        assert!(made.contains("Scenes not picked"), "{made}");
        assert!(call(
            &ctx,
            "short_create",
            json!({ "title": "Too short", "fromWord": "w-0", "toWord": "w-1" })
        )
        .unwrap_err()
        .contains("a short is 3 s to 3 min"));

        let set = call(
            &ctx,
            "short_scenes",
            json!({ "short": "Next step", "op": "set", "scenes": [{ "fromS": 0, "toS": 1.5, "mode": "camera" }] }),
        )
        .unwrap();
        assert!(set.contains("0.0-1.5 s: camera only"), "{set}");
        let listed = call(&ctx, "short_scenes", json!({})).unwrap();
        assert!(
            listed.contains("short-0 \"Next step\":\n  0.0-1.5 s: camera only"),
            "{listed}"
        );
        let cleared = call(
            &ctx,
            "short_scenes",
            json!({ "short": "short-0", "op": "clear" }),
        )
        .unwrap();
        assert!(cleared.contains("Split throughout."), "{cleared}");
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
