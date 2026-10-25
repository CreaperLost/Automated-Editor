//! Tutorial effects on the screen (FX-2): blur or hide a part of it (API keys, emails,
//! balances), a spotlight that dims all but a part, a magnifier, and dimming or blurring
//! behind a graphic.
//!
//! A screen effect is a rectangle of a screen recording's own picture (0..1 of its frame) for a
//! stretch of its own clock, like chapters and shorts are anchored in an asset's time. Crops,
//! zooms and layouts change how the picture is drawn, and cuts change what plays, but never
//! what a moment of the recording shows, so the effect stays on what it covers in the preview
//! and the export alike (the one drawing path, `export::SceneEvaluator::scene_at`). Dimming
//! and blurring behind a graphic follow its graphic clip instead.
//!
//! A blur or hide is drawn fully for its whole stretch: it never fades, so nothing it covers
//! shows for even a frame.
use crate::project::revision::EditDocument;
use crate::sequence::Role;
use serde::{Deserialize, Serialize};

/// Spotlights and magnifiers ease in and out over this much of the recording's clock.
pub const EASE_US: u64 = 250_000;
/// The smallest effect: a hundredth of the picture across.
const MIN_SIZE: f32 = 0.01;
/// At most this many effects in a video.
pub const MAX_EFFECTS: usize = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EffectKind {
    /// Blurred so text can't be read: what it shows is reduced to a coarse grid of colour.
    Blur,
    /// Covered by a solid block: the safest way to keep something out of a video.
    Hide,
    /// The rest of the screen dims, this part stays bright.
    Spotlight,
    /// This part shown larger beside itself, in a round lens.
    Magnify,
    /// Everything behind a graphic dims while it plays.
    Dim,
    /// The screen and camera behind a graphic blur while it plays.
    BlurBehind,
}

impl EffectKind {
    /// Drawn on a recording's picture (all but the ones behind a graphic).
    pub fn on_screen(self) -> bool {
        !self.behind_graphic()
    }

    /// Follows a graphic clip rather than a stretch of a recording.
    pub fn behind_graphic(self) -> bool {
        matches!(self, Self::Dim | Self::BlurBehind)
    }

    /// Never eased: shown whole for its stretch.
    pub fn covers(self) -> bool {
        matches!(self, Self::Blur | Self::Hide)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Blur => "blur",
            Self::Hide => "hide",
            Self::Spotlight => "spotlight",
            Self::Magnify => "magnifier",
            Self::Dim => "dim behind graphic",
            Self::BlurBehind => "blur behind graphic",
        }
    }
}

/// A rectangle of a picture, each value 0..1 of its width or height.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Area {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Area {
    /// Inside the picture, at least [`MIN_SIZE`] across, or `None` when it isn't a rectangle.
    pub fn clamped(self) -> Option<Self> {
        let finite = [self.x, self.y, self.w, self.h]
            .iter()
            .all(|v| v.is_finite());
        if !finite {
            return None;
        }
        let x0 = self.x.clamp(0.0, 1.0);
        let y0 = self.y.clamp(0.0, 1.0);
        let x1 = (self.x + self.w).clamp(0.0, 1.0);
        let y1 = (self.y + self.h).clamp(0.0, 1.0);
        (x1 - x0 >= MIN_SIZE && y1 - y0 >= MIN_SIZE).then_some(Self {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScreenEffect {
    pub id: String,
    pub kind: EffectKind,
    /// The screen recording (or video) it is drawn on; none behind a graphic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<String>,
    /// The stretch of that media's own clock it shows for.
    #[serde(default)]
    pub start_us: u64,
    #[serde(default)]
    pub end_us: u64,
    /// The part of the picture it covers or points at (a dim covers the whole canvas).
    #[serde(default = "whole")]
    pub area: Area,
    /// How strong: spotlight and dim, how dark the rest goes (0..1, default 0.6 and 0.5);
    /// magnifier, how much larger (1.5..4, default 2); blur and blur behind, how coarse (0..1,
    /// default 0.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<f32>,
    /// The graphic clip a dim or blur behind sits under: it plays with that clip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graphic: Option<String>,
}

fn whole() -> Area {
    Area {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    }
}

impl ScreenEffect {
    /// Its strength, defaulted and within range.
    pub fn amount(&self) -> f32 {
        let (default, low, high) = match self.kind {
            EffectKind::Spotlight => (0.6, 0.1, 0.95),
            EffectKind::Dim => (0.5, 0.1, 0.95),
            EffectKind::Magnify => (2.0, 1.5, 4.0),
            EffectKind::Blur | EffectKind::BlurBehind => (0.5, 0.0, 1.0),
            EffectKind::Hide => (1.0, 1.0, 1.0),
        };
        self.amount
            .filter(|a| a.is_finite())
            .unwrap_or(default)
            .clamp(low, high)
    }

    /// How much of it shows at `source_us` of its media (0 when not at all), easing a spotlight
    /// or magnifier in and out.
    pub fn weight_at(&self, source_us: u64) -> f32 {
        if source_us < self.start_us || source_us >= self.end_us {
            return 0.0;
        }
        if self.kind.covers() {
            return 1.0;
        }
        let ease = EASE_US.min((self.end_us - self.start_us) / 2).max(1);
        let edge = (source_us - self.start_us).min(self.end_us - 1 - source_us);
        let t = (edge as f32 / ease as f32).min(1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

/// The screen effects on `asset` showing at `source_us` of its clock, in the order drawn (the
/// order they were added, a magnifier last so it shows what the others did), with how much.
pub fn on_screen_at<'a>(
    effects: &'a [ScreenEffect],
    asset: &str,
    source_us: u64,
) -> Vec<(&'a ScreenEffect, f32)> {
    let mut shown: Vec<(&ScreenEffect, f32)> = effects
        .iter()
        .filter(|e| e.kind.on_screen() && e.asset.as_deref() == Some(asset))
        .filter_map(|e| {
            let weight = e.weight_at(source_us);
            (weight > 0.0).then_some((e, weight))
        })
        .collect();
    shown.sort_by_key(|(e, _)| e.kind == EffectKind::Magnify);
    shown
}

/// The `kind` effect (a dim or a blur) behind graphic clip `clip_id` at `local_us` of its
/// `len_us`: its amount and how far in it is (eased in and out), if it has one.
pub fn behind(
    effects: &[ScreenEffect],
    kind: EffectKind,
    clip_id: &str,
    local_us: u64,
    len_us: u64,
) -> Option<(f32, f32)> {
    let effect = effects
        .iter()
        .find(|e| e.kind == kind && e.graphic.as_deref() == Some(clip_id))?;
    let ease = EASE_US.min(len_us / 2).max(1);
    let last = len_us.saturating_sub(1);
    let local = local_us.min(last);
    let edge = local.min(last - local);
    let t = (edge as f32 / ease as f32).min(1.0);
    Some((effect.amount(), t * t * (3.0 - 2.0 * t)))
}

/// How dark the canvas goes behind graphic clip `clip_id` at `local_us` of its `len_us`
/// (eased in and out), if it has a dim.
pub fn dim_behind(
    effects: &[ScreenEffect],
    clip_id: &str,
    local_us: u64,
    len_us: u64,
) -> Option<f32> {
    behind(effects, EffectKind::Dim, clip_id, local_us, len_us).map(|(a, w)| a * w)
}

/// `effect` checked against `document`: a known screen media (or graphic clip for one behind a
/// graphic), a real stretch and rectangle. Answers it tidied.
pub fn checked(document: &EditDocument, mut effect: ScreenEffect) -> Result<ScreenEffect, String> {
    if effect.kind.behind_graphic() {
        let graphic = effect
            .graphic
            .as_deref()
            .ok_or_else(|| format!("A {} needs its graphic clip", effect.kind.label()))?;
        let (_, clip) = document
            .sequence
            .clip(graphic)
            .ok_or_else(|| format!("No clip {graphic}"))?;
        if !document.asset(&clip.asset).is_some_and(|a| a.is_graphic()) {
            return Err(format!("Clip {graphic} is not a graphic"));
        }
        effect.asset = None;
        effect.area = whole();
        effect.start_us = 0;
        effect.end_us = 0;
        return Ok(effect);
    }
    effect.graphic = None;
    let asset_id = effect
        .asset
        .as_deref()
        .ok_or("A screen effect needs the media it is drawn on")?;
    let asset = document
        .asset(asset_id)
        .ok_or_else(|| format!("No media {asset_id}"))?;
    if asset.is_still() || asset.width == 0 {
        return Err(format!("\"{}\" has no moving picture to mark", asset.name));
    }
    effect.end_us = effect.end_us.min(asset.duration_us);
    if effect.end_us <= effect.start_us + 100_000 {
        return Err("An effect needs at least a tenth of a second".into());
    }
    effect.area = effect
        .area
        .clamped()
        .ok_or("The area must be a rectangle inside the picture (x, y, w, h from 0 to 1)")?;
    Ok(effect)
}

/// The screen clip playing at `edited_us` (the topmost one whose picture is the screen).
fn screen_clip_at(document: &EditDocument, edited_us: u64) -> Option<&crate::sequence::Clip> {
    document
        .sequence
        .video_tracks()
        .rev()
        .filter(|t| !t.hidden)
        .flat_map(|t| t.clips.iter().map(move |c| (t, c)))
        .find(|(t, c)| {
            c.local_us(edited_us).is_some()
                && crate::sequence::clip_role(&document.assets, t, c) == Some(Role::Screen)
        })
        .map(|(_, c)| c)
}

/// The screen media and its clock for edited time `[from_us, to_us)`: the media of the screen
/// playing at `from_us`, from the earliest to the latest moment of it the stretch plays.
pub fn anchor(
    document: &EditDocument,
    from_us: u64,
    to_us: u64,
) -> Result<(String, u64, u64), String> {
    let first = screen_clip_at(document, from_us)
        .ok_or("No screen recording plays there to put an effect on")?;
    let asset = first.asset.clone();
    let mut start = u64::MAX;
    let mut end = 0;
    for track in document.sequence.video_tracks() {
        for clip in track.clips.iter().filter(|c| c.asset == asset) {
            let a = clip.start_us.max(from_us);
            let b = clip.end_us().min(to_us.max(from_us + 1));
            if a >= b {
                continue;
            }
            if let (Some(s), Some(e)) = (clip.local_us(a), clip.local_us(b - 1)) {
                start = start.min(s);
                end = end.max(e + 1);
            }
        }
    }
    if start >= end {
        return Err("No screen recording plays there to put an effect on".into());
    }
    Ok((asset, start, end))
}

/// The edited stretch an effect plays in (its first moment to its last on the timeline), if it
/// plays at all.
pub fn edited_span(document: &EditDocument, effect: &ScreenEffect) -> Option<(u64, u64)> {
    if effect.kind.behind_graphic() {
        let (_, clip) = document.sequence.clip(effect.graphic.as_deref()?)?;
        return Some((clip.start_us, clip.end_us()));
    }
    let asset = effect.asset.as_deref()?;
    let mut span: Option<(u64, u64)> = None;
    for track in document.sequence.video_tracks() {
        for clip in track.clips.iter().filter(|c| c.asset == asset) {
            let a = effect.start_us.max(clip.in_us);
            let b = effect.end_us.min(clip.out_us());
            if a >= b {
                continue;
            }
            let (s, e) = (clip.timeline_at(a), clip.timeline_at(b));
            span = Some(span.map_or((s, e), |(x, y)| (x.min(s), y.max(e))));
        }
    }
    span
}

/// A new effect id not taken in `document`.
pub fn new_id(document: &EditDocument) -> String {
    (1..)
        .map(|n| format!("fx{n}"))
        .find(|id| !document.effects.iter().any(|e| &e.id == id))
        .expect("an id is free")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(kind: EffectKind, start: u64, end: u64) -> ScreenEffect {
        ScreenEffect {
            id: "fx1".into(),
            kind,
            asset: Some("rec".into()),
            start_us: start,
            end_us: end,
            area: Area {
                x: 0.1,
                y: 0.1,
                w: 0.2,
                h: 0.1,
            },
            amount: None,
            graphic: None,
        }
    }

    #[test]
    fn a_blur_shows_whole_for_its_stretch_and_a_spotlight_eases() {
        let blur = effect(EffectKind::Blur, 1_000_000, 2_000_000);
        assert_eq!(blur.weight_at(999_999), 0.0);
        assert_eq!(blur.weight_at(1_000_000), 1.0);
        assert_eq!(blur.weight_at(1_999_999), 1.0);
        assert_eq!(blur.weight_at(2_000_000), 0.0);
        let spot = effect(EffectKind::Spotlight, 1_000_000, 2_000_000);
        assert!(spot.weight_at(1_000_000) < 0.01);
        assert_eq!(spot.weight_at(1_500_000), 1.0);
        assert!(spot.weight_at(1_100_000) > 0.0 && spot.weight_at(1_100_000) < 1.0);
    }

    #[test]
    fn areas_stay_inside_the_picture() {
        let a = Area {
            x: 0.9,
            y: -0.2,
            w: 0.5,
            h: 0.4,
        }
        .clamped()
        .unwrap();
        assert!((a.x - 0.9).abs() < 1e-6 && (a.w - 0.1).abs() < 1e-6);
        assert!(a.y == 0.0 && (a.h - 0.2).abs() < 1e-6);
        assert!(Area {
            x: 0.5,
            y: 0.5,
            w: 0.001,
            h: 0.3
        }
        .clamped()
        .is_none());
        assert!(Area {
            x: f32::NAN,
            y: 0.0,
            w: 0.5,
            h: 0.5
        }
        .clamped()
        .is_none());
    }

    #[test]
    fn magnifiers_are_drawn_last() {
        let mut lens = effect(EffectKind::Magnify, 0, 1_000_000);
        lens.id = "fx2".into();
        let effects = vec![lens, effect(EffectKind::Blur, 0, 1_000_000)];
        let shown = on_screen_at(&effects, "rec", 500_000);
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[1].0.kind, EffectKind::Magnify);
        assert!(on_screen_at(&effects, "other", 500_000).is_empty());
    }

    #[test]
    fn a_dim_eases_with_its_graphic() {
        let mut dim = effect(EffectKind::Dim, 0, 0);
        dim.asset = None;
        dim.graphic = Some("g1".into());
        let effects = vec![dim];
        assert_eq!(dim_behind(&effects, "g2", 0, 5_000_000), None);
        assert!(dim_behind(&effects, "g1", 0, 5_000_000).unwrap() < 0.01);
        assert!((dim_behind(&effects, "g1", 2_500_000, 5_000_000).unwrap() - 0.5).abs() < 1e-6);
        assert!(dim_behind(&effects, "g1", 4_999_999, 5_000_000).unwrap() < 0.01);
        assert_eq!(
            behind(&effects, EffectKind::BlurBehind, "g1", 2_500_000, 5_000_000),
            None
        );
    }

    #[test]
    fn a_blur_behind_follows_its_graphic_not_the_screen() {
        let mut frost = effect(EffectKind::BlurBehind, 0, 0);
        frost.asset = None;
        frost.graphic = Some("g1".into());
        assert!(!frost.kind.on_screen());
        let effects = vec![frost];
        assert!(on_screen_at(&effects, "rec", 0).is_empty());
        let (amount, weight) =
            behind(&effects, EffectKind::BlurBehind, "g1", 2_500_000, 5_000_000).unwrap();
        assert_eq!((amount, weight), (0.5, 1.0));
    }
}
