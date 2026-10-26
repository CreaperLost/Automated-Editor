//! Screen effects' commands (FX-2): add one over a box drawn on the preview, see the ones at the
//! playhead, move or end one there, or set them all. Each change is one undo step.
use super::{mutate_opened, require_handle, AppState};
use crate::effects::{self, Area, EffectKind, ScreenEffect};
use crate::project::revision::EditDocument;
use crate::project::OpenedProject;
use serde::Serialize;
use std::path::PathBuf;

/// How wide the canvas is drawn to find where the screen is: small, so the decode is quick.
const PROBE_WIDTH: u32 = 480;

/// The screen as drawn at a moment: its media and time, and the scene (screen layer included).
struct ScreenAt {
    asset: String,
    local: u64,
    scene: crate::render::Scene,
}

fn document_and_root(
    state: &AppState,
    project_handle: &str,
) -> Result<(EditDocument, PathBuf), String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, project_handle)?;
    Ok((reader.document().clone(), reader.root().to_path_buf()))
}

fn screen_at(
    root: PathBuf,
    document: &EditDocument,
    edited_us: u64,
) -> Result<Option<ScreenAt>, String> {
    let (w, h) = document.layout.preview_dimensions()?;
    let scale = (PROBE_WIDTH as f64 / w as f64).min(1.0);
    let even = |v: u32| ((v as f64 * scale).round() as u32 / 2 * 2).max(2);
    let (w, h) = (even(w), even(h));
    let evaluator = crate::export::SceneEvaluator::without_gpu(root, document.clone(), w, h)
        .with_decode_limit(crate::media::ffmpeg::DecodeLimit {
            max_width: w * 2,
            max_height: h * 2,
            max_rate: 0,
            interactive: true,
            yuv: false,
        });
    let Some((asset, local)) = evaluator.screen_clip_at(edited_us) else {
        return Ok(None);
    };
    let scene = evaluator.scene_at(edited_us)?;
    Ok(Some(ScreenAt {
        asset,
        local,
        scene,
    }))
}

/// An effect showing at a moment: where it is on the canvas (0..1) and when it plays.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectAt {
    pub effect: ScreenEffect,
    pub canvas: Area,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<(u64, u64)>,
}

/// The screen effects showing at `edited_us`, with where each is on the canvas.
pub fn project_effects_at_impl(
    state: &AppState,
    project_handle: String,
    edited_us: u64,
) -> Result<Vec<EffectAt>, String> {
    let (document, root) = document_and_root(state, &project_handle)?;
    if document.effects.is_empty() {
        return Ok(Vec::new());
    }
    let Some(at) = screen_at(root, &document, edited_us)? else {
        return Ok(Vec::new());
    };
    let (cw, ch) = (at.scene.width as f32, at.scene.height as f32);
    Ok(
        effects::on_screen_at(&document.effects, &at.asset, at.local)
            .into_iter()
            .filter_map(|(effect, _)| {
                let a = effect.area;
                let (_, [x, y, w, h], _) = at.scene.screen_area([a.x, a.y, a.w, a.h])?;
                Some(EffectAt {
                    canvas: Area {
                        x: x as f32 / cw,
                        y: y as f32 / ch,
                        w: w as f32 / cw,
                        h: h as f32 / ch,
                    },
                    span: effects::edited_span(&document, effect),
                    effect: effect.clone(),
                })
            })
            .collect(),
    )
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectAdded {
    pub project: OpenedProject,
    pub effect: ScreenEffect,
}

/// The part of the screen's picture under `canvas` (0..1 of the canvas) at `edited_us`.
fn picture_area(at: &ScreenAt, canvas: Area) -> Result<Area, String> {
    let index = at
        .scene
        .layers
        .iter()
        .position(|l| l.role == crate::render::LayerRole::Screen)
        .ok_or("The screen isn't showing there")?;
    let s = &at.scene.layers[index];
    let (cw, ch) = (at.scene.width as f32, at.scene.height as f32);
    let (sx, sy, sw, sh) = (s.x as f32, s.y as f32, s.width as f32, s.height as f32);
    let x0 = (canvas.x * cw).max(sx);
    let y0 = (canvas.y * ch).max(sy);
    let x1 = ((canvas.x + canvas.w) * cw).min(sx + sw);
    let y1 = ((canvas.y + canvas.h) * ch).min(sy + sh);
    if x1 <= x0 || y1 <= y0 {
        return Err("Draw the box over the screen recording".into());
    }
    let u = |x: f32| s.uv_x + (x - sx) / sw * s.uv_w;
    let v = |y: f32| s.uv_y + (y - sy) / sh * s.uv_h;
    Area {
        x: u(x0),
        y: v(y0),
        w: u(x1) - u(x0),
        h: v(y1) - v(y0),
    }
    .clamped()
    .ok_or_else(|| "The box is too small".into())
}

/// Adds a `kind` effect over the box `canvas` (0..1 of the canvas, as drawn on the preview at
/// `at_us`), on what the recording shows there for `duration_us` of the edit from `at_us`.
pub fn project_effect_add_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    kind: EffectKind,
    at_us: u64,
    duration_us: u64,
    canvas: Area,
    amount: Option<f32>,
) -> Result<EffectAdded, String> {
    if !kind.on_screen() {
        return Err(format!("A {} is set on its graphic", kind.label()));
    }
    let (document, root) = document_and_root(state, &project_handle)?;
    let at = screen_at(root, &document, at_us)?
        .ok_or("No screen recording plays at the playhead to put an effect on")?;
    let area = picture_area(&at, canvas)?;
    let end_us = (at_us + duration_us.max(500_000)).min(document.duration_us());
    let (asset, start, end) = effects::anchor(&document, at_us, end_us.max(at_us + 1))?;
    if asset != at.asset {
        return Err("The screen at the playhead isn't the one under the box".into());
    }
    let mut added = None;
    let project = mutate_opened(state, project_handle, |reader| {
        let mut next = reader.document().clone();
        let effect = effects::checked(
            &next,
            ScreenEffect {
                id: effects::new_id(&next),
                kind,
                asset: Some(asset),
                start_us: start.min(at.local),
                end_us: end,
                area,
                amount,
                graphic: None,
            },
        )?;
        next.effects.push(effect.clone());
        added = Some(effect.clone());
        reader.name_next_step(format!("Add {}", kind.label()));
        reader.commit_document(expected_revision, next)
    })?;
    Ok(EffectAdded {
        project,
        effect: added.expect("set when committed"),
    })
}

/// Ends effect `effect_id` at edited time `at_us` (where its recording plays).
pub fn project_effect_end_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    effect_id: String,
    at_us: u64,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        let mut next = reader.document().clone();
        let effect = next
            .effects
            .iter_mut()
            .find(|e| e.id == effect_id)
            .ok_or_else(|| format!("No effect {effect_id}"))?;
        let asset = effect
            .asset
            .clone()
            .ok_or_else(|| format!("A {} ends with its graphic", effect.kind.label()))?;
        let (_, local, _) = effects::anchor(reader.document(), at_us, at_us + 1)
            .ok()
            .filter(|(a, _, _)| *a == asset)
            .ok_or("Its recording doesn't play at the playhead")?;
        if local <= effect.start_us + 100_000 {
            return Err("That is before it starts; remove it instead".into());
        }
        effect.end_us = local;
        let label = format!("End {}", effect.kind.label());
        reader.name_next_step(label);
        reader.commit_document(expected_revision, next)
    })
}

/// Moves or resizes effect `effect_id` to the box `canvas` (0..1 of the canvas, as drawn on the
/// preview at `at_us`, where its recording plays): one undo step.
pub fn project_effect_move_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    effect_id: String,
    at_us: u64,
    canvas: Area,
) -> Result<OpenedProject, String> {
    let (document, root) = document_and_root(state, &project_handle)?;
    let effect = document
        .effects
        .iter()
        .find(|e| e.id == effect_id)
        .ok_or_else(|| format!("No effect {effect_id}"))?;
    if !effect.kind.on_screen() {
        return Err(format!("A {} covers its graphic", effect.kind.label()));
    }
    let at = screen_at(root, &document, at_us)?
        .filter(|at| effect.asset.as_deref() == Some(at.asset.as_str()))
        .ok_or("Its recording doesn't play at the playhead")?;
    let area = picture_area(&at, canvas)?;
    mutate_opened(state, project_handle, |reader| {
        let mut next = reader.document().clone();
        let effect = next
            .effects
            .iter_mut()
            .find(|e| e.id == effect_id)
            .ok_or_else(|| format!("No effect {effect_id}"))?;
        effect.area = area;
        let label = format!("Move {}", effect.kind.label());
        reader.name_next_step(label);
        reader.commit_document(expected_revision, next)
    })
}

/// Sets the video's effects (each checked; new ones without an id get one): one undo step.
pub fn project_effects_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    effects: Vec<ScreenEffect>,
) -> Result<OpenedProject, String> {
    if effects.len() > effects::MAX_EFFECTS {
        return Err(format!(
            "A video has {} effects at most",
            effects::MAX_EFFECTS
        ));
    }
    mutate_opened(state, project_handle, |reader| {
        let mut next = reader.document().clone();
        next.effects = Vec::new();
        for effect in effects {
            let mut effect = effects::checked(&next, effect)?;
            if effect.id.trim().is_empty() || next.effects.iter().any(|e| e.id == effect.id) {
                effect.id = effects::new_id(&next);
            }
            next.effects.push(effect);
        }
        reader.name_next_step("Change effects".into());
        reader.commit_document(expected_revision, next)
    })
}
