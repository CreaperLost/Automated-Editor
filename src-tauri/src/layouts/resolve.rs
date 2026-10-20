//! What a timeline shows, layout by layout: its sections mapped through their clocks, with
//! the rules that keep it from flickering, and the moves between them. Worked out once per
//! edit; each frame then costs a binary search ([`Resolved::frame_at`]).
//!
//! - Sections that overlap on the timeline (they are on different assets): one made by hand
//!   wins over a picked one, then the shorter one.
//! - A piece shorter than [`MIN_SHOWN_US`] (a cut left it) is not shown.
//! - The base showing for less than [`MIN_BASE_GAP_US`] between two sections: the earlier one
//!   runs on to the next.
//! - A move plays inside its section: over its first moments coming in, its last going back
//!   to the base. Between two sections that meet, over the start of the later one.
use super::{
    merge, subtract, Clocks, Layout, LayoutTrack, Transition, MIN_BASE_GAP_US, MIN_SHOWN_US,
};
use crate::timeline::TimelineMapper;

/// A layout change this close to a cut in the video is on it, so it cuts too.
pub const CUT_SNAP_US: u64 = 50_000;

#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub start_us: u64,
    pub end_us: u64,
    pub layout: Layout,
    /// The section shown, by id.
    pub section: String,
    /// What it changes from: the piece it meets, else the base.
    pub from: Layout,
    /// How long the change in and the change back to the base take; 0 is a cut.
    pub rise_us: u64,
    pub fall_us: u64,
    /// The next piece starts where this one ends (and takes over the change).
    pub meets_next: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub base: Layout,
    pub pieces: Vec<Piece>,
    pub duration_us: u64,
}

/// The layouts showing at a moment: `from` moving to `to`, `weight` of the way (0 to 1).
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    pub from: &'a Layout,
    pub to: &'a Layout,
    pub weight: f32,
}

impl Frame<'_> {
    /// How much a layout matching `test` shows, 0 to 1.
    pub fn share(&self, test: impl Fn(&Layout) -> bool) -> f32 {
        let a = if test(self.from) { 1.0 - self.weight } else { 0.0 };
        let b = if test(self.to) { self.weight } else { 0.0 };
        a + b
    }
}

/// `track`'s layouts on a timeline `duration_us` long. `clock` gives an asset's clock
/// (`None`: the first recording); `cuts` are the timeline's clip edges, sorted.
pub fn resolve(
    track: &LayoutTrack,
    clock: impl Fn(Option<&str>) -> Option<TimelineMapper>,
    cuts: &[u64],
    duration_us: u64,
) -> Resolved {
    let mut clocks = Clocks::new(clock);
    // Each section's pieces on the timeline, those of one section that touch joined.
    struct Candidate {
        start: u64,
        end: u64,
        index: usize,
    }
    let mut candidates: Vec<Candidate> = Vec::new();
    for (index, section) in track.sections.iter().enumerate() {
        let Some(clock) = clocks.get(section.media.as_deref()) else {
            continue;
        };
        let ranges = clock.source_range_to_edited(section.source_start_us, section.source_end_us);
        for (start, end) in merge(ranges, 0) {
            candidates.push(Candidate { start, end, index });
        }
    }
    // Overlaps (sections on different assets): by hand first, then the shorter.
    candidates.sort_by_key(|c| (track.sections[c.index].auto, c.end - c.start, c.start));
    let mut taken: Vec<(u64, u64)> = Vec::new();
    // (start, end, section index)
    let mut placed: Vec<(u64, u64, usize)> = Vec::new();
    for c in candidates {
        for (a, b) in subtract(&[(c.start, c.end)], &taken) {
            placed.push((a, b, c.index));
        }
        taken.push((c.start, c.end));
        taken = merge(taken, 0);
    }
    placed.retain(|&(a, b, _)| b - a >= MIN_SHOWN_US && a < duration_us);
    placed.sort_by_key(|p| p.0);
    // A short stretch of base between two sections: the first runs on.
    for i in 1..placed.len() {
        let next_start = placed[i].0;
        let previous = &mut placed[i - 1];
        if next_start > previous.1 && next_start - previous.1 < MIN_BASE_GAP_US {
            previous.1 = next_start;
        }
    }
    let shown = |index: usize| track.sections[index].layout.clone();
    // Pieces back to back that show the same thing play as one.
    let mut joined: Vec<(u64, u64, usize)> = Vec::with_capacity(placed.len());
    for piece in placed {
        match joined.last_mut() {
            Some(last) if last.1 == piece.0 && shown(last.2) == shown(piece.2) => {
                last.1 = piece.1
            }
            _ => joined.push(piece),
        }
    }
    let on_cut = |at: u64| {
        let i = cuts.partition_point(|&c| c + CUT_SNAP_US < at);
        cuts.get(i).is_some_and(|&c| c <= at + CUT_SNAP_US && c > 0)
    };
    let length = |index: usize, from: &Layout, to: &Layout, at: u64| -> u64 {
        let section = &track.sections[index];
        let ms = section.transition_ms.unwrap_or(track.transition_ms) as u64 * 1_000;
        if from == to {
            return 0;
        }
        match section.transition {
            Transition::Cut => 0,
            Transition::Move => ms,
            Transition::Auto if (from.is_single() && to.is_single()) || on_cut(at) => 0,
            Transition::Auto => ms,
        }
    };
    let mut pieces: Vec<Piece> = Vec::with_capacity(joined.len());
    for (i, &(start, end, index)) in joined.iter().enumerate() {
        // A section that shows the base keeps others away but changes nothing.
        let Some(layout) = shown(index) else {
            continue;
        };
        let before = (i > 0 && joined[i - 1].1 == start)
            .then(|| shown(joined[i - 1].2))
            .flatten();
        let meets_next = joined
            .get(i + 1)
            .is_some_and(|n| n.0 == end && shown(n.2).is_some());
        let from = before.unwrap_or_else(|| track.base.clone());
        let mut rise = length(index, &from, &layout, start);
        let mut fall = if meets_next {
            0
        } else {
            length(index, &layout, &track.base, end)
        };
        let half = (end - start) / 2;
        if rise + fall > end - start {
            rise = rise.min(half);
            fall = fall.min(half);
        }
        pieces.push(Piece {
            start_us: start,
            end_us: end,
            layout,
            section: track.sections[index].id.clone(),
            from,
            rise_us: rise,
            fall_us: fall,
            meets_next,
        });
    }
    Resolved {
        base: track.base.clone(),
        pieces,
        duration_us,
    }
}

fn ease(t: f64) -> f32 {
    crate::zoom::cubic_bezier_unit(t) as f32
}

impl Resolved {
    /// Only the base, everywhere.
    pub fn base_only(base: Layout, duration_us: u64) -> Self {
        Self {
            base,
            pieces: Vec::new(),
            duration_us,
        }
    }

    /// The piece playing at `edited_us`, if any.
    pub fn piece_at(&self, edited_us: u64) -> Option<&Piece> {
        let index = self.pieces.partition_point(|p| p.end_us <= edited_us);
        self.pieces.get(index).filter(|p| p.start_us <= edited_us)
    }

    pub fn frame_at(&self, edited_us: u64) -> Frame<'_> {
        let still = |layout| Frame {
            from: layout,
            to: layout,
            weight: 1.0,
        };
        let Some(piece) = self.piece_at(edited_us) else {
            return still(&self.base);
        };
        if piece.rise_us > 0 && edited_us < piece.start_us + piece.rise_us {
            let t = (edited_us - piece.start_us) as f64 / piece.rise_us as f64;
            return Frame {
                from: &piece.from,
                to: &piece.layout,
                weight: ease(t),
            };
        }
        if piece.fall_us > 0 && edited_us + piece.fall_us >= piece.end_us {
            let t = (piece.end_us - edited_us) as f64 / piece.fall_us as f64;
            return Frame {
                from: &self.base,
                to: &piece.layout,
                weight: ease(t),
            };
        }
        still(&piece.layout)
    }

    /// The layout mostly showing at `edited_us`.
    pub fn layout_at(&self, edited_us: u64) -> &Layout {
        let frame = self.frame_at(edited_us);
        if frame.weight >= 0.5 {
            frame.to
        } else {
            frame.from
        }
    }

    /// Timeline stretches, sorted and merged, where the layout showing matches `test`: the
    /// base's gaps included.
    pub fn ranges_where(&self, test: impl Fn(&Layout) -> bool) -> Vec<(u64, u64)> {
        let mut out = Vec::new();
        let mut at = 0;
        for piece in &self.pieces {
            if test(&self.base) && piece.start_us > at {
                out.push((at, piece.start_us));
            }
            if test(&piece.layout) {
                out.push((piece.start_us, piece.end_us));
            }
            at = piece.end_us;
        }
        if test(&self.base) && self.duration_us > at {
            out.push((at, self.duration_us));
        }
        merge(out, 0)
    }

    /// Whether any layout shown has the camera in it.
    pub fn shows_camera(&self) -> bool {
        self.base.shows_camera() || self.pieces.iter().any(|p| p.layout.shows_camera())
    }

    pub fn shows_screen(&self) -> bool {
        self.base.shows_screen() || self.pieces.iter().any(|p| p.layout.shows_screen())
    }

    /// Where the layout changes: (when, true for into a section).
    pub fn changes(&self) -> Vec<(u64, bool)> {
        let mut out = Vec::new();
        for piece in &self.pieces {
            out.push((piece.start_us, true));
            if !piece.meets_next {
                out.push((piece.end_us.saturating_sub(piece.fall_us), false));
            }
        }
        out
    }
}

/// The edges of `sequence`'s video clips, sorted: where the picture cuts.
pub fn cut_points(sequence: &crate::sequence::Sequence) -> Vec<u64> {
    let mut cuts: Vec<u64> = sequence
        .video_tracks()
        .flat_map(|t| t.clips.iter())
        .flat_map(|c| [c.start_us, c.end_us()])
        .collect();
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

/// `document`'s layouts as its timeline plays them.
pub fn of_document(document: &crate::project::revision::EditDocument) -> Resolved {
    resolve(
        &document.layouts,
        |media| {
            document
                .clock_asset(media)
                .map(|asset| document.asset_clock(asset))
        },
        &cut_points(&document.sequence),
        document.duration_us(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::{CameraOnly, LayoutSection, ScreenOnly};
    use crate::timeline::SourceInterval;

    const S: u64 = 1_000_000;
    const MS: u64 = 1_000;

    fn camera() -> Layout {
        Layout::CameraOnly(CameraOnly::default())
    }

    fn screen() -> Layout {
        Layout::ScreenOnly(ScreenOnly::default())
    }

    fn track(sections: Vec<LayoutSection>) -> LayoutTrack {
        LayoutTrack {
            sections,
            ..LayoutTrack::default()
        }
    }

    fn section(id: &str, a: u64, b: u64, layout: Option<Layout>) -> LayoutSection {
        LayoutSection::new(id.into(), None, a, b, layout)
    }

    fn whole(len: u64) -> impl Fn(Option<&str>) -> Option<TimelineMapper> {
        move |_| Some(TimelineMapper::new(vec![SourceInterval::new("r".into(), 0, len)]))
    }

    fn spans(resolved: &Resolved) -> Vec<(u64, u64, String)> {
        resolved
            .pieces
            .iter()
            .map(|p| (p.start_us, p.end_us, p.section.clone()))
            .collect()
    }

    #[test]
    fn sections_follow_their_content_through_a_ripple_cut() {
        let t = track(vec![section("a", 20 * S, 30 * S, Some(camera()))]);
        // 5-10 s of the recording was cut: everything after moves 5 s earlier.
        let cut = |_: Option<&str>| {
            Some(TimelineMapper::new(vec![
                SourceInterval::new("a".into(), 0, 5 * S),
                SourceInterval::new("b".into(), 10 * S, 60 * S),
            ]))
        };
        let resolved = resolve(&t, cut, &[0, 5 * S, 55 * S], 55 * S);
        assert_eq!(spans(&resolved), vec![(15 * S, 25 * S, "a".into())]);
        // Outside it, the base: never another default.
        assert_eq!(resolved.layout_at(10 * S), &Layout::default());
        assert_eq!(resolved.layout_at(20 * S), &camera());
    }

    #[test]
    fn short_slivers_are_absorbed_and_short_pieces_hidden() {
        let t = track(vec![
            section("a", 10 * S, 20 * S, Some(camera())),
            // 0.4 s of base before it, then the same layout again: one stretch.
            section("b", 20 * S + 400 * MS, 30 * S, Some(camera())),
            // Too short to show.
            section("c", 40 * S, 40 * S + 600 * MS, Some(screen())),
        ]);
        let resolved = resolve(&t, whole(60 * S), &[0, 60 * S], 60 * S);
        assert_eq!(spans(&resolved), vec![(10 * S, 30 * S, "a".into())]);
    }

    #[test]
    fn moves_play_inside_the_section_and_cuts_switch_at_once() {
        let mut t = track(vec![
            section("a", 10 * S, 20 * S, Some(camera())),
            section("b", 20 * S, 30 * S, Some(screen())),
        ]);
        let resolved = resolve(&t, whole(60 * S), &[0, 60 * S], 60 * S);
        // From the bubble to the camera: a move over the first 350 ms.
        let a = &resolved.pieces[0];
        assert_eq!((a.rise_us, a.fall_us, a.meets_next), (350 * MS, 0, true));
        let mid = resolved.frame_at(10 * S + 175 * MS);
        assert!(mid.weight > 0.3 && mid.weight < 0.7);
        assert_eq!(mid.from, &Layout::default());
        // Camera only to screen only is a cut; back to the bubble a move at the end.
        let b = &resolved.pieces[1];
        assert_eq!((b.rise_us, b.fall_us), (0, 350 * MS));
        assert_eq!(resolved.frame_at(20 * S).weight, 1.0);
        let going = resolved.frame_at(30 * S - 100 * MS);
        assert!(going.weight < 0.5 && going.from == &Layout::default());
        // A change on a cut in the video is a cut.
        let resolved = resolve(&t, whole(60 * S), &[0, 10 * S + 20 * MS, 60 * S], 60 * S);
        assert_eq!(resolved.pieces[0].rise_us, 0);
        // A section can insist on a move.
        t.sections[1].transition = Transition::Move;
        t.sections[1].transition_ms = Some(500);
        let resolved = resolve(&t, whole(60 * S), &[0, 60 * S], 60 * S);
        assert_eq!(resolved.pieces[1].rise_us, 500 * MS);
    }

    #[test]
    fn a_base_section_keeps_picked_ones_away() {
        let mut picked = section("p", 0, 60 * S, Some(camera()));
        picked.auto = true;
        picked.media = Some("other".into());
        let pin = section("pin", 20 * S, 30 * S, None);
        let resolved = resolve(&track(vec![picked, pin]), whole(60 * S), &[0], 60 * S);
        let shown: Vec<(u64, u64)> = resolved
            .pieces
            .iter()
            .map(|p| (p.start_us, p.end_us))
            .collect();
        assert_eq!(shown, vec![(0, 20 * S), (30 * S, 60 * S)]);
        assert_eq!(resolved.layout_at(25 * S), &Layout::default());
        assert_eq!(
            resolved.ranges_where(|l| l.camera_fills()),
            vec![(0, 20 * S), (30 * S, 60 * S)]
        );
    }
}
