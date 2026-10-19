//! What renders graphics: a pinned portable Node, the pinned HyperFrames package (installed
//! with `npm ci` from the lock file shipped in the app, so every package is checked against its
//! integrity hash) and a pinned chrome-headless-shell. Downloaded on first use into the app's
//! data folder, like the Parakeet model; nothing on `PATH` is used or changed.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

pub const NODE_VERSION: &str = "22.23.3";
/// The version HyperFrames 0.8.162 itself pins (`CHROME_VERSION` in its browser manager).
pub const CHROME_VERSION: &str = "152.0.7977.30";
pub const HYPERFRAMES_VERSION: &str = "0.8.162";

const PACKAGE_JSON: &str = include_str!("../../graphics/runtime/package.json");
const PACKAGE_LOCK: &str = include_str!("../../graphics/runtime/package-lock.json");

/// One platform's downloads: URL, SHA-256, and where the program is once unpacked.
struct Download {
    url: String,
    sha256: &'static str,
    /// The folder the archive unpacks to.
    folder: String,
}

struct Platform {
    node: Download,
    /// The node program inside the node folder.
    node_exe: &'static str,
    /// npm's command-line script inside the node folder.
    npm_cli: &'static str,
    chrome: Download,
    chrome_exe: &'static str,
}

fn node(name: &str, ext: &str, sha256: &'static str) -> Download {
    let folder = format!("node-v{NODE_VERSION}-{name}");
    Download {
        url: format!("https://nodejs.org/dist/v{NODE_VERSION}/{folder}.{ext}"),
        sha256,
        folder,
    }
}

fn chrome(name: &str, sha256: &'static str) -> Download {
    let folder = format!("chrome-headless-shell-{name}");
    Download {
        url: format!(
            "https://storage.googleapis.com/chrome-for-testing-public/{CHROME_VERSION}/{name}/{folder}.zip"
        ),
        sha256,
        folder,
    }
}

fn platform() -> Result<Platform, String> {
    let windows_chrome = || {
        chrome(
            "win64",
            "5d7df999a6e4a65a1b16b25b61064f7337b8aa8ee2ed1b4e07bfdd24f6e4275e",
        )
    };
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok(Platform {
            node: node(
                "win-x64",
                "zip",
                "2b0ff57b049cda1bbcea2240eec20467018713c1efe1f7360c2681859b90ed71",
            ),
            node_exe: "node.exe",
            npm_cli: "node_modules/npm/bin/npm-cli.js",
            chrome: windows_chrome(),
            chrome_exe: "chrome-headless-shell.exe",
        }),
        // Chrome for Testing has no ARM build for Windows; the x64 one runs emulated.
        ("windows", "aarch64") => Ok(Platform {
            node: node(
                "win-arm64",
                "zip",
                "33dad22e4cef5ee8f9fbb1b0d037fdacd0e56d12a4580f0d63f68b894deab535",
            ),
            node_exe: "node.exe",
            npm_cli: "node_modules/npm/bin/npm-cli.js",
            chrome: windows_chrome(),
            chrome_exe: "chrome-headless-shell.exe",
        }),
        ("macos", "aarch64") => Ok(Platform {
            node: node(
                "darwin-arm64",
                "tar.gz",
                "23b25245dcfb9af7262f8ff142e9e2e0af025368117329e7a7458a51e5922f53",
            ),
            node_exe: "bin/node",
            npm_cli: "lib/node_modules/npm/bin/npm-cli.js",
            chrome: chrome(
                "mac-arm64",
                "1db16c16e72bedfa747ed01c207a3fe8ee7370c35f2094279eb8878c996e8c79",
            ),
            chrome_exe: "chrome-headless-shell",
        }),
        ("macos", "x86_64") => Ok(Platform {
            node: node(
                "darwin-x64",
                "tar.gz",
                "8a677b0219178efd6eb0e475457c4afb452b521a92f6e67845a73bd85727f2a8",
            ),
            node_exe: "bin/node",
            npm_cli: "lib/node_modules/npm/bin/npm-cli.js",
            chrome: chrome(
                "mac-x64",
                "48bf06f34412f9df5c7d3f3306d61a54a050789f395dd0129bae2234adb0033a",
            ),
            chrome_exe: "chrome-headless-shell",
        }),
        ("linux", "x86_64") => Ok(Platform {
            node: node(
                "linux-x64",
                "tar.xz",
                "df450af89261115ef9f9e3830c3eeb2cc9213b63c720b1af623cb5dcbe2e02de",
            ),
            node_exe: "bin/node",
            npm_cli: "lib/node_modules/npm/bin/npm-cli.js",
            chrome: chrome(
                "linux64",
                "1b150320178ecabe39726bcf3198ebb896ba2b7a07d870db1710914c42d55221",
            ),
            chrome_exe: "chrome-headless-shell",
        }),
        (os, arch) => Err(format!("Graphics can't render on {os} {arch} yet")),
    }
}

/// Where the runtime lives: `<data>/AeroEdits/graphics-runtime`, or `AEROEDITS_GRAPHICS_RUNTIME`.
pub fn runtime_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("AEROEDITS_GRAPHICS_RUNTIME") {
        return PathBuf::from(dir);
    }
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("AeroEdits")
        .join("graphics-runtime")
}

/// What is installed, written last so a half-finished install never counts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Installed {
    node: String,
    chrome: String,
    hyperframes: String,
}

impl Installed {
    fn pinned() -> Self {
        Self {
            node: NODE_VERSION.into(),
            chrome: CHROME_VERSION.into(),
            hyperframes: HYPERFRAMES_VERSION.into(),
        }
    }
}

const MARKER: &str = "installed.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    /// Ready to render.
    pub installed: bool,
    /// Something is there, but not the versions this build pins (an update is needed).
    pub outdated: bool,
    pub dir: String,
    /// What a first install downloads, roughly.
    pub download_mb: u32,
    pub hyperframes: String,
    pub node: String,
    pub chrome: String,
}

pub fn status() -> RuntimeStatus {
    let dir = runtime_dir();
    let found = read_marker(&dir);
    let installed = found.as_ref() == Some(&Installed::pinned()) && paths().is_ok();
    RuntimeStatus {
        installed,
        outdated: found.is_some() && !installed,
        dir: dir.display().to_string(),
        download_mb: 200,
        hyperframes: HYPERFRAMES_VERSION.into(),
        node: NODE_VERSION.into(),
        chrome: CHROME_VERSION.into(),
    }
}

fn read_marker(dir: &Path) -> Option<Installed> {
    let text = std::fs::read_to_string(dir.join(MARKER)).ok()?;
    serde_json::from_str(&text).ok()
}

/// The programs a render runs.
pub struct RuntimePaths {
    pub node: PathBuf,
    /// HyperFrames' command-line script.
    pub cli: PathBuf,
    pub chrome: PathBuf,
    /// Scratch space HyperFrames may fill (extracted frames, fonts).
    pub cache: PathBuf,
}

pub fn paths() -> Result<RuntimePaths, String> {
    let platform = platform()?;
    let dir = runtime_dir();
    let paths = RuntimePaths {
        node: dir.join("node").join(platform.node_exe),
        cli: dir
            .join("hyperframes")
            .join("node_modules")
            .join("hyperframes")
            .join("bin")
            .join("hyperframes.mjs"),
        chrome: dir.join("chrome").join(platform.chrome_exe),
        cache: dir.join("cache"),
    };
    if paths.node.is_file() && paths.cli.is_file() && paths.chrome.is_file() {
        Ok(paths)
    } else {
        Err(NOT_INSTALLED.into())
    }
}

pub const NOT_INSTALLED: &str =
    "The graphics renderer is not installed yet. Install it from the Graphics tab (a 200 MB download).";

/// Which part of the install is running, for the progress bar.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub step: String,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
}

/// Downloads and unpacks whatever is missing or out of date. Safe to run again after a failure
/// or a cancel: finished parts are kept.
pub fn install(
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(InstallProgress),
) -> Result<RuntimeStatus, String> {
    let platform = platform()?;
    let dir = runtime_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let found = read_marker(&dir).unwrap_or_default();
    // Anything not matching the pinned versions goes, so an update starts clean.
    let _ = std::fs::remove_file(dir.join(MARKER));
    let pinned = Installed::pinned();

    let node_dir = dir.join("node");
    if found.node != pinned.node || !node_dir.join(platform.node_exe).is_file() {
        fetch_unpacked(&platform.node, &dir, &node_dir, "Node.js", cancel, progress)?;
    }
    let chrome_dir = dir.join("chrome");
    if found.chrome != pinned.chrome || !chrome_dir.join(platform.chrome_exe).is_file() {
        fetch_unpacked(
            &platform.chrome,
            &dir,
            &chrome_dir,
            "Chrome",
            cancel,
            progress,
        )?;
    }
    let package_dir = dir.join("hyperframes");
    let cli = package_dir.join("node_modules/hyperframes/bin/hyperframes.mjs");
    if found.hyperframes != pinned.hyperframes || !cli.is_file() {
        progress(InstallProgress {
            step: "HyperFrames".into(),
            bytes_done: 0,
            bytes_total: None,
        });
        npm_ci(&node_dir, &platform, &package_dir, cancel)?;
    }
    let marker = serde_json::to_string_pretty(&pinned).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(MARKER), marker).map_err(|e| e.to_string())?;
    Ok(status())
}

/// Removes the whole runtime.
pub fn remove() -> Result<RuntimeStatus, String> {
    let dir = runtime_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    Ok(status())
}

/// Downloads `download`, checks its hash, unpacks it and moves its top folder to `target`.
fn fetch_unpacked(
    download: &Download,
    dir: &Path,
    target: &Path,
    label: &str,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(InstallProgress),
) -> Result<(), String> {
    let archive_name = download
        .url
        .rsplit('/')
        .next()
        .unwrap_or("download")
        .to_string();
    let archive = dir.join(format!("{archive_name}.part"));
    download_checked(download, &archive, label, cancel, progress)?;
    let staging = dir.join(format!("{label}.unpacking").replace(' ', "-"));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    progress(InstallProgress {
        step: format!("Unpacking {label}"),
        bytes_done: 0,
        bytes_total: None,
    });
    let unpacked = unpack(&archive, &staging);
    let _ = std::fs::remove_file(&archive);
    unpacked?;
    let inner = staging.join(&download.folder);
    if !inner.is_dir() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(format!(
            "{label}'s download did not hold {}",
            download.folder
        ));
    }
    if target.exists() {
        std::fs::remove_dir_all(target).map_err(|e| format!("{}: {e}", target.display()))?;
    }
    std::fs::rename(&inner, target).map_err(|e| format!("{}: {e}", target.display()))?;
    let _ = std::fs::remove_dir_all(&staging);
    Ok(())
}

fn download_checked(
    download: &Download,
    to: &Path,
    label: &str,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(InstallProgress),
) -> Result<(), String> {
    // As for the Parakeet model: the timeout bounds each wait, not the whole download.
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(&download.url)
        .send()
        .map_err(|e| format!("Could not download {label}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Downloading {label} failed: {}", response.status()));
    }
    let total = response.content_length();
    let mut file = std::fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(to);
            return Err("Download cancelled".into());
        }
        let n = response
            .read(&mut buf)
            .map_err(|e| format!("Download of {label} was interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        progress(InstallProgress {
            step: format!("Downloading {label}"),
            bytes_done: done,
            bytes_total: total,
        });
    }
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    let got = hex(&hasher.finalize());
    if got != download.sha256 {
        let _ = std::fs::remove_file(to);
        return Err(format!(
            "{label}'s download did not match its checksum, so it was not used"
        ));
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The system's own `tar` unpacks zip and tar archives alike (bsdtar on Windows and macOS).
fn unpack(archive: &Path, into: &Path) -> Result<(), String> {
    let tar = if cfg!(windows) {
        std::env::var_os("SystemRoot")
            .map(|root| PathBuf::from(root).join("System32").join("tar.exe"))
            .unwrap_or_else(|| PathBuf::from("tar.exe"))
    } else if Path::new("/usr/bin/tar").is_file() {
        PathBuf::from("/usr/bin/tar")
    } else {
        PathBuf::from("tar")
    };
    let mut command = Command::new(&tar);
    command
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(into)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_window(&mut command);
    let output = command
        .output()
        .map_err(|e| format!("Could not run {}: {e}", tar.display()))?;
    if !output.status.success() {
        return Err(format!(
            "Unpacking {} failed: {}",
            archive.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

/// `npm ci` with the shipped lock file: exact versions, each checked against its hash.
fn npm_ci(
    node_dir: &Path,
    platform: &Platform,
    package_dir: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    if package_dir.exists() {
        std::fs::remove_dir_all(package_dir)
            .map_err(|e| format!("{}: {e}", package_dir.display()))?;
    }
    std::fs::create_dir_all(package_dir).map_err(|e| e.to_string())?;
    std::fs::write(package_dir.join("package.json"), PACKAGE_JSON).map_err(|e| e.to_string())?;
    std::fs::write(package_dir.join("package-lock.json"), PACKAGE_LOCK)
        .map_err(|e| e.to_string())?;
    let npm_cache = package_dir.join(".npm-cache");
    let mut command = Command::new(node_dir.join(platform.node_exe));
    command
        .arg(node_dir.join(platform.npm_cli))
        .args([
            "ci",
            "--omit=dev",
            "--no-fund",
            "--no-audit",
            "--loglevel=error",
        ])
        .current_dir(package_dir)
        .env("PATH", with_node_on_path(node_dir, platform))
        .env("npm_config_cache", &npm_cache)
        .env("npm_config_update_notifier", "false")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_window(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start npm: {e}"))?;
    let mut stderr = child.stderr.take();
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(stderr) = stderr.as_mut() {
            let _ = stderr.read_to_string(&mut text);
        }
        text
    });
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Download cancelled".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let errors = reader.join().unwrap_or_default();
    let _ = std::fs::remove_dir_all(&npm_cache);
    if !status.success() {
        let tail: String = errors
            .lines()
            .rev()
            .take(6)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("Installing HyperFrames failed:\n{tail}"));
    }
    Ok(())
}

/// `PATH` with the runtime's Node first, for npm's own scripts and HyperFrames' children.
fn with_node_on_path(node_dir: &Path, platform: &Platform) -> std::ffi::OsString {
    let bin = node_dir
        .join(platform.node_exe)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| node_dir.to_path_buf());
    let mut dirs = vec![bin];
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    std::env::join_paths(dirs).unwrap_or_default()
}

/// `PATH` for a render: the runtime's Node first.
pub fn render_path() -> Result<std::ffi::OsString, String> {
    let platform = platform()?;
    Ok(with_node_on_path(&runtime_dir().join("node"), &platform))
}

pub fn hide_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lock_file_pins_hyperframes() {
        let lock: serde_json::Value = serde_json::from_str(PACKAGE_LOCK).unwrap();
        assert_eq!(
            lock["packages"]["node_modules/hyperframes"]["version"],
            HYPERFRAMES_VERSION
        );
        assert!(lock["packages"]["node_modules/hyperframes"]["integrity"]
            .as_str()
            .is_some_and(|i| i.starts_with("sha512-")));
        let package: serde_json::Value = serde_json::from_str(PACKAGE_JSON).unwrap();
        assert_eq!(package["dependencies"]["hyperframes"], HYPERFRAMES_VERSION);
    }

    #[test]
    fn this_platform_has_pinned_downloads() {
        let platform = platform().unwrap();
        for download in [&platform.node, &platform.chrome] {
            assert!(download.url.starts_with("https://"));
            assert_eq!(download.sha256.len(), 64);
            assert!(download.url.contains(&download.folder));
        }
    }

    /// The real install, about 200 MB: `AEROEDITS_GRAPHICS_RUNTIME=<empty dir> cargo test
    /// --lib graphics_runtime_installs -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn graphics_runtime_installs() {
        let cancel = AtomicBool::new(false);
        let mut last = String::new();
        let status = install(&cancel, &mut |p| {
            if p.step != last {
                eprintln!("{}", p.step);
                last = p.step.clone();
            }
        })
        .unwrap();
        assert!(status.installed, "{status:?}");
        paths().unwrap();
    }
}
