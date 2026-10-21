//! Check before export: problems a viewer would notice, found from the edit itself (nothing is
//! decoded, no model runs), each with where it is on the timeline and, when one is safe, a fix
//! that applies as one undo step. The export dialog lists them; agents get them from the
//! `check` action and fix them before asking to export.
use crate::project::revision::EditDocument;
use crate::sequence::{Asset, Role};
use crate::timeline::TimelineMapper;
use crate::transcript::{store, Transcript, WordKind};
use serde::Serialize;
use std::path::Path;

/// A gap in the picture shorter than this (a frame at 30 fps) never reaches a frame.
const BLACK_MIN_US: u64 = 34_000;
/// A piece of the edit between two cuts shorter than this, with no word in it, is a blip.
const FRAGMENT_MAX_US: u64 = 300_000;
/// Without a transcript only a piece too short to hold a word counts as a blip.
const FRAGMENT_NO_WORDS_MAX_US: u64 = 120_000;
/// Word-edge refinement trims up to 200 ms of quiet from a word; more than that cut from a
/// kept word (and this share of it) takes sound with it.
const CLIPPED_REMOVED_MIN_US: u64 = 200_000;
const CLIPPED_SHARE: f64 = 0.3;
/// A zoom on screen for less than this flashes.
const ZOOM_MIN_US: u64 = 800_000;
/// YouTube shows chapters only when there are at least 3, each at least 10 s.
const MIN_CHAPTERS: usize = 3;
const CHAPTER_MIN_US: u64 = 10_000_000;
/// What YouTube plays at; it turns louder videos down but never quieter ones up.
pub const YOUTUBE_LUFS: f32 = -14.0;
/// How far a normalize target may sit below YouTube's before the video sounds quiet.
const QUIET_TARGET_MARGIN: f32 = 2.0;
/// Room left between raised captions and the webcam bubble, as a share of the canvas height.
const CAPTION_CLEARANCE_PCT: f32 = 1.5;
/// Findings of one kind listed at most; the rest are summed up in the last one.
const MAX_PER_KIND: usize = 40;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// The export fails or comes out broken.
    Error,
    /// Something a viewer will likely notice.
    Warning,
    /// Worth knowing; often fine.
    Info,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    MissingMedia,
    BlackFrames,
    NoSound,
    Fragment,
    ClippedWord,
    ZoomTooShort,
    CaptionsCoverWebcam,
    CaptionsWithoutTranscript,
    ChapterCut,
    ChapterTooShort,
    TooFewChapters,
    Loudness,
    ProposalsPending,
}

impl Kind {
    pub fn name(self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default()
    }
}

/// A fix that applies without asking anything more.
#[derive(Clone, Debug, PartialEq)]
pub enum Fix {
    /// Ripple-delete this edited range from every track.
    Cut {
        start_us: u64,
        end_us: u64,
    },
    DeleteZoom {
        id: String,
    },
    CaptionOffset {
        offset_pct: f32,
    },
    RemoveChapter {
        id: String,
    },
    Normalize {
        target_lufs: f32,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Names this problem while the edit stays at this revision.
    pub id: String,
    pub kind: Kind,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    /// Where it is on the edited timeline; none when it is about the whole video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_us: Option<u64>,
    /// What the fix does, for its button; none when it needs someone to decide.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_label: Option<String>,
    #[serde(skip)]
    pub fix: Option<Fix>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckReport {
    pub revision: u64,
    pub duration_us: u64,
    /// Errors first, then warnings, then the rest; each in timeline order.
    pub findings: Vec<Finding>,
}

struct Builder {
    findings: Vec<Finding>,
}

impl Builder {
    fn push(
        &mut self,
        kind: Kind,
        severity: Severity,
        title: String,
        detail: String,
    ) -> &mut Finding {
        self.findings.push(Finding {
            id: String::new(),
            kind,
            severity,
            title,
            detail,
            start_us: None,
            end_us: None,
            fix_label: None,
            fix: None,
        });
        self.findings.last_mut().unwrap()
    }
}

impl Finding {
    fn at(&mut self, start_us: u64, end_us: u64) -> &mut Self {
        self.start_us = Some(start_us);
        self.end_us = Some(end_us.max(start_us));
        self
    }

    fn fix(&mut self, label: impl Into<String>, fix: Fix) -> &mut Self {
        self.fix_label = Some(label.into());
        self.fix = Some(fix);
        self
    }
}

/// `m:ss.s` on the edited timeline.
pub fn clock(us: u64) -> String {
    let tenths = us / 100_000;
    format!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
}

fn seconds(us: u64) -> String {
    format!("{:.2} s", us as f64 / 1e6)
}

/// Checks the edit in `document` (the project at `root`). `missing` names the assets whose
/// files are gone.
pub fn run(root: &Path, document: &EditDocument, missing: &[&str]) -> CheckReport {
    let mut out = Builder {
        findings: Vec::new(),
    };
    let speech = speech_transcripts(root, document);
    missing_media(&mut out, document, missing);
    picture_gaps(&mut out, document);
    let has_sound = sound(&mut out, document);
    cuts(&mut out, document, &speech);
    clipped_words(&mut out, &speech);
    zooms(&mut out, document);
    captions(&mut out, root, document);
    chapters(&mut out, document);
    if has_sound {
        loudness(&mut out, document);
    }
    proposals(&mut out, root);

    let mut findings = cap(out.findings);
    findings.sort_by(|a, b| {
        (a.severity, a.start_us.is_some(), a.start_us).cmp(&(
            b.severity,
            b.start_us.is_some(),
            b.start_us,
        ))
    });
    // Ids from the kind and the moment (or the thing) it is about, made unique.
    let mut taken = std::collections::HashSet::new();
    for finding in &mut findings {
        let about = match &finding.fix {
            Some(Fix::DeleteZoom { id }) | Some(Fix::RemoveChapter { id }) => id.clone(),
            _ => finding
                .start_us
                .map(|us| (us / 1000).to_string())
                .unwrap_or_else(|| "all".into()),
        };
        let base = format!("{}-{about}", finding.kind.name());
        let mut id = base.clone();
        let mut n = 2;
        while !taken.insert(id.clone()) {
            id = format!("{base}-{n}");
            n += 1;
        }
        finding.id = id;
    }
    CheckReport {
        revision: document.revision,
        duration_us: document.duration_us(),
        findings,
    }
}

/// At most [`MAX_PER_KIND`] findings of each kind; the last one says how many more there are.
fn cap(findings: Vec<Finding>) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::with_capacity(findings.len());
    let mut counts = std::collections::HashMap::<String, usize>::new();
    for finding in findings {
        let seen = counts.entry(finding.kind.name()).or_default();
        *seen += 1;
        if *seen <= MAX_PER_KIND {
            out.push(finding);
        }
    }
    for (kind, count) in counts {
        if count > MAX_PER_KIND {
            if let Some(last) = out.iter_mut().rev().find(|f| f.kind.name() == kind) {
                last.detail.push_str(&format!(
                    " ({} more like this are not listed.)",
                    count - MAX_PER_KIND
                ));
            }
        }
    }
    out
}

/// The transcripts of the speech on the timeline, each with the clock of its sound.
fn speech_transcripts(root: &Path, document: &EditDocument) -> Vec<(Transcript, TimelineMapper)> {
    let mut keys: Vec<String> = Vec::new();
    for track in document.sequence.audio_tracks() {
        for clip in &track.clips {
            if crate::sequence::clip_role(&document.assets, track, clip) == Some(Role::Mic) {
                let key = clip.source().key();
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
    }
    keys.into_iter()
        .filter_map(|key| {
            let transcript = store::load_transcript(root, &key).ok()??;
            let mapper = document.transcript_clock(&key).ok()?;
            Some((transcript, mapper))
        })
        .collect()
}

fn missing_media(out: &mut Builder, document: &EditDocument, missing: &[&str]) {
    for asset in document
        .assets
        .iter()
        .filter(|a| missing.contains(&a.id.as_str()))
    {
        let Some(first) = document
            .sequence
            .clips()
            .filter(|(_, c)| c.asset == asset.id)
            .map(|(_, c)| c)
            .min_by_key(|c| c.start_us)
        else {
            continue;
        };
        out.push(
            Kind::MissingMedia,
            Severity::Error,
            format!("\"{}\" is missing", asset.name),
            format!(
                "Its file is gone ({}), so export stops. Put the file back or remove its clips.",
                asset.path
            ),
        )
        .at(first.start_us, first.end_us());
    }
}

/// Stretches where no visible video track shows anything: black frames.
fn picture_gaps(out: &mut Builder, document: &EditDocument) {
    let duration = document.duration_us();
    if duration == 0 {
        return;
    }
    let mut shown: Vec<(u64, u64)> = document
        .sequence
        .video_tracks()
        .filter(|t| !t.hidden)
        .flat_map(|t| t.clips.iter().map(|c| (c.start_us, c.end_us())))
        .collect();
    shown.sort_unstable();
    let mut gaps = Vec::new();
    let mut cursor = 0u64;
    for (a, b) in shown {
        if a > cursor {
            gaps.push((cursor, a));
        }
        cursor = cursor.max(b);
    }
    if cursor < duration {
        gaps.push((cursor, duration));
    }
    for (a, b) in gaps.into_iter().filter(|(a, b)| b - a >= BLACK_MIN_US) {
        let empty = document.sequence.tracks.iter().all(|t| t.is_free(a, b));
        let finding = out.push(
            Kind::BlackFrames,
            Severity::Warning,
            "Black frames".into(),
            format!(
                "{} with no picture from {} to {}{}.",
                seconds(b - a),
                clock(a),
                clock(b),
                if empty { "" } else { " (sound keeps playing)" }
            ),
        );
        finding.at(a, b);
        if empty {
            finding.fix(
                "Close the gap",
                Fix::Cut {
                    start_us: a,
                    end_us: b,
                },
            );
        }
    }
}

/// Whether the export has any sound; says so when it has none.
fn sound(out: &mut Builder, document: &EditDocument) -> bool {
    let audible = document
        .sequence
        .audio_tracks()
        .any(|t| !t.muted && !t.clips.is_empty());
    if !audible && document.duration_us() > 0 {
        out.push(
            Kind::NoSound,
            Severity::Warning,
            "No sound".into(),
            "Every audio track is empty or muted, so the video exports silent.".into(),
        );
    }
    audible
}

/// A stretch of the edit that plays its sound's moments back to back, with whether a cut
/// (a jump in the recording, or a gap) comes right before and after it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Piece {
    edited_start: u64,
    edited_end: u64,
    source_start: u64,
    source_end: u64,
    cut_before: bool,
    cut_after: bool,
}

/// The pieces of `clock`, in timeline order.
fn pieces(clock: &TimelineMapper) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    let mut edited = 0u64;
    let mut broke = false;
    for interval in clock.intervals() {
        let length = interval.duration_us();
        if !interval.is_recording() {
            broke = true;
            edited += length;
            continue;
        }
        match out.last_mut() {
            Some(last) if !broke && last.source_end == interval.start_us => {
                last.edited_end = edited + length;
                last.source_end = interval.end_us;
            }
            last => {
                let cut = last.is_some();
                if let Some(last) = last {
                    last.cut_after = true;
                }
                out.push(Piece {
                    edited_start: edited,
                    edited_end: edited + length,
                    source_start: interval.start_us,
                    source_end: interval.end_us,
                    cut_before: cut,
                    cut_after: false,
                });
            }
        }
        broke = false;
        edited += length;
    }
    out
}

/// The clock the cuts are counted on, with its sound's key: the first speech on the timeline,
/// else the first recording's picture.
fn edit_clock(document: &EditDocument) -> Option<(Option<String>, TimelineMapper)> {
    let mic = document.sequence.audio_tracks().find_map(|track| {
        track.clips.iter().find(|clip| {
            crate::sequence::clip_role(&document.assets, track, clip) == Some(Role::Mic)
        })
    });
    if let Some(clip) = mic {
        return Some((
            Some(clip.source().key()),
            document.stream_clock(&clip.source()),
        ));
    }
    let recording = document.assets.iter().find(|a| a.is_recording())?;
    Some((None, document.picture_clock(&recording.id)))
}

fn cuts(out: &mut Builder, document: &EditDocument, speech: &[(Transcript, TimelineMapper)]) {
    let Some((key, mapper)) = edit_clock(document) else {
        return;
    };
    let pieces = pieces(&mapper);
    // The words of that sound, when it is transcribed.
    let words: Option<Vec<(u64, u64)>> = speech
        .iter()
        .find(|(t, _)| key.as_deref() == Some(t.track_id.as_str()))
        .map(|(t, _)| {
            t.words
                .iter()
                .filter(|w| w.kind == WordKind::Word)
                .map(|w| (w.source_start_us, w.source_end_us))
                .collect()
        });
    for piece in pieces.iter().filter(|p| p.cut_before && p.cut_after) {
        let length = piece.edited_end - piece.edited_start;
        let blip = match &words {
            Some(words) => {
                length < FRAGMENT_MAX_US
                    && !words.iter().any(|&(a, b)| {
                        let mid = a + (b - a) / 2;
                        (piece.source_start..piece.source_end).contains(&mid)
                    })
            }
            None => length < FRAGMENT_NO_WORDS_MAX_US,
        };
        if blip {
            out.push(
                Kind::Fragment,
                Severity::Warning,
                "Blip between two cuts".into(),
                format!(
                    "{} of sound with no words plays between two cuts at {}; it usually \
                     sounds like a click or half a breath.",
                    seconds(length),
                    clock(piece.edited_start)
                ),
            )
            .at(piece.edited_start, piece.edited_end)
            .fix(
                "Cut it",
                Fix::Cut {
                    start_us: piece.edited_start,
                    end_us: piece.edited_end,
                },
            );
        }
    }
}

/// Kept words that lost more than word-edge refinement trims: cut partway through.
fn clipped_words(out: &mut Builder, speech: &[(Transcript, TimelineMapper)]) {
    for (transcript, mapper) in speech {
        for word in transcript.words.iter().filter(|w| w.kind == WordKind::Word) {
            let length = word.source_end_us.saturating_sub(word.source_start_us);
            if length == 0 {
                continue;
            }
            let kept_ranges =
                mapper.source_range_to_edited(word.source_start_us, word.source_end_us);
            // In the word's own time: a word played faster is shorter, not cut.
            let kept = mapper.source_kept_us(word.source_start_us, word.source_end_us);
            let removed = length - kept.min(length);
            if kept == 0
                || removed < CLIPPED_REMOVED_MIN_US
                || (removed as f64) < length as f64 * CLIPPED_SHARE
            {
                continue;
            }
            let (start, end) = (kept_ranges[0].0, kept_ranges.last().unwrap().1);
            out.push(
                Kind::ClippedWord,
                Severity::Warning,
                format!("\"{}\" cut partway", word.text.trim()),
                format!(
                    "A cut takes {} of this word's {}, so it may sound cut off. Restore the \
                     cut next to it or cut the whole word.",
                    seconds(removed),
                    seconds(length)
                ),
            )
            .at(start, end);
        }
    }
}

fn zooms(out: &mut Builder, document: &EditDocument) {
    for zoom in document.zooms_with_ranges() {
        let shown: u64 = zoom
            .edited_ranges
            .iter()
            .map(|r| r.end_us - r.start_us)
            .sum();
        let Some(first) = zoom.edited_ranges.first() else {
            continue;
        };
        let (start, end) = (first.start_us, zoom.edited_ranges.last().unwrap().end_us);
        if shown < ZOOM_MIN_US {
            out.push(
                Kind::ZoomTooShort,
                Severity::Warning,
                "Zoom flashes by".into(),
                format!(
                    "Cuts leave this zoom {} on screen, so it zooms in and straight back out.",
                    seconds(shown)
                ),
            )
            .at(start, end)
            .fix(
                "Delete the zoom",
                Fix::DeleteZoom {
                    id: zoom.id.clone(),
                },
            );
        }
        // A cut through a zoom's glide in or out no longer makes it jump: the glide is timed on
        // the edited video (`zoom::kept_stretches`).
    }
}

/// The size of a camera's picture: from the recording's manifest, else the asset's own.
fn camera_size(asset: &Asset, stream: &str) -> (u32, u32) {
    let from_manifest = || -> Option<(u32, u32)> {
        let bytes = std::fs::read(Path::new(&asset.path).join("manifest.json")).ok()?;
        let manifest: crate::project::ProjectManifest = serde_json::from_slice(&bytes).ok()?;
        let track = manifest.tracks.iter().find(|t| t.id == stream)?;
        Some((track.width?, track.height?))
    };
    if asset.is_recording() {
        from_manifest().unwrap_or((1280, 720))
    } else if asset.width > 0 && asset.height > 0 {
        (asset.width, asset.height)
    } else {
        (1280, 720)
    }
}

fn captions(out: &mut Builder, root: &Path, document: &EditDocument) {
    let settings = &document.captions;
    if !settings.enabled || document.short_layout.is_some() {
        return;
    }
    let transcript = crate::captions::caption_source(
        settings,
        crate::export::caption_candidates(document),
        Vec::new(),
        |key| store::load_transcript(root, key).ok().flatten(),
    );
    let Some(transcript) = transcript else {
        out.push(
            Kind::CaptionsWithoutTranscript,
            Severity::Warning,
            "Captions are on but nothing is transcribed".into(),
            "No captions will show. Transcribe the speech, or switch captions off.".into(),
        );
        return;
    };
    let layout = &document.layout;
    let Ok(mapper) = document.transcript_clock(&transcript.track_id) else {
        return;
    };
    let Ok((canvas_w, canvas_h)) = layout.preview_dimensions() else {
        return;
    };
    let cues = crate::captions::build_cues(&transcript, &mapper, settings);
    let layouts = crate::layouts::resolve::of_document(document);
    let mut sizes = std::collections::HashMap::<(String, String), (u32, u32)>::new();
    // (cue start, cue end, the bubble's top and bottom) of each caption over the bubble.
    let mut covered: Vec<(u64, u64, u32, u32)> = Vec::new();
    for cue in &cues {
        let mid = cue.start_us + (cue.end_us - cue.start_us) / 2;
        // Only while a bubble shows (another layout puts the camera elsewhere).
        let crate::layouts::Layout::Bubble(bubble) = layouts.layout_at(mid) else {
            continue;
        };
        let Some((_, clip)) = document.top_clip_at(mid, Role::Webcam) else {
            continue;
        };
        let Some(asset) = document.asset(&clip.asset) else {
            continue;
        };
        let (src_w, src_h) = *sizes
            .entry((clip.asset.clone(), clip.stream.clone()))
            .or_insert_with(|| camera_size(asset, &clip.stream));
        let (bx, by, bw, bh) =
            crate::render::webcam_bubble_rect(canvas_w, canvas_h, src_w, src_h, layout, bubble);
        let Some((w, h)) = crate::captions::cue_size(cue, settings, canvas_w, canvas_h) else {
            continue;
        };
        let (x, y) = crate::captions::placement(settings, canvas_w, canvas_h, w, h);
        if x < bx + bw && bx < x + w && y < by + bh && by < y + h {
            covered.push((cue.start_us, cue.end_us, by, by + bh));
        }
    }
    let Some(&(first_start, first_end, _, _)) = covered.first() else {
        return;
    };
    let finding = out.push(
        Kind::CaptionsCoverWebcam,
        Severity::Warning,
        "Captions cover the webcam".into(),
        format!(
            "{} of {} captions overlap the webcam bubble, the first at {}.",
            covered.len(),
            cues.len(),
            clock(first_start)
        ),
    );
    finding.at(first_start, first_end);
    // Captions moved clear of the bubble: above it from the bottom, below it from the top.
    let clearance = CAPTION_CLEARANCE_PCT / 100.0 * canvas_h as f32;
    let needed_px = match settings.position.as_str() {
        "bottom" => covered
            .iter()
            .map(|&(_, _, top, _)| canvas_h as f32 - top as f32 + clearance)
            .fold(0.0, f32::max),
        "top" => covered
            .iter()
            .map(|&(_, _, _, bottom)| bottom as f32 + clearance)
            .fold(0.0, f32::max),
        _ => return,
    };
    let offset_pct = (needed_px / canvas_h as f32 * 100.0 * 2.0).ceil() / 2.0;
    if offset_pct > settings.offset_pct && offset_pct <= crate::captions::OFFSET_PCT_RANGE.1 {
        finding.fix(
            format!(
                "Move captions {} the bubble ({offset_pct}% from the {})",
                if settings.position == "bottom" {
                    "above"
                } else {
                    "below"
                },
                settings.position
            ),
            Fix::CaptionOffset { offset_pct },
        );
    }
}

fn chapters(out: &mut Builder, document: &EditDocument) {
    if document.chapters.is_empty() {
        return;
    }
    for chapter in &document.chapters {
        if crate::chapters::edited_at(chapter, document).is_none() {
            out.push(
                Kind::ChapterCut,
                Severity::Warning,
                format!("Chapter \"{}\" was cut", chapter.title),
                "The moment it starts at is no longer in the edit, so export leaves it out. \
                 Remove it, or make chapters again after the last cut."
                    .into(),
            )
            .fix(
                "Remove the chapter",
                Fix::RemoveChapter {
                    id: chapter.id.clone(),
                },
            );
        }
    }
    let placed = crate::chapters::timeline(&document.chapters, document);
    let duration = document.duration_us();
    if (1..MIN_CHAPTERS).contains(&placed.len()) {
        out.push(
            Kind::TooFewChapters,
            Severity::Info,
            format!(
                "Only {} chapter{}",
                placed.len(),
                if placed.len() == 1 { "" } else { "s" }
            ),
            format!("YouTube shows chapters only when a video has at least {MIN_CHAPTERS}."),
        );
    }
    for (i, (start, title)) in placed.iter().enumerate() {
        let end = placed.get(i + 1).map_or(duration, |next| next.0);
        let length = end.saturating_sub(*start);
        if length < CHAPTER_MIN_US {
            out.push(
                Kind::ChapterTooShort,
                Severity::Warning,
                format!("Chapter \"{title}\" is short"),
                format!(
                    "It lasts {}; YouTube shows no chapters at all when one is under 10 s.",
                    seconds(length)
                ),
            )
            .at(*start, end);
        }
    }
}

fn loudness(out: &mut Builder, document: &EditDocument) {
    let audio = &document.audio;
    let fix = Fix::Normalize {
        target_lufs: YOUTUBE_LUFS,
    };
    if !audio.normalize {
        out.push(
            Kind::Loudness,
            Severity::Info,
            "Loudness is not normalized".into(),
            format!(
                "YouTube plays videos at {YOUTUBE_LUFS} LUFS and never turns a quiet one up; \
                 normalizing makes the level match other videos."
            ),
        )
        .fix(format!("Normalize to {YOUTUBE_LUFS} LUFS"), fix);
    } else if audio.target_lufs < YOUTUBE_LUFS - QUIET_TARGET_MARGIN {
        out.push(
            Kind::Loudness,
            Severity::Info,
            "Normalized quieter than YouTube".into(),
            format!(
                "The target is {} LUFS; YouTube plays at {YOUTUBE_LUFS} and never turns a \
                 quiet video up, so it will sound quieter than others.",
                audio.target_lufs
            ),
        )
        .fix(format!("Normalize to {YOUTUBE_LUFS} LUFS"), fix);
    }
}

fn proposals(out: &mut Builder, root: &Path) {
    let Ok(proposals) = crate::proposals::load(root) else {
        return;
    };
    let pending = proposals
        .batches
        .iter()
        .filter(|b| b.status == crate::proposals::Status::Pending)
        .count();
    if pending > 0 {
        out.push(
            Kind::ProposalsPending,
            Severity::Info,
            format!(
                "{pending} agent proposal{} waiting",
                if pending == 1 { "" } else { "s" }
            ),
            "They are not in the edit until accepted in the Agent panel.".into(),
        );
    }
}

/// The report as text, for agents and the CLI: one line per finding with its id.
pub fn report_text(report: &CheckReport) -> String {
    if report.findings.is_empty() {
        return format!(
            "No problems found (revision {}, {} long).",
            report.revision,
            clock(report.duration_us)
        );
    }
    let count = |s: Severity| report.findings.iter().filter(|f| f.severity == s).count();
    let mut lines = vec![format!(
        "Check of revision {}: {} errors, {} warnings, {} notes.",
        report.revision,
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info)
    )];
    lines.extend(report.findings.iter().map(finding_line));
    if report.findings.iter().any(|f| f.fix.is_some()) {
        lines.push(
            "`check` with `fix` (ids, or [\"all\"]) applies the fixes, each one undo step.".into(),
        );
    }
    lines.join("\n")
}

/// What is left after fixes, briefly: the counts, and only the errors and warnings in full.
pub fn left_text(report: &CheckReport) -> String {
    let count = |s: Severity| report.findings.iter().filter(|f| f.severity == s).count();
    let mut lines = vec![format!(
        "Now, at revision {}: {} errors, {} warnings, {} notes (this is the check after the \
         fixes; `check` again only after another edit).",
        report.revision,
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info)
    )];
    lines.extend(
        report
            .findings
            .iter()
            .filter(|f| f.severity != Severity::Info)
            .map(finding_line),
    );
    lines.join("\n")
}

/// `- [warning] 1:02.3-1:02.5 fragment-62300: Blip between two cuts ... Fix: Cut it.`
fn finding_line(f: &Finding) -> String {
    let severity = match f.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    };
    let at = match (f.start_us, f.end_us) {
        (Some(a), Some(b)) if b > a => format!(" {}-{}", clock(a), clock(b)),
        (Some(a), _) => format!(" {}", clock(a)),
        _ => String::new(),
    };
    let fix = f
        .fix_label
        .as_ref()
        .map(|label| format!(" Fix: {label}."))
        .unwrap_or_default();
    format!("- [{severity}]{at} {}: {} {}{fix}", f.id, f.title, f.detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::edit::SequenceEdit;
    use crate::transcript::{test_word, ProviderKind};
    use crate::zoom::EditedRange;

    const S: u64 = 1_000_000;
    const MS: u64 = 1_000;

    fn document(seconds: u64) -> EditDocument {
        EditDocument::from_recording(crate::sequence::tests::recording("rec", seconds * S, &[]))
            .unwrap()
    }

    fn cut(document: &mut EditDocument, ranges: &[(u64, u64)]) {
        let edit = SequenceEdit::DeleteRange {
            ranges: ranges
                .iter()
                .map(|&(start_us, end_us)| EditedRange { start_us, end_us })
                .collect(),
            ripple: Some(true),
        };
        document.sequence =
            crate::sequence::edit::apply(&document.sequence, &document.assets, &edit)
                .unwrap()
                .0;
    }

    fn save_words(root: &Path, words: &[(&str, u64, u64)]) {
        let mut words: Vec<_> = words.iter().map(|(t, s, e)| test_word(t, *s, *e)).collect();
        for (i, w) in words.iter_mut().enumerate() {
            w.id = format!("w-{i}");
        }
        let transcript = Transcript::new(
            "rec.mic".into(),
            ProviderKind::Parakeet,
            "test".into(),
            None,
            words,
        );
        store::save_transcript(root, &transcript).unwrap();
    }

    fn kinds(report: &CheckReport) -> Vec<Kind> {
        report.findings.iter().map(|f| f.kind).collect()
    }

    #[test]
    fn a_plain_recording_only_suggests_loudness() {
        let dir = tempfile::tempdir().unwrap();
        let report = run(dir.path(), &document(60), &[]);
        assert_eq!(kinds(&report), vec![Kind::Loudness]);
        let fix = report.findings[0].fix.clone();
        assert_eq!(fix, Some(Fix::Normalize { target_lufs: -14.0 }));
        assert_eq!(report.findings[0].id, "loudness-all");
    }

    #[test]
    fn a_blip_between_cuts_is_found_and_a_short_word_is_not() {
        let dir = tempfile::tempdir().unwrap();
        // Words at 1-2 s and "so" at 5.0-5.2 s; everything else around them is cut.
        save_words(dir.path(), &[("hello", 1000, 2000), ("so", 5000, 5200)]);
        let mut doc = document(20);
        doc.audio.normalize = true;
        // Keep 0-2 s, a 0.2 s blip at 3.0-3.2 s, "so" at 4.95-5.25 s, then 8 s on.
        cut(&mut doc, &[(5250 * MS, 8 * S)]);
        cut(&mut doc, &[(3200 * MS, 4950 * MS)]);
        cut(&mut doc, &[(2 * S, 3 * S)]);
        let report = run(dir.path(), &doc, &[]);
        let blips: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.kind == Kind::Fragment)
            .collect();
        assert_eq!(blips.len(), 1, "{:?}", report.findings);
        assert_eq!(blips[0].start_us, Some(2 * S));
        assert_eq!(
            blips[0].fix,
            Some(Fix::Cut {
                start_us: 2 * S,
                end_us: 2200 * MS
            })
        );
    }

    #[test]
    fn a_word_cut_beyond_edge_refinement_is_flagged() {
        let dir = tempfile::tempdir().unwrap();
        save_words(dir.path(), &[("long", 1000, 2000), ("fine", 3000, 3500)]);
        let mut doc = document(10);
        doc.audio.normalize = true;
        // 0.5 s off the end of "long"; 0.1 s off "fine" is within refinement.
        cut(&mut doc, &[(3400 * MS, 4 * S)]);
        cut(&mut doc, &[(1500 * MS, 2500 * MS)]);
        let report = run(dir.path(), &doc, &[]);
        let clipped: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.kind == Kind::ClippedWord)
            .collect();
        assert_eq!(clipped.len(), 1, "{:?}", report.findings);
        assert!(clipped[0].title.contains("long"));
    }

    #[test]
    fn black_gaps_get_a_fix_only_when_nothing_plays() {
        let dir = tempfile::tempdir().unwrap();
        let mut doc = document(20);
        doc.audio.normalize = true;
        // Lift 5-6 s out of every track without closing it.
        let edit = SequenceEdit::DeleteRange {
            ranges: vec![EditedRange {
                start_us: 5 * S,
                end_us: 6 * S,
            }],
            ripple: Some(false),
        };
        doc.sequence = crate::sequence::edit::apply(&doc.sequence, &doc.assets, &edit)
            .unwrap()
            .0;
        let report = run(dir.path(), &doc, &[]);
        let black = report
            .findings
            .iter()
            .find(|f| f.kind == Kind::BlackFrames)
            .expect("a gap");
        assert_eq!((black.start_us, black.end_us), (Some(5 * S), Some(6 * S)));
        assert_eq!(
            black.fix,
            Some(Fix::Cut {
                start_us: 5 * S,
                end_us: 6 * S
            })
        );
    }

    #[test]
    fn chapters_that_were_cut_or_are_short_are_flagged() {
        let dir = tempfile::tempdir().unwrap();
        let mut doc = document(120);
        doc.audio.normalize = true;
        let chapter = |id: &str, at: u64, title: &str| crate::chapters::Chapter {
            id: id.into(),
            source_us: at,
            title: title.into(),
            media: None,
            edited_us: None,
        };
        doc.chapters = vec![
            chapter("a", 0, "Intro"),
            chapter("b", 30 * S, "Gone"),
            chapter("c", 60 * S, "Short"),
            chapter("d", 65 * S, "Rest"),
        ];
        cut(&mut doc, &[(25 * S, 35 * S)]);
        let report = run(dir.path(), &doc, &[]);
        let cut_one = report
            .findings
            .iter()
            .find(|f| f.kind == Kind::ChapterCut)
            .unwrap();
        assert_eq!(cut_one.fix, Some(Fix::RemoveChapter { id: "b".into() }));
        assert_eq!(cut_one.id, "chapter_cut-b");
        let short: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.kind == Kind::ChapterTooShort)
            .collect();
        assert_eq!(short.len(), 1);
        assert!(short[0].title.contains("Short"));
    }

    #[test]
    fn captions_over_the_bubble_get_moved_above_it() {
        let dir = tempfile::tempdir().unwrap();
        save_words(
            dir.path(),
            &[
                ("a", 1000, 1300),
                ("caption", 1300, 1700),
                ("long", 1700, 2000),
                ("enough", 2000, 2400),
                ("to", 2400, 2500),
                ("reach", 2500, 2900),
            ],
        );
        let mut doc = document(10);
        doc.audio.normalize = true;
        doc.captions.enabled = true;
        doc.captions.font_size_pct = 12.0;
        let report = run(dir.path(), &doc, &[]);
        let covered = report
            .findings
            .iter()
            .find(|f| f.kind == Kind::CaptionsCoverWebcam)
            .expect("captions over the bottom-left bubble");
        let Some(Fix::CaptionOffset { offset_pct }) = covered.fix.clone() else {
            panic!("no fix: {covered:?}");
        };
        // Applied, the check is clean.
        doc.captions.offset_pct = offset_pct;
        let report = run(dir.path(), &doc, &[]);
        assert!(
            !kinds(&report).contains(&Kind::CaptionsCoverWebcam),
            "{:?}",
            report.findings
        );
    }

    #[test]
    fn missing_media_is_an_error_and_sorts_first() {
        let dir = tempfile::tempdir().unwrap();
        let report = run(dir.path(), &document(10), &["rec"]);
        assert_eq!(report.findings[0].kind, Kind::MissingMedia);
        assert_eq!(report.findings[0].severity, Severity::Error);
        assert!(report_text(&report).contains("[error]"));
    }
}
