//! Hide the webcam bubble while you work under it: when a click, a drag, scrolling, a pointer
//! resting or the typing caret falls where the bubble sits on the canvas (after layout, window
//! crop and zoom), the bubble fades out for that stretch and back in afterwards. It never moves
//! to another corner.
//!
//! Stretches are found once per edit in timeline time from the recording's telemetry, with
//! hysteresis (activity near the bubble keeps it hidden, only activity on it hides it) and
//! minimum hidden and visible times so it never flickers. Preview and export read the same
//! stretches.
use crate::telemetry::activity::RecordingActivity;

/// How long the bubble takes to fade out or back in.
pub const FADE_US: u64 = 250_000;
/// Gone this much before the activity starts (on top of the fade).
const LEAD_US: u64 = 100_000;
/// Stays hidden this long after the activity ends.
const HOLD_US: u64 = 700_000;
/// The bubble comes back only for gaps at least this long; shorter ones stay hidden.
const MIN_VISIBLE_US: u64 = 1_500_000;
/// Hidden at least this long once it hides (fades not counted).
const MIN_HIDDEN_US: u64 = 1_200_000;
/// Activity this close to the bubble (share of the canvas short side) hides it...
pub const ENTER_MARGIN: f32 = 0.015;
/// ...and activity this close keeps it hidden.
pub const STAY_MARGIN: f32 = 0.06;
/// A pointer resting within this (share of the source) at least this long counts.
const REST_RADIUS: f64 = 0.008;
const REST_MIN_US: u64 = 700_000;
/// Stretches of activity (a rest, typing) are checked this often, since a zoom can move them.
const SPAN_STEP_US: u64 = 150_000;

/// Activity at one place on the recorded source: (start, end, x, y), source time.
pub type Probe = (u64, u64, f64, f64);

/// Where the hands were, from a recording's activity: presses, the pointer through drags,
/// scrolls, rests, and the caret while typing.
pub fn probes(activity: &RecordingActivity) -> Vec<Probe> {
    let mut out: Vec<Probe> = activity
        .presses
        .iter()
        .map(|p| (p.t_us, p.t_us, p.x, p.y))
        .collect();
    for &(start, end) in &activity.drags {
        let from = activity.pointer.partition_point(|p| p.0 < start);
        for &(t, x, y) in activity.pointer[from..].iter().take_while(|p| p.0 <= end) {
            out.push((t, t, x, y));
        }
    }
    for &(t, at) in &activity.scrolls {
        if let Some((x, y)) = at {
            out.push((t, t, x, y));
        }
    }
    out.extend(activity.rests(REST_RADIUS, REST_MIN_US));
    for run in &activity.typing {
        let Some(start) = run.caret_start else {
            continue;
        };
        let end = run.caret_end.unwrap_or(start);
        let middle = run.start_us + (run.end_us - run.start_us) / 2;
        out.push((run.start_us, middle, start.norm_x, start.norm_y));
        out.push((middle, run.end_us, end.norm_x, end.norm_y));
    }
    out.sort_by_key(|p| (p.0, p.1));
    out
}

/// The source stretches where `probes` fall inside the bubble: `inside(t, x, y)` says whether
/// a point of the source at time `t` lands on it.
pub fn hits(probes: &[Probe], inside: impl Fn(u64, f64, f64) -> bool) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    for &(start, end, x, y) in probes {
        let mut open: Option<(u64, u64)> = None;
        let mut t = start;
        loop {
            if inside(t, x, y) {
                open = Some(open.map_or((t, t), |(a, _)| (a, t)));
            } else if let Some(hit) = open.take() {
                out.push(hit);
            }
            if t >= end {
                break;
            }
            t = (t + SPAN_STEP_US).min(end);
        }
        out.extend(open);
    }
    out.sort_unstable();
    out
}

fn merge(mut ranges: Vec<(u64, u64)>, gap: u64) -> Vec<(u64, u64)> {
    ranges.sort_unstable();
    let mut out: Vec<(u64, u64)> = Vec::new();
    for (a, b) in ranges {
        match out.last_mut() {
            Some(last) if a <= last.1.saturating_add(gap) => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

fn padded(ranges: &[(u64, u64)]) -> Vec<(u64, u64)> {
    ranges
        .iter()
        .map(|&(a, b)| {
            (
                a.saturating_sub(FADE_US + LEAD_US),
                b.saturating_add(HOLD_US + FADE_US),
            )
        })
        .collect()
}

/// When the bubble is hidden (fades included), timeline time: from activity on the bubble
/// (`enter`) and near it (`stay`), both timeline stretches.
pub fn hide_ranges(enter: &[(u64, u64)], stay: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let mut ranges = merge(padded(enter), MIN_VISIBLE_US);
    // Activity near the bubble keeps a hidden stretch going, but never starts one.
    let near = merge(padded(stay), 0);
    loop {
        let touching: Vec<(u64, u64)> = near
            .iter()
            .filter(|&&(a, b)| {
                ranges.iter().any(|&(c, d)| {
                    a <= d.saturating_add(MIN_VISIBLE_US) && c <= b.saturating_add(MIN_VISIBLE_US)
                })
            })
            .copied()
            .collect();
        let grown = merge(
            ranges.iter().copied().chain(touching).collect(),
            MIN_VISIBLE_US,
        );
        if grown == ranges {
            break;
        }
        ranges = grown;
    }
    let shortest = MIN_HIDDEN_US + 2 * FADE_US;
    merge(
        ranges
            .into_iter()
            .map(|(a, b)| (a, b.max(a + shortest)))
            .collect(),
        MIN_VISIBLE_US,
    )
}

/// How hidden the bubble is at `edited_us`: 0 shown, 1 gone, easing over [`FADE_US`] at each
/// end of a stretch.
pub fn hidden_weight(ranges: &[(u64, u64)], edited_us: u64) -> f32 {
    crate::webcam_focus::focus_weight(ranges, edited_us, FADE_US)
}

/// Whether the canvas point `(px, py)` is on the bubble `rect` (x, y, w, h) grown by `margin`
/// canvas pixels each way.
pub fn on_bubble(rect: (u32, u32, u32, u32), margin: f32, px: f32, py: f32) -> bool {
    let (x, y, w, h) = (rect.0 as f32, rect.1 as f32, rect.2 as f32, rect.3 as f32);
    px >= x - margin && px <= x + w + margin && py >= y - margin && py <= y + h + margin
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::activity::Press;

    const S: u64 = 1_000_000;

    #[test]
    fn one_click_under_the_bubble_hides_it_long_enough_to_read() {
        let ranges = hide_ranges(&[(10 * S, 10 * S)], &[]);
        assert_eq!(ranges.len(), 1);
        let (a, b) = ranges[0];
        // Gone by the click, and back only after the minimum time.
        assert!(hidden_weight(&ranges, 10 * S) == 1.0);
        assert!(a + FADE_US <= 10 * S - LEAD_US);
        assert!(b - a >= MIN_HIDDEN_US + 2 * FADE_US);
        assert_eq!(hidden_weight(&ranges, b + 1), 0.0);
        // Fades rather than pops.
        let w = hidden_weight(&ranges, a + FADE_US / 2);
        assert!(w > 0.1 && w < 0.9, "{w}");
    }

    #[test]
    fn nearby_stretches_merge_so_the_bubble_does_not_flicker() {
        let ranges = hide_ranges(&[(10 * S, 10 * S), (11 * S, 11 * S), (12 * S, 12 * S)], &[]);
        assert_eq!(ranges.len(), 1, "{ranges:?}");
        // Far apart: two stretches.
        let apart = hide_ranges(&[(10 * S, 10 * S), (20 * S, 20 * S)], &[]);
        assert_eq!(apart.len(), 2);
    }

    #[test]
    fn activity_near_the_bubble_keeps_it_hidden_but_never_hides_it() {
        // Working just beside it after a click on it: stays hidden through 14 s.
        let kept = hide_ranges(&[(10 * S, 10 * S)], &[(11 * S, 14 * S)]);
        assert_eq!(kept.len(), 1);
        assert_eq!(hidden_weight(&kept, 14 * S), 1.0);
        // Near it with nothing on it: shown.
        assert!(hide_ranges(&[], &[(11 * S, 14 * S)]).is_empty());
        // Near it, much later: not joined.
        let later = hide_ranges(&[(10 * S, 10 * S)], &[(30 * S, 31 * S)]);
        assert_eq!(later.len(), 1);
        assert_eq!(hidden_weight(&later, 30 * S), 0.0);
    }

    #[test]
    fn hits_follow_where_the_view_puts_the_activity() {
        let probes = vec![
            (0, 0, 0.9, 0.9),
            (S, 3 * S, 0.9, 0.9),
            (5 * S, 5 * S, 0.1, 0.1),
        ];
        // The bubble covers the bottom-right corner until 2 s (a zoom moves it away then).
        let hits = hits(&probes, |t, x, y| t < 2 * S && x > 0.8 && y > 0.8);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0], (0, 0));
        assert_eq!(hits[1].0, S);
        assert!(hits[1].1 < 2 * S && hits[1].1 >= 2 * S - SPAN_STEP_US);
    }

    #[test]
    fn probes_cover_presses_drags_scrolls_rests_and_the_caret() {
        use crate::telemetry::keys::{Caret, TypingRun};
        let activity = RecordingActivity {
            presses: vec![Press {
                t_us: S,
                x: 0.2,
                y: 0.2,
                button: Some(0),
            }],
            drags: vec![(S, 2 * S)],
            scrolls: vec![(3 * S, Some((0.4, 0.4))), (3 * S, None)],
            pointer: vec![(S, 0.2, 0.2), (S + S / 2, 0.3, 0.3), (2 * S, 0.5, 0.5)],
            typing: vec![TypingRun {
                start_us: 6 * S,
                end_us: 8 * S,
                keys: 12,
                caret_start: Some(Caret {
                    norm_x: 0.7,
                    norm_y: 0.7,
                }),
                caret_end: None,
            }],
        };
        let probes = probes(&activity);
        assert!(probes.contains(&(S, S, 0.2, 0.2)));
        assert!(probes.contains(&(S + S / 2, S + S / 2, 0.3, 0.3)));
        assert!(probes.contains(&(3 * S, 3 * S, 0.4, 0.4)));
        assert!(probes.contains(&(6 * S, 7 * S, 0.7, 0.7)));
        assert!(probes.contains(&(7 * S, 8 * S, 0.7, 0.7)));
        // The last pointer sample rests until the end of what was seen: too short to count.
        assert!(!probes.iter().any(|p| p.1 > p.0 && p.2 == 0.5));
    }

    #[test]
    fn the_bubble_test_grows_by_the_margin() {
        let rect = (100, 100, 50, 50);
        assert!(on_bubble(rect, 0.0, 120.0, 120.0));
        assert!(!on_bubble(rect, 0.0, 95.0, 120.0));
        assert!(on_bubble(rect, 10.0, 95.0, 120.0));
    }
}
