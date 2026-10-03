//! Video tracks above the main sequence. The main sequence (the recording and the media
//! inserted into it) is V1 and ripples as it is cut. Overlay tracks V2, V3, ... hold clips of
//! imported media at fixed positions on the timeline; a higher track draws over the ones below,
//! and their sound plays along with the main sequence.
use crate::media_bin::MediaKind;
use crate::project::revision::EditDocument;
use crate::project::RetainedInterval;
use serde::{Deserialize, Serialize};

pub const MAX_OVERLAY_TRACKS: usize = 8;
pub const MAX_CLIPS_PER_TRACK: usize = 256;
/// Shortest clip a trim can leave.
pub const MIN_CLIP_US: u64 = 100_000;

/// How a clip's picture fills the canvas.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OverlayFit {
    /// The whole picture, centred; what is below shows around it.
    #[default]
    Contain,
    /// The whole canvas, cropping the picture's edges.
    Cover,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayClip {
    pub id: String,
    pub asset_id: String,
    /// Where the clip starts on the timeline.
    pub start_us: u64,
    /// Where in the media the clip starts.
    #[serde(default)]
    pub in_us: u64,
    pub duration_us: u64,
    #[serde(default)]
    pub fit: OverlayFit,
}

impl OverlayClip {
    pub fn end_us(&self) -> u64 {
        self.start_us.saturating_add(self.duration_us)
    }

    /// The media time shown at timeline time `edited_us`, if the clip covers it.
    pub fn local_us(&self, edited_us: u64) -> Option<u64> {
        (self.start_us <= edited_us && edited_us < self.end_us())
            .then(|| self.in_us + (edited_us - self.start_us))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTrack {
    pub id: String,
    #[serde(default)]
    pub clips: Vec<OverlayClip>,
    /// Not drawn.
    #[serde(default)]
    pub hidden: bool,
    /// Not heard.
    #[serde(default)]
    pub muted: bool,
}

impl OverlayTrack {
    pub fn clip_at(&self, edited_us: u64) -> Option<&OverlayClip> {
        self.clips
            .iter()
            .find(|clip| clip.local_us(edited_us).is_some())
    }
}

/// One change to the tracks, applied as one undoable edit.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TrackEdit {
    /// A new empty track above the others.
    AddTrack,
    RemoveTrack {
        track_id: String,
    },
    SetTrack {
        track_id: String,
        hidden: bool,
        muted: bool,
    },
    /// Media from the bin onto a track at `start_us`, at its default length.
    PlaceMedia {
        asset_id: String,
        track_id: String,
        start_us: u64,
    },
    /// Moves, trims or refits a clip; it may change track.
    UpdateClip {
        clip: OverlayClip,
        track_id: String,
    },
    RemoveClip {
        clip_id: String,
    },
    /// Takes the media clip `[start_us, end_us)` out of the main sequence (which closes up)
    /// and puts it on a track at `at_us`.
    LiftFromMain {
        start_us: u64,
        end_us: u64,
        track_id: String,
        at_us: u64,
    },
    /// Takes a clip off its track and inserts it into the main sequence at `target_us`.
    DropToMain {
        clip_id: String,
        target_us: u64,
    },
}

fn track_index(document: &EditDocument, track_id: &str) -> Result<usize, String> {
    document
        .overlay_tracks
        .iter()
        .position(|track| track.id == track_id)
        .ok_or_else(|| "No such track".to_string())
}

fn take_clip(document: &mut EditDocument, clip_id: &str) -> Result<OverlayClip, String> {
    for track in &mut document.overlay_tracks {
        if let Some(index) = track.clips.iter().position(|clip| clip.id == clip_id) {
            return Ok(track.clips.remove(index));
        }
    }
    Err("No such clip".into())
}

fn new_id(document: &EditDocument, prefix: &str) -> String {
    let taken = |id: &str| {
        document
            .overlay_tracks
            .iter()
            .any(|track| track.id == id || track.clips.iter().any(|clip| clip.id == id))
    };
    (1..)
        .map(|n| format!("{prefix}-{n}"))
        .find(|id| !taken(id))
        .unwrap()
}

fn put_clip(document: &mut EditDocument, track_id: &str, clip: OverlayClip) -> Result<(), String> {
    let index = track_index(document, track_id)?;
    let clips = &mut document.overlay_tracks[index].clips;
    clips.push(clip);
    clips.sort_by_key(|clip| clip.start_us);
    Ok(())
}

/// The document after `edit`. Validation of the result happens when it is committed.
pub fn apply(document: &EditDocument, edit: &TrackEdit) -> Result<EditDocument, String> {
    let mut next = document.clone();
    match edit {
        TrackEdit::AddTrack => {
            if next.overlay_tracks.len() >= MAX_OVERLAY_TRACKS {
                return Err(format!(
                    "A project has at most {} video tracks",
                    MAX_OVERLAY_TRACKS + 1
                ));
            }
            let id = new_id(&next, "track");
            next.overlay_tracks.push(OverlayTrack {
                id,
                clips: Vec::new(),
                hidden: false,
                muted: false,
            });
        }
        TrackEdit::RemoveTrack { track_id } => {
            let index = track_index(&next, track_id)?;
            next.overlay_tracks.remove(index);
        }
        TrackEdit::SetTrack {
            track_id,
            hidden,
            muted,
        } => {
            let index = track_index(&next, track_id)?;
            let track = &mut next.overlay_tracks[index];
            track.hidden = *hidden;
            track.muted = *muted;
        }
        TrackEdit::PlaceMedia {
            asset_id,
            track_id,
            start_us,
        } => {
            let asset = next
                .media_assets
                .iter()
                .find(|asset| &asset.id == asset_id)
                .ok_or("No such imported media")?;
            let clip = OverlayClip {
                id: new_id(&next, "clip"),
                asset_id: asset_id.clone(),
                start_us: *start_us,
                in_us: 0,
                duration_us: asset.default_clip_us(),
                fit: OverlayFit::default(),
            };
            put_clip(&mut next, track_id, clip)?;
        }
        TrackEdit::UpdateClip { clip, track_id } => {
            let old = take_clip(&mut next, &clip.id)?;
            if old.asset_id != clip.asset_id {
                return Err("A clip keeps its media".into());
            }
            put_clip(&mut next, track_id, clip.clone())?;
        }
        TrackEdit::RemoveClip { clip_id } => {
            take_clip(&mut next, clip_id)?;
        }
        TrackEdit::LiftFromMain {
            start_us,
            end_us,
            track_id,
            at_us,
        } => {
            let (asset_id, in_us) = media_under(&next.retained_intervals, *start_us, *end_us)?;
            let retained = &mut next.retained_intervals;
            let first = crate::project::revision::split_at_edited(retained, *start_us)?;
            let last = crate::project::revision::split_at_edited(retained, *end_us)?;
            retained.drain(first..last);
            if retained.is_empty() {
                return Err("The main track cannot be left empty".into());
            }
            next.retained_intervals =
                crate::project::revision::canonical_retained(next.retained_intervals);
            let clip = OverlayClip {
                id: new_id(&next, "clip"),
                asset_id,
                start_us: *at_us,
                in_us,
                duration_us: end_us - start_us,
                fit: OverlayFit::default(),
            };
            put_clip(&mut next, track_id, clip)?;
        }
        TrackEdit::DropToMain { clip_id, target_us } => {
            let clip = take_clip(&mut next, clip_id)?;
            let at = crate::project::revision::split_at_edited(
                &mut next.retained_intervals,
                *target_us,
            )?;
            next.retained_intervals.insert(
                at,
                RetainedInterval {
                    start_us: clip.in_us,
                    end_us: clip.in_us + clip.duration_us,
                    media: Some(clip.asset_id),
                },
            );
        }
    }
    Ok(next)
}

/// The imported media and the offset into it that the main-sequence range `[start, end)`
/// shows, when all of it is one clip of one file. Recording clips stay on the main track.
fn media_under(
    retained: &[RetainedInterval],
    start_us: u64,
    end_us: u64,
) -> Result<(String, u64), String> {
    if start_us >= end_us {
        return Err("Choose a clip to move".into());
    }
    let mut cursor = 0u64;
    for interval in retained {
        let length = interval.end_us - interval.start_us;
        if start_us >= cursor && end_us <= cursor + length {
            return match &interval.media {
                Some(id) => Ok((id.clone(), interval.start_us + (start_us - cursor))),
                None => {
                    Err("The recording stays on V1; imported media can move to other tracks".into())
                }
            };
        }
        cursor += length;
    }
    Err("Only one whole imported clip can move to another track".into())
}

pub fn validate(document: &EditDocument) -> Result<(), String> {
    let tracks = &document.overlay_tracks;
    if tracks.len() > MAX_OVERLAY_TRACKS {
        return Err(format!(
            "A project has at most {} video tracks",
            MAX_OVERLAY_TRACKS + 1
        ));
    }
    let mut ids = std::collections::HashSet::new();
    for track in tracks {
        if track.id.is_empty() || track.id.len() > 64 || !ids.insert(track.id.as_str()) {
            return Err("Every track needs its own id".into());
        }
        if track.clips.len() > MAX_CLIPS_PER_TRACK {
            return Err(format!("A track holds at most {MAX_CLIPS_PER_TRACK} clips"));
        }
        for clip in &track.clips {
            if clip.id.is_empty() || clip.id.len() > 64 || !ids.insert(clip.id.as_str()) {
                return Err("Every clip needs its own id".into());
            }
            let asset = document
                .media_assets
                .iter()
                .find(|asset| asset.id == clip.asset_id)
                .ok_or("A track uses media that is not imported")?;
            if clip.duration_us < MIN_CLIP_US {
                return Err("A clip must be at least 0.1 s long".into());
            }
            let fits = match asset.kind {
                // A still can be shown for as long as an image clip may last.
                MediaKind::Image => clip.in_us == 0 && clip.duration_us <= asset.duration_us,
                _ => clip
                    .in_us
                    .checked_add(clip.duration_us)
                    .is_some_and(|end| end <= asset.duration_us),
            };
            if !fits {
                return Err("The clip must lie within the media's length".into());
            }
            clip.start_us
                .checked_add(clip.duration_us)
                .ok_or("Clip position is out of range")?;
        }
        if track
            .clips
            .windows(2)
            .any(|pair| pair[0].start_us > pair[1].start_us || pair[0].end_us() > pair[1].start_us)
        {
            return Err("Clips on a track cannot overlap".into());
        }
    }
    Ok(())
}

/// Removes every clip of `asset_id` from the tracks.
pub fn remove_asset(document: &mut EditDocument, asset_id: &str) {
    for track in &mut document.overlay_tracks {
        track.clips.retain(|clip| clip.asset_id != asset_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media_bin::MediaAsset;

    const S: u64 = 1_000_000;

    fn document() -> EditDocument {
        let mut document = EditDocument::from_retained(vec![
            RetainedInterval {
                start_us: 0,
                end_us: 10 * S,
                media: None,
            },
            RetainedInterval {
                start_us: S,
                end_us: 5 * S,
                media: Some("v".into()),
            },
            RetainedInterval {
                start_us: 10 * S,
                end_us: 20 * S,
                media: None,
            },
        ])
        .unwrap();
        document.media_assets = vec![
            MediaAsset {
                id: "v".into(),
                name: "broll.mp4".into(),
                kind: MediaKind::Video,
                relative_path: "assets/media/v.mp4".into(),
                audio_path: None,
                duration_us: 8 * S,
                width: 1920,
                height: 1080,
            },
            MediaAsset {
                id: "img".into(),
                name: "logo.png".into(),
                kind: MediaKind::Image,
                relative_path: "assets/media/img.png".into(),
                audio_path: None,
                duration_us: 60 * S,
                width: 400,
                height: 400,
            },
        ];
        document
    }

    #[test]
    fn tracks_stack_and_clips_place_move_and_refuse_overlap() {
        let mut doc = apply(&document(), &TrackEdit::AddTrack).unwrap();
        doc = apply(&doc, &TrackEdit::AddTrack).unwrap();
        let ids: Vec<_> = doc.overlay_tracks.iter().map(|t| t.id.clone()).collect();
        assert_eq!(ids, ["track-1", "track-2"]);
        doc = apply(
            &doc,
            &TrackEdit::PlaceMedia {
                asset_id: "img".into(),
                track_id: "track-1".into(),
                start_us: 2 * S,
            },
        )
        .unwrap();
        validate(&doc).unwrap();
        let clip = doc.overlay_tracks[0].clips[0].clone();
        assert_eq!((clip.start_us, clip.duration_us), (2 * S, 5 * S));
        assert_eq!(
            doc.overlay_tracks[0].clip_at(6 * S).map(|c| c.id.as_str()),
            Some("clip-1")
        );
        assert_eq!(clip.local_us(3 * S), Some(S));

        // Up a track and later in time.
        let moved = OverlayClip {
            start_us: 4 * S,
            ..clip.clone()
        };
        doc = apply(
            &doc,
            &TrackEdit::UpdateClip {
                clip: moved,
                track_id: "track-2".into(),
            },
        )
        .unwrap();
        assert!(doc.overlay_tracks[0].clips.is_empty());
        assert_eq!(doc.overlay_tracks[1].clips[0].start_us, 4 * S);

        // A second clip on top of the first is refused when committed.
        let overlapping = apply(
            &doc,
            &TrackEdit::PlaceMedia {
                asset_id: "img".into(),
                track_id: "track-2".into(),
                start_us: 6 * S,
            },
        )
        .unwrap();
        assert!(validate(&overlapping).is_err());

        // A video clip cannot run past the end of its file.
        let mut long = doc.clone();
        long.overlay_tracks[1].clips[0] = OverlayClip {
            id: "clip-9".into(),
            asset_id: "v".into(),
            start_us: 0,
            in_us: 6 * S,
            duration_us: 3 * S,
            fit: OverlayFit::Cover,
        };
        assert!(validate(&long).is_err());

        doc = apply(
            &doc,
            &TrackEdit::RemoveTrack {
                track_id: "track-1".into(),
            },
        )
        .unwrap();
        assert_eq!(doc.overlay_tracks.len(), 1);
        remove_asset(&mut doc, "img");
        assert!(doc.overlay_tracks[0].clips.is_empty());
    }

    #[test]
    fn media_lifts_off_the_main_track_and_drops_back() {
        let mut doc = apply(&document(), &TrackEdit::AddTrack).unwrap();
        // The recording stays on V1.
        assert!(apply(
            &doc,
            &TrackEdit::LiftFromMain {
                start_us: 0,
                end_us: 10 * S,
                track_id: "track-1".into(),
                at_us: 0,
            },
        )
        .is_err());
        // The imported clip at 10..14 s goes up; the main sequence closes up to 20 s.
        doc = apply(
            &doc,
            &TrackEdit::LiftFromMain {
                start_us: 10 * S,
                end_us: 14 * S,
                track_id: "track-1".into(),
                at_us: 3 * S,
            },
        )
        .unwrap();
        validate(&doc).unwrap();
        assert_eq!(doc.edited_duration_us().unwrap(), 20 * S);
        assert!(doc.retained_intervals.iter().all(|i| i.media.is_none()));
        let clip = doc.overlay_tracks[0].clips[0].clone();
        assert_eq!(
            (clip.start_us, clip.in_us, clip.duration_us),
            (3 * S, S, 4 * S)
        );

        // And back down, into the middle of the first recording clip.
        doc = apply(
            &doc,
            &TrackEdit::DropToMain {
                clip_id: clip.id,
                target_us: 5 * S,
            },
        )
        .unwrap();
        assert!(doc.overlay_tracks[0].clips.is_empty());
        assert_eq!(doc.edited_duration_us().unwrap(), 24 * S);
        assert_eq!(
            doc.retained_intervals[1],
            RetainedInterval {
                start_us: S,
                end_us: 5 * S,
                media: Some("v".into())
            }
        );
    }
}
