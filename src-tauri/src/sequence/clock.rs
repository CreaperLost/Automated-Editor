//! Where an asset's own time lands on the timeline: through the clips that play it.
//!
//! Zooms, the cursor and webcam focus are on a recording's clock and follow its picture
//! clips; transcript words, captions and pauses are on a sound stream's clock and follow that
//! stream's clips; chapters and shorts follow any clip of their asset. A clock is a
//! [`TimelineMapper`] whose entries are those clips in timeline order, with the time between
//! them as gaps that map nowhere. Where one moment of the asset plays twice, the earlier clip
//! on the timeline counts.
use super::{Asset, Clip, Role, Sequence, StreamKind, Track};
use crate::project::RetainedInterval;
use crate::timeline::TimelineMapper;

/// The media marker of a clock entry that maps nowhere.
pub const GAP: &str = "gap";

/// The clock through the clips `pick` chooses, as entries in timeline order: each clip as a
/// range of the asset's time, the time between as gaps.
pub fn entries(sequence: &Sequence, pick: impl Fn(&Track, &Clip) -> bool) -> Vec<RetainedInterval> {
    layers(sequence, pick)
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// At most this many plays of one moment get a clock of their own.
const MAX_LAYERS: usize = 8;

/// Clocks through the clips `pick` chooses that, together, place every play of each moment:
/// the first is [`entries`] (the first time a moment plays), the next holds the moments played
/// a second time, and so on. Captions read each, so a repeated line is captioned each time it
/// plays (GAP-023).
pub fn layers(
    sequence: &Sequence,
    pick: impl Fn(&Track, &Clip) -> bool,
) -> Vec<Vec<RetainedInterval>> {
    let mut clips: Vec<(usize, &Clip)> = sequence
        .tracks
        .iter()
        .enumerate()
        .flat_map(|(i, t)| t.clips.iter().map(move |c| (i, t, c)))
        .filter(|(_, t, c)| pick(t, c))
        .map(|(i, _, c)| (i, c))
        .collect();
    clips.sort_by_key(|(track, c)| (c.start_us, *track));
    struct Layer {
        out: Vec<RetainedInterval>,
        /// The asset's time placed so far, merged and in order.
        used: Vec<(u64, u64)>,
        cursor: u64,
    }
    let mut layers: Vec<Layer> = Vec::new();
    for (_, clip) in clips {
        let out_us = clip.out_us();
        // The parts of the clip still to place; those already placed in a layer try the next.
        let mut pending = vec![(clip.in_us, out_us)];
        for index in 0..MAX_LAYERS {
            if pending.is_empty() {
                break;
            }
            if index == layers.len() {
                layers.push(Layer {
                    out: Vec::new(),
                    used: Vec::new(),
                    cursor: 0,
                });
            }
            let layer = &mut layers[index];
            // What the clip adds here: its part after the timeline already used, less the
            // moments already placed (the first time a moment plays counts).
            let skip = clip.source_offset(
                layer
                    .cursor
                    .saturating_sub(clip.start_us)
                    .min(clip.duration_us),
            );
            let from_us = (clip.in_us + skip).min(out_us);
            let mut repeats = Vec::new();
            for (from, to) in pending {
                let from = from.max(from_us);
                if from >= to {
                    continue;
                }
                let fresh = unplaced(&layer.used, from, to);
                let mut at = from;
                for &(a, b) in &fresh {
                    if a > at {
                        repeats.push((at, a));
                    }
                    at = b;
                }
                if at < to {
                    repeats.push((at, to));
                }
                for (a, b) in fresh {
                    place(&mut layer.used, (a, b));
                    // Where it plays, at the clip's speed (rounding never steps back over the last).
                    let start = clip.timeline_at(a).max(layer.cursor);
                    let end = if b == out_us {
                        clip.end_us()
                    } else {
                        clip.timeline_at(b)
                    };
                    if end <= start {
                        continue;
                    }
                    if start > layer.cursor {
                        layer.out.push(RetainedInterval {
                            start_us: 0,
                            end_us: start - layer.cursor,
                            media: Some(GAP.into()),
                            ..Default::default()
                        });
                    }
                    layer.out.push(RetainedInterval {
                        edited_len_us: (end - start != b - a).then_some(end - start),
                        ..RetainedInterval::recording(a, b)
                    });
                    layer.cursor = end;
                }
            }
            pending = repeats;
        }
    }
    layers.into_iter().map(|layer| layer.out).collect()
}

/// `[from, to)` less the ranges in `used` (merged and in order). Starts at the first range
/// that can touch it, so a long jump-cut sequence costs a binary search per clip.
fn unplaced(used: &[(u64, u64)], from: u64, to: u64) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut at = from;
    let first = used.partition_point(|&(_, b)| b <= from);
    for &(a, b) in &used[first..] {
        if a >= to {
            break;
        }
        if a > at {
            out.push((at, a));
        }
        at = at.max(b);
    }
    if at < to {
        out.push((at, to));
    }
    out
}

/// Adds `range` to `used`, keeping it merged and in order: only the ranges it touches are
/// merged, in place.
fn place(used: &mut Vec<(u64, u64)>, range: (u64, u64)) {
    let (mut a, mut b) = range;
    let first = used.partition_point(|&(_, end)| end < a);
    let mut last = first;
    while last < used.len() && used[last].0 <= b {
        a = a.min(used[last].0);
        b = b.max(used[last].1);
        last += 1;
    }
    used.splice(first..last, std::iter::once((a, b)));
}

pub fn mapper(entries: &[RetainedInterval]) -> TimelineMapper {
    TimelineMapper::new(
        entries
            .iter()
            .enumerate()
            .map(|(i, e)| e.interval(format!("k{i}")))
            .collect(),
    )
}

/// One stream's clock: where its clips play it.
pub fn stream_entries(sequence: &Sequence, asset: &str, stream: &str) -> Vec<RetainedInterval> {
    entries(sequence, |_, c| c.asset == asset && c.stream == stream)
}

pub fn stream_clock(sequence: &Sequence, asset: &str, stream: &str) -> TimelineMapper {
    mapper(&stream_entries(sequence, asset, stream))
}

/// The clock of an asset's picture as the screen: its screen clips, else any of its picture
/// clips. Zooms and the cursor follow it.
pub fn picture_clock(sequence: &Sequence, assets: &[Asset], asset: &str) -> TimelineMapper {
    let picture = |t: &Track, c: &Clip| {
        c.asset == asset
            && assets
                .iter()
                .find(|a| a.id == asset)
                .and_then(|a| a.stream(&c.stream))
                .is_some_and(|s| s.kind == StreamKind::Picture && t.kind.holds(s.kind))
    };
    let screen = entries(sequence, |t, c| {
        picture(t, c) && super::clip_role(assets, t, c) == Some(Role::Screen)
    });
    if screen.iter().any(|e| e.media.is_none()) {
        return mapper(&screen);
    }
    mapper(&entries(sequence, picture))
}

/// The clock of a stream with role `role` of `asset` (its camera, for webcam focus).
pub fn role_clock(
    sequence: &Sequence,
    assets: &[Asset],
    asset: &str,
    role: Role,
) -> TimelineMapper {
    mapper(&entries(sequence, |t, c| {
        c.asset == asset && super::clip_role(assets, t, c) == Some(role)
    }))
}

/// Any clip of the asset: chapters and shorts.
pub fn asset_clock(sequence: &Sequence, asset: &str) -> TimelineMapper {
    mapper(&entries(sequence, |_, c| c.asset == asset))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::{Fit, TrackKind};

    fn clip(id: &str, stream: &str, start: u64, in_us: u64, len: u64) -> Clip {
        Clip {
            mix: Default::default(),
            id: id.into(),
            asset: "rec".into(),
            stream: stream.into(),
            start_us: start,
            in_us,
            duration_us: len,
            link: None,
            fit: Fit::Contain,
            speed: 1.0,
        }
    }

    #[test]
    fn a_stream_maps_through_its_clips_with_gaps_between() {
        let mut a1 = Track::new("a1".into(), TrackKind::Audio);
        // 0-2 s plays source 5-7 s; 3-4 s plays source 0-1 s.
        a1.clips = vec![
            clip("c1", "mic", 0, 5_000_000, 2_000_000),
            clip("c2", "mic", 3_000_000, 0, 1_000_000),
        ];
        let seq = Sequence {
            tracks: vec![a1],
            magnetic: true,
        };
        let clock = stream_clock(&seq, "rec", "mic");
        assert_eq!(clock.source_to_edited_us(5_500_000), Some(500_000));
        assert_eq!(clock.source_to_edited_us(500_000), Some(3_500_000));
        assert_eq!(clock.source_to_edited_us(8_000_000), None, "never placed");
        assert_eq!(clock.edited_to_source_us(2_500_000), None, "the gap");
        assert_eq!(clock.edited_to_source_us(3_000_000), Some(0));
        assert!(stream_clock(&seq, "rec", "system").intervals().is_empty());
    }

    #[test]
    fn a_clip_that_partly_repeats_keeps_what_is_new() {
        // Source 0-5 s at 0-5 s, then source 4-10 s at 5-11 s: 4-5 s plays twice.
        let mut a1 = Track::new("a1".into(), TrackKind::Audio);
        a1.clips = vec![
            clip("c1", "mic", 0, 0, 5_000_000),
            clip("c2", "mic", 5_000_000, 4_000_000, 6_000_000),
        ];
        let seq = Sequence {
            tracks: vec![a1],
            magnetic: true,
        };
        let clock = stream_clock(&seq, "rec", "mic");
        assert_eq!(
            clock.source_to_edited_us(4_500_000),
            Some(4_500_000),
            "first counts"
        );
        assert_eq!(clock.source_to_edited_us(8_000_000), Some(9_000_000));
        assert_eq!(clock.edited_to_source_us(9_000_000), Some(8_000_000));
        assert_eq!(
            clock.edited_to_source_us(5_500_000),
            None,
            "the repeat maps nowhere"
        );
        // Overlapping on the timeline too: a higher track's clip starting inside another.
        let mut v1 = Track::new("v1".into(), TrackKind::Video);
        let mut v2 = Track::new("v2".into(), TrackKind::Video);
        v1.clips = vec![clip("c1", "screen", 0, 0, 2_000_000)];
        v2.clips = vec![clip("c2", "screen", 1_000_000, 10_000_000, 2_000_000)];
        let seq = Sequence {
            tracks: vec![v1, v2],
            magnetic: true,
        };
        let clock = asset_clock(&seq, "rec");
        assert_eq!(clock.source_to_edited_us(11_500_000), Some(2_500_000));
        assert_eq!(
            clock.source_to_edited_us(10_500_000),
            None,
            "under the first"
        );
    }

    #[test]
    fn a_clip_played_faster_maps_at_its_speed() {
        let mut a1 = Track::new("a1".into(), TrackKind::Audio);
        // 0-2 s plays source 0-4 s at 2x; 2-4 s plays source 10-11 s at half speed.
        a1.clips = vec![
            Clip {
                speed: 2.0,
                ..clip("c1", "mic", 0, 0, 2_000_000)
            },
            Clip {
                speed: 0.5,
                ..clip("c2", "mic", 2_000_000, 10_000_000, 2_000_000)
            },
        ];
        let seq = Sequence {
            tracks: vec![a1],
            magnetic: true,
        };
        let clock = stream_clock(&seq, "rec", "mic");
        assert_eq!(clock.source_to_edited_us(3_000_000), Some(1_500_000));
        assert_eq!(clock.edited_to_source_us(1_000_000), Some(2_000_000));
        assert_eq!(clock.source_to_edited_us(10_500_000), Some(3_000_000));
        assert_eq!(clock.edited_to_source_us(3_000_000), Some(10_500_000));
        assert_eq!(clock.source_to_edited_us(11_000_000), None, "never placed");
        assert_eq!(clock.total_edited_duration_us(), 4_000_000);
    }

    #[test]
    fn the_same_moment_twice_counts_once() {
        let mut v1 = Track::new("v1".into(), TrackKind::Video);
        let mut v2 = Track::new("v2".into(), TrackKind::Video);
        v1.clips = vec![clip("c1", "screen", 0, 0, 2_000_000)];
        v2.clips = vec![clip("c2", "screen", 1_000_000, 0, 2_000_000)];
        let seq = Sequence {
            tracks: vec![v1, v2],
            magnetic: true,
        };
        let clock = asset_clock(&seq, "rec");
        assert_eq!(clock.source_to_edited_us(1_500_000), Some(1_500_000));
        assert_eq!(clock.total_edited_duration_us(), 2_000_000);
    }
}
