//! Where a screen cut down to a narrow view looks (DLV-5): a 16:9 recording filling a 9:16
//! frame shows about a third of its width, so the view has to go where the work is. Planned
//! once per recording from its telemetry, the way a camera operator would: it holds still
//! while the work stays well inside the view, moves (eased, briefly) when it leaves, gets there
//! a moment before a click or typing, and holds at least `HOLD_US` after each move so what is
//! on screen can be read. Clicks and the typing caret count before the pointer.
use crate::telemetry::activity::RecordingActivity;

/// The view is there this long before the action it moves for.
const LEAD_US: u64 = 600_000;
/// After a move the view holds at least this long, unless the action leaves it altogether.
const HOLD_US: u64 = 1_200_000;
/// The pointer has to stay out of the view's middle this long before the view follows it.
const DWELL_US: u64 = 250_000;
/// Of the view, the share either side the action may reach before the view moves.
const EDGE: f64 = 0.18;
/// A move takes this long, longer for a longer way (in views), up to `MAX_MOVE_US`.
const MIN_MOVE_US: u64 = 300_000;
const MAX_MOVE_US: u64 = 700_000;
/// The pointer has landed once it stays within this (share of the source) for `LAND_US`; the
/// view goes where it lands, never where it is passing through.
const LAND_RADIUS: f64 = 0.02;
const LAND_US: u64 = 150_000;
/// How far ahead to look for where the pointer lands.
const LAND_AHEAD_US: u64 = 1_000_000;
/// A click or the caret counts as where the work is for this long, unless the pointer goes
/// well away from it.
const STRONG_US: u64 = 1_500_000;
const STEP_US: u64 = 50_000;

/// The view's centre over time, on the recording's clock in source-normalised coordinates:
/// holds between moves, each move eased from one key to the next.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewPath {
    /// (time, x, y), in time order. Two keys with the same place are a hold.
    pub keys: Vec<(u64, f32, f32)>,
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl ViewPath {
    /// The view's centre at `t_us`.
    pub fn at(&self, t_us: u64) -> Option<(f32, f32)> {
        let first = self.keys.first()?;
        if t_us <= first.0 {
            return Some((first.1, first.2));
        }
        let i = self.keys.partition_point(|k| k.0 <= t_us);
        let a = self.keys[i - 1];
        let Some(&b) = self.keys.get(i) else {
            return Some((a.1, a.2));
        };
        let k = ease((t_us - a.0) as f32 / (b.0 - a.0).max(1) as f32);
        Some((a.1 + (b.1 - a.1) * k, a.2 + (b.2 - a.2) * k))
    }

    /// How many moves it makes.
    pub fn moves(&self) -> usize {
        self.keys
            .windows(2)
            .filter(|w| w[0].1 != w[1].1 || w[0].2 != w[1].2)
            .count()
    }
}

/// Where the pointer is at `t`, or where it lands if it is moving then.
fn landing(pointer: &[(u64, f64, f64)], t: u64) -> Option<(f64, f64)> {
    let first = pointer.partition_point(|p| p.0 <= t).checked_sub(1)?;
    let mut i = first;
    while i < pointer.len() && pointer[i].0 <= t + LAND_AHEAD_US {
        let (t0, x0, y0) = pointer[i];
        let mut j = i + 1;
        let mut still = true;
        while j < pointer.len() && pointer[j].0 <= t0 + LAND_US {
            if (pointer[j].1 - x0).abs() > LAND_RADIUS || (pointer[j].2 - y0).abs() > LAND_RADIUS {
                still = false;
                break;
            }
            j += 1;
        }
        // The last sample is where it ends up.
        if still {
            return Some((x0, y0));
        }
        i += 1;
    }
    let last = pointer[(i - 1).max(first)];
    Some((last.1, last.2))
}

/// Where the work is at `t`: the caret while typing, a click for a while after it (unless
/// the pointer has gone well away), else where the pointer lands. `strong` when it is a click
/// or the caret.
fn work_at(activity: &RecordingActivity, t: u64, window: (f64, f64)) -> Option<(f64, f64, bool)> {
    if let Some(run) = activity
        .typing
        .iter()
        .find(|r| r.start_us <= t && t < r.end_us + LEAD_US)
    {
        if let Some(caret) = run.caret_end.as_ref().or(run.caret_start.as_ref()) {
            return Some((caret.norm_x, caret.norm_y, true));
        }
    }
    let pointer = landing(&activity.pointer, t).map(|(x, y)| (t, x, y));
    let press = {
        let i = activity.presses.partition_point(|p| p.t_us <= t);
        i.checked_sub(1)
            .map(|i| activity.presses[i])
            .filter(|p| t - p.t_us < STRONG_US)
    };
    match (press, pointer) {
        (Some(p), Some((_, x, y)))
            if (x - p.x).abs() < window.0 / 2.0 && (y - p.y).abs() < window.1 / 2.0 =>
        {
            Some((p.x, p.y, true))
        }
        (Some(p), None) => Some((p.x, p.y, true)),
        (_, Some((_, x, y))) => Some((x, y, false)),
        (None, None) => None,
    }
}

/// The view's path through `activity` for a view `window` wide and tall (shares of the
/// source) that must stay inside `crop` (x, y, w, h, shares of the source). Empty when the
/// view already shows the whole crop or nothing was recorded.
pub fn plan(
    activity: &RecordingActivity,
    window: (f64, f64),
    crop: (f64, f64, f64, f64),
) -> ViewPath {
    let (cx, cy, cw, ch) = crop;
    let narrow = (window.0 < cw - 1e-3, window.1 < ch - 1e-3);
    if !narrow.0 && !narrow.1 {
        return ViewPath::default();
    }
    let ends = [
        activity.pointer.first().map(|p| p.0),
        activity.presses.first().map(|p| p.t_us),
        activity.typing.first().map(|r| r.start_us),
    ];
    let Some(start) = ends.iter().flatten().min().copied() else {
        return ViewPath::default();
    };
    let end = [
        activity.pointer.last().map(|p| p.0),
        activity.presses.last().map(|p| p.t_us),
        activity.typing.last().map(|r| r.end_us),
    ]
    .iter()
    .flatten()
    .max()
    .copied()
    .unwrap_or(start);
    // The centre may go only where the view stays inside the crop.
    let fit = |v: f64, c0: f64, cl: f64, w: f64| {
        if w >= cl {
            c0 + cl / 2.0
        } else {
            v.clamp(c0 + w / 2.0, c0 + cl - w / 2.0)
        }
    };
    let place = |x: f64, y: f64| (fit(x, cx, cw, window.0), fit(y, cy, ch, window.1));
    let Some((x0, y0, _)) = work_at(activity, start + LEAD_US, window) else {
        return ViewPath::default();
    };
    let mut at = place(x0, y0);
    let mut keys = vec![(start, at.0 as f32, at.1 as f32)];
    let mut busy_until = start;
    let mut out_since: Option<u64> = None;
    let mut t = start;
    while t <= end {
        if t < busy_until {
            t += STEP_US;
            continue;
        }
        let Some((x, y, strong)) = work_at(activity, t + LEAD_US, window) else {
            t += STEP_US;
            continue;
        };
        let (dx, dy) = ((x - at.0).abs(), (y - at.1).abs());
        let inside = |d: f64, w: f64, narrow: bool| !narrow || d <= w / 2.0 * (1.0 - 2.0 * EDGE);
        let in_view = |d: f64, w: f64, narrow: bool| !narrow || d <= w / 2.0;
        if inside(dx, window.0, narrow.0) && inside(dy, window.1, narrow.1) {
            out_since = None;
            t += STEP_US;
            continue;
        }
        let since = *out_since.get_or_insert(t);
        let gone = !(in_view(dx, window.0, narrow.0) && in_view(dy, window.1, narrow.1));
        if !strong && !gone && t - since < DWELL_US {
            t += STEP_US;
            continue;
        }
        let to = place(x, y);
        let way = ((to.0 - at.0).abs() / window.0).max((to.1 - at.1).abs() / window.1);
        if way < 1e-3 {
            // Already as close as the crop lets it go.
            out_since = None;
            t += STEP_US;
            continue;
        }
        let length = (MIN_MOVE_US as f64 + way * 300_000.0).min(MAX_MOVE_US as f64) as u64;
        if keys.last().is_some_and(|k| k.0 < t) {
            keys.push((t, at.0 as f32, at.1 as f32));
        }
        keys.push((t + length, to.0 as f32, to.1 as f32));
        at = to;
        busy_until = t + length + HOLD_US;
        out_since = None;
        t += STEP_US;
    }
    ViewPath { keys }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::activity::Press;
    use crate::telemetry::keys::{Caret, TypingRun};

    const S: u64 = 1_000_000;
    /// A 9:16 view of a 16:9 screen: about a third of the width, all of the height.
    const WINDOW: (f64, f64) = (0.316, 1.0);
    const CROP: (f64, f64, f64, f64) = (0.0, 0.0, 1.0, 1.0);

    /// The pointer moving from `a` to `b` (x only, y 0.5) over `from`..`to`, sampled every 20 ms.
    fn glide(out: &mut Vec<(u64, f64, f64)>, from: u64, to: u64, a: f64, b: f64) {
        let mut t = from;
        while t <= to {
            let k = (t - from) as f64 / (to - from).max(1) as f64;
            out.push((t, a + (b - a) * k, 0.5));
            t += 20_000;
        }
    }

    #[test]
    fn small_moves_inside_the_view_leave_it_still() {
        let mut pointer = Vec::new();
        glide(&mut pointer, 0, 5 * S, 0.45, 0.55);
        let activity = RecordingActivity {
            pointer,
            ..Default::default()
        };
        let path = plan(&activity, WINDOW, CROP);
        assert_eq!(path.moves(), 0);
        // Where the pointer was as it started, all the way through.
        let (x, y) = path.at(S).unwrap();
        assert!((x - 0.46).abs() < 0.01 && y == 0.5, "{x} {y}");
        assert_eq!(path.at(4 * S), Some((x, y)));
    }

    #[test]
    fn the_view_is_on_a_click_before_it_happens_and_holds_to_read() {
        // Resting left, then a quick trip right to click at 4 s, then straight back left.
        let mut pointer = Vec::new();
        glide(&mut pointer, 0, 3_600_000, 0.15, 0.15);
        glide(&mut pointer, 3_600_000, 3_900_000, 0.15, 0.85);
        glide(&mut pointer, 3_900_000, 4_300_000, 0.85, 0.85);
        glide(&mut pointer, 4_300_000, 4_500_000, 0.85, 0.15);
        glide(&mut pointer, 4_500_000, 9 * S, 0.15, 0.15);
        let activity = RecordingActivity {
            pointer,
            presses: vec![Press {
                t_us: 4 * S,
                x: 0.85,
                y: 0.5,
                button: Some(0),
            }],
            ..Default::default()
        };
        let path = plan(&activity, WINDOW, CROP);
        let x_at = |t| path.at(t).unwrap().0 as f64;
        // Kept inside the frame at the left, then on the click as it happens.
        assert!((x_at(S) - WINDOW.0 / 2.0).abs() < 1e-3, "{}", x_at(S));
        assert!(
            (x_at(4 * S) - (1.0 - WINDOW.0 / 2.0)).abs() < 0.02,
            "{}",
            x_at(4 * S)
        );
        // The pointer is back left at 4.5 s, but the view holds on what was clicked.
        assert!(x_at(4_800_000) > 0.8, "{}", x_at(4_800_000));
        // Then it goes back and stays: two moves in all.
        assert!(x_at(8 * S) < 0.2, "{}", x_at(8 * S));
        assert_eq!(path.moves(), 2);
        // Every move is eased in under a second.
        for w in path.keys.windows(2).filter(|w| w[0].1 != w[1].1) {
            assert!(w[1].0 - w[0].0 <= MAX_MOVE_US);
        }
    }

    #[test]
    fn typing_follows_the_caret_and_a_wide_view_never_moves() {
        let mut pointer = Vec::new();
        glide(&mut pointer, 0, 6 * S, 0.2, 0.2);
        let activity = RecordingActivity {
            pointer,
            typing: vec![TypingRun {
                start_us: 2 * S,
                end_us: 4 * S,
                keys: 20,
                caret_start: Some(Caret {
                    norm_x: 0.7,
                    norm_y: 0.5,
                }),
                caret_end: Some(Caret {
                    norm_x: 0.75,
                    norm_y: 0.5,
                }),
            }],
            ..Default::default()
        };
        let path = plan(&activity, WINDOW, CROP);
        let x = path.at(3 * S).unwrap().0;
        assert!((x - 0.75).abs() < 0.01, "on the caret while typing: {x}");
        // A view that shows the whole screen has nothing to plan.
        assert!(plan(&activity, (1.0, 1.0), CROP).keys.is_empty());
        assert!(plan(&RecordingActivity::default(), WINDOW, CROP)
            .keys
            .is_empty());
    }
}
