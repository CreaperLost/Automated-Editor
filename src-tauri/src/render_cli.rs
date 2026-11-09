//! `aeroedits render <project> [--out x.mp4] [--short ID] [--size WxH] [--fps N] [--frames N]`:
//! exports a project's saved edit to an MP4 with no app running, for cloud sessions and scripts.
//! It reads the project as the app saved it and renders with the same export as the Export
//! button. `--frames N` also saves N evenly spaced PNG stills and a contact sheet beside the
//! MP4, so an agent that cannot watch video can look at the result.
use crate::export::{prepare_job, run_export, ExportOwner, ExportSettings};
use crate::media::EncoderGate;
use crate::project::ProjectReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

const USAGE: &str = "Usage: aeroedits render <project folder> [--out <file.mp4>] [--short <id>] \
[--size <W>x<H>] [--fps <n>] [--frames <n>]";

#[derive(Debug, PartialEq)]
pub struct Request {
    pub project: PathBuf,
    pub out: Option<PathBuf>,
    pub short: Option<String>,
    pub size: (u32, u32),
    pub fps: u32,
    pub frames: u32,
}

pub fn parse(args: &[String]) -> Result<Request, String> {
    let mut request = Request {
        project: PathBuf::new(),
        out: None,
        short: None,
        size: (1920, 1080),
        fps: 30,
        frames: 0,
    };
    let mut project = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        let number = |name: &str, text: String| {
            text.parse::<u32>()
                .ok()
                .filter(|n| *n > 0)
                .ok_or_else(|| format!("{name} takes a whole number above 0, not {text:?}"))
        };
        match arg.as_str() {
            "--help" | "-h" => return Err(USAGE.into()),
            "--out" | "-o" => request.out = Some(PathBuf::from(value("--out")?)),
            "--short" => request.short = Some(value("--short")?),
            "--fps" => request.fps = number("--fps", value("--fps")?)?,
            "--frames" => request.frames = number("--frames", value("--frames")?)?,
            "--size" => {
                let text = value("--size")?;
                let (w, h) = text
                    .split_once(['x', 'X'])
                    .ok_or_else(|| format!("--size is <W>x<H>, like 1280x720, not {text:?}"))?;
                request.size = (number("--size", w.into())?, number("--size", h.into())?);
            }
            flag if flag.starts_with('-') => return Err(format!("Unknown option {flag}\n{USAGE}")),
            path if project.is_none() => project = Some(PathBuf::from(path)),
            extra => return Err(format!("Unexpected {extra:?}\n{USAGE}")),
        }
    }
    request.project = project.ok_or(USAGE)?;
    Ok(request)
}

/// What a render made.
#[derive(Debug)]
pub struct Rendered {
    pub video: PathBuf,
    pub stills: Vec<PathBuf>,
    pub contact_sheet: Option<PathBuf>,
}

/// Renders `request`, telling `progress` the frames done out of the total.
pub fn render(request: &Request, progress: impl FnMut(u32, u32)) -> Result<Rendered, String> {
    let folder = std::path::absolute(&request.project)
        .map_err(|e| format!("{}: {e}", request.project.display()))?;
    let reader = ProjectReader::open(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let base = &reader.history().current;
    let (document, name) = match &request.short {
        None => (base.clone(), reader.summary.name.clone()),
        Some(id) => {
            let short = base.shorts.iter().find(|s| &s.id == id).ok_or_else(|| {
                let ids: Vec<&str> = base.shorts.iter().map(|s| s.id.as_str()).collect();
                format!("No short {id:?}; this project has {ids:?}")
            })?;
            let captions = crate::commands::has_speech_transcript(&reader);
            let document = crate::shorts::short_document(base, short, captions)?;
            (
                document,
                format!("{} - {}", reader.summary.name, short.title),
            )
        }
    };
    // The export draws missing media as black; a render with no one watching refuses instead.
    let missing: Vec<&str> = document
        .assets
        .iter()
        .filter(|asset| !Path::new(&asset.path).exists())
        .map(|asset| asset.path.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "The project's media is missing: {}",
            missing.join(", ")
        ));
    }
    let root = reader.root().to_path_buf();
    let out = match &request.out {
        Some(path) => std::path::absolute(path).map_err(|e| format!("{}: {e}", path.display()))?,
        None => crate::export::default_destination(&root, &name, 0),
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let settings = ExportSettings {
        width: request.size.0,
        height: request.size.1,
        fps: request.fps,
        destination: Some(out.to_string_lossy().into()),
        ..ExportSettings::default()
    };
    let captured = prepare_job(&root, &name, document, settings, &mut ExportOwner::new()).map_err(
        |status| {
            status.failure.map_or_else(
                || "The export could not start".into(),
                |f| format!("{} ({})", f.message(), out.display()),
            )
        },
    )?;
    drop(reader);
    let video = run_export(
        &captured,
        &AtomicBool::new(false),
        progress,
        &EncoderGate::new(),
    )
    .map_err(|failure| format!("The export failed: {}", failure.message()))?;
    let (stills, contact_sheet) = if request.frames > 0 {
        let (stills, sheet) = snapshot(&video, request.frames)?;
        (stills, Some(sheet))
    } else {
        (Vec::new(), None)
    };
    Ok(Rendered {
        video,
        stills,
        contact_sheet,
    })
}

/// `count` PNG stills from the centres of equal slices of `video` (never a black first or last
/// frame), in `<video stem> frames/`, plus a contact sheet of them.
pub fn snapshot(video: &Path, count: u32) -> Result<(Vec<PathBuf>, PathBuf), String> {
    let ffmpeg = crate::media::ffmpeg::ffmpeg_path()?;
    let seconds = duration_s(video)?;
    let stem = video.file_stem().unwrap_or_default().to_string_lossy();
    let folder = video.with_file_name(format!("{stem} frames"));
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let run = |args: &[&std::ffi::OsStr]| -> Result<(), String> {
        let output = std::process::Command::new(&ffmpeg)
            .args(["-hide_banner", "-v", "error", "-y"])
            .args(args)
            .output()
            .map_err(|e| format!("ffmpeg: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "ffmpeg failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    };
    let mut stills = Vec::new();
    for i in 0..count {
        let at = seconds * (i as f64 + 0.5) / count as f64;
        let file = folder.join(format!("frame-{:02}.png", i + 1));
        let at = format!("{at:.3}");
        run(&[
            "-ss".as_ref(),
            at.as_ref(),
            "-i".as_ref(),
            video.as_os_str(),
            "-frames:v".as_ref(),
            "1".as_ref(),
            file.as_os_str(),
        ])?;
        stills.push(file);
    }
    let cols = count.min(3);
    let rows = count.div_ceil(cols);
    let sheet = folder.join("contact-sheet.png");
    let pattern = folder.join("frame-%02d.png");
    let filter = format!("scale=480:-2,tile={cols}x{rows}:padding=4:color=white");
    run(&[
        "-start_number".as_ref(),
        "1".as_ref(),
        "-i".as_ref(),
        pattern.as_os_str(),
        "-vf".as_ref(),
        filter.as_ref(),
        "-frames:v".as_ref(),
        "1".as_ref(),
        sheet.as_os_str(),
    ])?;
    Ok((stills, sheet))
}

fn duration_s(video: &Path) -> Result<f64, String> {
    let output = std::process::Command::new(crate::media::ffmpeg::ffprobe_path()?)
        .args(["-v", "error", "-show_entries", "format=duration"])
        .args(["-of", "default=nw=1:nk=1"])
        .arg(video)
        .output()
        .map_err(|e| format!("ffprobe: {e}"))?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| format!("ffprobe gave no duration for {}", video.display()))
}

/// The `aeroedits render` command: prints the MP4 (and stills) on stdout, progress on stderr.
pub fn run(args: &[String]) -> i32 {
    let request = match parse(args) {
        Ok(request) => request,
        Err(message) => {
            eprintln!("{message}");
            return 2;
        }
    };
    let mut shown = 0;
    let result = render(&request, |done, total| {
        let percent = if total == 0 { 0 } else { done * 100 / total };
        if percent >= shown + 10 || done == total {
            shown = percent;
            eprintln!("render: {done}/{total} frames");
        }
    });
    match result {
        Ok(rendered) => {
            println!("{}", rendered.video.display());
            for still in &rendered.stills {
                println!("{}", still.display());
            }
            if let Some(sheet) = rendered.contact_sheet {
                println!("{}", sheet.display());
            }
            0
        }
        Err(message) => {
            eprintln!("{message}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn parses_the_options() {
        let request = parse(&args(
            "proj --out a.mp4 --short s1 --size 1280x720 --fps 25 --frames 6",
        ))
        .unwrap();
        assert_eq!(
            request,
            Request {
                project: "proj".into(),
                out: Some("a.mp4".into()),
                short: Some("s1".into()),
                size: (1280, 720),
                fps: 25,
                frames: 6,
            }
        );
        let plain = parse(&args("proj")).unwrap();
        assert_eq!((plain.size, plain.fps, plain.frames), ((1920, 1080), 30, 0));
    }

    #[test]
    fn refuses_bad_options() {
        for bad in [
            "",
            "--help",
            "proj --size 720",
            "proj --fps 0",
            "proj --frames",
            "proj --wat 1",
            "proj other",
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?} should be refused");
        }
    }
}
