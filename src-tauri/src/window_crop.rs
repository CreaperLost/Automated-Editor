//! Crop to the active window: when the window in front is clearly smaller than the recorded
//! display (a big or second monitor), the screen shows just that window with a little room
//! around it, and glides to the next window when the user switches.
//!
//! The plan is worked out from the recording's window telemetry (`telemetry/windows.jsonl`,
//! see `docs/window-telemetry.md`) in source time, so every frame of preview and export reads
//! the same crop. Quick app switches, popups, menus and tiny windows never move it. Zooms work
//! inside the crop (see [`zoom_inside`]), so their targets stay on the same pixels.
use crate::telemetry::windows::{WindowBox, WindowRole, WindowStream};
use serde::{Deserialize, Serialize};

/// (x, y, w, h), normalized to the recorded source.
pub type Rect = (f32, f32, f32, f32);

/// A window must stay in front this long to be cropped to; quicker switches are skipped.
pub const DEBOUNCE_US: u64 = 1_200_000;
/// How long the crop takes to glide to the next window.
pub const TRANSITION_US: u64 = 700_000;
/// The glide starts this much before the switch, so the new window is framed as it comes up.
const LEAD_US: u64 = 150_000;
/// Windows smaller than this (area, or either side, of the source) are popups in all but name.
const MIN_WINDOW_AREA: f64 = 0.04;
const MIN_WINDOW_SIDE: f64 = 0.12;
/// A window this big (area of the source), or this close to filling it, is shown whole.
const FULL_AREA: f64 = 0.72;
const FULL_SIDE: f64 = 0.9;
/// Boxes this close (each edge, of the source) are the same crop.
const SAME_EDGE: f64 = 0.02;
/// A crop wider than this share of the whole screen is not worth it.
const FULL_SHARE: f32 = 0.92;
/// The crop never enlarges the screen more than this.
const MAX_CROP_SCALE: f32 = 2.5;
/// A zoom inside the crop goes no deeper than this (or the zoom alone, if that is deeper).
const MAX_TOTAL_SCALE: f32 = 4.0;
pub const DEFAULT_PADDING_PCT: f32 = 3.0;
pub const MAX_PADDING_PCT: f32 = 15.0;
pub const MAX_OFF_RANGES: usize = 1_024;

/// A stretch of a recording (its own time) where the crop is off: the whole screen shows.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CropOffRange {
    /// The recording whose clock the times are on; `None` is the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    pub source_start_us: u64,
    pub source_end_us: u64,
}

#[derive(Clone, Debug, PartialEq)]
enum Target {
    Full,
    Window(f64, f64, f64, f64),
}

impl Target {
    fn same(&self, other: &Target) -> bool {
        match (self, other) {
            (Target::Full, Target::Full) => true,
            (Target::Window(ax, ay, aw, ah), Target::Window(bx, by, bw, bh)) => {
                (ax - bx).abs() < SAME_EDGE
                    && (ay - by).abs() < SAME_EDGE
                    && (ax + aw - bx - bw).abs() < SAME_EDGE
                    && (ay + ah - by - bh).abs() < SAME_EDGE
            }
            _ => false,
        }
    }
}

/// What a front window asks of the crop; `None` leaves it where it is (popups, menus, tiny
/// windows).
fn target_of(window: Option<&WindowBox>) -> Option<Target> {
    let Some(window) = window else {
        return Some(Target::Full);
    };
    if window.role.is_transient() {
        return None;
    }
    let area = window.w * window.h;
    if area < MIN_WINDOW_AREA || window.w < MIN_WINDOW_SIDE || window.h < MIN_WINDOW_SIDE {
        return None;
    }
    let full = window.maximized
        || window.role == WindowRole::Fullscreen
        || area >= FULL_AREA
        || (window.w >= FULL_SIDE && window.h >= FULL_SIDE);
    Some(if full {
        Target::Full
    } else {
        Target::Window(window.x, window.y, window.w, window.h)
    })
}

/// Joins neighbours that ask for the same crop.
fn collapse(segments: &mut Vec<(u64, Target)>) {
    segments.dedup_by(|next, kept| next.1.same(&kept.1));
}

/// Where the crop goes over a recording, from its front windows: (from time, target), with
/// stretches shorter than [`DEBOUNCE_US`] given to the one before.
fn debounced(windows: &WindowStream) -> Vec<(u64, Target)> {
    let mut segments = vec![(0u64, Target::Full)];
    for front in &windows.fronts {
        if let Some(target) = target_of(front.window.as_ref()) {
            segments.push((front.t_us, target));
        }
    }
    collapse(&mut segments);
    loop {
        let mut changed = false;
        let mut i = 0;
        while i < segments.len() {
            let Some(&(next, _)) = segments.get(i + 1) else {
                break;
            };
            if next - segments[i].0 < DEBOUNCE_US {
                if i == 0 {
                    // The very start: the next one begins with the recording.
                    segments.remove(0);
                    segments[0].0 = 0;
                } else {
                    segments.remove(i);
                }
                changed = true;
                collapse(&mut segments);
                continue;
            }
            i += 1;
        }
        if !changed {
            return segments;
        }
    }
}

/// `segments` with the whole screen inside each of `off` (source ranges), and what was there
/// before coming back after it.
fn with_off(segments: Vec<(u64, Target)>, off: &[(u64, u64)]) -> Vec<(u64, Target)> {
    let at = |segments: &[(u64, Target)], t: u64| {
        let i = segments.partition_point(|(start, _)| *start <= t);
        segments[i.saturating_sub(1)].1.clone()
    };
    let mut out = segments.clone();
    for &(start, end) in off.iter().filter(|(a, b)| b > a) {
        let after = at(&out, end);
        out.retain(|(t, _)| *t < start || *t > end);
        out.push((start, Target::Full));
        out.push((end, after));
        out.sort_by_key(|(t, _)| *t);
    }
    // A stretch starting at 0 is the first.
    if out.first().is_some_and(|(t, _)| *t > 0) {
        out.insert(0, (0, Target::Full));
    }
    collapse(&mut out);
    out
}

/// The crop for a window at (x, y, w, h): `padding_pct` of the source height around it (the
/// same in pixels across), widened to `base`'s shape so the screen keeps its size on the
/// canvas, and kept inside `base` (the layout's own crop). `None` when it is about the whole
/// of `base` anyway.
pub fn fit_window(
    window: (f64, f64, f64, f64),
    base: Rect,
    source_aspect: f32,
    padding_pct: f32,
) -> Option<Rect> {
    let (bx, by, bw, bh) = base;
    if bw <= 0.0 || bh <= 0.0 {
        return None;
    }
    let aspect = bw / bh;
    let pad_y = padding_pct.clamp(0.0, MAX_PADDING_PCT) / 100.0;
    let pad_x = pad_y / source_aspect.max(0.01);
    let (x, y, w, h) = (
        window.0 as f32,
        window.1 as f32,
        window.2 as f32,
        window.3 as f32,
    );
    let x0 = (x - pad_x).max(bx);
    let y0 = (y - pad_y).max(by);
    let x1 = (x + w + pad_x).min(bx + bw);
    let y1 = (y + h + pad_y).min(by + bh);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (mut w, mut h) = (x1 - x0, y1 - y0);
    if w / h < aspect {
        w = h * aspect;
    } else {
        h = w / aspect;
    }
    if w < bw / MAX_CROP_SCALE {
        w = bw / MAX_CROP_SCALE;
        h = w / aspect;
    }
    if w > bw {
        w = bw;
        h = bw / aspect;
    }
    if h > bh {
        h = bh;
        w = bh * aspect;
    }
    if w >= bw * FULL_SHARE {
        return None;
    }
    let x = (cx - w / 2.0).min(bx + bw - w).max(bx);
    let y = (cy - h / 2.0).min(by + bh - h).max(by);
    Some((x, y, w, h))
}

#[derive(Clone, Debug, PartialEq)]
struct Key {
    start_us: u64,
    from: Rect,
    to: Rect,
}

/// The crop over one recording's time.
#[derive(Clone, Debug, PartialEq)]
pub struct CropPlan {
    keys: Vec<Key>,
}

fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    let l = |p: f32, q: f32| p + (q - p) * t;
    (l(a.0, b.0), l(a.1, b.1), l(a.2, b.2), l(a.3, b.3))
}

impl CropPlan {
    /// The whole of `base` throughout.
    pub fn none(base: Rect) -> Self {
        Self {
            keys: vec![Key {
                start_us: 0,
                from: base,
                to: base,
            }],
        }
    }

    /// The crop from `windows`, off inside `off` (source ranges), within `base` (the layout's
    /// crop) of a source `source_aspect` wide per unit of height.
    pub fn plan(
        windows: &WindowStream,
        off: &[(u64, u64)],
        base: Rect,
        source_aspect: f32,
        padding_pct: f32,
    ) -> Self {
        let segments = with_off(debounced(windows), off);
        let mut rects: Vec<(u64, Rect)> = segments
            .into_iter()
            .map(|(t, target)| {
                let rect = match target {
                    Target::Full => None,
                    Target::Window(x, y, w, h) => {
                        fit_window((x, y, w, h), base, source_aspect, padding_pct)
                    }
                };
                (t, rect.unwrap_or(base))
            })
            .collect();
        rects.dedup_by(|next, kept| next.1 == kept.1);
        let mut plan = Self::none(rects.first().map_or(base, |r| r.1));
        for &(t, rect) in rects.iter().skip(1) {
            let start = t.saturating_sub(LEAD_US);
            let from = plan.rect_at(start);
            plan.keys.push(Key {
                start_us: start,
                from,
                to: rect,
            });
        }
        plan
    }

    /// The crop at `t_us` on the recording's clock.
    pub fn rect_at(&self, t_us: u64) -> Rect {
        let i = self.keys.partition_point(|k| k.start_us <= t_us).max(1) - 1;
        let key = &self.keys[i];
        let progress = t_us.saturating_sub(key.start_us) as f64 / TRANSITION_US as f64;
        if progress >= 1.0 {
            return key.to;
        }
        // Smootherstep: no jolt as it starts or settles.
        let t = progress.clamp(0.0, 1.0);
        let eased = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        lerp_rect(key.from, key.to, eased as f32)
    }

    /// Whether the crop ever leaves the whole of `base`.
    pub fn crops(&self, base: Rect) -> bool {
        self.keys.iter().any(|k| k.to != base || k.from != base)
    }

    /// Where the crop settles, (from time, rect), for display.
    pub fn stretches(&self) -> Vec<(u64, Rect)> {
        self.keys.iter().map(|k| (k.start_us, k.to)).collect()
    }
}

/// A zoom camera's view (`zoom`, full-source UV) inside the crop `crop`: its target stays on
/// the same point of the source, its size is relative to the crop, and it never goes deeper
/// than [`MAX_TOTAL_SCALE`] times `base` unless the zoom alone does.
pub fn zoom_inside(base: Rect, crop: Rect, zoom: Rect) -> Rect {
    let (x, y, w, h) = crate::render::zoom_within_crop(crop, zoom);
    let min_w = (base.2 / MAX_TOTAL_SCALE).min(zoom.2 * base.2);
    if w >= min_w || w <= 0.0 {
        return (x, y, w, h);
    }
    let k = (min_w / w).min(crop.2 / w).min(crop.3 / h);
    let (nw, nh) = (w * k, h * k);
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let nx = (cx - nw / 2.0).min(crop.0 + crop.2 - nw).max(crop.0);
    let ny = (cy - nh / 2.0).min(crop.1 + crop.3 - nh).max(crop.1);
    (nx, ny, nw, nh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::windows::FrontWindow;

    const BASE: Rect = (0.0, 0.0, 1.0, 1.0);

    fn window(id: &str, x: f64, y: f64, w: f64, h: f64) -> Option<WindowBox> {
        Some(WindowBox {
            id: id.into(),
            role: WindowRole::Window,
            x,
            y,
            w,
            h,
            maximized: false,
        })
    }

    fn stream(fronts: Vec<(u64, Option<WindowBox>)>) -> WindowStream {
        WindowStream {
            fronts: fronts
                .into_iter()
                .map(|(t_us, window)| FrontWindow { t_us, window })
                .collect(),
            ..Default::default()
        }
    }

    fn close(a: Rect, b: Rect) -> bool {
        (a.0 - b.0).abs() < 1e-4
            && (a.1 - b.1).abs() < 1e-4
            && (a.2 - b.2).abs() < 1e-4
            && (a.3 - b.3).abs() < 1e-4
    }

    #[test]
    fn a_small_window_is_cropped_to_with_padding_in_the_screens_shape() {
        let rect = fit_window((0.1, 0.1, 0.4, 0.4), BASE, 16.0 / 9.0, 3.0).unwrap();
        // The shape of the base (square in UV), padded: 0.4 + 2 * 3 %.
        assert!((rect.2 - rect.3).abs() < 1e-5);
        assert!((rect.3 - 0.46).abs() < 1e-5, "{rect:?}");
        // Centred on the window.
        assert!((rect.0 + rect.2 / 2.0 - 0.3).abs() < 1e-5);
        // A window that nearly fills the screen is not worth a crop.
        assert!(fit_window((0.0, 0.0, 0.97, 0.9), BASE, 16.0 / 9.0, 3.0).is_none());
        // Kept inside the layout's own crop.
        let base = (0.0, 0.0, 1.0, 0.9);
        let rect = fit_window((0.5, 0.5, 0.45, 0.45), base, 16.0 / 9.0, 3.0).unwrap();
        assert!(rect.1 + rect.3 <= 0.9 + 1e-6 && rect.0 + rect.2 <= 1.0 + 1e-6);
        assert!((rect.2 / rect.3 - 1.0 / 0.9).abs() < 1e-4);
    }

    #[test]
    fn quick_switches_popups_and_tiny_windows_never_move_the_crop() {
        let editor = window("editor", 0.05, 0.05, 0.5, 0.6);
        let mut menu = window("menu", 0.3, 0.3, 0.2, 0.3);
        menu.as_mut().unwrap().role = WindowRole::Menu;
        let plan = CropPlan::plan(
            &stream(vec![
                (0, editor.clone()),
                // Alt-tab to a browser and straight back.
                (5_000_000, window("browser", 0.4, 0.1, 0.5, 0.5)),
                (5_600_000, editor.clone()),
                // A menu and a tiny tool window.
                (8_000_000, menu),
                (9_000_000, window("color", 0.8, 0.8, 0.1, 0.1)),
            ]),
            &[],
            BASE,
            16.0 / 9.0,
            3.0,
        );
        let editor_rect = plan.rect_at(0);
        assert!(editor_rect.2 < 0.8, "{editor_rect:?}");
        for t in [4_000_000, 5_300_000, 6_000_000, 8_500_000, 12_000_000] {
            assert!(close(plan.rect_at(t), editor_rect), "moved at {t}");
        }
        assert_eq!(plan.stretches().len(), 1);
    }

    #[test]
    fn switching_windows_glides_and_a_big_window_shows_the_whole_screen() {
        let plan = CropPlan::plan(
            &stream(vec![
                (0, window("a", 0.0, 0.0, 0.5, 0.5)),
                (10_000_000, window("b", 0.5, 0.5, 0.5, 0.5)),
                (20_000_000, window("c", 0.0, 0.0, 0.95, 0.95)),
            ]),
            &[],
            BASE,
            16.0 / 9.0,
            3.0,
        );
        let a = plan.rect_at(9_000_000);
        let b = plan.rect_at(15_000_000);
        assert!(a.0 < 0.1 && b.0 > 0.4, "{a:?} {b:?}");
        // Half way through the glide it is between the two.
        let mid = plan.rect_at(10_000_000 - LEAD_US + TRANSITION_US / 2);
        assert!(mid.0 > a.0 + 0.05 && mid.0 < b.0 - 0.05, "{mid:?}");
        // It eases: the first frame of the glide has barely moved.
        let first = plan.rect_at(10_000_000 - LEAD_US + 10_000);
        assert!((first.0 - a.0).abs() < 0.001);
        assert!(close(plan.rect_at(25_000_000), BASE));
        assert!(plan.crops(BASE));
    }

    #[test]
    fn a_stretch_switched_off_shows_the_whole_screen_then_comes_back() {
        let windows = stream(vec![(0, window("a", 0.0, 0.0, 0.5, 0.5))]);
        let plan = CropPlan::plan(&windows, &[(4_000_000, 8_000_000)], BASE, 1.6, 3.0);
        let cropped = plan.rect_at(2_000_000);
        assert!(cropped.2 < 0.6);
        assert!(close(plan.rect_at(6_000_000), BASE));
        assert!(close(plan.rect_at(9_000_000), cropped));
        // No windows recorded: nothing ever crops.
        let none = CropPlan::plan(&WindowStream::default(), &[], BASE, 1.6, 3.0);
        assert!(!none.crops(BASE));
    }

    #[test]
    fn zooms_work_inside_the_crop_without_moving_their_target() {
        let crop = (0.5, 0.0, 0.5, 0.5);
        // A 2x zoom on (0.7, 0.25): half the crop, centred there.
        let zoom = (0.45, 0.0, 0.5, 0.5);
        let (x, y, w, h) = zoom_inside(BASE, crop, zoom);
        assert!((w - 0.25).abs() < 1e-6 && (h - 0.25).abs() < 1e-6);
        assert!((x + w / 2.0 - 0.7).abs() < 1e-6 && (y + h / 2.0 - 0.25).abs() < 1e-6);
        // Never deeper than 4x the whole screen when the zoom alone is not.
        let (_, _, w, _) = zoom_inside(BASE, (0.5, 0.0, 0.4, 0.4), (0.3, 0.3, 0.4, 0.4));
        assert!((w - 0.25).abs() < 1e-6, "{w}");
        // Without a crop, the zoom as it is.
        let zoom = (0.1, 0.1, 0.1, 0.1);
        assert!(close(zoom_inside(BASE, BASE, zoom), zoom));
    }
}
