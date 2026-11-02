//! The music library's commands (SND-3): browse, hear (downloading a built-in track the first
//! time), add the user's own (refused unless it needs no credit), and place one under the video
//! in the open project as one undo step.
use super::AppState;
use crate::music_library::{self as library, Bed, Track};
use crate::project::OpenedProject;
use crate::sequence::Licence;
use serde::Serialize;
use std::path::Path;

/// Biggest file the preview hands the window whole.
const MAX_PREVIEW_BYTES: u64 = 24 * 1024 * 1024;

pub fn music_library_list_impl(query: Option<String>) -> Result<Vec<Track>, String> {
    let tracks = library::list(&library::dir());
    Ok(library::search(tracks, query.as_deref().unwrap_or("")))
}

/// The track on disk, downloading a built-in one the first time.
pub fn music_library_fetch_impl(id: String) -> Result<Track, String> {
    let root = library::dir();
    let track = library::find(&root, &id)?;
    library::ensure(&root, &track)
}

/// A track as a data URL for the window to play.
pub fn music_library_preview_impl(id: String) -> Result<String, String> {
    let track = music_library_fetch_impl(id)?;
    super::sounds::audio_data_url(
        Path::new(&track.path),
        MAX_PREVIEW_BYTES,
        "The track is too big to preview here; add it to hear it",
    )
}

pub fn music_library_add_impl(
    path: String,
    name: Option<String>,
    tags: Vec<String>,
    licence: Licence,
) -> Result<Track, String> {
    library::add(&library::dir(), Path::new(&path), name, tags, licence)
}

pub fn music_library_update_impl(
    id: String,
    name: Option<String>,
    tags: Option<Vec<String>>,
    licence: Option<Licence>,
) -> Result<Track, String> {
    library::update(&library::dir(), &id, name, tags, licence)
}

pub fn music_library_remove_impl(id: String) -> Result<(), String> {
    library::remove(&library::dir(), &id)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicPlaced {
    pub project: OpenedProject,
    pub asset_id: String,
    #[serde(flatten)]
    pub bed: Bed,
}

/// Plays library track `track_id` under `[at_us, until_us)` of the timeline shown (the video's,
/// or short `short_id`'s; to its end when not given) on a music track: one undo step, importing it into the project (with its licence)
/// the first time, and downloading a built-in track the first time it is used.
pub fn project_music_place_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    track_id: String,
    at_us: u64,
    until_us: Option<u64>,
    short_id: Option<String>,
) -> Result<MusicPlaced, String> {
    // Downloaded before taking the project's locks: it can take a few seconds.
    let track = music_library_fetch_impl(track_id)?;
    let (project, asset_id, bed) = super::sounds::place_on_shown(
        state,
        &project_handle,
        expected_revision,
        short_id.as_deref(),
        |document, root| library::asset_for(document, root, &track),
        |shown, asset, taken| library::place(shown, &track, asset, at_us, until_us, taken),
    )?;
    Ok(MusicPlaced {
        project,
        asset_id,
        bed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A track of the user's own placed under the whole video: on a music track, repeated to
    /// fill it, faded, and the music set under the speech.
    #[test]
    fn a_track_fills_the_video_on_a_music_track() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let state = &host.state;
        let song = dir.path().join("song.wav");
        let status = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=330:duration=12",
            ])
            .arg(&song)
            .status()
            .unwrap();
        assert!(status.success());
        let library_dir = library::dir();
        let mine = library::add(
            &library_dir,
            &song,
            Some("Test bed".into()),
            vec!["calm".into()],
            Licence {
                kind: crate::sequence::LicenceKind::NoCredit,
                source: None,
            },
        )
        .unwrap();
        let (handle, revision, length) = {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().unwrap();
            (
                reader.summary.project_handle.clone(),
                reader.summary.revision,
                reader.document().duration_us(),
            )
        };
        let placed =
            project_music_place_impl(state, handle, revision, mine.id.clone(), 0, None, None)
                .unwrap();
        library::remove(&library_dir, &mine.id).unwrap();
        assert_eq!(placed.bed.start_us, 0);
        assert!(placed.bed.end_us <= length);
        let opened = state.opened_project.lock();
        let document = opened.as_ref().unwrap().document();
        assert_eq!(document.duration_us(), length, "nothing moved");
        assert_eq!(
            document.audio.music_under_speech_db,
            Some(library::DEFAULT_UNDER_SPEECH_DB)
        );
        let (track, first) = document.sequence.clip(&placed.bed.clip_ids[0]).unwrap();
        assert_eq!(track.name, library::TRACK_NAME);
        assert_eq!(track.role, Some(crate::sequence::Role::Music));
        assert!(first.mix.fade_in_us > 0);
        let expected = length.div_ceil(12_000_000) as usize;
        assert!(
            placed.bed.clip_ids.len() >= expected.saturating_sub(1)
                && placed.bed.clip_ids.len() <= expected,
            "{} clips for {length}",
            placed.bed.clip_ids.len()
        );
    }
}
