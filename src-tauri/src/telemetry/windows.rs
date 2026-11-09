//! The front window recorded beside the mouse stream (`telemetry/windows.jsonl`), as
//! specified in `docs/architecture.md#window-telemetry`: which window is in front and where it is on the
//! recorded source, and gaps where that was not known. Same clock as `events.jsonl`.
//!
//! No file means the windows were not recorded: the screen is never cropped to one.
use super::stream::{read_once, read_stream, FolderCache};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const WINDOWS_VERSION: u32 = 1;
use super::stream::MAX_SAFE_TIME;

/// What kind of window is in front. Popups, menus and tooltips never decide the crop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowRole {
    Window,
    Dialog,
    Popup,
    Menu,
    Tooltip,
    Fullscreen,
    #[serde(other)]
    Other,
}

impl WindowRole {
    /// Short-lived windows that open over another one: they never move the crop.
    pub fn is_transient(self) -> bool {
        matches!(self, Self::Popup | Self::Menu | Self::Tooltip | Self::Other)
    }
}

/// A front window's box on the recorded source, normalized like the mouse (0..1 of the
/// source), cut to the source.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowBox {
    pub id: String,
    pub role: WindowRole,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub maximized: bool,
}

/// From `t_us` on, `window` is in front (`None`: no app window on the recorded source, such
/// as the desktop or a window on another display).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontWindow {
    pub t_us: u64,
    pub window: Option<WindowBox>,
}

/// A recording's front windows, sorted by time.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStream {
    pub platform: Option<String>,
    pub fronts: Vec<FrontWindow>,
    /// Stretches where the front window was not seen (start, end).
    pub gaps: Vec<(u64, u64)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

#[derive(Deserialize)]
struct RawBounds {
    norm_x: f64,
    norm_y: f64,
    norm_w: f64,
    norm_h: f64,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RawKind {
    Capture {
        #[serde(default)]
        platform: Option<String>,
    },
    Front {
        #[serde(default)]
        window: Option<String>,
        #[serde(default)]
        role: Option<WindowRole>,
        #[serde(default)]
        bounds: Option<RawBounds>,
        #[serde(default)]
        maximized: bool,
    },
    Gap {
        start_us: u64,
        end_us: u64,
    },
    #[serde(other)]
    Unknown,
}

/// The part of `bounds` on the source, or `None` when none of it is.
fn on_source(bounds: &RawBounds) -> Option<(f64, f64, f64, f64)> {
    let values = [bounds.norm_x, bounds.norm_y, bounds.norm_w, bounds.norm_h];
    if values.iter().any(|v| !v.is_finite()) || bounds.norm_w <= 0.0 || bounds.norm_h <= 0.0 {
        return None;
    }
    let x0 = bounds.norm_x.max(0.0);
    let y0 = bounds.norm_y.max(0.0);
    let x1 = (bounds.norm_x + bounds.norm_w).min(1.0);
    let y1 = (bounds.norm_y + bounds.norm_h).min(1.0);
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
}

/// Reads `telemetry/windows.jsonl` under `root`. `Ok(None)` when the recording has none or the
/// file is of a version this build does not read (`diagnostics` says why).
pub fn read_windows(
    root: &Path,
    diagnostics: &mut Vec<String>,
) -> Result<Option<WindowStream>, String> {
    let mut stream = WindowStream::default();
    let problems = read_stream(
        root,
        "windows.jsonl",
        "Window",
        WINDOWS_VERSION,
        diagnostics,
        |t_us, kind| match kind {
            RawKind::Capture { platform } => stream.platform = platform,
            RawKind::Front {
                window,
                role,
                bounds,
                maximized,
            } => {
                let window = window.and_then(|id| {
                    let (x, y, w, h) = on_source(bounds.as_ref()?)?;
                    Some(WindowBox {
                        id: id.chars().take(64).collect(),
                        role: role.unwrap_or(WindowRole::Window),
                        x,
                        y,
                        w,
                        h,
                        maximized,
                    })
                });
                stream.fronts.push(FrontWindow { t_us, window });
            }
            RawKind::Gap { start_us, end_us } => {
                if end_us >= start_us && end_us <= MAX_SAFE_TIME {
                    stream.gaps.push((start_us, end_us));
                }
            }
            RawKind::Unknown => {}
        },
    )?;
    let Some(problems) = problems else {
        return Ok(None);
    };
    stream.diagnostics = problems;
    stream.fronts.sort_by_key(|f| f.t_us);
    stream.gaps.sort_unstable();
    Ok(Some(stream))
}

/// The front windows of the recording in `folder`, read once while the file is unchanged.
pub fn recording_windows(folder: &Path) -> std::sync::Arc<Option<WindowStream>> {
    static CACHE: FolderCache<Option<WindowStream>> = std::sync::OnceLock::new();
    read_once(&CACHE, folder, &["windows.jsonl"], || {
        read_windows(folder, &mut Vec::new()).ok().flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, lines: &[&str]) {
        std::fs::create_dir_all(dir.join("telemetry")).unwrap();
        std::fs::write(dir.join("telemetry/windows.jsonl"), lines.join("\n")).unwrap();
    }

    #[test]
    fn reads_fronts_cut_to_the_source_and_skips_the_unknown() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            &[
                r#"{"version":1,"seq":0,"t_us":0,"kind":"capture","platform":"windows"}"#,
                r#"{"version":1,"seq":1,"t_us":5000,"kind":"front","window":"0x1","app":"Code.exe","role":"window","bounds":{"norm_x":-0.1,"norm_y":0.1,"norm_w":0.6,"norm_h":0.5}}"#,
                r#"{"version":1,"seq":2,"t_us":9000,"kind":"front","window":"0x2","role":"menu","bounds":{"norm_x":0.2,"norm_y":0.2,"norm_w":0.1,"norm_h":0.2}}"#,
                r#"{"version":1,"seq":3,"t_us":12000,"kind":"front","window":null}"#,
                r#"{"version":1,"seq":4,"t_us":13000,"kind":"front","window":"0x3","bounds":{"norm_x":1.2,"norm_y":0.1,"norm_w":0.5,"norm_h":0.5}}"#,
                r#"{"version":1,"seq":5,"t_us":20000,"kind":"gap","reason":"recording_paused","start_us":14000,"end_us":20000}"#,
                r#"{"version":1,"seq":6,"t_us":21000,"kind":"something_new"}"#,
                r#"{"version":1,"seq":7,"t_us":22000,"kind":"fro"#,
            ],
        );
        let stream = read_windows(dir.path(), &mut Vec::new()).unwrap().unwrap();
        assert_eq!(stream.platform.as_deref(), Some("windows"));
        assert_eq!(stream.fronts.len(), 4);
        let first = stream.fronts[0].window.as_ref().unwrap();
        assert_eq!((first.x, first.w), (0.0, 0.5));
        assert_eq!(first.role, WindowRole::Window);
        assert!(stream.fronts[1]
            .window
            .as_ref()
            .unwrap()
            .role
            .is_transient());
        assert!(stream.fronts[2].window.is_none());
        // Wholly off the source: as good as no window there.
        assert!(stream.fronts[3].window.is_none());
        assert_eq!(stream.gaps, vec![(14000, 20000)]);
    }

    #[test]
    fn another_version_or_no_file_reads_as_not_recorded() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_windows(dir.path(), &mut Vec::new()).unwrap().is_none());
        write(
            dir.path(),
            &[r#"{"version":2,"seq":0,"t_us":0,"kind":"capture"}"#],
        );
        let mut diagnostics = Vec::new();
        assert!(read_windows(dir.path(), &mut diagnostics)
            .unwrap()
            .is_none());
        assert!(diagnostics[0].contains("version 2"));
    }
}
