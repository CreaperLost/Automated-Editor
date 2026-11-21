//! Each graphic of an edit on its own, see-through, for another editor (DLV-3, first part):
//! the export's own final render of every graphic clip on a shown track, in the slot it plays
//! in, written as a QuickTime ProRes 4444 file with its alpha, which Premiere, Resolve and
//! Final Cut all read. Named in timeline order with where each starts, so it can be lined up
//! by hand; the edit itself is not handed over yet.
use crate::project::revision::EditDocument;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// One graphic written.
#[derive(Clone, Debug, PartialEq)]
pub struct Written {
    pub path: PathBuf,
    pub start_us: u64,
    pub duration_us: u64,
}

/// `m.ss.cc` (minutes, seconds, hundredths): sorts and reads in a file name, which can't hold
/// a colon.
fn at(us: u64) -> String {
    let cs = us / 10_000;
    format!("{}m{:02}.{:02}s", cs / 6000, cs / 100 % 60, cs % 100)
}

/// A name a file system takes: no separators or reserved marks, not empty.
fn file_safe(name: &str) -> String {
    let clean: String = name
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        .collect();
    let clean = clean.trim().trim_matches('.').to_string();
    if clean.is_empty() {
        "Graphic".into()
    } else {
        clean.chars().take(60).collect()
    }
}

/// The graphic clips of `document` on shown tracks, by where they start.
fn graphic_clips(document: &EditDocument) -> Vec<&crate::sequence::Clip> {
    let mut clips: Vec<&crate::sequence::Clip> = document
        .sequence
        .video_tracks()
        .filter(|t| !t.hidden)
        .flat_map(|t| t.clips.iter())
        .filter(|c| document.asset(&c.asset).is_some_and(|a| a.is_graphic()))
        .collect();
    clips.sort_by_key(|c| (c.start_us, c.id.clone()));
    clips
}

/// Writes every graphic of `document` (on a `canvas` export at `fps`) into `folder`, made if
/// missing, rendering those not rendered yet. `progress` gets the share done and what is
/// happening.
pub fn write(
    root: &Path,
    document: &EditDocument,
    canvas: (u32, u32),
    fps: u32,
    folder: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f64, &str),
) -> Result<Vec<Written>, String> {
    use crate::graphics::render::{ensure, Quality, Spec};
    let clips = graphic_clips(document);
    if clips.is_empty() {
        return Err("The edit has no graphics".into());
    }
    let ffmpeg = crate::media::ffmpeg::ffmpeg_path()?;
    let full_camera = crate::graphics::full_camera_ranges(document);
    let specs: Vec<Spec> = clips
        .iter()
        .map(|clip| {
            let asset = document.asset(&clip.asset).expect("filtered to graphics");
            Spec::for_clip(
                root,
                document,
                asset,
                clip,
                canvas,
                &full_camera,
                Quality::Final { fps },
            )
        })
        .collect();
    // Rendering is most of the time; writing each file the rest.
    let renders = ensure(&specs, cancel, &mut |done, label, line| {
        if line.is_none() {
            progress(done * 0.8, label);
        }
    })?;
    std::fs::create_dir_all(folder)
        .map_err(|e| format!("Could not make {}: {e}", folder.display()))?;
    let mut written = Vec::new();
    for (index, (clip, spec)) in clips.iter().zip(&specs).enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        let render = renders
            .iter()
            .find(|(s, _)| s == spec)
            .map(|(_, path)| path)
            .ok_or_else(|| format!("Graphic \"{}\" has no render", spec.name))?;
        let path = folder.join(format!(
            "{:02} {} {}.mov",
            index + 1,
            at(clip.start_us),
            file_safe(&spec.name)
        ));
        progress(
            0.8 + 0.2 * index as f64 / clips.len() as f64,
            &format!("Writing \"{}\"", spec.name),
        );
        // The render's own alpha, kept: VP9 must be read by libvpx for it.
        let output = std::process::Command::new(ffmpeg)
            .args([
                "-nostdin",
                "-loglevel",
                "error",
                "-y",
                "-c:v",
                "libvpx-vp9",
                "-i",
            ])
            .arg(render)
            .args([
                "-map",
                "0:v:0",
                "-c:v",
                "prores_ks",
                "-profile:v",
                "4444",
                "-pix_fmt",
                "yuva444p10le",
                "-alpha_bits",
                "16",
                "-vendor",
                "apl0",
                "-an",
            ])
            .arg(&path)
            .output()
            .map_err(|e| format!("Could not run FFmpeg: {e}"))?;
        if !output.status.success() {
            let _ = std::fs::remove_file(&path);
            return Err(format!(
                "Graphic \"{}\" could not be written: {}",
                spec.name,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        written.push(Written {
            path,
            start_us: clip.start_us,
            duration_us: clip.duration_us,
        });
    }
    progress(1.0, "Done");
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_sort_by_time_and_keep_to_safe_marks() {
        assert_eq!(at(0), "0m00.00s");
        assert_eq!(at(83_456_789), "1m23.45s");
        assert_eq!(file_safe("Intro: \"Part 1\" / 2"), "Intro Part 1  2");
        assert_eq!(file_safe(" .. "), "Graphic");
    }

    /// A title card on the timeline at 2 s, its render made beforehand (the renderer isn't in
    /// tests), comes out as ProRes 4444 with its alpha, its length and its place in the name.
    #[test]
    fn each_graphic_is_written_see_through_with_its_place_in_the_name() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("p.aeroedit");
        std::fs::create_dir_all(&root).unwrap();
        let graphic =
            crate::graphics::create(&root, "title", None, &serde_json::Map::new()).unwrap();
        let mut document = EditDocument::default();
        let mut track = crate::sequence::Track::new("t1".into(), crate::sequence::TrackKind::Video);
        track.clips.push(crate::sequence::Clip {
            mix: Default::default(),
            id: "g1".into(),
            asset: graphic.id.clone(),
            stream: "picture".into(),
            start_us: 2_000_000,
            in_us: 0,
            duration_us: 1_000_000,
            link: None,
            fit: crate::sequence::Fit::Contain,
            speed: 1.0,
        });
        document.assets.push(graphic.clone());
        document.sequence.tracks.push(track);
        document.validate().unwrap();
        let fps = 30;
        // Where the export's render of it goes, filled with a see-through square.
        let clip = &graphic_clips(&document)[0];
        let spec = crate::graphics::render::Spec::for_clip(
            &root,
            &document,
            &graphic,
            clip,
            (1920, 1080),
            &[],
            crate::graphics::render::Quality::Final { fps },
        );
        let target = spec.cache.join(spec.file_name(&spec.hash().unwrap()));
        std::fs::create_dir_all(&spec.cache).unwrap();
        let made = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
            .args(["-nostdin", "-loglevel", "error", "-y", "-f", "lavfi", "-i"])
            .arg(format!(
                "color=c=black@0.0:s={}x{}:r={fps}:d=1,format=rgba,\
                 drawbox=x=100:y=100:w=200:h=200:color=white:t=fill:replace=1",
                spec.width, spec.height
            ))
            .args([
                "-c:v",
                "libvpx-vp9",
                "-pix_fmt",
                "yuva420p",
                "-b:v",
                "0",
                "-crf",
                "30",
            ])
            .arg(&target)
            .status()
            .unwrap();
        assert!(made.success());
        let out = dir.path().join("p graphics");
        let mut steps = Vec::new();
        let written = write(
            &root,
            &document,
            (1920, 1080),
            fps,
            &out,
            &AtomicBool::new(false),
            &mut |done, what| steps.push((done, what.to_string())),
        )
        .unwrap();
        assert_eq!(written.len(), 1);
        let name = written[0].path.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("01 0m02.00s "), "{name}");
        assert!(name.ends_with(".mov"));
        assert_eq!(steps.last().unwrap().0, 1.0);
        let probe = std::process::Command::new(crate::media::ffmpeg::ffprobe_path().unwrap())
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=codec_name,pix_fmt,width,height:format=duration",
                "-of",
                "default=nw=1",
            ])
            .arg(&written[0].path)
            .output()
            .unwrap();
        let probe = String::from_utf8_lossy(&probe.stdout);
        assert!(probe.contains("codec_name=prores"), "{probe}");
        assert!(probe.contains("pix_fmt=yuva444p"), "{probe}");
        assert!(probe.contains(&format!("width={}", spec.width)), "{probe}");
        let duration: f64 = probe
            .lines()
            .find_map(|l| l.strip_prefix("duration="))
            .unwrap()
            .parse()
            .unwrap();
        assert!((duration - 1.0).abs() < 0.05, "{duration}");
        // The alpha came through: a corner is clear and the square is solid.
        let alpha = |x: u32, y: u32| {
            let raw = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
                .args(["-nostdin", "-loglevel", "error", "-i"])
                .arg(&written[0].path)
                .args(["-frames:v", "1", "-vf"])
                .arg(format!("crop=1:1:{x}:{y},format=rgba"))
                .args(["-f", "rawvideo", "-"])
                .output()
                .unwrap()
                .stdout;
            raw[3]
        };
        assert!(alpha(20, 20) < 10, "corner");
        assert!(alpha(200, 200) > 245, "square");
        // Nothing to write is said plainly.
        let empty = write(
            &root,
            &EditDocument::default(),
            (1920, 1080),
            fps,
            &out,
            &AtomicBool::new(false),
            &mut |_, _| {},
        );
        assert_eq!(empty.unwrap_err(), "The edit has no graphics");
    }
}
