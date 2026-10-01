pub mod journal;
pub mod layout;
pub mod manifest;
pub mod pcm;
pub mod reader;
pub mod revision;
pub mod silence;
pub mod waveform;

pub use journal::JournalRecord;
pub use layout::EditLayout;
pub use manifest::{ManifestError, ProjectManifest, TrackDescriptor, TrackType};
pub use reader::{OpenedProject, ProjectReader, RetainedInterval, SegmentPage};
pub use waveform::{WaveformPage, WaveformTrackContext};

const MAX_PROJECT_NAME_CHARS: usize = 80;

/// Returns the default dated project name for a given datetime, e.g. "Untitled 9 Sep 2026".
pub fn default_project_name_at<Tz: chrono::TimeZone>(dt: &chrono::DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    dt.format("Untitled %-d %b %Y").to_string()
}

/// Returns the default dated project name using local time, e.g. "Untitled 9 Sep 2026".
pub fn default_project_name() -> String {
    default_project_name_at(&chrono::Local::now())
}

/// Display name stored in `manifest.json`. Empty input becomes a dated default, e.g. `Untitled 9 Sep 2026`.
pub fn display_name_from_input(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        default_project_name()
    } else {
        trimmed.chars().take(MAX_PROJECT_NAME_CHARS).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dated_untitled_name_format() {
        use chrono::TimeZone;
        let dt = chrono::Utc.with_ymd_and_hms(2026, 9, 9, 12, 0, 0).unwrap();
        assert_eq!(default_project_name_at(&dt), "Untitled 9 Sep 2026");

        let dt2 = chrono::Utc
            .with_ymd_and_hms(2026, 12, 25, 8, 30, 0)
            .unwrap();
        assert_eq!(default_project_name_at(&dt2), "Untitled 25 Dec 2026");
    }

    #[test]
    fn display_name_trims_and_defaults_empty_input() {
        assert_eq!(display_name_from_input("  Custom Take  "), "Custom Take");
        assert_eq!(display_name_from_input("   "), default_project_name());
        assert_eq!(
            display_name_from_input(&"x".repeat(200)).chars().count(),
            MAX_PROJECT_NAME_CHARS
        );
    }
}
