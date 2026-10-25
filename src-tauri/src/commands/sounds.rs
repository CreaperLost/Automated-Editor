//! The sound effects library's commands (SND-3): browse, hear, add the user's own, and place
//! one in the open project as one undo step.
use super::{require_handle, AppState};
use crate::project::revision::EditDocument;
use crate::project::OpenedProject;
use crate::sequence::Licence;
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
    use base64::Engine;
    let sound = library::find(&library::dir(), &id)?;
    let path = Path::new(&sound.path);
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > MAX_PREVIEW_BYTES {
        return Err("The sound file is too big to preview".into());
    }
    let mime = match path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .as_deref()
    {
        Some("flac") => "audio/flac",
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
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
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let root = reader.root().to_path_buf();
    let (asset, new) = library::asset_for(reader.document(), &root, &sound)?;
    let asset_id = asset.id.clone();
    let placed = on_shown(reader.document(), short_id.as_deref(), |shown, taken| {
        library::place(shown, asset.clone(), at_us, taken)
    })
    .and_then(|(next, clip_id)| {
        let project = reader.commit_document(expected_revision, next)?;
        let project = match short_id.as_deref() {
            Some(id) => reader.short_view(id)?,
            None => project,
        };
        Ok((project, clip_id))
    });
    let (project, clip_id) = match placed {
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
    Ok(SoundPlaced {
        project,
        asset_id,
        clip_id,
    })
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
}
