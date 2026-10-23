//! The `layouts` action: the base layout and the sections that change it, on the video or a
//! short, and the favourite layouts kept for the app. See [`crate::layouts`].
use super::actions::{applied, opened, patched, plural};
use super::tools::{clock, params, text, with_reader, Ctx, Output};
use crate::commands::{self, LayoutChange};
use crate::layouts::{favourites, Layout, LayoutTrack, Transition};
use crate::project::revision::EditDocument;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

pub const DESCRIPTION: &str = "Layouts: how the screen and the camera share the frame, on the \
    video or a short (`short`: its id or title). A timeline has a base layout and sections that \
    change it, anchored to their content so cuts never move them; the base shows everywhere \
    else. Kinds: `bubble` (screen with the camera in a bubble: `spot` topLeft..bottomRight on a \
    3x3 grid or custom, `sizePct` 5-60, small 20/medium 35/large 50, `shape` rect/circle/squircle), \
    `sideBySide` (camera left or right, top or bottom in 9:16: `cameraFirst`, `cameraPct` 20-80, \
    `overlap`, `screen` fit/fill/auto), `cameraOnly` (`sizePct` 40-100, `punchIn` 1-2), \
    `screenOnly`. `layout` is a built-in name (Camera bubble, Side by side, Camera only, \
    Punch-in, Screen only), a favourite's name, `base`, or a spec like {\"kind\":\"sideBySide\", \
    \"cameraPct\":30}. `op`: `list` (default); `add` with `sections` [{fromS, toS, layout, \
    transition?}] in one step; `update` `id` with `layout`/`transition`; `retime` `id` to \
    `fromS`-`toS`; `remove` `ids`; `clear` `fromS`-`toS` (or all, or `autoOnly`); `base` \
    `layout`; `switch` to `layout` from `atS` to the next change (number-key switching); `pick` \
    from the recording, no AI (camera while the mouse rests as you talk, screen while busy, \
    the opening line in a short; `settings` changes the rules first, `apply: false` only \
    reports; sections made by hand stay); `settings` (`transitionMs`, rules); `save` a \
    favourite `name` from `id`, `layout` or the base; `forget` a favourite. Transitions: auto \
    (a cut between full-frame shots and on cuts, else a short move), cut or move. Seconds are \
    on the target's own timeline.";

#[derive(Deserialize, JsonSchema, Default, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum LayoutsOp {
    #[default]
    List,
    Add,
    Update,
    Retime,
    Remove,
    Clear,
    Base,
    Switch,
    Pick,
    Settings,
    Save,
    Forget,
}

/// A layout by name (a built-in, a favourite, or `base`) or as a spec.
#[derive(Deserialize, JsonSchema, Clone)]
#[serde(untagged)]
pub enum LayoutRef {
    Name(String),
    Spec(Layout),
}

#[derive(Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SectionInput {
    pub from_s: f64,
    pub to_s: f64,
    pub layout: LayoutRef,
    #[serde(default)]
    pub transition: Option<Transition>,
    #[serde(default)]
    pub transition_ms: Option<u32>,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct LayoutsArgs {
    #[serde(default)]
    pub op: Option<LayoutsOp>,
    /// A short's id or title; absent is the video.
    #[serde(default)]
    pub short: Option<String>,
    #[serde(default)]
    pub sections: Option<Vec<SectionInput>>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub ids: Option<Vec<String>>,
    #[serde(default)]
    pub layout: Option<LayoutRef>,
    #[serde(default)]
    pub from_s: Option<f64>,
    #[serde(default)]
    pub to_s: Option<f64>,
    #[serde(default)]
    pub at_s: Option<f64>,
    #[serde(default)]
    pub transition: Option<Transition>,
    #[serde(default)]
    pub transition_ms: Option<u32>,
    /// `clear` without a range: only the picked sections.
    #[serde(default)]
    pub auto_only: Option<bool>,
    /// The pick rules to change, as in the `pick` part of the list.
    #[serde(default)]
    pub settings: Option<Value>,
    /// A favourite's name, for `save` and `forget`.
    #[serde(default)]
    pub name: Option<String>,
    /// false: `pick` only reports.
    #[serde(default)]
    pub apply: Option<bool>,
}

fn us(seconds: f64) -> Result<u64, String> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(format!("{seconds} is not a time"));
    }
    Ok((seconds * 1e6).round() as u64)
}

/// The short a name means: its id, else its title.
fn short_id(document: &EditDocument, wanted: &str) -> Result<String, String> {
    document
        .shorts
        .iter()
        .find(|s| s.id == wanted)
        .or_else(|| {
            document
                .shorts
                .iter()
                .find(|s| s.title.eq_ignore_ascii_case(wanted.trim()))
        })
        .map(|s| s.id.clone())
        .ok_or_else(|| {
            let known: Vec<String> = document
                .shorts
                .iter()
                .map(|s| format!("{} \"{}\"", s.id, s.title))
                .collect();
            format!("No short {wanted}. Shorts: {}", known.join(", "))
        })
}

/// The target's timeline (the short's own) and its layouts.
fn target(document: &EditDocument, short: Option<&str>) -> Result<EditDocument, String> {
    match short {
        None => Ok(document.clone()),
        Some(id) => {
            let short = document
                .shorts
                .iter()
                .find(|s| s.id == id)
                .ok_or("That short no longer exists")?;
            crate::shorts::short_timeline(document, short)
        }
    }
}

/// A layout by reference, with the name to show for it; `None` is the base.
fn resolve_ref(layout: &LayoutRef) -> Result<(Option<Layout>, Option<String>), String> {
    match layout {
        LayoutRef::Spec(spec) => {
            spec.validate()?;
            Ok((Some(spec.clone()), None))
        }
        LayoutRef::Name(name) if name.trim().eq_ignore_ascii_case("base") => Ok((None, None)),
        LayoutRef::Name(name) => favourites::named(&favourites::dir(), name)
            .map(|(name, layout)| (Some(layout), Some(name)))
            .ok_or_else(|| {
                let favourites: Vec<String> = favourites::list(&favourites::dir())
                    .into_iter()
                    .map(|f| f.name)
                    .collect();
                format!(
                    "No layout \"{name}\". Built-in: {}. Favourites: {}.",
                    crate::layouts::built_ins()
                        .iter()
                        .map(|(n, _)| *n)
                        .collect::<Vec<_>>()
                        .join(", "),
                    if favourites.is_empty() {
                        "none".into()
                    } else {
                        favourites.join(", ")
                    }
                )
            }),
    }
}

/// "Side by side (camera first, 40 %, fit)", for lists.
pub fn describe(layout: &Layout) -> String {
    let fit = |f: crate::layouts::ScreenFit| match f {
        crate::layouts::ScreenFit::Auto => "",
        crate::layouts::ScreenFit::Fit => ", screen fitted",
        crate::layouts::ScreenFit::Fill => ", screen filled",
    };
    match layout {
        Layout::Bubble(b) => format!(
            "Camera bubble ({}, {:.0} %, {}{})",
            serde_json::to_value(b.spot)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            b.size_pct,
            serde_json::to_value(b.shape)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            fit(b.screen)
        ),
        Layout::SideBySide(s) => format!(
            "Side by side (camera {}, {:.0} %{}{})",
            if s.camera_first { "first" } else { "second" },
            s.camera_pct,
            if s.overlap { ", overlapping" } else { "" },
            fit(s.screen)
        ),
        Layout::CameraOnly(c) if c.punch_in > 1.0 => {
            format!("Punch-in ({:.1}x, {:.0} %)", c.punch_in, c.size_pct)
        }
        Layout::CameraOnly(c) => format!("Camera only ({:.0} %)", c.size_pct),
        Layout::ScreenOnly(s) => match s.screen {
            crate::layouts::ScreenFit::Auto => "Screen only".into(),
            crate::layouts::ScreenFit::Fit => "Screen only (fitted)".into(),
            crate::layouts::ScreenFit::Fill => "Screen only (filled)".into(),
        },
    }
}

/// The target's layouts, a line each.
fn listing(timeline: &EditDocument, label: &str) -> Vec<String> {
    let track: LayoutTrack = timeline.layouts_with_ranges();
    let resolved = crate::layouts::resolve::of_document(timeline);
    let mut lines = vec![format!(
        "{label}: base {}; moves take {} ms.",
        describe(&track.base),
        track.transition_ms
    )];
    if track.sections.is_empty() {
        lines.push("No sections: the base shows throughout.".into());
    }
    for section in &track.sections {
        let shown = resolved.pieces.iter().any(|p| p.section == section.id);
        let place = match (section.edited_ranges.first(), section.edited_ranges.last()) {
            (Some(a), Some(b)) => format!("{}-{}", clock(a.start_us), clock(b.end_us)),
            _ => "cut".into(),
        };
        let what = match &section.layout {
            Some(layout) => match &section.name {
                Some(name) => format!("{name}: {}", describe(layout)),
                None => describe(layout),
            },
            None => "the base (keeps picks away)".into(),
        };
        let by = match (&section.auto, &section.rule) {
            (true, Some(rule)) => format!("picked: {rule}"),
            (true, None) => "picked".into(),
            (false, _) => "by hand".into(),
        };
        let mut line = format!("{} {place} {what} ({by}", section.id);
        if section.transition != Transition::Auto {
            line.push_str(&format!(
                ", {}",
                serde_json::to_value(section.transition)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default()
            ));
        }
        line.push(')');
        if !shown && section.layout.is_some() && !section.edited_ranges.is_empty() {
            line.push_str(" (not shown: too short after cuts)");
        }
        lines.push(line);
    }
    lines.push(format!(
        "Pick rules: {}",
        serde_json::to_string(&track.pick).unwrap_or_default()
    ));
    lines
}

fn favourite_lines() -> String {
    let favourites = favourites::list(&favourites::dir());
    format!(
        "Built-in layouts: {}. Favourites: {}.",
        crate::layouts::built_ins()
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join(", "),
        if favourites.is_empty() {
            "none".to_string()
        } else {
            favourites
                .iter()
                .map(|f| format!("{} = {}", f.name, describe(&f.layout)))
                .collect::<Vec<_>>()
                .join("; ")
        }
    )
}

pub fn layouts(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: LayoutsArgs = params(args)?;
    text(run(ctx, args)?)
}

/// Picks the sections of the video or a short and reports them; `apply` false only reports.
pub fn run_pick(
    ctx: &Ctx,
    short: Option<String>,
    settings: Option<Value>,
    apply: bool,
) -> Result<String, String> {
    let before = opened(ctx)?;
    let current = with_reader(ctx.state, |reader| {
        let document = reader.document();
        Ok(match &short {
            Some(id) => document
                .shorts
                .iter()
                .find(|s| &s.id == id)
                .map(|s| s.layouts.pick.clone())
                .unwrap_or_default(),
            None => document.layouts.pick.clone(),
        })
    })?;
    let settings = match &settings {
        Some(patch) => Some(patched("pick", &current, patch)?),
        None => None,
    };
    ctx.progress(0.1, "Reading the recording's mouse and speech");
    let found = commands::layouts_find_impl(ctx.state, &before.handle, short.as_deref(), settings)?;
    let counts = |test: fn(&Layout) -> bool| {
        found
            .sections
            .iter()
            .filter(|s| s.layout.as_ref().is_some_and(test))
            .count()
    };
    let total: u64 = found
        .sections
        .iter()
        .map(|s| s.source_end_us - s.source_start_us)
        .sum();
    let mut lines = vec![if found.sections.is_empty() {
        "Nothing to pick: the base layout shows throughout.".to_string()
    } else {
        format!(
            "{} {}: {} with the camera alone, {} with the screen alone, {} in all.",
            if apply { "Picked" } else { "Would pick" },
            plural(found.sections.len(), "section"),
            counts(|l| matches!(l, Layout::CameraOnly(_))),
            counts(|l| matches!(l, Layout::ScreenOnly(_))),
            clock(total)
        )
    }];
    lines.extend(found.diagnostics.iter().cloned());
    if apply {
        commands::layouts_store_impl(
            ctx.state,
            before.handle.clone(),
            before.revision,
            short,
            found,
        )?;
    }
    applied(ctx, "layouts", &before, lines)
}

fn run(ctx: &Ctx, args: LayoutsArgs) -> Result<String, String> {
    let before = opened(ctx)?;
    let document = with_reader(ctx.state, |reader| Ok(reader.document().clone()))?;
    let short = args
        .short
        .as_deref()
        .map(|s| short_id(&document, s))
        .transpose()?;
    let label = match &short {
        Some(id) => format!("Short {id}"),
        None => "The video".to_string(),
    };
    let op = args.op.unwrap_or_default();
    let range = || -> Result<(u64, u64), String> {
        let (from, to) = (
            args.from_s.ok_or("Give `fromS`")?,
            args.to_s.ok_or("Give `toS`")?,
        );
        Ok((us(from)?, us(to)?))
    };
    let layout_arg = || args.layout.as_ref().ok_or("Give `layout`".to_string());
    let change = match op {
        LayoutsOp::List => {
            let timeline = target(&document, short.as_deref())?;
            let mut lines = listing(&timeline, &label);
            lines.push(favourite_lines());
            return Ok(lines.join("\n"));
        }
        LayoutsOp::Pick => {
            return run_pick(ctx, short, args.settings.clone(), args.apply != Some(false));
        }
        LayoutsOp::Save => {
            let name = args.name.as_deref().ok_or("Give the favourite a `name`")?;
            let layout = match (&args.id, &args.layout) {
                (Some(id), _) => {
                    let timeline = target(&document, short.as_deref())?;
                    timeline
                        .layouts
                        .sections
                        .iter()
                        .find(|s| &s.id == id)
                        .ok_or_else(|| format!("No layout section {id}"))?
                        .layout
                        .clone()
                        .unwrap_or_else(|| timeline.layouts.base.clone())
                }
                (None, Some(layout)) => match resolve_ref(layout)?.0 {
                    Some(layout) => layout,
                    None => target(&document, short.as_deref())?.layouts.base,
                },
                (None, None) => target(&document, short.as_deref())?.layouts.base,
            };
            let saved = favourites::save(&favourites::dir(), name, layout.clone())?;
            return Ok(format!(
                "Saved favourite \"{}\": {}. {} kept.",
                name.trim(),
                describe(&layout),
                plural(saved.len(), "favourite")
            ));
        }
        LayoutsOp::Forget => {
            let which = args
                .name
                .as_deref()
                .or(args.id.as_deref())
                .ok_or("Give the favourite's `name`")?;
            let left = favourites::remove(&favourites::dir(), which)?;
            return Ok(format!(
                "Forgot \"{which}\"; {} left.",
                plural(left.len(), "favourite")
            ));
        }
        LayoutsOp::Add => {
            let sections = args.sections.clone().ok_or("`add` needs `sections`")?;
            let mut changes = Vec::new();
            for input in sections {
                let (layout, name) = resolve_ref(&input.layout)?;
                changes.push(LayoutChange::Add {
                    start_us: us(input.from_s)?,
                    end_us: us(input.to_s)?,
                    layout,
                    name,
                    transition: input.transition,
                    transition_ms: input.transition_ms,
                });
            }
            LayoutChange::Many { changes }
        }
        LayoutsOp::Update => {
            let id = args.id.clone().ok_or("`update` needs the section's `id`")?;
            let (layout, name, base) = match &args.layout {
                Some(layout) => {
                    let (layout, name) = resolve_ref(layout)?;
                    let base = layout.is_none();
                    (layout, name, base)
                }
                None => (None, None, false),
            };
            LayoutChange::Update {
                id,
                layout,
                base,
                name,
                transition: args.transition,
                transition_ms: args.transition_ms,
            }
        }
        LayoutsOp::Retime => {
            let (start_us, end_us) = range()?;
            LayoutChange::Retime {
                id: args.id.clone().ok_or("`retime` needs the section's `id`")?,
                start_us,
                end_us,
            }
        }
        LayoutsOp::Remove => LayoutChange::Remove {
            ids: args
                .ids
                .clone()
                .or_else(|| args.id.clone().map(|id| vec![id]))
                .ok_or("`remove` needs `ids`")?,
        },
        LayoutsOp::Clear => match (args.from_s, args.to_s) {
            (Some(_), Some(_)) => {
                let (start_us, end_us) = range()?;
                LayoutChange::Clear { start_us, end_us }
            }
            _ => LayoutChange::ClearAll {
                auto_only: args.auto_only == Some(true),
            },
        },
        LayoutsOp::Base => match resolve_ref(layout_arg()?)? {
            (Some(layout), _) => LayoutChange::Base { layout },
            (None, _) => return Err("Give the base a layout, not `base`".into()),
        },
        LayoutsOp::Switch => {
            let at = us(args.at_s.ok_or("`switch` needs `atS`")?)?;
            let timeline = target(&document, short.as_deref())?;
            let resolved = crate::layouts::resolve::of_document(&timeline);
            // To the end of what plays there: the section, or the stretch of base.
            let end = match resolved.piece_at(at) {
                Some(piece) => piece.end_us,
                None => resolved
                    .pieces
                    .iter()
                    .map(|p| p.start_us)
                    .find(|&start| start > at)
                    .unwrap_or(resolved.duration_us),
            };
            let (layout, name) = resolve_ref(layout_arg()?)?;
            LayoutChange::Add {
                start_us: at,
                end_us: end,
                layout,
                name,
                transition: args.transition,
                transition_ms: args.transition_ms,
            }
        }
        LayoutsOp::Settings => {
            let current = target(&document, short.as_deref())?.layouts.pick;
            LayoutChange::Settings {
                transition_ms: args.transition_ms,
                pick: args
                    .settings
                    .as_ref()
                    .map(|patch| patched("pick", &current, patch))
                    .transpose()?,
            }
        }
    };
    commands::project_layouts_change_impl(
        ctx.state,
        before.handle.clone(),
        before.revision,
        short.clone(),
        change,
    )?;
    let lines = with_reader(ctx.state, |reader| {
        Ok(listing(
            &target(reader.document(), short.as_deref())?,
            &label,
        ))
    })?;
    applied(ctx, "layouts", &before, lines)
}
