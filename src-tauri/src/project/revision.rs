//! Versioned edit document (`project.json`). Source media is never rewritten.
use super::checkpoint::Restored;
use super::history::{describe, Checkpoint, History, RunUndo, StepMeta};
use super::layout::validate_layout;
use super::reader::{open_regular, safe_path};
use crate::layouts::LayoutTrack;
use crate::sequence::edit::{EditOutcome, SequenceEdit};
use crate::sequence::{clock, Asset, Role, Sequence, SourceRange, StreamRef};
use crate::timeline::TimelineMapper;
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
pub const MAX_CUTS_PER_REVISION: usize = 256;
/// Muted moments one sound can hold.
pub const MAX_NOISE_MUTES: usize = 50_000;

/// Muted moments by sound: see [`EditDocument::noise_mutes`].
pub type NoiseMutes = std::collections::BTreeMap<String, Vec<SourceRange>>;

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
    /// The video's base layout and the sections that change it.
    #[serde(default, skip_serializing_if = "LayoutTrack::is_default")]
    pub layouts: LayoutTrack,
    /// Loudness, noise reduction and ducking. Applies to playback and export.
    #[serde(default, skip_serializing_if = "AudioSettings::is_default")]
    pub audio: AudioSettings,
    /// Moments muted in a sound (mouth noises, knocks), by stream key (`<asset>.<stream>`), in
    /// that sound's own time: wherever a clip plays it, those moments are silent, and pause
    /// detection hears them as silence. Merged and in order.
    #[serde(default, skip_serializing_if = "NoiseMutes::is_empty")]
    pub noise_mutes: NoiseMutes,
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
    /// The style it is dressed in: graphics, captions and the frame (see `crate::style`).
    #[serde(default = "default_style")]
    pub style: String,
    /// The preset it follows: what its settings are unless changed here (see `crate::preset`;
    /// stored, only the changes are written).
    #[serde(default = "default_preset")]
    pub preset: String,
    /// The preset its new shorts start from.
    #[serde(default = "default_shorts_preset")]
    pub shorts_preset: String,
    /// What the cleanup passes start from (silence and gap levels, mouth-noise sensitivity).
    #[serde(default)]
    pub cleanup: crate::preset::settings::CleanupSettings,
    /// What the Export dialog starts with.
    #[serde(default)]
    pub export: crate::preset::settings::ExportPreferences,
}

fn default_style() -> String {
    crate::style::DEFAULT.into()
}

fn default_preset() -> String {
    crate::preset::DEFAULT.into()
}

fn default_shorts_preset() -> String {
    crate::preset::SHORTS.into()
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
            layouts: LayoutTrack::default(),
            audio: AudioSettings::default(),
            noise_mutes: NoiseMutes::new(),
            captions: CaptionSettings::default(),
            chapters: Vec::new(),
            shorts: Vec::new(),
            short_layout: None,
            zoom_settings: Default::default(),
            style: default_style(),
            preset: default_preset(),
            shorts_preset: default_shorts_preset(),
            cleanup: Default::default(),
            export: Default::default(),
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

    /// Every track id the document knows: the video's tracks, the shorts' own and those
    /// with mix settings (a deleted track's stay until undo has no use for them).
    pub fn track_ids_in_use(&self) -> impl Iterator<Item = &str> {
        let shorts = self
            .shorts
            .iter()
            .filter_map(|s| s.edit.as_ref())
            .flat_map(|own| own.sequence.tracks.iter());
        self.sequence
            .tracks
            .iter()
            .chain(shorts)
            .map(|t| t.id.as_str())
            .chain(self.audio.tracks.keys().map(String::as_str))
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

    /// The clock something anchored on `media` (`None`: the first recording) plays on.
    pub fn anchor_clock(&self, media: Option<&str>) -> Option<TimelineMapper> {
        self.clock_asset(media).map(|asset| self.asset_clock(asset))
    }

    /// The layouts with where each section plays filled in, for the UI.
    pub fn layouts_with_ranges(&self) -> LayoutTrack {
        let mut track = self.layouts.clone();
        track.attach_edited_ranges(|media| self.anchor_clock(media));
        track
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
        let clocks = self.zoom_clocks();
        let edited = |z: &ZoomKeyframe| {
            clocks(z.media.as_deref())
                .map(|m| m.source_range_to_edited(z.source_start_us, z.source_end_us))
                .unwrap_or_default()
        };
        let mine = edited(zoom);
        self.zooms.iter().filter(|z| z.id != zoom.id).any(|other| {
            let theirs = edited(other);
            mine.iter()
                .any(|&(a, b)| theirs.iter().any(|&(c, d)| a < d && c < b))
        })
    }

    /// The zooms with where each lands on the timeline, each on its own clock.
    pub fn zooms_with_ranges(&self) -> Vec<ZoomKeyframe> {
        let mut zooms = self.zooms.clone();
        let clocks = self.zoom_clocks();
        crate::zoom::attach_zoom_edited_ranges_with(&mut zooms, &|media| clocks(media));
        zooms
    }

    /// Zoom clocks by recording, each built once: building one walks the whole sequence,
    /// and there is a zoom every few seconds.
    fn zoom_clocks(&self) -> impl Fn(Option<&str>) -> Option<std::rc::Rc<TimelineMapper>> + '_ {
        let built = std::cell::RefCell::new(std::collections::HashMap::<
            String,
            std::rc::Rc<TimelineMapper>,
        >::new());
        move |media| {
            let asset = self.clock_asset(media)?;
            let mut built = built.borrow_mut();
            Some(
                built
                    .entry(asset.to_string())
                    .or_insert_with(|| std::rc::Rc::new(self.picture_clock(asset)))
                    .clone(),
            )
        }
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
        self.layouts.validate()?;
        self.audio.validate()?;
        self.captions.validate()?;
        crate::chapters::validate(&self.chapters)?;
        crate::shorts::validate(&self.shorts)?;
        crate::shorts::validate_edits(self)?;
        if !crate::style::is_builtin(&self.style) {
            crate::style::store::check_name(&self.style)?;
        }
        self.cleanup.validate()?;
        self.export.validate()?;
        for preset in [&self.preset, &self.shorts_preset] {
            if !crate::preset::is_builtin(preset) {
                crate::preset::store::check_name(preset)?;
            }
        }
        Ok(())
    }
}

/// Moves a `project.json` the editor could not read (made by an earlier AeroEdits, or
/// damaged) out of the way, to `project.unreadable.json` (numbered when that is taken), so the
/// next save cannot overwrite it. Nothing reads it again; old edits are not migrated. Returns
/// the name it was given.
pub fn set_aside_edit_document(root: &Path) -> Result<String, String> {
    let path = safe_path(root, "project.json")?;
    for n in 1..1000 {
        let name = if n == 1 {
            "project.unreadable.json".to_string()
        } else {
            format!("project.unreadable-{n}.json")
        };
        let target = safe_path(root, &name)?;
        if fs::symlink_metadata(&target).is_ok() {
            continue;
        }
        fs::rename(&path, &target).map_err(|e| e.to_string())?;
        return Ok(name);
    }
    Err("There are too many set-aside edit documents".into())
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
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid edit document: {e}"))?;
    let document = crate::preset::read(value)?;
    document.validate()?;
    Ok(Some(document))
}

/// `document` as saved: what the UI is shown (where things land, missing files) is worked
/// out, not stored.
pub(super) fn stored_form(document: &EditDocument) -> EditDocument {
    let mut stored = document.clone();
    for zoom in &mut stored.zooms {
        zoom.edited_ranges.clear();
    }
    for asset in &mut stored.assets {
        asset.missing = false;
    }
    stored
}

pub fn save_edit_document(root: &Path, document: &EditDocument) -> Result<(), String> {
    document.validate()?;
    let path = safe_path(root, "project.json")?;
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if meta.file_type().is_symlink() {
            return Err("Edit document cannot be a symlink".into());
        }
    }
    // Only what differs from its preset is written.
    let stored = crate::preset::stored(&stored_form(document))?;
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

/// The current edit and its undo history, which lives on disk (see `history.rs`).
#[derive(Clone, Debug)]
pub struct EditHistory {
    pub current: EditDocument,
    log: History,
    /// What the next step is, set by whoever is about to make it.
    next_step: Option<StepMeta>,
}

impl EditHistory {
    /// A history that starts with `current`.
    pub fn new(current: EditDocument) -> Self {
        Self {
            current,
            log: History::empty(),
            next_step: None,
        }
    }

    /// `current` with the history saved in the project at `root`, and a note when that
    /// history had to be repaired.
    pub fn open(root: &Path, current: EditDocument) -> (Self, Option<String>) {
        let (log, note) = History::open(root, &current);
        (
            Self {
                current,
                log,
                next_step: None,
            },
            note,
        )
    }

    pub fn undo_available(&self) -> bool {
        self.log.undo_step().is_some()
    }

    pub fn redo_available(&self) -> bool {
        self.log.redo_step().is_some()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.log.undo_step().map(|s| s.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.log.redo_step().map(|s| s.label.as_str())
    }

    pub fn steps(&self) -> &History {
        &self.log
    }

    pub fn steps_mut(&mut self) -> &mut History {
        &mut self.log
    }

    /// Names the next step this history makes; without it a step is called "Edit".
    pub fn set_next_step(&mut self, meta: StepMeta) {
        self.next_step = Some(meta);
    }

    pub fn clear_next_step(&mut self) {
        self.next_step = None;
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
        // Taken first, so a failed commit does not name the next one.
        let meta = self.next_step.take();
        self.check(expected_revision)?;
        next.schema_version = EDIT_SCHEMA_VERSION;
        crate::shorts::give_own_timelines(&mut next);
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
        // Both states are on disk before the edit is, so the step can always be undone.
        let before = self.log.current_state(persist_root, &self.current)?;
        let after = self.log.save_state(persist_root, &next)?;
        persist_revision(persist_root, expected_revision, &next)?;
        let meta = meta.unwrap_or_else(StepMeta::for_this_thread);
        let was = &self.current;
        self.log
            .record_commit(persist_root, before, after, meta, next.revision, || {
                describe(was, &next)
            });
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
        let (sequence, outcome) = crate::sequence::edit::apply_reserving(
            &self.current.sequence,
            &self.current.assets,
            edit,
            self.current.track_ids_in_use(),
        )?;
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

    /// Changes the layouts of the video (`short` none) or of a short, in one step: `change`
    /// gets that timeline (the short's own, in its time) and its layouts.
    pub fn change_layouts(
        &mut self,
        expected_revision: u64,
        short: Option<&str>,
        change: impl FnOnce(&EditDocument, &mut LayoutTrack) -> Result<(), String>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let mut next = self.current.clone();
        let track = match short {
            None => {
                let mut track = next.layouts.clone();
                change(&self.current, &mut track)?;
                next.layouts = track.normalized();
                &next.layouts
            }
            Some(id) => {
                let index = next
                    .shorts
                    .iter()
                    .position(|s| s.id == id)
                    .ok_or("That short no longer exists")?;
                let timeline = crate::shorts::short_timeline(&self.current, &next.shorts[index])?;
                let mut track = next.shorts[index].layouts.clone();
                change(&timeline, &mut track)?;
                next.shorts[index].layouts = track.normalized();
                &next.shorts[index].layouts
            }
        };
        track.validate()?;
        if next == self.current {
            return Ok(&self.current);
        }
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

    /// Adds a short made from a take, giving the take's streams their roles (camera, speech)
    /// in the same step.
    pub fn add_take_short(
        &mut self,
        expected_revision: u64,
        short: crate::shorts::Short,
        roles: &[(String, String, Role)],
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let mut next = self.current.clone();
        for (asset, stream, role) in roles {
            let stream = next
                .assets
                .iter_mut()
                .find(|a| &a.id == asset)
                .and_then(|a| a.streams.iter_mut().find(|s| &s.id == stream))
                .ok_or("The take's media is no longer in the project")?;
            stream.role = *role;
        }
        let mut shorts = next.shorts.clone();
        shorts.push(short);
        crate::shorts::validate(&shorts)?;
        next.shorts = crate::shorts::normalized(shorts);
        self.commit_next(expected_revision, persist_root, next)
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

    /// Mutes (`mute`) or unmutes (`unmute` set) moments of sound `key`'s own time.
    pub fn update_noise_mutes(
        &mut self,
        expected_revision: u64,
        key: &str,
        ranges: &[SourceRange],
        unmute: bool,
        short: Option<&str>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let source = self.current.stream_ref(key)?;
        let asset = self
            .current
            .asset(&source.asset)
            .ok_or("That sound is not in the project")?;
        if asset
            .stream(&source.stream)
            .is_none_or(|s| s.kind != crate::sequence::StreamKind::Sound)
        {
            return Err("That stream has no sound".into());
        }
        if ranges
            .iter()
            .any(|r| r.end_us <= r.start_us || r.end_us > asset.duration_us)
        {
            return Err("A muted moment must lie inside its sound".into());
        }
        // A short's mutes are its own; without one, the video's.
        let own_mutes = |document: &EditDocument| -> Result<NoiseMutes, String> {
            match short {
                None => Ok(document.noise_mutes.clone()),
                Some(id) => document
                    .shorts
                    .iter()
                    .find(|s| s.id == id)
                    .ok_or("That short no longer exists")?
                    .edit
                    .as_ref()
                    .map(|e| e.noise_mutes.clone())
                    .ok_or_else(|| {
                        "Part of this short was cut from the video; adjust it first".into()
                    }),
            }
        };
        let mut mutes = own_mutes(&self.current)?;
        let current: Vec<(u64, u64)> = mutes
            .get(key)
            .map(|ranges| ranges.iter().map(|r| (r.start_us, r.end_us)).collect())
            .unwrap_or_default();
        let changed: Vec<(u64, u64)> = ranges.iter().map(|r| (r.start_us, r.end_us)).collect();
        let next_ranges = if unmute {
            crate::transcript::pauses::subtract_ranges(&current, &changed)
        } else {
            crate::transcript::pauses::merge_ranges([current, changed].concat())
        };
        if next_ranges.len() > MAX_NOISE_MUTES {
            return Err(format!("Mute at most {MAX_NOISE_MUTES} moments of a sound"));
        }
        let before = mutes.clone();
        if next_ranges.is_empty() {
            mutes.remove(key);
        } else {
            mutes.insert(
                key.to_string(),
                next_ranges
                    .into_iter()
                    .map(|(start_us, end_us)| SourceRange { start_us, end_us })
                    .collect(),
            );
        }
        if mutes == before {
            return Err(if unmute {
                "Those moments are not muted".into()
            } else {
                "Those moments are already muted".into()
            });
        }
        let mut next = self.current.clone();
        match short {
            None => next.noise_mutes = mutes,
            Some(id) => {
                if let Some(edit) = next
                    .shorts
                    .iter_mut()
                    .find(|s| s.id == id)
                    .and_then(|s| s.edit.as_mut())
                {
                    edit.noise_mutes = mutes;
                }
            }
        }
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
        if let Some(key) = &captions.track_id {
            let sound = crate::sequence::StreamRef::parse(key).and_then(|s| {
                self.current
                    .assets
                    .iter()
                    .find(|a| a.id == s.asset)?
                    .stream(&s.stream)
            });
            if sound.map(|s| s.kind) != Some(crate::sequence::StreamKind::Sound) {
                return Err(format!("{key} is not a sound in this project"));
            }
        }
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
        with_zoom_settings(&mut next, settings);
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
        let (sequence, outcome) = crate::sequence::edit::apply_reserving(
            &timeline.sequence,
            &self.current.assets,
            edit,
            self.current.track_ids_in_use(),
        )?;
        let mut next = self.current.clone();
        let own = next
            .shorts
            .iter_mut()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?;
        let noise_mutes = own.edit.take().map(|e| e.noise_mutes).unwrap_or_default();
        own.edit = Some(crate::shorts::ShortEdit {
            sequence,
            noise_mutes,
        });
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
        if short.take.is_some() {
            return Err(
                "This short is made from a take: it has no stretch of the video to follow".into(),
            );
        }
        // Its stretch of the video as edited now; its own muted noises stay.
        let noise_mutes = short.edit.take().map(|e| e.noise_mutes).unwrap_or_default();
        let (start, end) = crate::shorts::edited_range_in(short, &self.current)
            .ok_or("Part of this short was cut from the video; adjust it first")?;
        short.edit = Some(crate::shorts::ShortEdit {
            sequence: crate::shorts::slice_sequence(
                &self.current.sequence,
                &self.current.assets,
                start,
                end,
            ),
            noise_mutes,
        });
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Ends the agents' current run (they began a new workflow).
    pub fn end_run(&mut self) {
        self.log.end_run();
    }

    /// Takes back everything the newest agent run did, as one new step, while nothing came
    /// after it. Undo then brings the run back.
    pub fn undo_run(
        &mut self,
        expected_revision: u64,
        persist_root: &Path,
    ) -> Result<RunUndo, String> {
        self.check(expected_revision)?;
        let run = self.log.run_to_undo()?;
        let previous = self.log.load_state(persist_root, &run.before)?;
        let mut meta = StepMeta::for_this_thread();
        meta.joins_run = false;
        meta.undoes_run = Some(run.run.clone());
        meta.label = Some(format!(
            "Undo {}'s run ({} change{})",
            run.agent,
            run.changes,
            if run.changes == 1 { "" } else { "s" }
        ));
        self.next_step = Some(meta);
        self.commit_next(expected_revision, persist_root, previous)?;
        Ok(run)
    }

    /// Saves a checkpoint of the edit, the graphics and the caption marks as they are now. An
    /// automatic one the same as the newest automatic one is not saved again.
    pub fn save_checkpoint(
        &mut self,
        persist_root: &Path,
        name: &str,
        auto: bool,
    ) -> Result<Checkpoint, String> {
        let state = self.log.current_state(persist_root, &self.current)?;
        let meta = StepMeta::for_this_thread();
        let checkpoint =
            super::checkpoint::capture(persist_root, &mut self.log, state, name, &meta, auto)?;
        if auto {
            let newest = self.log.checkpoints().iter().rev().find(|c| c.auto);
            if let Some(newest) = newest.filter(|c| c.same_as(&checkpoint)) {
                return Ok(newest.clone());
            }
        }
        Ok(self.log.add_checkpoint(persist_root, checkpoint))
    }

    /// Goes back to checkpoint `id`: its edit as one new step (Undo takes it back), then its
    /// graphic files and caption marks. What is there now is saved first as an automatic
    /// checkpoint, since Undo brings back only the edit.
    pub fn restore_checkpoint(
        &mut self,
        expected_revision: u64,
        persist_root: &Path,
        id: &str,
    ) -> Result<(Checkpoint, Restored), String> {
        self.check(expected_revision)?;
        let checkpoint = self.log.checkpoint(id)?.clone();
        let document = self.log.load_state(persist_root, &checkpoint.state)?;
        let now = self.save_checkpoint(
            persist_root,
            &format!("Before restoring \"{}\"", checkpoint.name),
            true,
        )?;
        if now.state != checkpoint.state {
            let mut meta = StepMeta::for_this_thread();
            meta.joins_run = false;
            meta.label = Some(format!("Restore \"{}\"", checkpoint.name));
            self.next_step = Some(meta);
            self.commit_next(expected_revision, persist_root, document)?;
        }
        let restored = super::checkpoint::restore(persist_root, &self.log, &checkpoint);
        Ok((checkpoint, restored))
    }

    pub fn rename_checkpoint(
        &mut self,
        persist_root: &Path,
        id: &str,
        name: &str,
    ) -> Result<(), String> {
        self.log.rename_checkpoint(persist_root, id, name)
    }

    pub fn delete_checkpoint(&mut self, persist_root: &Path, id: &str) -> Result<(), String> {
        self.log.delete_checkpoint(persist_root, id)
    }

    pub fn undo(
        &mut self,
        expected_revision: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let step = self.log.undo_step().ok_or("Nothing to undo")?;
        let mut previous = self.log.load_state(persist_root, &step.before)?;
        previous.revision = self
            .current
            .revision
            .checked_add(1)
            .ok_or("Revision overflow")?;
        persist_revision(persist_root, expected_revision, &previous)?;
        self.log.record_undo(persist_root);
        self.current = previous;
        Ok(&self.current)
    }

    pub fn redo(
        &mut self,
        expected_revision: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        self.check(expected_revision)?;
        let step = self.log.redo_step().ok_or("Nothing to redo")?;
        let mut next = self.log.load_state(persist_root, &step.after)?;
        next.revision = self
            .current
            .revision
            .checked_add(1)
            .ok_or("Revision overflow")?;
        persist_revision(persist_root, expected_revision, &next)?;
        self.log.record_redo(persist_root);
        self.current = next;
        Ok(&self.current)
    }
}

/// New auto-zoom settings on `document`: automatic zooms take the new amounts and transition.
fn with_zoom_settings(document: &mut EditDocument, settings: crate::zoom::ZoomSettings) {
    let transition = settings.transition_ms as u64 * 1_000;
    for zoom in document
        .zooms
        .iter_mut()
        .filter(|z| z.source == ZoomSource::Generated)
    {
        zoom.scale = settings.scale_for(zoom.origin);
        let length = zoom.source_end_us - zoom.source_start_us;
        zoom.transition_us = transition.min(length.saturating_sub(1) / 2).max(1);
    }
    document.zoom_settings = settings;
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
    fn an_unreadable_edit_is_set_aside_before_anything_saves_over_it() {
        let dir = tempdir().unwrap();
        let root = crate::project::folder::create_project_folder(dir.path(), "Old", None).unwrap();
        let original = r#"{"schemaVersion":1,"revision":42,"retainedIntervals":[]}"#;
        fs::write(root.join("project.json"), original).unwrap();
        let mut reader = crate::project::ProjectReader::open(&root).unwrap();
        assert!(reader
            .summary
            .diagnostics
            .iter()
            .any(|d| d.contains("project.unreadable.json")));
        reader
            .edit_sequence(
                0,
                &SequenceEdit::AddTrack {
                    track_kind: TrackKind::Video,
                },
                None,
            )
            .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("project.unreadable.json")).unwrap(),
            original
        );
        assert_eq!(load_edit_document(&root).unwrap().unwrap().revision, 1);
        drop(reader);
        // A second one is kept beside the first.
        fs::write(root.join("project.json"), "not json").unwrap();
        crate::project::ProjectReader::open(&root).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("project.unreadable-2.json")).unwrap(),
            "not json"
        );
    }

    #[test]
    fn a_new_track_never_inherits_a_deleted_tracks_mix() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(S));
        let a2 = history.current.sequence.tracks[3].id.clone();
        let mut audio = history.current.audio.clone();
        audio.tracks.insert(
            a2.clone(),
            crate::project::audio::TrackMix {
                volume_db: -12.0,
                denoise_db: Some(30.0),
                duck_db: None,
            },
        );
        history.update_audio(0, audio, dir.path()).unwrap();
        history
            .edit_sequence(
                1,
                &SequenceEdit::RemoveTrack {
                    track_id: a2.clone(),
                },
                dir.path(),
            )
            .unwrap();
        history
            .edit_sequence(
                2,
                &SequenceEdit::AddTrack {
                    track_kind: TrackKind::Audio,
                },
                dir.path(),
            )
            .unwrap();
        let added = history.current.sequence.tracks.last().unwrap().id.clone();
        assert_ne!(added, a2);
        assert_eq!(history.current.audio.track_gain(&added), 1.0);
        // Undo brings the old track back with its mix.
        history.undo(3, dir.path()).unwrap();
        history.undo(4, dir.path()).unwrap();
        assert!(history.current.audio.track_gain(&a2) < 1.0);
    }

    #[test]
    fn layout_sections_map_through_cuts_and_undo() {
        use crate::layouts::{CameraOnly, Layout, LayoutSection};
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history
            .ripple_cuts(0, &[(4 * S, 6 * S)], dir.path())
            .unwrap();
        let camera = Layout::CameraOnly(CameraOnly::default());
        // Timeline 3 s to 5 s spans the cut: one section, 3 s to 7 s of the recording.
        history
            .change_layouts(
                1,
                None,
                |timeline, track| {
                    let template = LayoutSection::new(track.new_id(), None, 0, 1, Some(camera));
                    track.add(crate::layouts::sections_on(
                        timeline,
                        3 * S,
                        5 * S,
                        &template,
                    )?);
                    Ok(())
                },
                dir.path(),
            )
            .unwrap();
        let sources = |track: &LayoutTrack| {
            track
                .sections
                .iter()
                .map(|s| (s.media.clone(), s.source_start_us, s.source_end_us))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            sources(&history.current.layouts),
            vec![(Some("rec".to_string()), 3 * S, 7 * S)]
        );
        let shown = history.current.layouts_with_ranges();
        assert_eq!(
            shown.sections[0].edited_ranges,
            vec![crate::zoom::EditedRange {
                start_us: 3 * S,
                end_us: 5 * S
            }]
        );
        // Too short, or nothing there: refused, and nothing changes.
        assert!(history
            .change_layouts(
                2,
                None,
                |timeline, track| {
                    let template = LayoutSection::new(track.new_id(), None, 0, 1, None);
                    track.add(crate::layouts::sections_on(timeline, 0, S / 2, &template)?);
                    Ok(())
                },
                dir.path(),
            )
            .is_err());
        history.undo(2, dir.path()).unwrap();
        assert!(history.current.layouts.sections.is_empty());
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
            score: 0.0,
            kept: Vec::new(),
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
            mouth_clicks: true,
            mouth_click_strength: 60,
            ..Default::default()
        };
        history.update_audio(3, audio.clone(), dir.path()).unwrap();
        assert_eq!(
            load_edit_document(dir.path()).unwrap().unwrap().audio,
            audio
        );
        history.undo(4, dir.path()).unwrap();
        assert!(!history.current.audio.mouth_clicks);
        history.redo(5, dir.path()).unwrap();
        assert!(history.current.audio.mouth_clicks);
        assert_eq!(history.current.audio.mouth_click_strength, 60);
        let bad = AudioSettings {
            target_lufs: 0.0,
            ..audio
        };
        assert!(history.update_audio(6, bad, dir.path()).is_err());
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

    fn reopen(root: &Path) -> (EditHistory, Option<String>) {
        EditHistory::open(root, load_edit_document(root).unwrap().unwrap())
    }

    fn states(root: &Path) -> usize {
        fs::read_dir(root.join("history/states")).map_or(0, |d| d.count())
    }

    #[test]
    fn undo_and_redo_survive_reopening() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history.ripple_cuts(0, &[(S, 2 * S)], dir.path()).unwrap();
        history.ripple_cuts(1, &[(S, 3 * S)], dir.path()).unwrap();
        history.undo(2, dir.path()).unwrap();
        drop(history);

        let (mut history, note) = reopen(dir.path());
        assert_eq!(note, None);
        assert_eq!(history.current.duration_us(), 9 * S);
        assert_eq!(history.undo_label(), Some("Cut 1.0 s"));
        assert_eq!(history.redo_label(), Some("Cut 2.0 s"));
        history.redo(3, dir.path()).unwrap();
        assert_eq!(history.current.duration_us(), 7 * S);
        history.undo(4, dir.path()).unwrap();
        history.undo(5, dir.path()).unwrap();
        assert_eq!(history.current.duration_us(), 10 * S);
        assert!(!history.undo_available());
        // Going back and forth saves nothing new: three states, one per edit.
        assert_eq!(states(dir.path()), 3);
    }

    #[test]
    fn more_steps_than_the_old_stack_come_back_after_reopening() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        for revision in 0..100 {
            history
                .ripple_cuts(revision, &[(0, 10_000)], dir.path())
                .unwrap();
        }
        assert_eq!(history.current.duration_us(), 9 * S);
        drop(history);
        let (mut history, _) = reopen(dir.path());
        assert_eq!(history.steps().undo_len(), 100);
        for revision in 100..200 {
            history.undo(revision, dir.path()).unwrap();
        }
        assert_eq!(history.current.duration_us(), 10 * S);
        assert!(history.undo(200, dir.path()).is_err());
    }

    #[test]
    fn an_edit_saved_without_its_log_line_becomes_a_step() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history.ripple_cuts(0, &[(S, 2 * S)], dir.path()).unwrap();
        // As if the app stopped after saving project.json but before writing the log.
        let mut changed = history.current.clone();
        changed.revision = 2;
        changed.chapters = vec![crate::chapters::Chapter {
            id: "c-1".into(),
            source_us: 0,
            title: "Intro".into(),
            media: None,
            edited_us: None,
        }];
        save_edit_document(dir.path(), &changed).unwrap();
        drop(history);

        let (mut history, note) = reopen(dir.path());
        assert!(note.unwrap().contains("kept as an undo step"));
        assert_eq!(history.undo_label(), Some("Change made outside AeroEdits"));
        history.undo(2, dir.path()).unwrap();
        assert!(history.current.chapters.is_empty());
        assert_eq!(history.current.duration_us(), 9 * S);
        history.undo(3, dir.path()).unwrap();
        assert_eq!(history.current.duration_us(), 10 * S);
    }

    #[test]
    fn a_log_line_cut_short_is_ignored_and_a_broken_log_set_aside() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history.ripple_cuts(0, &[(S, 2 * S)], dir.path()).unwrap();
        history.ripple_cuts(1, &[(S, 2 * S)], dir.path()).unwrap();
        drop(history);
        let log = dir.path().join("history/log.jsonl");
        let mut text = fs::read_to_string(&log).unwrap();
        text.push_str("{\"op\":\"und");
        fs::write(&log, &text).unwrap();
        let (history, note) = reopen(dir.path());
        assert_eq!(note, None);
        assert_eq!(history.steps().undo_len(), 2);

        fs::write(
            &log,
            "not json
{\"op\":\"undo\"}
",
        )
        .unwrap();
        let (history, note) = reopen(dir.path());
        assert!(note.unwrap().contains("could not be read"));
        assert!(!history.undo_available());
        assert!(dir.path().join("history/log.unreadable-1.jsonl").is_file());
    }

    #[test]
    fn the_oldest_steps_go_when_history_is_over_its_limit() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history.ripple_cuts(0, &[(0, 10_000)], dir.path()).unwrap();
        let one_state = history.steps().stored_bytes() / 2;
        history.steps_mut().set_limit_bytes(one_state * 4);
        for revision in 1..20 {
            history
                .ripple_cuts(revision, &[(0, 10_000)], dir.path())
                .unwrap();
        }
        let kept = history.steps().undo_len();
        assert!((2..=4).contains(&kept), "{kept} steps kept");
        assert_eq!(states(dir.path()), kept + 1);
        drop(history);
        let (mut history, _) = reopen(dir.path());
        assert_eq!(history.steps().undo_len(), kept);
        for revision in 20..20 + kept as u64 {
            history.undo(revision, dir.path()).unwrap();
        }
        assert_eq!(
            history.current.duration_us(),
            10 * S - (20 - kept as u64) * 10_000
        );
    }

    #[test]
    fn a_step_made_in_an_agent_call_is_the_agents() {
        use crate::project::history::{as_agent, Author};
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        as_agent("claude-code", "silence", || {
            history.ripple_cuts(0, &[(S, 4 * S)], dir.path()).unwrap();
        });
        history.ripple_cuts(1, &[(0, S)], dir.path()).unwrap();
        history.undo(2, dir.path()).unwrap();
        let step = history.steps().undo_step().unwrap();
        assert_eq!(step.by, Author::Agent);
        assert_eq!(step.agent.as_deref(), Some("claude-code"));
        assert_eq!(step.tool.as_deref(), Some("silence"));
        assert_eq!(step.label, "Cut 3.0 s");
        assert_eq!(history.steps().redo_step().unwrap().by, Author::User);
    }

    #[test]
    fn an_agents_steps_form_one_run_until_someone_else_edits() {
        use crate::project::history::as_agent;
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        as_agent("claude-code", "silence", || {
            history.ripple_cuts(0, &[(S, 2 * S)], dir.path()).unwrap();
            history.ripple_cuts(1, &[(S, 2 * S)], dir.path()).unwrap();
            history.ripple_cuts(2, &[(S, 2 * S)], dir.path()).unwrap();
        });
        let run = history.steps().last_run().unwrap();
        assert_eq!((run.changes, run.can_undo), (3, true));

        history.ripple_cuts(3, &[(0, S)], dir.path()).unwrap();
        let run = history.steps().last_run().unwrap();
        assert!(!run.can_undo);
        assert_eq!(run.blocked_by.as_deref(), Some("\"Cut 1.0 s\""));
        let refused = history.undo_run(4, dir.path()).unwrap_err();
        assert!(refused.contains("\"Cut 1.0 s\" came after it"), "{refused}");

        history.undo(4, dir.path()).unwrap();
        let run = history.undo_run(5, dir.path()).unwrap();
        assert_eq!((run.agent.as_str(), run.changes), ("claude-code", 3));
        assert_eq!(history.current.duration_us(), 10 * S);
        assert_eq!(
            history.undo_label(),
            Some("Undo claude-code's run (3 changes)")
        );
        // Undoing the run undo brings all of it back; the run is still one.
        history.undo(6, dir.path()).unwrap();
        assert_eq!(history.current.duration_us(), 7 * S);
        assert_eq!(history.steps().last_run().unwrap().changes, 3);
    }

    #[test]
    fn a_new_workflow_or_another_agent_starts_a_new_run_and_runs_survive_reopening() {
        use crate::project::history::as_agent;
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        as_agent("claude-code", "silence", || {
            history.ripple_cuts(0, &[(S, 2 * S)], dir.path()).unwrap();
        });
        history.end_run();
        as_agent("claude-code", "fillers", || {
            history.ripple_cuts(1, &[(S, 2 * S)], dir.path()).unwrap();
        });
        as_agent("codex", "fillers", || {
            history.ripple_cuts(2, &[(S, 2 * S)], dir.path()).unwrap();
            history.ripple_cuts(3, &[(S, 2 * S)], dir.path()).unwrap();
        });
        drop(history);
        let (mut history, _) = reopen(dir.path());
        let run = history.steps().last_run().unwrap();
        assert_eq!((run.agent.as_str(), run.changes), ("codex", 2));
        history.undo_run(4, dir.path()).unwrap();
        assert_eq!(history.current.duration_us(), 8 * S);
        let run = history.steps().last_run().unwrap();
        assert_eq!(
            (run.agent.as_str(), run.undone, run.can_undo),
            ("codex", true, false)
        );
        assert!(history
            .undo_run(5, dir.path())
            .unwrap_err()
            .contains("already undone"));
        // Undo the run undo and codex's two steps: claude-code's second run is next.
        history.undo(5, dir.path()).unwrap();
        history.undo(6, dir.path()).unwrap();
        history.undo(7, dir.path()).unwrap();
        let run = history.steps().last_run().unwrap();
        assert_eq!(
            (run.agent.as_str(), run.changes, run.can_undo),
            ("claude-code", 1, true)
        );
        assert_eq!(history.current.duration_us(), 8 * S);
    }

    fn graphic(root: &Path, id: &str, text: &str) {
        let dir = root.join("graphics").join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("index.html"), text).unwrap();
    }

    fn graphic_text(root: &Path, id: &str) -> String {
        fs::read_to_string(root.join("graphics").join(id).join("index.html")).unwrap()
    }

    fn transcript(created_at: &str) -> crate::transcript::Transcript {
        serde_json::from_value(serde_json::json!({
            "schemaVersion": crate::transcript::TRANSCRIPT_SCHEMA_VERSION,
            "trackId": "rec.mic",
            "provider": "parakeet",
            "model": "test",
            "createdAt": created_at,
            "words": [
                { "id": "w-1", "text": "Hello", "kind": "word", "sourceStartUs": 0, "sourceEndUs": 100_000 },
                { "id": "w-2", "text": "there", "kind": "word", "sourceStartUs": 100_000, "sourceEndUs": 200_000 },
            ],
        }))
        .unwrap()
    }

    fn marks(root: &Path) -> usize {
        crate::transcript::store::load_transcript(root, "rec.mic")
            .unwrap()
            .unwrap()
            .caption_marks
            .len()
    }

    fn mark(root: &Path) {
        let mut t = crate::transcript::store::load_transcript(root, "rec.mic")
            .unwrap()
            .unwrap();
        t.caption_marks.insert(
            "w-2".into(),
            crate::transcript::CaptionMark {
                cue_break: true,
                ..Default::default()
            },
        );
        crate::transcript::store::save_transcript(root, &t).unwrap();
    }

    #[test]
    fn a_checkpoint_brings_back_the_edit_graphics_and_caption_marks_after_reopening() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        graphic(root, "m-1", "<h1>Clean</h1>");
        crate::transcript::store::save_transcript(root, &transcript("2026-10-23T10:00:00Z"))
            .unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history.ripple_cuts(0, &[(S, 2 * S)], root).unwrap();
        let saved = history.save_checkpoint(root, "Clean cut", false).unwrap();
        assert_eq!((saved.id.as_str(), saved.graphics.len()), ("c-1", 1));

        // Later: another cut, a graphic changed and one added, a caption break.
        history.ripple_cuts(1, &[(S, 3 * S)], root).unwrap();
        graphic(root, "m-1", "<h1>Changed</h1>");
        fs::write(root.join("graphics/m-1/extra.css"), "h1{}").unwrap();
        graphic(root, "m-2", "<h1>New</h1>");
        mark(root);
        drop(history);

        let (mut history, _) = reopen(root);
        assert_eq!(history.steps().checkpoints().len(), 1);
        let (checkpoint, restored) = history.restore_checkpoint(2, root, "c-1").unwrap();
        assert_eq!(checkpoint.name, "Clean cut");
        assert_eq!(history.current.duration_us(), 9 * S);
        assert_eq!(history.undo_label(), Some("Restore \"Clean cut\""));
        assert_eq!((restored.graphics, restored.captions), (1, 1));
        assert_eq!(graphic_text(root, "m-1"), "<h1>Clean</h1>");
        assert!(!root.join("graphics/m-1/extra.css").exists());
        // Made after the checkpoint: left alone.
        assert_eq!(graphic_text(root, "m-2"), "<h1>New</h1>");
        assert_eq!(marks(root), 0);
        // What was there is kept too, so going back can itself be taken back.
        let before = &history.steps().checkpoints()[1];
        assert!(before.auto && before.name == "Before restoring \"Clean cut\"");
        let (_, restored) = history
            .restore_checkpoint(3, root, &before.id.clone())
            .unwrap();
        assert_eq!(graphic_text(root, "m-1"), "<h1>Changed</h1>");
        assert_eq!(history.current.duration_us(), 7 * S);
        assert_eq!(restored.captions, 1);
    }

    #[test]
    fn caption_marks_stay_when_the_sound_was_transcribed_again() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        crate::transcript::store::save_transcript(root, &transcript("2026-10-23T10:00:00Z"))
            .unwrap();
        let mut history = EditHistory::new(doc(10 * S));
        history.save_checkpoint(root, "Before", false).unwrap();
        crate::transcript::store::save_transcript(root, &transcript("2026-10-23T11:00:00Z"))
            .unwrap();
        mark(root);
        let (_, restored) = history.restore_checkpoint(0, root, "c-1").unwrap();
        assert_eq!(restored.captions, 0);
        assert!(restored
            .summary()
            .contains("rec.mic stay as they are: it was transcribed again"));
        assert_eq!(marks(root), 1);
        // Nothing in the edit changed, so no step was made.
        assert_eq!(history.current.revision, 0);
    }

    #[test]
    fn checkpoints_before_runs_keep_the_newest_few_and_their_states_outlive_the_steps() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let mut history = EditHistory::new(doc(10 * S));
        for revision in 0..8u64 {
            history
                .save_checkpoint(root, &format!("Before run {revision}"), true)
                .unwrap();
            // The same again adds nothing.
            history.save_checkpoint(root, "Again", true).unwrap();
            history.ripple_cuts(revision, &[(0, 10_000)], root).unwrap();
        }
        let names: Vec<&str> = history
            .steps()
            .checkpoints()
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "Before run 3",
                "Before run 4",
                "Before run 5",
                "Before run 6",
                "Before run 7"
            ]
        );
        // Renaming one keeps it.
        history.rename_checkpoint(root, "c-4", "Keep me").unwrap();
        assert!(!history.steps().checkpoint("c-4").unwrap().auto);
        // A tiny limit lets every old step go, but not the checkpoints' states.
        history.steps_mut().set_limit_bytes(1);
        history.ripple_cuts(8, &[(0, 10_000)], root).unwrap();
        assert!(history.steps().undo_len() <= 1);
        let (_, _) = history.restore_checkpoint(9, root, "c-4").unwrap();
        assert_eq!(history.current.duration_us(), 10 * S - 3 * 10_000);
        history.delete_checkpoint(root, "c-4").unwrap();
        assert!(history.steps().checkpoint("c-4").is_err());
        drop(history);
        let (history, _) = reopen(root);
        assert!(history.steps().checkpoint("c-4").is_err());
        assert_eq!(history.steps().checkpoints().len(), 5);
    }

    #[test]
    fn files_no_checkpoint_needs_are_let_go() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        graphic(root, "m-1", "one");
        let mut history = EditHistory::new(doc(10 * S));
        history.save_checkpoint(root, "A", false).unwrap();
        graphic(root, "m-1", "two");
        history.save_checkpoint(root, "B", false).unwrap();
        assert_eq!(history.steps().kept_files(), 2);
        history.delete_checkpoint(root, "c-1").unwrap();
        assert_eq!(history.steps().kept_files(), 1);
        assert_eq!(fs::read_dir(root.join("history/files")).unwrap().count(), 1);
    }

    #[test]
    fn a_step_that_adds_a_graphic_is_named_for_it() {
        use crate::project::history::describe;
        let before = doc(10 * S);
        let mut after = before.clone();
        let mut graphic = after.assets[0].clone();
        graphic.id = "m-1".into();
        graphic.kind = crate::sequence::AssetKind::Graphic;
        after.assets.push(graphic);
        after.sequence.tracks[0].clips.clear();
        assert_eq!(describe(&before, &after), "Added a graphic");
        assert_eq!(describe(&after, &before), "Removed a graphic");
        after.captions.enabled = !after.captions.enabled;
        assert_eq!(describe(&before, &after), "Added a graphic, Captions");
    }

    /// The EVAL-1 rerun's zoom centre: read back with serde_json's fast float parser it came
    /// out one bit off, so the reopened edit hashed as a change made outside the app.
    #[test]
    fn numbers_read_back_exactly_so_a_reopened_edit_is_the_same_state() {
        let text = "0.20614035087719298";
        let value: f64 = serde_json::from_str(text).unwrap();
        assert_eq!(serde_json::to_string(&value).unwrap(), text);
    }

    /// What history adds to one edit of a real project:
    /// `AEROEDITS_HISTORY_PROJECT=<project folder> cargo test --lib history_cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn history_cost_on_a_real_project() {
        let Ok(project) = std::env::var("AEROEDITS_HISTORY_PROJECT") else {
            return;
        };
        let document = load_edit_document(Path::new(&project)).unwrap().unwrap();
        let dir = tempdir().unwrap();
        save_edit_document(dir.path(), &document).unwrap();
        let mut history = EditHistory::new(document);
        let mut times = Vec::new();
        for _ in 0..20 {
            let revision = history.current.revision;
            let started = std::time::Instant::now();
            history
                .ripple_cuts(revision, &[(0, 10_000)], dir.path())
                .unwrap();
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        // The part that is history's own: saving one new state.
        let mut saves = Vec::new();
        for i in 0..20u64 {
            let mut next = history.current.clone();
            next.zoom_settings.transition_ms = 100 + i as u32;
            let started = std::time::Instant::now();
            history.steps_mut().save_state(dir.path(), &next).unwrap();
            saves.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        saves.sort_by(|a, b| a.partial_cmp(b).unwrap());
        eprintln!("saving a state p50 {:.1} ms", saves[10]);
        let json = fs::metadata(dir.path().join("project.json")).unwrap().len();
        eprintln!(
            "project.json {} KB, state {} KB, edit p50 {:.1} ms, max {:.1} ms",
            json / 1024,
            history.steps().stored_bytes() / states(dir.path()) as u64 / 1024,
            times[10],
            times[19]
        );
    }
}
