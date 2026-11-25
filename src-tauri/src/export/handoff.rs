//! The edit for another editor (DLV-3): its cut as timeline files that Premiere, DaVinci
//! Resolve and Final Cut Pro open, every clip on the track and at the time it has here, read
//! from its own source file, with each graphic as its see-through MOV from
//! [`super::graphics_out`]. FCPXML is for Final Cut and Resolve, FCP7 XML for Premiere and
//! Resolve. What the app draws on top of the cut (layouts, zooms, captions, sound cleanup)
//! is not carried over; the notes say so.
use super::graphics_out::Written;
use crate::project::revision::EditDocument;
use crate::sequence::{AssetKind, StreamKind, TrackKind};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// One file the timeline reads.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub path: PathBuf,
    pub duration_us: u64,
    /// Width and height, when it has a picture.
    pub size: Option<(u32, u32)>,
    /// Frames per second as a fraction, when it moves.
    pub rate: Option<(u32, u32)>,
    pub sound: bool,
    pub still: bool,
}

/// A stretch of a source on a track.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub source: usize,
    pub name: String,
    /// Where it starts on the timeline, and from where in the file, in frames.
    pub start: u64,
    pub from: u64,
    pub frames: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lane {
    pub name: String,
    pub enabled: bool,
    pub pieces: Vec<Piece>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Handoff {
    pub name: String,
    pub canvas: (u32, u32),
    pub fps: u32,
    pub frames: u64,
    pub sources: Vec<Source>,
    /// Bottom to top.
    pub video: Vec<Lane>,
    pub audio: Vec<Lane>,
    /// What did not carry over, one line each.
    pub notes: Vec<String>,
}

/// What the app adds on top of the cut that no timeline file carries.
const NOT_CARRIED: &str = "Not carried over: layouts and the camera bubble, zooms and the \
    cursor, crops, captions (export the SRT for those), sound cleanup, levels, fades and music \
    ducking. Each picture sits full frame on its own track.";

fn frame(us: u64, fps: u32) -> u64 {
    ((us as u128 * fps as u128 + 500_000) / 1_000_000) as u64
}

fn clock(us: u64) -> String {
    let s = us / 1_000_000;
    format!("{}:{:02}", s / 60, s % 60)
}

/// The timeline of `document` on a `canvas` at `fps`, with `graphics` as written for it.
pub fn build(
    root: &Path,
    document: &EditDocument,
    name: &str,
    canvas: (u32, u32),
    fps: u32,
    graphics: &[Written],
) -> Result<Handoff, String> {
    let mut sources: Vec<Source> = Vec::new();
    let mut by_path: HashMap<PathBuf, usize> = HashMap::new();
    let mut source_for = |path: PathBuf, still: bool| -> usize {
        *by_path.entry(path.clone()).or_insert_with(|| {
            sources.push(Source {
                path,
                duration_us: 0,
                size: None,
                rate: None,
                sound: false,
                still,
            });
            sources.len() - 1
        })
    };
    let mut retimed = Vec::new();
    let mut missing = Vec::new();
    let (mut video, mut audio) = (Vec::new(), Vec::new());
    let sequence = &document.sequence;
    for track in &sequence.tracks {
        let label = sequence.number(&track.id).unwrap_or_default();
        let mut lane = Lane {
            name: if track.name.is_empty() {
                label
            } else {
                track.name.clone()
            },
            enabled: match track.kind {
                TrackKind::Video => !track.hidden,
                TrackKind::Audio => !track.muted,
            },
            pieces: Vec::new(),
        };
        for clip in &track.clips {
            let Some(asset) = document.asset(&clip.asset) else {
                continue;
            };
            if (clip.speed - 1.0).abs() > 1e-6 {
                retimed.push(format!("{} at {}", asset.name, clock(clip.start_us)));
            }
            // Its file's own time for each stretch of it: (file, from, timeline start, length).
            let mut stretches: Vec<(PathBuf, u64, u64, u64)> = Vec::new();
            let mut still = false;
            match asset.kind {
                AssetKind::Recording => {
                    let mount = format!("{}/", crate::project::folder::mount_path(&asset.id));
                    let Ok(files) = crate::sequence::sources::recording(root, asset) else {
                        missing.push(asset.path.clone());
                        continue;
                    };
                    let segments = files
                        .track(&clip.stream)
                        .map(|(_, s)| s.as_slice())
                        .unwrap_or_default();
                    let (a, b) = (clip.in_us, clip.in_us + clip.duration_us);
                    for segment in segments {
                        let (s, e) = (a.max(segment.start_us), b.min(segment.end_us));
                        if s >= e {
                            continue;
                        }
                        let relative = segment
                            .relative_path
                            .strip_prefix(&mount)
                            .unwrap_or(&segment.relative_path);
                        let path = Path::new(&asset.path).join(relative);
                        if !segment.available || !path.is_file() {
                            missing.push(path.display().to_string());
                            continue;
                        }
                        stretches.push((
                            path,
                            s - segment.start_us,
                            clip.start_us + (s - a),
                            e - s,
                        ));
                    }
                }
                AssetKind::Graphic => {
                    // Graphics on hidden tracks are not rendered and stay out.
                    if let Some(made) = graphics.iter().find(|w| w.clip == clip.id) {
                        stretches.push((made.path.clone(), 0, clip.start_us, clip.duration_us));
                    }
                }
                AssetKind::Video | AssetKind::Audio | AssetKind::Image => {
                    still = asset.kind == AssetKind::Image;
                    let path = PathBuf::from(&asset.path);
                    if path.is_file() {
                        let from = if still { 0 } else { clip.in_us };
                        stretches.push((path, from, clip.start_us, clip.duration_us));
                    } else {
                        missing.push(asset.path.clone());
                    }
                }
            }
            let sound = asset
                .stream(&clip.stream)
                .is_some_and(|s| s.kind == StreamKind::Sound);
            for (path, from, start, length) in stretches {
                let start_frame = frame(start, fps);
                let frames = frame(start + length, fps).saturating_sub(start_frame);
                if frames == 0 {
                    continue;
                }
                let source = source_for(path, still);
                lane.pieces.push(Piece {
                    source,
                    name: if sound && asset.kind == AssetKind::Recording {
                        format!("{} {}", asset.name, clip.stream)
                    } else {
                        asset.name.clone()
                    },
                    start: start_frame,
                    from: frame(from, fps),
                    frames,
                });
            }
        }
        lane.pieces.sort_by_key(|p| p.start);
        match track.kind {
            TrackKind::Video => video.push(lane),
            TrackKind::Audio => audio.push(lane),
        }
    }
    for source in &mut sources {
        probe(source);
    }
    // A file that would not say how long it is runs at least as far as it is used.
    let pieces = video.iter().chain(&audio).flat_map(|l| &l.pieces);
    for piece in pieces.clone() {
        let needed = (piece.from + piece.frames) * 1_000_000 / fps as u64;
        let source = &mut sources[piece.source];
        source.duration_us = source.duration_us.max(needed);
    }
    let frames = pieces.map(|p| p.start + p.frames).max().unwrap_or(0).max(1);
    let mut notes = vec![NOT_CARRIED.to_string()];
    if !retimed.is_empty() {
        notes.push(format!(
            "These play at normal speed here, but faster or slower in the edit: {}.",
            retimed.join(", ")
        ));
    }
    if !missing.is_empty() {
        missing.dedup();
        notes.push(format!(
            "Left out because the file is missing: {}.",
            missing.join(", ")
        ));
    }
    Ok(Handoff {
        name: name.to_string(),
        canvas,
        fps,
        frames,
        sources,
        video,
        audio,
        notes,
    })
}

fn probe(source: &mut Source) {
    use crate::media::ffmpeg;
    if let Ok(info) = ffmpeg::probe_video(&source.path) {
        source.size = Some((info.width, info.height));
        if !source.still {
            source.rate = Some(info.rate);
        }
    }
    if !source.still {
        source.duration_us = ffmpeg::duration_us(&source.path).unwrap_or(0);
        source.sound = ffmpeg::audio_stream_names(&source.path).is_ok_and(|n| !n.is_empty());
    }
}

fn esc(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\t' | '\n'))
        .fold(String::new(), |mut out, c| {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                c => out.push(c),
            }
            out
        })
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `file:///…`, the form both formats take for an absolute path.
fn file_url(path: &Path) -> String {
    url::Url::from_file_path(path)
        .map(String::from)
        .unwrap_or_else(|_| format!("file://{}", path.display()))
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

impl Handoff {
    /// `frames` at the timeline's rate as FCPXML writes a time.
    fn time(&self, frames: u64) -> String {
        if frames == 0 {
            return "0s".into();
        }
        let d = gcd(frames, self.fps as u64);
        let (n, den) = (frames / d, self.fps as u64 / d);
        if den == 1 {
            format!("{n}s")
        } else {
            format!("{n}/{den}s")
        }
    }

    fn source_frames(&self, index: usize) -> u64 {
        let source = &self.sources[index];
        if source.still {
            return 0;
        }
        frame(source.duration_us, self.fps).max(1)
    }

    /// Final Cut Pro XML 1.9, which Final Cut Pro 10.5 and later and DaVinci Resolve open.
    /// The storyline is one gap as long as the edit; every track is a lane on it, pictures
    /// above (bottom track first) and sound below.
    pub fn fcpxml(&self) -> String {
        let mut resources = String::new();
        let mut formats: HashMap<(Option<(u32, u32)>, Option<(u32, u32)>), String> = HashMap::new();
        let mut next = 1;
        let mut format = |resources: &mut String, rate: Option<(u32, u32)>, size| {
            formats
                .entry((rate, size))
                .or_insert_with(|| {
                    let id = format!("r{next}");
                    next += 1;
                    let _ = write!(resources, "        <format id=\"{id}\"");
                    match rate {
                        Some((num, den)) => {
                            let _ = write!(resources, " frameDuration=\"{den}/{num}s\"");
                        }
                        None => resources.push_str(" name=\"FFVideoFormatRateUndefined\""),
                    }
                    if let Some((w, h)) = size {
                        let _ = write!(resources, " width=\"{w}\" height=\"{h}\"");
                    }
                    resources.push_str("/>\n");
                    id
                })
                .clone()
        };
        let sequence_format = format(
            &mut resources,
            Some((self.fps, 1)),
            Some((self.canvas.0, self.canvas.1)),
        );
        let mut assets = String::new();
        for (index, source) in self.sources.iter().enumerate() {
            let id = format!("a{}", index + 1);
            let mut attrs = format!(
                "id=\"{id}\" name=\"{}\" start=\"0s\" duration=\"{}\"",
                esc(&file_name(&source.path)),
                self.time(self.source_frames(index))
            );
            if source.size.is_some() {
                let rate = if source.still {
                    None
                } else {
                    Some(source.rate.unwrap_or((self.fps, 1)))
                };
                let f = format(&mut resources, rate, source.size);
                let _ = write!(attrs, " hasVideo=\"1\" format=\"{f}\"");
            }
            if source.sound {
                attrs.push_str(
                    " hasAudio=\"1\" audioSources=\"1\" audioChannels=\"2\" audioRate=\"48000\"",
                );
            }
            let _ = writeln!(
                assets,
                "        <asset {attrs}>\n            <media-rep kind=\"original-media\" src=\"{}\"/>\n        </asset>",
                esc(&file_url(&source.path))
            );
        }
        let mut clips: Vec<(u64, String)> = Vec::new();
        let lanes = self
            .video
            .iter()
            .enumerate()
            .map(|(i, l)| (i as i64 + 1, l, false))
            .chain(
                self.audio
                    .iter()
                    .enumerate()
                    .map(|(i, l)| (-(i as i64) - 1, l, true)),
            );
        for (lane, track, sound) in lanes {
            for piece in &track.pieces {
                let source = &self.sources[piece.source];
                let mut attrs = format!(
                    "ref=\"a{}\" lane=\"{lane}\" offset=\"{}\" name=\"{}\" duration=\"{}\"",
                    piece.source + 1,
                    self.time(piece.start),
                    esc(&piece.name),
                    self.time(piece.frames)
                );
                if !source.still {
                    let _ = write!(attrs, " start=\"{}\"", self.time(piece.from));
                }
                // Each track keeps to its own kind: sound comes from the sound tracks.
                if source.size.is_some() && source.sound {
                    let _ = write!(
                        attrs,
                        " srcEnable=\"{}\"",
                        if sound { "audio" } else { "video" }
                    );
                }
                if !track.enabled {
                    attrs.push_str(" enabled=\"0\"");
                }
                let tag = if source.still { "video" } else { "asset-clip" };
                clips.push((
                    piece.start,
                    format!("                        <{tag} {attrs}/>"),
                ));
            }
        }
        clips.sort_by_key(|(start, _)| *start);
        let total = self.time(self.frames);
        let mut out = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE fcpxml>\n<fcpxml version=\"1.9\">\n    <resources>\n",
        );
        out.push_str(&resources);
        out.push_str(&assets);
        let name = esc(&self.name);
        let _ = write!(
            out,
            "    </resources>\n    <library>\n        <event name=\"{name}\">\n            <project name=\"{name}\">\n                <sequence format=\"{sequence_format}\" duration=\"{total}\" tcStart=\"0s\" tcFormat=\"NDF\" audioLayout=\"stereo\" audioRate=\"48k\">\n                    <spine>\n                        <gap name=\"Gap\" offset=\"0s\" start=\"0s\" duration=\"{total}\">\n"
        );
        for (_, clip) in clips {
            out.push_str("    ");
            out.push_str(&clip);
            out.push('\n');
        }
        out.push_str("                        </gap>\n                    </spine>\n                </sequence>\n            </project>\n        </event>\n    </library>\n</fcpxml>\n");
        out
    }

    /// Final Cut Pro 7 XML (xmeml 4), which Premiere Pro and DaVinci Resolve open.
    pub fn xmeml(&self) -> String {
        let rate = format!(
            "<rate><timebase>{}</timebase><ntsc>FALSE</ntsc></rate>",
            self.fps
        );
        let sound_format =
            "<samplecharacteristics><depth>16</depth><samplerate>48000</samplerate></samplecharacteristics>";
        let mut defined = vec![false; self.sources.len()];
        let mut item = 0;
        let file = |out: &mut String, index: usize, defined: &mut Vec<bool>| {
            if defined[index] {
                let _ = write!(out, "<file id=\"file-{}\"/>", index + 1);
                return;
            }
            defined[index] = true;
            let source = &self.sources[index];
            let _ = write!(
                out,
                "<file id=\"file-{}\"><name>{}</name><pathurl>{}</pathurl>{rate}<duration>{}</duration><media>",
                index + 1,
                esc(&file_name(&source.path)),
                esc(&file_url(&source.path).replacen("file:///", "file://localhost/", 1)),
                self.file_frames(index)
            );
            if let Some((w, h)) = source.size {
                let _ = write!(
                    out,
                    "<video><samplecharacteristics>{rate}<width>{w}</width><height>{h}</height><pixelaspectratio>square</pixelaspectratio></samplecharacteristics></video>"
                );
            }
            if source.sound {
                let _ = write!(
                    out,
                    "<audio>{sound_format}<channelcount>2</channelcount></audio>"
                );
            }
            out.push_str("</media></file>");
        };
        let mut tracks = |out: &mut String, lanes: &[Lane], sound: bool| {
            for lane in lanes {
                out.push_str("<track>");
                for piece in &lane.pieces {
                    item += 1;
                    let source = &self.sources[piece.source];
                    let from = if source.still { 0 } else { piece.from };
                    let _ = write!(
                        out,
                        "<clipitem id=\"clipitem-{item}\"><name>{}</name><enabled>{}</enabled><duration>{}</duration>{rate}<start>{}</start><end>{}</end><in>{from}</in><out>{}</out>",
                        esc(&piece.name),
                        if lane.enabled { "TRUE" } else { "FALSE" },
                        self.file_frames(piece.source),
                        piece.start,
                        piece.start + piece.frames,
                        from + piece.frames
                    );
                    file(out, piece.source, &mut defined);
                    if sound {
                        out.push_str("<sourcetrack><mediatype>audio</mediatype><trackindex>1</trackindex></sourcetrack>");
                    }
                    out.push_str("</clipitem>");
                }
                let _ = write!(
                    out,
                    "<enabled>{}</enabled><locked>FALSE</locked></track>",
                    if lane.enabled { "TRUE" } else { "FALSE" }
                );
            }
        };
        let (w, h) = self.canvas;
        let mut out = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE xmeml>\n<xmeml version=\"4\"><sequence id=\"sequence-1\"><name>{}</name><duration>{}</duration>{rate}<media><video><format><samplecharacteristics>{rate}<width>{w}</width><height>{h}</height><pixelaspectratio>square</pixelaspectratio></samplecharacteristics></format>",
            esc(&self.name),
            self.frames
        );
        tracks(&mut out, &self.video, false);
        let _ = write!(
            out,
            "</video><audio><numOutputChannels>2</numOutputChannels><format>{sound_format}</format>"
        );
        tracks(&mut out, &self.audio, true);
        let _ = writeln!(
            out,
            "</audio></media><timecode>{rate}<string>00:00:00:00</string><frame>0</frame><displayformat>NDF</displayformat></timecode></sequence></xmeml>"
        );
        out
    }

    /// A file's length in frames for FCP7 XML; a still runs as long as its longest use.
    fn file_frames(&self, index: usize) -> u64 {
        if self.sources[index].still {
            return self
                .video
                .iter()
                .flat_map(|l| &l.pieces)
                .filter(|p| p.source == index)
                .map(|p| p.frames)
                .max()
                .unwrap_or(1);
        }
        self.source_frames(index)
    }

    /// What the folder's read-me says.
    pub fn read_me(&self) -> String {
        let name = super::graphics_out::file_safe(&self.name);
        let mut text = format!(
            "{name}.fcpxml opens in Final Cut Pro (File > Import > XML) and DaVinci Resolve \
             (File > Import > Timeline).\n{name}.xml opens in Premiere Pro (File > Import) and \
             DaVinci Resolve.\n\nCarried over: every cut and where each clip sits, on the same \
             tracks, at {w}x{h} and {fps} fps. Hidden and muted tracks come in turned off. The \
             graphics are the see-through MOVs in the Graphics folder.\n",
            w = self.canvas.0,
            h = self.canvas.1,
            fps = self.fps
        );
        for note in &self.notes {
            text.push('\n');
            text.push_str(note);
            text.push('\n');
        }
        text
    }
}

/// Writes `handoff` into `folder` as `<name>.fcpxml`, `<name>.xml` and `Read me.txt`.
pub fn write(handoff: &Handoff, folder: &Path) -> Result<(), String> {
    let stem = super::graphics_out::file_safe(&handoff.name);
    for (file, text) in [
        (format!("{stem}.fcpxml"), handoff.fcpxml()),
        (format!("{stem}.xml"), handoff.xmeml()),
        ("Read me.txt".to_string(), handoff.read_me()),
    ] {
        crate::files::write_atomic(&folder.join(&file), text.as_bytes())
            .map_err(|e| format!("Could not write {file}: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::{Asset, Clip, Role, Stream, Track};

    fn well_formed(xml: &str) {
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut depth = 0i32;
        loop {
            match reader.read_event().expect("well-formed XML") {
                quick_xml::events::Event::Start(_) => depth += 1,
                quick_xml::events::Event::End(_) => depth -= 1,
                quick_xml::events::Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(depth, 0);
    }

    fn clip(id: &str, asset: &str, stream: &str, start_s: f64, in_s: f64, length_s: f64) -> Clip {
        Clip {
            mix: Default::default(),
            id: id.into(),
            asset: asset.into(),
            stream: stream.into(),
            start_us: (start_s * 1e6) as u64,
            in_us: (in_s * 1e6) as u64,
            duration_us: (length_s * 1e6) as u64,
            link: None,
            fit: crate::sequence::Fit::Contain,
            speed: 1.0,
        }
    }

    /// A video file cut in on V1 with its sound on A1 (played faster there), and a graphic on
    /// V2: both files carry every place and in point, each track keeps to its own kind, and the
    /// speed change is named.
    #[test]
    fn the_cut_is_written_for_final_cut_premiere_and_resolve() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let footage = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/clip.mp4");
        let stream = |id: &str, kind, role| Stream {
            id: id.into(),
            kind,
            role,
            name: id.into(),
            audio_path: None,
            fps: None,
        };
        let mut document = EditDocument::default();
        document.assets.push(Asset {
            id: "v".into(),
            name: "Talk & demo".into(),
            kind: AssetKind::Video,
            path: footage.to_string_lossy().into_owned(),
            streams: vec![
                stream("picture", StreamKind::Picture, Role::Overlay),
                stream("sound0", StreamKind::Sound, Role::Background),
            ],
            duration_us: 6_000_000,
            width: 0,
            height: 0,
            pauses: Vec::new(),
            missing: false,
            licence: None,
        });
        let graphic =
            crate::graphics::create(dir.path(), "title", None, &serde_json::Map::new()).unwrap();
        document.assets.push(graphic.clone());
        let mut v1 = Track::new("t1".into(), TrackKind::Video);
        v1.clips.push(clip("c1", "v", "picture", 0.0, 1.0, 2.0));
        let mut v2 = Track::new("t2".into(), TrackKind::Video);
        v2.clips
            .push(clip("g1", &graphic.id, "picture", 0.5, 0.0, 1.0));
        let mut a1 = Track::new("t3".into(), TrackKind::Audio);
        let mut sound = clip("c2", "v", "sound0", 0.0, 1.0, 2.0);
        sound.speed = 2.0;
        a1.clips.push(sound);
        document.sequence.tracks = vec![v1, v2, a1];
        let made = Written {
            clip: "g1".into(),
            path: footage.clone(),
            start_us: 500_000,
            duration_us: 1_000_000,
        };
        let handoff = build(dir.path(), &document, "Lesson", (1920, 1080), 30, &[made]).unwrap();
        assert_eq!(handoff.frames, 60);
        assert_eq!(handoff.sources.len(), 1, "one file read three times");
        assert!(handoff.sources[0].sound && handoff.sources[0].size.is_some());
        assert!(
            handoff.notes[1].contains("Talk & demo at 0:00"),
            "{:?}",
            handoff.notes
        );

        let fcpxml = handoff.fcpxml();
        well_formed(&fcpxml);
        assert!(fcpxml.contains("<fcpxml version=\"1.9\">"));
        assert!(fcpxml.contains("frameDuration=\"1/30s\" width=\"1920\" height=\"1080\""));
        assert!(fcpxml.contains("name=\"Talk &amp; demo\""));
        assert!(
            fcpxml.contains("lane=\"1\" offset=\"0s\" name=\"Talk &amp; demo\" duration=\"2s\" start=\"1s\" srcEnable=\"video\""),
            "{fcpxml}"
        );
        assert!(fcpxml.contains("lane=\"2\" offset=\"1/2s\""), "{fcpxml}");
        assert!(fcpxml.contains("lane=\"-1\" offset=\"0s\""), "{fcpxml}");
        assert!(fcpxml.contains("srcEnable=\"audio\""));
        assert!(fcpxml.contains("src=\"file:///"));

        let xml = handoff.xmeml();
        well_formed(&xml);
        assert!(
            xml.contains("<start>0</start><end>60</end><in>30</in><out>90</out>"),
            "{xml}"
        );
        assert!(
            xml.contains("<start>15</start><end>45</end><in>0</in><out>30</out>"),
            "{xml}"
        );
        assert!(xml.contains("<pathurl>file://localhost/"));
        // The file is described once, then pointed at.
        assert_eq!(xml.matches("<pathurl>").count(), 1);
        assert_eq!(xml.matches("<file id=\"file-1\"/>").count(), 2);
        assert!(xml.contains("<mediatype>audio</mediatype>"));

        write(&handoff, dir.path()).unwrap();
        // HANDOFF_OUT=<dir> keeps a copy to open in an editor.
        if let Ok(out) = std::env::var("HANDOFF_OUT") {
            write(&handoff, Path::new(&out)).unwrap();
        }
        assert!(dir.path().join("Lesson.fcpxml").is_file());
        assert!(dir.path().join("Lesson.xml").is_file());
        let read_me = std::fs::read_to_string(dir.path().join("Read me.txt")).unwrap();
        assert!(
            read_me.contains("Lesson.fcpxml opens in Final Cut Pro"),
            "{read_me}"
        );
    }
}
