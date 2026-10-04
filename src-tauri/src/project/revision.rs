//! Versioned edit document (`project.json`). Source media is never rewritten.
use super::layout::validate_layout;
use super::reader::{open_regular, safe_path};
use crate::sequence::edit::{EditOutcome, SequenceEdit};
use crate::sequence::{clock, Asset, Role, Sequence, StreamRef};
use crate::timeline::TimelineMapper;
use crate::webcam_focus::WebcamFocus;
use crate::zoom::{
    validate_zooms, ZoomKeyframe, ZoomSource, ZoomSuggestion, MAX_DISMISSED_ZOOMS, MAX_ZOOMS,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub use super::audio::AudioSettings;
pub use super::layout::EditLayout;
pub use crate::captions::CaptionSettings;
pub use crate::sequence::edit::TrimSide;

/// Version 2: assets and a sequence of tracks. Earlier documents are not read.
pub const EDIT_SCHEMA_VERSION: u32 = 2;
pub const MAX_EDIT_BYTES: u64 = 4_194_304;
pub const MAX_UNDO: usize = 64;
pub const MAX_CUTS_PER_REVISION: usize = 256;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditDocument {
    pub schema_version: u32,
    pub revision: u64,
    /// Everything that can be played: recordings, videos, images, audio.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<Asset>,
    /// The timeline: video and audio tracks of clips.
    #[serde(default)]
    pub sequence: Sequence,
    #[serde(default)]
    pub layout: EditLayout,
    #[serde(default)]
    pub zooms: Vec<ZoomKeyframe>,
    #[serde(default)]
    pub dismissed_zoom_ids: Vec<String>,
    /// Auto webcam layout: when the webcam grows to fill the canvas.
    #[serde(default, skip_serializing_if = "WebcamFocus::is_default")]
    pub webcam_focus: WebcamFocus,
    /// Loudness, noise reduction and ducking. Applies to playback and export.
    #[serde(default, skip_serializing_if = "AudioSettings::is_default")]
    pub audio: AudioSettings,
    /// Captions burned into playback and export from a sound stream's transcript.
    #[serde(default, skip_serializing_if = "CaptionSettings::is_default")]
    pub captions: CaptionSettings,
    /// Chapter markers, anchored in an asset's time. Exported as MP4 chapters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<crate::chapters::Chapter>,
    /// Vertical clips picked from this video, anchored in an asset's time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shorts: Vec<crate::shorts::Short>,
    /// Set only on the document a short renders from: draw a split-screen vertical frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_layout: Option<crate::shorts::ShortLayout>,
    /// Auto-zoom settings: how far zooms go, how many, how the camera follows.
    #[serde(default, skip_serializing_if = "crate::zoom::ZoomSettings::is_default")]
    pub zoom_settings: crate::zoom::ZoomSettings,
}

impl Default for EditDocument {
    fn default() -> Self {
        Self {
            schema_version: EDIT_SCHEMA_VERSION,
            revision: 0,
            assets: Vec::new(),
            sequence: Sequence::default(),
            layout: EditLayout::default(),
            zooms: Vec::new(),
            dismissed_zoom_ids: Vec::new(),
            webcam_focus: WebcamFocus::default(),
            audio: AudioSettings::default(),
            captions: CaptionSettings::default(),
            chapters: Vec::new(),
            shorts: Vec::new(),
            short_layout: None,
            zoom_settings: Default::default(),
        }
    }
}

impl EditDocument {
    /// A project made from a recording: the recording as its first asset, on V1, V2, A1, A2.
    pub fn from_recording(recording: Asset) -> Result<Self, String> {
        let sequence = crate::sequence::edit::starting_sequence(
            std::slice::from_ref(&recording),
            &recording.id,
        )?;
        Ok(Self {
            assets: vec![recording],
            sequence,
            ..Self::default()
        })
    }

    /// Where the timeline ends.
    pub fn duration_us(&self) -> u64 {
        self.sequence.duration_us()
    }

    pub fn asset(&self, id: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.id == id)
    }

    /// The asset something anchored in time is on: its own, else the first recording.
    pub fn clock_asset<'a>(&'a self, media: Option<&'a str>) -> Option<&'a str> {
        media.or_else(|| {
            self.assets
                .iter()
                .find(|a| a.is_recording())
                .map(|a| a.id.as_str())
        })
    }

    /// Where an asset's picture (as the screen) plays: zooms and the cursor.
    pub fn picture_clock(&self, asset: &str) -> TimelineMapper {
        clock::picture_clock(&self.sequence, &self.assets, asset)
    }

    /// Where any clip of an asset plays: chapters and shorts.
    pub fn asset_clock(&self, asset: &str) -> TimelineMapper {
        clock::asset_clock(&self.sequence, asset)
    }

    /// Where a sound stream plays: transcripts, captions and pauses.
    pub fn stream_clock(&self, source: &StreamRef) -> TimelineMapper {
        clock::stream_clock(&self.sequence, &source.asset, &source.stream)
    }

    /// The clock of the stream a transcript (or waveform, or pause scan) is of, by its key.
    pub fn transcript_clock(&self, key: &str) -> Result<TimelineMapper, String> {
        Ok(self.stream_clock(&self.stream_ref(key)?))
    }

    /// The stream a key names, checked against the project.
    pub fn stream_ref(&self, key: &str) -> Result<StreamRef, String> {
        let source = StreamRef::parse(key).ok_or("Unknown sound")?;
        self.asset(&source.asset)
            .and_then(|a| a.stream(&source.stream))
            .ok_or("That sound is no longer in the project")?;
        Ok(source)
    }

    /// Where webcam focus lands: its recording's camera clips.
    pub fn focus_clock(&self) -> TimelineMapper {
        match self.clock_asset(self.webcam_focus.media.as_deref()) {
            Some(asset) => clock::role_clock(&self.sequence, &self.assets, asset, Role::Webcam),
            None => TimelineMapper::new(Vec::new()),
        }
    }

    /// The zooms on `asset`'s clock.
    pub fn media_zoom_suggestions(&self, asset: &str) -> Vec<ZoomSuggestion> {
        self.zooms
            .iter()
            .filter(|zoom| self.clock_asset(zoom.media.as_deref()) == Some(asset))
            .map(ZoomKeyframe::as_suggestion)
            .collect()
    }

    fn zoom_clock(&self, media: Option<&str>) -> Option<TimelineMapper> {
        self.clock_asset(media)
            .map(|asset| self.picture_clock(asset))
    }

    /// Where `zoom` shows on the timeline (nothing if its time was all cut).
    pub fn zoom_edited(&self, zoom: &ZoomKeyframe) -> Vec<(u64, u64)> {
        self.zoom_clock(zoom.media.as_deref())
            .map(|m| m.source_range_to_edited(zoom.source_start_us, zoom.source_end_us))
            .unwrap_or_default()
    }

    /// Whether `zoom` shares timeline time with any other zoom: zooms never overlap.
    pub fn zoom_overlaps(&self, zoom: &ZoomKeyframe) -> bool {
        let mine = self.zoom_edited(zoom);
        self.zooms.iter().filter(|z| z.id != zoom.id).any(|other| {
            let theirs = self.zoom_edited(other);
            mine.iter()
                .any(|&(a, b)| theirs.iter().any(|&(c, d)| a < d && c < b))
        })
    }

    /// The zooms with where each lands on the timeline, each on its own clock.
    pub fn zooms_with_ranges(&self) -> Vec<ZoomKeyframe> {
        let mut zooms = self.zooms.clone();
        crate::zoom::attach_zoom_edited_ranges_with(&mut zooms, &|media| self.zoom_clock(media));
        zooms
    }

    pub fn attach_zoom_ranges(&mut self) {
        self.zooms = self.zooms_with_ranges();
    }

    /// The topmost visible clip with role `role` at timeline time `us`, with its track.
    pub fn top_clip_at(
        &self,
        us: u64,
        role: Role,
    ) -> Option<(&crate::sequence::Track, &crate::sequence::Clip)> {
        self.sequence
            .video_tracks()
            .rev()
            .filter(|t| !t.hidden)
            .filter_map(|t| t.clip_at(us).map(|c| (t, c)))
            .find(|(t, c)| crate::sequence::clip_role(&self.assets, t, c) == Some(role))
    }

    /// Checks everything a stored document must satisfy.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != EDIT_SCHEMA_VERSION {
            return Err("Unsupported edit schema version".into());
        }
        crate::sequence::validate_assets(&self.assets)?;
        crate::sequence::validate_sequence(&self.sequence, &self.assets)?;
        validate_layout(&self.layout)?;
        validate_zooms(&self.zooms)?;
        validate_dismissed(&self.dismissed_zoom_ids)?;
        self.webcam_focus.validate()?;
        self.audio.validate()?;
        self.captions.validate()?;
        crate::chapters::validate(&self.chapters)?;
        crate::shorts::validate(&self.shorts)?;
        crate::shorts::validate_edits(self)?;
        Ok(())
    }
}

pub fn load_edit_document(root: &Path) -> Result<Option<EditDocument>, String> {
    let path = safe_path(root, "project.json")?;
    if !path.is_file() {
        return Ok(None);
    }
    let file = open_regular(&path)?;
    let mut bytes = Vec::new();
    file.take(MAX_EDIT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_EDIT_BYTES {
        return Err("Edit document exceeds size limit".into());
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Version {
        schema_version: u32,
    }
    let version: Version =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid edit document: {e}"))?;
    if version.schema_version != EDIT_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported edit schema version: {}",
            version.schema_version
        ));
    }
    let document: EditDocument =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid edit document: {e}"))?;
    document.validate()?;
    Ok(Some(document))
}

pub fn save_edit_document(root: &Path, document: &EditDocument) -> Result<(), String> {
    document.validate()?;
    let path = safe_path(root, "project.json")?;
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if meta.file_type().is_symlink() {
            return Err("Edit document cannot be a symlink".into());
        }
    }
    // What the UI is shown (where things land, missing files) is worked out, not stored.
    let mut stored = document.clone();
    for zoom in &mut stored.zooms {
        zoom.edited_ranges.clear();
    }
    for asset in &mut stored.assets {
        asset.missing = false;
    }
    let serialized = serde_json::to_vec_pretty(&stored).map_err(|e| e.to_string())?;
    if serialized.len() as u64 > MAX_EDIT_BYTES {
        return Err("Edit document exceeds size limit".into());
    }
    let mut temp = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    temp.write_all(&serialized).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(&path).map_err(|e| e.to_string())?;
    // The document is committed after the atomic rename. Directory sync is best-effort:
    // reporting a failed edit after publication would leave memory and disk divergent.
    if let Ok(directory) = fs::File::open(root) {
        let _ = directory.sync_all();
    }
    Ok(())
}

fn validate_dismissed(ids: &[String]) -> Result<(), String> {
    if ids.len() > MAX_DISMISSED_ZOOMS {
        return Err("Too many dismissed zoom ids".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        if id.is_empty() || id.len() > 128 || !seen.insert(id) {
            return Err("Invalid dismissed zoom id".into());
        }
    }
    Ok(())
}

fn persist_revision(root: &Path, expected: u64, next: &EditDocument) -> Result<(), String> {
    let lock_path = safe_path(root, ".edit.lock")?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        // No sharing: a second editor saving the same project gets a sharing
        // violation, which stands in for the Unix flock below.
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    let lock = options.open(lock_path).map_err(|e| e.to_string())?;
    if !lock.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Invalid edit lock".into());
    }
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("Another editor is saving this project".into());
        }
    }
    let result = (|| {
        // A document the editor could not read (from an earlier AeroEdits) counts as none.
        let on_disk = load_edit_document(root).ok().flatten();
        if on_disk.map(|d| d.revision).unwrap_or(0) != expected {
            return Err("Stale edit revision on disk; reopen the project".into());
        }
        save_edit_document(root, next)
    })();
    // Explicitly unlock: another thread may have forked a child that briefly
    // inherits this open-file description before exec closes CLOEXEC handles.
    // Merely closing our descriptor can then leave the lock held by the child.
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        unsafe {
            libc::flock(lock.as_raw_fd(), libc::LOCK_UN);
        }
    }
    result
}

#[derive(Clone, Debug)]
pub struct EditHistory {
    pub current: EditDocument,
    undo: Vec<EditDocument>,
    redo: Vec<EditDocument>,
}

impl EditHistory {
    pub fn new(current: EditDocument) -> Self {
        Self {
            current,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn undo_available(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn redo_available(&self) -> bool {
        !self.redo.is_empty()
    }

    fn check(&self, expected_revision: u64) -> Result<(), String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        Ok(())
    }

    pub fn commit_next(
        &mut self,
        expected_revision: u64,
        persist_root: &Path,
        mut next: EditDocument,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        next.schema_version = EDIT_SCHEMA_VERSION;
        next.validate()?;
        if next.short_layout.is_some() {
            return Err("A project's own edit cannot use a short's split layout".into());
        }
        next.revision = self
            .current
            .revision
            .checked_add(1)
            .ok_or("Revision overflow")?;
        next.attach_zoom_ranges();
        persist_revision(persist_root, expected_revision, &next)?;
        self.undo.push(self.current.clone());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.current = next;
        Ok(&self.current)
    }

    /// One change to the timeline.
    pub fn edit_sequence(
        &mut self,
        expected_revision: u64,
        edit: &SequenceEdit,
        persist_root: &Path,
    ) -> Result<EditOutcome, String> {
        self.check(expected_revision)?;
        let (sequence, outcome) =
            crate::sequence::edit::apply(&self.current.sequence, &self.current.assets, edit)?;
        let mut next = self.current.clone();
        next.sequence = sequence;
        self.commit_next(expected_revision, persist_root, next)?;
        Ok(outcome)
    }

    /// Cuts timeline ranges out of every track and closes them up, whatever magnetic says:
    /// jump cuts and cutting words always shorten the video.
    pub fn ripple_cuts(
        &mut self,
        expected_revision: u64,
        cuts: &[(u64, u64)],
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if cuts.is_empty() {
            return Err("No cuts selected".into());
        }
        if cuts.len() > MAX_CUTS_PER_REVISION {
            return Err("Too many cuts in one revision".into());
        }
        let mut ordered = cuts.to_vec();
        ordered.sort_unstable();
        if ordered.windows(2).any(|w| w[0].1 > w[1].0) {
            return Err("Cut ranges overlap".into());
        }
        let duration = self.current.duration_us();
        if ordered.iter().any(|&(a, b)| a >= b || b > duration) {
            return Err("A cut must be a range on the timeline".into());
        }
        self.edit_sequence(
            expected_revision,
            &SequenceEdit::DeleteRange {
                ranges: ordered
                    .iter()
                    .map(|&(start_us, end_us)| crate::zoom::EditedRange { start_us, end_us })
                    .collect(),
                ripple: Some(true),
            },
            persist_root,
        )?;
        Ok(&self.current)
    }

    pub fn update_layout(
        &mut self,
        expected_revision: u64,
        layout: EditLayout,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        validate_layout(&layout)?;
        if layout == self.current.layout {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.layout = layout;
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn update_webcam_focus(
        &mut self,
        expected_revision: u64,
        focus: WebcamFocus,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let focus = focus.normalized();
        focus.validate()?;
        if focus == self.current.webcam_focus {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.webcam_focus = focus;
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Replaces the chapter markers.
    pub fn set_chapters(
        &mut self,
        expected_revision: u64,
        chapters: Vec<crate::chapters::Chapter>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        crate::chapters::validate(&chapters)?;
        let chapters = crate::chapters::normalized(chapters);
        if chapters == self.current.chapters {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.chapters = chapters;
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Replaces the shorts list.
    pub fn set_shorts(
        &mut self,
        expected_revision: u64,
        shorts: Vec<crate::shorts::Short>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        crate::shorts::validate(&shorts)?;
        let shorts = crate::shorts::normalized(shorts);
        if shorts == self.current.shorts {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.shorts = shorts;
        self.commit_next(expected_revision, persist_root, next)
    }

    /// The recording webcam focus is on, and the source ranges of its camera that a timeline
    /// range covers. Focus stays on one recording; the first edit picks the camera under it.
    fn focus_pieces(&self, start_us: u64, end_us: u64) -> (Option<String>, Vec<(u64, u64)>) {
        let document = &self.current;
        let media = document.webcam_focus.media.clone().or_else(|| {
            document
                .top_clip_at(start_us, Role::Webcam)
                .map(|(_, c)| c.asset.clone())
                .or_else(|| document.clock_asset(None).map(String::from))
        });
        let pieces = media
            .as_deref()
            .map(|asset| {
                clock::role_clock(&document.sequence, &document.assets, asset, Role::Webcam)
                    .edited_range_to_source(start_us, end_us)
            })
            .unwrap_or_default();
        (media, pieces)
    }

    /// Adds a webcam focus segment over a timeline range of the camera.
    pub fn add_webcam_focus(
        &mut self,
        expected_revision: u64,
        edited_start_us: u64,
        edited_end_us: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if edited_end_us <= edited_start_us {
            return Err("Webcam focus must be a range on the timeline".into());
        }
        let (media, pieces) = self.focus_pieces(edited_start_us, edited_end_us);
        if pieces.is_empty() {
            return Err("Webcam focus needs the recording's camera there".into());
        }
        let mut focus = self.current.webcam_focus.clone();
        focus.media = media;
        focus.enabled = true;
        focus.add_focus(&pieces);
        self.update_webcam_focus(expected_revision, focus, persist_root)
    }

    /// Switches webcam focus off over a timeline range, whichever segments cover it.
    pub fn remove_webcam_focus(
        &mut self,
        expected_revision: u64,
        edited_start_us: u64,
        edited_end_us: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if edited_end_us <= edited_start_us {
            return Err("Webcam focus must be a range on the timeline".into());
        }
        let (_, pieces) = self.focus_pieces(edited_start_us, edited_end_us);
        let mut focus = self.current.webcam_focus.clone();
        focus.remove_focus(&pieces);
        self.update_webcam_focus(expected_revision, focus, persist_root)
    }

    pub fn update_audio(
        &mut self,
        expected_revision: u64,
        audio: AudioSettings,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        audio.validate()?;
        if audio == self.current.audio {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.audio = audio;
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn update_captions(
        &mut self,
        expected_revision: u64,
        captions: CaptionSettings,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        captions.validate()?;
        if captions == self.current.captions {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.captions = captions;
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn accept_zooms(
        &mut self,
        expected_revision: u64,
        suggestions: &[ZoomSuggestion],
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if suggestions.is_empty() {
            return Err("No zoom suggestions selected".into());
        }
        let mut next = self.current.clone();
        let existing: std::collections::BTreeSet<_> =
            next.zooms.iter().map(|z| z.id.clone()).collect();
        let dismissed: std::collections::BTreeSet<_> =
            next.dismissed_zoom_ids.iter().cloned().collect();
        let mut overlapping = 0;
        for suggestion in suggestions {
            if existing.contains(&suggestion.id) || dismissed.contains(&suggestion.id) {
                continue;
            }
            let zoom = ZoomKeyframe::from_suggestion(suggestion.clone(), ZoomSource::Generated);
            // Zooms never overlap: one landing on a zoom already there is left out.
            if next.zoom_overlaps(&zoom) {
                overlapping += 1;
                continue;
            }
            next.zooms.push(zoom);
        }
        if next.zooms.len() == self.current.zooms.len() {
            return Err(if overlapping > 0 {
                "Those zooms would overlap zooms already on the timeline".into()
            } else {
                "Those zoom suggestions are already applied or dismissed".into()
            });
        }
        if next.zooms.len() > MAX_ZOOMS {
            return Err("Too many zoom keyframes".into());
        }
        sort_zooms(&mut next.zooms);
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn dismiss_zooms(
        &mut self,
        expected_revision: u64,
        ids: &[String],
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if ids.is_empty() {
            return Err("No zoom ids selected".into());
        }
        let mut next = self.current.clone();
        let remove: std::collections::BTreeSet<_> = ids.iter().cloned().collect();
        next.zooms.retain(|z| !remove.contains(&z.id));
        for id in ids {
            if !next.dismissed_zoom_ids.iter().any(|d| d == id) {
                next.dismissed_zoom_ids.push(id.clone());
            }
        }
        if next.zooms == self.current.zooms
            && next.dismissed_zoom_ids == self.current.dismissed_zoom_ids
        {
            return Err("Those zoom ids are already dismissed".into());
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn update_zoom(
        &mut self,
        expected_revision: u64,
        patch: ZoomKeyframe,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        let mut next = self.current.clone();
        let Some(existing) = next.zooms.iter_mut().find(|z| z.id == patch.id) else {
            return Err("Unknown zoom keyframe".into());
        };
        existing.source_start_us = patch.source_start_us;
        existing.source_end_us = patch.source_end_us;
        existing.center_x = patch.center_x;
        existing.center_y = patch.center_y;
        existing.scale = patch.scale;
        existing.transition_us = patch.transition_us;
        existing.fixed = patch.fixed;
        // A zoom stays on its own clock.
        // Moving/resizing a generated zoom keeps its id so regeneration cannot
        // replace it, and marks it manual so a later accept cannot reset it.
        existing.source = ZoomSource::Manual;
        let moved = existing.clone();
        if next.zoom_overlaps(&moved) {
            return Err("Zooms can't overlap: it stops where the next zoom starts".into());
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    /// New auto-zoom settings. Automatic zooms take the new amounts and transition at once,
    /// so every zoom of a kind looks the same; zooms you set yourself keep theirs.
    pub fn set_zoom_settings(
        &mut self,
        expected_revision: u64,
        settings: crate::zoom::ZoomSettings,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        settings.validate()?;
        let mut next = self.current.clone();
        let transition = settings.transition_ms as u64 * 1_000;
        for zoom in next
            .zooms
            .iter_mut()
            .filter(|z| z.source == ZoomSource::Generated)
        {
            zoom.scale = settings.scale_for(zoom.origin);
            let length = zoom.source_end_us - zoom.source_start_us;
            zoom.transition_us = transition.min(length.saturating_sub(1) / 2).max(1);
        }
        next.zoom_settings = settings;
        if next == self.current {
            return Ok(&self.current);
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Takes the generated zooms off and puts the recordings' zooms back on, found again with
    /// the current auto-zoom settings (dismissed ones included); zooms you made or changed
    /// stay, and new ones never overlap them.
    pub fn reload_zooms(
        &mut self,
        expected_revision: u64,
        suggestions: &[ZoomSuggestion],
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        let mut next = self.current.clone();
        next.zooms.retain(|z| z.source == ZoomSource::Manual);
        next.dismissed_zoom_ids.clear();
        for suggestion in suggestions {
            if next.zooms.iter().any(|z| z.id == suggestion.id) {
                continue;
            }
            let zoom = ZoomKeyframe::from_suggestion(suggestion.clone(), ZoomSource::Generated);
            if !next.zoom_overlaps(&zoom) && next.zooms.len() < MAX_ZOOMS {
                next.zooms.push(zoom);
            }
        }
        sort_zooms(&mut next.zooms);
        if next.zooms == self.current.zooms
            && next.dismissed_zoom_ids == self.current.dismissed_zoom_ids
        {
            return Err("The recording's zooms are already on the timeline".into());
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    /// A zoom over a timeline range of the screen, on the clock of the screen clip there.
    #[allow(clippy::too_many_arguments)]
    pub fn add_manual_zoom(
        &mut self,
        expected_revision: u64,
        edited_start_us: u64,
        edited_end_us: u64,
        center_x: f64,
        center_y: f64,
        scale: f64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if edited_end_us <= edited_start_us {
            return Err("Zoom must be a range on the timeline".into());
        }
        // Zooms never overlap: the new one fits into the free time around its start.
        let (mut edited_start_us, mut edited_end_us) = (edited_start_us, edited_end_us);
        let mut taken: Vec<(u64, u64)> = self
            .current
            .zooms
            .iter()
            .flat_map(|z| self.current.zoom_edited(z))
            .collect();
        taken.sort_unstable();
        for (a, b) in taken {
            if a < edited_end_us && edited_start_us < b {
                if a <= edited_start_us {
                    edited_start_us = b;
                } else {
                    edited_end_us = edited_end_us.min(a);
                }
            }
        }
        if edited_end_us <= edited_start_us || edited_end_us - edited_start_us < 300_000 {
            return Err("There's a zoom here already: zooms can't overlap".into());
        }
        let media = self
            .current
            .top_clip_at(edited_start_us, Role::Screen)
            .map(|(_, clip)| clip.asset.clone())
            .ok_or("Zooms go on the screen, and there is no screen here")?;
        let mapper = self.current.picture_clock(&media);
        let source_start = mapper
            .edited_to_source_us(edited_start_us)
            .ok_or("Zoom start is not on the screen")?;
        let source_end_sample = mapper
            .edited_to_source_us(edited_end_us.saturating_sub(1))
            .ok_or("Zoom end must be on the same clip as its start")?;
        let source_end = source_end_sample.saturating_add(1);
        if source_end <= source_start {
            return Err("Zoom range does not map onto source time".into());
        }
        let duration = source_end - source_start;
        if duration < 3 {
            return Err("Zoom range is too short".into());
        }
        let transition_us = (duration / 5)
            .clamp(1, 400_000)
            .min(duration.saturating_sub(1));
        let mut next = self.current.clone();
        let id = format!("m-{}-{}", source_start, next.zooms.len());
        next.zooms.push(ZoomKeyframe {
            id,
            source_start_us: source_start,
            source_end_us: source_end,
            center_x,
            center_y,
            scale,
            transition_us,
            origin: crate::zoom::ZoomOrigin::Click,
            contributing_event_seqs: Vec::new(),
            source: ZoomSource::Manual,
            edited_ranges: Vec::new(),
            media: Some(media),
            fixed: false,
        });
        sort_zooms(&mut next.zooms);
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn delete_zoom(
        &mut self,
        expected_revision: u64,
        id: &str,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        let mut next = self.current.clone();
        let Some(index) = next.zooms.iter().position(|z| z.id == id) else {
            return Err("Unknown zoom keyframe".into());
        };
        let removed = next.zooms.remove(index);
        if removed.source == ZoomSource::Generated
            && !next.dismissed_zoom_ids.iter().any(|d| d == id)
        {
            next.dismissed_zoom_ids.push(id.to_string());
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Adds imported media to the project.
    pub fn add_assets(
        &mut self,
        expected_revision: u64,
        assets: Vec<Asset>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        if assets.is_empty() {
            return Err("Nothing to import".into());
        }
        let mut next = self.current.clone();
        next.assets.extend(assets);
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Removes media from the project with every clip of it (closing up where magnetic) and
    /// its zooms. Its files stay, so undo brings it back.
    pub fn remove_asset(
        &mut self,
        expected_revision: u64,
        asset_id: &str,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let mut next = self.current.clone();
        if next.asset(asset_id).is_none() {
            return Err("No such imported media".into());
        }
        let clips: Vec<String> = next
            .sequence
            .clips()
            .filter(|(_, c)| c.asset == asset_id)
            .map(|(_, c)| c.id.clone())
            .collect();
        if !clips.is_empty() {
            // Locked tracks keep their clips; the media stays while anything uses it.
            let mut unlocked = next.sequence.clone();
            for track in &mut unlocked.tracks {
                track.locked = false;
            }
            let (mut sequence, _) = crate::sequence::edit::apply(
                &unlocked,
                &next.assets,
                &SequenceEdit::Delete {
                    clip_ids: clips,
                    ripple: None,
                },
            )?;
            for (track, before) in sequence.tracks.iter_mut().zip(&next.sequence.tracks) {
                track.locked = before.locked;
            }
            next.sequence = sequence;
        }
        // Shorts edited on their own lose it too.
        for short in &mut next.shorts {
            if let Some(own) = &mut short.edit {
                for track in &mut own.sequence.tracks {
                    track.clips.retain(|clip| clip.asset != asset_id);
                }
            }
        }
        next.zooms.retain(|z| z.media.as_deref() != Some(asset_id));
        next.assets.retain(|asset| asset.id != asset_id);
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Sets what an asset's streams stand for (screen or camera; speech or background).
    pub fn set_stream_roles(
        &mut self,
        expected_revision: u64,
        asset_id: &str,
        roles: &[(String, Role)],
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let mut next = self.current.clone();
        let asset = next
            .assets
            .iter_mut()
            .find(|asset| asset.id == asset_id)
            .ok_or("No such imported media")?;
        for (stream, role) in roles {
            let stream = asset
                .streams
                .iter_mut()
                .find(|s| &s.id == stream)
                .ok_or("That media has no such stream")?;
            stream.role = *role;
        }
        if next == self.current {
            return Err("Nothing changed".into());
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    /// One timeline change made in short `short_id`'s own timeline. The short's first edit
    /// copies its stretch of the video into it; from then on it is edited on its own.
    pub fn edit_short_sequence(
        &mut self,
        expected_revision: u64,
        short_id: &str,
        edit: &SequenceEdit,
        persist_root: &Path,
    ) -> Result<EditOutcome, String> {
        self.check(expected_revision)?;
        let short = self
            .current
            .shorts
            .iter()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?;
        let timeline = crate::shorts::short_timeline(&self.current, short)?;
        let (sequence, outcome) =
            crate::sequence::edit::apply(&timeline.sequence, &self.current.assets, edit)?;
        let mut next = self.current.clone();
        let own = next
            .shorts
            .iter_mut()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?;
        own.edit = Some(crate::shorts::ShortEdit { sequence });
        self.commit_next(expected_revision, persist_root, next)?;
        Ok(outcome)
    }

    /// Lets a short follow the video again: its own edit is dropped.
    pub fn resync_short(
        &mut self,
        expected_revision: u64,
        short_id: &str,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let mut next = self.current.clone();
        let short = next
            .shorts
            .iter_mut()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?;
        if short.edit.take().is_none() {
            return Err("This short already follows the video".into());
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn undo(
        &mut self,
        expected_revision: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let mut previous = self.undo.last().cloned().ok_or("Nothing to undo")?;
        previous.revision = self
            .current
            .revision
            .checked_add(1)
            .ok_or("Revision overflow")?;
        persist_revision(persist_root, expected_revision, &previous)?;
        self.undo.pop();
        self.redo.push(self.current.clone());
        self.current = previous;
        Ok(&self.current)
    }

    pub fn redo(
        &mut self,
        expected_revision: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let mut next = self.redo.last().cloned().ok_or("Nothing to redo")?;
        next.revision = self
            .current
            .revision
            .checked_add(1)
            .ok_or("Revision overflow")?;
        persist_revision(persist_root, expected_revision, &next)?;
        self.redo.pop();
        self.undo.push(self.current.clone());
        self.current = next;
        Ok(&self.current)
    }
}

fn sort_zooms(zooms: &mut [ZoomKeyframe]) {
    zooms.sort_by(|a, b| {
        a.source_start_us
            .cmp(&b.source_start_us)
            .then(a.id.cmp(&b.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::edit::{Edge, SequenceEdit};
    use crate::sequence::tests::{recording, video};
    use crate::sequence::TrackKind;
    use tempfile::tempdir;

    const S: u64 = 1_000_000;

    /// A project made from a recording `len` long.
    pub(crate) fn doc(len: u64) -> EditDocument {
        EditDocument::from_recording(recording("rec", len, &[])).unwrap()
    }

    fn spans(history: &EditHistory, track: usize) -> Vec<(u64, u64, u64)> {
        history.current.sequence.tracks[track]
            .clips
            .iter()
            .map(|c| (c.start_us, c.in_us, c.duration_us))
            .collect()
    }

    #[test]
    fn failed_save_preserves_history_and_revision_ids_never_repeat() {
        let dir = tempdir().unwrap();
        let initial = doc(S);
        let mut history = EditHistory::new(initial.clone());
        fs::create_dir(dir.path().join("project.json")).unwrap();
        assert!(history
            .ripple_cuts(0, &[(100_000, 200_000)], dir.path())
            .is_err());
        assert_eq!(history.current, initial);
        assert!(!history.undo_available());
        fs::remove_dir(dir.path().join("project.json")).unwrap();
        history
            .ripple_cuts(0, &[(100_000, 200_000)], dir.path())
            .unwrap();
        history.undo(1, dir.path()).unwrap();
        history
            .ripple_cuts(2, &[(300_000, 400_000)], dir.path())
            .unwrap();
        assert_eq!(history.current.revision, 3);
        assert!(history.ripple_cuts(1, &[(0, 1)], dir.path()).is_err());
        let snapshot = history.current.clone();
        fs::remove_file(dir.path().join("project.json")).unwrap();
        fs::create_dir(dir.path().join("project.json")).unwrap();
        assert!(history.undo(3, dir.path()).is_err());
        assert_eq!(history.current, snapshot);
        assert!(history.undo_available());
        assert!(!history.redo_available());
    }

    #[test]
    fn cuts_ripple_every_track_and_persist() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history
            .ripple_cuts(0, &[(2 * S, 5 * S)], dir.path())
            .unwrap();
        assert_eq!(history.current.revision, 1);
        assert_eq!(history.current.duration_us(), 7 * S);
        for track in 0..4 {
            assert_eq!(
                spans(&history, track),
                vec![(0, 0, 2 * S), (2 * S, 5 * S, 5 * S)]
            );
        }
        assert!(history
            .ripple_cuts(0, &[(0, S)], dir.path())
            .unwrap_err()
            .contains("Stale"));
        history.undo(1, dir.path()).unwrap();
        assert_eq!(history.current.revision, 2);
        assert_eq!(history.current.duration_us(), 10 * S);
        history.redo(2, dir.path()).unwrap();
        let loaded = load_edit_document(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.revision, 3);
        assert_eq!(loaded.sequence, history.current.sequence);
        assert_eq!(loaded.assets, history.current.assets);
        // Cuts must be on the timeline and not overlap.
        assert!(history
            .ripple_cuts(3, &[(6 * S, 8 * S)], dir.path())
            .is_err());
        assert!(history
            .ripple_cuts(3, &[(0, 2 * S), (S, 3 * S)], dir.path())
            .is_err());
    }

    #[test]
    fn earlier_documents_are_not_read() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("project.json"),
            r#"{"schemaVersion":1,"revision":4,"retainedIntervals":[]}"#,
        )
        .unwrap();
        assert!(load_edit_document(dir.path()).is_err());
        // Saving over it starts the history again.
        let mut history = EditHistory::new(doc(S));
        history.ripple_cuts(0, &[(0, 100_000)], dir.path()).unwrap();
        assert_eq!(load_edit_document(dir.path()).unwrap().unwrap().revision, 1);
    }

    #[test]
    fn webcam_focus_maps_through_the_camera_clips_and_undoes() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history
            .ripple_cuts(0, &[(4 * S, 6 * S)], dir.path())
            .unwrap();
        // Timeline 3 s to 5 s spans the cut: source 3 s to 4 s and 6 s to 7 s.
        history
            .add_webcam_focus(1, 3 * S, 5 * S, dir.path())
            .unwrap();
        let focus = &history.current.webcam_focus;
        assert!(focus.enabled);
        assert_eq!(focus.media.as_deref(), Some("rec"));
        let sources = |focus: &WebcamFocus| {
            focus
                .segments
                .iter()
                .map(|s| (s.source_start_us, s.source_end_us))
                .collect::<Vec<_>>()
        };
        assert_eq!(sources(focus), vec![(3 * S, 4 * S), (6 * S, 7 * S)]);
        assert_eq!(
            focus.edited_ranges(&history.current.focus_clock()),
            vec![(3 * S, 5 * S)]
        );
        history
            .remove_webcam_focus(2, 4_500_000, 5 * S, dir.path())
            .unwrap();
        assert_eq!(
            sources(&history.current.webcam_focus),
            vec![(3 * S, 4 * S), (6 * S, 6_500_000)]
        );
        history.undo(3, dir.path()).unwrap();
        assert_eq!(
            sources(&history.current.webcam_focus),
            vec![(3 * S, 4 * S), (6 * S, 7 * S)]
        );
        // Without the camera on the timeline there is nothing to focus.
        let camera = history.current.sequence.tracks[1].clips[0].id.clone();
        history
            .edit_sequence(
                4,
                &SequenceEdit::Unlink {
                    clip_ids: vec![camera.clone()],
                },
                dir.path(),
            )
            .unwrap();
        let camera_track = history.current.sequence.tracks[1].clone();
        let all: Vec<String> = camera_track.clips.iter().map(|c| c.id.clone()).collect();
        history
            .edit_sequence(
                5,
                &SequenceEdit::Delete {
                    clip_ids: all,
                    ripple: Some(false),
                },
                dir.path(),
            )
            .unwrap();
        assert!(history.add_webcam_focus(6, 0, S, dir.path()).is_err());
    }

    #[test]
    fn chapters_are_undoable_and_saved() {
        use crate::chapters::Chapter;
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        let chapter = |id: &str, source_us: u64, title: &str| Chapter {
            id: id.into(),
            source_us,
            title: title.into(),
            media: Some("rec".into()),
            edited_us: Some(123),
        };
        history
            .set_chapters(
                0,
                vec![chapter("b", 5 * S, " Main "), chapter("a", 0, "Intro")],
                dir.path(),
            )
            .unwrap();
        let saved = &history.current.chapters;
        assert_eq!(saved[0].id, "a");
        assert_eq!(saved[1].title, "Main");
        assert!(saved.iter().all(|c| c.edited_us.is_none()));
        let json = fs::read_to_string(dir.path().join("project.json")).unwrap();
        assert!(json.contains("\"chapters\"") && !json.contains("editedUs"));
        let same = history.current.chapters.clone();
        history.set_chapters(1, same, dir.path()).unwrap();
        assert_eq!(history.current.revision, 1);
        assert!(history
            .set_chapters(1, vec![chapter("x", 0, "")], dir.path())
            .is_err());
        history.undo(1, dir.path()).unwrap();
        assert!(history.current.chapters.is_empty());
    }

    #[test]
    fn second_editor_cannot_overwrite_newer_disk_revision() {
        let dir = tempdir().unwrap();
        let initial = doc(S);
        let mut first = EditHistory::new(initial.clone());
        let mut second = EditHistory::new(initial.clone());
        first.ripple_cuts(0, &[(0, 100_000)], dir.path()).unwrap();
        assert!(second
            .ripple_cuts(0, &[(0, 200_000)], dir.path())
            .unwrap_err()
            .contains("Stale"));
        assert_eq!(second.current, initial);
        assert_eq!(
            load_edit_document(dir.path()).unwrap().unwrap().sequence,
            first.current.sequence
        );
    }

    fn suggestion(
        id: &str,
        start: u64,
        origin: crate::zoom::ZoomOrigin,
        scale: f64,
    ) -> ZoomSuggestion {
        ZoomSuggestion {
            path: Vec::new(),
            id: id.into(),
            source_start_us: start,
            source_end_us: start + 2 * S,
            center_x: 0.4,
            center_y: 0.4,
            scale,
            transition_us: 400_000,
            origin,
            contributing_event_seqs: vec![1],
            edited_ranges: Vec::new(),
            media: Some("rec".into()),
        }
    }

    #[test]
    fn zoom_settings_give_every_automatic_zoom_the_same_amount() {
        use crate::zoom::{ZoomOrigin, ZoomSettings};
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(20 * S));
        history
            .accept_zooms(
                0,
                &[
                    suggestion("a", S, ZoomOrigin::Click, 2.0),
                    suggestion("b", 6 * S, ZoomOrigin::Dwell, 1.5),
                    suggestion("c", 11 * S, ZoomOrigin::Cluster, 2.5),
                ],
                dir.path(),
            )
            .unwrap();
        let mut mine = history.current.zooms[2].clone();
        mine.scale = 3.0;
        history.update_zoom(1, mine, dir.path()).unwrap();
        let settings = ZoomSettings {
            click_scale: 1.6,
            hover_scale: 1.3,
            transition_ms: 900,
            ..ZoomSettings::default()
        };
        history
            .set_zoom_settings(2, settings.clone(), dir.path())
            .unwrap();
        let scales: Vec<f64> = history.current.zooms.iter().map(|z| z.scale).collect();
        assert_eq!(scales, vec![1.6, 1.3, 3.0]);
        assert_eq!(history.current.zooms[0].transition_us, 900_000);
        let loaded = load_edit_document(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.zoom_settings, settings);
    }

    #[test]
    fn zooms_follow_the_screen_clips_and_never_overlap() {
        use crate::zoom::ZoomOrigin;
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history
            .accept_zooms(
                0,
                &[suggestion("z", 3 * S, ZoomOrigin::Click, 2.0)],
                dir.path(),
            )
            .unwrap();
        assert_eq!(
            history.current.zoom_edited(&history.current.zooms[0]),
            vec![(3 * S, 5 * S)]
        );
        // Cutting 1 s before it moves it back.
        history.ripple_cuts(1, &[(0, S)], dir.path()).unwrap();
        assert_eq!(
            history.current.zooms_with_ranges()[0].edited_ranges,
            vec![crate::zoom::EditedRange {
                start_us: 2 * S,
                end_us: 4 * S
            }]
        );
        // A zoom by hand lands on the recording's clock and stops before the next one.
        history
            .add_manual_zoom(2, S, 3 * S, 0.5, 0.5, 2.0, dir.path())
            .unwrap();
        let manual = history
            .current
            .zooms
            .iter()
            .find(|z| z.source == ZoomSource::Manual)
            .unwrap();
        assert_eq!(manual.media.as_deref(), Some("rec"));
        assert_eq!(
            (manual.source_start_us, manual.source_end_us),
            (2 * S, 3 * S)
        );
        // Dismissed zooms do not come back; undo does.
        history.dismiss_zooms(3, &["z".into()], dir.path()).unwrap();
        assert!(history
            .accept_zooms(
                4,
                &[suggestion("z", 3 * S, ZoomOrigin::Click, 2.0)],
                dir.path()
            )
            .is_err());
        history.undo(4, dir.path()).unwrap();
        assert_eq!(history.current.zooms.len(), 2);
    }

    #[test]
    fn layout_and_audio_edits_undo_and_reopen() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(S));
        let mut layout = EditLayout::default();
        layout.aspect_ratio = "9:16".into();
        layout.padding_px = 24;
        history
            .update_layout(0, layout.clone(), dir.path())
            .unwrap();
        history.undo(1, dir.path()).unwrap();
        assert_eq!(history.current.layout.aspect_ratio, "16:9");
        history.redo(2, dir.path()).unwrap();
        let loaded = load_edit_document(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.layout.padding_px, 24);
        let mut bad = layout;
        bad.padding_px = 999;
        assert!(history.update_layout(3, bad, dir.path()).is_err());
        let audio = AudioSettings {
            normalize: true,
            ..Default::default()
        };
        history.update_audio(3, audio.clone(), dir.path()).unwrap();
        assert_eq!(
            load_edit_document(dir.path()).unwrap().unwrap().audio,
            audio
        );
        let bad = AudioSettings {
            target_lufs: 0.0,
            ..audio
        };
        assert!(history.update_audio(4, bad, dir.path()).is_err());
    }

    /// Media that is imported, placed, and removed again: its clips go with it and undo
    /// brings everything back.
    #[test]
    fn removing_media_takes_its_clips_and_undoes() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history
            .add_assets(0, vec![video("m1", 3 * S)], dir.path())
            .unwrap();
        let v1 = history.current.sequence.tracks[0].id.clone();
        history
            .edit_sequence(
                1,
                &SequenceEdit::PlaceAsset {
                    asset_id: "m1".into(),
                    at_us: 4 * S,
                    track_id: Some(v1),
                    streams: vec![],
                    range: None,
                },
                dir.path(),
            )
            .unwrap();
        assert_eq!(history.current.duration_us(), 13 * S);
        // Its sound maps through its own clip.
        let clock = history.current.transcript_clock("m1.sound0").unwrap();
        assert_eq!(clock.source_to_edited_us(S), Some(5 * S));
        history.remove_asset(2, "m1", dir.path()).unwrap();
        assert!(history.current.asset("m1").is_none());
        assert_eq!(history.current.duration_us(), 10 * S, "magnetic closes up");
        assert!(history
            .current
            .sequence
            .clips()
            .all(|(_, c)| c.asset == "rec"));
        assert!(history.current.transcript_clock("m1.sound0").is_err());
        history.undo(3, dir.path()).unwrap();
        assert_eq!(history.current.duration_us(), 13 * S);
    }

    /// A transcript's words follow its sound's clips through cuts, moves and an unlink.
    #[test]
    fn transcript_clocks_follow_sound_clips() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        let clock = |h: &EditHistory| h.current.transcript_clock("rec.mic").unwrap();
        assert_eq!(clock(&history).source_to_edited_us(6 * S), Some(6 * S));
        history.ripple_cuts(0, &[(S, 2 * S)], dir.path()).unwrap();
        assert_eq!(clock(&history).source_to_edited_us(6 * S), Some(5 * S));
        assert_eq!(clock(&history).source_to_edited_us(1_500_000), None, "cut");
        // Slide the microphone alone 1 s later (without magnetic: leaves a gap).
        let mic = history.current.sequence.tracks[2].clips[1].id.clone();
        history
            .edit_sequence(
                1,
                &SequenceEdit::Unlink {
                    clip_ids: vec![mic.clone()],
                },
                dir.path(),
            )
            .unwrap();
        history
            .edit_sequence(
                2,
                &SequenceEdit::SetMagnetic { magnetic: false },
                dir.path(),
            )
            .unwrap();
        history
            .edit_sequence(
                3,
                &SequenceEdit::MoveClips {
                    clip_ids: vec![mic],
                    delta_us: S as i64,
                    track_id: None,
                    anchor_id: None,
                },
                dir.path(),
            )
            .unwrap();
        assert_eq!(clock(&history).source_to_edited_us(6 * S), Some(6 * S));
        assert_eq!(
            history
                .current
                .picture_clock("rec")
                .source_to_edited_us(6 * S),
            Some(5 * S),
            "the screen stays"
        );
        assert!(history.current.transcript_clock("rec.nope").is_err());
    }

    #[test]
    fn sequence_edits_are_undoable_and_restoring_a_cut_ripples() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history
            .ripple_cuts(0, &[(2 * S, 4 * S)], dir.path())
            .unwrap();
        let first = history.current.sequence.tracks[0].clips[0].id.clone();
        // Restore the 2 s: the first set grows back and everything after moves along.
        history
            .edit_sequence(
                1,
                &SequenceEdit::TrimClip {
                    clip_id: first,
                    edge: Edge::End,
                    to_us: 4 * S,
                    ripple: Some(true),
                },
                dir.path(),
            )
            .unwrap();
        assert_eq!(history.current.duration_us(), 10 * S);
        assert_eq!(
            spans(&history, 2),
            vec![(0, 0, 4 * S), (4 * S, 4 * S, 6 * S)]
        );
        history
            .edit_sequence(
                2,
                &SequenceEdit::AddTrack {
                    track_kind: TrackKind::Audio,
                },
                dir.path(),
            )
            .unwrap();
        assert_eq!(history.current.sequence.audio_tracks().count(), 3);
        history.undo(3, dir.path()).unwrap();
        history.undo(4, dir.path()).unwrap();
        assert_eq!(history.current.duration_us(), 8 * S);
    }
}
