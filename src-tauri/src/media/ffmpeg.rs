//! FFmpeg subprocess backend for probing, decoding and encoding. Used on Windows and Linux,
//! and on macOS when `AEROEDITS_MEDIA_BACKEND=ffmpeg`. Frames cross a pipe as raw BGRA.
use super::{validate_dim, ColorInfo, PixelFormat, VideoFrame, MAX_FRAME_DIM};
use parking_lot::Mutex;
use serde::Deserialize;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::OnceLock;

/// Overrides the ffmpeg binary location.
pub const FFMPEG_ENV: &str = "AEROEDITS_FFMPEG";
/// Overrides the ffprobe binary location.
pub const FFPROBE_ENV: &str = "AEROEDITS_FFPROBE";
/// Decoded streams kept open between calls so sequential reads stay on one process.
const MAX_OPEN_STREAMS: usize = 4;
/// A request this far past the stream position is read forward instead of re-seeking.
const MAX_FORWARD_READ_US: u64 = 2_000_000;
/// Decode rate used when the source does not report a usable frame rate.
const FALLBACK_RATE: (u32, u32) = (30, 1);
const MAX_DECODE_RATE: u32 = 120;
/// How far before the end to seek when a request lands past the last frame.
const TAIL_SEEK_US: u64 = 1_000_000;
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
}

impl DecodeLimit {
    pub const NONE: Self = Self {
        max_width: 0,
        max_height: 0,
        max_rate: 0,
    };

    /// Output size and rate for a source, keeping its aspect ratio.
    fn apply(&self, info: &VideoInfo) -> VideoInfo {
        let mut scale = 1.0f64;
        if self.max_width > 0 && info.width > self.max_width {
            scale = scale.min(self.max_width as f64 / info.width as f64);
        }
        if self.max_height > 0 && info.height > self.max_height {
            scale = scale.min(self.max_height as f64 / info.height as f64);
        }
        let fit = |value: u32| ((value as f64 * scale).round() as u32).max(2);
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

/// One ffmpeg process emitting constant-rate BGRA frames from `start_us` onward.
struct FrameStream {
    path: PathBuf,
    limit: DecodeLimit,
    child: Child,
    stdout: ChildStdout,
    log: File,
    width: u32,
    height: u32,
    rate: (u32, u32),
    start_us: u64,
    /// Index of the next frame the pipe will deliver.
    next_index: u64,
    /// The frame at `next_index - 1`, if any has been read.
    last: Option<Vec<u8>>,
    eof: bool,
}

impl FrameStream {
    fn open(
        path: &Path,
        source: &VideoInfo,
        limit: DecodeLimit,
        start_us: u64,
    ) -> Result<Self, String> {
        let info = limit.apply(source);
        let (mut cmd, log) = command(ffmpeg_path()?)?;
        let (num, den) = info.rate;
        let (width, height) = (info.width, info.height);
        cmd.arg("-nostdin")
            .args(["-ss", &seconds_arg(start_us)])
            .arg("-i")
            .arg(file_arg(path))
            .args(["-map", "0:v:0", "-an", "-sn"])
            .arg("-vf")
            .arg(format!(
                "fps={num}/{den},scale={width}:{height}:flags=bilinear:in_color_matrix=auto:in_range=auto:out_range=full,format=bgra"
            ))
            .args(["-f", "rawvideo", "pipe:1"])
            .stdout(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Could not start the FFmpeg decoder: {e}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or("FFmpeg decoder has no output pipe")?;
        Ok(Self {
            path: path.to_path_buf(),
            limit,
            child,
            stdout,
            log,
            width: info.width,
            height: info.height,
            rate: info.rate,
            start_us,
            next_index: 0,
            last: None,
            eof: false,
        })
    }

    fn frame_len(&self) -> usize {
        self.width as usize * self.height as usize * 4
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

    fn read_next(&mut self) -> Result<bool, String> {
        if self.eof {
            return Ok(false);
        }
        let mut buffer = self.last.take().unwrap_or_default();
        buffer.resize(self.frame_len(), 0);
        let mut filled = 0;
        while filled < buffer.len() {
            match self.stdout.read(&mut buffer[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(format!("FFmpeg decoder pipe failed: {e}")),
            }
        }
        if filled < buffer.len() {
            self.eof = true;
            let status = self.child.wait().ok();
            if filled == 0 && status.is_some_and(|s| s.success()) {
                // A clean end of stream: keep showing the last frame.
                self.last = if self.next_index > 0 {
                    Some(buffer)
                } else {
                    None
                };
                return Ok(false);
            }
            return Err(failure("FFmpeg decoder stopped early", &mut self.log));
        }
        self.last = Some(buffer);
        self.next_index += 1;
        Ok(true)
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
        Ok(VideoFrame {
            pts_us: self.time_of(self.next_index.saturating_sub(1)),
            width: self.width,
            height: self.height,
            stride: self.width * 4,
            format: PixelFormat::Bgra8888,
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
}

static DECODERS: Mutex<DecoderCache> = Mutex::new(DecoderCache {
    info: Vec::new(),
    streams: Vec::new(),
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
    let taken = {
        let mut cache = DECODERS.lock();
        cache
            .streams
            .iter()
            .position(|s| s.path == path && s.limit == limit && s.can_serve(time_us))
            .map(|index| cache.streams.remove(index))
    };
    let (mut stream, mut frame) = match taken {
        Some(mut stream) => {
            let frame = stream.frame_at(time_us);
            (stream, frame)
        }
        None => {
            let started = std::time::Instant::now();
            let info = cached_info(path)?;
            let mut stream = FrameStream::open(path, &info, limit, time_us)?;
            let frame = stream.frame_at(time_us);
            super::profile("decoder seek (new FFmpeg process)", started);
            (stream, frame)
        }
    };
    if frame.is_err() && stream.next_index == 0 && stream.eof {
        // Seeking past the last frame yields nothing; hold the final frame instead.
        let info = cached_info(path)?;
        let tail_start = duration_us(path)?.saturating_sub(TAIL_SEEK_US);
        stream = FrameStream::open(path, &info, limit, tail_start.min(time_us))?;
        frame = stream.frame_at(time_us);
    }
    if frame.is_ok() {
        let mut cache = DECODERS.lock();
        cache.streams.insert(0, stream);
        cache.streams.truncate(MAX_OPEN_STREAMS);
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

#[derive(Clone, Debug)]
struct H264Encoder {
    name: &'static str,
    /// Constant-quality settings; `None` falls back to a bitrate target.
    quality: Option<&'static [&'static str]>,
    /// GPU encoders are listed by any build that supports them, so they are only used after a
    /// one-frame test encode succeeds on this machine.
    hardware: bool,
}

/// In order of preference: GPU encoders first, then libx264, then the remaining fallbacks.
const H264_ENCODERS: &[H264Encoder] = &[
    H264Encoder {
        name: "h264_nvenc",
        quality: Some(&[
            "-preset", "p5", "-tune", "hq", "-rc", "vbr", "-cq", "19", "-b:v", "0",
        ]),
        hardware: true,
    },
    H264Encoder {
        name: "h264_amf",
        quality: Some(&[
            "-quality", "quality", "-rc", "cqp", "-qp_i", "18", "-qp_p", "20",
        ]),
        hardware: true,
    },
    H264Encoder {
        name: "h264_qsv",
        quality: Some(&["-preset", "medium", "-global_quality", "20"]),
        hardware: true,
    },
    H264Encoder {
        name: "libx264",
        quality: Some(&["-preset", "veryfast", "-crf", "18"]),
        hardware: false,
    },
    H264Encoder {
        name: "h264_mf",
        quality: None,
        hardware: false,
    },
    H264Encoder {
        name: "h264_videotoolbox",
        quality: None,
        hardware: false,
    },
    H264Encoder {
        name: "libopenh264",
        quality: None,
        hardware: false,
    },
];

/// Encodes one small frame to check the encoder's device and driver are actually present.
fn encoder_works(ffmpeg: &Path, encoder: &H264Encoder) -> bool {
    let Ok((mut cmd, log)) = command(ffmpeg) else {
        return false;
    };
    cmd.args(["-f", "lavfi", "-i", "color=black:size=256x256:rate=30"])
        .args([
            "-frames:v",
            "1",
            "-vf",
            "format=yuv420p",
            "-c:v",
            encoder.name,
        ])
        .args(encoder.quality.unwrap_or_default())
        .args(["-f", "null", "-"]);
    run(cmd, log, "Test encode failed").is_ok()
}

fn h264_encoder() -> Result<&'static H264Encoder, String> {
    static ENCODER: OnceLock<Result<&'static H264Encoder, String>> = OnceLock::new();
    ENCODER
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
                    .ok_or_else(|| format!("{ENCODER_ENV}={forced} is not available"));
            }
            H264_ENCODERS
                .iter()
                .filter(|encoder| has(encoder.name))
                .find(|encoder| !encoder.hardware || encoder_works(ffmpeg, encoder))
                .ok_or_else(|| "This FFmpeg build has no H.264 encoder".to_string())
        })
        .clone()
}

pub fn encoder_name() -> Option<&'static str> {
    h264_encoder().ok().map(|encoder| encoder.name)
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
        validate_dim(width, height)?;
        if width % 2 != 0 || height % 2 != 0 {
            return Err("H.264 export requires even dimensions".into());
        }
        if width > MAX_FRAME_DIM || height > MAX_FRAME_DIM {
            return Err("Export canvas exceeds the working-set limit".into());
        }
        let fps = fps.max(1);
        let encoder = h264_encoder()?;
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
            .args(["-c:v", encoder.name]);
        if let Some(quality) = encoder.quality {
            cmd.args(quality);
        } else {
            let bitrate = (width as u64 * height as u64 * fps as u64 / 5).max(1_000_000);
            cmd.args(["-b:v", &bitrate.to_string()]);
        }
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
