//! `library`: the user's styles, presets and favourite layouts, kept by the app for every
//! project, in one tool (instead of one tool per kind and per operation, to keep the tool list
//! short). What a project picks from them is one undo step; changing one changes it app-wide.
use super::actions::{applied, opened, plural};
use super::tools::{params, pretty, schema, text, with_reader, Ctx, Output, ToolDef};
use crate::commands;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Style,
    Preset,
    Layout,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    #[default]
    List,
    Show,
    Use,
    Set,
    Remove,
    Preview,
    Check,
    BringInLine,
    Save,
    Reset,
    ProposeRule,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LibraryArgs {
    /// style, preset or layout.
    pub kind: Kind,
    /// list (default), show, use (in this project), set, remove; for styles also preview,
    /// check (graphics not following the style) and bring_in_line; for presets also save (the
    /// project's settings into its preset, or a new one with `saveAs`) and reset (the
    /// project's changes back to its preset's values).
    #[serde(default)]
    pub op: Op,
    /// Which one; for show, set and preview the project's when left out.
    #[serde(default)]
    pub name: Option<String>,
    /// set / preview: only the fields to change, as in `show` (null takes one back to the
    /// built-in value), such as {"brand": {"colors": {"accent": "#FF5500"}}}.
    #[serde(default)]
    pub set: Option<Value>,
    /// set / save: save as a new one under this name (a built-in is never changed: without
    /// it the change goes into a copy, "My <name>").
    #[serde(default)]
    pub save_as: Option<String>,
    /// propose_rule: the rule, in the user's words.
    #[serde(default)]
    pub rule: Option<String>,
    /// propose_rule: what the user did or said that taught it.
    #[serde(default)]
    pub example: Option<String>,
    /// Presets, use: what the preset is for: `video` (default), or the project's new `shorts`
    /// (from a take) or new `clips` (cut from the video).
    #[serde(default, rename = "for")]
    pub target: crate::commands::preset::PresetTarget,
    /// Presets, reset: only this part (captions, layout, zoomSettings, audio, layouts).
    #[serde(default)]
    pub part: Option<String>,
}

pub static TOOLS: [ToolDef; 1] = [ToolDef {
    name: "library",
    description: "The user's styles (how graphics, captions and the frame look: brand \
        colours, fonts, logo, graphics look, caption look, frame, 9:16 variant), presets (what \
        the app does for a kind of video: cleanup, captions, zooms, layouts, shorts, export, \
        the method you follow) and favourite layouts, kept for every project. `op`: list, \
        show, use (this project takes it: one undo step), set (change fields app-wide, every \
        project using it follows: ask the user first; a built-in goes into a copy), remove \
        (at the user's word). Styles also: preview (stills of the video's graphics with `set` \
        applied, changing nothing: show the user before set), check (graphics not following \
        the style) and bring_in_line (one batch rewriting them, for the user to accept). \
        Presets also: save (the project's settings into its preset, or a new one: \
        `saveAs`), reset (the project's changes back), propose_rule (`rule`, `example`: the \
        user accepts it in the Agent panel).",
    schema: schema::<LibraryArgs>,
    read_only: false,
    run: library,
}];

fn library(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: LibraryArgs = params(args)?;
    match args.kind {
        Kind::Style => style(ctx, args),
        Kind::Preset => super::presets::library(ctx, args),
        Kind::Layout => layout(args),
    }
}

/// The open project's style name, if a project is open.
fn project_style(ctx: &Ctx) -> Option<String> {
    with_reader(ctx.state, |reader| Ok(reader.document().style.clone())).ok()
}

fn style(ctx: &Ctx, args: LibraryArgs) -> Result<Vec<Output>, String> {
    use crate::style::store;
    let named = || {
        args.name
            .clone()
            .or_else(|| project_style(ctx))
            .unwrap_or_else(|| crate::style::DEFAULT.into())
    };
    match args.op {
        Op::List => {
            let ours = project_style(ctx).map(|s| store::current_name(&store::dir(), &s));
            let styles: Vec<Value> = store::list()
                .iter()
                .map(|s| {
                    let c = &s.brand.colors;
                    json!({
                        "name": s.name,
                        "builtIn": crate::style::is_builtin(&s.name),
                        "colors": [c.background, c.text, c.accent, c.muted],
                        "font": s.brand.fonts.title.family,
                        "graphics": s.graphics.preset,
                        "captions": s.captions.style.label(),
                        "thisProject": ours.as_deref() == Some(s.name.as_str()),
                    })
                })
                .collect();
            pretty(&json!({ "styles": styles, "folder": store::dir().display().to_string() }))
        }
        Op::Show => {
            let style = store::load(&named())?;
            pretty(&json!({
                "name": style.name,
                "builtIn": crate::style::is_builtin(&style.name),
                "style": style,
                "graphicsTokens": style.graphics.values(),
                "graphicsPresets": crate::graphics::look::presets(),
                "backgrounds": crate::project::layout::BACKGROUND_PRESETS,
                "folder": store::folder_of(&style).map(|f| f.display().to_string()),
            }))
        }
        Op::Use => {
            let name = args.name.ok_or("Give the style's `name`")?;
            let before = opened(ctx)?;
            let name = store::load(&name)?.name;
            commands::style::project_style_set_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                name.clone(),
            )?;
            text(applied(
                ctx,
                "library",
                &before,
                vec![
                    format!("The project is dressed in style {name}."),
                    "Graphics render again by themselves.".into(),
                ],
            )?)
        }
        Op::Set => {
            let patch = args.set.ok_or("Give the fields to change in `set`")?;
            let (saved, copied) = commands::style::style_patch_impl(
                ctx.state,
                &named(),
                &patch,
                args.save_as.as_deref(),
            )?;
            if copied && args.save_as.is_none() {
                return text(format!(
                    "{} is built in, so the change went into a copy, style {}, which the project \
                     wears now. Graphics render again by themselves.",
                    named(),
                    saved.name
                ));
            }
            text(format!(
                "Saved style {} for every project. Projects using it redraw; graphics render \
                 again by themselves.",
                saved.name
            ))
        }
        Op::Remove => {
            let name = args.name.ok_or("Give the style's `name`")?;
            commands::style::style_remove_impl(ctx.state, name.clone())?;
            text(format!(
                "Removed style {name}. Projects that used it take Light."
            ))
        }
        Op::Preview => {
            let before = opened(ctx)?;
            let base = store::load(&named())?;
            let style = match &args.set {
                Some(patch) => base.patched(patch)?,
                None => base,
            };
            let stills =
                commands::graphics::style_preview_impl(ctx.state, before.handle.clone(), style)?;
            let mut out = vec![Output::Text(format!(
                "Not applied. {}: `set` to keep it.",
                stills
                    .iter()
                    .map(|s| format!("\"{}\"", s.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))];
            use base64::Engine;
            for still in stills {
                let data = still.image.trim_start_matches("data:image/png;base64,");
                out.push(Output::Png(
                    base64::engine::general_purpose::STANDARD
                        .decode(data)
                        .map_err(|e| e.to_string())?,
                ));
            }
            Ok(out)
        }
        Op::Check => {
            let before = opened(ctx)?;
            let plans =
                commands::graphics::graphics_in_line_plan_impl(ctx.state, before.handle.clone())?;
            let view = commands::style::styles_impl(ctx.state, Some(before.handle))?;
            pretty(&json!({
                "notInLine": plans.iter().map(|p| json!({
                    "graphic": p.graphic, "name": p.name, "changes": p.changes,
                })).collect::<Vec<_>>(),
                "ownStyleOrTokens": view.own.iter().map(|(name, style, tokens)| json!({
                    "name": name, "style": style, "tokens": tokens,
                })).collect::<Vec<_>>(),
            }))
        }
        Op::BringInLine => {
            let agent = ctx.agent.clone();
            super::propose::propose(ctx, |reader, proposals| {
                crate::proposals::propose_restyle(
                    reader,
                    proposals,
                    &agent,
                    "Bring every graphic in line with the style",
                )
            })
        }
        Op::Save | Op::Reset | Op::ProposeRule => {
            Err("Styles take list, show, use, set, remove, preview, check or bring_in_line".into())
        }
    }
}

fn layout(args: LibraryArgs) -> Result<Vec<Output>, String> {
    use crate::layouts::favourites;
    let dir = favourites::dir();
    match args.op {
        Op::List => {
            let all = favourites::list(&dir);
            pretty(&json!({
                "favourites": all.iter().map(|f| json!({
                    "name": f.name,
                    "layout": super::layouts::describe(&f.layout),
                })).collect::<Vec<_>>(),
                "note": "Built-in layouts, and saving or using one: the `layouts` tool.",
            }))
        }
        Op::Show => {
            let name = args.name.ok_or("Give the favourite's `name`")?;
            let (name, layout) = favourites::named(&dir, &name)
                .ok_or_else(|| format!("There is no favourite layout \"{name}\""))?;
            pretty(&json!({ "name": name, "layout": layout }))
        }
        Op::Remove => {
            let name = args.name.ok_or("Give the favourite's `name`")?;
            let left = favourites::remove(&dir, &name)?;
            text(format!(
                "Forgot \"{name}\"; {} left.",
                plural(left.len(), "favourite")
            ))
        }
        _ => Err("Use or save a layout with the `layouts` tool".into()),
    }
}
