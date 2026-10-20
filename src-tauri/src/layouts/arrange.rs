//! Where the screen and the camera go for a layout on a canvas, and how two of those blend
//! during a move. Pure geometry in canvas pixels; the evaluator turns it into layers and fills
//! in what changes every frame (the zoom, the face).
use super::{Bubble, CameraOnly, CameraShape, Layout, ScreenFit, ScreenOnly, SideBySide};
use crate::project::EditLayout;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn of((x, y, w, h): (u32, u32, u32, u32)) -> Self {
        Self::new(x as f32, y as f32, w as f32, h as f32)
    }

    pub fn lerp(self, to: Rect, t: f32) -> Rect {
        let l = |a: f32, b: f32| a + (b - a) * t;
        Rect::new(l(self.x, to.x), l(self.y, to.y), l(self.w, to.w), l(self.h, to.h))
    }

    /// Grown or shrunk by `factor` about its centre.
    pub fn scaled(self, factor: f32) -> Rect {
        let (w, h) = (self.w * factor, self.h * factor);
        Rect::new(self.x + (self.w - w) / 2.0, self.y + (self.h - h) / 2.0, w, h)
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Whole pixels inside a `canvas_w` x `canvas_h` canvas, at least one each way.
    pub fn pixels(&self, canvas_w: u32, canvas_h: u32) -> (u32, u32, u32, u32) {
        let x = self.x.round().clamp(0.0, canvas_w.saturating_sub(1) as f32) as u32;
        let y = self.y.round().clamp(0.0, canvas_h.saturating_sub(1) as f32) as u32;
        let w = (self.w.round().max(1.0) as u32).min(canvas_w - x);
        let h = (self.h.round().max(1.0) as u32).min(canvas_h - y);
        (x, y, w.max(1), h.max(1))
    }
}

/// How a camera's corners are cut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Corners {
    /// Rounded by this radius in canvas pixels; 0 is square.
    Round(f32),
    Circle,
    Squircle,
}

impl Corners {
    /// The rounded rectangle closest to it on `rect`.
    fn radius_on(self, rect: Rect) -> f32 {
        match self {
            Corners::Round(r) => r,
            Corners::Circle => rect.w.min(rect.h) / 2.0,
            // The shapes differ by a hair at the start of a move.
            Corners::Squircle => rect.w.min(rect.h) * 0.3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenSlot {
    pub rect: Rect,
    /// Cut to the slot's shape (following the zooms) rather than fitted inside it.
    pub fill: bool,
    pub radius: f32,
    /// How much of the style's screen shadow it has, 0 to 1.
    pub shadow: f32,
    pub opacity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraSlot {
    pub rect: Rect,
    pub corners: Corners,
    /// How much of the style's border and shadow it has, 0 to 1.
    pub border: f32,
    pub shadow: f32,
    pub opacity: f32,
    /// How much closer on the face, 1 or more.
    pub punch_in: f32,
    /// A bubble over the screen (the kind that hides while you work under it).
    pub bubble: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Arrangement {
    pub screen: Option<ScreenSlot>,
    pub camera: Option<CameraSlot>,
    /// Camera and screen stacked on a portrait canvas: the line between them, where a short's
    /// captions can sit.
    pub seam: Option<f32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    /// Style pixel values times this (see [`crate::render::layout_px_unit`]).
    pub unit: f32,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            unit: crate::render::layout_px_unit(width, height),
        }
    }

    fn rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width as f32, self.height as f32)
    }

    fn landscape(&self) -> bool {
        self.width >= self.height
    }

    fn short_side(&self) -> f32 {
        self.width.min(self.height) as f32
    }
}

/// The pictures there are to place, by their pixel size.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sources {
    pub screen: Option<(u32, u32)>,
    pub camera: Option<(u32, u32)>,
}

/// What `layout` becomes when a picture is missing: the screen alone without a camera, the
/// camera alone without a screen.
pub fn available(layout: &Layout, sources: Sources) -> Option<Layout> {
    let fit = match layout {
        Layout::Bubble(b) => b.screen,
        Layout::SideBySide(s) => s.screen,
        Layout::ScreenOnly(s) => s.screen,
        Layout::CameraOnly(_) => ScreenFit::Auto,
    };
    match (sources.screen.is_some(), sources.camera.is_some()) {
        (false, false) => None,
        (true, false) if layout.shows_camera() => {
            Some(Layout::ScreenOnly(ScreenOnly { screen: fit }))
        }
        (false, true) if layout.shows_screen() => Some(Layout::CameraOnly(CameraOnly::default())),
        _ => Some(layout.clone()),
    }
}

pub fn arrange(layout: &Layout, style: &EditLayout, canvas: Canvas, sources: Sources) -> Arrangement {
    let Some(layout) = available(layout, sources) else {
        return Arrangement::default();
    };
    let screen_src = sources.screen.unwrap_or((16, 9));
    let camera_src = sources.camera.unwrap_or((16, 9));
    let style_radius = style.corner_radius_px as f32 * canvas.unit;
    let style_shadow = if style.shadow_blur_px > 0 && style.shadow_opacity > 0.0 {
        1.0
    } else {
        0.0
    };
    // The screen in `area`: cut to it, or fitted inside it with the style's scale and corners.
    let screen_in = |area: Rect, fill: bool, fill_radius: f32| ScreenSlot {
        rect: if fill {
            area
        } else {
            fitted_screen(area, style, screen_src, canvas.unit)
        },
        fill,
        radius: if fill { fill_radius } else { style_radius },
        shadow: if fill { 0.0 } else { style_shadow },
        opacity: 1.0,
    };
    let full = canvas.rect();
    match &layout {
        Layout::ScreenOnly(s) => Arrangement {
            screen: Some(screen_in(full, s.screen.fills(canvas.width, canvas.height), 0.0)),
            ..Default::default()
        },
        Layout::Bubble(b) => Arrangement {
            screen: Some(screen_in(full, b.screen.fills(canvas.width, canvas.height), 0.0)),
            camera: Some(bubble_slot(b, style, canvas, camera_src)),
            seam: None,
        },
        Layout::CameraOnly(c) => {
            let frac = c.size_pct.clamp(super::MIN_CAMERA_ONLY_PCT, 100.0) / 100.0;
            let rect = if frac >= 1.0 { full } else { full.scaled(frac) };
            Arrangement {
                camera: Some(CameraSlot {
                    rect,
                    corners: Corners::Round(if frac < 1.0 { style_radius } else { 0.0 }),
                    border: 0.0,
                    shadow: 0.0,
                    opacity: 1.0,
                    punch_in: c.punch_in.max(1.0),
                    bubble: false,
                }),
                ..Default::default()
            }
        }
        Layout::SideBySide(s) => side_by_side(s, style, canvas, &screen_in, style_radius),
    }
}

fn side_by_side(
    s: &SideBySide,
    style: &EditLayout,
    canvas: Canvas,
    screen_in: &dyn Fn(Rect, bool, f32) -> ScreenSlot,
    style_radius: f32,
) -> Arrangement {
    let (w, h) = (canvas.width as f32, canvas.height as f32);
    let landscape = canvas.landscape();
    let side = if landscape { w } else { h };
    let cam = (side * s.camera_pct.clamp(super::MIN_SIDE_PCT, super::MAX_SIDE_PCT) / 100.0).round();
    let fill = s.screen.fills(canvas.width, canvas.height);
    // Along the split: (camera start, camera length, screen start, screen length).
    let (cam_at, cam_len, screen_at, screen_len) = if s.overlap {
        let margin = (canvas.short_side() * 0.04).round();
        let inner = (cam * 0.5).round();
        if s.camera_first {
            (margin, cam - margin, inner, side - inner)
        } else {
            (side - cam, cam - margin, 0.0, side - inner)
        }
    } else if s.camera_first {
        (0.0, cam, cam, side - cam)
    } else {
        (side - cam, cam, 0.0, side - cam)
    };
    let across_margin = if s.overlap {
        (canvas.short_side() * 0.04).round()
    } else {
        0.0
    };
    let (camera_rect, screen_area) = if landscape {
        (
            Rect::new(cam_at, across_margin, cam_len, h - 2.0 * across_margin),
            Rect::new(screen_at, 0.0, screen_len, h),
        )
    } else {
        (
            Rect::new(across_margin, cam_at, w - 2.0 * across_margin, cam_len),
            Rect::new(0.0, screen_at, w, screen_len),
        )
    };
    let has_border = style.webcam_border_width > 0;
    let camera = CameraSlot {
        rect: camera_rect,
        corners: Corners::Round(if s.overlap { style_radius } else { 0.0 }),
        border: if s.overlap && has_border { 1.0 } else { 0.0 },
        shadow: if s.overlap && style.webcam_shadow { 1.0 } else { 0.0 },
        opacity: 1.0,
        punch_in: 1.0,
        bubble: false,
    };
    let seam = (!landscape).then(|| {
        if s.camera_first {
            camera_rect.bottom()
        } else {
            camera_rect.y
        }
    });
    Arrangement {
        screen: Some(screen_in(screen_area, fill, if s.overlap { style_radius } else { 0.0 })),
        camera: Some(camera),
        seam,
    }
}

/// The screen fitted inside `area` as the style frames it: scaled, cropped and centred.
pub fn fitted_screen(area: Rect, style: &EditLayout, src: (u32, u32), unit: f32) -> Rect {
    let (x, y, w, h) = (
        area.x.round() as u32,
        area.y.round() as u32,
        area.w.round().max(1.0) as u32,
        area.h.round().max(1.0) as u32,
    );
    let (rx, ry, rw, rh) = crate::render::screen_layer_rect(w, h, style, src.0, src.1, unit);
    Rect::of((x + rx, y + ry, rw, rh))
}

/// A bubble's size on the canvas, for a camera picture of `src`.
pub fn bubble_size(b: &Bubble, canvas_w: u32, canvas_h: u32, src: (u32, u32)) -> (u32, u32) {
    let frac = b.size_pct.clamp(super::MIN_BUBBLE_PCT, super::MAX_BUBBLE_PCT) / 100.0;
    let target = (((canvas_w.min(canvas_h) as f32) * frac).round() as u32).max(8);
    let (src_w, src_h) = src;
    if src_w == 0 || src_h == 0 {
        return (target, target);
    }
    let (w, h) = if src_w >= src_h {
        let h = ((target as f64 * src_h as f64 / src_w as f64).round() as u32).max(8);
        (target.min(canvas_w).max(1), h.min(canvas_h).max(1))
    } else {
        let w = ((target as f64 * src_w as f64 / src_h as f64).round() as u32).max(8);
        (w.min(canvas_w).max(1), target.min(canvas_h).max(1))
    };
    if matches!(b.shape, CameraShape::Circle | CameraShape::Squircle) {
        let side = w.min(h).max(8);
        return (side.min(canvas_w).max(1), side.min(canvas_h).max(1));
    }
    (w, h)
}

/// Where a `bubble_w` x `bubble_h` bubble sits: its grid spot a small margin from the edges,
/// or its custom spot.
pub fn bubble_origin(
    b: &Bubble,
    canvas_w: u32,
    canvas_h: u32,
    bubble_w: u32,
    bubble_h: u32,
    unit: f32,
) -> (u32, u32) {
    let max_x = canvas_w.saturating_sub(bubble_w);
    let max_y = canvas_h.saturating_sub(bubble_h);
    let margin = ((8.0 * unit).round() as u32).min(max_x / 2).min(max_y / 2);
    let place = |cell: u8, max: u32| match cell {
        0 => margin.min(max),
        1 => max / 2,
        _ => max.saturating_sub(margin),
    };
    match b.spot.cell() {
        Some((col, row)) => (place(col, max_x), place(row, max_y)),
        None => {
            let x = (b.custom_x.clamp(0.0, 100.0) / 100.0 * max_x as f32).round() as u32;
            let y = (b.custom_y.clamp(0.0, 100.0) / 100.0 * max_y as f32).round() as u32;
            (x.min(max_x), y.min(max_y))
        }
    }
}

/// The bubble's corners: its shape, or a rectangle rounded by its roundness.
pub fn bubble_corners(b: &Bubble, bubble_w: u32, bubble_h: u32) -> Corners {
    match b.shape {
        CameraShape::Circle => Corners::Circle,
        CameraShape::Squircle => Corners::Squircle,
        CameraShape::Rect => Corners::Round(
            b.roundness_pct.clamp(0.0, super::MAX_ROUNDNESS_PCT) / 100.0
                * bubble_w.min(bubble_h) as f32,
        ),
    }
}

fn bubble_slot(b: &Bubble, style: &EditLayout, canvas: Canvas, src: (u32, u32)) -> CameraSlot {
    let (w, h) = bubble_size(b, canvas.width, canvas.height, src);
    let (x, y) = bubble_origin(b, canvas.width, canvas.height, w, h, canvas.unit);
    CameraSlot {
        rect: Rect::of((x, y, w, h)),
        corners: bubble_corners(b, w, h),
        border: if style.webcam_border_width > 0 { 1.0 } else { 0.0 },
        shadow: if style.webcam_shadow { 1.0 } else { 0.0 },
        opacity: 1.0,
        punch_in: 1.0,
        bubble: true,
    }
}

/// Where a picture comes from or goes to when only one side of a move has it: a little
/// smaller, in place, faded out.
const APPEAR_SCALE: f32 = 0.85;

/// `from` moving to `to`, `t` of the way.
pub fn blend(from: &Arrangement, to: &Arrangement, t: f32) -> Arrangement {
    let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 1.0 };
    if t <= 0.0 {
        return *from;
    }
    if t >= 1.0 {
        return *to;
    }
    let l = |a: f32, b: f32| a + (b - a) * t;
    let screen = match (from.screen, to.screen) {
        (Some(a), Some(b)) => Some(ScreenSlot {
            rect: a.rect.lerp(b.rect, t),
            fill: if t < 0.5 { a.fill } else { b.fill },
            radius: l(a.radius, b.radius),
            shadow: l(a.shadow, b.shadow),
            opacity: l(a.opacity, b.opacity),
        }),
        (Some(a), None) => Some(ScreenSlot {
            opacity: a.opacity * (1.0 - t),
            ..a
        }),
        (None, Some(b)) => Some(ScreenSlot {
            opacity: b.opacity * t,
            ..b
        }),
        (None, None) => None,
    };
    let camera = match (from.camera, to.camera) {
        (Some(a), Some(b)) => {
            let rect = a.rect.lerp(b.rect, t);
            let corners = if a.corners == b.corners {
                a.corners
            } else {
                Corners::Round(l(a.corners.radius_on(a.rect), b.corners.radius_on(b.rect)))
            };
            Some(CameraSlot {
                rect,
                corners,
                border: l(a.border, b.border),
                shadow: l(a.shadow, b.shadow),
                opacity: l(a.opacity, b.opacity),
                punch_in: l(a.punch_in, b.punch_in),
                bubble: if t < 0.5 { a.bubble } else { b.bubble },
            })
        }
        (Some(a), None) => Some(CameraSlot {
            rect: a.rect.scaled(l(1.0, APPEAR_SCALE)),
            opacity: a.opacity * (1.0 - t),
            ..a
        }),
        (None, Some(b)) => Some(CameraSlot {
            rect: b.rect.scaled(l(APPEAR_SCALE, 1.0)),
            opacity: b.opacity * t,
            ..b
        }),
        (None, None) => None,
    };
    let seam = match (from.seam, to.seam) {
        (Some(a), Some(b)) => Some(l(a, b)),
        (a, b) => {
            if t < 0.5 {
                a
            } else {
                b
            }
        }
    };
    Arrangement {
        screen,
        camera,
        seam,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::BubbleSpot;

    fn style() -> EditLayout {
        EditLayout::plain()
    }

    const BOTH: Sources = Sources {
        screen: Some((1920, 1080)),
        camera: Some((1280, 720)),
    };

    #[test]
    fn bubble_spots_cover_a_three_by_three_grid() {
        let canvas = Canvas::new(1920, 1080);
        let at = |spot| {
            let layout = Layout::Bubble(Bubble {
                spot,
                size_pct: 20.0,
                shape: CameraShape::Rect,
                ..Bubble::default()
            });
            let camera = arrange(&layout, &style(), canvas, BOTH).camera.unwrap();
            (camera.rect.x, camera.rect.y, camera.rect.w, camera.rect.h)
        };
        let (x, y, w, h) = at(BubbleSpot::Center);
        assert_eq!((w, h), (216.0, 122.0));
        assert_eq!((x, y), ((1920.0 - w) / 2.0, ((1080.0 - h) / 2.0).floor()));
        let (x, y, ..) = at(BubbleSpot::TopLeft);
        assert_eq!((x, y), (8.0, 8.0));
        let (x, y, ..) = at(BubbleSpot::Right);
        assert_eq!((x, y), (1920.0 - w - 8.0, ((1080.0 - h) / 2.0).floor()));
    }

    #[test]
    fn side_by_side_splits_across_on_landscape_and_down_on_portrait() {
        let layout = Layout::SideBySide(SideBySide {
            camera_pct: 30.0,
            ..SideBySide::default()
        });
        let wide = arrange(&layout, &style(), Canvas::new(1920, 1080), BOTH);
        let camera = wide.camera.unwrap();
        assert_eq!(camera.rect, Rect::new(0.0, 0.0, 576.0, 1080.0));
        let screen = wide.screen.unwrap();
        // Fitted in the rest, letterboxed: the whole 16:9 screen in a 1344 px wide part.
        assert!(!screen.fill);
        assert_eq!((screen.rect.x, screen.rect.w), (576.0, 1344.0));
        assert!(screen.rect.h < 1080.0);
        assert_eq!(wide.seam, None);
        let tall = arrange(&layout, &style(), Canvas::new(1080, 1920), BOTH);
        assert_eq!(tall.camera.unwrap().rect, Rect::new(0.0, 0.0, 1080.0, 576.0));
        let screen = tall.screen.unwrap();
        assert!(screen.fill);
        assert_eq!(screen.rect, Rect::new(0.0, 576.0, 1080.0, 1344.0));
        assert_eq!(tall.seam, Some(576.0));
    }

    #[test]
    fn missing_pictures_resolve_down() {
        let layout = Layout::SideBySide(SideBySide::default());
        let canvas = Canvas::new(1920, 1080);
        let no_camera = Sources {
            camera: None,
            ..BOTH
        };
        let only_screen = arrange(&layout, &style(), canvas, no_camera);
        assert!(only_screen.camera.is_none());
        assert_eq!(only_screen.screen.unwrap().rect.w, 1920.0);
        let no_screen = Sources {
            screen: None,
            ..BOTH
        };
        let only_camera = arrange(&layout, &style(), canvas, no_screen);
        assert!(only_camera.screen.is_none());
        assert_eq!(only_camera.camera.unwrap().rect, Rect::new(0.0, 0.0, 1920.0, 1080.0));
    }

    #[test]
    fn a_move_blends_rects_and_fades_what_only_one_side_has() {
        let canvas = Canvas::new(1920, 1080);
        let bubble = arrange(&Layout::default(), &style(), canvas, BOTH);
        let camera = arrange(&Layout::CameraOnly(CameraOnly::default()), &style(), canvas, BOTH);
        assert_eq!(blend(&bubble, &camera, 0.0), bubble);
        assert_eq!(blend(&bubble, &camera, 1.0), camera);
        let mid = blend(&bubble, &camera, 0.5);
        let (a, b, m) = (
            bubble.camera.unwrap().rect,
            camera.camera.unwrap().rect,
            mid.camera.unwrap().rect,
        );
        assert!(m.w > a.w && m.w < b.w);
        assert!(matches!(mid.camera.unwrap().corners, Corners::Round(_)));
        // The screen fades out under the growing camera.
        assert_eq!(mid.screen.unwrap().opacity, 0.5);
        // Coming in from nothing, the camera grows from a little smaller, faded.
        let screen_only = arrange(&Layout::ScreenOnly(ScreenOnly::default()), &style(), canvas, BOTH);
        let appearing = blend(&screen_only, &bubble, 0.0001).camera.unwrap();
        assert!(appearing.opacity < 0.01 && appearing.rect.w < a.w);
    }
}
