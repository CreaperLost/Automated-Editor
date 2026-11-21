//! The sound effects library's commands (SND-3): browse, hear, add the user's own, and place
//! one in the open project as one undo step.
use super::{require_handle, AppState};
use crate::project::revision::EditDocument;
use crate::project::OpenedProject;
use crate::sequence::{Asset, Licence};
use crate::sound_library::{self as library, Sound};
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::Ordering;

/// Biggest file the preview hands the window whole.
const MAX_PREVIEW_BYTES: u64 = 16 * 1024 * 1024;

pub fn sound_library_list_impl(query: Option<String>) -> Result<Vec<Sound>, String> {
    let sounds = library::list(&library::dir())?;
    Ok(library::search(sounds, query.as_deref().unwrap_or("")))
}

/// A sound as a data URL for the window to play.
pub fn sound_library_preview_impl(id: String) -> Result<String, String> {
    let sound = library::find(&library::dir(), &id)?;
    audio_data_url(
        Path::new(&sound.path),
        MAX_PREVIEW_BYTES,
        "The sound file is too big to preview",
    )
}

/// A sound file as a data URL for the window to play.
pub(crate) fn audio_data_url(path: &Path, max_bytes: u64, too_big: &str) -> Result<String, String> {
    use base64::Engine;
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > max_bytes {
        return Err(too_big.into());
    }
    let mime = match path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .as_deref()
    {
        Some("flac") => "audio/flac",
        Some("mp3") => "audio/mpeg",
        Some("ogg") | Some("opus") => "audio/ogg",
        Some("m4a") | Some("aac") => "audio/mp4",
        _ => "audio/wav",
    };
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

pub fn sound_library_add_impl(
    path: String,
    name: Option<String>,
    tags: Vec<String>,
    licence: Licence,
) -> Result<Sound, String> {
    library::add(&library::dir(), Path::new(&path), name, tags, licence)
}

pub fn sound_library_update_impl(
    id: String,
    name: Option<String>,
    tags: Option<Vec<String>>,
    licence: Option<Licence>,
) -> Result<Sound, String> {
    library::update(&library::dir(), &id, name, tags, licence)
}

pub fn sound_library_remove_impl(id: String) -> Result<(), String> {
    library::remove(&library::dir(), &id)
}

/// Runs `change` on the timeline shown: the video's, or short `short_id`'s own, which then keeps
/// the result as its own edit (as any edit made on it does). `change` gets the track ids already
/// in use anywhere in the project. Answers the whole next document.
pub(crate) fn on_shown<T>(
    document: &EditDocument,
    short_id: Option<&str>,
    change: impl FnOnce(&EditDocument, &[String]) -> Result<(EditDocument, T), String>,
) -> Result<(EditDocument, T), String> {
    let taken: Vec<String> = document.track_ids_in_use().map(String::from).collect();
    let Some(id) = short_id else {
        return change(document, &taken);
    };
    let short = document
        .shorts
        .iter()
        .find(|s| s.id == id)
        .ok_or("That short no longer exists")?;
    let timeline = crate::shorts::short_timeline(document, short)?;
    let (changed, out) = change(&timeline, &taken)?;
    let mut next = document.clone();
    next.assets = changed.assets;
    next.audio = changed.audio;
    let own = next
        .shorts
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or("That short no longer exists")?;
    let noise_mutes = own.edit.take().map(|e| e.noise_mutes).unwrap_or_default();
    own.edit = Some(crate::shorts::ShortEdit {
        sequence: changed.sequence,
        noise_mutes,
    });
    Ok((next, out))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundPlaced {
    pub project: OpenedProject,
    pub asset_id: String,
    pub clip_id: String,
}

/// Puts library sound `sound_id` at `at_us` on the timeline shown (the video's, or short
/// `short_id`'s), on an effects track: one undo step, importing it into the project (with its
/// licence) the first time.
pub fn project_sound_place_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    sound_id: String,
    at_us: u64,
    short_id: Option<String>,
) -> Result<SoundPlaced, String> {
    let sound = library::find(&library::dir(), &sound_id)?;
    let (project, asset_id, clip_id) = place_on_shown(
        state,
        &project_handle,
        expected_revision,
        short_id.as_deref(),
        |document, root| library::asset_for(document, root, &sound),
        |shown, asset, taken| library::place(shown, asset, at_us, taken),
    )?;
    Ok(SoundPlaced {
        project,
        asset_id,
        clip_id,
    })
}

/// What placing sounds at the graphics' moments found, and the project when it changed.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicCuesPlaced {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<OpenedProject>,
    /// How many graphic moments the video has.
    pub moments: usize,
    pub planned: Vec<crate::graphic_cues::Planned>,
    /// Compositions whose `data-cues` couldn't be read.
    pub problems: Vec<String>,
}

/// Sounds from the style's sound set (`cues` replace its roles for this call) at the video's
/// graphic moments ([`crate::graphic_cues`]); `apply` false only plans. Placing is one undo
/// step that replaces the cues placed before.
pub fn project_graphic_cues_impl(
    state: &AppState,
    project_handle: &str,
    expected_revision: u64,
    cues: Vec<crate::style::Cue>,
    apply: bool,
) -> Result<GraphicCuesPlaced, String> {
    use crate::graphic_cues as gc;
    use crate::sequence::edit::{apply_reserving, SequenceEdit};
    use crate::sequence::{Role, SourceRange};
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, project_handle)?;
    let root = reader.root().to_path_buf();
    let document = reader.document().clone();
    let mut set = crate::style::of(&document).sound.cues.clone();
    for cue in cues {
        set.retain(|c| !c.role.eq_ignore_ascii_case(&cue.role));
        set.push(cue);
    }
    let problems = std::cell::RefCell::new(Vec::new());
    let moments = gc::moments(&document, |id| {
        let html = crate::graphics::read_file(&crate::graphics::folder(&root, id), "index.html");
        match html.map(|h| gc::marks(&h)) {
            Ok(Ok(marks)) => marks,
            Ok(Err(e)) => {
                problems.borrow_mut().push(format!("{id}: {e}"));
                Vec::new()
            }
            Err(_) => Vec::new(),
        }
    });
    let planned = gc::plan(&moments, &set);
    let mut out = GraphicCuesPlaced {
        project: None,
        moments: moments.len(),
        planned,
        problems: problems.into_inner(),
    };
    let existing = document
        .sequence
        .audio_tracks()
        .find(|t| t.name == gc::TRACK_NAME && !t.locked)
        .map(|t| t.id.clone());
    if !apply || (out.planned.is_empty() && existing.is_none()) {
        return Ok(out);
    }
    let taken: Vec<String> = document.track_ids_in_use().map(String::from).collect();
    let mut next = document.clone();
    let track_id = match existing {
        Some(id) => {
            if let Some(track) = next.sequence.tracks.iter_mut().find(|t| t.id == id) {
                track.clips.clear();
            }
            id
        }
        None => {
            let (sequence, id) = library::add_track(
                &next,
                Role::Effects,
                gc::TRACK_NAME,
                taken.iter().map(String::as_str),
            )?;
            next.sequence = sequence;
            id
        }
    };
    // Each sound once: its media in the project and where its loudest moment is.
    let lib = library::dir();
    let mut imported: Vec<Asset> = Vec::new();
    let mut sounds: std::collections::BTreeMap<String, (String, u64, u64)> = Default::default();
    let mut placed_ok = || -> Result<(), String> {
        for p in &out.planned {
            if sounds.contains_key(&p.sound) {
                continue;
            }
            let sound = library::find(&lib, &p.sound)?;
            let (asset, new) = library::asset_for(&next, &root, &sound)?;
            let peak = crate::media::decode_pcm(Path::new(&sound.path), 48_000 * 10)
                .map(|b| gc::peak_offset_us(&b.samples, b.channels, b.sample_rate))
                .unwrap_or(0);
            sounds.insert(
                p.sound.clone(),
                (asset.id.clone(), asset.duration_us.max(1), peak),
            );
            if new {
                imported.push(asset.clone());
            }
            if next.asset(&asset.id).is_none() {
                next.assets.push(asset);
            }
        }
        let starts: Vec<(u64, &str)> = out
            .planned
            .iter()
            .map(|p| {
                let peak = sounds[&p.sound].2;
                (p.moment.at_us.saturating_sub(peak), p.sound.as_str())
            })
            .collect();
        let length_before = next.sequence.duration_us();
        for (i, &(at, sound)) in starts.iter().enumerate() {
            let (asset_id, length, _) = sounds[sound].clone();
            let room = starts
                .get(i + 1)
                .map_or(u64::MAX, |n| n.0.saturating_sub(at));
            let length = length.min(room);
            if length == 0 {
                continue;
            }
            let before: Vec<String> = next.sequence.clips().map(|(_, c)| c.id.clone()).collect();
            let (placed, _) = apply_reserving(
                &next.sequence,
                &next.assets,
                &SequenceEdit::PlaceAsset {
                    asset_id: asset_id.clone(),
                    at_us: at,
                    track_id: Some(track_id.clone()),
                    streams: Vec::new(),
                    range: Some(SourceRange {
                        start_us: 0,
                        end_us: length,
                    }),
                },
                taken.iter().map(String::as_str),
            )?;
            if placed.duration_us() > length_before.max(at + length) {
                return Err("Placing the cues would have moved the video".into());
            }
            next.sequence = placed;
            if length == room {
                let fresh = next
                    .sequence
                    .tracks
                    .iter_mut()
                    .flat_map(|t| t.clips.iter_mut())
                    .find(|c| c.asset == asset_id && !before.contains(&c.id));
                if let Some(clip) = fresh {
                    clip.mix.fade_out_us = gc::TAIL_FADE_US.min(length / 2);
                }
            }
        }
        next.audio
            .tracks
            .entry(track_id.clone())
            .or_default()
            .duck_db
            .get_or_insert(gc::DUCK_DB);
        Ok(())
    };
    let result = placed_ok().and_then(|()| reader.commit_document(expected_revision, next));
    match result {
        Ok(project) => {
            out.project = Some(project);
            Ok(out)
        }
        Err(error) => {
            for asset in &imported {
                crate::media_bin::remove_files(&root, asset);
            }
            Err(error)
        }
    }
}

/// Puts a library file on the timeline shown as one undo step, importing it into the project
/// the first time (and taking an import away again when placing fails). `asset_for` finds or
/// imports it. Answers the project, the asset's id and what `place` says.
pub(crate) fn place_on_shown<T>(
    state: &AppState,
    project_handle: &str,
    expected_revision: u64,
    short_id: Option<&str>,
    asset_for: impl FnOnce(&EditDocument, &Path) -> Result<(Asset, bool), String>,
    place: impl FnOnce(&EditDocument, Asset, &[String]) -> Result<(EditDocument, T), String>,
) -> Result<(OpenedProject, String, T), String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, project_handle)?;
    let root = reader.root().to_path_buf();
    let (asset, new) = asset_for(reader.document(), &root)?;
    let asset_id = asset.id.clone();
    let placed = on_shown(reader.document(), short_id, |shown, taken| {
        place(shown, asset.clone(), taken)
    })
    .and_then(|(next, out)| {
        let project = reader.commit_document(expected_revision, next)?;
        let project = match short_id {
            Some(id) => reader.short_view(id)?,
            None => project,
        };
        Ok((project, out))
    });
    let (project, out) = match placed {
        Ok(done) => done,
        Err(error) => {
            if new {
                crate::media_bin::remove_files(&root, &asset);
            }
            return Err(error);
        }
    };
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    Ok((project, asset_id, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sound_lands_on_an_effects_track_and_is_imported_once() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let state = &host.state;
        let (handle, revision) = {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().unwrap();
            (
                reader.summary.project_handle.clone(),
                reader.summary.revision,
            )
        };
        let length = |state: &AppState| {
            state
                .opened_project
                .lock()
                .as_ref()
                .unwrap()
                .document()
                .duration_us()
        };
        let before = length(state);
        let first =
            project_sound_place_impl(state, handle.clone(), revision, "pop".into(), 500_000, None)
                .unwrap();
        let second = project_sound_place_impl(
            state,
            handle,
            first.project.revision,
            "pop".into(),
            1_000_000,
            None,
        )
        .unwrap();
        assert_eq!(first.asset_id, second.asset_id, "imported once");
        assert_eq!(length(state), before, "nothing moved");
        let opened = state.opened_project.lock();
        let document = opened.as_ref().unwrap().document();
        let asset = document.asset(&first.asset_id).unwrap();
        assert_eq!(
            asset.licence.as_ref().map(|l| l.kind),
            Some(crate::sequence::LicenceKind::Cc0)
        );
        assert!(asset
            .streams
            .iter()
            .all(|s| s.role == crate::sequence::Role::Effects));
        let (track, clip) = document.sequence.clip(&second.clip_id).unwrap();
        assert_eq!(clip.start_us, 1_000_000);
        assert_eq!(track.name, library::TRACK_NAME);
        assert_eq!(track.role, Some(crate::sequence::Role::Effects));
        // Both on the one track: the pop is short, so the second fits after the first.
        assert_eq!(track.clips.len(), 2);
    }

    #[test]
    fn a_sound_placed_while_a_short_shows_goes_into_the_short() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let state = &host.state;
        let (handle, revision, recording) = {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().unwrap();
            let recording = reader
                .document()
                .sequence
                .clips()
                .next()
                .unwrap()
                .1
                .asset
                .clone();
            (
                reader.summary.project_handle.clone(),
                reader.summary.revision,
                recording,
            )
        };
        let short = crate::shorts::Short {
            id: "s1".into(),
            title: "Best bit".into(),
            source_start_us: 0,
            source_end_us: 2_000_000,
            reason: String::new(),
            layout: Default::default(),
            layouts: crate::layouts::LayoutTrack::for_short(),
            media: Some(recording),
            edit: None,
            take: None,
            caption_marks: Default::default(),
            idea: String::new(),
            pieces: Vec::new(),
            length_us: None,
            edited_start_us: None,
            edited_end_us: None,
        };
        let project =
            super::super::project_shorts_set_impl(state, handle.clone(), revision, vec![short])
                .unwrap();
        let video_clips = |state: &AppState| {
            let opened = state.opened_project.lock();
            opened.as_ref().unwrap().document().sequence.clips().count()
        };
        let before = video_clips(state);
        let placed = project_sound_place_impl(
            state,
            handle,
            project.revision,
            "pop".into(),
            500_000,
            Some("s1".into()),
        )
        .unwrap();
        assert_eq!(placed.project.short_view.as_deref(), Some("s1"));
        assert!(placed.project.sequence.clip(&placed.clip_id).is_some());
        assert_eq!(video_clips(state), before, "the video is as it was");
        let opened = state.opened_project.lock();
        let document = opened.as_ref().unwrap().document();
        let own = document.shorts[0]
            .edit
            .as_ref()
            .expect("the short keeps its own edit");
        assert!(own.sequence.clip(&placed.clip_id).is_some());
        assert!(document.asset(&placed.asset_id).is_some());
    }

    #[test]
    fn cues_land_on_a_graphics_moments_on_their_own_ducked_track_and_replace_the_last() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let state = &host.state;
        let (handle, revision, root) = {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().unwrap();
            (
                reader.summary.project_handle.clone(),
                reader.summary.revision,
                reader.root().to_path_buf(),
            )
        };
        let created = crate::commands::graphics::project_graphic_create_impl(
            state,
            handle.clone(),
            revision,
            "blank".into(),
            Some("Card".into()),
            serde_json::Map::new(),
            0,
            Some(3_000_000),
        )
        .unwrap();
        // The composition marks a pop 1.5 s in.
        let folder = crate::graphics::folder(&root, &created.asset_id);
        let html = crate::graphics::read_file(&folder, "index.html").unwrap();
        let marked = crate::graphics::with_root_attribute(&html, "data-cues", "1.5:pop");
        std::fs::write(folder.join("index.html"), marked).unwrap();
        let set = vec![
            crate::style::Cue {
                role: "in".into(),
                sound: "whoosh-soft".into(),
            },
            crate::style::Cue {
                role: "pop".into(),
                sound: "pop".into(),
            },
        ];
        let revision = created.project.revision;
        let dry = project_graphic_cues_impl(state, &handle, revision, set.clone(), false).unwrap();
        assert!(dry.project.is_none());
        let got: Vec<(u64, &str)> = dry
            .planned
            .iter()
            .map(|p| (p.moment.at_us, p.sound.as_str()))
            .collect();
        assert_eq!(got, vec![(0, "whoosh-soft"), (1_500_000, "pop")]);

        let length_before = state
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .document()
            .duration_us();
        let placed = project_graphic_cues_impl(state, &handle, revision, set.clone(), true)
            .unwrap()
            .project
            .unwrap();
        let again = project_graphic_cues_impl(state, &handle, placed.revision, set, true)
            .unwrap()
            .project
            .unwrap();
        let opened = state.opened_project.lock();
        let document = opened.as_ref().unwrap().document();
        assert_eq!(document.duration_us(), length_before, "nothing moved");
        let tracks: Vec<_> = document
            .sequence
            .audio_tracks()
            .filter(|t| t.name == crate::graphic_cues::TRACK_NAME)
            .collect();
        assert_eq!(tracks.len(), 1, "one track, used again");
        let track = tracks[0];
        assert_eq!(track.clips.len(), 2, "placing again replaced the first");
        assert_eq!(
            document.audio.tracks[&track.id].duck_db,
            Some(crate::graphic_cues::DUCK_DB)
        );
        // The pop's loudest moment is on the mark: it starts a little before it.
        let pop = &track.clips[1];
        assert!(
            pop.start_us <= 1_500_000 && pop.start_us > 1_000_000,
            "{}",
            pop.start_us
        );
        assert!(again.revision > placed.revision);
    }
}
