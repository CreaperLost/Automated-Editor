//! The timeline, as editors like Premiere have it: **assets** (things that can be played, each
//! with one or more streams on one clock) and a **sequence** of video and audio tracks that
//! hold clips of those streams.
//!
//! A recording is an asset like any imported file: its screen and camera are picture streams,
//! its microphone and system audio are sound streams, all on the recording's own clock. A clip
//! plays one stream from `in_us` for `duration_us`, starting at `start_us` on the timeline.
//! Clips cut from one moment of an asset share a `link`, so they move and trim together until
//! they are unlinked. Picture never carries sound: a video's sound is its own clips on audio
//! tracks, linked to the picture.
//!
//! Features that work in an asset's time (zooms, cursor, transcripts, pauses, webcam focus,
//! chapters, shorts) find the timeline through the clips that play it: see [`clock`].
pub mod clock;
pub mod edit;
pub mod sources;

use serde::{Deserialize, Serialize};

pub const MAX_ASSETS: usize = 512;
pub const MAX_STREAMS: usize = 16;
/// Video tracks and audio tracks, each.
pub const MAX_TRACKS: usize = 16;
pub const MAX_CLIPS: usize = 20_000;
/// How long an image plays when first placed on the timeline.
pub const IMAGE_CLIP_US: u64 = 5_000_000;
/// How long an image can be stretched to.
pub const IMAGE_MAX_US: u64 = 3_600_000_000;
/// Pieces shorter than this are dropped when a cut leaves them (well under a frame).
pub const MIN_PIECE_US: u64 = 10_000;
const MAX_SAFE_US: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    /// An AeroEdits recording folder: screen, camera, microphone and system audio.
    Recording,
    Video,
    Image,
    Audio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamKind {
    Picture,
    Sound,
}

/// What a stream stands for, so it is drawn or heard like that part of a recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Picture drawn with the canvas layout (size, corners, shadow, crop), zooms and cursor.
    Screen,
    /// Picture drawn in the camera bubble (or full frame under webcam focus).
    Webcam,
    /// Picture drawn over the whole canvas (contain or cover).
    Overlay,
    /// Sound that is speech: it can be transcribed and captioned, and ducks background sound.
    Mic,
    /// Music, game or desktop sound.
    Background,
}

impl Role {
    pub fn kind(self) -> StreamKind {
        match self {
            Role::Screen | Role::Webcam | Role::Overlay => StreamKind::Picture,
            Role::Mic | Role::Background => StreamKind::Sound,
        }
    }
}

/// A half-open range of an asset's own time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRange {
    pub start_us: u64,
    pub end_us: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    /// A recording's own track id (`screen`, `webcam`, `mic`, `system`); for a file,
    /// `picture` or `sound0`, `sound1`, ...
    pub id: String,
    pub kind: StreamKind,
    pub role: Role,
    pub name: String,
    /// A file's sound, extracted once to a 48 kHz WAV, relative to the project root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_path: Option<String>,
    /// A recording's frame rate for this stream, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    /// The file or recording's name, for display.
    pub name: String,
    pub kind: AssetKind,
    /// Where it is, used in place (absolute): a recording's folder or the file.
    pub path: String,
    pub streams: Vec<Stream>,
    /// The asset's length; for an image, how far a clip can be stretched.
    pub duration_us: u64,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    /// A recording's pauses: time on its clock that holds no media.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pauses: Vec<SourceRange>,
    /// The file or folder is no longer there. Worked out when the project is read.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub missing: bool,
}

impl Asset {
    pub fn stream(&self, id: &str) -> Option<&Stream> {
        self.streams.iter().find(|s| s.id == id)
    }

    pub fn is_still(&self) -> bool {
        self.kind == AssetKind::Image
    }

    pub fn is_recording(&self) -> bool {
        self.kind == AssetKind::Recording
    }

    /// The parts of the asset's clock that hold media, in order: a recording between its
    /// pauses, anything else whole.
    pub fn spans(&self) -> Vec<SourceRange> {
        let mut spans = Vec::new();
        let mut cursor = 0;
        let mut pauses = self.pauses.clone();
        pauses.sort_by_key(|p| p.start_us);
        for pause in pauses {
            if pause.start_us > cursor {
                spans.push(SourceRange {
                    start_us: cursor,
                    end_us: pause.start_us.min(self.duration_us),
                });
            }
            cursor = cursor.max(pause.end_us);
        }
        if cursor < self.duration_us {
            spans.push(SourceRange {
                start_us: cursor,
                end_us: self.duration_us,
            });
        }
        spans.retain(|s| s.end_us > s.start_us);
        spans
    }

    /// The span holding source time `us` (for a clip starting there, the span it may use).
    pub fn span_at(&self, us: u64) -> Option<SourceRange> {
        if self.is_still() {
            return Some(SourceRange {
                start_us: 0,
                end_us: self.duration_us,
            });
        }
        self.spans()
            .into_iter()
            .find(|s| s.start_us <= us && us < s.end_us)
    }

    /// Whether `[in_us, in_us + duration_us)` is media the asset holds, without a pause.
    pub fn holds(&self, in_us: u64, duration_us: u64) -> bool {
        if self.is_still() {
            return in_us == 0 && duration_us <= self.duration_us;
        }
        self.span_at(in_us)
            .is_some_and(|span| in_us + duration_us <= span.end_us)
    }

    /// The length a clip of this asset starts with on the timeline.
    pub fn default_clip_us(&self) -> u64 {
        if self.is_still() {
            IMAGE_CLIP_US
        } else {
            self.duration_us
        }
    }

    /// The first sound stream that is speech, else the first sound stream.
    pub fn speech_stream(&self) -> Option<&Stream> {
        self.streams
            .iter()
            .find(|s| s.role == Role::Mic)
            .or_else(|| self.streams.iter().find(|s| s.kind == StreamKind::Sound))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    #[default]
    Video,
    Audio,
}

impl TrackKind {
    pub fn holds(self, stream: StreamKind) -> bool {
        matches!(
            (self, stream),
            (TrackKind::Video, StreamKind::Picture) | (TrackKind::Audio, StreamKind::Sound)
        )
    }
}

/// How an overlay picture fills the canvas.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Fit {
    /// The whole picture, centred; what is below shows around it.
    #[default]
    Contain,
    /// The whole canvas, cropping the picture's edges.
    Cover,
}

impl Fit {
    pub fn is_default(&self) -> bool {
        *self == Fit::Contain
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: String,
    pub asset: String,
    pub stream: String,
    /// Where the clip starts on the timeline.
    pub start_us: u64,
    /// Where in the asset's time it starts.
    pub in_us: u64,
    pub duration_us: u64,
    /// Clips cut from one moment of an asset share this, so they move and trim together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(default, skip_serializing_if = "Fit::is_default")]
    pub fit: Fit,
}

impl Clip {
    pub fn end_us(&self) -> u64 {
        self.start_us + self.duration_us
    }

    pub fn out_us(&self) -> u64 {
        self.in_us + self.duration_us
    }

    /// The asset time playing at timeline time `us`, if the clip covers it.
    pub fn local_us(&self, us: u64) -> Option<u64> {
        (self.start_us <= us && us < self.end_us()).then(|| self.in_us + (us - self.start_us))
    }

    pub fn source(&self) -> StreamRef {
        StreamRef::new(&self.asset, &self.stream)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub kind: TrackKind,
    /// The user's name for it; empty shows its number (V1, A2, ...).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Not drawn.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// Not heard.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub muted: bool,
    /// Never changed by edits, ripples included.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    /// What the track's clips stand for; unset, each stream's own role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// In timeline order, never overlapping.
    #[serde(default)]
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new(id: String, kind: TrackKind) -> Self {
        Self {
            id,
            kind,
            name: String::new(),
            hidden: false,
            muted: false,
            locked: false,
            role: None,
            clips: Vec::new(),
        }
    }

    pub fn clip_at(&self, us: u64) -> Option<&Clip> {
        let i = self.clips.partition_point(|c| c.end_us() <= us);
        self.clips.get(i).filter(|c| c.start_us <= us)
    }

    /// Whether nothing on the track plays in `[a, b)`.
    pub fn is_free(&self, a: u64, b: u64) -> bool {
        !self.clips.iter().any(|c| c.start_us < b && a < c.end_us())
    }

    pub fn end_us(&self) -> u64 {
        self.clips.last().map_or(0, Clip::end_us)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sequence {
    /// Video tracks bottom to top, then audio tracks top to bottom.
    #[serde(default)]
    pub tracks: Vec<Track>,
    /// Ripple editing: cuts close up on every track, moves and longer clips make room.
    #[serde(default = "yes")]
    pub magnetic: bool,
}

fn yes() -> bool {
    true
}

impl Default for Sequence {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            magnetic: true,
        }
    }
}

impl Sequence {
    /// Where the last clip ends.
    pub fn duration_us(&self) -> u64 {
        self.tracks.iter().map(Track::end_us).max().unwrap_or(0)
    }

    pub fn video_tracks(&self) -> impl DoubleEndedIterator<Item = &Track> {
        self.tracks.iter().filter(|t| t.kind == TrackKind::Video)
    }

    pub fn audio_tracks(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter().filter(|t| t.kind == TrackKind::Audio)
    }

    pub fn track(&self, id: &str) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    pub fn clips(&self) -> impl Iterator<Item = (&Track, &Clip)> {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter().map(move |c| (t, c)))
    }

    pub fn clip(&self, id: &str) -> Option<(&Track, &Clip)> {
        self.clips().find(|(_, c)| c.id == id)
    }

    /// "V1", "A2": the track's number among its kind.
    pub fn number(&self, track_id: &str) -> Option<String> {
        let track = self.track(track_id)?;
        let index = self
            .tracks
            .iter()
            .filter(|t| t.kind == track.kind)
            .position(|t| t.id == track_id)?;
        Some(match track.kind {
            TrackKind::Video => format!("V{}", index + 1),
            TrackKind::Audio => format!("A{}", index + 1),
        })
    }

    /// Every clip of `asset`'s stream `stream`, in timeline order.
    pub fn clips_of<'a>(
        &'a self,
        asset: &'a str,
        stream: &'a str,
    ) -> impl Iterator<Item = (&'a Track, &'a Clip)> + 'a {
        self.clips()
            .filter(move |(_, c)| c.asset == asset && c.stream == stream)
    }

    /// The edit points: every clip's start and end, sorted and unique, with 0.
    pub fn edges(&self) -> Vec<u64> {
        let mut edges: Vec<u64> = std::iter::once(0)
            .chain(self.clips().flat_map(|(_, c)| [c.start_us, c.end_us()]))
            .collect();
        edges.sort_unstable();
        edges.dedup();
        edges
    }
}

/// One stream of one asset: what a clip plays, what a transcript or a waveform is of.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamRef {
    pub asset: String,
    pub stream: String,
}

impl StreamRef {
    pub fn new(asset: &str, stream: &str) -> Self {
        Self {
            asset: asset.into(),
            stream: stream.into(),
        }
    }

    /// `<asset>.<stream>`: the name transcripts and waveforms are kept under.
    pub fn key(&self) -> String {
        format!("{}.{}", self.asset, self.stream)
    }

    pub fn parse(key: &str) -> Option<Self> {
        let (asset, stream) = key.split_once('.')?;
        (!asset.is_empty() && !stream.is_empty()).then(|| Self::new(asset, stream))
    }
}

/// The role a clip plays as: its track's, else its stream's.
pub fn clip_role(assets: &[Asset], track: &Track, clip: &Clip) -> Option<Role> {
    let stream = assets
        .iter()
        .find(|a| a.id == clip.asset)?
        .stream(&clip.stream)?;
    Some(
        track
            .role
            .filter(|r| r.kind() == stream.kind)
            .unwrap_or(stream.role),
    )
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn valid_stream_id(id: &str) -> bool {
    crate::project::reader::is_safe_track_id(id) && id.len() <= 64
}

/// Checks assets: unique ids, streams that make sense, paths that are absolute.
pub fn validate_assets(assets: &[Asset]) -> Result<(), String> {
    if assets.len() > MAX_ASSETS {
        return Err("Too many imported files".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for asset in assets {
        if !valid_id(&asset.id) || !ids.insert(asset.id.as_str()) {
            return Err("Invalid media id".into());
        }
        if asset.path.is_empty()
            || asset.path.len() > 4096
            || asset.path.contains('\0')
            || !std::path::Path::new(&asset.path).is_absolute()
        {
            return Err(format!("{} needs its full path", asset.name));
        }
        if asset.duration_us == 0 || asset.duration_us > MAX_SAFE_US {
            return Err(format!("{} has no length", asset.name));
        }
        if asset.streams.is_empty() || asset.streams.len() > MAX_STREAMS {
            return Err(format!("{} has nothing to play", asset.name));
        }
        let mut stream_ids = std::collections::BTreeSet::new();
        for stream in &asset.streams {
            if !valid_stream_id(&stream.id) || !stream_ids.insert(stream.id.as_str()) {
                return Err(format!("{} has an invalid stream", asset.name));
            }
            if stream.role.kind() != stream.kind {
                return Err(format!("{}: a stream's role must suit it", asset.name));
            }
            if let Some(path) = &stream.audio_path {
                if !path.starts_with("assets/media/") || path.contains("..") || path.contains('\\')
                {
                    return Err("Extracted sound must live in assets/media".into());
                }
            }
        }
        if asset
            .pauses
            .iter()
            .any(|p| p.end_us <= p.start_us || p.end_us > asset.duration_us)
        {
            return Err(format!("{} has invalid pauses", asset.name));
        }
    }
    Ok(())
}

/// Checks the sequence against the assets: clips in order without overlaps, each on a track of
/// its kind and inside what its asset holds; ids unique.
pub fn validate_sequence(sequence: &Sequence, assets: &[Asset]) -> Result<(), String> {
    let videos = sequence.video_tracks().count();
    if videos > MAX_TRACKS || sequence.tracks.len() - videos > MAX_TRACKS {
        return Err(format!(
            "Use at most {MAX_TRACKS} video and {MAX_TRACKS} audio tracks"
        ));
    }
    // Video tracks come first, then audio tracks.
    if sequence
        .tracks
        .windows(2)
        .any(|w| w[0].kind == TrackKind::Audio && w[1].kind == TrackKind::Video)
    {
        return Err("Video tracks must come before audio tracks".into());
    }
    let mut track_ids = std::collections::BTreeSet::new();
    let mut clip_ids = std::collections::BTreeSet::new();
    let mut count = 0;
    for track in &sequence.tracks {
        if !valid_id(&track.id) || !track_ids.insert(track.id.as_str()) {
            return Err("Invalid track id".into());
        }
        if track.name.chars().count() > 40 || track.name.chars().any(char::is_control) {
            return Err("A track name is one line of at most 40 characters".into());
        }
        let kind = match track.kind {
            TrackKind::Video => StreamKind::Picture,
            TrackKind::Audio => StreamKind::Sound,
        };
        if track.role.is_some_and(|r| r.kind() != kind) {
            return Err("A track's role must suit the track".into());
        }
        count += track.clips.len();
        if count > MAX_CLIPS {
            return Err("Too many clips".into());
        }
        let mut end = 0;
        for clip in &track.clips {
            if !valid_id(&clip.id) || !clip_ids.insert(clip.id.as_str()) {
                return Err("Invalid clip id".into());
            }
            if clip.link.as_deref().is_some_and(|l| !valid_id(l)) {
                return Err("Invalid link id".into());
            }
            if clip.duration_us == 0 || clip.end_us() > MAX_SAFE_US {
                return Err("A clip must have a length".into());
            }
            if clip.start_us < end {
                return Err("Clips on a track must not overlap".into());
            }
            end = clip.end_us();
            let asset = assets.iter().find(|a| a.id == clip.asset).ok_or_else(|| {
                format!(
                    "The timeline uses media {} that is not imported",
                    clip.asset
                )
            })?;
            let stream = asset
                .stream(&clip.stream)
                .ok_or_else(|| format!("{} has no stream {}", asset.name, clip.stream))?;
            if stream.kind != kind {
                return Err(match kind {
                    StreamKind::Picture => "Sound goes on audio tracks".into(),
                    StreamKind::Sound => "Pictures go on video tracks".into(),
                });
            }
            if !asset.holds(clip.in_us, clip.duration_us) {
                return Err(format!("A clip of {} runs past its media", asset.name));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn stream(id: &str, kind: StreamKind, role: Role) -> Stream {
        Stream {
            id: id.into(),
            kind,
            role,
            name: id.into(),
            audio_path: None,
            fps: None,
        }
    }

    /// A recording `rec` of `duration_us` with screen, webcam, mic and system streams.
    pub fn recording(id: &str, duration_us: u64, pauses: &[(u64, u64)]) -> Asset {
        Asset {
            id: id.into(),
            name: "Recording".into(),
            kind: AssetKind::Recording,
            path: if cfg!(windows) { "C:\\rec" } else { "/rec" }.into(),
            streams: vec![
                stream("screen", StreamKind::Picture, Role::Screen),
                stream("webcam", StreamKind::Picture, Role::Webcam),
                stream("mic", StreamKind::Sound, Role::Mic),
                stream("system", StreamKind::Sound, Role::Background),
            ],
            duration_us,
            width: 1920,
            height: 1080,
            pauses: pauses
                .iter()
                .map(|&(start_us, end_us)| SourceRange { start_us, end_us })
                .collect(),
            missing: false,
        }
    }

    /// A video file with a picture and one sound stream.
    pub fn video(id: &str, duration_us: u64) -> Asset {
        Asset {
            id: id.into(),
            name: format!("{id}.mp4"),
            kind: AssetKind::Video,
            path: if cfg!(windows) { "C:\\v.mp4" } else { "/v.mp4" }.into(),
            streams: vec![
                stream("picture", StreamKind::Picture, Role::Screen),
                Stream {
                    audio_path: Some(format!("assets/media/{id}.audio.wav")),
                    ..stream("sound0", StreamKind::Sound, Role::Mic)
                },
            ],
            duration_us,
            width: 1280,
            height: 720,
            pauses: Vec::new(),
            missing: false,
        }
    }

    pub fn image(id: &str) -> Asset {
        Asset {
            id: id.into(),
            name: format!("{id}.png"),
            kind: AssetKind::Image,
            path: if cfg!(windows) { "C:\\i.png" } else { "/i.png" }.into(),
            streams: vec![stream("picture", StreamKind::Picture, Role::Overlay)],
            duration_us: IMAGE_MAX_US,
            width: 10,
            height: 10,
            pauses: Vec::new(),
            missing: false,
        }
    }

    #[test]
    fn spans_skip_pauses_and_clips_stay_inside_them() {
        let rec = recording("rec", 10_000_000, &[(4_000_000, 5_000_000)]);
        assert_eq!(
            rec.spans(),
            vec![
                SourceRange {
                    start_us: 0,
                    end_us: 4_000_000
                },
                SourceRange {
                    start_us: 5_000_000,
                    end_us: 10_000_000
                }
            ]
        );
        assert!(rec.holds(0, 4_000_000));
        assert!(!rec.holds(3_000_000, 2_000_000), "across the pause");
        assert!(!rec.holds(4_500_000, 100), "inside the pause");
        assert!(rec.holds(5_000_000, 5_000_000));
        let still = image("img");
        assert!(still.holds(0, 60_000_000));
        assert!(!still.holds(1, 10));
    }

    #[test]
    fn stream_keys_round_trip() {
        let r = StreamRef::new("m-abc", "sound1");
        assert_eq!(r.key(), "m-abc.sound1");
        assert_eq!(StreamRef::parse("m-abc.sound1"), Some(r));
        assert_eq!(StreamRef::parse("nodot"), None);
    }

    #[test]
    fn validation_catches_overlaps_wrong_tracks_and_unknown_media() {
        let assets = vec![video("m1", 10_000_000)];
        let clip = |id: &str, stream: &str, start: u64, len: u64| Clip {
            id: id.into(),
            asset: "m1".into(),
            stream: stream.into(),
            start_us: start,
            in_us: 0,
            duration_us: len,
            link: None,
            fit: Fit::Contain,
        };
        let mut v1 = Track::new("t1".into(), TrackKind::Video);
        v1.clips = vec![
            clip("c1", "picture", 0, 1_000_000),
            clip("c2", "picture", 1_000_000, 1_000_000),
        ];
        let mut seq = Sequence {
            tracks: vec![v1],
            magnetic: true,
        };
        validate_sequence(&seq, &assets).unwrap();
        seq.tracks[0].clips[1].start_us = 500_000;
        assert!(validate_sequence(&seq, &assets).is_err(), "overlap");
        seq.tracks[0].clips[1].start_us = 1_000_000;
        seq.tracks[0].clips[1].stream = "sound0".into();
        assert!(
            validate_sequence(&seq, &assets).is_err(),
            "sound on a video track"
        );
        seq.tracks[0].clips[1].stream = "picture".into();
        seq.tracks[0].clips[1].duration_us = 20_000_000;
        assert!(validate_sequence(&seq, &assets).is_err(), "past the end");
        seq.tracks[0].clips[1].duration_us = 1_000_000;
        seq.tracks[0].clips[1].asset = "gone".into();
        assert!(validate_sequence(&seq, &assets).is_err(), "unknown media");
    }
}
