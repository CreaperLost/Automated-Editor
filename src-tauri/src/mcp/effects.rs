//! `effects`: screen effects (FX-2): blur or hide what must not be seen, a spotlight, a
//! magnifier, and dimming or blurring behind a graphic. Boxes are given as seen in a rendered frame, and
//! stay on what the recording shows there through zooms, crops, layouts and cuts.
use super::actions::{applied, opened, plural};
use super::tools::{clock, params, schema, text, with_reader, Ctx, Output, ToolDef};
use crate::commands::effects as commands;
use crate::effects::{Area, EffectKind, ScreenEffect};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

/// A box drawn without an end lasts this long.
const DEFAULT_LENGTH_US: u64 = 5_000_000;

pub static TOOLS: [ToolDef; 1] = [ToolDef {
    name: "effects",
    description: "Blur or hide (solid block, safest) what must not be seen: API keys, \
        tokens, emails, balances. Spotlight and magnify only when asked; dim or blur_behind \
        under a graphic clip (`graphic`). add: `kind`, `box` (0..1 of the frame as frame_render shows it at \
        `atUs`/`atWord`) until `untilUs` (default +5 s); it stays on what the recording shows \
        there. Also list, move (`id`, new `box` at `atUs`), end (`id` at `atUs`), remove (`id`).",
    schema: schema::<EffectsArgs>,
    read_only: false,
    run: effects,
}];

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    #[default]
    List,
    Add,
    Move,
    End,
    Remove,
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Blur,
    Hide,
    Spotlight,
    Magnify,
    Dim,
    BlurBehind,
}

impl From<Kind> for EffectKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Blur => Self::Blur,
            Kind::Hide => Self::Hide,
            Kind::Spotlight => Self::Spotlight,
            Kind::Magnify => Self::Magnify,
            Kind::Dim => Self::Dim,
            Kind::BlurBehind => Self::BlurBehind,
        }
    }
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EffectsArgs {
    #[serde(default)]
    pub op: Op,
    #[serde(default)]
    pub kind: Option<Kind>,
    #[serde(default, rename = "box")]
    pub area: Option<Frame>,
    #[serde(default)]
    pub at_us: Option<u64>,
    #[serde(default)]
    pub at_word: Option<String>,
    #[serde(default)]
    pub until_us: Option<u64>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub graphic: Option<String>,
    /// Spotlight, dim: 0.1-0.95 dark; magnify: 1.5-4x; blur, blur_behind: 0-1 coarse.
    #[serde(default)]
    pub amount: Option<f32>,
}

fn effects(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: EffectsArgs = params(args)?;
    let at = |args: &EffectsArgs| -> Result<Option<u64>, String> {
        Ok(match (args.at_us, &args.at_word) {
            (Some(at), _) => Some(at),
            (None, Some(word)) => Some(super::actions::word_edited_us(ctx, None, word)?),
            (None, None) => None,
        })
    };
    match args.op {
        Op::List => {
            let lines = with_reader(ctx.state, |reader| {
                let document = reader.document();
                let mut lines = vec![format!("{}:", plural(document.effects.len(), "effect"))];
                lines.extend(document.effects.iter().map(|e| {
                    let when = match crate::effects::edited_span(document, e) {
                        Some((a, b)) => format!("{}-{}", clock(a), clock(b)),
                        None => "not in the edit".into(),
                    };
                    let a = e.area;
                    match &e.graphic {
                        Some(g) => format!("{} {} {g} {:.2}", e.id, e.kind.label(), e.amount()),
                        None => format!(
                            "{} {} {when} area {:.3},{:.3} {:.3}x{:.3} of {} {:.2}",
                            e.id,
                            e.kind.label(),
                            a.x,
                            a.y,
                            a.w,
                            a.h,
                            e.asset.as_deref().unwrap_or("?"),
                            e.amount()
                        ),
                    }
                }));
                Ok(lines)
            })?;
            text(lines.join("\n"))
        }
        Op::Add => {
            let kind: EffectKind = args.kind.ok_or("Give `kind`")?.into();
            let before = opened(ctx)?;
            if kind.behind_graphic() {
                let graphic = args
                    .graphic
                    .clone()
                    .ok_or_else(|| format!("A {} needs `graphic` (clip id)", kind.label()))?;
                let mut all = with_reader(ctx.state, |r| Ok(r.document().effects.clone()))?;
                all.retain(|e| !(e.kind == kind && e.graphic.as_deref() == Some(graphic.as_str())));
                all.push(ScreenEffect {
                    id: String::new(),
                    kind,
                    asset: None,
                    start_us: 0,
                    end_us: 0,
                    area: Area {
                        x: 0.0,
                        y: 0.0,
                        w: 1.0,
                        h: 1.0,
                    },
                    amount: args.amount,
                    graphic: Some(graphic.clone()),
                });
                commands::project_effects_set_impl(
                    ctx.state,
                    before.handle.clone(),
                    before.revision,
                    all,
                )?;
                let line = match kind {
                    EffectKind::Dim => format!("The canvas dims behind {graphic} while it plays."),
                    _ => format!("The video blurs behind {graphic} while it plays."),
                };
                return text(applied(ctx, "effects", &before, vec![line])?);
            }
            let at = at(&args)?.ok_or("Give `atUs` or `atWord`")?;
            let area = args
                .area
                .map(|b| Area {
                    x: b.x,
                    y: b.y,
                    w: b.w,
                    h: b.h,
                })
                .ok_or("Give `box` (x, y, w, h from 0 to 1)")?;
            let until = args.until_us.unwrap_or(at + DEFAULT_LENGTH_US);
            if until <= at {
                return Err("`untilUs` must be after the start".into());
            }
            let effect = commands::project_effect_add_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                kind,
                at,
                until - at,
                area,
                args.amount,
            )?
            .effect;
            let line = format!(
                "Added {} {}, {}-{} on what the recording shows there.",
                effect.kind.label(),
                effect.id,
                clock(at),
                clock(until)
            );
            text(applied(ctx, "effects", &before, vec![line])?)
        }
        Op::Move => {
            let id = args.id.clone().ok_or("Give the effect's `id`")?;
            let at = at(&args)?.ok_or("Give `atUs` or `atWord`")?;
            let area = args
                .area
                .map(|b| Area {
                    x: b.x,
                    y: b.y,
                    w: b.w,
                    h: b.h,
                })
                .ok_or("Give the new `box` (x, y, w, h from 0 to 1)")?;
            let before = opened(ctx)?;
            commands::project_effect_move_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                id.clone(),
                at,
                area,
            )?;
            text(applied(
                ctx,
                "effects",
                &before,
                vec![format!("Moved {id}.")],
            )?)
        }
        Op::End => {
            let id = args.id.clone().ok_or("Give the effect's `id`")?;
            let at = at(&args)?.ok_or("Give `atUs` or `atWord`")?;
            let before = opened(ctx)?;
            commands::project_effect_end_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                id.clone(),
                at,
            )?;
            text(applied(
                ctx,
                "effects",
                &before,
                vec![format!("{id} ends at {}.", clock(at))],
            )?)
        }
        Op::Remove => {
            let id = args.id.ok_or("Give the effect's `id`")?;
            let before = opened(ctx)?;
            let mut all = with_reader(ctx.state, |r| Ok(r.document().effects.clone()))?;
            let count = all.len();
            all.retain(|e| e.id != id);
            if all.len() == count {
                return Err(format!("No effect {id}"));
            }
            commands::project_effects_set_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                all,
            )?;
            text(applied(
                ctx,
                "effects",
                &before,
                vec![format!("Removed {id}.")],
            )?)
        }
    }
}
