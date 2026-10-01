use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackType {
    Screen,
    Webcam,
    SystemAudio,
    MicAudio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackDescriptor {
    pub id: String,
    pub track_type: TrackType,
    pub codec: String,
    pub relative_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u16>,
    /// Number of timeline gaps (discontinuities) observed on this track.
    /// Persisted so downstream tools can re-derive diagnostics without
    /// re-scanning the journal.
    #[serde(default)]
    pub gaps_total: u64,
    /// Native media-clock timescale parsed from the track's `mdhd` box.
    /// Used to re-establish a rational media-clock → host-clock mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_timescale: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PauseInterval {
    pub start_us: u64,
    pub end_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitMode {
    Fit,
    Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceGeometry {
    pub source_rect: SourceRect,
    pub dest_rect: SourceRect,
    pub fit_mode: FitMode,
    pub preserves_aspect_ratio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub version: u32,
    pub session_id: String,
    pub project_name: String,
    pub created_at: String,
    pub duration_us: u64,
    #[serde(default)]
    pub active_duration_us: u64,
    #[serde(default)]
    pub pause_intervals: Vec<PauseInterval>,
    /// Total number of timeline gaps (discontinuities) observed across
    /// every track. Mirrors the journal's `Discontinuity` records so the
    /// editor can surface this without re-scanning the journal.
    #[serde(default)]
    pub gaps_total: u64,
    /// Most-recent source geometry revision (display/window/app rect +
    /// destination rect + fit mode). `None` until the first compute
    /// happens. Written by the recorder; the editor round-trips it unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_geometry: Option<SourceGeometry>,
    /// Unknown on legacy bundles; native recording currently bakes the OS cursor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_mode: Option<String>,
    pub tracks: Vec<TrackDescriptor>,
}

#[derive(Error, Debug, PartialEq)]
pub enum ManifestError {
    #[error("Unsupported manifest version: {0}")]
    UnsupportedVersion(u32),
    #[error("Invalid track path: {0} (traversal or absolute path detected)")]
    InvalidTrackPath(String),
    #[error("Path escapes project directory root: {0}")]
    PathEscapesRoot(String),
    #[error("Empty session ID or project name")]
    InvalidMetadata,
    #[error("Serialization error: {0}")]
    Serde(String),
    #[error("IO error: {0}")]
    Io(String),
}

impl ProjectManifest {
    pub const CURRENT_VERSION: u32 = 1;

    #[cfg(test)]
    pub fn new(session_id: String, project_name: String) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            session_id,
            project_name,
            created_at: chrono::Utc::now().to_rfc3339(),
            duration_us: 0,
            active_duration_us: 0,
            pause_intervals: Vec::new(),
            gaps_total: 0,
            source_geometry: None,
            cursor_mode: None,
            tracks: Vec::new(),
        }
    }

    /// Validates the manifest integrity and path security.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.version != Self::CURRENT_VERSION {
            return Err(ManifestError::UnsupportedVersion(self.version));
        }
        if self.session_id.trim().is_empty() || self.project_name.trim().is_empty() {
            return Err(ManifestError::InvalidMetadata);
        }

        for track in &self.tracks {
            let path = Path::new(&track.relative_path);
            if path.is_absolute()
                || path
                    .components()
                    .any(|c| c == std::path::Component::ParentDir)
            {
                return Err(ManifestError::InvalidTrackPath(track.relative_path.clone()));
            }
        }

        Ok(())
    }

    /// Saves manifest with durable atomic replacement and prior revision backup (.bak).
    /// Uses exclusive file creation (create_new) and symlink-safe handling to prevent
    /// following symlinks to targets outside the project.
    pub fn save_with_backup<P: AsRef<Path>>(&self, manifest_path: P) -> Result<(), ManifestError> {
        self.validate()?;
        let path = manifest_path.as_ref();

        // Reject if target manifest path itself is a symlink
        if let Ok(meta) = fs::symlink_metadata(path) {
            if meta.file_type().is_symlink() {
                return Err(ManifestError::PathEscapesRoot(format!(
                    "Manifest file cannot be a symlink: {:?}",
                    path
                )));
            }
        }

        let serialized =
            serde_json::to_string_pretty(self).map_err(|e| ManifestError::Serde(e.to_string()))?;

        // Safely prepare tmp path: remove any pre-existing tmp file/symlink safely
        let tmp_path = path.with_extension("tmp");
        if let Ok(_) = fs::symlink_metadata(&tmp_path) {
            fs::remove_file(&tmp_path).map_err(|e| ManifestError::Io(e.to_string()))?;
        }

        // Exclusively create temporary file (O_CREAT | O_EXCL) to guarantee it never follows a symlink
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
            .map_err(|e| ManifestError::Io(e.to_string()))?;

        file.write_all(serialized.as_bytes())
            .map_err(|e| ManifestError::Io(e.to_string()))?;
        file.sync_all()
            .map_err(|e| ManifestError::Io(e.to_string()))?;
        drop(file);

        // Retain prior revision backup if manifest exists
        if path.exists() {
            let bak_path = path.with_extension("bak");
            if let Ok(meta) = fs::symlink_metadata(&bak_path) {
                if meta.file_type().is_symlink() {
                    fs::remove_file(&bak_path).map_err(|e| ManifestError::Io(e.to_string()))?;
                }
            }

            // Exclusively create backup temporary file and atomically rename over bak_path
            let bak_tmp_path = path.with_extension(format!("bak.{}.tmp", uuid::Uuid::new_v4()));
            if let Ok(existing_bytes) = fs::read(path) {
                if let Ok(mut bak_file) = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&bak_tmp_path)
                {
                    let _ = bak_file.write_all(&existing_bytes);
                    let _ = bak_file.sync_all();
                    drop(bak_file);
                    let _ = fs::rename(&bak_tmp_path, &bak_path);
                }
            }
        }

        fs::rename(&tmp_path, path).map_err(|e| ManifestError::Io(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_validation() {
        let mut manifest = ProjectManifest::new("sess-1".into(), "Demo Project".into());
        manifest.tracks.push(TrackDescriptor {
            id: "screen-1".into(),
            track_type: TrackType::Screen,
            codec: "h264".into(),
            relative_path: "media/screen/000001.mp4".into(),
            width: Some(1920),
            height: Some(1080),
            fps: Some(30),
            sample_rate: None,
            channels: None,
            gaps_total: 0,
            media_timescale: None,
        });

        assert!(manifest.validate().is_ok());

        // Test path traversal rejection
        manifest.tracks[0].relative_path = "../etc/passwd".into();
        assert!(matches!(
            manifest.validate(),
            Err(ManifestError::InvalidTrackPath(_))
        ));
    }

    #[test]
    fn save_with_backup_preserves_prior_revision() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = dir.path().join("manifest.json");
        let bak_path = dir.path().join("manifest.bak");
        let mut manifest = ProjectManifest::new("sess-1".into(), "Rev 1".into());
        manifest.save_with_backup(&manifest_path).unwrap();
        assert!(manifest_path.exists());
        assert!(!bak_path.exists());

        manifest.project_name = "Rev 2".into();
        manifest.save_with_backup(&manifest_path).unwrap();
        assert!(fs::read_to_string(&bak_path).unwrap().contains("Rev 1"));
        assert!(fs::read_to_string(&manifest_path).unwrap().contains("Rev 2"));
    }
}
