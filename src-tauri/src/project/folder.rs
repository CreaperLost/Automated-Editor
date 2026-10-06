//! Project folders, separate from recordings. A project folder holds `aeroedits.json` (its
//! name and the recording it was made from, if any), the edit (`project.json`), extracted
//! sound and transcripts. Recording folders are only read; inside the project a recording
//! asset is seen at `recordings/<asset id>/`, so `recordings/rec-1/media/screen/000001.mp4`
//! names a file of that recording.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const PROJECT_FILE: &str = "aeroedits.json";
/// Recordings are seen inside a project at `recordings/<asset id>/`.
pub const RECORDINGS_MOUNT: &str = "recordings";
const PROJECT_FILE_VERSION: u32 = 1;
const PROJECT_FILE_LIMIT: u64 = 65_536;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub created_at: String,
    /// The recording this project was made from; it becomes the project's first asset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording: Option<String>,
}

/// Project roots and the recording folders seen inside them, by asset id.
static MOUNTS: parking_lot::RwLock<Vec<(PathBuf, String, PathBuf)>> =
    parking_lot::RwLock::new(Vec::new());

/// Where recording asset `asset_id`'s files are seen inside a project.
pub fn mount_path(asset_id: &str) -> String {
    format!("{RECORDINGS_MOUNT}/{asset_id}")
}

/// From now on, `recordings/<asset_id>/...` inside `project_root` resolves into `folder`. Kept
/// for the life of the app, so an export that outlives the open project still finds its files.
pub(crate) fn mount_recording(project_root: &Path, asset_id: &str, folder: &Path) {
    let mut mounts = MOUNTS.write();
    mounts.retain(|(root, id, _)| !(root == project_root && id == asset_id));
    mounts.push((
        project_root.to_path_buf(),
        asset_id.to_string(),
        folder.to_path_buf(),
    ));
}

/// The folder and the rest of the path, when `relative` names a file of a mounted recording.
pub(crate) fn resolve_mount<'a>(
    project_root: &Path,
    relative: &'a str,
) -> Option<(PathBuf, &'a str)> {
    let rest = relative
        .strip_prefix(RECORDINGS_MOUNT)
        .and_then(|rest| rest.strip_prefix('/'))?;
    let (id, inside) = rest.split_once('/')?;
    MOUNTS
        .read()
        .iter()
        .find(|(root, mounted, _)| root == project_root && mounted == id)
        .map(|(_, _, folder)| (folder.clone(), inside))
}

pub(crate) fn load_project_file(root: &Path) -> Result<Option<ProjectFile>, String> {
    let path = root.join(PROJECT_FILE);
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    if !meta.is_file() || meta.len() > PROJECT_FILE_LIMIT {
        return Err(format!("{PROJECT_FILE} is not a project file"));
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let file: ProjectFile =
        serde_json::from_slice(&bytes).map_err(|e| format!("{PROJECT_FILE} is damaged: {e}"))?;
    if file.version != PROJECT_FILE_VERSION {
        return Err(format!(
            "This project was made by a newer AeroEdits (format {})",
            file.version
        ));
    }
    if file.name.trim().is_empty() {
        return Err(format!("{PROJECT_FILE} has no project name"));
    }
    Ok(Some(file))
}

pub(crate) fn save_project_file(root: &Path, file: &ProjectFile) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(file).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, &bytes).map_err(|e| e.to_string())?;
    temp.persist(root.join(PROJECT_FILE))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// A folder name from a project name: no path separators or characters Windows refuses.
fn folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "Untitled".into()
    } else {
        trimmed.into()
    }
}

/// Makes a new project folder named after `name` inside `parent` (" 2", " 3", ... when the
/// name is taken), linked to `recording` if one is given. Returns the folder.
pub fn create_project_folder(
    parent: &Path,
    name: &str,
    recording: Option<&Path>,
) -> Result<PathBuf, String> {
    let name = super::display_name_from_input(name);
    if !parent.is_absolute() {
        return Err("Choose a full folder path for the project".into());
    }
    fs::create_dir_all(parent).map_err(|e| format!("Could not make {}: {e}", parent.display()))?;
    let parent = dunce::canonicalize(parent).map_err(|e| e.to_string())?;
    let recording = match recording {
        Some(path) => {
            let path = dunce::canonicalize(path)
                .map_err(|_| format!("There is no recording at {}", path.display()))?;
            if !path.join("manifest.json").is_file() {
                return Err(format!(
                    "{} is not an AeroEdits recording (it has no manifest.json)",
                    path.display()
                ));
            }
            if path.join(PROJECT_FILE).exists() {
                return Err("That folder is a project, not a recording".into());
            }
            Some(path)
        }
        None => None,
    };
    let base = folder_name(&name);
    let folder = (1..1000)
        .map(|n| {
            if n == 1 {
                parent.join(&base)
            } else {
                parent.join(format!("{base} {n}"))
            }
        })
        .find(|candidate| !candidate.exists())
        .ok_or("Too many projects with that name")?;
    if let Some(recording) = &recording {
        if folder.starts_with(recording) {
            return Err("Keep the project outside the recording folder".into());
        }
    }
    fs::create_dir(&folder).map_err(|e| format!("Could not make {}: {e}", folder.display()))?;
    let file = ProjectFile {
        version: PROJECT_FILE_VERSION,
        name,
        created_at: chrono::Utc::now().to_rfc3339(),
        recording: recording.map(|path| path.to_string_lossy().into_owned()),
    };
    if let Err(error) = save_project_file(&folder, &file) {
        let _ = fs::remove_dir_all(&folder);
        return Err(error);
    }
    Ok(folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names_are_safe_and_unique() {
        assert_eq!(folder_name("My: cut/one?"), "My- cut-one-");
        assert_eq!(folder_name("  ..  "), "Untitled");
        let dir = tempfile::tempdir().unwrap();
        let first = create_project_folder(dir.path(), "Launch video", None).unwrap();
        let second = create_project_folder(dir.path(), "Launch video", None).unwrap();
        assert_eq!(first.file_name().unwrap(), "Launch video");
        assert_eq!(second.file_name().unwrap(), "Launch video 2");
        let file = load_project_file(&first).unwrap().unwrap();
        assert_eq!((file.name.as_str(), file.recording), ("Launch video", None));
        // A recording must look like one.
        assert!(create_project_folder(dir.path(), "x", Some(&first)).is_err());
        assert!(create_project_folder(dir.path(), "x", Some(&dir.path().join("nope"))).is_err());
        mount_recording(dir.path(), "rec-1", &first);
        assert_eq!(
            resolve_mount(dir.path(), "recordings/rec-1/media/a.mp4"),
            Some((first.clone(), "media/a.mp4"))
        );
        assert_eq!(
            resolve_mount(dir.path(), "recordings/rec-2/media/a.mp4"),
            None
        );
        assert_eq!(resolve_mount(dir.path(), "recording/a"), None);
    }

    /// A recording with one second of a loud microphone, as the recorder writes it.
    fn mic_recording(parent: &Path) -> PathBuf {
        use crate::fixtures::{generate_pcm16_wav, TestProject};
        use crate::project::manifest::{TrackDescriptor, TrackType};
        use crate::project::JournalRecord;
        let mut bundle = TestProject::create(parent, "take");
        let wav = generate_pcm16_wav(48_000, 1, &vec![12_000i16; 48_000]);
        let relative = "media/mic/000001.wav";
        fs::write(bundle.root_path().join(relative), &wav).unwrap();
        bundle.manifest_mut().tracks.push(TrackDescriptor {
            id: "mic".into(),
            track_type: TrackType::MicAudio,
            codec: "pcm".into(),
            relative_path: relative.into(),
            width: None,
            height: None,
            fps: None,
            sample_rate: Some(48_000),
            channels: Some(1),
            gaps_total: 0,
            media_timescale: Some(48_000),
        });
        bundle.append_journal(JournalRecord::SegmentCommitted {
            seq: 0,
            track_id: "mic".into(),
            relative_path: relative.into(),
            start_us: 0,
            end_us: 1_000_000,
            size_bytes: wav.len() as u64,
            is_keyframe_start: true,
            media_timescale: 48_000,
            media_start_value: 0,
            host_anchor_us: 0,
        });
        bundle.manifest_mut().duration_us = 1_000_000;
        bundle.manifest_mut().active_duration_us = 1_000_000;
        bundle.save_manifest();
        bundle.root_path().to_path_buf()
    }

    /// Everything about the recording comes from its folder; every edit lands in the project.
    #[test]
    fn a_project_reads_its_recording_and_writes_only_to_itself() {
        use crate::project::ProjectReader;
        use crate::sequence::edit::SequenceEdit;
        let dir = tempfile::tempdir().unwrap();
        let recording = mic_recording(dir.path());
        let recording = dunce::canonicalize(recording).unwrap();
        let manifest_before = fs::read(recording.join("manifest.json")).unwrap();
        let folder =
            create_project_folder(&dir.path().join("Projects"), "Edit", Some(&recording)).unwrap();

        let mut reader = ProjectReader::open(&folder).unwrap();
        assert_eq!(reader.summary.name, "Edit");
        assert_eq!(reader.summary.duration_us, 1_000_000);
        let asset = reader.summary.assets[0].clone();
        assert_eq!(asset.path, recording.to_string_lossy());
        let key = format!("{}.mic", asset.id);
        let segment = reader.page(&key, 0, 10).unwrap().segments[0].clone();
        assert!(segment.available, "the recording's media is found");
        assert_eq!(
            segment.relative_path,
            format!("recordings/{}/media/mic/000001.wav", asset.id)
        );
        assert_eq!(
            crate::project::reader::safe_path(reader.root(), &segment.relative_path).unwrap(),
            recording.join("media/mic/000001.wav")
        );
        // Playback and export read the microphone through the mount.
        let mixer =
            crate::media::audio::AudioMixer::new(reader.root(), &reader.history().current).unwrap();
        let pcm = mixer.read_frames(24_000, 480).unwrap();
        assert!(pcm.iter().any(|&s| s.unsigned_abs() > 5_000));
        // The mixer keeps its files open while it plays; Windows will not move them then.
        drop(mixer);

        reader
            .edit_sequence(
                0,
                &SequenceEdit::Split {
                    at_us: 500_000,
                    clip_ids: vec![],
                },
                None,
            )
            .unwrap();
        reader.rename_project("Launch edit").unwrap();
        drop(reader);
        assert!(folder.join("project.json").is_file());
        assert!(!recording.join("project.json").exists());
        assert_eq!(
            fs::read(recording.join("manifest.json")).unwrap(),
            manifest_before
        );
        let reopened = ProjectReader::open(&folder).unwrap();
        assert_eq!(reopened.summary.name, "Launch edit");
        assert_eq!(reopened.summary.sequence.tracks[0].clips.len(), 2);
        assert!(!reopened.summary.assets[0].missing);
        drop(reopened);

        // A recording that moved away shows as missing; the project still opens.
        fs::rename(&recording, dir.path().join("moved.aero")).unwrap();
        let reopened = ProjectReader::open(&folder).unwrap();
        assert!(reopened.summary.assets[0].missing);
        assert!(reopened
            .summary
            .diagnostics
            .iter()
            .any(|d| d.contains("missing")));
    }

    /// A project made from a recording that is gone says where it was.
    #[test]
    fn a_new_project_whose_recording_is_gone_says_so() {
        use crate::project::ProjectReader;
        let dir = tempfile::tempdir().unwrap();
        let recording = mic_recording(dir.path());
        let folder =
            create_project_folder(&dir.path().join("Projects"), "Edit", Some(&recording)).unwrap();
        fs::rename(&recording, dir.path().join("moved.aero")).unwrap();
        let error = ProjectReader::open(&folder).err().unwrap();
        assert!(error.contains("recording is missing"), "{error}");
    }

    /// A project without a recording opens empty and fills up with imported media.
    #[test]
    fn an_empty_project_opens_and_takes_imported_media() {
        use crate::project::ProjectReader;
        use crate::sequence::edit::SequenceEdit;
        let dir = tempfile::tempdir().unwrap();
        let folder = create_project_folder(dir.path(), "From scratch", None).unwrap();
        let mut reader = ProjectReader::open(&folder).unwrap();
        assert_eq!(reader.summary.duration_us, 0);
        assert!(reader.summary.assets.is_empty());
        assert!(reader.summary.sequence.tracks.is_empty());
        let owner = crate::playback::PlaybackOwner::open(
            reader.summary.project_handle.clone(),
            &reader.history().current,
        );
        assert!(owner.is_ok(), "playback opens on an empty timeline");

        let png = dir.path().join("card.png");
        image::RgbaImage::from_pixel(16, 9, image::Rgba([0, 0, 255, 255]))
            .save(&png)
            .unwrap();
        let summary = reader.import_media(0, &[png.clone()]).unwrap();
        let id = summary.assets[0].id.clone();
        let (summary, _) = reader
            .edit_sequence(
                1,
                &SequenceEdit::PlaceAsset {
                    asset_id: id,
                    at_us: 0,
                    track_id: None,
                    streams: vec![],
                    range: None,
                },
                None,
            )
            .unwrap();
        assert_eq!(summary.duration_us, crate::sequence::IMAGE_CLIP_US);
        drop(reader);
        let reopened = ProjectReader::open(&folder).unwrap();
        assert_eq!(reopened.summary.duration_us, crate::sequence::IMAGE_CLIP_US);
        // The image is used where it is, and shows as missing once it is gone.
        let asset = &reopened.summary.assets[0];
        assert!(Path::new(&asset.path).is_file() && !asset.missing);
        drop(reopened);
        std::fs::remove_file(&png).unwrap();
        let reopened = ProjectReader::open(&folder).unwrap();
        assert!(reopened.summary.assets[0].missing);
    }
}
