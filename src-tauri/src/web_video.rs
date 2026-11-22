//! Videos from a link (YouTube, archive.org and the other sites yt-dlp reads), downloaded into
//! the project's `media/downloads/` folder and imported like any file. yt-dlp is one program
//! from its GitHub releases, fetched on first use into the app's data folder and checked
//! against the release's SHA-256 sums; a copy next to the app, on PATH or in
//! `AEROEDITS_YTDLP` wins. Sites change often, so a failed download updates our own copy once
//! (`yt-dlp -U`) and tries again.
//!
//! The licence a site states (archive.org's Creative Commons links, YouTube's "Creative
//! Commons" flag) becomes the media's licence; anything else is `unknown` with the link as its
//! source, so `check` asks about it before a monetised upload.
use crate::sequence::{Licence, LicenceKind};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The project folder downloads are saved in.
pub const DIR: &str = "media/downloads";
pub const ENV: &str = "AEROEDITS_YTDLP";
const RELEASES: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
/// The tallest picture chosen when a site has several (1080p, else the best below it).
const MAX_HEIGHT: u32 = 1080;
const PROGRESS: &str = "aeroedits-progress ";
const FILE: &str = "aeroedits-file ";

/// What to download: `url`, or only `from_s`..`to_s` of it (cut at the nearest keyframes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Request {
    pub url: String,
    pub from_s: Option<f64>,
    pub to_s: Option<f64>,
}

/// What was saved.
#[derive(Clone, Debug, PartialEq)]
pub struct Downloaded {
    pub path: PathBuf,
    pub title: String,
    pub uploader: Option<String>,
    pub page_url: String,
    pub licence: Licence,
}

#[derive(Deserialize)]
struct Info {
    filepath: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    uploader: Option<String>,
    #[serde(default)]
    webpage_url: Option<String>,
    #[serde(default)]
    license: Option<String>,
}

/// The release file for this computer.
fn asset_name() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "aarch64") => Ok("yt-dlp_arm64.exe"),
        ("windows", _) => Ok("yt-dlp.exe"),
        ("macos", _) => Ok("yt-dlp_macos"),
        ("linux", "x86_64") => Ok("yt-dlp_linux"),
        ("linux", "aarch64") => Ok("yt-dlp_linux_aarch64"),
        (os, arch) => Err(format!(
            "Downloading from a link isn't available on {os} {arch}"
        )),
    }
}

/// Where the app keeps its own copy.
fn own_copy() -> PathBuf {
    let name = if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("AeroEdits")
        .join("tools")
        .join(name)
}

/// yt-dlp to run, and whether it is the app's own copy (only that one is updated).
fn find() -> Option<(PathBuf, bool)> {
    if let Some(found) = crate::media::ffmpeg::locate(ENV, "yt-dlp") {
        return Some((found, false));
    }
    let own = own_copy();
    own.is_file().then_some((own, true))
}

/// The sum for `name` in a SHA2-256SUMS file.
fn sum_for<'a>(sums: &'a str, name: &str) -> Option<&'a str> {
    sums.lines().find_map(|line| {
        let (sum, file) = line.split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name && sum.len() == 64).then_some(sum)
    })
}

fn install(progress: &dyn Fn(f64, &str)) -> Result<PathBuf, String> {
    let name = asset_name()?;
    progress(0.02, "Getting yt-dlp (about 35 MB, once)");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let get = |url: String| {
        client
            .get(&url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Could not download yt-dlp: {e}"))
    };
    let sums = get(format!("{RELEASES}/SHA2-256SUMS"))?
        .text()
        .map_err(|e| e.to_string())?;
    let want = sum_for(&sums, name)
        .ok_or_else(|| format!("yt-dlp's release lists no {name}"))?
        .to_ascii_lowercase();
    let mut response = get(format!("{RELEASES}/{name}"))?;
    let to = own_copy();
    let dir = to.parent().expect("a file in a folder");
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let part = to.with_extension("part");
    let mut file = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = response
            .read(&mut buf)
            .map_err(|e| format!("The yt-dlp download was interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
    }
    drop(file);
    let got: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if got != want {
        let _ = std::fs::remove_file(&part);
        return Err("yt-dlp's download did not match its checksum, so it was not used".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    std::fs::rename(&part, &to).map_err(|e| format!("{}: {e}", to.display()))?;
    Ok(to)
}

/// The arguments for `request`, saving into `dir`. `node` runs YouTube's player script
/// (the graphics runtime's Node, when installed).
fn args(request: &Request, dir: &Path, ffmpeg: Option<&Path>, node: Option<&Path>) -> Vec<String> {
    let mut args: Vec<String> = [
        "--no-playlist",
        "--newline",
        "--progress",
        "--no-mtime",
        "--windows-filenames",
        "--merge-output-format",
        "mp4",
    ]
    .map(String::from)
    .to_vec();
    // H.264 and AAC first: they decode fastest in the preview.
    args.push("-S".into());
    args.push(format!("res:{MAX_HEIGHT},vcodec:h264,acodec:aac"));
    args.push("--progress-template".into());
    args.push(format!("download:{PROGRESS}%(progress._percent_str)s"));
    args.push("--print".into());
    args.push(format!(
        "after_move:{FILE}%(.{{filepath,title,uploader,webpage_url,license}})j"
    ));
    args.push("-o".into());
    args.push(
        dir.join("%(title).80B [%(id)s].%(ext)s")
            .display()
            .to_string(),
    );
    if let Some(ffmpeg) = ffmpeg {
        args.push("--ffmpeg-location".into());
        args.push(ffmpeg.display().to_string());
    }
    if let Some(node) = node {
        args.push("--js-runtimes".into());
        args.push(format!("node:{}", node.display()));
    }
    if request.from_s.is_some() || request.to_s.is_some() {
        let from = request.from_s.unwrap_or(0.0);
        let to = request
            .to_s
            .map(|t| format!("{t:.2}"))
            .unwrap_or_else(|| "inf".into());
        args.push("--download-sections".into());
        args.push(format!("*{from:.2}-{to}"));
    }
    // After `--`, a link starting with `-` can't be read as an option.
    args.push("--".into());
    args.push(request.url.clone());
    args
}

/// The licence a site states for it.
fn licence_from(stated: Option<&str>, source: &str) -> Licence {
    let text = stated.unwrap_or("").to_ascii_lowercase();
    let kind = if text.contains("publicdomain/zero") || text.contains("cc0") {
        LicenceKind::Cc0
    } else if text.contains("publicdomain") || text.contains("public domain") {
        LicenceKind::PublicDomain
    } else if text.contains("-nc") || text.contains("noncommercial") {
        LicenceKind::NonCommercial
    } else if text.contains("creativecommons") || text.contains("creative commons") {
        LicenceKind::CreditRequired
    } else {
        LicenceKind::Unknown
    };
    let source = match stated.filter(|s| !s.trim().is_empty()) {
        Some(stated) if kind != LicenceKind::Unknown => format!("{source} ({stated})"),
        _ => source.to_string(),
    };
    Licence {
        kind,
        source: Some(source),
    }
}

/// The last line of yt-dlp's errors that says what went wrong.
fn reason(stderr: &str) -> String {
    let line = stderr
        .lines()
        .rev()
        .find(|l| l.starts_with("ERROR:"))
        .or_else(|| stderr.lines().rev().find(|l| !l.trim().is_empty()))
        .unwrap_or("yt-dlp stopped without saying why")
        .trim_start_matches("ERROR:")
        .trim();
    let mut reason = reason_text(line);
    if line.contains("Sign in") || line.contains("cookies") {
        reason.push_str(
            " The site wants a signed-in browser; this works from a home connection more often \
             than from a server.",
        );
    }
    reason
}

fn reason_text(line: &str) -> String {
    let mut text: String = line.chars().take(400).collect();
    if !text.ends_with('.') {
        text.push('.');
    }
    text
}

/// Runs `program` with `args`, reporting progress, and reads what it saved.
fn run(
    program: &Path,
    args: &[String],
    progress: &dyn Fn(f64, &str),
    stop: &dyn Fn() -> bool,
) -> Result<Info, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::graphics::runtime::hide_window(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start yt-dlp ({}): {e}", program.display()))?;
    let mut stderr = child.stderr.take().expect("piped");
    let errors = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let mut info = None;
    let stdout = BufReader::new(child.stdout.take().expect("piped"));
    for line in stdout.lines() {
        let Ok(line) = line else { break };
        if stop() {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Stopped".into());
        }
        if let Some(percent) = line.strip_prefix(PROGRESS) {
            if let Ok(p) = percent.trim().trim_end_matches('%').trim().parse::<f64>() {
                progress(0.05 + 0.85 * p / 100.0, &format!("Downloading {p:.0}%"));
            }
        } else if let Some(json) = line.strip_prefix(FILE) {
            info = serde_json::from_str::<Info>(json).ok();
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let errors = errors.join().unwrap_or_default();
    match info {
        Some(info) if status.success() => Ok(info),
        _ if !status.success() => Err(reason(&errors)),
        _ => Err("yt-dlp finished but did not say which file it saved".into()),
    }
}

/// Downloads `request` into `root`'s [`DIR`] with `program`.
pub fn download_with(
    program: &Path,
    root: &Path,
    request: &Request,
    node: Option<&Path>,
    progress: &dyn Fn(f64, &str),
    stop: &dyn Fn() -> bool,
) -> Result<Downloaded, String> {
    validate(request)?;
    let dir = crate::project::reader::safe_path(root, DIR)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let ffmpeg = crate::media::ffmpeg::ffmpeg_path().ok();
    let info = run(program, &args(request, &dir, ffmpeg, node), progress, stop)?;
    let path = PathBuf::from(&info.filepath);
    if !path.is_file() {
        return Err(format!(
            "yt-dlp said it saved {}, but it isn't there",
            info.filepath
        ));
    }
    let page_url = info.webpage_url.unwrap_or_else(|| request.url.clone());
    Ok(Downloaded {
        licence: licence_from(info.license.as_deref(), &page_url),
        title: info.title.unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        }),
        uploader: info.uploader,
        page_url,
        path,
    })
}

fn validate(request: &Request) -> Result<(), String> {
    let url = url::Url::parse(request.url.trim()).map_err(|_| "Give the video's full link")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Give an http or https link".into());
    }
    let bad = |v: Option<f64>| v.is_some_and(|v| !v.is_finite() || v < 0.0);
    if bad(request.from_s) || bad(request.to_s) {
        return Err("`fromS` and `toS` are seconds from the start".into());
    }
    if let (Some(from), Some(to)) = (request.from_s, request.to_s) {
        if to <= from {
            return Err("`toS` must be after `fromS`".into());
        }
    }
    Ok(())
}

/// Downloads `request` into `root`'s [`DIR`]: yt-dlp is fetched first if there is none, and
/// the app's own copy is updated once when a download fails.
pub fn download(
    root: &Path,
    request: &Request,
    progress: &dyn Fn(f64, &str),
    stop: &dyn Fn() -> bool,
) -> Result<Downloaded, String> {
    validate(request)?;
    let (program, own) = match find() {
        Some(found) => found,
        None => (install(progress)?, true),
    };
    let node = crate::graphics::runtime::paths().ok().map(|p| p.node);
    let first = download_with(&program, root, request, node.as_deref(), progress, stop);
    match first {
        Err(error) if own && error != "Stopped" => {
            progress(0.05, "Updating yt-dlp and trying again");
            let mut update = Command::new(&program);
            update.arg("-U").stdout(Stdio::null()).stderr(Stdio::null());
            crate::graphics::runtime::hide_window(&mut update);
            if update.status().is_ok_and(|s| s.success()) {
                download_with(&program, root, request, node.as_deref(), progress, stop)
                    .map_err(|again| format!("{again} (yt-dlp was updated and tried twice)"))
            } else {
                Err(error)
            }
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_sum_is_found_by_its_exact_name() {
        let a = "a".repeat(64);
        let b = "b".repeat(64);
        let sums = format!("{a}  yt-dlp_linux.zip\n{b}  yt-dlp_linux\n");
        assert_eq!(sum_for(&sums, "yt-dlp_linux"), Some(b.as_str()));
        assert_eq!(sum_for(&sums, "yt-dlp.exe"), None);
        assert!(asset_name().is_ok());
    }

    #[test]
    fn the_arguments_ask_for_mp4_a_section_and_the_saved_file() {
        let request = Request {
            url: "https://example.com/v?id=1".into(),
            from_s: Some(5.0),
            to_s: Some(12.5),
        };
        let args = args(
            &request,
            Path::new("/p/media/downloads"),
            Some(Path::new("/bin/ffmpeg")),
            Some(Path::new("/rt/node")),
        );
        let after = |flag: &str| {
            let i = args.iter().position(|a| a == flag).unwrap();
            args[i + 1].clone()
        };
        assert_eq!(after("--download-sections"), "*5.00-12.50");
        assert_eq!(after("--merge-output-format"), "mp4");
        assert_eq!(after("--ffmpeg-location"), "/bin/ffmpeg");
        assert_eq!(after("--js-runtimes"), "node:/rt/node");
        assert!(after("-o").starts_with("/p/media/downloads"));
        assert!(after("--print").starts_with("after_move:aeroedits-file "));
        assert_eq!(args[args.len() - 2..], ["--", "https://example.com/v?id=1"]);
        let whole = super::args(
            &Request {
                url: request.url.clone(),
                ..Default::default()
            },
            Path::new("/d"),
            None,
            None,
        );
        assert!(!whole
            .iter()
            .any(|a| a == "--download-sections" || a == "--js-runtimes"));
    }

    #[test]
    fn a_stated_licence_is_read_and_anything_else_is_unknown() {
        let kind = |s: Option<&str>| licence_from(s, "https://x").kind;
        assert_eq!(
            kind(Some("http://creativecommons.org/licenses/by/3.0/")),
            LicenceKind::CreditRequired
        );
        assert_eq!(
            kind(Some("Creative Commons Attribution license (reuse allowed)")),
            LicenceKind::CreditRequired
        );
        assert_eq!(
            kind(Some("https://creativecommons.org/licenses/by-nc/4.0/")),
            LicenceKind::NonCommercial
        );
        assert_eq!(
            kind(Some("http://creativecommons.org/publicdomain/zero/1.0/")),
            LicenceKind::Cc0
        );
        assert_eq!(
            kind(Some("http://creativecommons.org/publicdomain/mark/1.0/")),
            LicenceKind::PublicDomain
        );
        assert_eq!(kind(None), LicenceKind::Unknown);
        let unknown = licence_from(None, "https://youtu.be/x");
        assert_eq!(unknown.source.as_deref(), Some("https://youtu.be/x"));
    }

    #[test]
    fn bad_links_and_sections_are_refused_before_anything_runs() {
        let refused = |url: &str, from: Option<f64>, to: Option<f64>| {
            validate(&Request {
                url: url.into(),
                from_s: from,
                to_s: to,
            })
            .is_err()
        };
        assert!(refused("not a link", None, None));
        assert!(refused("file:///etc/passwd", None, None));
        assert!(refused("https://a.b/v", Some(10.0), Some(5.0)));
        assert!(refused("https://a.b/v", Some(-1.0), None));
        assert!(!refused("https://a.b/v", Some(1.0), None));
    }

    #[test]
    fn errors_say_the_sites_reason() {
        let stderr = "WARNING: slow\nERROR: [youtube] x: Sign in to confirm you're not a bot. Use --cookies\n";
        let said = reason(stderr);
        assert!(said.starts_with("[youtube] x: Sign in"), "{said}");
        assert!(said.contains("signed-in browser"));
    }

    /// A stand-in yt-dlp: prints progress, saves a copy of the fixture clip where `-o` says and
    /// reports it as yt-dlp does.
    #[cfg(unix)]
    #[test]
    fn a_download_is_saved_in_the_projects_downloads_with_its_licence() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let clip = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/clip.mp4");
        let fake = dir.path().join("yt-dlp");
        let script = format!(
            "#!/bin/sh\nfor a in \"$@\"; do last=\"$a\"; done\n\
             out=\"$(dirname \"$(echo \"$@\" | sed 's/.* -o \\([^ ]*\\).*/\\1/')\")\"\n\
             echo 'aeroedits-progress  42.0%'\n\
             cp '{clip}' \"$out/Clip [abc].mp4\"\n\
             printf 'aeroedits-file {{\"filepath\": \"%s\", \"title\": \"Clip\", \"uploader\": \"Me\", \"webpage_url\": \"%s\", \"license\": \"http://creativecommons.org/licenses/by/3.0/\"}}\\n' \"$out/Clip [abc].mp4\" \"$last\"\n",
            clip = clip.display()
        );
        std::fs::write(&fake, script).unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let root = dir.path().join("project");
        std::fs::create_dir_all(&root).unwrap();
        let seen = std::sync::Mutex::new(Vec::new());
        let got = download_with(
            &fake,
            &root,
            &Request {
                url: "https://archive.org/details/x".into(),
                ..Default::default()
            },
            None,
            &|p, m| seen.lock().unwrap().push((p, m.to_string())),
            &|| false,
        )
        .unwrap();
        assert_eq!(got.path, root.join(DIR).join("Clip [abc].mp4"));
        assert!(got.path.is_file());
        assert_eq!(got.title, "Clip");
        assert_eq!(got.licence.kind, LicenceKind::CreditRequired);
        assert!(got
            .licence
            .source
            .unwrap()
            .starts_with("https://archive.org/details/x"));
        assert!(seen
            .lock()
            .unwrap()
            .iter()
            .any(|(_, m)| m == "Downloading 42%"));

        std::fs::write(
            &fake,
            "#!/bin/sh\necho 'ERROR: [x] Video unavailable' >&2\nexit 1\n",
        )
        .unwrap();
        let failed = download_with(
            &fake,
            &root,
            &Request {
                url: "https://a.b/v".into(),
                ..Default::default()
            },
            None,
            &|_, _| {},
            &|| false,
        );
        assert_eq!(failed.unwrap_err(), "[x] Video unavailable.");
    }

    /// Live: fetches yt-dlp if needed and downloads 5 s of an archive.org film.
    /// `cargo test --lib web_video_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn web_video_live() {
        let dir = tempfile::tempdir().unwrap();
        let got = download(
            dir.path(),
            &Request {
                url: "https://archive.org/details/BigBuckBunny_124".into(),
                from_s: Some(5.0),
                to_s: Some(10.0),
            },
            &|p, m| eprintln!("{p:.2} {m}"),
            &|| false,
        )
        .unwrap();
        eprintln!("{got:?}");
        assert!(got.path.is_file());
    }
}
