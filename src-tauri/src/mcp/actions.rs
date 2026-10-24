//! The app's own buttons, for an agent: transcription, jump cuts, mouth noises, fillers and
//! retakes, layouts, zooms, settings, chapters, shorts and export. Each runs the same
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
            they are transcribed, mouth noises muted, zooms, layouts, captions, chapters, \
            shorts, whether each camera has a face track for framing shorts, and anything \
            running. Call it first and after a cleanup. With no project open, or `readiness`, \
            it also says what this computer is ready for (keys, models, renderer).",
        schema: schema::<StatusArgs>,
        read_only: true,
        run: status,
    },
    ToolDef {
        name: "scan",
        description: "An overview before an edit, changing nothing: microphones, cameras, mouse \
            data, the transcribers and AI key ready, and a first estimate of each pass for a \
            preset (mouth noises, silence, gaps, fillers and retakes, layouts, zooms). \
            Each pass changes what the next finds, so before running one, ask with its own dry \
            run (`apply: false`), not these numbers.",
        schema: schema::<ScanArgs>,
        read_only: true,
        run: scan,
    },
    ToolDef {
        name: "open",
        description: "Opens an AeroEdits project folder in the editor window, replacing the one \
            open. A .aero recording folder is not a project: a take recorded for a short goes \
            into the open project with `media_import` (then `short_from_take`); a recording to \
            edit as the video gets a new project with `create` and `recording`.",
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
        description: "Undoes the last change to the edit, whoever made it. `run: true` undoes \
            everything you changed since your last `preset` call (your run) as one step, \
            while the user has changed nothing since.",
        schema: schema::<UndoArgs>,
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
        name: "checkpoint",
        description: "Named moments to go back to: the edit, the graphics' files and the \
            caption marks. `op`: list; save (with `name`); restore (with `id`; one undo step, \
            and what is there now is saved first). The app saves one before each of your runs.",
        schema: schema::<CheckpointArgs>,
        read_only: false,
        run: checkpoint,
    },
    ToolDef {
        name: "speed",
        description: "Plays clips faster or slower, 0.25x to 8x, keeping their media and the sound's pitch: a clip gets shorter or longer (with what it is linked to). Give `clips` (ids from timeline_get) or a timeline range `fromUs`-`toUs` (every unlocked track is split at its ends), and `speed` (1 is normal). Magnetic closes up or pushes along; `ripple` overrides it (false leaves a gap or stops at the next clip). `short` works on a short's own timeline. Without `speed` it lists what plays at other speeds. Only when the user asks: never speed up waiting by yourself.",
        schema: schema::<SpeedArgs>,
        read_only: false,
        run: speed,
    },
    ToolDef {
        name: "transcribe",
        description: "Transcribes a sound. By default (`provider: auto`) online with Scribe, \
            which detects the language (English, Greek, ...): ElevenLabs first, OpenRouter when \
            ElevenLabs has no key or credits. `parakeet` is local and free, English only. The \
            answer says the language heard; the later passes follow it. Default sound: the \
            speech on the timeline. Skips a sound already transcribed unless `force`. Takes a \
            while; reports progress.",
        schema: schema::<TranscribeArgs>,
        read_only: false,
        run: transcribe,
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
            transcript, one undo step: the app's rules plus an AI review. By default (`review: \
            auto`) the app runs its OpenRouter/OpenAI review now when the user has a key set, \
            else its built-in local model when downloaded, and otherwise uses the review you \
            saved with `fillers_review` (in-house: you read the transcript). `review: \
            in-house`, `openrouter` or `local` forces one. `apply: false` only lists. `auto` \
            reuses a saved review.",
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
        name: "layouts",
        description: super::layouts::DESCRIPTION,
        schema: schema::<super::layouts::LayoutsArgs>,
        read_only: false,
        run: super::layouts::layouts,
    },
    ToolDef {
        name: "faces",
        description: "Webcam face detection: finds the face in every camera stream (on this computer, YuNet) so shorts frame each camera clip on it. Starts the cameras that have no face track yet (and retries failed ones), waits until they are done unless `wait: false`, and reports each camera: ready (and how much of it shows a face), detecting, failed or missing. `redo: true` finds faces again even where a track exists.",
        schema: schema::<FacesArgs>,
        read_only: false,
        run: faces,
    },
    ToolDef {
        name: "zooms",
        description: "Zooms. Automatic ones from the recording's clicks: `op` `add` (default) \
            adds the suggested zooms, `reload` finds them again with the zoom settings \
            (dismissed ones too; manual ones stay), `dismiss` drops the suggestions; `settings` \
            changes the zoom settings first; `apply: false` only reports what would be added, \
            with `settings` tried but not saved. Single \
            zooms: `list` (ids, times, scale, centre), `create` (`startUs`, `endUs` on the edited \
            timeline, `scale`, `centerX`/`centerY` 0-1), `update` (`id` with `scale`, centre or \
            `fixed`), `remove` (`ids`). Each change is one undo step.",
        schema: schema::<ZoomArgs>,
        read_only: false,
        run: zooms,
    },
    ToolDef {
        name: "settings",
        description: "Reads or changes this project's settings (changes from its preset; \
            `library` `kind: preset`). `area`: captions (enabled, position, size, words), layout \
            (canvas, screen size and crop, cursor, `faceFraming`; where things go is `layouts`; \
            how it looks is the style's), audio, zoom, cleanup (silence, gap and mouth-noise \
            levels the passes start from), export (resolution, fps, quality), transcription, \
            ai. Without `set` it shows the current values; `set` is a JSON object of \
            just the fields to change (nested objects merge). \
            Telemetry polish: layout `webcamAutoHide` (on by default; the bubble fades while work \
            is under it), `windowCrop` (on by default) with `windowCropPaddingPct` and \
            `windowCropOff` (recording stretches `{media, sourceStartUs, sourceEndUs}` shown \
            uncropped). Audio `musicUnderSpeechDb` (~18 dB), \
            `soundEffects` (off by default; `clicks`, `whooshes`, `muted`, \
            `volumeDb`). API keys are not reachable here.",
        schema: schema::<SettingsArgs>,
        read_only: false,
        run: settings,
    },
    ToolDef {
        name: "chapters",
        description: "YouTube chapters. With `set` (a list of `title` + `atWord` or `atUs`) \
            the chapters you wrote replace the project's; without it the app's AI pass makes \
            them (the user's OpenAI/OpenRouter key). Run it last, after every cut and right \
            before export: cuts made after it can leave chapters titled for talk that is gone. \
            YouTube needs at least 3, each 10 s or longer, the first at 0:00. `status` says \
            when they are out of date.",
        schema: schema::<ChaptersArgs>,
        read_only: false,
        run: chapters,
    },
    ToolDef {
        name: "media_import",
        description: "Imports files (videos, sounds, pictures, or .aero recording folders) into \
            the open project's Media, as the Media panel's Import does. Use it for a take the \
            user recorded for a short, whatever its form: a camera video (often with its own \
            microphone) and, if they used one, a separate microphone file, or a .aero \
            recording (its camera and microphone together). Never `open` a take. A short from a \
            take is an edit of its own: read `preset` with `name: shorts` first, even when \
            another preset was read already. Answers each new media id and its streams. `roles` \
            (`media`, `stream`, `role`) sets what streams stand for, after importing or on media \
            already in the project; music and sound effects get `music` / `effects` and their \
            `licences` (music must need no credit).",
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
            `short_footage`. Its steps are `preset` with `name: shorts`. Transcribe the voice \
            next (`transcribe` with its stream).",
        schema: schema::<ShortFromTakeArgs>,
        read_only: false,
        run: short_from_take,
    },
    ToolDef {
        name: "short_footage",
        description: "What plays under a short made from a take, while the take talks. \
            `op: add` proposes `items` as one batch the user accepts in the Agent panel (put \
            all your picks in one call, with a `reason`): each under the take's words \
            `fromWord`..`toWord` (ids from `transcript_get` with the take's voice as `stream`) \
            or seconds `atS`..`toS` on the short, showing the video as edited (only what it \
            keeps, its cuts included) from its word `sourceWord` (an id from the video's \
            transcript) or from `videoS` seconds on its timeline; the footage's game or \
            desktop sound plays ducked under the voice (`sound: false` mutes it); or B-roll \
            from an imported file (`media`, from `mediaS` seconds in); or a graphic over it \
            all (`template`, `library`, or `graphic`: a copy of one of the video's, made for \
            9:16), its own length without an end. Where nothing plays, the camera fills the \
            frame. `op: remove` takes footage off `fromS`..`toS` (all without them); \
            `op: sound` sets the footage and B-roll sound's `volumeDb` and `duckDb` (one \
            track); `op: list` (default) lists it.",
        schema: schema::<ShortFootageArgs>,
        read_only: false,
        run: short_footage,
    },
    ToolDef {
        name: "check",
        description: "The check before export, the one the export dialog shows: black frames, \
            blips and words cut partway, zooms that flash by, captions \
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
            false`, which returns a job id for export_status; `cancel` with that id stops it. \
            `file` saves subtitles, the transcript or the sound alone instead.",
        schema: schema::<ExportArgs>,
        // Writes the video file (over `destination` if it exists).
        read_only: false,
        run: export,
    },
    ToolDef {
        name: "export_status",
        description: "How the export started with `export` is going. `waitS` waits for it to \
            finish (up to 300 s) instead of checking again and again.",
        schema: schema::<ExportStatusArgs>,
        read_only: true,
        run: export_status,
    },
    ToolDef {
        name: "graphics",
        description: "Motion graphics: whether the renderer is installed, the templates (with \
            their variables, types and defaults), the user's graphics library (templates saved \
            from earlier projects, each with when to use it: look there before writing a new \
            graphic) and the graphics in the project (id, where they play, their variables, \
            style, how their render is going). Graphics play full screen over everything \
            (screen, camera, captions), and cuts never go through one, so add them as one of \
            the last passes, after the cuts are checked and before chapters.",
        schema: schema::<NoArgs>,
        read_only: true,
        run: graphics_list,
    },
    ToolDef {
        name: "graphic_add",
        description: "Adds graphics, all `items` in one call and one undo step: each from a \
            template or a copy of a library item (`library`), at `atUs` on the edited timeline \
            or at `atWord` (an id from transcript_get), on a Graphics track over the video, for \
            `lengthUs` (default its own). They render in the background; `graphic_render` \
            waits. `blank` starts an empty composition for graphic_write_file. Graphics under a \
            take go in `short_footage` items.",
        schema: schema::<GraphicAddArgs>,
        read_only: false,
        run: graphic_add,
    },
    ToolDef {
        name: "graphic_set",
        description: "Changes a graphic: its variables (checked against what it declares; \
            null puts one back to its default), its name, its length on the timeline, its \
            style (`project` or a style from `library`), `look` tokens of its own, and with \
            `short` how it shows there (`slot`: `full` frame, `screen` split beside the face, \
            `overFace` over the face filling the frame, `camera`, or `overScreen` under the \
            captions). It renders again by itself. `moveToUs` or \
            `moveToWord` moves it; `remove: true` takes it out of the project with its clips \
            (one undo step).",
        schema: schema::<GraphicSetArgs>,
        read_only: false,
        run: graphic_set,
    },
    ToolDef {
        name: "graphic_write_file",
        description: "Writes a file into a graphic's folder (index.html, or an image or font it \
            uses), for a composition of your own; answers with HyperFrames' lint of it. \
            `contents` is the whole file; `edits` change part of the one in the folder. The \
            folder (in `graphics`) holds the only copy: keep none in your working folder or \
            anywhere else. Rules: a \
            HyperFrames composition, full-screen over the video, laid out with vw/vh/vmin so it \
            fits 16:9 and 9:16; a transparent page with a background of your own; a graphic \
            that fills the frame never eases in or out: show it whole from the first frame to \
            the last, and fade or slide it in and out only when the root's data-entrance is \
            \"ease\" (the app sets that only for an overlay over the full-screen camera; mark a \
            lower third or callout with data-overlay on the root); motion with GSAP 3.15 \
            (`<script src=\"gsap.min.js\">`, plugins by file name; a CDN link to GSAP is made \
            local) as one paused timeline in window.__timelines[\"graphic\"], or Web Animations \
            with data-no-timeline on the root; no other network (no CDN scripts or web fonts); \
            editable text as data-composition-variables.",
        schema: schema::<GraphicWriteArgs>,
        read_only: false,
        run: graphic_write_file,
    },
    ToolDef {
        name: "graphic_render",
        description: "Renders the graphics (or one) for the video's canvas now, or with `short` \
            a short's for its 9:16 frame, waiting, and reports each one's lint, render and \
            whether it cuts in and out or eases (only an overlay over the full-screen camera \
            eases). Look at the result with frame_render (or storyboard) inside the graphic.",
        schema: schema::<GraphicRenderArgs>,
        // Writes renders into the project's graphics cache.
        read_only: false,
        run: graphic_render,
    },
    ToolDef {
        name: "graphic_templatize",
        description: "Turns a graphic you wrote (one the user likes) toward a general template \
            for the user's library: returns its source, what the app already sees in it (network \
            references, missing brand colours, fixed text) and the rules to rewrite it by (text \
            as variables, style colours, timing from data-duration, viewport layout with a \
            portrait block, a cut unless data-entrance is \"ease\", no network). Rewrite its \
            files, then graphic_check.",
        schema: schema::<GraphicOnlyArgs>,
        read_only: true,
        run: graphic_templatize,
    },
    ToolDef {
        name: "graphic_check",
        description: "The library's gate for a graphic, run by the app (no model): HyperFrames' \
            check (lint, runtime, overflow, contrast) at 16:9 and 9:16 with its text and with \
            long text, stills at 3 s and 8 s (see-through at both ends, so its timing follows \
            the clip), each text variable changing the picture, no network, style colours. \
            Answers pass or fail per check with why, and a contact sheet (both shapes, its text \
            and long text). Takes 30-90 s; an unchanged graphic answers at once.",
        schema: schema::<GraphicOnlyArgs>,
        // Writes its report and contact sheet into the gate cache.
        read_only: false,
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
        name: "library_remove",
        description: "Removes an item (id or name, from `graphics`) from the user's graphics \
            library, for every project. Graphics already placed from it stay. Only at the \
            user's word.",
        schema: schema::<LibraryRemoveArgs>,
        read_only: false,
        run: library_remove,
    },
    ToolDef {
        name: "transcript_fix",
        description: "Corrects words the transcriber got wrong: `fixes` is a list of `word` (id \
            from transcript_get) and its right `text`. The fix shows in captions, in the video \
            and every short; times and cuts don't change. It is not an undo step: the answer \
            lists the old text so you can put it back.",
        schema: schema::<TranscriptFixArgs>,
        read_only: false,
        run: transcript_fix,
    },
    ToolDef {
        name: "captions",
        description: "The caption lines as they show, numbered, with each line's emphasis word \
            in *stars*; `short` for a short's or clip's. `emphasis`: per `line`, the `word` (as it reads \
            there) drawn in the accent colour, the one word that makes a viewer keep watching (a \
            number, a name, the twist, the verdict), never a small word; most lines get one, a \
            flat line none. A line has one: a new pick replaces the old, `word` left out clears \
            it. `split` starts a new line at a word, `join` puts a line on the one before, \
            `hide` hides a word (or a whole line) from the captions. On the video these are \
            kept on the transcript (not undo steps); in a `short` they are its own (undo \
            steps), and caption text stays shared. Where they sit and their size is `settings` `area: captions`; how they look \
            (pop/karaoke/static, colours, box) is the project's style (`library`).",
        schema: schema::<CaptionsArgs>,
        read_only: false,
        run: captions,
    },
];

// ---------------------------------------------------------------------------------------
// Shared pieces.

#[derive(Deserialize, JsonSchema)]
pub struct NoArgs {}

/// The open project as an action starts.
pub(super) struct Open {
    pub(super) handle: String,
    pub(super) revision: u64,
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

/// The project open now. A call bound to a project (`project`) runs only on that one; the
/// commands it then runs check the same handle again under the command lock.
pub(super) fn opened(ctx: &Ctx) -> Result<Open, String> {
    let open = current(ctx)?;
    if ctx.project.as_ref().is_some_and(|p| *p != open.handle) {
        return Err(super::tools::OTHER_PROJECT_MESSAGE.into());
    }
    Ok(open)
}

/// The answer for an action that may have changed the edit: `lines`, then the length and the
/// revision when it did, and the action listed in the Agent panel's history.
pub(super) fn applied(
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

pub(super) enum Want {
    /// Speech on the timeline, else any sound there.
    Speech,
    /// A transcribed sound, speech on the timeline first.
    Transcribed,
}

pub(super) fn pick_sound(ctx: &Ctx, stream: Option<String>, want: Want) -> Result<String, String> {
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
pub(super) fn patched<T: Serialize + DeserializeOwned>(
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

pub(super) fn plural(n: usize, one: &str) -> String {
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

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatusArgs {
    /// Also say what this computer is ready for: app version, keys set, models and runtimes
    /// installed, and what is unavailable and why. Always given when no project is open.
    #[serde(default)]
    pub readiness: bool,
}

fn status(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: StatusArgs = params(args)?;
    let project = match status_text(ctx) {
        Ok(lines) => lines,
        Err(error) if error == super::tools::NO_PROJECT_MESSAGE => {
            return text(format!(
                "No project is open: `open` or `create` one.\n{}",
                readiness_text()
            ));
        }
        Err(error) => return Err(error),
    };
    text(if args.readiness {
        format!("{project}\n{}", readiness_text())
    } else {
        project
    })
}

/// What this computer is ready for, a line a part: what's missing says why and where to fix it.
fn readiness_text() -> String {
    let mut lines = vec![format!(
        "Readiness: AeroEdits {}{}; agents reach it through this same program ({}).",
        env!("CARGO_PKG_VERSION"),
        if cfg!(debug_assertions) {
            " (development build)"
        } else {
            ""
        },
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "path unknown".into()),
    )];
    let view = tc::transcript_settings_get_impl();
    let key = |source: &Option<String>| {
        if source.is_some() {
            "key set"
        } else {
            "no key"
        }
    };
    lines.push(format!(
        "Transcription online: ElevenLabs {}, OpenRouter {}{}.",
        key(&view.elevenlabs_key_source),
        key(&view.openrouter_key_source),
        match not_ready(&view, crate::transcript::ProviderKind::ElevenLabs) {
            Some(why) => format!(" ({why}; Greek needs one)"),
            None => String::new(),
        }
    ));
    lines.push(format!(
        "Transcription local (Parakeet, English): {}.",
        match not_ready(&view, crate::transcript::ProviderKind::Parakeet) {
            None => "ready".to_string(),
            Some(why) => format!("not ready: {why}"),
        }
    ));
    if crate::cuda::wanted() {
        lines.push(format!(
            "GPU pack (CUDA, speeds up Parakeet): {}.",
            if crate::cuda::present(&crate::cuda::default_dir()) {
                "installed".to_string()
            } else if !crate::cuda::nvidia_driver() {
                "not needed: no NVIDIA graphics card driver".to_string()
            } else {
                format!(
                    "not installed ({} MB download, Transcription settings in the app); \
                     Parakeet runs on the CPU meanwhile",
                    crate::cuda::download_bytes() / 1_000_000
                )
            }
        ));
    }
    let ai = tc::ai_settings_get_impl();
    let (provider, ai_key) = match ai.settings.provider {
        crate::ai::AiProvider::OpenAi => ("OpenAI", &ai.openai_key_source),
        crate::ai::AiProvider::OpenRouter => ("OpenRouter", &ai.openrouter_key_source),
    };
    lines.push(format!(
        "AI key ({provider}): {}.",
        if ai_key.is_some() {
            "set, so chapters, finding clips and the filler review can run"
        } else {
            "none, so no chapters or found clips from the app; you review fillers yourself"
        }
    ));
    lines.push(format!(
        "Local filler review model: {}.",
        if !crate::ai::local::compiled_in() {
            "not part of this build"
        } else if crate::ai::local::ready() {
            "ready"
        } else {
            "not downloaded (AI settings in the app)"
        }
    ));
    lines.push(format!(
        "Face model (framing shorts): {}.",
        if !crate::face::yunet::compiled_in() {
            "not part of this build"
        } else if crate::face::yunet::model_present(&crate::face::yunet::default_model_dir()) {
            "ready"
        } else {
            "downloads by itself the first time faces are tracked"
        }
    ));
    let graphics = crate::graphics::runtime::status();
    lines.push(format!(
        "Graphics renderer (HyperFrames {}): {}.",
        graphics.hyperframes,
        if graphics.installed {
            "installed".to_string()
        } else if graphics.outdated {
            "needs an update: `graphics_install`".to_string()
        } else {
            format!(
                "not installed: `graphics_install` (about {} MB)",
                graphics.download_mb
            )
        }
    ));
    lines.push(format!(
        "FFmpeg: {}.",
        if crate::media::ffmpeg::available() {
            "found"
        } else {
            "missing, so nothing can be imported or exported (reinstall the app)"
        }
    ));
    let skills: Vec<String> = super::skill::states()
        .into_iter()
        .filter(|s| s.available)
        .map(|s| {
            format!(
                "{} {}",
                match s.home {
                    super::skill::SkillHome::ClaudeCode => "Claude Code",
                    super::skill::SkillHome::Codex => "Codex",
                },
                match (s.installed, s.current) {
                    (true, true) => "installed",
                    (true, false) => "installed but from another version: reinstall it",
                    _ => "not installed",
                }
            )
        })
        .collect();
    if !skills.is_empty() {
        lines.push(format!(
            "Agent skill (Agent settings in the app): {}.",
            skills.join(", ")
        ));
    }
    lines.join("\n")
}

fn status_text(ctx: &Ctx) -> Result<String, String> {
    let open = opened(ctx)?;
    let mut lines = with_reader(ctx.state, |reader| {
        let document = reader.document();
        let summary = &reader.summary;
        let recorded: u64 = reader.recordings().map(|a| a.duration_us).sum();
        let (width, height) = document.layout.preview_dimensions()?;
        let mut lines = vec![format!(
            "Project \"{}\", revision {}, {} long{}, canvas {width}x{height}. Its `project` id \
             (pass it on changes): {}.",
            summary.name,
            document.revision,
            clock(document.duration_us()),
            if recorded > 0 {
                format!(" ({} recorded)", clock(recorded))
            } else {
                String::new()
            },
            summary.project_handle,
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
        lines.push(format!(
            "Zooms: {} in the edit. Layout: {}, {}. Captions: {}. Chapters: {}. Shorts (from a take): {}. Clips (cut from the video): {}.",
            document.zooms.len(),
            super::layouts::describe(&document.layouts.base),
            plural(document.layouts.sections.len(), "section"),
            if document.captions.enabled {
                "on"
            } else {
                "off"
            },
            document.chapters.len(),
            document.shorts.iter().filter(|s| s.take.is_some()).count(),
            document.shorts.iter().filter(|s| s.take.is_none()).count(),
        ));
        let changed = crate::preset::changed_count(document);
        lines.push(format!(
            "Preset: {}{} (its shorts: {}, its clips: {}). Style: {}.",
            crate::preset::of(document).name,
            if changed == 0 {
                String::new()
            } else {
                format!(", {} changed here", plural(changed, "value"))
            },
            crate::preset::store::resolve(&document.shorts_preset).name,
            crate::preset::store::resolve(&document.clips_preset).name,
            crate::style::of(document).name,
        ));
        let checkpoints = reader.history().steps().checkpoints();
        if let Some(newest) = checkpoints.iter().rev().find(|c| !c.auto) {
            lines.push(format!(
                "Checkpoints: {}; newest named \"{}\" ({}, {}).",
                checkpoints.len(),
                newest.name,
                newest.id,
                newest.at
            ));
        }
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
/// The edited length each project's chapters were last set or confirmed at, by project handle.
/// Setting the same chapters again changes no revision, so the history can't tell it happened.
static CHAPTERS_AT_LENGTH: std::sync::Mutex<Vec<(String, u64)>> = std::sync::Mutex::new(Vec::new());

fn chapters_made(open: &Open) {
    if let Ok(mut made) = CHAPTERS_AT_LENGTH.lock() {
        made.retain(|(handle, _)| handle != &open.handle);
        made.push((open.handle.clone(), open.duration));
    }
}

fn chapters_stale(ctx: &Ctx) -> Option<String> {
    let open = current(ctx).ok()?;
    let made_at = CHAPTERS_AT_LENGTH.lock().ok().and_then(|made| {
        made.iter()
            .find(|(h, _)| h == &open.handle)
            .map(|(_, d)| *d)
    });
    if let Some(length) = made_at {
        // Only cuts move what chapters point at; zooms, layouts or graphics don't.
        return (length != open.duration).then(|| {
            format!(
                "Chapters were set when the edit was {} long and it is {} now: run `chapters` \
                 again, as the last step before export.",
                clock(length),
                clock(open.duration)
            )
        });
    }
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

/// What the recording has and what each pass would do, for the questions a method asks.
fn scan(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ScanArgs = params(args)?;
    // A level asked for, else the project's own levels.
    let preset = args.preset;
    let (mut lines, transcribed, cameras, mouse) =
        with_reader(ctx.state, |reader| {
            let document = reader.document();
            let all = sounds(reader);
            let mut lines = vec![format!(
                "Scan of \"{}\", {} long, measured with {}; nothing was changed.",
                reader.summary.name,
                clock(document.duration_us()),
                match preset {
                    Some(level) => format!("the {} level", one_line(&level).trim_matches('"')),
                    None => "the project's cleanup settings".into(),
                },
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
                    "Cameras: {} ({}), base layout {}.",
                    cameras.len(),
                    cameras.join(", "),
                    super::layouts::describe(&document.layouts.base)
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
                "Mouse data: none, so no zooms or picked layouts from it.".to_string()
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
    let key = |source: &Option<String>| {
        if source.is_some() {
            "key set"
        } else {
            "no key"
        }
    };
    lines.push(format!(
        "Transcribers: online by default (`provider: auto`, Scribe detects the language): \
         ElevenLabs {}, then OpenRouter {}. Local: parakeet {} (free, English only, when the \
         user asks).",
        key(&view.elevenlabs_key_source),
        key(&view.openrouter_key_source),
        match not_ready(&view, crate::transcript::ProviderKind::Parakeet) {
            None => "ready".to_string(),
            Some(reason) => format!("not ready: {reason}"),
        },
    ));
    let ai = tc::ai_settings_get_impl();
    let (provider, key) = match ai.settings.provider {
        crate::ai::AiProvider::OpenAi => ("OpenAI", &ai.openai_key_source),
        crate::ai::AiProvider::OpenRouter => ("OpenRouter", &ai.openrouter_key_source),
    };
    lines.push(if key.is_some() {
        format!("AI key: {provider} key set (chapters and finding clips can run). `fillers` runs the rules and the {provider} review together.")
    } else {
        format!("AI key: none for {provider}, so no chapters or found clips. Fillers and retakes are reviewed in-house by you (`fillers_review`), then `fillers` cuts them with the rules' finds.")
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
                sensitivity: preset.map(Preset::noise_sensitivity),
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
                preset,
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
                    preset,
                    apply: dry,
                    ..Default::default()
                },
            ),
        );
        pass(
            &mut lines,
            "Fillers and retakes (the rules, plus any saved review)",
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
            "Layouts",
            super::layouts::run_pick(ctx, None, None, dry != Some(false)),
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
    /// A project folder (a .aero recording is made into one with `create`).
    pub path: String,
}

fn open(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: OpenArgs = params(args)?;
    commands::open_project_impl(ctx.state, args.path.clone()).map_err(|e| {
        if e.starts_with("That folder is a recording") {
            "That folder is a recording, not a project, so it can't be opened. If it is a take \
             for a short, import it into the open project with `media_import` and make the \
             short with `short_from_take`. To edit it as the video, make a new project from it \
             with `create` and `recording`."
                .to_string()
        } else {
            e
        }
    })?;
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

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointArgs {
    /// list, save or restore.
    pub op: String,
    /// For save: what to call it, such as "clean cut".
    #[serde(default)]
    pub name: Option<String>,
    /// For restore: the checkpoint's id from list (`c-2`).
    #[serde(default)]
    pub id: Option<String>,
}

fn checkpoint(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: CheckpointArgs = params(args)?;
    match args.op.trim() {
        "list" => with_reader(ctx.state, |reader| {
            let checkpoints = reader.history().steps().checkpoints();
            if checkpoints.is_empty() {
                return text("No checkpoints yet.".to_string());
            }
            let lines: Vec<String> = checkpoints
                .iter()
                .rev()
                .map(|c| {
                    format!(
                        "{}: \"{}\", {}{}",
                        c.id,
                        c.name,
                        c.at,
                        if c.auto { " (saved by the app)" } else { "" }
                    )
                })
                .collect();
            text(lines.join("\n"))
        }),
        "save" => {
            let name = args
                .name
                .filter(|n| !n.trim().is_empty())
                .ok_or("save needs `name`: what to call this moment, such as \"clean cut\"")?;
            let saved = {
                let _guard = ctx.state.command_lock.lock();
                let mut opened = ctx.state.opened_project.lock();
                let reader = opened.as_mut().ok_or(super::tools::NO_PROJECT_MESSAGE)?;
                reader.save_checkpoint(&name)?
            };
            text(format!(
                "Saved checkpoint {} \"{}\": the edit, {} and the caption marks. \
                 `checkpoint` op restore with this id goes back to it.",
                saved.id,
                saved.name,
                plural(saved.graphics.len(), "graphic")
            ))
        }
        "restore" => {
            let id = args.id.ok_or("restore needs `id` (from op list)")?;
            let before = opened(ctx)?;
            let (what, view) = crate::commands::proposals::with_proposals(
                ctx.state,
                ctx.project.as_deref(),
                |reader, proposals| {
                    crate::proposals::restore_checkpoint(reader, proposals, before.revision, &id)
                        .map(|(_, what)| what)
                },
            )?;
            ctx.host.transcript_changed();
            let mut answer = undone(ctx, &before, &what)?;
            if let Some(warning) = view.warning {
                answer.push(Output::Text(warning));
            }
            Ok(answer)
        }
        other => Err(format!("No op {other}: `op` is list, save or restore")),
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UndoArgs {
    /// Undo your whole run as one step instead of the last change.
    #[serde(default)]
    pub run: bool,
}

fn undo(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: UndoArgs = params(args)?;
    let before = opened(ctx)?;
    if args.run {
        let (what, view) = crate::commands::proposals::with_proposals(
            ctx.state,
            ctx.project.as_deref(),
            |reader, proposals| {
                crate::proposals::undo_run(reader, proposals, before.revision, Some(&ctx.agent))
                    .map(|(_, what)| what)
            },
        )?;
        let mut answer = undone(ctx, &before, &format!("{what}."))?;
        if let Some(warning) = view.warning {
            answer.push(Output::Text(warning));
        }
        return Ok(answer);
    }
    let label = with_reader(ctx.state, |r| {
        Ok(r.history().undo_label().map(str::to_string))
    })?;
    commands::project_undo_impl(ctx.state, before.handle.clone(), before.revision, None)?;
    let what = match label {
        Some(label) => format!("Undid \"{label}\"."),
        None => "Undid the last change.".into(),
    };
    undone(ctx, &before, &what)
}

fn redo(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    params::<NoArgs>(args)?;
    let before = opened(ctx)?;
    let label = with_reader(ctx.state, |r| {
        Ok(r.history().redo_label().map(str::to_string))
    })?;
    commands::project_redo_impl(ctx.state, before.handle.clone(), before.revision, None)?;
    let what = match label {
        Some(label) => format!("Redid \"{label}\"."),
        None => "Redid the last undone change.".into(),
    };
    undone(ctx, &before, &what)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpeedArgs {
    /// How fast to play, from 0.25 to 8 (2 is twice as fast, 1 normal). Leave it out to list
    /// what plays at other speeds.
    #[serde(default)]
    pub speed: Option<f64>,
    /// Clip ids; the clips linked to them change too.
    #[serde(default)]
    pub clips: Vec<String>,
    /// A range of the timeline instead of clips (edited microseconds).
    #[serde(default)]
    pub from_us: Option<u64>,
    #[serde(default)]
    pub to_us: Option<u64>,
    /// Overrides magnetic for this change.
    #[serde(default)]
    pub ripple: Option<bool>,
    /// A short's or clip's id, for its own timeline instead of the video's.
    #[serde(default)]
    pub short: Option<String>,
}

fn speed(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: SpeedArgs = params(args)?;
    let before = opened(ctx)?;
    let report = |ctx: &Ctx| -> Result<Vec<String>, String> {
        with_reader(ctx.state, |reader| {
            let document = reader.document();
            let sequence = match &args.short {
                None => &document.sequence,
                Some(id) => {
                    let short = document
                        .shorts
                        .iter()
                        .find(|s| &s.id == id)
                        .ok_or_else(|| format!("No short {id}"))?;
                    &short
                        .edit
                        .as_ref()
                        .ok_or_else(|| format!("{id} has no timeline of its own yet"))?
                        .sequence
                }
            };
            let lines: Vec<String> = sequence
                .tracks
                .iter()
                .flat_map(|t| t.clips.iter().map(move |c| (t, c)))
                .filter(|(_, c)| c.speed != 1.0)
                .map(|(t, c)| {
                    format!(
                        "{} on {}: {} at {}-{}, playing {}-{} of {}.",
                        c.id,
                        sequence.number(&t.id).unwrap_or_default(),
                        super::views::speed_label(c.speed),
                        clock(c.start_us),
                        clock(c.end_us()),
                        clock(c.in_us),
                        clock(c.out_us()),
                        document
                            .asset(&c.asset)
                            .map_or(c.asset.as_str(), |a| a.name.as_str())
                    )
                })
                .collect();
            Ok(lines)
        })
    };
    let Some(speed) = args.speed else {
        let lines = report(ctx)?;
        return text(if lines.is_empty() {
            "Every clip plays at normal speed.".to_string()
        } else {
            lines.join(
                "
",
            )
        });
    };
    let range = match (args.from_us, args.to_us) {
        (Some(start_us), Some(end_us)) => Some(crate::zoom::EditedRange { start_us, end_us }),
        (None, None) => None,
        _ => return Err("Give `fromUs` and `toUs` together".into()),
    };
    if range.is_none() && args.clips.is_empty() {
        return Err("Give `clips` or `fromUs` and `toUs`".into());
    }
    commands::project_sequence_edit_impl(
        ctx.state,
        before.handle.clone(),
        before.revision,
        crate::sequence::edit::SequenceEdit::SetSpeed {
            clip_ids: args.clips.clone(),
            speed,
            range,
            ripple: args.ripple,
        },
        args.short.clone(),
    )?;
    let mut lines = report(ctx)?;
    if lines.is_empty() {
        lines.push("Every clip plays at normal speed now.".into());
    }
    text(applied(ctx, "speed", &before, lines)?)
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
    /// auto (default): online, ElevenLabs then OpenRouter. parakeet: on this computer, free,
    /// English only. elevenlabs or openrouter: that online route first. The app's saved
    /// settings stay as they are.
    #[serde(default)]
    pub provider: Transcriber,
    /// auto (default online: Scribe detects it), en or el. Parakeet is always English.
    #[serde(default)]
    pub language: Option<crate::transcript::SpokenLanguage>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Transcriber {
    #[default]
    Auto,
    Parakeet,
    Elevenlabs,
    Openrouter,
}

impl Transcriber {
    /// Parakeet, or the online route tried first.
    fn kind(self) -> crate::transcript::ProviderKind {
        match self {
            Transcriber::Parakeet => crate::transcript::ProviderKind::Parakeet,
            Transcriber::Auto | Transcriber::Elevenlabs => {
                crate::transcript::ProviderKind::ElevenLabs
            }
            Transcriber::Openrouter => crate::transcript::ProviderKind::OpenRouter,
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
        // Online, either route will do: the other is tried when the first has no key.
        ElevenLabs | OpenRouter
            if view.elevenlabs_key_source.is_none() && view.openrouter_key_source.is_none() =>
        {
            Some(
                "no ElevenLabs or OpenRouter key is set (Transcription settings in the app)".into(),
            )
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
    let provider = args.provider.kind();
    if let Some(reason) = not_ready(&view, provider) {
        return Err(format!("Can't transcribe with {provider:?}: {reason}."));
    }
    use crate::transcript::SpokenLanguage;
    let language = match provider {
        crate::transcript::ProviderKind::Parakeet => SpokenLanguage::English,
        _ => args.language.unwrap_or(SpokenLanguage::Auto),
    };
    let result = ctx.transcript_job(|| {
        tc::transcript_run_impl(
            ctx.state,
            ctx.host.transcripts(),
            open.handle,
            key.clone(),
            tc::TranscribeWith {
                provider: Some(provider),
                language: Some(language),
            },
            &mut |p| ctx.progress(p.fraction, &p.message),
        )
    })?;
    let provider = result.view.provider;
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
    /// Automatic mode: apply only if it looks right, else say why (no dry run first).
    #[serde(default)]
    pub guard: bool,
    /// A short or clip (id or title): its own timeline and muted noises. Default: the take short the
    /// sound is the voice of, else the video.
    #[serde(default)]
    pub short: Option<String>,
}

fn mouth_noises(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    text(run_noises(ctx, params(args)?)?)
}

/// Automatic mode's check before a pass cuts `cut_us` from the video (`guard: true`): why it
/// waits for the user instead, if it should. A pass cutting more than a quarter of the video,
/// or the cleanup coming to more than a third of what was recorded for it (pauses aside), looks
/// wrong. A take (`key` the voice of a take short) is cut much tighter: it is not guarded.
fn cut_looks_wrong(
    ctx: &Ctx,
    before: &Open,
    cut_us: u64,
    key: Option<&str>,
) -> Result<Option<String>, String> {
    let (take, recorded) = with_reader(ctx.state, |reader| {
        let document = reader.document();
        let take = key.is_some_and(|key| {
            document
                .shorts
                .iter()
                .any(|s| s.take.as_ref().is_some_and(|t| t.voice == key))
        });
        let on_video: std::collections::HashSet<&str> = document
            .sequence
            .clips()
            .map(|(_, c)| c.asset.as_str())
            .collect();
        let recorded: u64 = reader
            .recordings()
            .filter(|a| on_video.contains(a.id.as_str()))
            .map(|a| {
                let paused: u64 = a
                    .pauses
                    .iter()
                    .map(|p| p.end_us.saturating_sub(p.start_us))
                    .sum();
                a.duration_us.saturating_sub(paused)
            })
            .sum();
        Ok((take, recorded))
    })?;
    if take {
        return Ok(None);
    }
    let after = before.duration.saturating_sub(cut_us);
    Ok(if cut_us.saturating_mul(4) > before.duration {
        Some(format!(
            "it would cut {} of {} (more than a quarter)",
            clock(cut_us),
            clock(before.duration)
        ))
    } else if recorded > 0 && recorded.saturating_sub(after).saturating_mul(3) > recorded {
        Some(format!(
            "the cleanup would come to {} of the {} recorded (more than a third)",
            clock(recorded.saturating_sub(after)),
            clock(recorded)
        ))
    } else {
        None
    })
}

/// The line that says a guarded pass waits for the user.
fn held(reason: &str) -> String {
    format!(
        "Not applied: {reason}. Ask the user with these numbers; on their go, run it again \
         without `guard`."
    )
}

/// The voice of take short `short` (id or title), for a pass given a short but no stream.
fn take_voice(ctx: &Ctx, short: Option<&str>) -> Result<Option<String>, String> {
    match short {
        Some(short) => Ok(short_named(ctx, short)?.take.map(|t| t.voice)),
        None => Ok(None),
    }
}

fn run_noises(ctx: &Ctx, args: NoiseArgs) -> Result<String, String> {
    let before = opened(ctx)?;
    let stream = match args.stream {
        Some(stream) => Some(stream),
        None => take_voice(ctx, args.short.as_deref())?,
    };
    let key = pick_sound(ctx, stream, Want::Speech)?;
    ctx.progress(0.1, &format!("Listening for mouth noises in {key}"));
    let scan = commands::noises::detect_mouth_noises_impl(
        ctx.state,
        before.handle.clone(),
        key.clone(),
        crate::dsp::noises::NoiseConfig {
            sensitivity: match args.sensitivity {
                Some(sensitivity) => sensitivity,
                None => with_reader(ctx.state, |reader| {
                    Ok(reader.document().cleanup.mouth_noise_sensitivity)
                })?,
            },
            padding_ms: args.padding_ms.unwrap_or(20),
        },
        args.short.clone(),
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
    let hold = (args.guard && !scan.guarded && !chosen.is_empty())
        .then(|| held("no transcript guards the words"));
    if let Some(line) = &hold {
        lines.push(line.clone());
    }
    if args.apply.unwrap_or(true) && hold.is_none() && !chosen.is_empty() {
        let ranges = chosen.iter().flat_map(|n| n.parts.clone()).collect();
        commands::noises::project_noise_mutes_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            key,
            ranges,
            false,
            args.short.clone(),
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
    } else if near > 0 {
        lines.push(format!(
            "None to mute: all {near} are next to words (`nearSpeech: true` mutes them too)."
        ));
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
    /// Automatic mode: apply only if it looks right, else say why (no dry run first).
    #[serde(default)]
    pub guard: bool,
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
    // A level asked for, else the project's own (from its preset unless changed there).
    let ours = with_reader(ctx.state, |reader| Ok(reader.document().cleanup.clone()))?;
    let levels = match args.preset {
        Some(preset) => {
            let (threshold_db, min_duration_ms, padding_ms) = preset.silence();
            let (gap_min, gap_padding, refine) = preset.gaps();
            crate::preset::settings::CleanupSettings {
                silence: crate::preset::settings::SilenceLevel {
                    threshold_db,
                    min_duration_ms,
                    padding_ms,
                },
                gaps: crate::preset::settings::GapLevel {
                    min_duration_ms: gap_min,
                    padding_ms: gap_padding,
                    refine_word_edges: refine,
                    edge_threshold_db: ours.gaps.edge_threshold_db,
                },
                ..ours
            }
        }
        None => ours,
    };
    let short = args
        .short
        .as_deref()
        .map(|s| take_short_id(ctx, s))
        .transpose()?;
    let short = short.as_deref();
    let (result, described) = match args.mode {
        SilenceMode::Silence => {
            let level = &levels.silence;
            let config = crate::dsp::SilenceConfig {
                threshold_db: args.threshold_db.unwrap_or(level.threshold_db),
                min_duration_ms: args.min_duration_ms.unwrap_or(level.min_duration_ms),
                padding_ms: args.padding_ms.unwrap_or(level.padding_ms),
                auto_level: args.auto_level,
                window_ms: None,
                step_ms: None,
                channel_policy: None,
                below_speech_db: None,
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
            let level = &levels.gaps;
            let config = crate::transcript::pauses::TranscriptGapConfig {
                min_duration_ms: args.min_duration_ms.unwrap_or(level.min_duration_ms),
                padding_ms: args.padding_ms.unwrap_or(level.padding_ms),
                refine_word_edges: args.refine_word_edges.unwrap_or(level.refine_word_edges),
                edge_threshold_db: args.edge_threshold_db.unwrap_or(level.edge_threshold_db),
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
    // A short's take is cut much tighter than a video: the guard is for the video.
    let hold = match args.guard && short.is_none() && !chosen.is_empty() {
        true => cut_looks_wrong(ctx, &before, total, None)?.map(|reason| held(&reason)),
        false => None,
    };
    if let Some(line) = &hold {
        lines.push(line.clone());
    }
    if args.apply.unwrap_or(true) && hold.is_none() && !chosen.is_empty() {
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
    /// Whose review adds to the rules (see the tool's description). `local` and `in-house` send
    /// nothing off the computer; `openrouter` uses the user's paid key.
    #[serde(default)]
    pub review: Option<FillerReview>,
    /// false: only list them. Default true.
    #[serde(default)]
    pub apply: Option<bool>,
    /// Automatic mode: apply only if it looks right, else say why (no dry run first).
    #[serde(default)]
    pub guard: bool,
    /// A take short (id or title): its voice's transcript, cut on the short's own timeline.
    #[serde(default)]
    pub short: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
pub enum FillerReview {
    /// The app's review when a key is set, else the local model when downloaded, else the
    /// agent's own.
    #[default]
    #[serde(rename = "auto")]
    Auto,
    /// The app's built-in local model (Qwen3.5 4B), run now.
    #[serde(rename = "local")]
    Local,
    /// The connected agent's own review, saved with `fillers_review`.
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
                crate::ai::fillers::criteria(page.language),
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
                crate::ai::fillers::criteria(page.language),
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
    let stream = match args.stream {
        Some(stream) => Some(stream),
        None => take_voice(ctx, args.short.as_deref())?,
    };
    let key = pick_sound(ctx, stream, Want::Transcribed)?;
    let kinds = args.kinds.unwrap_or_else(|| {
        vec![
            TranscriptSuggestionKind::Filler,
            TranscriptSuggestionKind::Retake,
        ]
    });
    // Rules and AI together by default: the rules alone miss most fillers and retakes.
    let review = args.review.unwrap_or_default();
    use crate::ai::FillerEngine;
    let engine = match review {
        FillerReview::OpenRouter => Some(FillerEngine::Cloud),
        FillerReview::Local => Some(FillerEngine::Local),
        // The OpenRouter review first: measured against the local one on 21 Oct 2026 it missed
        // fewer cuts and made no wrong ones (docs/plans/local-review-eval.md).
        FillerReview::Auto if ctx.host.ai_review_ready() => Some(FillerEngine::Cloud),
        FillerReview::Auto if ctx.host.local_review_ready() => Some(FillerEngine::Local),
        FillerReview::Auto | FillerReview::InHouse => None,
    };
    let name = match engine {
        Some(FillerEngine::Local) => "the local review",
        _ => "the OpenRouter review",
    };
    // A review of this sound still running (a caller that gave up waiting for it, or the
    // app's own): wait for it and use what it saves, rather than starting another.
    let running = format!("AI review of {key}");
    let mut waited = false;
    while ctx.host.transcripts().running.lock().as_deref() == Some(running.as_str()) {
        if !waited {
            ctx.progress(0.1, "Waiting for the review already running");
            waited = true;
        }
        // Stopping this call leaves the review it waits on running.
        ctx.stop_point()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    // By default a saved review is used as it is (a call that timed out still saved its
    // review); naming a review runs it again.
    let saved = with_reader(ctx.state, |reader| {
        Ok(store::load_transcript(reader.root(), &key)?.and_then(|t| t.ai_model))
    })?;
    let engine = engine.filter(|_| review != FillerReview::Auto || saved.is_none());
    let mut use_ai = engine.is_some();
    let mut fallback = None;
    let found = if let Some(engine) = engine {
        ctx.progress(0.1, &format!("Asking {name} for fillers and retakes"));
        match ctx.transcript_job(|| {
            tc::transcript_ai_suggest_impl(
                ctx.state,
                ctx.host.transcripts(),
                before.handle.clone(),
                key.clone(),
                Some(engine),
                &mut |p| ctx.progress(p.fraction, &p.message),
            )
        }) {
            Ok(found) => found,
            Err(error) if ctx.stopped() => return Err(error),
            // Only by default: carry on with the in-house review and say why.
            Err(error) if review == FillerReview::Auto => {
                use_ai = false;
                fallback = Some(error);
                tc::transcript_suggestions_impl(
                    ctx.state,
                    before.handle.clone(),
                    key.clone(),
                    None,
                )?
            }
            Err(error) => return Err(error),
        }
    } else {
        tc::transcript_suggestions_impl(ctx.state, before.handle.clone(), key.clone(), None)?
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
    let describe = |r: &str| {
        if r.starts_with(tc::IN_HOUSE) {
            "the in-house review".to_string()
        } else if r.starts_with("local/") {
            format!("the local review ({r})")
        } else {
            format!("the OpenRouter review ({r})")
        }
    };
    let reviewer = chosen
        .iter()
        .find(|s| s.source == SuggestionSource::Ai)
        .and_then(|s| s.reviewer.as_deref())
        .or(saved.as_deref())
        .map(describe)
        .unwrap_or_else(|| "the review".into());
    let mut lines = vec![format!(
        "Found {} and {} in {key}{}.",
        plural(filler_count, "filler"),
        plural(retake_count, "retake"),
        if use_ai {
            format!(" (rules and {name} run now; {from_ai} from it)")
        } else if from_ai > 0 {
            // The review's finds are kept with the transcript.
            format!(" (rules, and {from_ai} from {reviewer})")
        } else if saved.is_some() {
            // A saved review that found nothing more is still a review: don't ask for another.
            format!(" (rules; {reviewer}, saved, found nothing more)")
        } else {
            " (the rules only)".to_string()
        },
    )];
    if let Some(error) = fallback {
        lines.push(format!(
            "The {} could not run ({error}), so the in-house review was used.",
            name.trim_start_matches("the ")
        ));
    }
    if !use_ai && from_ai == 0 && saved.is_none() {
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
    let hold = match args.guard && !chosen.is_empty() {
        true => {
            let cut: u64 = chosen
                .iter()
                .map(|s| s.edited_end_us.saturating_sub(s.edited_start_us))
                .sum();
            cut_looks_wrong(ctx, &before, cut, Some(&key))?.map(|reason| held(&reason))
        }
        false => None,
    };
    if let Some(line) = &hold {
        lines.push(line.clone());
    }
    if args.apply.unwrap_or(true) && hold.is_none() && !chosen.is_empty() {
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
            None,
        )?;
        lines.push("Cut them all.".into());
    }
    applied(ctx, "fillers", &before, lines)
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct FacesArgs {
    /// false: start detection and report at once. Default true: wait until every camera is
    /// done (a long recording takes a few minutes).
    #[serde(default)]
    pub wait: Option<bool>,
    /// true: drop the existing face tracks and find them again.
    #[serde(default)]
    pub redo: Option<bool>,
}

fn faces(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: FacesArgs = params(args)?;
    let before = opened(ctx)?;
    let (root, document) = with_reader(ctx.state, |reader| {
        Ok((reader.root().to_path_buf(), reader.document().clone()))
    })?;
    if crate::face::camera_streams(&document).is_empty() {
        return text("This project has no camera, so there is no face to find.");
    }
    if args.redo == Some(true) {
        for (asset, stream) in crate::face::camera_streams(&document) {
            let path = crate::face::track_path(&root, &asset, &stream)?;
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
    }
    let mut status = commands::project_face_tracks_impl(ctx.state, before.handle.clone(), true)?;
    while args.wait != Some(false) && status.iter().any(|s| s.state == "detecting") {
        let progress = status.iter().map(|s| {
            s.progress
                .max(if s.state == "detecting" { 0.0 } else { 1.0 })
        });
        let fraction = progress.map(f64::from).sum::<f64>() / status.len() as f64;
        ctx.progress(
            fraction,
            &format!("Finding faces, {:.0}%", fraction * 100.0),
        );
        // Detection runs in the app, not in this call: stopping the call only stops waiting.
        if ctx.stopped() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
        status = commands::project_face_tracks_impl(ctx.state, before.handle.clone(), false)?;
    }
    let lines: Vec<String> = status
        .iter()
        .map(|s| {
            let name = &s.asset_name;
            match (s.state.as_str(), s.face_share) {
                ("ready", Some(share)) => {
                    format!("{name}: face found in {:.0}% of the camera.", share * 100.0)
                }
                ("detecting", _) => format!(
                    "{name}: still finding the face ({:.0}%).",
                    s.progress * 100.0
                ),
                ("failed", _) => format!(
                    "{name}: face detection failed: {}",
                    s.error.as_deref().unwrap_or("unknown error")
                ),
                ("unavailable", _) => format!("{name}: face detection is not in this build."),
                (state, _) => format!("{name}: {state}."),
            }
        })
        .collect();
    text(lines.join(
        "
",
    ))
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
    /// List the zooms in the edit with their ids.
    List,
    /// A zoom of your own over `startUs`..`endUs` (edited time) at `scale`, centred on
    /// `centerX`, `centerY`.
    Create,
    /// Change zoom `id`: `scale`, `centerX`, `centerY`, `fixed`.
    Update,
    /// Remove the zooms in `ids`.
    Remove,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomArgs {
    /// add (default), reload, dismiss; list, create, update, remove for single zooms.
    #[serde(default)]
    pub op: ZoomOp,
    /// create: where it starts and ends, microseconds on the edited timeline.
    #[serde(default)]
    pub start_us: Option<u64>,
    #[serde(default)]
    pub end_us: Option<u64>,
    /// create, update: how far in (1.6 is a usual zoom).
    #[serde(default)]
    pub scale: Option<f64>,
    /// create, update: the point to zoom on, 0 to 1 across and down the screen recording.
    #[serde(default)]
    pub center_x: Option<f64>,
    #[serde(default)]
    pub center_y: Option<f64>,
    /// update: stay on the centre instead of following the mouse.
    #[serde(default)]
    pub fixed: Option<bool>,
    /// update: the zoom's id (from `list`).
    #[serde(default)]
    pub id: Option<String>,
    /// remove: the zooms' ids (from `list`).
    #[serde(default)]
    pub ids: Vec<String>,
    /// Zoom settings to change first, as in `settings zoom`, such as {"clickScale": 1.8}.
    #[serde(default)]
    pub settings: Option<Value>,
    /// false: only report what would be added (with `settings` tried, not saved). Default true.
    #[serde(default)]
    pub apply: Option<bool>,
}

fn zooms(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    text(run_zooms(ctx, params(args)?)?)
}

/// Every zoom in the edit, with its id and where it plays.
fn list_zooms(ctx: &Ctx) -> Result<String, String> {
    with_reader(ctx.state, |reader| {
        let zooms = reader.document().zooms_with_ranges();
        if zooms.is_empty() {
            return Ok("No zooms in the edit.".to_string());
        }
        let mut lines = vec![format!(
            "{} (times on the edited timeline):",
            plural(zooms.len(), "zoom")
        )];
        for zoom in zooms {
            let kind = match zoom.source {
                crate::zoom::ZoomSource::Manual => "manual",
                crate::zoom::ZoomSource::Generated => "from the recording",
            };
            let fixed = if zoom.fixed { ", fixed" } else { "" };
            let at = match zoom.edited_ranges.first() {
                Some(r) => format!(
                    "{}-{} ({}-{} us)",
                    clock(r.start_us),
                    clock(r.end_us),
                    r.start_us,
                    r.end_us
                ),
                None => "cut out".to_string(),
            };
            lines.push(format!(
                "{}: {at}, {:.1}x at ({:.2}, {:.2}), {kind}{fixed}",
                zoom.id, zoom.scale, zoom.center_x, zoom.center_y
            ));
        }
        Ok(lines.join("\n"))
    })
}

/// Creates, changes or removes zooms one by one, as the zoom lane does.
fn single_zoom(ctx: &Ctx, before: &Open, args: ZoomArgs) -> Result<String, String> {
    let in_unit = |v: Option<f64>, name: &str| match v {
        Some(v) if !(0.0..=1.0).contains(&v) => Err(format!("`{name}` goes from 0 to 1")),
        _ => Ok(v),
    };
    let (center_x, center_y) = (
        in_unit(args.center_x, "centerX")?,
        in_unit(args.center_y, "centerY")?,
    );
    let line = match args.op {
        ZoomOp::Create => {
            let (Some(start), Some(end)) = (args.start_us, args.end_us) else {
                return Err("`create` needs `startUs` and `endUs` (edited time)".into());
            };
            let scale = args.scale.unwrap_or(1.6);
            commands::project_zoom_add_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                commands::ManualZoomInput {
                    edited_start_us: start,
                    edited_end_us: end,
                    center_x: center_x.unwrap_or(0.5),
                    center_y: center_y.unwrap_or(0.5),
                    scale,
                },
            )?;
            format!(
                "Added a {scale:.1}x zoom from {} to {}.",
                clock(start),
                clock(end)
            )
        }
        ZoomOp::Update => {
            let id = args
                .id
                .ok_or("`update` needs the zoom's `id` (zooms `op: list`)")?;
            if args.start_us.is_some() || args.end_us.is_some() {
                return Err(
                    "To move a zoom, `remove` it and `create` it where it should be".into(),
                );
            }
            let mut zoom = with_reader(ctx.state, |reader| {
                reader
                    .document()
                    .zooms
                    .iter()
                    .find(|z| z.id == id)
                    .cloned()
                    .ok_or_else(|| format!("No zoom {id} (zooms `op: list`)"))
            })?;
            if let Some(scale) = args.scale {
                zoom.scale = scale;
            }
            if let Some(x) = center_x {
                zoom.center_x = x;
            }
            if let Some(y) = center_y {
                zoom.center_y = y;
            }
            if let Some(fixed) = args.fixed {
                zoom.fixed = fixed;
            }
            commands::project_zoom_update_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                zoom,
            )?;
            format!("Changed zoom {id}.")
        }
        ZoomOp::Remove => {
            if args.ids.is_empty() {
                return Err("`remove` needs `ids` (zooms `op: list`)".into());
            }
            for id in &args.ids {
                let now = opened(ctx)?;
                commands::project_zoom_delete_impl(
                    ctx.state,
                    now.handle,
                    now.revision,
                    id.clone(),
                )?;
            }
            format!("Removed {}.", plural(args.ids.len(), "zoom"))
        }
        ZoomOp::Add | ZoomOp::Reload | ZoomOp::Dismiss | ZoomOp::List => unreachable!(),
    };
    applied(ctx, "zooms", before, vec![line])
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
    match args.op {
        ZoomOp::List => return list_zooms(ctx),
        ZoomOp::Create | ZoomOp::Update | ZoomOp::Remove => return single_zoom(ctx, &before, args),
        ZoomOp::Add | ZoomOp::Reload | ZoomOp::Dismiss => {}
    }
    let mut lines = Vec::new();
    if args.apply == Some(false) {
        if args.op == ZoomOp::Dismiss {
            return Err(
                "`apply: false` previews `add` or `reload`; `dismiss` has nothing to preview"
                    .into(),
            );
        }
        // Other settings are tried without saving them: a dry run changes nothing.
        let generation = match &args.settings {
            Some(patch) => {
                let current = with_reader(ctx.state, |reader| {
                    Ok(reader.document().zoom_settings.clone())
                })?;
                let trial = patched("zoom", &current, patch)?;
                commands::project_zoom_preview_impl(ctx.state, before.handle.clone(), &trial)?
            }
            None => {
                commands::project_zoom_suggestions_impl(ctx.state, before.handle.clone(), None)?
            }
        };
        lines.push(if generation.suggestions.is_empty() {
            "No zoom suggestions to add.".to_string()
        } else {
            format!("Would add {}.", describe_zooms(&generation.suggestions))
        });
        lines.extend(generation.diagnostics);
        if args.settings.is_some() {
            lines.push(
                "Nothing was saved: run it again without `apply: false` to use these settings."
                    .into(),
            );
        }
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
        // Answered at the start.
        ZoomOp::List | ZoomOp::Create | ZoomOp::Update | ZoomOp::Remove => unreachable!(),
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
    Transcription,
    Ai,
    Export,
    Cleanup,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SettingsArgs {
    /// captions, layout, audio, zoom, transcription, ai, export or cleanup.
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
            style_owned(area, &patch)?;
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
                Area::Cleanup => {
                    let next = patched(&name, &document.cleanup, &patch)?;
                    commands::preset::project_cleanup_set_impl(ctx.state, handle, revision, next)?;
                }
                Area::Export => {
                    let next = patched(&name, &document.export, &patch)?;
                    commands::preset::project_export_set_impl(ctx.state, handle, revision, next)?;
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

/// Refuses fields that are how things look: they are the project's style's, never set here.
fn style_owned(area: Area, patch: &Value) -> Result<(), String> {
    let owned: &[&str] = match area {
        Area::Captions => &[
            "style",
            "textColor",
            "emphasis",
            "emphasisColor",
            "weight",
            "outline",
            "background",
            "backgroundColor",
            "backgroundOpacity",
        ],
        Area::Layout => &[
            "backgroundType",
            "backgroundPreset",
            "colorStart",
            "colorEnd",
            "cornerRadiusPx",
            "shadowBlurPx",
            "shadowOpacity",
            "webcamBorderColor",
            "webcamBorderWidth",
            "webcamShadow",
        ],
        _ => &[],
    };
    let named: Vec<&str> = owned
        .iter()
        .copied()
        .filter(|key| patch.get(*key).is_some())
        .collect();
    if named.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} is how it looks: that is the project's style (`library` `kind: style`), not a \
             setting of the project",
            named.join(", ")
        ))
    }
}

fn value<T: Serialize>(settings: &T) -> Value {
    serde_json::to_value(settings).unwrap_or_default()
}

fn current_settings(ctx: &Ctx, area: Area) -> Result<Value, String> {
    Ok(match area {
        Area::Transcription => value(&tc::transcript_settings_get_impl().settings),
        Area::Ai => value(&tc::ai_settings_get_impl().settings),
        Area::Cleanup => {
            let ours = with_reader(ctx.state, |reader| Ok(value(&reader.document().cleanup)))?;
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
            json!({ "project": ours, "levels": presets })
        }
        Area::Export => with_reader(ctx.state, |reader| Ok(value(&reader.document().export)))?,
        area => with_reader(ctx.state, |reader| {
            let document = reader.document();
            Ok(match area {
                Area::Captions => value(&document.captions),
                Area::Layout => value(&document.layout),
                Area::Audio => value(&document.audio),
                Area::Zoom => value(&document.zoom_settings),
                _ => Value::Null,
            })
        })?,
    })
}

// ---------------------------------------------------------------------------------------
// Chapters, shorts, export.

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChapterInput {
    pub title: String,
    /// Where it starts: a word (id from `transcript_get`)...
    #[serde(default)]
    pub at_word: Option<String>,
    /// ...or microseconds on the edited timeline.
    #[serde(default)]
    pub at_us: Option<u64>,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChaptersArgs {
    /// Sound key of a transcript. Default: the transcribed speech.
    #[serde(default)]
    pub stream: Option<String>,
    /// The chapters you wrote, in order: they replace the project's chapters and no AI pass
    /// runs. Leave out to have the app's AI pass make them.
    #[serde(default)]
    pub set: Option<Vec<ChapterInput>>,
}

/// Chapters written by the agent, anchored where they start in the recording.
fn set_chapters(ctx: &Ctx, before: &Open, args: ChaptersArgs) -> Result<Vec<Output>, String> {
    let wanted = args.set.unwrap_or_default();
    let key = match wanted.iter().any(|c| c.at_word.is_some()) {
        true => Some(pick_sound(ctx, args.stream, Want::Transcribed)?),
        false => None,
    };
    let chapters = with_reader(ctx.state, |reader| {
        let document = reader.document();
        let transcript = match &key {
            Some(key) => Some(
                store::load_transcript(reader.root(), key)?
                    .ok_or("That sound is not transcribed")?,
            ),
            None => None,
        };
        let mut chapters = Vec::new();
        for (n, input) in wanted.iter().enumerate() {
            let title = input.title.trim();
            if title.is_empty() {
                return Err(format!("Chapter {} has no title", n + 1));
            }
            let (media, source_us) = match (&input.at_word, input.at_us) {
                (Some(word), None) => {
                    let (Some(key), Some(transcript)) = (&key, &transcript) else {
                        unreachable!()
                    };
                    let found = transcript
                        .words
                        .iter()
                        .find(|w| &w.id == word)
                        .ok_or_else(|| format!("No word {word} in the transcript of {key}"))?;
                    (document.stream_ref(key)?.asset, found.source_start_us)
                }
                (None, Some(at)) => {
                    // Anchored to what plays there: the top picture, else any clip.
                    let tracks = document.sequence.tracks.iter();
                    let pictures = tracks
                        .clone()
                        .filter(|t| t.kind == crate::sequence::TrackKind::Video)
                        .rev();
                    let sounds = tracks.filter(|t| t.kind == crate::sequence::TrackKind::Audio);
                    let clip = pictures
                        .chain(sounds)
                        .find_map(|t| t.clip_at(at))
                        .ok_or_else(|| {
                            format!("Nothing plays at {} for chapter {}", clock(at), n + 1)
                        })?;
                    (clip.asset.clone(), clip.local_us(at).unwrap_or(clip.in_us))
                }
                _ => return Err(format!("Chapter {}: give one of `atWord` or `atUs`", n + 1)),
            };
            chapters.push(crate::chapters::Chapter {
                id: format!("ch-{source_us}-{n}"),
                source_us,
                title: title.to_string(),
                media: Some(media),
                edited_us: None,
            });
        }
        Ok(chapters)
    })?;
    let project = commands::project_chapters_set_impl(
        ctx.state,
        before.handle.clone(),
        before.revision,
        chapters,
    )?;
    let mut lines = vec![format!(
        "Set {}:",
        plural(project.chapters.len(), "chapter")
    )];
    lines.extend(chapter_lines(&project.chapters));
    let answer = applied(ctx, "chapters", before, lines)?;
    if let Ok(now) = current(ctx) {
        chapters_made(&now);
    }
    text(answer)
}

/// One line per chapter, as the export writes them: the first plays from 0:00 (YouTube's rule),
/// wherever its word starts.
fn chapter_lines(chapters: &[crate::chapters::Chapter]) -> Vec<String> {
    let first = chapters.iter().filter_map(|c| c.edited_us).min();
    chapters
        .iter()
        .map(|c| {
            let at = match c.edited_us {
                Some(at) if Some(at) == first => clock(0),
                Some(at) => clock(at),
                None => "(cut)".to_string(),
            };
            format!("{at} {}", c.title)
        })
        .collect()
}

fn chapters(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ChaptersArgs = params(args)?;
    let before = opened(ctx)?;
    if args.set.is_some() {
        return set_chapters(ctx, &before, args);
    }
    let key = pick_sound(ctx, args.stream, Want::Transcribed)?;
    ctx.progress(0.1, "Asking the AI pass for chapters");
    let project = ctx.transcript_job(|| {
        tc::project_chapters_generate_impl(
            ctx.state,
            ctx.host.transcripts(),
            before.handle.clone(),
            key,
        )
    })?;
    let mut lines = vec![format!(
        "Made {}:",
        plural(project.chapters.len(), "chapter")
    )];
    lines.extend(chapter_lines(&project.chapters));
    let answer = applied(ctx, "chapters", &before, lines)?;
    if let Ok(now) = current(ctx) {
        chapters_made(&now);
    }
    text(answer)
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaImportArgs {
    /// Full paths of the files (or recording folders) to import.
    #[serde(default)]
    pub paths: Vec<String>,
    /// What streams stand for, set after importing (or alone, for media already imported).
    #[serde(default)]
    pub roles: Vec<MediaRoleArgs>,
    /// Licences of music and sound files (after importing, or alone).
    #[serde(default)]
    pub licences: Vec<MediaLicenceArgs>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaLicenceArgs {
    /// The media id.
    pub media: String,
    /// cc0, publicDomain, noCredit (free without credit), creditRequired or unknown.
    pub kind: crate::sequence::LicenceKind,
    /// Where it came from (a site, a pack, the user's own).
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaRoleArgs {
    /// The media id (from this answer or project_info).
    pub media: String,
    /// Its stream, such as `picture`, `camera` or `audio:0`.
    pub stream: String,
    /// screen, webcam, overlay (pictures); mic (speech), background (game, desktop), music
    /// or effects (sounds).
    pub role: crate::sequence::Role,
}

fn media_import(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: MediaImportArgs = params(args)?;
    if args.paths.is_empty() && args.roles.is_empty() && args.licences.is_empty() {
        return Err("Give `paths` to import, `roles` or `licences` to set, or both".into());
    }
    let before = opened(ctx)?;
    if args.paths.is_empty() {
        let mut lines = set_roles(ctx, args.roles)?;
        lines.extend(set_licences(ctx, args.licences)?);
        return text(applied(ctx, "media_import", &before, lines)?);
    }
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
    lines.extend(set_roles(ctx, args.roles)?);
    lines.extend(set_licences(ctx, args.licences)?);
    text(applied(ctx, "media_import", &before, lines)?)
}

/// Sets licences, one undo step per media.
fn set_licences(ctx: &Ctx, licences: Vec<MediaLicenceArgs>) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    for licence in licences {
        let now = opened(ctx)?;
        let kind = licence.kind;
        commands::project_media_licence_impl(
            ctx.state,
            now.handle,
            now.revision,
            licence.media.clone(),
            Some(crate::sequence::Licence {
                kind,
                source: licence.source,
            }),
        )?;
        let warn = if kind.needs_no_credit() {
            ""
        } else {
            " It needs credit or is unknown: don't use it as music unless the user says so."
        };
        lines.push(format!("Set the licence of {}.{warn}", licence.media));
    }
    Ok(lines)
}

/// Sets stream roles, one undo step per media.
fn set_roles(ctx: &Ctx, roles: Vec<MediaRoleArgs>) -> Result<Vec<String>, String> {
    let mut by_media: Vec<(String, Vec<commands::StreamRoleInput>)> = Vec::new();
    for role in roles {
        let input = commands::StreamRoleInput {
            stream: role.stream,
            role: role.role,
        };
        match by_media.iter_mut().find(|(m, _)| *m == role.media) {
            Some((_, list)) => list.push(input),
            None => by_media.push((role.media, vec![input])),
        }
    }
    let mut lines = Vec::new();
    for (media, roles) in by_media {
        let now = opened(ctx)?;
        let count = roles.len();
        commands::project_media_roles_impl(
            ctx.state,
            now.handle,
            now.revision,
            media.clone(),
            roles,
        )?;
        lines.push(format!(
            "Set the role of {} of {media}.",
            plural(count, "stream")
        ));
    }
    Ok(lines)
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
    lines.push(
        "Unless the user named it, rename it after the take's words once they are read \
         (`shorts` `op: rename`), never after the video."
            .into(),
    );
    text(applied(ctx, "short_from_take", &before, lines)?)
}

#[derive(Deserialize, JsonSchema, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FootageOp {
    #[default]
    List,
    Add,
    Remove,
    Sound,
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
    /// ...or from `videoS` seconds on the video's timeline (the times `transcript_get` shows)...
    #[serde(default)]
    pub video_s: Option<f64>,
    /// ...or B-roll from outside the video: an imported file's id (`media_import`, `status`),
    /// from `mediaS` seconds into it (default 0).
    #[serde(default)]
    pub media: Option<String>,
    #[serde(default)]
    pub media_s: Option<f64>,
    /// Play the footage's game or desktop sound, ducked under the voice (never its microphone).
    /// Default true.
    #[serde(default)]
    pub sound: Option<bool>,
    /// How fast the footage plays, 0.25-4 (default 1): 1.5 fits 1.5x the footage into the
    /// gap, 0.75 stretches less to fill it. Trim by choosing a shorter stretch; jump-cut with
    /// several items in a row.
    #[serde(default)]
    pub speed: Option<f64>,
    /// For a graphic: how it shows. `full` (the whole frame: the graphic is the point),
    /// `screen` (split: the face above, the graphic in the screen's part, footage or not) or
    /// `overFace` (the face fills the frame and the graphic is drawn over it, see-through:
    /// callouts, labels, arrows). Picked before it renders, so it renders once. Default full.
    #[serde(default)]
    pub mode: Option<crate::shorts::GraphicSlot>,
    /// Or a graphic: a template name, a library item, or one of the video's graphics (its id
    /// from `graphics`), with its `name` and `variables`.
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub library: Option<String>,
    #[serde(default)]
    pub graphic: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub variables: serde_json::Map<String, Value>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShortFootageArgs {
    /// The short's id (or title).
    pub short: String,
    /// For `add`: what the batch is for, in a sentence the user reads.
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub op: Option<FootageOp>,
    #[serde(default)]
    pub items: Vec<FootageItemArgs>,
    /// For `remove`: seconds on the short; without them, all footage.
    #[serde(default)]
    pub from_s: Option<f64>,
    #[serde(default)]
    pub to_s: Option<f64>,
    /// For `sound`: the footage and B-roll sound's volume, -30 to 12 dB (0 as recorded)...
    #[serde(default)]
    pub volume_db: Option<f32>,
    /// ...and how far it is lowered while the voice talks, 3 to 30 dB.
    #[serde(default)]
    pub duck_db: Option<f32>,
}

fn to_us(seconds: f64) -> Result<u64, String> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(format!("{seconds} is not a time in seconds"));
    }
    Ok((seconds * 1e6).round() as u64)
}

/// A short by its id or title.
fn short_named(ctx: &Ctx, wanted: &str) -> Result<crate::shorts::Short, String> {
    super::clips::named(ctx, wanted)
}

/// A short made from a take by its id or title: footage and graphics go on its own timeline.
fn take_short_named(ctx: &Ctx, wanted: &str) -> Result<crate::shorts::Short, String> {
    let short = short_named(ctx, wanted)?;
    if short.take.is_none() {
        return Err(format!(
            "\"{}\" is a clip (cut from the video): it shows the video itself, so footage, B-roll and graphics under a take go only on shorts made from a take (`short_from_take`). A graphic in a clip is `graphic_add` on the video, then `graphic_set` with `short` for how it shows there.",
            short.title
        ));
    }
    Ok(short)
}

fn short_footage(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ShortFootageArgs = params(args)?;
    let before = opened(ctx)?;
    let short = take_short_named(ctx, &args.short)?;
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
                let from = if f.outside {
                    format!("{} at {}", f.media, clock(f.in_us))
                } else {
                    f.video_us
                        .map(|v| format!("the video at {}", clock(v)))
                        .unwrap_or_else(|| "a part of the video cut since".into())
                };
                format!(
                    "{}-{}: {from}{}",
                    clock(f.at_us),
                    clock(f.to_us),
                    if f.sound { ", with its sound" } else { "" }
                )
            })
            .collect())
    };
    let sound_line = |short: &crate::shorts::Short| -> Result<String, String> {
        with_reader(ctx.state, |reader| {
            Ok(
                crate::take_short::footage_sound_mix(reader.document(), short)
                    .map(|(_, volume, duck)| {
                        format!("Footage sound: {volume:+} dB, lowered {duck} dB under the voice.")
                    })
                    .unwrap_or_default(),
            )
        })
    };
    let change = match args.op.unwrap_or_default() {
        FootageOp::List => {
            let mut lines = list(&short)?;
            lines.push(sound_line(&short)?);
            return text(lines.join("\n"));
        }
        FootageOp::Sound => {
            let (track, volume, duck) = with_reader(ctx.state, |reader| {
                crate::take_short::footage_sound_mix(reader.document(), &short)
                    .ok_or_else(|| format!("{} has no footage sound track", short.id))
            })?;
            let mut audio = with_reader(ctx.state, |reader| Ok(reader.document().audio.clone()))?;
            let (lo, hi) = crate::project::audio::TRACK_VOLUME_DB_RANGE;
            let volume_db = args.volume_db.unwrap_or(volume);
            if !(lo..=hi).contains(&volume_db) {
                return Err(format!("volumeDb is {lo} to {hi}"));
            }
            let (lo, hi) = crate::project::audio::DUCK_DB_RANGE;
            let duck_db = args.duck_db.unwrap_or(duck);
            if !(lo..=hi).contains(&duck_db) {
                return Err(format!("duckDb is {lo} to {hi}"));
            }
            let mix = audio.tracks.entry(track).or_default();
            mix.volume_db = volume_db;
            mix.duck_db = Some(duck_db);
            commands::project_audio_update_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                audio,
            )?;
            let line = sound_line(&short)?;
            return text(applied(ctx, "short_footage", &before, vec![line])?);
        }
        FootageOp::Remove => commands::FootageChange::Remove {
            from_us: args.from_s.map(to_us).transpose()?,
            to_us: args.to_s.map(to_us).transpose()?,
        },
        FootageOp::Add => return propose_under(ctx, &short, &args),
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

/// `short_footage` `op: add`: the items as one batch for the user (autopilot accepts it).
fn propose_under(
    ctx: &Ctx,
    short: &crate::shorts::Short,
    args: &ShortFootageArgs,
) -> Result<Vec<Output>, String> {
    use crate::under_take::{GraphicFrom, Under};
    let items = with_reader(ctx.state, |reader| {
        let take = short.take.as_ref().ok_or("No take")?;
        let take_words = store::load_transcript(reader.root(), &take.voice)?;
        let mut out = Vec::new();
        for (n, item) in args.items.iter().enumerate() {
            let at = |error: String| format!("Item {}: {error}", n + 1);
            let (at_us, to) = match (&item.from_word, &item.to_word, item.at_s, item.to_s) {
                (Some(from), to_word, _, _) => {
                    let transcript = take_words.as_ref().ok_or_else(|| {
                        format!(
                            "The take is not transcribed: `transcribe` with stream {}",
                            take.voice
                        )
                    })?;
                    let last = to_word.as_deref().unwrap_or(from);
                    let (a, b) =
                        crate::take_short::words_at(short, transcript, from, last).map_err(at)?;
                    (a, to_word.is_some().then_some(b))
                }
                (None, _, Some(a), b) => (to_us(a)?, b.map(to_us).transpose()?),
                _ => {
                    return Err(at(
                        "give `fromWord` (and `toWord`) or `atS` (and `toS`)".into()
                    ))
                }
            };
            let graphic = match (&item.template, &item.library, &item.graphic) {
                (None, None, None) => None,
                (Some(template), None, None) => Some(GraphicFrom::Template {
                    template: template.clone(),
                }),
                (None, Some(id), None) => Some(GraphicFrom::Library { id: id.clone() }),
                (None, None, Some(graphic)) => Some(GraphicFrom::Video {
                    graphic: graphic.clone(),
                }),
                _ => return Err(at("give one of `template`, `library` or `graphic`".into())),
            };
            if let Some(from) = graphic {
                out.push(Under::Graphic {
                    from,
                    at_us,
                    length_us: to.map(|t| t.saturating_sub(at_us)).filter(|l| *l > 0),
                    name: item.name.clone(),
                    variables: item.variables.clone(),
                    mode: item.mode,
                });
                continue;
            }
            let to = to.ok_or_else(|| at("footage needs an end: `toWord` or `toS`".into()))?;
            let video_us = match (&item.source_word, item.video_s) {
                _ if item.media.is_some() => to_us(item.media_s.unwrap_or(0.0))?,
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
                        .ok_or_else(|| at(format!("no word {word} in {key}")))?;
                    // Only what the video keeps: the word must be on its timeline.
                    reader
                        .document()
                        .transcript_clock(&key)?
                        .source_to_edited_us(source.source_start_us)
                        .ok_or_else(|| {
                            at(format!(
                                "word {word} was cut from the video; pick one it keeps"
                            ))
                        })?
                }
                (None, Some(seconds)) => to_us(seconds)?,
                _ => {
                    return Err(at(
                        "give `sourceWord`, `videoS`, `media` or a graphic".into()
                    ))
                }
            };
            out.push(Under::Footage(crate::take_short::Footage {
                at_us,
                to_us: to,
                video_us,
                media: item.media.clone(),
                sound: item.sound.unwrap_or(true),
                speed: item.speed.unwrap_or(1.0),
            }));
        }
        Ok(out)
    })?;
    let reason = args
        .reason
        .clone()
        .filter(|r| !r.trim().is_empty())
        .unwrap_or_else(|| format!("What plays under \"{}\"", short.title));
    let (agent, id) = (ctx.agent.clone(), short.id.clone());
    super::propose::propose(ctx, |reader, proposals| {
        crate::proposals::propose_under(reader, proposals, &agent, &id, items, &reason)
    })
}

/// The id of the take short `wanted` names (its id or title), for cleaning its take.
fn take_short_id(ctx: &Ctx, wanted: &str) -> Result<String, String> {
    let short = short_named(ctx, wanted)?;
    if short.take.is_none() {
        return Err(format!(
            "\"{}\" is a clip cut from the video: clean the video itself, then `clips` with `op: range` (or the Clips tab's \"Take the video's cuts again\") brings the cleaned words into it",
            short.title
        ));
    }
    Ok(short.id)
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
    let mut lines = vec![format!(
        "Fixed {}, each its own undo step: {}.",
        plural(fixed.len(), "finding"),
        fixed.join("; ")
    )];
    // What is left, briefly: this is the check after the fixes, so no need to run it again.
    lines.push(crate::check::left_text(&after));
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
    /// Export vertical shorts or clips instead of the video: one's id or title, "shorts" (every
    /// short made from a take), "clips" (every clip) or "all". They go next to the project,
    /// named after it and the short (1080x1920), one after another.
    #[serde(default)]
    pub short: Option<String>,
    /// Stop the export with this job id (from `wait: false` or export_status) instead.
    #[serde(default)]
    pub cancel: Option<String>,
    /// Save another file instead of the video, of the video or of `short`: subtitles (`srt`,
    /// `vtt`, from the captions as cut, hidden words left out), the transcript (`txt`, `md`
    /// with the chapters as headings) or the sound alone (`wav`). `settings` don't apply.
    #[serde(default)]
    pub file: Option<crate::export::files::FileKind>,
}

/// The project's export settings (from its preset unless changed here).
fn export_defaults(ctx: &Ctx) -> Result<crate::export::ExportSettings, String> {
    with_reader(ctx.state, |reader| {
        let document = reader.document();
        document.export.settings(&document.layout)
    })
}

fn export(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ExportArgs = params(args)?;
    if let Some(job) = args.cancel {
        let status = commands::export_cancel_impl(ctx.state, job.clone())?;
        return text(format!("Asked export {job} to stop: {:?}.", status.state));
    }
    let open = opened(ctx)?;
    if let Some(kind) = args.file {
        return export_files(
            ctx,
            &open.handle,
            kind,
            args.short.as_deref(),
            args.destination,
        );
    }
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
    // Refused before it began (a folder that doesn't exist, say): say so, not "started".
    if started.state == crate::export::ExportState::Failed {
        return Err(export_failure(&started).replace("stopped", "did not start"));
    }
    ctx.host.export_started(&started, false);
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
                if ctx.stopped() {
                    commands::export_cancel_impl(ctx.state, job_id.to_string())?;
                    return Err(format!(
                        "Stopped: export {job_id} was cancelled (the user, or your client, \
                         cancelled the call) and no file was saved."
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }
}

/// Saves the file `kind` of the video, or of the short `short` (id, title or "all").
fn export_files(
    ctx: &Ctx,
    handle: &str,
    kind: crate::export::files::FileKind,
    short: Option<&str>,
    destination: Option<String>,
) -> Result<Vec<Output>, String> {
    let targets: Vec<(Option<String>, String)> = match short {
        Some(wanted) => chosen_shorts(ctx, wanted)?
            .into_iter()
            .map(|(id, title)| (Some(id), format!("\"{title}\"")))
            .collect(),
        None => vec![(None, "The video".into())],
    };
    // One file name can't hold several shorts: each goes next to the project.
    let destination = if targets.len() == 1 {
        destination
    } else {
        None
    };
    let mut lines = Vec::new();
    for (i, (id, label)) in targets.iter().enumerate() {
        let path = commands::export_file_impl(
            ctx.state,
            handle.to_string(),
            id.clone(),
            kind,
            destination.clone(),
            &mut |done| {
                ctx.progress(
                    (i as f64 + done) / targets.len() as f64,
                    &format!("Writing {label}, {:.0}%", done * 100.0),
                )
            },
        )?;
        lines.push(format!("{label}: {}", path.display()));
    }
    text(lines.join("\n"))
}

/// The shorts `wanted` names (id or title), or all of them for "all", as (id, title).
fn chosen_shorts(ctx: &Ctx, wanted: &str) -> Result<Vec<(String, String)>, String> {
    let shorts: Vec<(String, String, bool)> = with_reader(ctx.state, |reader| {
        Ok(reader
            .document()
            .shorts
            .iter()
            .map(|s| (s.id.clone(), s.title.clone(), s.take.is_some()))
            .collect())
    })?;
    let every = |take: Option<bool>| -> Vec<(String, String)> {
        shorts
            .iter()
            .filter(|(_, _, t)| take.is_none_or(|take| *t == take))
            .map(|(id, title, _)| (id.clone(), title.clone()))
            .collect()
    };
    let wanted_kind = wanted.trim().to_ascii_lowercase();
    let chosen: Vec<(String, String)> = if wanted_kind == "all" {
        every(None)
    } else if wanted_kind == "shorts" {
        every(Some(true))
    } else if wanted_kind == "clips" {
        every(Some(false))
    } else {
        let shorts = every(None);
        shorts
            .iter()
            .filter(|(id, title)| id == wanted || title.eq_ignore_ascii_case(wanted.trim()))
            .take(1)
            .cloned()
            .collect()
    };
    if chosen.is_empty() {
        return Err(if shorts.is_empty() {
            "No shorts or clips yet: `short_from_take` makes a short, `clips` a clip.".into()
        } else {
            format!(
                "No short or clip {wanted}. There are: {}",
                shorts
                    .iter()
                    .map(|(id, title, take)| {
                        format!(
                            "{id} \"{title}\" ({})",
                            if *take { "short" } else { "clip" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });
    }
    Ok(chosen)
}

/// Exports the short `wanted` (id or title) or "all", one after another.
fn export_shorts(
    ctx: &Ctx,
    handle: &str,
    wanted: &str,
    mut settings: crate::export::ExportSettings,
) -> Result<Vec<Output>, String> {
    let chosen = chosen_shorts(ctx, wanted)?;
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
        ctx.host.export_started(&started, true);
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

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExportStatusArgs {
    /// Seconds to wait for the export to finish before answering, at most 300. Default 0.
    #[serde(default)]
    pub wait_s: Option<u64>,
}

fn export_status(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ExportStatusArgs = params(args)?;
    use crate::export::ExportState::*;
    let until = std::time::Instant::now()
        + std::time::Duration::from_secs(args.wait_s.unwrap_or(0).min(300));
    let status = loop {
        let status = commands::export_status_impl(ctx.state, None)?;
        // Stopping a wait answers at once; the export carries on (`export cancel` stops it).
        if !matches!(status.state, Queued | Running)
            || std::time::Instant::now() >= until
            || ctx.stopped()
        {
            break status;
        }
        if let (Some(stage), 0) = (&status.stage, status.progress_numerator) {
            ctx.progress(status.stage_fraction.unwrap_or(0.0), stage);
        } else if status.progress_denominator > 0 {
            let fraction = status.progress_numerator as f64 / status.progress_denominator as f64;
            ctx.progress(fraction, &format!("Exporting, {:.0}%", fraction * 100.0));
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    };
    text(match status.state {
        Idle => "No export has run since the app started.".to_string(),
        // Graphics render before the first frame, at 0 frames: say so, or it reads as stuck.
        Queued | Running => match (&status.stage, status.progress_numerator) {
            (Some(stage), 0) => format!(
                "Export {} running: {stage} ({:.0}%), before its frames. It is working: wait.",
                status.job_id,
                status.stage_fraction.unwrap_or(0.0) * 100.0
            ),
            _ => format!(
                "Export {} running: {}/{} frames.",
                status.job_id, status.progress_numerator, status.progress_denominator
            ),
        },
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
    // Templates and the library as a line each (AI-1: about 40 % fewer characters than JSON):
    // the colours every one takes are said once, since the style sets them.
    let mut lines = vec![format!(
        "Renderer: {}.",
        if runtime.installed {
            format!("installed (HyperFrames {})", runtime.hyperframes)
        } else {
            "not installed: ask the user, then call graphics_install".to_string()
        }
    )];
    lines.push(
        "Templates (name, length: what it is; its variables and their defaults; besides these, \
         each takes the colours `background`, `text`, `accent`, which the style sets):"
            .into(),
    );
    lines.extend(templates.iter().map(|t| {
        format!(
            "- {}, {}: {} {}",
            t.name,
            seconds(t.length_us),
            t.description,
            variable_list(&t.variables)
        )
    }));
    let library = crate::graphics::library::list(&crate::graphics::library::dir());
    if library.is_empty() {
        lines.push("Library: empty.".into());
    } else {
        lines.push(
            "Library (name and id, length: when to use it; its variables and their defaults):"
                .into(),
        );
        lines.extend(library.iter().map(|item| {
            format!(
                "- {} ({}), {}{}: {} {}",
                item.item.name,
                item.id,
                seconds(item.item.length_us),
                if item.item.general {
                    ""
                } else {
                    ", as is (not general)"
                },
                item.item.use_when,
                variable_list(&item.variables)
            )
        }));
    }
    let mut out = json!({});
    out["graphics"] = json!(in_project
        .iter()
        .map(|g| {
            let mut listed = json!({
            "id": g.graphic.asset_id,
            "name": g.graphic.name,
            "template": g.graphic.template,
            "folder": g.graphic.folder,
            "variables": g.graphic.variables,
            "style": g.graphic.style.clone().unwrap_or_else(|| "project".into()),
            "plays": g.clips.iter().map(|(id, start, length)| json!({
                "clip": id,
                "at": clock(*start),
                "atUs": start,
                "lengthUs": length,
            })).collect::<Vec<_>>(),
            "render": render_line(&g.graphic.render),
            });
            // Look tokens of its own, only when it has some.
            if !g.graphic.look.is_empty() {
                listed["look"] = json!(g.graphic.look);
            }
            listed
        })
        .collect::<Vec<_>>());
    let graphics = serde_json::to_string(&out["graphics"]).map_err(|e| e.to_string())?;
    lines.push(format!("Graphics in the project: {graphics}"));
    // The rules for graphics come with the list: only edits with graphics need them.
    let shorts =
        with_reader(ctx.state, |reader| Ok(!reader.document().shorts.is_empty())).unwrap_or(false);
    text(format!(
        "{}\n\n{}",
        super::skill::graphics_guide(shorts),
        lines.join("\n")
    ))
}

/// `4 s`, `10.6 s`.
fn seconds(us: u64) -> String {
    let s = us as f64 / 1e6;
    match s.fract() {
        f if f < 0.05 => format!("{s:.0} s"),
        _ => format!("{s:.1} s"),
    }
}

/// A graphic's variables in a line: `title "Setting up"`, `count 3`, `side (left|right) "left"`,
/// without the colours the style sets.
fn variable_list(variables: &[crate::graphics::VariableDecl]) -> String {
    let listed: Vec<String> = variables
        .iter()
        .filter(|v| {
            !(v.kind == "color" && matches!(v.id.as_str(), "background" | "text" | "accent"))
        })
        .map(|v| {
            let options = match v.options.is_empty() {
                true => String::new(),
                false => format!(
                    " ({})",
                    v.options
                        .iter()
                        .map(|o| o.value.to_string().trim_matches('"').to_string())
                        .collect::<Vec<_>>()
                        .join("|")
                ),
            };
            // A default of 3 or true says its type; a colour's reads as text.
            let kind = match v.kind.as_str() {
                "color" => " (color)",
                _ => "",
            };
            match &v.default {
                Value::Null => format!("{}{kind}{options}", v.id),
                default => format!("{}{kind}{options} {default}", v.id),
            }
        })
        .collect();
    match listed.is_empty() {
        true => "No variables.".into(),
        false => format!("Variables: {}.", listed.join(", ")),
    }
}

/// One graphic to add: from a template or the library, where and how long it plays.
#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraphicItem {
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
    /// How long it plays; default the template's.
    #[serde(default)]
    pub length_us: Option<u64>,
    #[serde(default)]
    pub name: Option<String>,
    /// Values for its variables, by id.
    #[serde(default)]
    pub variables: serde_json::Map<String, Value>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GraphicAddArgs {
    /// The old one-graphic shape (its fields at the top level), still read; not published.
    #[serde(flatten)]
    #[schemars(skip)]
    pub one: GraphicItem,
    /// The graphics to add, in one undo step: send them all in one call.
    #[serde(default)]
    pub items: Vec<GraphicItem>,
    /// The sound whose transcript `atWord` is from; default the transcribed speech.
    #[serde(default)]
    pub stream: Option<String>,
    /// Not here: graphics under a take short go in `short_footage` items.
    #[serde(default)]
    pub short: Option<String>,
}

fn graphic_add(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: GraphicAddArgs = params(args)?;
    if args.short.is_some() {
        return Err(
            "Graphics under a take short go in `short_footage` items (`op: add` with \
                    `template`, `library` or `graphic`), proposed as one batch"
                .into(),
        );
    }
    let before = opened(ctx)?;
    let single = args.one.template.is_some() || args.one.library.is_some();
    let items: Vec<GraphicItem> = single
        .then_some(args.one)
        .into_iter()
        .chain(args.items)
        .collect();
    if items.is_empty() {
        return Err("Give `template` or `library` (or `items`)".into());
    }
    let several = items.len() > 1;
    let mut graphics = Vec::with_capacity(items.len());
    for (n, item) in items.into_iter().enumerate() {
        let at = |error: String| {
            if several {
                format!("Graphic {}: {error}", n + 1)
            } else {
                error
            }
        };
        let at_us = match (item.at_us, &item.at_word) {
            (Some(at), None) => at,
            (None, Some(word)) => word_edited_us(ctx, args.stream.clone(), word).map_err(at)?,
            _ => return Err(at("give one of `atUs` or `atWord`".into())),
        };
        let from = match (item.template, item.library) {
            (Some(template), None) => commands::graphics::ShortGraphicFrom::Template { template },
            (None, Some(id)) => commands::graphics::ShortGraphicFrom::Library { id },
            _ => return Err(at("give one of `template` or `library`".into())),
        };
        graphics.push(commands::graphics::VideoGraphic {
            from,
            at_us,
            length_us: item.length_us,
            name: item.name,
            variables: item.variables,
        });
    }
    let starts: Vec<u64> = graphics.iter().map(|g| g.at_us).collect();
    let (_, placed) = commands::graphics::project_graphics_add_impl(
        ctx.state,
        before.handle.clone(),
        before.revision,
        graphics,
    )?;
    let mut lines: Vec<String> = placed
        .iter()
        .zip(&starts)
        .map(|(g, at)| {
            format!(
                "Added graphic {} \"{}\" ({}) at {} for {:.1} s, clip {}.",
                g.asset_id,
                g.name,
                g.said,
                clock(*at),
                g.length_us as f64 / 1e6,
                g.clip_id
            )
        })
        .collect();
    lines.push(if crate::graphics::runtime::status().installed {
        "They render in the background; graphic_render waits for them.".into()
    } else {
        "The renderer is not installed, so they show as placeholders: ask the user, then graphics_install.".into()
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
    /// Its style: `project` (the project's) or a style's name from `library`.
    #[serde(default)]
    pub style: Option<String>,
    /// Graphics look tokens of its own over its style's, by name (`radius`, `ease`...); null
    /// takes one back.
    #[serde(default)]
    pub look: std::collections::BTreeMap<String, Option<String>>,
    /// A short or clip (id or title): set the graphic's `slot` in it; for a short made from a take,
    /// moves and lengths are on its own timeline too (`moveToWord` a word of the take).
    #[serde(default)]
    pub short: Option<String>,
    /// Where it plays in that short.
    #[serde(default)]
    pub slot: Option<crate::shorts::GraphicSlot>,
    /// Take the graphic out of the project, with its clips. One undo step.
    #[serde(default)]
    pub remove: bool,
    /// Move it so it starts here, microseconds on the edited timeline...
    #[serde(default)]
    pub move_to_us: Option<u64>,
    /// ...or at this word (id from `transcript_get`).
    #[serde(default)]
    pub move_to_word: Option<String>,
    /// The sound `moveToWord` is from; default the speech that is transcribed.
    #[serde(default)]
    pub stream: Option<String>,
}

/// Where word `word` of the transcript of `stream` starts on the edited timeline.
fn word_edited_us(ctx: &Ctx, stream: Option<String>, word: &str) -> Result<u64, String> {
    let key = pick_sound(ctx, stream, Want::Transcribed)?;
    with_reader(ctx.state, |reader| {
        let transcript =
            store::load_transcript(reader.root(), &key)?.ok_or("That sound is not transcribed")?;
        let found = transcript
            .words
            .iter()
            .find(|w| w.id == word)
            .ok_or_else(|| format!("No word {word} in the transcript of {key}"))?;
        reader
            .document()
            .transcript_clock(&key)?
            .source_to_edited_us(found.source_start_us)
            .ok_or_else(|| format!("Word {word} was cut from the video"))
    })
}

fn graphic_set(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: GraphicSetArgs = params(args)?;
    let before = opened(ctx)?;
    let dir = graphic_folder(ctx, &args.graphic)?;
    if args.remove {
        commands::project_media_remove_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            args.graphic.clone(),
        )?;
        let line = format!("Removed graphic {} and its clips.", args.graphic);
        return text(applied(ctx, "graphic_set", &before, vec![line])?);
    }
    // What can be wrong with the call is found before anything changes.
    if args.slot.is_some() && args.short.is_none() {
        return Err("`slot` needs `short`".into());
    }
    if let Some(style) = &args.style {
        crate::graphics::style_choice(style)?;
    }
    if !args.look.is_empty() {
        let tokens = args
            .look
            .iter()
            .filter_map(|(name, value)| Some((name.clone(), value.clone()?)))
            .collect();
        crate::graphics::look::check_tokens(&tokens)?;
    }
    if !args.variables.is_empty() {
        let source = crate::graphics::read_source(&dir)?;
        let declared = crate::graphics::declarations(&dir)?;
        crate::graphics::merge_variables(&declared, &source.variables, &args.variables)?;
    }
    let mut lines = Vec::new();
    // A take short's graphics are on its own timeline: moves and lengths happen there, at its
    // words. A clip's graphics are the video's.
    let take = match &args.short {
        Some(wanted) => Some(short_named(ctx, wanted)?).filter(|s| s.take.is_some()),
        None => None,
    };
    let on_take = take.as_ref().map(|s| s.id.clone());
    let clip_of = |ctx: &Ctx| {
        with_reader(ctx.state, |reader| {
            let document = reader.document();
            let found = match &on_take {
                Some(id) => document
                    .shorts
                    .iter()
                    .find(|s| &s.id == id)
                    .and_then(|s| s.edit.as_ref())
                    .and_then(|e| {
                        e.sequence
                            .clips()
                            .find(|(_, c)| c.asset == args.graphic)
                            .map(|(_, c)| (c.id.clone(), c.start_us))
                    }),
                None => document
                    .sequence
                    .clips()
                    .find(|(_, c)| c.asset == args.graphic)
                    .map(|(_, c)| (c.id.clone(), c.start_us)),
            };
            found.ok_or_else(|| match &on_take {
                Some(id) => format!("Graphic {} does not play in short {id}", args.graphic),
                None => format!("Graphic {} is not on the timeline", args.graphic),
            })
        })
    };
    let target = match (args.move_to_us, &args.move_to_word, &take) {
        (Some(at), None, _) => Some(at),
        (None, Some(word), Some(short)) => Some(with_reader(ctx.state, |reader| {
            let voice = &short.take.as_ref().ok_or("No take")?.voice;
            let transcript = store::load_transcript(reader.root(), voice)?.ok_or_else(|| {
                format!("The take is not transcribed: `transcribe` with stream {voice}")
            })?;
            Ok(crate::take_short::words_at(short, &transcript, word, word)?.0)
        })?),
        (None, Some(word), None) => Some(word_edited_us(ctx, args.stream.clone(), word)?),
        (None, None, _) => None,
        _ => return Err("Give one of `moveToUs` or `moveToWord`".into()),
    };
    // Each change is its own step on the revision the one before left. One that still fails
    // after others went in says what did go in.
    let steps = (|| -> Result<(), String> {
        if let Some(target) = target {
            let now = current(ctx)?;
            let (clip, start) = clip_of(ctx)?;
            commands::project_sequence_edit_impl(
                ctx.state,
                now.handle,
                now.revision,
                crate::sequence::edit::SequenceEdit::MoveClips {
                    clip_ids: vec![clip],
                    delta_us: target as i64 - start as i64,
                    track_id: None,
                    anchor_id: None,
                },
                on_take.clone(),
            )?;
            lines.push(format!(
                "Moved graphic {} to {}.",
                args.graphic,
                clock(target)
            ));
        }
        if !args.variables.is_empty() || args.name.is_some() || args.style.is_some() {
            let now = current(ctx)?;
            let updated = commands::graphics::project_graphic_set_impl(
                ctx.state,
                now.handle,
                now.revision,
                args.graphic.clone(),
                args.variables.clone(),
                args.name.clone(),
                args.style.clone(),
            )?;
            lines.push(format!(
                "Graphic {} \"{}\": {}{}.",
                args.graphic,
                updated.graphic.name,
                serde_json::to_string(&updated.graphic.variables).unwrap_or_default(),
                match &updated.graphic.style {
                    Some(style) => format!(", style {style}"),
                    None => ", the project's style".into(),
                }
            ));
        }
        if !args.look.is_empty() {
            let now = current(ctx)?;
            let view = commands::graphics::project_graphic_look_set_impl(
                ctx.state,
                now.handle,
                args.graphic.clone(),
                args.look.clone(),
            )?;
            lines.push(if view.look.is_empty() {
                format!("Graphic {} takes its style's look.", args.graphic)
            } else {
                format!(
                    "Graphic {} has look tokens of its own: {}.",
                    args.graphic,
                    serde_json::to_string(&view.look).unwrap_or_default()
                )
            });
        }
        match (&args.short, args.slot) {
            (Some(wanted), Some(slot)) => {
                let now = current(ctx)?;
                let (title, shorts, size) = with_reader(ctx.state, |reader| {
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
                    let size = crate::shorts::graphic_box(document, short, slot);
                    Ok((short.title.clone(), shorts, size))
                })?;
                commands::project_shorts_set_impl(ctx.state, now.handle, now.revision, shorts)?;
                lines.push(format!(
                    "In short \"{title}\" it plays {}, in {}.",
                    slot.describe(),
                    crate::shorts::describe_box(size)
                ));
            }
            _ => {}
        }
        if let Some(length) = args.length_us {
            let now = current(ctx)?;
            let clip = clip_of(ctx)?;
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
                on_take.clone(),
            )?;
            lines.push(format!("It plays for {:.1} s.", length as f64 / 1e6));
        }
        Ok(())
    })();
    if let Err(error) = steps {
        if lines.is_empty() {
            return Err(error);
        }
        let done = applied(ctx, "graphic_set", &before, lines)?;
        return Err(format!("{done}\nThen it stopped: {error}"));
    }
    if lines.is_empty() {
        return Err(
            "Give `variables`, `name`, `lengthUs`, `moveToUs`/`moveToWord`, `brand`, or `short` \
             with `slot`"
                .into(),
        );
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
    /// The whole file's text (for a binary file, base64 with `base64: true`).
    #[serde(default)]
    pub contents: Option<String>,
    #[serde(default)]
    pub base64: bool,
    /// Or, to change part of the file in its folder: each `old` (found exactly once, as the
    /// file is now) becomes `new`, in order; all or none.
    #[serde(default)]
    pub edits: Vec<TextEdit>,
}

#[derive(Deserialize, JsonSchema)]
pub struct TextEdit {
    pub old: String,
    pub new: String,
}

/// `text` with every edit applied in order, each `old` found exactly once.
fn apply_edits(mut text: String, edits: &[TextEdit]) -> Result<String, String> {
    for (n, edit) in edits.iter().enumerate() {
        let found = if edit.old.is_empty() {
            0
        } else {
            text.matches(&edit.old).count()
        };
        if found != 1 {
            return Err(format!(
                "Edit {}: its `old` is found {found} times in the file, not once; nothing was \
                 written. Give more of the text around it.",
                n + 1
            ));
        }
        text = text.replacen(&edit.old, &edit.new, 1);
    }
    Ok(text)
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
    let bytes = match (args.contents, args.edits.is_empty()) {
        (Some(_), false) => return Err("Give `contents` or `edits`, not both".into()),
        (None, true) => return Err("Give the file's `contents`, or `edits` to change it".into()),
        (None, false) => {
            let now = crate::graphics::read_file(&dir, &args.path)?;
            apply_edits(now, &args.edits)?.into_bytes()
        }
        (Some(contents), true) if args.base64 => {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(contents.trim())
                .map_err(|e| format!("Not base64: {e}"))?
        }
        (Some(contents), true) => contents.into_bytes(),
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
    /// A short or clip (id or title): its graphics, rendered for its 9:16 frame or their part of its
    /// split.
    #[serde(default)]
    pub short: Option<String>,
}

fn graphic_render(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: GraphicRenderArgs = params(args)?;
    let short = args
        .short
        .as_deref()
        .map(|s| short_named(ctx, s))
        .transpose()?;
    let (root, specs) = with_reader(ctx.state, |reader| {
        // A short's graphics render on its own timeline at its own size, as its export does.
        let shown;
        let document = match &short {
            Some(short) => {
                shown = crate::shorts::short_document(
                    reader.document(),
                    short,
                    commands::has_speech_transcript(reader),
                )?;
                &shown
            }
            None => reader.document(),
        };
        let canvas = document.layout.preview_dimensions()?;
        let specs: Vec<_> =
            crate::graphics::render::specs_for(reader.root(), document, canvas, Default::default())
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
    for spec in &specs {
        lines.push(format!(
            "{} \"{}\" ({:.1} s): {} x {} (data-mode {}), {}",
            spec.asset_id,
            spec.name,
            spec.length_us as f64 / 1e6,
            spec.width,
            spec.height,
            spec.mode,
            match spec.entrance {
                crate::graphics::Entrance::Cut => "cuts in and out",
                crate::graphics::Entrance::Ease => "eases in and out over the full-screen camera",
            }
        ));
    }
    let started = std::time::Instant::now();
    crate::graphics::render::ensure(&specs, ctx.stop_flag(), &mut |fraction, label, _| {
        ctx.progress(fraction, label)
    })
    .map_err(|error| match ctx.stopped() {
        true => super::tools::STOPPED_MESSAGE.to_string(),
        false => error,
    })?;
    lines.push(format!(
        "{} rendered, each at its box's size, in {:.0} s; check them with frame_render inside each.",
        plural(specs.len(), "graphic"),
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
keeping its design and motion, so it takes any text, fits anywhere and takes the video's look:
1. Text: every visible text and number becomes a variable in data-composition-variables on \
<html> (type string, or number for counts), shown with data-var-text or set by the script from \
window.__hyperframes.getVariables(). Defaults are good example text; a list is one string with \
an item per line. No text written into the markup that no variable fills.
2. Colours: only the shared color variables background, text and accent (declare all three and \
use var(--background), var(--text), var(--accent)), so the user's styles apply. Tints come \
from color-mix() or opacity, not new hex colours.
3. Timing: read the length from the root's data-duration (parseFloat(root.dataset.duration) * \
1000) and place every animation as a share of it: when it eases, in over about 300 ms and out \
over the last 450 ms; nothing at fixed end times, so it fits any clip length.
4. Layout: vw/vh/vmin with flex or grid, no px sizes; text wraps (max-width, never nowrap) and \
long text still fits; an @media (orientation: portrait) block re-flows it for 9:16 shorts. \
The same block lays it out in a part of a short's split up to 3:2 wide (1080x864 at most), so it \
must fit there too.
5. In and out: the page is transparent, with a card (var(--background)). A graphic that fills \
the frame never eases in or out: the app sets data-entrance=\"cut\" on the root and the card \
shows whole from the first frame to the last. Only a graphic over the full-screen camera eases: \
the app sets data-entrance=\"ease\" and the card fades (or slides) in and out, so the first and \
last frames are see-through. So fade only when root.dataset.entrance === \"ease\". A graphic \
that covers only part of the frame (a lower third, a callout) puts data-overlay on the root; \
without it the app treats it as full screen and always cuts it. A graphic meant to hold to the \
end of its clip (an end card) puts data-holds-end on the root and, when it eases, fades in only.
6. No network: no CDN scripts, web fonts or http(s) URLs; images and fonts go beside index.html.
7. The look: copy the [data-composition-id] block of --look-* tokens from the blank template \n(graphic_add blank) and take type, corners, borders, shadows, gaps and motion from it: \nvar(--look-font), var(--look-title), var(--look-body), var(--look-radius), var(--look-shadow), \nvar(--look-line), var(--look-muted), var(--look-gap)...; durations and easing from \ngetComputedStyle(root) (--look-duration, --look-stagger, --look-ease, --look-gsap-ease for \nGSAP). No font, radius or timing of its own, so one change to the video's look restyles it.
Then graphic_check, which renders it at 16:9, 9:16 and a split part, 3 s and 8 s, eased and cut, with long \
text and with each text variable changed, and says what fails. Fix and check again until it passes; then ask \
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

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRemoveArgs {
    /// The library item's id or name (from `graphics`).
    pub item: String,
}

fn library_remove(_ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: LibraryRemoveArgs = params(args)?;
    let dir = crate::graphics::library::dir();
    let item = crate::graphics::library::list(&dir)
        .into_iter()
        .find(|i| i.id == args.item || i.item.name == args.item)
        .ok_or_else(|| format!("No library item {} (see graphics)", args.item))?;
    commands::graphics::graphics_library_remove_impl(item.id.clone())?;
    text(format!(
        "Removed {} ({}) from the library.",
        item.item.name, item.id
    ))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WordFix {
    /// The word's id, from transcript_get.
    pub word: String,
    /// What was said.
    pub text: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptFixArgs {
    pub fixes: Vec<WordFix>,
    /// Sound key of the transcript. Default: the transcribed speech.
    #[serde(default)]
    pub stream: Option<String>,
}

fn transcript_fix(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: TranscriptFixArgs = params(args)?;
    let open = opened(ctx)?;
    if args.fixes.is_empty() {
        return Err("Give `fixes`: `word` and `text` for each".into());
    }
    let key = pick_sound(ctx, args.stream, Want::Transcribed)?;
    let old = with_reader(ctx.state, |reader| {
        let transcript =
            store::load_transcript(reader.root(), &key)?.ok_or("That sound is not transcribed")?;
        args.fixes
            .iter()
            .map(|fix| {
                transcript
                    .words
                    .iter()
                    .find(|w| w.id == fix.word)
                    .map(|w| w.text.clone())
                    .ok_or_else(|| format!("No word {} in the transcript of {key}", fix.word))
            })
            .collect::<Result<Vec<_>, _>>()
    })?;
    // One load and one save for the whole batch: every fix goes in, or none does.
    let fixes: Vec<(String, String)> = args
        .fixes
        .iter()
        .map(|fix| (fix.word.clone(), fix.text.clone()))
        .collect();
    crate::commands::transcript::transcript_set_words_text_impl(
        ctx.state,
        &open.handle,
        &key,
        &fixes,
    )?;
    let lines: Vec<String> = args
        .fixes
        .iter()
        .zip(old)
        .map(|(fix, old)| format!("{}: \"{old}\" -> \"{}\"", fix.word, fix.text.trim()))
        .collect();
    ctx.changed_project();
    text(format!(
        "Fixed {}:\n{}",
        plural(lines.len(), "word"),
        lines.join("\n")
    ))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CaptionPick {
    /// The line's number, as listed.
    pub line: usize,
    /// The word as it reads in the line; left out clears the line's emphasis.
    #[serde(default)]
    pub word: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CaptionsArgs {
    /// Emphasis words to set; without these the lines are listed.
    #[serde(default)]
    pub emphasis: Vec<CaptionPick>,
    /// A new line starts at each `word` of `line`.
    #[serde(default)]
    pub split: Vec<CaptionPick>,
    /// These lines join the line before them.
    #[serde(default)]
    pub join: Vec<usize>,
    /// Hides `word` of `line` from the captions, or the whole line without `word`.
    #[serde(default)]
    pub hide: Vec<CaptionPick>,
    /// A short or clip (id or title): its caption lines.
    #[serde(default)]
    pub short: Option<String>,
    /// List only the lines from here (seconds on the edited timeline).
    #[serde(default)]
    pub from_s: Option<f64>,
    /// ...to here.
    #[serde(default)]
    pub to_s: Option<f64>,
}

/// A word as it reads, for matching what an agent wrote: letters and digits, lower case.
fn plain_word(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn captions(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: CaptionsArgs = params(args)?;
    let open = opened(ctx)?;
    let picked = args
        .short
        .as_deref()
        .map(|s| short_named(ctx, s))
        .transpose()?;
    let short = picked.as_ref().map(|s| s.id.clone());
    let track = commands::transcript::project_caption_cues_impl(
        ctx.state,
        open.handle.clone(),
        short.clone(),
    )?;
    let Some(track_id) = track.track_id.clone() else {
        return Err("No captions yet: nothing is transcribed.".into());
    };
    if args.emphasis.is_empty()
        && args.split.is_empty()
        && args.join.is_empty()
        && args.hide.is_empty()
    {
        let from = args.from_s.map(|s| (s.max(0.0) * 1e6) as u64).unwrap_or(0);
        let to = args
            .to_s
            .map(|s| (s.max(0.0) * 1e6) as u64)
            .unwrap_or(u64::MAX);
        let (settings, look) = with_reader(ctx.state, |reader| {
            let document = match &picked {
                Some(short) => crate::shorts::short_document(reader.document(), short, true)?,
                None => reader.document().clone(),
            };
            Ok((
                document.captions.clone(),
                crate::captions::CaptionLook::of(&document),
            ))
        })?;
        let mut lines = vec![format!(
            "{} caption lines ({}{}, {}; emphasis in {}, font {}).",
            track.cues.len(),
            if settings.enabled { "on" } else { "off" },
            if short.is_some() {
                " in this short"
            } else {
                ""
            },
            settings.style.label(),
            look.accent,
            look.face.label
        )];
        let mut without = 0;
        for (i, cue) in track.cues.iter().enumerate() {
            if cue.emphasis.is_none() {
                without += 1;
            }
            if cue.end_us <= from || cue.start_us >= to {
                continue;
            }
            let words: Vec<&str> = cue.text.split(' ').collect();
            let text = if words.len() == cue.word_ids.len() {
                cue.word_ids
                    .iter()
                    .zip(&words)
                    .map(|(id, w)| {
                        if cue.emphasis.as_ref() == Some(id) {
                            format!("*{w}*")
                        } else {
                            w.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                cue.text.clone()
            };
            let covered = if cue.under_graphic {
                " (a full-frame graphic covers it there: not shown)"
            } else {
                ""
            };
            lines.push(format!("{i} {} {text}{covered}", clock(cue.start_us)));
        }
        lines.push(format!("{without} without an emphasis word."));
        return text(lines.join("\n"));
    }
    // Each pick names a line and one of its words as listed now: all are found before any
    // change renumbers the lines.
    let line = |n: usize| {
        track.cues.get(n).ok_or_else(|| {
            format!(
                "There is no line {n} (there are {}); list them again",
                track.cues.len()
            )
        })
    };
    let word_of = |pick: &CaptionPick| -> Result<Option<String>, String> {
        let cue = line(pick.line)?;
        let Some(word) = pick
            .word
            .as_deref()
            .map(str::trim)
            .filter(|w| !w.is_empty())
        else {
            return Ok(None);
        };
        let wanted = plain_word(word);
        let index = cue
            .text
            .split(' ')
            .position(|w| plain_word(w) == wanted)
            .filter(|i| *i < cue.word_ids.len())
            .ok_or_else(|| {
                format!(
                    "Line {} (\"{}\") has no word \"{word}\"",
                    pick.line, cue.text
                )
            })?;
        Ok(Some(cue.word_ids[index].clone()))
    };
    use crate::commands::transcript::CaptionEdit;
    let mut edits = Vec::new();
    let mut on = Vec::new();
    let mut off = Vec::new();
    for pick in &args.emphasis {
        match word_of(pick)? {
            None => off.extend(line(pick.line)?.emphasis.clone()),
            Some(id) => on.push(id),
        }
    }
    for (word_ids, on) in [(off, false), (on, true)] {
        if !word_ids.is_empty() {
            edits.push(CaptionEdit::Emphasis { word_ids, on });
        }
    }
    for pick in &args.split {
        let word_id = word_of(pick)?.ok_or("`split` needs the word a new line starts at")?;
        edits.push(CaptionEdit::Split { word_id });
    }
    for &n in &args.join {
        let word_id = line(n)?
            .word_ids
            .first()
            .cloned()
            .ok_or("That line is empty")?;
        edits.push(CaptionEdit::Merge { word_id });
    }
    let mut hidden = Vec::new();
    for pick in &args.hide {
        match word_of(pick)? {
            Some(id) => hidden.push(id),
            None => hidden.extend(line(pick.line)?.word_ids.iter().cloned()),
        }
    }
    if !hidden.is_empty() {
        edits.push(CaptionEdit::Hide {
            word_ids: hidden,
            hidden: true,
        });
    }
    for edit in edits {
        commands::transcript::transcript_caption_edit_impl(
            ctx.state,
            open.handle.clone(),
            track_id.clone(),
            edit,
            short.clone(),
        )?;
    }
    ctx.changed_project();
    let said: Vec<String> = [
        ("emphasis set", args.emphasis.len()),
        ("split", args.split.len()),
        ("joined", args.join.len()),
        ("hidden", args.hide.len()),
    ]
    .into_iter()
    .filter(|(_, n)| *n > 0)
    .map(|(what, n)| format!("{} {what}", plural(n, "line")))
    .collect();
    let now = commands::transcript::project_caption_cues_impl(ctx.state, open.handle, short)?;
    text(format!(
        "Captions: {}; {} now (list them again for the new numbers).",
        said.join(", "),
        plural(now.cues.len(), "line")
    ))
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
    fn a_graphic_file_changes_in_place_by_edits() {
        let edit = |old: &str, new: &str| TextEdit {
            old: old.into(),
            new: new.into(),
        };
        let page = "<h1>Old title</h1><p>Old title below</p>".to_string();
        let done = apply_edits(
            page.clone(),
            &[edit("<h1>Old", "<h1>New"), edit("below", "under")],
        );
        assert_eq!(done.unwrap(), "<h1>New title</h1><p>Old title under</p>");
        // Ambiguous or missing text changes nothing.
        let twice = apply_edits(page.clone(), &[edit("Old title", "x")]).unwrap_err();
        assert!(
            twice.contains("2 times") && twice.contains("nothing was written"),
            "{twice}"
        );
        assert!(apply_edits(page, &[edit("", "x")]).is_err());
        // Reading for edits stays inside the graphic's folder.
        let dir = tempfile::tempdir().unwrap();
        crate::graphics::write_file(dir.path(), "index.html", b"<p>hi</p>").unwrap();
        assert_eq!(
            crate::graphics::read_file(dir.path(), "index.html").unwrap(),
            "<p>hi</p>"
        );
        assert!(crate::graphics::read_file(dir.path(), "../outside.html").is_err());
    }

    #[test]
    #[ignore]
    fn dump_graphics_answer() {
        let host = crate::mcp::tests_support::BareHost::default();
        let ctx = Ctx::new(&host, "claude-code", false);
        let out = call(&ctx, "graphics", json!({})).unwrap();
        eprintln!("LEN {}\n{out}", out.len());
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
    fn an_agent_run_is_undone_as_one_step_with_its_batches() {
        use crate::project::history::as_agent;
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);
        let agent =
            |name: &str, args: Value| as_agent("claude-code", name, || call(&ctx, name, args));
        let start = length(&ctx);
        agent("fillers", json!({})).unwrap();
        agent(
            "settings",
            json!({ "area": "captions", "set": { "enabled": true } }),
        )
        .unwrap();
        assert!(length(&ctx) < start);

        let undone = agent("undo", json!({ "run": true })).unwrap();
        assert!(
            undone.starts_with("Undid claude-code's run: 2 changes"),
            "{undone}"
        );
        assert_eq!(length(&ctx), start);
        assert!(history(&ctx)
            .iter()
            .all(|b| b.status == crate::proposals::Status::Undone));
        // The run undo is itself a step, so Undo brings the run back.
        let back = call(&ctx, "undo", json!({})).unwrap();
        assert!(
            back.starts_with("Undid \"Undo claude-code's run (2 changes)\""),
            "{back}"
        );
        assert!(length(&ctx) < start);
        // A new workflow starts a new run: only what came after `preset` goes.
        agent("preset", json!({})).unwrap();
        agent(
            "settings",
            json!({ "area": "captions", "set": { "fontSizePct": 7 } }),
        )
        .unwrap();
        let undone = agent("undo", json!({ "run": true })).unwrap();
        assert!(
            undone.starts_with("Undid claude-code's run: 1 change"),
            "{undone}"
        );
        assert!(length(&ctx) < start);
        // Not another agent's run.
        let other = as_agent("codex", "undo", || {
            call(&ctx, "undo", json!({ "run": true }))
        });
        assert!(other.is_err());
    }

    #[test]
    fn the_app_saves_a_checkpoint_before_an_agent_run_and_the_agent_can_go_back_to_it() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let mut ctx = Ctx::new(&host, "claude-code", false);
        let mut through_the_server = |name: &str, args: Value| {
            let tool = crate::mcp::tools::find(name).unwrap();
            crate::mcp::tools::call(tool, &mut ctx, args).map(|outputs| {
                outputs
                    .into_iter()
                    .filter_map(|o| match o {
                        Output::Text(text) => Some(text),
                        Output::Png(_) => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        };
        let start = through_the_server("status", json!({})).unwrap();
        assert!(!start.contains("Checkpoints:"), "{start}");
        through_the_server("fillers", json!({})).unwrap();
        through_the_server(
            "settings",
            json!({ "area": "captions", "set": { "enabled": true } }),
        )
        .unwrap();
        let listed = through_the_server("checkpoint", json!({ "op": "list" })).unwrap();
        // One for the run, not one per call.
        assert_eq!(listed.lines().count(), 1, "{listed}");
        assert!(
            listed.starts_with("c-1: \"Before claude-code's run\""),
            "{listed}"
        );

        let saved =
            through_the_server("checkpoint", json!({ "op": "save", "name": "Cleaned" })).unwrap();
        assert!(
            saved.starts_with("Saved checkpoint c-2 \"Cleaned\""),
            "{saved}"
        );
        let status = through_the_server("status", json!({})).unwrap();
        assert!(
            status.contains("Checkpoints: 2; newest named \"Cleaned\" (c-2"),
            "{status}"
        );

        let back =
            through_the_server("checkpoint", json!({ "op": "restore", "id": "c-1" })).unwrap();
        assert!(
            back.starts_with("Went back to checkpoint \"Before claude-code's run\"."),
            "{back}"
        );
        assert!(back.contains("Length 0:03"), "{back}");
        let captions = through_the_server("settings", json!({ "area": "captions" })).unwrap();
        assert!(captions.contains("\"enabled\":false"), "{captions}");
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
        assert!(undone.starts_with("Undid \"Cut "), "{undone}");
        assert_eq!(length(&ctx), before);
    }

    #[test]
    fn a_saved_review_that_found_nothing_is_not_asked_for_again() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, key) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);
        let page = call(&ctx, "fillers_review", json!({ "review": "thorough" })).unwrap();
        let stamp = review_stamp(&page);
        call(
            &ctx,
            "fillers_review",
            json!({ "cuts": [], "stamp": stamp }),
        )
        .unwrap();
        let dry = call(&ctx, "fillers", json!({ "apply": false })).unwrap();
        assert!(
            dry.starts_with(&format!("Found 1 filler and 0 retakes in {key}"))
                && dry.contains("the in-house review, saved, found nothing more")
                && !dry.contains("`fillers_review`"),
            "{dry}"
        );
    }

    #[test]
    fn an_agent_places_lists_changes_and_removes_its_own_zooms() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);
        // A picture as the screen, set with `roles` on import.
        let png = dir.path().join("screen.png");
        image::RgbaImage::from_pixel(64, 36, image::Rgba([40, 80, 120, 255]))
            .save(&png)
            .unwrap();
        let imported = call(
            &ctx,
            "media_import",
            json!({ "paths": [png.to_string_lossy()] }),
        )
        .unwrap();
        let media = imported
            .split_whitespace()
            .skip_while(|w| *w != "as")
            .nth(1)
            .unwrap()
            .to_string();
        let roles = call(
            &ctx,
            "media_import",
            json!({ "roles": [{ "media": media, "stream": "picture", "role": "screen" }] }),
        )
        .unwrap();
        assert!(roles.contains("Set the role of 1 stream"), "{roles}");
        let open = opened(&ctx).unwrap();
        commands::project_sequence_edit_impl(
            ctx.state,
            open.handle,
            open.revision,
            serde_json::from_value(json!({ "kind": "placeAsset", "assetId": media, "atUs": 0 }))
                .unwrap(),
            None,
        )
        .unwrap();

        let made = call(
            &ctx,
            "zooms",
            json!({ "op": "create", "startUs": 500_000, "endUs": 2_000_000, "scale": 1.8,
                    "centerX": 0.25, "centerY": 0.75 }),
        )
        .unwrap();
        assert!(made.starts_with("Added a 1.8x zoom"), "{made}");
        let listed = call(&ctx, "zooms", json!({ "op": "list" })).unwrap();
        assert!(listed.contains("1.8x at (0.25, 0.75), manual"), "{listed}");
        let id = listed
            .lines()
            .nth(1)
            .unwrap()
            .split(':')
            .next()
            .unwrap()
            .to_string();
        call(
            &ctx,
            "zooms",
            json!({ "op": "update", "id": id, "scale": 2.0, "fixed": true }),
        )
        .unwrap();
        let listed = call(&ctx, "zooms", json!({ "op": "list" })).unwrap();
        assert!(
            listed.contains("2.0x") && listed.contains("fixed"),
            "{listed}"
        );
        assert!(call(
            &ctx,
            "zooms",
            json!({ "op": "update", "id": id, "startUs": 0 })
        )
        .unwrap_err()
        .contains("remove"));
        assert!(call(
            &ctx,
            "zooms",
            json!({ "op": "create", "startUs": 0, "endUs": 9, "centerX": 2 })
        )
        .unwrap_err()
        .contains("0 to 1"));
        let removed = call(&ctx, "zooms", json!({ "op": "remove", "ids": [id] })).unwrap();
        assert!(removed.starts_with("Removed 1 zoom"), "{removed}");
        assert_eq!(
            call(&ctx, "zooms", json!({ "op": "list" })).unwrap(),
            "No zooms in the edit."
        );
        // Each change was one undo step.
        let undone = call(&ctx, "undo", json!({})).unwrap();
        assert!(undone.starts_with("Undid \"Zooms\"."), "{undone}");
        assert!(call(&ctx, "zooms", json!({ "op": "list" }))
            .unwrap()
            .contains("2.0x"));
    }

    #[test]
    fn an_agent_fixes_words_writes_chapters_and_edits_shorts() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);

        let fixed = call(
            &ctx,
            "transcript_fix",
            json!({ "fixes": [{ "word": "w-0", "text": "Hullo" }] }),
        )
        .unwrap();
        assert!(fixed.contains("w-0: \"Hello\" -> \"Hullo\""), "{fixed}");
        let words = call(&ctx, "transcript_get", json!({})).unwrap();
        assert!(
            words.contains("Hullo") && !words.contains("Hello"),
            "{words}"
        );
        assert!(call(
            &ctx,
            "transcript_fix",
            json!({ "fixes": [{ "word": "w-99", "text": "x" }] })
        )
        .unwrap_err()
        .contains("No word w-99"));

        let set = call(
            &ctx,
            "chapters",
            json!({ "set": [
                { "title": "Hello", "atWord": "w-0" },
                { "title": "Next step", "atUs": 1_600_000 },
            ] }),
        )
        .unwrap();
        assert!(set.starts_with("Set 2 chapters:"), "{set}");
        assert!(
            set.contains("0:00.00 Hello") && set.contains("0:01.60 Next step"),
            "{set}"
        );
        assert_eq!(history(&ctx)[0].action.as_deref(), Some("chapters"));
        assert!(call(&ctx, "chapters", json!({ "set": [{ "title": "x" }] }))
            .unwrap_err()
            .contains("give one of"));

        call(
            &ctx,
            "clips",
            json!({ "op": "create", "title": "Whole", "fromWord": "w-0", "toWord": "w-6", "scenes": false }),
        )
        .unwrap();
        let listed = call(&ctx, "clips", json!({})).unwrap();
        assert!(
            listed.starts_with("1 clip:") && listed.contains("\"Whole\""),
            "{listed}"
        );
        // Exports pick by kind: "clips" is every clip, "shorts" every short (none here).
        assert_eq!(chosen_shorts(&ctx, "clips").unwrap().len(), 1);
        assert_eq!(chosen_shorts(&ctx, "all").unwrap().len(), 1);
        assert!(chosen_shorts(&ctx, "shorts").is_err());
        // Shorts are the ones made from a take: listing them changes nothing and finds none.
        let revision = with_reader(ctx.state, |r| Ok(r.document().revision)).unwrap();
        let shorts = call(&ctx, "shorts", json!({})).unwrap();
        assert!(shorts.starts_with("No shorts made from a take"), "{shorts}");
        assert_eq!(
            with_reader(ctx.state, |r| Ok(r.document().revision)).unwrap(),
            revision
        );
        let wrong = call(&ctx, "shorts", json!({ "op": "remove", "short": "Whole" })).unwrap_err();
        assert!(
            wrong.contains("is a clip (cut from the video)") && wrong.contains("`clips`"),
            "{wrong}"
        );
        let ranged = call(
            &ctx,
            "clips",
            json!({ "op": "range", "clip": "Whole", "fromWord": "w-0", "toWord": "w-6" }),
        )
        .unwrap();
        assert!(ranged.contains("now plays w-0..w-6"), "{ranged}");
        assert!(
            call(&ctx, "clips", json!({ "op": "range", "clip": "Whole" }))
                .unwrap_err()
                .contains("Give `fromWord` and `toWord`")
        );
        let renamed = call(
            &ctx,
            "clips",
            json!({ "op": "rename", "clip": "whole", "title": "The whole lesson" }),
        )
        .unwrap();
        assert!(renamed.contains("to \"The whole lesson\""), "{renamed}");
        let removed = call(
            &ctx,
            "clips",
            json!({ "op": "remove", "clip": "The whole lesson" }),
        )
        .unwrap();
        assert!(
            removed.starts_with("Removed clip \"The whole lesson\""),
            "{removed}"
        );
        let shorts = with_reader(ctx.state, |r| Ok(r.document().shorts.len())).unwrap();
        assert_eq!(shorts, 0);
    }

    #[test]
    fn a_shorts_caption_breaks_are_its_own_and_undo() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);
        call(
            &ctx,
            "clips",
            json!({ "op": "create", "title": "Whole", "fromWord": "w-0", "toWord": "w-6", "scenes": false }),
        )
        .unwrap();
        let count = |short: bool| {
            let args = if short {
                json!({ "short": "Whole" })
            } else {
                json!({})
            };
            let listed = call(&ctx, "captions", args).unwrap();
            listed.split(' ').next().unwrap().parse::<usize>().unwrap()
        };
        let (video, short) = (count(false), count(true));
        // The first line's second word starts a new line, in the short only.
        let listed = call(&ctx, "captions", json!({ "short": "Whole" })).unwrap();
        let first = listed.lines().nth(1).unwrap();
        let word = first.split(' ').nth(3).unwrap().trim_matches('*');
        let split = call(
            &ctx,
            "captions",
            json!({ "short": "Whole", "split": [{ "line": 0, "word": word }] }),
        )
        .unwrap();
        assert!(split.starts_with("Captions: 1 line split"), "{split}");
        assert_eq!((count(false), count(true)), (video, short + 1));
        let step = with_reader(ctx.state, |r| {
            Ok(r.history().steps().undo_step().unwrap().label.clone())
        })
        .unwrap();
        assert_eq!(step, "Short captions");
        call(&ctx, "undo", json!({})).unwrap();
        assert_eq!(count(true), short);
    }

    #[test]
    fn an_agent_puts_graphics_under_a_take() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);
        // A graphic on the video, and a short made from a take (here the voice file alone).
        call(
            &ctx,
            "graphic_add",
            json!({ "template": "step", "atUs": 0, "lengthUs": 1_000_000 }),
        )
        .unwrap();
        let (video_graphic, voice) = with_reader(ctx.state, |reader| {
            let document = reader.document();
            let graphic = document
                .assets
                .iter()
                .find(|a| a.is_graphic())
                .unwrap()
                .id
                .clone();
            let voice = document
                .assets
                .iter()
                .find(|a| !a.is_graphic())
                .unwrap()
                .id
                .clone();
            Ok((graphic, voice))
        })
        .unwrap();
        call(
            &ctx,
            "short_from_take",
            json!({ "title": "Take", "media": [voice] }),
        )
        .unwrap();
        let listed = call(&ctx, "shorts", json!({})).unwrap();
        assert!(
            listed.starts_with("1 short made from a take:") && listed.contains("\"Take\""),
            "{listed}"
        );
        let wrong = call(
            &ctx,
            "clips",
            json!({ "op": "rename", "clip": "Take", "title": "x" }),
        )
        .unwrap_err();
        assert!(
            wrong.contains("is a short (made from a take)") && wrong.contains("`shorts`"),
            "{wrong}"
        );
        assert!(call(&ctx, "clips", json!({}))
            .unwrap()
            .starts_with("No clips yet"));
        // Everything under the take goes as one batch: a template, and a copy of the video's
        // own graphic with new text. Nothing changes until the user accepts it.
        let revision = |ctx: &Ctx| with_reader(ctx.state, |r| Ok(r.document().revision)).unwrap();
        let handle = with_reader(ctx.state, |r| Ok(r.summary.project_handle.clone())).unwrap();
        let batch_of = |said: &str| {
            said.split_whitespace()
                .find(|w| w.starts_with("b-"))
                .unwrap()
                .trim_end_matches('.')
                .to_string()
        };
        let before = revision(&ctx);
        let proposed = call(
            &ctx,
            "short_footage",
            json!({ "short": "Take", "op": "add", "reason": "Graphics for the take",
                    "items": [
                        { "atS": 0.5, "toS": 1.5, "template": "title", "mode": "overFace" },
                        { "atS": 2.0, "graphic": video_graphic,
                          "variables": { "title": "In the short" } }
                    ] }),
        )
        .unwrap();
        assert!(
            proposed.contains("Proposed b-")
                && proposed.contains("(template title)")
                && proposed.contains("a copy of the video's")
                && proposed.contains("over the face, which fills the frame"),
            "{proposed}"
        );
        assert_eq!(revision(&ctx), before, "proposing changes nothing");
        let graphics_made = |ctx: &Ctx| {
            with_reader(ctx.state, |r| {
                Ok(r.document()
                    .assets
                    .iter()
                    .filter(|a| a.is_graphic())
                    .count())
            })
            .unwrap()
        };
        assert_eq!(graphics_made(&ctx), 1, "the check leaves no graphic behind");
        let first = batch_of(&proposed);
        commands::proposals::proposal_accept_impl(
            ctx.state,
            handle.clone(),
            before,
            first.clone(),
            None,
            false,
        )
        .unwrap();
        assert_eq!(revision(&ctx), before + 1, "accepted as one undo step");
        // The title's mode came with it, set before it ever rendered.
        let modes = with_reader(ctx.state, |r| {
            Ok(r.document().shorts[0]
                .layout
                .graphic_slots
                .values()
                .copied()
                .collect::<Vec<_>>())
        })
        .unwrap();
        assert_eq!(modes, vec![crate::shorts::GraphicSlot::OverFace]);
        let (on_video, in_short, copy_title) = with_reader(ctx.state, |reader| {
            let document = reader.document();
            let graphics = |clips: Vec<&crate::sequence::Clip>| {
                clips
                    .iter()
                    .filter(|c| document.asset(&c.asset).is_some_and(|a| a.is_graphic()))
                    .count()
            };
            let short = document.shorts.iter().find(|s| s.title == "Take").unwrap();
            let own = &short.edit.as_ref().unwrap().sequence;
            // The copy is the one placed at 2 s.
            let copy = own
                .clips()
                .find(|(_, c)| c.start_us == 2_000_000)
                .map(|(_, c)| c.asset.clone())
                .unwrap();
            assert_ne!(copy, video_graphic);
            let source =
                crate::graphics::read_source(&crate::graphics::folder(reader.root(), &copy)).ok();
            Ok((
                graphics(document.sequence.clips().map(|(_, c)| c).collect()),
                graphics(own.clips().map(|(_, c)| c).collect()),
                source.and_then(|s| s.variables.get("title").cloned()),
            ))
        })
        .unwrap();
        assert_eq!(on_video, 1, "the video keeps its one graphic");
        assert_eq!(in_short, 2);
        // The copy took the new text; the video's graphic is its own.
        assert_eq!(copy_title, Some(json!("In the short")));
        // A batch made before the short changed can't be accepted as it is.
        let second = batch_of(
            &call(
                &ctx,
                "short_footage",
                json!({ "short": "Take", "op": "add",
                        "items": [{ "atS": 1.5, "toS": 2.0, "template": "key-point" }] }),
            )
            .unwrap(),
        );
        commands::proposals::proposal_undo_impl(ctx.state, handle.clone(), before + 1, first)
            .unwrap();
        let stale = commands::proposals::proposal_accept_impl(
            ctx.state,
            handle.clone(),
            revision(&ctx),
            second,
            None,
            false,
        )
        .unwrap_err();
        assert!(stale.contains("propose it again"), "{stale}");
        // Under autopilot it lands at once.
        let auto = Ctx::new(&host, "claude-code", true);
        let landed = call(
            &auto,
            "short_footage",
            json!({ "short": "Take", "op": "add",
                    "items": [{ "atS": 1.0, "template": "key-point" }] }),
        )
        .unwrap();
        assert!(landed.contains("autopilot accepted it"), "{landed}");
        // The footage and B-roll sound share one track: its volume and ducking are set once.
        let listed = call(&ctx, "short_footage", json!({ "short": "Take" })).unwrap();
        assert!(
            listed.contains("Footage sound: +0 dB, lowered 12 dB"),
            "{listed}"
        );
        let sound = call(
            &ctx,
            "short_footage",
            json!({ "short": "Take", "op": "sound", "volumeDb": -6, "duckDb": 18 }),
        )
        .unwrap();
        assert!(
            sound.contains("Footage sound: -6 dB, lowered 18 dB"),
            "{sound}"
        );
        assert!(call(
            &ctx,
            "short_footage",
            json!({ "short": "Take", "op": "sound", "duckDb": 50 })
        )
        .is_err());
        let ducked = with_reader(ctx.state, |reader| {
            let document = reader.document();
            let short = document.shorts.iter().find(|s| s.title == "Take").unwrap();
            let timeline = crate::shorts::short_timeline(document, short)?;
            let track = crate::take_short::footage_sound_track(short)
                .unwrap()
                .id
                .clone();
            Ok(timeline.audio.tracks[&track].clone())
        })
        .unwrap();
        assert_eq!((ducked.volume_db, ducked.duck_db), (-6.0, Some(18.0)));
        // graphic_add points to short_footage; a clip cut from the video takes neither.
        assert!(call(
            &ctx,
            "graphic_add",
            json!({ "short": "Take", "template": "title", "atUs": 0 })
        )
        .unwrap_err()
        .contains("short_footage"));
        call(
            &ctx,
            "clips",
            json!({ "op": "create", "title": "Clip", "fromWord": "w-0", "toWord": "w-6", "scenes": false }),
        )
        .unwrap();
        let refused = call(
            &ctx,
            "short_footage",
            json!({ "short": "Clip", "op": "add", "items": [{ "atS": 0.0, "template": "title" }] }),
        )
        .unwrap_err();
        assert!(
            refused.contains("is a clip (cut from the video)"),
            "{refused}"
        );
        assert!(call(
            &ctx,
            "graphic_add",
            json!({ "graphic": video_graphic, "atUs": 0 })
        )
        .is_err());

        // A graphic under the take moves and lengthens on the short's own timeline.
        let under = |ctx: &Ctx| {
            with_reader(ctx.state, |reader| {
                let document = reader.document();
                let short = document.shorts.iter().find(|s| s.title == "Take").unwrap();
                Ok(short
                    .edit
                    .as_ref()
                    .unwrap()
                    .sequence
                    .clips()
                    .find(|(_, c)| document.asset(&c.asset).is_some_and(|a| a.is_graphic()))
                    .map(|(_, c)| (c.asset.clone(), c.start_us, c.duration_us))
                    .unwrap())
            })
            .unwrap()
        };
        let (key_point, ..) = under(&ctx);
        let on_video = with_reader(ctx.state, |r| Ok(r.document().sequence.clone())).unwrap();
        let set = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": key_point, "short": "Take", "moveToUs": 300_000,
                    "lengthUs": 800_000 }),
        )
        .unwrap();
        assert!(
            set.contains("Moved graphic") && set.contains("0.8 s"),
            "{set}"
        );
        assert_eq!(under(&ctx), (key_point.clone(), 300_000, 800_000));
        let after = with_reader(ctx.state, |r| Ok(r.document().sequence.clone())).unwrap();
        assert_eq!(after, on_video, "the video's timeline is left alone");
        // A batch whose graphic would play over that one is refused, with the times.
        let overlap = call(
            &ctx,
            "short_footage",
            json!({ "short": "Take", "op": "add",
                    "items": [{ "atS": 0.5, "toS": 1.0, "template": "title" }] }),
        )
        .unwrap_err();
        assert!(
            overlap.contains("would overlap")
                && overlap.contains("play together at 0:00.50-0:01.00"),
            "{overlap}"
        );
    }

    #[test]
    fn an_agent_moves_and_removes_a_graphic() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);
        call(
            &ctx,
            "graphic_add",
            json!({ "template": "step", "atWord": "w-0", "lengthUs": 1_000_000 }),
        )
        .unwrap();
        let graphic = |ctx: &Ctx| {
            with_reader(ctx.state, |reader| {
                let document = reader.document();
                Ok(document.assets.iter().find(|a| a.is_graphic()).map(|a| {
                    let start = document
                        .sequence
                        .clips()
                        .find(|(_, c)| c.asset == a.id)
                        .map(|(_, c)| c.start_us);
                    (a.id.clone(), start)
                }))
            })
            .unwrap()
        };
        let (id, start) = graphic(&ctx).unwrap();
        assert_eq!(start, Some(0));
        let total = length(&ctx);
        let moved = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "moveToWord": "w-3" }),
        )
        .unwrap();
        assert!(moved.contains("Moved graphic"), "{moved}");
        assert_eq!(graphic(&ctx).unwrap().1, Some(1_600_000));
        assert_eq!(length(&ctx), total, "moving a graphic changes nothing else");
        // Moved and renamed in one call: both go in.
        let both = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "moveToWord": "w-0", "name": "Renamed" }),
        )
        .unwrap();
        assert!(
            both.contains("Moved graphic") && both.contains("\"Renamed\""),
            "{both}"
        );
        assert_eq!(graphic(&ctx).unwrap().1, Some(0));
        // A call with a wrong part changes nothing, not even the parts that were right.
        let wrong = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "moveToWord": "w-3", "style": "No Such Style" }),
        )
        .unwrap_err();
        assert!(wrong.contains("There is no style"), "{wrong}");
        assert_eq!(graphic(&ctx).unwrap().1, Some(0));
        let removed = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "remove": true }),
        )
        .unwrap();
        assert!(removed.starts_with("Removed graphic"), "{removed}");
        assert!(graphic(&ctx).is_none());
        assert_eq!(length(&ctx), total);
    }

    #[test]
    fn word_fixes_go_in_together_or_not_at_all() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "claude-code", false);
        let words = |ctx: &Ctx| {
            let key = pick_sound(ctx, None, Want::Transcribed).unwrap();
            with_reader(ctx.state, |reader| {
                let transcript = store::load_transcript(reader.root(), &key)?.unwrap();
                Ok(transcript
                    .words
                    .iter()
                    .take(2)
                    .map(|w| w.text.clone())
                    .collect::<Vec<_>>())
            })
            .unwrap()
        };
        let before = words(&ctx);
        let refused = call(
            &ctx,
            "transcript_fix",
            json!({ "fixes": [{ "word": "w-0", "text": "First" }, { "word": "w-1", "text": " " }] }),
        )
        .unwrap_err();
        assert!(refused.contains("cannot be empty"), "{refused}");
        assert_eq!(words(&ctx), before, "the good fix waits for the bad one");
        let fixed = call(
            &ctx,
            "transcript_fix",
            json!({ "fixes": [{ "word": "w-0", "text": "First" }, { "word": "w-1", "text": "Second" }] }),
        )
        .unwrap();
        assert!(fixed.starts_with("Fixed 2 words"), "{fixed}");
        assert_eq!(words(&ctx), ["First", "Second"]);
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
        assert!(
            added.contains("(template step) at 0:00.00 for 1.0 s"),
            "{added}"
        );
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

        // Styles: the graphic's own and the project's.
        let own = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "style": "Dark" }),
        )
        .unwrap();
        assert!(own.contains("style Dark"), "{own}");
        let missing = call(
            &ctx,
            "library",
            json!({ "kind": "style", "op": "use", "name": "No Such Style" }),
        )
        .unwrap_err();
        assert!(missing.contains("There is no style"), "{missing}");
        let project = call(
            &ctx,
            "library",
            json!({ "kind": "style", "op": "use", "name": "dark" }),
        )
        .unwrap();
        assert!(project.contains("dressed in style Dark"), "{project}");
        assert_eq!(history(&ctx)[0].action.as_deref(), Some("library"));
        let ours = with_reader(ctx.state, |r| Ok(r.document().style.clone())).unwrap();
        assert_eq!(ours, "Dark");
        let listed = call(&ctx, "graphics", json!({})).unwrap();
        assert!(listed.contains("\"style\":\"Dark\""), "{listed}");
        // The graphics rules come with the list, once, instead of in every edit's rules.
        assert!(listed.starts_with("# Graphics"), "{listed}");
        call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "style": "project" }),
        )
        .unwrap();

        // A style: shown, never changed when built in, copied and changed app-wide.
        let shown = call(&ctx, "library", json!({ "kind": "style", "op": "show" })).unwrap();
        assert!(
            shown.contains("\"accent\"") && shown.contains("radius"),
            "{shown}"
        );
        // The project wears Light: a change to it goes into a copy the project then wears.
        call(
            &ctx,
            "library",
            json!({ "kind": "style", "op": "use", "name": "Light" }),
        )
        .unwrap();
        let built_in = call(
            &ctx,
            "library",
            json!({ "kind": "style", "op": "set", "set": { "graphics": { "speed": 1.5 } } }),
        )
        .unwrap();
        assert!(built_in.contains("style My light"), "{built_in}");
        let ours = with_reader(ctx.state, |r| Ok(r.document().style.clone())).unwrap();
        assert_eq!(ours, "My light");
        assert_eq!(
            crate::style::store::load("Light").unwrap().graphics.speed,
            1.0
        );
        let bad = call(
            &ctx,
            "library",
            json!({ "kind": "style", "op": "set", "saveAs": "Mine", "set": { "graphics": { "speed": 9 } } }),
        )
        .unwrap_err();
        assert!(bad.contains("speed"), "{bad}");
        call(
            &ctx,
            "library",
            json!({ "kind": "style", "op": "set", "saveAs": "Mine", "set": { "graphics": { "preset": "calm" } } }),
        )
        .unwrap();
        call(
            &ctx,
            "library",
            json!({ "kind": "style", "op": "use", "name": "Mine" }),
        )
        .unwrap();
        let listed = call(&ctx, "library", json!({ "kind": "style" })).unwrap();
        assert!(listed.contains("\"name\": \"Mine\""), "{listed}");

        // A graphic's own tokens put it out of line: one batch brings it back.
        let own = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "look": { "radius": "0" } }),
        )
        .unwrap();
        assert!(own.contains("\"radius\":\"0\""), "{own}");
        let listed = call(&ctx, "graphics", json!({})).unwrap();
        assert!(listed.contains("\"look\":{\"radius\":\"0\"}"), "{listed}");
        let check = call(&ctx, "library", json!({ "kind": "style", "op": "check" })).unwrap();
        assert!(check.contains("Step card"), "{check}");
        let proposed = call(
            &ctx,
            "library",
            json!({ "kind": "style", "op": "bring_in_line" }),
        )
        .unwrap();
        assert!(
            proposed.contains("Proposed b-") && proposed.contains("its own look (radius)"),
            "{proposed}"
        );
        let batch = proposed
            .split_whitespace()
            .find(|w| w.starts_with("b-"))
            .unwrap()
            .trim_end_matches('.')
            .to_string();
        let (handle, revision) = with_reader(ctx.state, |r| {
            Ok((r.summary.project_handle.clone(), r.document().revision))
        })
        .unwrap();
        commands::proposals::proposal_accept_impl(ctx.state, handle, revision, batch, None, false)
            .unwrap();
        let in_line = with_reader(ctx.state, |reader| {
            let document = reader.document();
            let graphic = document.assets.iter().find(|a| a.is_graphic()).unwrap();
            let on_timeline = document
                .sequence
                .clips()
                .any(|(_, c)| c.asset == graphic.id);
            let source =
                crate::graphics::read_source(&crate::graphics::folder(reader.root(), &graphic.id))?;
            Ok((graphic.id.clone(), on_timeline, source.look.is_empty()))
        })
        .unwrap();
        assert_ne!(in_line.0, id, "a copy, so undo has the old one");
        assert!(in_line.1 && in_line.2, "{in_line:?}");
        assert!(call(
            &ctx,
            "library",
            json!({ "kind": "style", "op": "bring_in_line" })
        )
        .unwrap_err()
        .contains("nothing to bring in line"));
        let id = in_line.0;
        let slot = call(
            &ctx,
            "graphic_set",
            json!({ "graphic": id, "short": "nope", "slot": "camera" }),
        )
        .unwrap_err();
        assert!(slot.contains("No short or clip nope"), "{slot}");
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
    fn automatic_passes_hold_what_looks_wrong_and_graphics_go_in_one_call() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "codex", false);
        let before = length(&ctx);
        // Everything quieter than -10 dB "is silence": far more than a quarter, held.
        let held = call(
            &ctx,
            "silence",
            json!({ "guard": true, "thresholdDb": -10.0, "minDurationMs": 50 }),
        )
        .unwrap();
        assert!(held.contains("Not applied: it would cut"), "{held}");
        assert!(held.contains("more than a quarter"), "{held}");
        assert_eq!(length(&ctx), before, "a held pass changes nothing");
        // A pass that looks right applies in the same one call: muting, guarded by the words.
        let muted = call(&ctx, "mouth_noises", json!({ "guard": true })).unwrap();
        assert!(muted.contains("Muted 1."), "{muted}");

        // Two graphics in one call are one undo step.
        let revision = |ctx: &Ctx| with_reader(ctx.state, |r| Ok(r.document().revision)).unwrap();
        let at = revision(&ctx);
        let added = call(
            &ctx,
            "graphic_add",
            json!({ "items": [
                { "template": "title", "atWord": "w-0", "lengthUs": 1_000_000 },
                { "template": "key-point", "atUs": 1_500_000, "lengthUs": 1_000_000 }
            ] }),
        )
        .unwrap();
        assert_eq!(added.matches("Added graphic").count(), 2, "{added}");
        assert_eq!(revision(&ctx), at + 1, "{added}");
        let graphics = with_reader(ctx.state, |r| {
            Ok(r.document()
                .assets
                .iter()
                .filter(|a| a.is_graphic())
                .count())
        })
        .unwrap();
        assert_eq!(graphics, 2);
        // One bad item places none of them.
        let bad = call(
            &ctx,
            "graphic_add",
            json!({ "items": [
                { "template": "title", "atUs": 0 },
                { "template": "no-such-template", "atUs": 0 }
            ] }),
        )
        .unwrap_err();
        assert!(bad.starts_with("Graphic 2:"), "{bad}");
        assert_eq!(revision(&ctx), at + 1);
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
            report.starts_with(
                "Scan of \"Lesson\", 0:04.00 long, measured with the project's cleanup settings"
            ),
            "{report}"
        );
        has(&format!("Microphones: 1, {key} (transcribed)."));
        has("Cameras: none, so no webcam questions.");
        has("Mouse data: none");
        has("Transcribers: online by default");
        has("AI key: ");
        has("Mouth noises: Found 1 mouth noise");
        has("1 would be muted.");
        // The click is not muted yet, so it splits the first pause.
        has("Silence from the audio: Found 3 pauses");
        has("Gaps from the transcript: Found ");
        // The scan's dry run never spends the AI key, and says the review finds more.
        has("Fillers and retakes (the rules, plus any saved review): Found 1 filler and 0 retakes");
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
        assert!(call(&ctx, "zooms", json!({ "apply": false, "op": "dismiss" })).is_err());
        // A dry run tries other settings without saving them.
        let before = call(&ctx, "settings", json!({ "area": "zoom" })).unwrap();
        let tried = call(
            &ctx,
            "zooms",
            json!({ "apply": false, "settings": { "minHoldMs": 7000, "clickScale": 2.0 } }),
        )
        .unwrap();
        assert!(tried.contains("Nothing was saved"), "{tried}");
        assert_eq!(
            call(&ctx, "settings", json!({ "area": "zoom" })).unwrap(),
            before
        );
        assert!(call(
            &ctx,
            "zooms",
            json!({ "apply": false, "settings": { "nope": 1 } })
        )
        .is_err());
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
        let presets = call(&ctx, "settings", json!({ "area": "cleanup" })).unwrap();
        assert!(
            presets.contains("\"preset\":\"tight\"") && presets.contains("\"thresholdDb\":-36.0"),
            "{presets}"
        );
        let export = call(&ctx, "settings", json!({ "area": "export" })).unwrap();
        assert!(export.contains("\"resolution\":\"1080p\""), "{export}");
        // The project's cleanup and export settings can be changed, one undo step each.
        let set = call(
            &ctx,
            "settings",
            json!({ "area": "cleanup", "set": { "silence": { "paddingMs": 60 } } }),
        )
        .unwrap();
        assert!(set.contains("\"paddingMs\":60"), "{set}");
        let set = call(
            &ctx,
            "settings",
            json!({ "area": "export", "set": { "resolution": "4K" } }),
        )
        .unwrap();
        assert!(set.contains("\"resolution\":\"4K\""), "{set}");
        assert!(call(
            &ctx,
            "settings",
            json!({ "area": "export", "set": { "fps": 7 } })
        )
        .is_err());
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
    fn an_agent_makes_a_short_and_sets_its_layouts() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        let made = call(
            &ctx,
            "clips",
            json!({ "op": "create", "title": "Next step", "fromWord": "w-0", "toWord": "w-6", "reason": "A tip" }),
        )
        .unwrap();
        // The voice is a file, not a recording: there is no mouse data to pick layouts from.
        assert!(made.contains("Made clip clip-0 \"Next step\""), "{made}");
        assert!(made.contains("Layouts not picked"), "{made}");
        assert!(call(
            &ctx,
            "clips",
            json!({ "op": "create", "title": "Too short", "fromWord": "w-0", "toWord": "w-1" })
        )
        .unwrap_err()
        .contains("a clip is 3 s to 3 min"));

        let set = call(
            &ctx,
            "layouts",
            json!({ "short": "Next step", "op": "add", "sections": [{ "fromS": 0, "toS": 1.5, "layout": "Camera only" }] }),
        )
        .unwrap();
        assert!(
            set.contains("s1 0:00.00-0:01.50 Camera only: Camera only (100 %) (by hand)"),
            "{set}"
        );
        let listed = call(&ctx, "layouts", json!({ "short": "clip-0" })).unwrap();
        assert!(
            listed.contains("base Side by side (camera first, 35 %)"),
            "{listed}"
        );
        assert!(
            listed.contains("Built-in layouts: Camera bubble"),
            "{listed}"
        );
        assert!(call(
            &ctx,
            "layouts",
            json!({ "short": "clip-0", "op": "add", "sections": [{ "fromS": 0, "toS": 1, "layout": "Nope" }] }),
        )
        .unwrap_err()
        .contains("No layout \"Nope\""));
        let cleared = call(&ctx, "layouts", json!({ "short": "clip-0", "op": "clear" })).unwrap();
        assert!(
            cleared.contains("No sections: the base shows throughout."),
            "{cleared}"
        );
    }

    #[test]
    fn subtitles_transcript_and_sound_are_saved_next_to_the_project() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        let saved = call(&ctx, "export", json!({ "file": "srt" })).unwrap();
        assert!(saved.starts_with("The video: "), "{saved}");
        let srt = std::fs::read_to_string(dir.path().join("Lesson.srt")).unwrap();
        assert!(
            srt.starts_with("1\r\n00:00:00,000 --> 00:00:01,190\r\nHello um there.\r\n\r\n2\r\n00:00:01,600 --> "),
            "{srt}"
        );
        call(&ctx, "export", json!({ "file": "md" })).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("Lesson.md")).unwrap(),
            "# Lesson\n\nHello um there. So next step now.\n"
        );
        call(&ctx, "export", json!({ "file": "wav" })).unwrap();
        let wav = std::fs::read(dir.path().join("Lesson.wav")).unwrap();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(wav.len(), 44 + 4 * 48_000 * 4);
        assert!(call(&ctx, "export", json!({ "file": "srt", "short": "x" }))
            .unwrap_err()
            .contains("No shorts or clips yet"));
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
