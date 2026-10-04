//! FFmpeg subprocess backend for probing, decoding and encoding. Used on Windows and Linux,
//! and on macOS when `AEROEDITS_MEDIA_BACKEND=ffmpeg`. Frames cross a pipe as raw BGRA.
use super::{
    validate_dim, ColorInfo, PixelFormat, RateControl, VideoFrame, VideoQuality, MAX_FRAME_DIM,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, OnceLock};

/// Overrides the ffmpeg binary location.
pub const FFMPEG_ENV: &str = "AEROEDITS_FFMPEG";
/// Overrides the ffprobe binary location.
pub const FFPROBE_ENV: &str = "AEROEDITS_FFPROBE";
/// Decoded streams kept open between calls so sequential reads stay on one process. Enough
/// for the screen, the camera and a few overlays, plus the decoders started ahead of cuts.
const MAX_OPEN_STREAMS: usize = 8;
/// A decoder this many frames short of a time reads forward to it; further than that, one is
/// started there ahead of time (see [`prefetch`]).
const PREFETCH_MIN_FRAMES: u64 = 6;
/// Decoders being started ahead at once, at most. Each is an FFmpeg process seeking; more
/// than this (many tracks with cuts close together) would crowd out the decoders playing.
const MAX_PENDING_PREFETCH: usize = 4;
/// Threads each preview decoder uses.
const PREVIEW_DECODE_THREADS: u32 = 4;
/// Interactive decoding uses the GPU only for sources bigger than this (in pixels).
const SOFTWARE_PREVIEW_MAX_PIXELS: u64 = 2560 * 1600;
/// A request this far past the stream position is read forward instead of re-seeking.
const MAX_FORWARD_READ_US: u64 = 2_000_000;
/// Decode rate used when the source does not report a usable frame rate.
const FALLBACK_RATE: (u32, u32) = (30, 1);
const MAX_DECODE_RATE: u32 = 120;
/// How far before the end to seek when a request lands past the last frame.
const TAIL_SEEK_US: u64 = 1_000_000;
/// Decoded frames a stream may hold ahead of the reader, so FFmpeg keeps decoding while the
/// caller composites. Bounded in bytes: large frames get fewer slots.
const READ_AHEAD_BYTES: usize = 32 << 20;
const MAX_READ_AHEAD_FRAMES: usize = 3;
const AUDIO_BITRATE: &str = "192k";

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn locate(env_key: &str, base: &str) -> Option<PathBuf> {
    if let Some(value) = env::var_os(env_key) {
        let path = PathBuf::from(value);
        if path.is_file() {
            return Some(path);
        }
    }
    let name = exe_name(base);
    // A bundled copy next to the app wins over whatever is on PATH.
    if let Some(dir) = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        let candidate = dir.join(&name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let mut dirs: Vec<PathBuf> = env::var_os("PATH")
        .map(|paths| env::split_paths(&paths).collect())
        .unwrap_or_default();
    // Apps started from Finder do not inherit the shell PATH.
    if cfg!(target_os = "macos") {
        dirs.push("/opt/homebrew/bin".into());
        dirs.push("/usr/local/bin".into());
    }
    dirs.into_iter()
        .map(|dir| dir.join(&name))
        .find(|candidate| candidate.is_file())
}

pub fn ffmpeg_path() -> Result<&'static Path, String> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| locate(FFMPEG_ENV, "ffmpeg"))
        .as_deref()
        .ok_or_else(|| {
            format!("FFmpeg was not found. Install it and add it to PATH, or set {FFMPEG_ENV}.")
        })
}

pub fn ffprobe_path() -> Result<&'static Path, String> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| locate(FFPROBE_ENV, "ffprobe"))
        .as_deref()
        .ok_or_else(|| {
            format!(
                "ffprobe was not found. Install FFmpeg and add it to PATH, or set {FFPROBE_ENV}."
            )
        })
}

pub fn available() -> bool {
    ffmpeg_path().is_ok() && ffprobe_path().is_ok()
}

/// `file:` keeps names containing `:` or starting with `-` from being read as options or
/// protocols.
fn file_arg(path: &Path) -> OsString {
    let mut arg = OsString::from("file:");
    arg.push(path.as_os_str());
    arg
}

/// Child processes log errors to an anonymous temp file so a full pipe can never stall them.
fn command(program: &Path) -> Result<(Command, File), String> {
    let log = tempfile::tempfile().map_err(|e| format!("FFmpeg log file: {e}"))?;
    let mut cmd = Command::new(program);
    cmd.arg("-hide_banner")
        .args(["-loglevel", "error"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(
            log.try_clone()
                .map_err(|e| format!("FFmpeg log file: {e}"))?,
        ));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    Ok((cmd, log))
}

fn log_tail(log: &mut File) -> String {
    let mut text = String::new();
    if log.seek(SeekFrom::Start(0)).is_ok() {
        let _ = log.read_to_string(&mut text);
    }
    let text = text.trim();
    let start = text
        .char_indices()
        .rev()
        .nth(600)
        .map(|(index, _)| index)
        .unwrap_or(0);
    text[start..].to_string()
}

fn failure(what: &str, log: &mut File) -> String {
    let tail = log_tail(log);
    if tail.is_empty() {
        what.to_string()
    } else {
        format!("{what}: {tail}")
    }
}

fn run(mut cmd: Command, mut log: File, what: &str) -> Result<Vec<u8>, String> {
    let output = cmd
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| format!("{what}: could not start FFmpeg: {e}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(failure(what, &mut log))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    /// Frame rate as numerator/denominator.
    pub rate: (u32, u32),
}

#[derive(Deserialize)]
struct ProbeOutput {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    format: Option<ProbeFormat>,
}

#[derive(Deserialize)]
struct ProbeStream {
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    #[serde(default)]
    tags: Option<ProbeTags>,
}

#[derive(Deserialize)]
struct ProbeTags {
    title: Option<String>,
    handler_name: Option<String>,
}

#[derive(Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

fn probe(path: &Path, extra: &[&str]) -> Result<ProbeOutput, String> {
    let (mut cmd, log) = command(ffprobe_path()?)?;
    cmd.args(extra).args(["-of", "json"]).arg(file_arg(path));
    let stdout = run(cmd, log, "ffprobe failed")?;
    serde_json::from_slice(&stdout).map_err(|e| format!("Unreadable ffprobe output: {e}"))
}

fn parse_rate(value: Option<&str>) -> Option<(u32, u32)> {
    let (num, den) = value?.split_once('/')?;
    let num: u32 = num.trim().parse().ok()?;
    let den: u32 = den.trim().parse().ok()?;
    if num == 0 || den == 0 || num / den > MAX_DECODE_RATE {
        return None;
    }
    Some((num, den))
}

pub fn probe_video(path: &Path) -> Result<VideoInfo, String> {
    let output = probe(
        path,
        &[
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,avg_frame_rate,r_frame_rate",
        ],
    )?;
    let stream = output
        .streams
        .into_iter()
        .next()
        .ok_or("Media file has no video stream")?;
    let (width, height) = match (stream.width, stream.height) {
        (Some(w), Some(h)) => (w, h),
        _ => return Err("Video stream has no dimensions".into()),
    };
    validate_dim(width, height)?;
    if width > MAX_FRAME_DIM || height > MAX_FRAME_DIM {
        return Err("Video exceeds the working-set limit".into());
    }
    let rate = parse_rate(stream.avg_frame_rate.as_deref())
        .or_else(|| parse_rate(stream.r_frame_rate.as_deref()))
        .unwrap_or(FALLBACK_RATE);
    Ok(VideoInfo {
        width,
        height,
        rate,
    })
}

pub fn duration_us(path: &Path) -> Result<u64, String> {
    let output = probe(path, &["-show_entries", "format=duration"])?;
    let seconds: f64 = output
        .format
        .and_then(|format| format.duration)
        .and_then(|value| value.trim().parse().ok())
        .ok_or("Media file reports no duration")?;
    if !seconds.is_finite() || seconds < 0.0 {
        return Err("Media file reports an invalid duration".into());
    }
    Ok((seconds * 1_000_000.0).round() as u64)
}

/// Display names of the file's audio streams, in stream order: the stream's title, or
/// "Audio N" when it has none. Empty when the file has no audio.
pub fn audio_stream_names(path: &Path) -> Result<Vec<String>, String> {
    let output = probe(
        path,
        &[
            "-select_streams",
            "a",
            "-show_entries",
            "stream=index:stream_tags=title,handler_name",
        ],
    )?;
    Ok(output
        .streams
        .into_iter()
        .enumerate()
        .map(|(i, stream)| {
            stream
                .tags
                .and_then(|tags| tags.title.or(tags.handler_name))
                .map(|name| name.trim().chars().take(64).collect::<String>())
                // Containers fill handler_name with generic values; those say nothing.
                .filter(|name| {
                    !name.is_empty()
                        && !matches!(
                            name.to_ascii_lowercase().as_str(),
                            "soundhandler" | "sound handler" | "core media audio"
                        )
                        // Encoders' handler names ("#Mainconcept MP4 Sound Media Handler").
                        && !name.to_ascii_lowercase().contains("handler")
                })
                .unwrap_or_else(|| format!("Audio {}", i + 1))
        })
        .collect())
}

/// Decodes audio stream `stream` (0-based among the audio streams) of `source` to 48 kHz
/// stereo 16-bit WAV at `target`, the format the audio mixer reads.
pub fn extract_audio_wav(source: &Path, stream: usize, target: &Path) -> Result<(), String> {
    let (mut cmd, log) = command(ffmpeg_path()?)?;
    cmd.args(["-nostdin", "-y", "-i"])
        .arg(file_arg(source))
        .args([
            "-map",
            &format!("0:a:{stream}"),
            "-vn",
            "-sn",
            "-ac",
            "2",
            "-ar",
            "48000",
        ])
        .args(["-c:a", "pcm_s16le", "-f", "wav"])
        .arg(file_arg(target));
    run(cmd, log, "Extracting the audio failed").map(|_| ())
}

/// Lays audio files out on one timeline (each at its start time, silence between) and writes
/// `duration_us` of it as 48 kHz stereo 16-bit WAV: a recording's audio segments as one file.
pub fn assemble_audio_wav(
    parts: &[(PathBuf, u64)],
    duration_us: u64,
    target: &Path,
) -> Result<(), String> {
    if parts.is_empty() || parts.len() > 256 {
        return Err("A recording track needs 1 to 256 audio segments".into());
    }
    let (mut cmd, log) = command(ffmpeg_path()?)?;
    cmd.arg("-nostdin").arg("-y");
    for (path, _) in parts {
        cmd.arg("-i").arg(file_arg(path));
    }
    let mut graph = String::new();
    for (i, (_, start_us)) in parts.iter().enumerate() {
        let delay = (*start_us as u128 * 48_000 / 1_000_000) as u64;
        graph.push_str(&format!(
            "[{i}:a:0]aresample=48000,aformat=channel_layouts=stereo,adelay=delays={delay}S:all=1[a{i}];"
        ));
    }
    for i in 0..parts.len() {
        graph.push_str(&format!("[a{i}]"));
    }
    graph.push_str(&format!(
        "amix=inputs={}:normalize=0:dropout_transition=0,apad,atrim=end={}[out]",
        parts.len(),
        seconds_arg(duration_us)
    ));
    cmd.args(["-filter_complex", &graph, "-map", "[out]"])
        .args(["-c:a", "pcm_s16le", "-ar", "48000", "-ac", "2", "-f", "wav"])
        .arg(file_arg(target));
    run(cmd, log, "Joining the recording's audio failed").map(|_| ())
}

/// Copies `source` to `target` with the chapters from an FFmpeg metadata file, keeping every
/// stream and the source's own metadata.
pub fn add_chapters(source: &Path, metadata: &Path, target: &Path) -> Result<(), String> {
    let (mut cmd, log) = command(ffmpeg_path()?)?;
    cmd.args(["-nostdin", "-y", "-i"])
        .arg(file_arg(source))
        .args(["-f", "ffmetadata", "-i"])
        .arg(file_arg(metadata))
        .args([
            "-map",
            "0",
            "-map_metadata",
            "0",
            "-map_chapters",
            "1",
            "-c",
            "copy",
        ])
        .args(["-movflags", "+faststart"])
        .arg(file_arg(target));
    run(cmd, log, "Adding the chapters failed").map(|_| ())
}

fn seconds_arg(us: u64) -> String {
    format!("{}.{:06}", us / 1_000_000, us % 1_000_000)
}

/// Caps on decoded size and frame rate, so preview does not pull full-resolution frames
/// through the pipe. Zero means no limit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DecodeLimit {
    pub max_width: u32,
    pub max_height: u32,
    pub max_rate: u32,
    /// For the preview, where seeks and cuts wait on a new decoder: see [`hwaccel_for_stream`].
    pub interactive: bool,
    /// Frames come as NV12 (a third of BGRA's bytes through the pipe, which is the slowest
    /// part of decoding) for a GPU compositor to convert; otherwise BGRA.
    pub yuv: bool,
}

impl DecodeLimit {
    pub const NONE: Self = Self {
        max_width: 0,
        max_height: 0,
        max_rate: 0,
        interactive: false,
        yuv: false,
    };

    /// Output size and rate for a source, keeping its aspect ratio. NV12 needs even sizes.
    fn apply(&self, info: &VideoInfo) -> VideoInfo {
        let mut scale = 1.0f64;
        if self.max_width > 0 && info.width > self.max_width {
            scale = scale.min(self.max_width as f64 / info.width as f64);
        }
        if self.max_height > 0 && info.height > self.max_height {
            scale = scale.min(self.max_height as f64 / info.height as f64);
        }
        let even = self.yuv;
        let fit = |value: u32| {
            let v = ((value as f64 * scale).round() as u32).max(2);
            if even {
                v & !1
            } else {
                v
            }
        };
        let (num, den) = info.rate;
        let rate = if self.max_rate > 0 && num > self.max_rate.saturating_mul(den) {
            (self.max_rate, 1)
        } else {
            info.rate
        };
        VideoInfo {
            width: fit(info.width),
            height: fit(info.height),
            rate,
        }
    }
}

/// Forces a GPU decode API for preview, playback and export (`d3d11va`, `cuda`, `qsv`,
/// `videotoolbox`, ...), or `none` for software decoding.
pub const HWACCEL_ENV: &str = "AEROEDITS_HWACCEL";

/// Tried in order; the first whose test decode succeeds is used.
#[cfg(windows)]
const HWACCEL_CANDIDATES: &[&str] = &["d3d11va", "cuda", "qsv"];
#[cfg(target_os = "macos")]
const HWACCEL_CANDIDATES: &[&str] = &["videotoolbox"];
#[cfg(not(any(windows, target_os = "macos")))]
const HWACCEL_CANDIDATES: &[&str] = &[];

/// Set once a GPU decode fails at runtime; software decoding is used from then on.
static HWACCEL_BROKEN: AtomicBool = AtomicBool::new(false);
static HWACCEL_CHOSEN: OnceLock<Option<&'static str>> = OnceLock::new();

/// The GPU decode API for a decoder of `path` at `limit`. On Windows the preview decodes in
/// software: starting a D3D11/CUDA decoder costs about 300 ms more than a software one, and
/// every seek and every cut far into a clip starts one; at preview sizes the CPU also decodes
/// faster. Sources larger than 1600p still use the GPU, as does anything when it is forced.
fn hwaccel_for_stream(path: &Path, info: &VideoInfo, limit: DecodeLimit) -> Option<&'static str> {
    let forced = env::var(HWACCEL_ENV).is_ok_and(|v| !v.trim().is_empty());
    let pixels = info.width as u64 * info.height as u64;
    if cfg!(windows) && limit.interactive && !forced && pixels <= SOFTWARE_PREVIEW_MAX_PIXELS {
        return None;
    }
    hwaccel_for(path)
}

/// The GPU decode API to use, probed once per machine against `sample` (any source video).
fn hwaccel_for(sample: &Path) -> Option<&'static str> {
    if HWACCEL_BROKEN.load(Ordering::Relaxed) {
        return None;
    }
    *HWACCEL_CHOSEN.get_or_init(|| {
        let started = std::time::Instant::now();
        let chosen = choose_hwaccel(sample);
        super::profile(
            &format!("GPU decode probe ({})", chosen.unwrap_or("software")),
            started,
        );
        chosen
    })
}

fn choose_hwaccel(sample: &Path) -> Option<&'static str> {
    if let Some(forced) = env::var(HWACCEL_ENV).ok().filter(|v| !v.trim().is_empty()) {
        let forced = forced.trim().to_ascii_lowercase();
        if forced == "none" || forced == "software" {
            return None;
        }
        return Some(Box::leak(forced.into_boxed_str()));
    }
    if HWACCEL_CANDIDATES.is_empty() {
        return None;
    }
    let ffmpeg = ffmpeg_path().ok()?;
    let cache = hwaccel_cache_path();
    let key = HwaccelProbe::key_for(ffmpeg);
    if let (Some(cache), Some(key)) = (&cache, &key) {
        if let Some(cached) = HwaccelProbe::read(cache).filter(|probe| probe.matches(key)) {
            return cached.api();
        }
    }
    let listed = {
        let (mut cmd, log) = command(ffmpeg).ok()?;
        cmd.arg("-hwaccels");
        String::from_utf8_lossy(&run(cmd, log, "Listing FFmpeg hwaccels failed").ok()?).into_owned()
    };
    let chosen = HWACCEL_CANDIDATES
        .iter()
        .copied()
        .filter(|api| listed.lines().any(|line| line.trim() == *api))
        .find(|api| hwaccel_works(ffmpeg, api, sample));
    if let (Some(cache), Some(key)) = (cache, key) {
        HwaccelProbe::from_key(key, chosen).write(&cache);
    }
    chosen
}

/// Decodes two frames of `sample` with `api`. FFmpeg logs at error level only, so any log
/// output (for example "Device creation failed") counts as a failure.
fn hwaccel_works(ffmpeg: &Path, api: &str, sample: &Path) -> bool {
    let Ok((mut cmd, mut log)) = command(ffmpeg) else {
        return false;
    };
    cmd.args(["-nostdin", "-hwaccel", api, "-i"])
        .arg(file_arg(sample))
        .args(["-map", "0:v:0", "-frames:v", "2", "-f", "null", "-"]);
    let ok = cmd.status().is_ok_and(|status| status.success());
    ok && log_tail(&mut log).is_empty()
}

fn hwaccel_cache_path() -> Option<PathBuf> {
    Some(
        dirs::cache_dir()?
            .join("aeroedits")
            .join("hwaccel-probe.json"),
    )
}

/// The probe result for one FFmpeg binary, cached so later launches skip the test decode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct HwaccelProbe {
    ffmpeg: String,
    size: u64,
    modified_secs: u64,
    hwaccel: Option<String>,
}

impl HwaccelProbe {
    fn key_for(ffmpeg: &Path) -> Option<(String, u64, u64)> {
        let meta = fs::metadata(ffmpeg).ok()?;
        let modified = meta
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        Some((ffmpeg.to_string_lossy().into_owned(), meta.len(), modified))
    }

    fn from_key(key: (String, u64, u64), api: Option<&str>) -> Self {
        Self {
            ffmpeg: key.0,
            size: key.1,
            modified_secs: key.2,
            hwaccel: api.map(str::to_string),
        }
    }

    fn matches(&self, key: &(String, u64, u64)) -> bool {
        (&self.ffmpeg, self.size, self.modified_secs) == (&key.0, key.1, key.2)
    }

    /// Only APIs this build would try are trusted from the cache.
    fn api(&self) -> Option<&'static str> {
        let wanted = self.hwaccel.as_deref()?;
        HWACCEL_CANDIDATES
            .iter()
            .copied()
            .find(|api| *api == wanted)
    }

    fn read(path: &Path) -> Option<Self> {
        let bytes = fs::read(path).ok()?;
        if bytes.len() > 4096 {
            return None;
        }
        serde_json::from_slice(&bytes).ok()
    }

    fn write(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec(self) {
            let _ = fs::write(path, json);
        }
    }
}

/// Switches to software decoding for the rest of the session and forgets the cached probe,
/// so the next launch tests the GPU again.
fn disable_hwaccel(api: &str, error: &str) {
    if !HWACCEL_BROKEN.swap(true, Ordering::Relaxed) {
        eprintln!("[media] GPU decode ({api}) failed, using software decoding: {error}");
        if let Some(cache) = hwaccel_cache_path() {
            let _ = fs::remove_file(cache);
        }
    }
}

/// One ffmpeg process emitting constant-rate BGRA frames from `start_us` onward.
/// What the reader thread passes on from the decoder pipe.
enum Piped {
    Frame(Vec<u8>),
    /// The pipe closed; `partial` when it closed in the middle of a frame.
    End {
        partial: bool,
    },
    Failed(String),
}

/// Reads whole frames from the decoder pipe on a thread of its own, up to a few ahead of the
/// caller. A pipe holds far less than one frame, so without this FFmpeg could only decode
/// while the caller was waiting on it. Spent buffers come back through the second channel.
fn spawn_frame_reader(
    mut stdout: ChildStdout,
    frame_len: usize,
) -> Result<(mpsc::Receiver<Piped>, mpsc::SyncSender<Vec<u8>>), String> {
    let ahead = (READ_AHEAD_BYTES / frame_len.max(1)).clamp(1, MAX_READ_AHEAD_FRAMES);
    let (frames, received) = mpsc::sync_channel(ahead);
    let (recycle, recycled) = mpsc::sync_channel::<Vec<u8>>(ahead + 1);
    std::thread::Builder::new()
        .name("aeroedits-decode-pipe".into())
        .spawn(move || loop {
            let mut buffer = recycled.try_recv().unwrap_or_default();
            buffer.resize(frame_len, 0);
            let mut filled = 0;
            while filled < frame_len {
                match stdout.read(&mut buffer[filled..]) {
                    Ok(0) => break,
                    Ok(n) => filled += n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(e) => {
                        let _ =
                            frames.send(Piped::Failed(format!("FFmpeg decoder pipe failed: {e}")));
                        return;
                    }
                }
            }
            let piped = if filled < frame_len {
                Piped::End {
                    partial: filled > 0,
                }
            } else {
                Piped::Frame(buffer)
            };
            let end = !matches!(piped, Piped::Frame(_));
            // The stream was dropped, or the pipe is done: either way this thread is finished.
            if frames.send(piped).is_err() || end {
                return;
            }
        })
        .map_err(|e| format!("Could not start the decoder reader: {e}"))?;
    Ok((received, recycle))
}

struct FrameStream {
    path: PathBuf,
    limit: DecodeLimit,
    child: Child,
    frames: mpsc::Receiver<Piped>,
    recycle: mpsc::SyncSender<Vec<u8>>,
    log: File,
    /// The GPU decode API this process was started with, if any.
    hwaccel: Option<&'static str>,
    /// FFmpeg exited with an error or the pipe broke (not a clean end of file).
    failed: bool,
    width: u32,
    height: u32,
    rate: (u32, u32),
    start_us: u64,
    /// Index of the next frame the pipe will deliver.
    next_index: u64,
    /// The frame at `next_index - 1`, if any has been read. Shared with the frames handed out,
    /// so handing one out copies nothing.
    last: Option<crate::media::PixelBuffer>,
    eof: bool,
}

impl FrameStream {
    fn open(
        path: &Path,
        source: &VideoInfo,
        limit: DecodeLimit,
        start_us: u64,
        hwaccel: Option<&'static str>,
    ) -> Result<Self, String> {
        let info = limit.apply(source);
        let (mut cmd, log) = command(ffmpeg_path()?)?;
        let (num, den) = info.rate;
        let (width, height) = (info.width, info.height);
        cmd.arg("-nostdin");
        if let Some(api) = hwaccel {
            // Frames come back to system memory, so the filters below work unchanged.
            cmd.args(["-hwaccel", api]);
        }
        if limit.interactive {
            // Several decoders run at once while previewing (the clip playing and those started
            // ahead of cuts). Four threads decode as fast as all of them here, and leave the
            // other cores to the decoders already playing.
            cmd.args(["-threads", &PREVIEW_DECODE_THREADS.to_string()]);
        }
        cmd.args(["-ss", &seconds_arg(start_us)])
            .arg("-i")
            .arg(file_arg(path))
            .args(["-map", "0:v:0", "-an", "-sn"])
            .arg("-vf")
            .arg(if limit.yuv {
                // Limited range keeps the recording's own values (no range conversion).
                format!(
                    "fps={num}/{den},scale={width}:{height}:flags=bilinear:in_color_matrix=auto:in_range=auto:out_color_matrix=bt709:out_range=tv,format=nv12"
                )
            } else {
                format!(
                    "fps={num}/{den},scale={width}:{height}:flags=bilinear:in_color_matrix=auto:in_range=auto:out_range=full,format=bgra"
                )
            })
            .args(["-f", "rawvideo", "pipe:1"])
            .stdout(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Could not start the FFmpeg decoder: {e}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or("FFmpeg decoder has no output pipe")?;
        let frame_len = if limit.yuv {
            width as usize * height as usize * 3 / 2
        } else {
            width as usize * height as usize * 4
        };
        let (frames, recycle) = spawn_frame_reader(stdout, frame_len)?;
        Ok(Self {
            path: path.to_path_buf(),
            limit,
            child,
            frames,
            recycle,
            log,
            hwaccel,
            failed: false,
            width: info.width,
            height: info.height,
            rate: info.rate,
            start_us,
            next_index: 0,
            last: None,
            eof: false,
        })
    }

    fn time_of(&self, index: u64) -> u64 {
        let (num, den) = self.rate;
        self.start_us + (index as u128 * 1_000_000 * den as u128 / num as u128) as u64
    }

    /// Index of the frame on screen at `time_us`: the last one starting at or before it.
    fn index_at(&self, time_us: u64) -> u64 {
        let (num, den) = self.rate;
        // One microsecond of slack so frame times rounded down by `time_of` still land.
        let offset = time_us.saturating_sub(self.start_us) as u128 + 1;
        (offset * num as u128 / (1_000_000 * den as u128)) as u64
    }

    /// Whether `time_us` can be served by reading forward from the current position.
    fn can_serve(&self, time_us: u64) -> bool {
        if time_us < self.start_us {
            return false;
        }
        let target = self.index_at(time_us);
        if let Some(current) = self.next_index.checked_sub(1) {
            if target < current {
                return false;
            }
            if target == current || self.eof {
                // At the end of the file every later time shows the last frame.
                return self.last.is_some();
            }
        }
        time_us
            <= self
                .time_of(self.next_index)
                .saturating_add(MAX_FORWARD_READ_US)
    }

    /// Frames to read before `time_us` (which it can serve) is on screen.
    fn frames_to(&self, time_us: u64) -> u64 {
        if self.eof {
            return 0;
        }
        (self.index_at(time_us) + 1).saturating_sub(self.next_index)
    }

    fn read_next(&mut self) -> Result<bool, String> {
        if self.eof {
            return Ok(false);
        }
        // A reader that is gone without a word saw the pipe close.
        let piped = self.frames.recv().unwrap_or(Piped::End { partial: false });
        match piped {
            Piped::Frame(buffer) => {
                // A buffer nobody holds any more goes back to the reader to fill again.
                if let Some(spent) = self.last.replace(buffer.into()) {
                    if spent.is_unique() {
                        let _ = self.recycle.try_send(spent.into_vec());
                    }
                }
                self.next_index += 1;
                Ok(true)
            }
            Piped::End { partial } => {
                self.eof = true;
                let status = self.child.wait().ok();
                if !partial && status.is_some_and(|s| s.success()) {
                    // A clean end of stream: keep showing the last frame.
                    return Ok(false);
                }
                self.failed = true;
                Err(failure("FFmpeg decoder stopped early", &mut self.log))
            }
            Piped::Failed(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }

    fn frame_at(&mut self, time_us: u64) -> Result<VideoFrame, String> {
        let target = self.index_at(time_us);
        while self.next_index <= target {
            if !self.read_next()? {
                break;
            }
        }
        let data = self
            .last
            .clone()
            .ok_or_else(|| format!("No video frame at {}us", time_us))?;
        let (stride, format) = if self.limit.yuv {
            (self.width, PixelFormat::Nv12)
        } else {
            (self.width * 4, PixelFormat::Bgra8888)
        };
        Ok(VideoFrame {
            pts_us: self.time_of(self.next_index.saturating_sub(1)),
            width: self.width,
            height: self.height,
            stride,
            format,
            color: ColorInfo::rec709_full(),
            data,
        })
    }
}

impl Drop for FrameStream {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct DecoderCache {
    info: Vec<(PathBuf, VideoInfo)>,
    /// Most recently used first.
    streams: Vec<FrameStream>,
    /// Decoders being started ahead of time: file, limit and start.
    pending: Vec<(PathBuf, DecodeLimit, u64)>,
}

static DECODERS: Mutex<DecoderCache> = Mutex::new(DecoderCache {
    info: Vec::new(),
    streams: Vec::new(),
    pending: Vec::new(),
});

fn cached_info(path: &Path) -> Result<VideoInfo, String> {
    if let Some((_, info)) = DECODERS.lock().info.iter().find(|(p, _)| p == path) {
        return Ok(info.clone());
    }
    let info = probe_video(path)?;
    let mut cache = DECODERS.lock();
    if cache.info.len() >= 64 {
        cache.info.remove(0);
    }
    cache.info.push((path.to_path_buf(), info.clone()));
    Ok(info)
}

/// Keeps `stream` as the most recently used. Returns the streams that no longer fit, to be
/// stopped with [`stop_streams`] once the lock is released.
fn keep_stream(cache: &mut DecoderCache, stream: FrameStream) -> Vec<FrameStream> {
    cache.streams.insert(0, stream);
    if cache.streams.len() > MAX_OPEN_STREAMS {
        cache.streams.split_off(MAX_OPEN_STREAMS)
    } else {
        Vec::new()
    }
}

/// Stops decoders on a thread of their own: waiting for a killed FFmpeg to exit takes long
/// enough on Windows to hold up the frame being drawn.
fn stop_streams(streams: Vec<FrameStream>) {
    if streams.is_empty() {
        return;
    }
    // Without a thread to spare, the closure and its streams are dropped right here.
    let _ = std::thread::Builder::new()
        .name("aeroedits-decoder-stop".into())
        .spawn(move || drop(streams));
}

/// The open decoder of `path` that reaches `time_us` with the least reading, taken out.
fn take_stream(path: &Path, limit: DecodeLimit, time_us: u64) -> Option<FrameStream> {
    let mut cache = DECODERS.lock();
    let best = cache
        .streams
        .iter()
        .enumerate()
        .filter(|(_, s)| s.path == path && s.limit == limit && s.can_serve(time_us))
        .min_by_key(|(_, s)| s.frames_to(time_us))
        .map(|(index, _)| index)?;
    Some(cache.streams.remove(best))
}

/// Starts a decoder of `path` at `time_us` in the background, unless an open one already
/// gets there within a few frames. The playback worker calls it for the clip edges coming
/// up, so a cut far into a clip plays from a decoder that is already there.
/// Returns false when it is turned away (too many being started), to be asked again later.
pub fn prefetch(path: &Path, time_us: u64, limit: DecodeLimit) -> bool {
    {
        let mut cache = DECODERS.lock();
        let ready = cache.streams.iter().any(|s| {
            s.path == path
                && s.limit == limit
                && s.can_serve(time_us)
                && s.frames_to(time_us) <= PREFETCH_MIN_FRAMES
        });
        let pending = cache
            .pending
            .iter()
            .any(|(p, l, t)| p == path && *l == limit && *t == time_us);
        if ready || pending {
            return true;
        }
        if cache.pending.len() >= MAX_PENDING_PREFETCH {
            return false;
        }
        cache.pending.push((path.to_path_buf(), limit, time_us));
    }
    let owned = path.to_path_buf();
    let started = std::thread::Builder::new()
        .name("aeroedits-prefetch".into())
        .spawn(move || {
            let opened = cached_info(&owned).and_then(|info| {
                let hwaccel = hwaccel_for_stream(&owned, &info, limit);
                FrameStream::open(&owned, &info, limit, time_us, hwaccel)
            });
            let evicted = {
                let mut cache = DECODERS.lock();
                cache
                    .pending
                    .retain(|(p, l, t)| !(p == &owned && *l == limit && *t == time_us));
                match opened {
                    Ok(stream) => keep_stream(&mut cache, stream),
                    Err(_) => Vec::new(),
                }
            };
            stop_streams(evicted);
        });
    if started.is_err() {
        DECODERS
            .lock()
            .pending
            .retain(|(p, l, t)| !(p == path && *l == limit && *t == time_us));
    }
    true
}

/// Decodes the frame shown at `time_us` (relative to the start of the file) as BGRA.
pub fn decode_bgra(path: &Path, time_us: u64) -> Result<VideoFrame, String> {
    decode_bgra_limited(path, time_us, DecodeLimit::NONE)
}

/// Like [`decode_bgra`], scaled down and rate-capped to `limit`.
pub fn decode_bgra_limited(
    path: &Path,
    time_us: u64,
    limit: DecodeLimit,
) -> Result<VideoFrame, String> {
    let mut reused = None;
    if let Some(mut stream) = take_stream(path, limit, time_us) {
        let frame = stream.frame_at(time_us);
        // A decoder started ahead that failed before its first frame (a GPU that does not
        // work, say) is replaced below by a fresh one, which can fall back to software.
        if frame.is_ok() || stream.next_index > 0 || !stream.failed {
            reused = Some((stream, frame));
        }
    }
    let (mut stream, mut frame) = match reused {
        Some(found) => found,
        None => {
            let started = std::time::Instant::now();
            let info = cached_info(path)?;
            let hwaccel = hwaccel_for_stream(path, &info, limit);
            let mut stream = FrameStream::open(path, &info, limit, time_us, hwaccel)?;
            let mut frame = stream.frame_at(time_us);
            if let (Err(error), Some(api)) = (&frame, stream.hwaccel) {
                // Only a failed process, not a seek past the last frame, rules out the GPU.
                if stream.failed && stream.next_index == 0 {
                    disable_hwaccel(api, error);
                    stream = FrameStream::open(path, &info, limit, time_us, None)?;
                    frame = stream.frame_at(time_us);
                }
            }
            super::profile(
                &format!(
                    "decoder seek (new FFmpeg process) {} at {time_us}",
                    path.display()
                ),
                started,
            );
            (stream, frame)
        }
    };
    if frame.is_err() && stream.next_index == 0 && stream.eof {
        // Seeking past the last frame yields nothing; hold the final frame instead.
        let info = cached_info(path)?;
        let tail_start = duration_us(path)?.saturating_sub(TAIL_SEEK_US);
        stream = FrameStream::open(
            path,
            &info,
            limit,
            tail_start.min(time_us),
            hwaccel_for_stream(path, &info, limit),
        )?;
        frame = stream.frame_at(time_us);
    }
    if frame.is_ok() {
        let evicted = keep_stream(&mut DECODERS.lock(), stream);
        stop_streams(evicted);
    }
    frame
}

/// Stops every cached decoder process, e.g. once an export finishes.
pub fn release_decoders() {
    let streams = std::mem::take(&mut DECODERS.lock().streams);
    drop(streams);
}

/// Forces one H.264 encoder by FFmpeg name, e.g. `libx264` or `h264_nvenc`.
pub const ENCODER_ENV: &str = "AEROEDITS_H264_ENCODER";

/// Which rate-control options an encoder understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EncoderFamily {
    Nvenc,
    Amf,
    Qsv,
    X264,
    /// Takes only `-b:v`.
    BitrateOnly,
}

#[derive(Clone, Debug)]
struct H264Encoder {
    name: &'static str,
    family: EncoderFamily,
    /// GPU encoders are listed by any build that supports them, so they are only used after a
    /// one-frame test encode succeeds on this machine.
    hardware: bool,
}

/// In order of preference: GPU encoders first, then libx264, then the remaining fallbacks.
const H264_ENCODERS: &[H264Encoder] = &[
    H264Encoder {
        name: "h264_nvenc",
        family: EncoderFamily::Nvenc,
        hardware: true,
    },
    H264Encoder {
        name: "h264_amf",
        family: EncoderFamily::Amf,
        hardware: true,
    },
    H264Encoder {
        name: "h264_qsv",
        family: EncoderFamily::Qsv,
        hardware: true,
    },
    H264Encoder {
        name: "libx264",
        family: EncoderFamily::X264,
        hardware: false,
    },
    H264Encoder {
        name: "h264_mf",
        family: EncoderFamily::BitrateOnly,
        hardware: false,
    },
    H264Encoder {
        name: "h264_videotoolbox",
        family: EncoderFamily::BitrateOnly,
        hardware: false,
    },
    H264Encoder {
        name: "libopenh264",
        family: EncoderFamily::BitrateOnly,
        hardware: false,
    },
];

impl H264Encoder {
    /// FFmpeg options for `rate` at this size and frame rate.
    fn rate_args(&self, rate: RateControl, width: u32, height: u32, fps: u32) -> Vec<String> {
        match (self.family, rate) {
            (EncoderFamily::Nvenc, RateControl::Quality(quality)) => {
                let cq = match quality {
                    VideoQuality::Standard => "23",
                    VideoQuality::High => "19",
                    VideoQuality::Max => "15",
                };
                [
                    "-preset", "p5", "-tune", "hq", "-rc", "vbr", "-cq", cq, "-b:v", "0",
                ]
                .map(String::from)
                .to_vec()
            }
            (EncoderFamily::Amf, RateControl::Quality(quality)) => {
                let (qp_i, qp_p) = match quality {
                    VideoQuality::Standard => ("22", "24"),
                    VideoQuality::High => ("18", "20"),
                    VideoQuality::Max => ("14", "16"),
                };
                [
                    "-quality", "quality", "-rc", "cqp", "-qp_i", qp_i, "-qp_p", qp_p,
                ]
                .map(String::from)
                .to_vec()
            }
            (EncoderFamily::Qsv, RateControl::Quality(quality)) => {
                let global = match quality {
                    VideoQuality::Standard => "24",
                    VideoQuality::High => "20",
                    VideoQuality::Max => "16",
                };
                ["-preset", "medium", "-global_quality", global]
                    .map(String::from)
                    .to_vec()
            }
            (EncoderFamily::X264, RateControl::Quality(quality)) => {
                let crf = match quality {
                    VideoQuality::Standard => "21",
                    VideoQuality::High => "17",
                    VideoQuality::Max => "14",
                };
                ["-preset", "fast", "-crf", crf].map(String::from).to_vec()
            }
            (family, rate) => {
                let bps = rate.target_bps(width, height, fps);
                let mut args = match family {
                    EncoderFamily::Nvenc => ["-preset", "p5", "-tune", "hq", "-rc", "vbr"]
                        .map(String::from)
                        .to_vec(),
                    EncoderFamily::Amf => ["-quality", "quality", "-rc", "vbr_peak"]
                        .map(String::from)
                        .to_vec(),
                    EncoderFamily::Qsv => vec!["-preset".into(), "medium".into()],
                    EncoderFamily::X264 => vec!["-preset".into(), "fast".into()],
                    EncoderFamily::BitrateOnly => Vec::new(),
                };
                args.extend(["-b:v".into(), bps.to_string()]);
                if family != EncoderFamily::BitrateOnly {
                    args.extend([
                        "-maxrate".into(),
                        (bps + bps / 2).to_string(),
                        "-bufsize".into(),
                        (bps * 2).to_string(),
                    ]);
                }
                args
            }
        }
    }
}

/// Encodes one frame at `width`x`height` to check the encoder's device and driver are present
/// and accept that size (NVENC, for one, refuses frames below a minimum size).
fn encoder_works(ffmpeg: &Path, encoder: &H264Encoder, width: u32, height: u32) -> bool {
    let Ok((mut cmd, log)) = command(ffmpeg) else {
        return false;
    };
    cmd.args([
        "-f",
        "lavfi",
        "-i",
        &format!("color=black:size={width}x{height}:rate=30"),
    ])
    .args([
        "-frames:v",
        "1",
        "-vf",
        "format=yuv420p",
        "-c:v",
        encoder.name,
    ])
    .args(encoder.rate_args(RateControl::default(), width, height, 30))
    .args(["-f", "null", "-"]);
    run(cmd, log, "Test encode failed").is_ok()
}

/// The size the first test encode uses; it shows whether a GPU encoder works at all.
const PROBE_DIM: u32 = 256;

/// The usable H.264 encoders, in order of preference: present in this FFmpeg build and, for
/// GPU encoders, able to encode a frame on this machine. Only the forced one, when forced.
fn h264_encoders() -> Result<&'static [&'static H264Encoder], String> {
    static ENCODERS: OnceLock<Result<Vec<&'static H264Encoder>, String>> = OnceLock::new();
    ENCODERS
        .get_or_init(|| {
            let ffmpeg = ffmpeg_path()?;
            let (mut cmd, log) = command(ffmpeg)?;
            cmd.arg("-encoders");
            let listing =
                String::from_utf8_lossy(&run(cmd, log, "Listing FFmpeg encoders failed")?)
                    .into_owned();
            let has = |name: &str| {
                listing
                    .lines()
                    .any(|line| line.split_whitespace().nth(1) == Some(name))
            };
            if let Some(forced) = env::var(ENCODER_ENV).ok().filter(|v| !v.is_empty()) {
                return H264_ENCODERS
                    .iter()
                    .find(|encoder| encoder.name == forced && has(encoder.name))
                    .map(|encoder| vec![encoder])
                    .ok_or_else(|| format!("{ENCODER_ENV}={forced} is not available"));
            }
            let usable: Vec<_> = H264_ENCODERS
                .iter()
                .filter(|encoder| has(encoder.name))
                .filter(|encoder| {
                    !encoder.hardware || encoder_works(ffmpeg, encoder, PROBE_DIM, PROBE_DIM)
                })
                .collect();
            if usable.is_empty() {
                Err("This FFmpeg build has no H.264 encoder".to_string())
            } else {
                Ok(usable)
            }
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

/// The preferred encoder that accepts `width`x`height` frames. GPU encoders have size limits
/// of their own, so each is checked once per size; libx264 takes any size.
fn h264_encoder_for(width: u32, height: u32) -> Result<&'static H264Encoder, String> {
    static SIZES: Mutex<Vec<(&'static str, u32, u32, bool)>> = Mutex::new(Vec::new());
    let encoders = h264_encoders()?;
    // A forced encoder is used as is, so its own error shows if it cannot take the size.
    if encoders.len() == 1 {
        return Ok(encoders[0]);
    }
    let ffmpeg = ffmpeg_path()?;
    for &encoder in encoders {
        if !encoder.hardware || (width, height) == (PROBE_DIM, PROBE_DIM) {
            return Ok(encoder);
        }
        let known = SIZES
            .lock()
            .iter()
            .find(|(name, w, h, _)| *name == encoder.name && (*w, *h) == (width, height))
            .map(|entry| entry.3);
        let works = known.unwrap_or_else(|| {
            let works = encoder_works(ffmpeg, encoder, width, height);
            let mut sizes = SIZES.lock();
            if sizes.len() >= 64 {
                sizes.remove(0);
            }
            sizes.push((encoder.name, width, height, works));
            works
        });
        if works {
            return Ok(encoder);
        }
    }
    Err(format!(
        "No H.264 encoder here can encode {width}x{height} video"
    ))
}

pub fn encoder_name() -> Option<&'static str> {
    h264_encoders()
        .ok()
        .and_then(|encoders| encoders.first())
        .map(|encoder| encoder.name)
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

/// Streams BGRA frames into an H.264 video track and PCM into a side file, then muxes them
/// with AAC audio into one MP4 on `finish`.
pub struct FfmpegExport {
    path: PathBuf,
    width: u32,
    height: u32,
    sample_rate: u32,
    channels: u16,
    child: Option<Child>,
    stdin: Option<BufWriter<ChildStdin>>,
    log: File,
    video_path: PathBuf,
    audio: Option<(PathBuf, BufWriter<File>)>,
    row: Vec<u8>,
}

impl FfmpegExport {
    pub fn begin(
        path: &Path,
        width: u32,
        height: u32,
        fps: u32,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Self, String> {
        Self::begin_with_rate(
            path,
            width,
            height,
            fps,
            sample_rate,
            channels,
            RateControl::default(),
        )
    }

    pub fn begin_with_rate(
        path: &Path,
        width: u32,
        height: u32,
        fps: u32,
        sample_rate: u32,
        channels: u16,
        rate: RateControl,
    ) -> Result<Self, String> {
        validate_dim(width, height)?;
        if width % 2 != 0 || height % 2 != 0 {
            return Err("H.264 export requires even dimensions".into());
        }
        if width > MAX_FRAME_DIM || height > MAX_FRAME_DIM {
            return Err("Export canvas exceeds the working-set limit".into());
        }
        let fps = fps.max(1);
        let encoder = h264_encoder_for(width, height)?;
        let video_path = sibling(path, ".video.mp4");
        let (mut cmd, log) = command(ffmpeg_path()?)?;
        cmd.args(["-nostdin", "-y", "-f", "rawvideo", "-pix_fmt", "bgra"])
            .args(["-video_size", &format!("{width}x{height}")])
            .args(["-framerate", &fps.to_string()])
            .args(["-i", "pipe:0", "-an"])
            .args([
                "-vf",
                "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p",
            ])
            .args(["-c:v", encoder.name])
            .args(encoder.rate_args(rate, width, height, fps));
        cmd.args([
            "-colorspace",
            "bt709",
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
            "-color_range",
            "tv",
            "-f",
            "mp4",
        ])
        .arg(file_arg(&video_path))
        .stdin(Stdio::piped());
        let audio = if sample_rate > 0 && channels > 0 {
            let audio_path = sibling(path, ".audio.pcm");
            let file = File::create(&audio_path)
                .map_err(|e| format!("Could not create the export audio file: {e}"))?;
            Some((audio_path, BufWriter::new(file)))
        } else {
            None
        };
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Could not start the FFmpeg encoder: {e}"))?;
        let stdin = child.stdin.take().map(BufWriter::new);
        Ok(Self {
            path: path.to_path_buf(),
            width,
            height,
            sample_rate,
            channels,
            child: Some(child),
            stdin,
            log,
            video_path,
            audio,
            row: Vec::new(),
        })
    }

    pub fn write_video(&mut self, frame: &VideoFrame) -> Result<(), String> {
        if frame.width != self.width || frame.height != self.height {
            return Err("Export frame geometry does not match the session".into());
        }
        let row_len = self.width as usize * 4;
        let stride = frame.stride as usize;
        if stride < row_len || frame.data.len() < stride * (self.height as usize - 1) + row_len {
            return Err("Export frame buffer is truncated".into());
        }
        let stdin = self.stdin.as_mut().ok_or("Export session is closed")?;
        let result = if stride == row_len {
            stdin.write_all(&frame.data[..row_len * self.height as usize])
        } else {
            self.row.clear();
            for y in 0..self.height as usize {
                self.row
                    .extend_from_slice(&frame.data[y * stride..y * stride + row_len]);
            }
            stdin.write_all(&self.row)
        };
        result.map_err(|_| failure("FFmpeg encoder stopped accepting frames", &mut self.log))
    }

    pub fn write_audio(&mut self, pcm: &[i16]) -> Result<(), String> {
        let (_, file) = self
            .audio
            .as_mut()
            .ok_or("Export session has no audio track")?;
        let mut bytes = Vec::with_capacity(pcm.len() * 2);
        for sample in pcm {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        file.write_all(&bytes)
            .map_err(|e| format!("Could not write export audio: {e}"))
    }

    pub fn finish(mut self) -> Result<(), String> {
        let mut stdin = self.stdin.take().ok_or("Export session is closed")?;
        let flushed = stdin.flush();
        drop(stdin);
        let mut child = self.child.take().ok_or("Export session is closed")?;
        let status = child
            .wait()
            .map_err(|e| format!("FFmpeg encoder did not finish: {e}"))?;
        if flushed.is_err() || !status.success() {
            return Err(failure("FFmpeg video encode failed", &mut self.log));
        }
        match self.audio.take() {
            None => fs::rename(&self.video_path, &self.path)
                .map_err(|e| format!("Could not move the encoded video: {e}")),
            Some((audio_path, mut file)) => {
                let result = file
                    .flush()
                    .map_err(|e| format!("Could not write export audio: {e}"))
                    .and_then(|_| self.mux(&audio_path));
                drop(file);
                let _ = fs::remove_file(&audio_path);
                result
            }
        }
    }

    fn mux(&self, audio_path: &Path) -> Result<(), String> {
        let (mut cmd, log) = command(ffmpeg_path()?)?;
        cmd.args(["-nostdin", "-y", "-i"])
            .arg(file_arg(&self.video_path))
            .args(["-f", "s16le", "-ar", &self.sample_rate.to_string()])
            .args(["-ac", &self.channels.to_string(), "-i"])
            .arg(file_arg(audio_path))
            .args(["-map", "0:v:0", "-map", "1:a:0", "-c:v", "copy"])
            .args(["-c:a", "aac", "-b:a", AUDIO_BITRATE])
            .args(["-movflags", "+faststart", "-f", "mp4"])
            .arg(file_arg(&self.path));
        run(cmd, log, "FFmpeg audio mux failed").map(|_| ())
    }
}

impl Drop for FfmpegExport {
    fn drop(&mut self) {
        drop(self.stdin.take());
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some((audio_path, file)) = self.audio.take() {
            drop(file);
            let _ = fs::remove_file(audio_path);
        }
        let _ = fs::remove_file(&self.video_path);
    }
}

/// Encodes `frames` as a short H.264 MP4 with no audio.
pub fn encode_bgra_mp4(path: &Path, frames: &[VideoFrame], fps: u32) -> Result<(), String> {
    let first = frames
        .first()
        .ok_or("Encoder requires at least one frame")?;
    let mut session = FfmpegExport::begin(path, first.width, first.height, fps, 0, 0)?;
    for frame in frames {
        session.write_video(frame)?;
    }
    session.finish()
}

/// Writes one second of a solid colour, for fixtures and parity checks.
pub fn write_solid_mp4(
    path: &Path,
    width: u32,
    height: u32,
    r: f32,
    g: f32,
    b: f32,
) -> Result<(), String> {
    let to_u8 = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let frame = VideoFrame::solid(width, height, to_u8(b), to_u8(g), to_u8(r), 0)?;
    encode_bgra_mp4(path, &vec![frame; 30], 30)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CI sets this so a missing FFmpeg fails the run instead of skipping media tests.
    const REQUIRE_ENV: &str = "AEROEDITS_REQUIRE_FFMPEG";

    fn encoder(name: &str) -> &'static H264Encoder {
        H264_ENCODERS.iter().find(|e| e.name == name).unwrap()
    }

    fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        let at = args.iter().position(|arg| arg == flag)?;
        args.get(at + 1).map(String::as_str)
    }

    #[test]
    fn quality_presets_map_to_each_encoders_constant_quality_option() {
        let high = RateControl::Quality(VideoQuality::High);
        let max = RateControl::Quality(VideoQuality::Max);
        let x264 = encoder("libx264").rate_args(high, 1920, 1080, 30);
        assert_eq!(value_after(&x264, "-crf"), Some("17"));
        assert_eq!(
            value_after(&encoder("libx264").rate_args(max, 1920, 1080, 30), "-crf"),
            Some("14")
        );
        let nvenc = encoder("h264_nvenc").rate_args(high, 1920, 1080, 30);
        assert_eq!(value_after(&nvenc, "-cq"), Some("19"));
        assert_eq!(value_after(&nvenc, "-b:v"), Some("0"));
        let amf = encoder("h264_amf").rate_args(max, 1920, 1080, 30);
        assert_eq!(value_after(&amf, "-qp_i"), Some("14"));
        let qsv = encoder("h264_qsv").rate_args(high, 1920, 1080, 30);
        assert_eq!(value_after(&qsv, "-global_quality"), Some("20"));
        // Bitrate-only encoders get a bitrate scaled from the preset.
        let mf = encoder("h264_mf").rate_args(high, 1920, 1080, 30);
        let bps: u64 = value_after(&mf, "-b:v").unwrap().parse().unwrap();
        assert_eq!(bps, (1920.0 * 1080.0 * 30.0 * 0.15) as u64);
    }

    #[test]
    fn custom_bitrate_sets_average_and_peak_on_every_encoder() {
        let rate = RateControl::Bitrate(20_000_000);
        for e in H264_ENCODERS {
            let args = e.rate_args(rate, 1920, 1080, 60);
            assert_eq!(value_after(&args, "-b:v"), Some("20000000"), "{}", e.name);
            assert!(
                !args.iter().any(|a| a == "-crf" || a == "-cq"),
                "{}",
                e.name
            );
            if e.family != EncoderFamily::BitrateOnly {
                assert_eq!(
                    value_after(&args, "-maxrate"),
                    Some("30000000"),
                    "{}",
                    e.name
                );
            }
        }
    }

    #[test]
    fn hwaccel_probe_cache_round_trips_and_trusts_only_known_apis() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("nested").join("hwaccel-probe.json");
        let key = ("/opt/ffmpeg".to_string(), 1234, 99);
        let probe = HwaccelProbe::from_key(key.clone(), Some("cuda"));
        probe.write(&cache);
        let read = HwaccelProbe::read(&cache).unwrap();
        assert_eq!(read, probe);
        assert!(read.matches(&key));
        // A different FFmpeg binary (size or date) needs a new probe.
        assert!(!read.matches(&("/opt/ffmpeg".to_string(), 1235, 99)));
        // Only APIs this platform would try are trusted.
        let expected = HWACCEL_CANDIDATES.contains(&"cuda").then_some("cuda");
        assert_eq!(read.api(), expected);
        assert_eq!(HwaccelProbe::from_key(key, Some("bogus")).api(), None);
        fs::write(&cache, b"not json").unwrap();
        assert!(HwaccelProbe::read(&cache).is_none());
    }

    fn ffmpeg_or_skip() -> bool {
        if available() {
            return true;
        }
        assert!(
            env::var_os(REQUIRE_ENV).is_none(),
            "{REQUIRE_ENV} is set but FFmpeg was not found"
        );
        eprintln!("FFmpeg not found; skipping");
        false
    }

    /// Frame `i` is a distinct grey level so tests can tell which frame came back.
    fn numbered_frames(count: u8, width: u32, height: u32) -> Vec<VideoFrame> {
        (0..count)
            .map(|i| {
                let level = 20 + i * (200 / count.max(1));
                VideoFrame::solid(width, height, level, level, level, 0).unwrap()
            })
            .collect()
    }

    fn mean_level(frame: &VideoFrame) -> f32 {
        let sum: u64 = frame.data.chunks_exact(4).map(|px| u64::from(px[1])).sum();
        sum as f32 / (frame.width * frame.height) as f32
    }

    #[test]
    fn parses_rates_and_rejects_nonsense() {
        assert_eq!(parse_rate(Some("30000/1001")), Some((30000, 1001)));
        assert_eq!(parse_rate(Some("60/1")), Some((60, 1)));
        assert_eq!(parse_rate(Some("0/0")), None);
        assert_eq!(parse_rate(Some("90000/1")), None);
        assert_eq!(parse_rate(None), None);
        assert_eq!(seconds_arg(1_500_000), "1.500000");
        assert_eq!(seconds_arg(42), "0.000042");
    }

    #[test]
    fn solid_colour_round_trips() {
        if !ffmpeg_or_skip() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("solid.mp4");
        write_solid_mp4(&path, 64, 48, 0.92, 0.12, 0.10).unwrap();
        let info = probe_video(&path).unwrap();
        assert_eq!((info.width, info.height), (64, 48));
        let duration = duration_us(&path).unwrap();
        assert!(duration.abs_diff(1_000_000) < 50_000, "duration {duration}");
        let frame = decode_bgra(&path, 500_000).unwrap();
        assert_eq!((frame.width, frame.height, frame.stride), (64, 48, 256));
        let centre = ((24 * 64 + 32) * 4) as usize;
        let bgr = &frame.data[centre..centre + 3];
        let expected = [26u8, 31, 235];
        for (got, want) in bgr.iter().zip(expected) {
            assert!(
                got.abs_diff(want) <= 12,
                "decoded {bgr:?}, expected {expected:?}"
            );
        }
        release_decoders();
    }

    #[test]
    fn sequential_and_random_access_return_the_right_frames() {
        if !ffmpeg_or_skip() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ramp.mp4");
        let frames = numbered_frames(10, 32, 32);
        encode_bgra_mp4(&path, &frames, 10).unwrap();
        let expected: Vec<f32> = frames.iter().map(mean_level).collect();
        let check = |time_us: u64, index: usize| {
            let frame = decode_bgra(&path, time_us).unwrap();
            let level = mean_level(&frame);
            assert!(
                (level - expected[index]).abs() < 6.0,
                "at {time_us}us got level {level}, expected frame {index} ({})",
                expected[index]
            );
        };
        // Forward reads, including a time between frames and a skipped frame.
        check(0, 0);
        check(100_000, 1);
        check(150_000, 1);
        check(300_000, 3);
        // Backwards seek, then the same frame twice.
        check(200_000, 2);
        check(200_000, 2);
        // Past the end holds the last frame.
        check(900_000, 9);
        check(5_000_000, 9);
        release_decoders();
    }

    #[test]
    fn limits_keep_aspect_and_cap_rate() {
        let source = VideoInfo {
            width: 3840,
            height: 2160,
            rate: (60, 1),
        };
        let limit = DecodeLimit {
            max_width: 1280,
            max_height: 1280,
            max_rate: 30,
            interactive: false,
            yuv: false,
        };
        assert_eq!(
            limit.apply(&source),
            VideoInfo {
                width: 1280,
                height: 720,
                rate: (30, 1)
            }
        );
        assert_eq!(DecodeLimit::NONE.apply(&source), source);
        let small = VideoInfo {
            width: 640,
            height: 480,
            rate: (24000, 1001),
        };
        assert_eq!(limit.apply(&small), small);
    }

    #[test]
    fn limited_decode_scales_frames() {
        if !ffmpeg_or_skip() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.mp4");
        write_solid_mp4(&path, 128, 64, 0.1, 0.8, 0.1).unwrap();
        let limit = DecodeLimit {
            max_width: 32,
            max_height: 32,
            max_rate: 10,
            interactive: true,
            yuv: false,
        };
        let frame = decode_bgra_limited(&path, 250_000, limit).unwrap();
        assert_eq!((frame.width, frame.height), (32, 16));
        // A fresh stream starts at the requested time; the next frame is one 10 fps step on.
        assert_eq!(frame.pts_us, 250_000);
        let next = decode_bgra_limited(&path, 350_000, limit).unwrap();
        assert_eq!(next.pts_us, 350_000);
        let full = decode_bgra(&path, 250_000).unwrap();
        assert_eq!((full.width, full.height), (128, 64));
        release_decoders();
    }

    #[test]
    fn export_muxes_video_and_audio() {
        if !ffmpeg_or_skip() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("export.mp4");
        let mut session = FfmpegExport::begin(&path, 64, 64, 30, 48_000, 2).unwrap();
        let frame = VideoFrame::solid(64, 64, 10, 200, 30, 0).unwrap();
        let tone: Vec<i16> = (0..1_600 * 2).map(|i| ((i % 100) * 100) as i16).collect();
        for _ in 0..30 {
            session.write_video(&frame).unwrap();
            session.write_audio(&tone).unwrap();
        }
        session.finish().unwrap();
        assert!(path.is_file());
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name != "export.mp4")
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp files left behind: {leftovers:?}"
        );
        let duration = duration_us(&path).unwrap();
        assert!(
            duration.abs_diff(1_000_000) <= 80_000,
            "duration {duration}"
        );
        let output = probe(&path, &["-show_entries", "stream=codec_name"]).unwrap();
        assert_eq!(output.streams.len(), 2);
    }

    #[test]
    fn dropped_session_cleans_up() {
        if !ffmpeg_or_skip() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("aborted.mp4");
        let mut session = FfmpegExport::begin(&path, 32, 32, 30, 48_000, 2).unwrap();
        session
            .write_video(&VideoFrame::solid(32, 32, 0, 0, 0, 0).unwrap())
            .unwrap();
        drop(session);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
