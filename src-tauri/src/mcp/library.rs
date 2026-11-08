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
    Sound,
    Music,
    Image,
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
    Propose,
    Versions,
    Restore,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LibraryArgs {
    /// style, preset, layout, sound, music or image.
    pub kind: Kind,
    /// list (default), show, use (in this project), set, remove; each kind's others are in
    /// the tool's description.
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
    /// Presets, save: this edit, as the user approved it, becomes a saved workflow. When the
    /// usual steps don't apply, one each, in the user's words.
    #[serde(default)]
    pub exceptions: Option<Vec<String>>,
    /// Presets, save: what must hold before export, one each.
    #[serde(default)]
    pub checks: Option<Vec<String>>,
    /// Presets, save: what the user approved, in a line.
    #[serde(default)]
    pub note: Option<String>,
    /// Presets, restore: which version (`versions` lists them).
    #[serde(default)]
    pub version: Option<u32>,
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
    /// Styles, propose: image or clip files (full paths), or graphics library items (id or
    /// name), to make a style from.
    #[serde(default)]
    pub references: Vec<String>,
    /// Styles, propose: the user's own words about the look they want.
    #[serde(default)]
    pub brief: Option<String>,
    /// Sounds and music, list: words its name or tags hold (`whoosh`, `riser hit`, `calm`).
    #[serde(default)]
    pub query: Option<String>,
    /// Sounds and music, use: where it starts (edited microseconds)...
    #[serde(default)]
    pub at_us: Option<u64>,
    /// ...or at a word (id from transcript_get).
    #[serde(default)]
    pub at_word: Option<String>,
    #[serde(default)]
    pub until_us: Option<u64>,
    /// Images, use: the graphic (id) the picture goes into.
    #[serde(default)]
    pub graphic: Option<String>,
    /// Images, list: broadsheet, light, dark, editorial, neon or all; default the project
    /// style's.
    #[serde(default)]
    pub look: Option<String>,
}

pub static TOOLS: [ToolDef; 1] = [ToolDef {
    name: "library",
    description: "The user's styles (how graphics, captions and the frame look and move: \
        brand, graphics look, captions, frame, 9:16 variant, motion, kinds, examples, look \
        rules, references, sounds), presets (what the app does for a kind of video: cleanup, \
        captions, zooms, layouts, shorts, export, the method you follow) and favourite \
        layouts, kept for every project. `op`: list, show (a style's brief: read it before \
        making graphics), use (this project takes it: one undo step), set (change fields \
        app-wide, every project using it follows: ask the user first; a built-in goes into a \
        copy), remove (at the user's word). Styles also: preview (stills of the video's \
        graphics with `set` applied, changing nothing: show the user before set), check (the \
        style's fonts, examples and sounds; with a project, graphics not following it), \
        bring_in_line (one batch rewriting those graphics) and propose (a new style from \
        `references`: shows them to you and the steps; saves nothing). \
        Presets also: save (the project's settings into its preset, or a new one: \
        `saveAs`; with `exceptions`, `checks`, `note` it is a workflow the user approved on this \
        edit: only at their word), versions and restore (`version`: each change to one is a new \
        version), reset (the project's changes back), propose_rule (`rule`, `example`: the \
        user accepts it in the Agent panel). Sounds (CC0 whooshes, risers, hits, clicks, \
        pops; the user's own): list (`query`), use (`name` = id, at `atUs`/`atWord`). Music \
        (CC0 beds by mood): the same, until `untilUs` or the end, kept under the speech. \
        Images (still pictures for graphics, one set per look so a video keeps one; sourced \
        flags, maps, wordmarks and portraits with every look): list (the project style's look \
        unless `look`; no `query`: its groups; `query`: what the beat shows), show (`name` = \
        a picture or a group: a board of them), use (`name` into `graphic`: copied into its \
        folder as one undo step).",
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
        Kind::Sound => sound(ctx, args),
        Kind::Music => music(ctx, args),
        Kind::Image => image(ctx, args),
    }
}

/// One picture as a line: id, size, what it depicts, where it goes; * when its licence asks
/// for credit or isn't known.
fn image_line(image: &crate::image_library::Image) -> String {
    let size = match (image.width, image.height) {
        (Some(w), Some(h)) => format!("{w}x{h}"),
        _ => "svg".into(),
    };
    format!(
        "{}{} {size}: {}. {}",
        image.id,
        if image.free() { "" } else { "*" },
        image.depicts.trim_end_matches('.'),
        image.placement
    )
}

const UNFREE_NOTE: &str = "* real material from elsewhere whose licence asks for credit or a \
    check (its `rights` in show): not without the user's word, and credit as it says.";

/// The looks `list` shows, and why: `look` (or `all`), else the project style's, else all.
fn image_looks(ctx: &Ctx, look: Option<&str>) -> Result<(Vec<String>, String), String> {
    use crate::image_library as images;
    let all = || {
        images::catalog()
            .looks
            .iter()
            .map(|l| l.id.clone())
            .collect::<Vec<_>>()
    };
    match look.map(str::trim).filter(|l| !l.is_empty()) {
        Some(l) if l.eq_ignore_ascii_case("all") => Ok((all(), "every look".into())),
        Some(l) => {
            let found = images::look(l).ok_or_else(|| {
                format!("No look {l}; the looks are {} (or all)", all().join(", "))
            })?;
            Ok((vec![found.id.clone()], format!("look {}", found.id)))
        }
        None => match project_style(ctx) {
            Some(name) => {
                let style = crate::style::store::resolve(&name);
                let found = images::look_for(&style);
                Ok((
                    vec![found.id.clone()],
                    format!(
                        "look {} (style {}'s; `look` for another, all for every one)",
                        found.id, style.name
                    ),
                ))
            }
            None => Ok((all(), "every look (no project open)".into())),
        },
    }
}

fn image(ctx: &Ctx, args: LibraryArgs) -> Result<Vec<Output>, String> {
    use crate::image_library as images;
    match args.op {
        Op::List => {
            let (looks, why) = image_looks(ctx, args.look.as_deref())?;
            let wanted: Vec<&str> = looks.iter().map(String::as_str).collect();
            let query = args.query.as_deref().unwrap_or_default();
            if query.trim().is_empty() {
                let shown = |look: &str| wanted.contains(&look) || look == images::SOURCED;
                let mut lines = vec![format!(
                    "Pictures of {why}, in groups (group, count, matched on; `query` finds \
                     pictures, `show` a group shows them):"
                )];
                for look in images::catalog().looks.iter().filter(|l| shown(&l.id)) {
                    lines.push(format!("{}: {}", look.id, look.description));
                    for topic in images::catalog()
                        .topics
                        .iter()
                        .filter(|t| t.look == look.id)
                    {
                        if topic.id != look.id {
                            lines.push(format!(" {}: {}", topic.id, topic.description));
                        }
                        lines.extend(topic.groups.iter().map(|g| {
                            format!(
                                "  {}/{} {}{}: {}",
                                topic.id,
                                g.id,
                                g.count,
                                if g.sourced { "*" } else { "" },
                                if g.terms.is_empty() {
                                    g.description.clone()
                                } else {
                                    g.terms.join(", ")
                                }
                            )
                        }));
                    }
                }
                lines.push(UNFREE_NOTE.into());
                return text(lines.join("\n"));
            }
            let found = images::search(query, Some(&wanted));
            if found.is_empty() {
                return text(format!(
                    "No picture of {why} matches \"{query}\". `list` with no query shows its \
                     groups."
                ));
            }
            let mut lines = vec![format!(
                "{} of {why} for \"{query}\", best first{}:",
                plural(found.len(), "picture"),
                if found.len() > images::MAX_FOUND {
                    format!(" (the first {})", images::MAX_FOUND)
                } else {
                    String::new()
                }
            )];
            lines.extend(found.iter().take(images::MAX_FOUND).map(|i| image_line(i)));
            if found.iter().any(|i| !i.free()) {
                lines.push(UNFREE_NOTE.into());
            }
            text(lines.join("\n"))
        }
        Op::Show => {
            let name = args
                .name
                .ok_or("Give a picture's id or a group's (as `list` names them) as `name`")?;
            let shown: Vec<&images::Image> = match images::group(&name) {
                Some((_, _, found)) => found,
                None => vec![images::find(&name)?],
            };
            let mut lines: Vec<String> = shown.iter().map(|i| image_line(i)).collect();
            for image in shown.iter().filter(|i| !i.free()) {
                lines.push(format!(
                    "{}: {} ({}).",
                    image.id,
                    image.rights.as_deref().unwrap_or("its licence isn't known"),
                    image.licence.source.as_deref().unwrap_or("no source")
                ));
            }
            let board = images::board(&shown)?;
            Ok(vec![Output::Text(lines.join("\n")), Output::Jpeg(board)])
        }
        Op::Use => {
            let image = images::find(
                args.name
                    .as_deref()
                    .ok_or("Give the picture's id as `name`")?,
            )?;
            let graphic = args
                .graphic
                .ok_or("Give the `graphic` (id) the picture goes into")?;
            let before = opened(ctx)?;
            commands::graphics::project_graphic_write_file_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                graphic.clone(),
                &image.src(),
                image.bytes(),
            )?;
            let mut lines = vec![format!(
                "Copied {} into graphic {graphic}: use it as src=\"{}\"{}.",
                image.id,
                image.src(),
                match (image.width, image.height) {
                    (Some(w), Some(h)) => format!(" ({w}x{h})"),
                    _ => String::new(),
                }
            )];
            if !image.free() {
                lines.push(format!(
                    "Its licence asks: {}. Only with the user's word; `check` flags it.",
                    image.rights.as_deref().unwrap_or("its licence isn't known")
                ));
            }
            let (ours, _) = image_looks(ctx, None)?;
            if image.look != images::SOURCED && !ours.contains(&image.look) {
                lines.push(format!(
                    "It is drawn for the {} look, not this project's ({}): a video keeps one \
                     look, so `check` flags it.",
                    image.look,
                    ours.join(", ")
                ));
            }
            text(applied(ctx, "library", &before, lines)?)
        }
        _ => Err("Images take `op` list, show or use".into()),
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
        Op::Show => text(brief(&store::load(&named())?)),
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
            // A style can't name a font it doesn't carry: a font changed here must be there.
            let current = store::load(&named())?;
            let next = current.patched(&patch)?;
            let folder = store::folder_of(&current);
            for ((which, font, weights), (_, before, _)) in
                next.font_weights().into_iter().zip(current.font_weights())
            {
                if font != before {
                    crate::captions::fonts::availability(font, folder.as_deref(), &weights)
                        .map_err(|why| format!("Not saved: the {which} font. {why}"))?;
                }
            }
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
            let style = store::load(&named())?;
            let mut out = json!({
                "style": style.name,
                "problems": problems(&style),
            });
            // With a project open, also its graphics that don't follow its style.
            if let Ok(before) = opened(ctx) {
                let plans = commands::graphics::graphics_in_line_plan_impl(
                    ctx.state,
                    before.handle.clone(),
                )?;
                let view = commands::style::styles_impl(ctx.state, Some(before.handle))?;
                out["notInLine"] = json!(plans
                    .iter()
                    .map(|p| json!({ "graphic": p.graphic, "name": p.name, "changes": p.changes }))
                    .collect::<Vec<_>>());
                out["ownStyleOrTokens"] = json!(view
                    .own
                    .iter()
                    .map(|(name, style, tokens)| json!({
                        "name": name, "style": style, "tokens": tokens,
                    }))
                    .collect::<Vec<_>>());
            }
            pretty(&out)
        }
        // One undo step at once, like every pass: no Accept card for graphics.
        Op::BringInLine => {
            let before = opened(ctx)?;
            let plans =
                commands::graphics::graphics_in_line_plan_impl(ctx.state, before.handle.clone())?;
            if plans.is_empty() {
                return Err(
                    "Every graphic follows the video's look already: nothing to bring in line"
                        .into(),
                );
            }
            let mut lines = vec![format!(
                "Brought {} in line with the style; each renders again.",
                plural(plans.len(), "graphic")
            )];
            lines.extend(plans.iter().map(|p| p.line()));
            commands::graphics::project_graphics_in_line_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                plans,
            )?;
            text(applied(ctx, "bring_in_line", &before, lines)?)
        }
        Op::Propose => {
            let out = propose(&args.references, args.brief.as_deref())?;
            // The Style panel's request is taken up (STY-3).
            crate::style::request::clear(&ctx.host.config_dir());
            Ok(out)
        }
        Op::Save | Op::Reset | Op::ProposeRule | Op::Versions | Op::Restore => Err(
            "Styles take list, show, use, set, remove, preview, check, bring_in_line or propose"
                .into(),
        ),
    }
}

const MAX_REFERENCES: usize = 8;
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];
const CLIP_EXTENSIONS: &[&str] = &["mp4", "mov", "webm", "mkv", "m4v"];
/// A clip is read at two frames a second (the rate references are read at), up to a board.
const BOARD_FRAMES: u32 = 24;

/// The first step of a style from references: each reference as the agent can see it (an
/// image, a clip as one board of frames, a library graphic's contact sheet and source), then
/// what to do next. Nothing is made or saved.
fn propose(references: &[String], brief: Option<&str>) -> Result<Vec<Output>, String> {
    if references.is_empty() {
        return Err(
            "Give `references`: image or clip files (full paths), or graphics library \
             items (id or name). If the user has none, ask for one; you can still work from \
             their words."
                .into(),
        );
    }
    if references.len() > MAX_REFERENCES {
        return Err(format!("Give at most {MAX_REFERENCES} references"));
    }
    let library = crate::graphics::library::dir();
    let mut lines = vec!["References, in the order of the images below:".to_string()];
    let mut images = Vec::new();
    for (i, reference) in references.iter().enumerate() {
        let n = i + 1;
        let reference = reference.trim();
        if let Ok(item) = crate::graphics::library::view(&library, reference) {
            let source = std::path::Path::new(&item.folder).join(crate::graphics::ENTRY);
            let sheet = item.sheet.as_ref().and_then(|s| std::fs::read(s).ok());
            lines.push(format!(
                "{n}. Library graphic {} ({}): {} Source: {}{}",
                item.item.label,
                item.id,
                item.item.use_when,
                source.display(),
                if sheet.is_some() {
                    " (its contact sheet below)"
                } else {
                    " (no contact sheet: read its source)"
                }
            ));
            images.extend(sheet.map(Output::Png));
            continue;
        }
        let path = std::path::Path::new(reference);
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if !path.is_absolute() || !path.is_file() {
            lines.push(format!(
                "{n}. {reference}: no such file (give a full path) or library graphic"
            ));
        } else if ext == "svg" {
            lines.push(format!("{n}. {reference}: an SVG, read it as a file"));
        } else if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            images.push(Output::Jpeg(crate::media::ffmpeg::still_jpeg(path, 1280)?));
            lines.push(format!("{n}. Image {reference}"));
        } else if CLIP_EXTENSIONS.contains(&ext.as_str()) {
            let length = crate::media::ffmpeg::duration_us(path)? as f64 / 1e6;
            let every = (length / BOARD_FRAMES as f64).max(0.5);
            let frames = ((length / every).floor() as u32).clamp(1, BOARD_FRAMES);
            images.push(Output::Jpeg(crate::media::ffmpeg::board_jpeg(
                path, every, frames, 320, 6,
            )?));
            lines.push(format!(
                "{n}. Clip {reference} ({length:.1} s): a board of {frames} frames, one every \
                 {every:.2} s from its start, left to right in rows of 6"
            ));
        } else {
            lines.push(format!(
                "{n}. {reference}: not an image ({}) or clip ({})",
                IMAGE_EXTENSIONS.join(", "),
                CLIP_EXTENSIONS.join(", ")
            ));
        }
    }
    if let Some(brief) = brief.map(str::trim).filter(|b| !b.is_empty()) {
        lines.push(format!("\nThe user's words: {brief}"));
    }
    lines.push(
        "\nNothing is made yet. Next:\n\
         1. Breakdown: a short table of what you see (palette, type, motion moves, kinds of \
         graphics, what to leave out and why). Show it to the user before making anything.\n\
         2. Up to three styles that differ in a named way (one when the references agree), \
         each a `set` patch on the closest style: colours, fonts that are there, the look, \
         motion rules, kinds, look rules. Show each with `op: preview` on this video's \
         graphics; with none, add one at a typical moment first.\n\
         3. On the user's pick only: `op: set` with `saveAs` (a new name), `references` \
         ([{\"file\": full path, \"note\": what it gives}]; library graphics go in `examples`) \
         and `breakdown`. Then `op: check`."
            .into(),
    );
    let mut out = vec![Output::Text(lines.join("\n"))];
    out.extend(images);
    Ok(out)
}

/// A library item as an example line: what it is, its kind, when to use it and its source.
fn example_line(example: &crate::style::Example) -> String {
    let kind = if example.kind.is_empty() {
        String::new()
    } else {
        format!(", {}", example.kind)
    };
    match crate::graphics::library::view(&crate::graphics::library::dir(), &example.graphic) {
        Ok(item) => format!(
            "- {} ({}{kind}): {} Source: {}",
            item.item.label,
            item.id,
            item.item.use_when,
            std::path::Path::new(&item.folder)
                .join(crate::graphics::ENTRY)
                .display()
        ),
        Err(_) => format!("- {}{kind}: not in the graphics library", example.graphic),
    }
}

fn kind_line(kind: &crate::style::GraphicKind) -> String {
    let short = kind
        .shorts
        .map(|slot| format!(" In a short: {}.", slot.describe()))
        .unwrap_or_default();
    format!("- {}: {}{short}", kind.name, kind.when)
}

fn section(out: &mut Vec<String>, title: &str, lines: Vec<String>) {
    if !lines.is_empty() {
        out.push(format!("\n## {title}"));
        out.extend(lines);
    }
}

fn bullets(items: &[String]) -> Vec<String> {
    items.iter().map(|i| format!("- {i}")).collect()
}

/// What an agent reads before it makes a graphic in `style`: its settings, how its graphics
/// move, the kinds it uses, its examples, its rules and sounds, then its fields for `set`.
fn brief(style: &crate::style::Style) -> String {
    use crate::style::store;
    let c = &style.brand.colors;
    let fonts: Vec<String> = style
        .fonts_status()
        .into_iter()
        .map(|(which, status)| match status {
            Ok(how) => format!("{which} font {how}"),
            Err(why) => format!("{which} font NOT AVAILABLE: {why}"),
        })
        .collect();
    let look = &style.graphics;
    let tokens = look
        .values()
        .into_iter()
        .filter(|(name, _)| *name != "font")
        .map(|(name, value)| format!("{name} {value}"))
        .collect::<Vec<_>>()
        .join("; ");
    let mut out = vec![
        format!(
            "# Style {}{}",
            style.name,
            if crate::style::is_builtin(&style.name) {
                " (built in)"
            } else {
                ""
            }
        ),
        format!(
            "Colours: background {}, text {}, accent {}, muted {}. Fonts: {}. Logo: {}.",
            c.background,
            c.text,
            c.accent,
            c.muted,
            if fonts.is_empty() {
                "the system's".to_string()
            } else {
                fonts.join("; ")
            },
            style.brand.logo.as_deref().unwrap_or("none")
        ),
        format!(
            "Graphics look: {} (size {}, roundness {}, speed {}, energy {}, density {}). \
             Tokens, as var(--look-NAME): {tokens}.",
            look.preset, look.size, look.roundness, look.speed, look.energy, look.density
        ),
    ];
    if style.motion.is_empty() && style.kinds.is_empty() && style.examples.is_empty() {
        out.push(
            "It has no motion rules, kinds or examples yet: only a look. Adding them changes \
             the user's style, so ask first."
                .into(),
        );
    }
    section(&mut out, "Motion", bullets(&style.motion));
    section(
        &mut out,
        "Kinds of graphics (pick per moment from these)",
        style.kinds.iter().map(kind_line).collect(),
    );
    section(
        &mut out,
        "Examples (read their source and adapt it)",
        style.examples.iter().map(example_line).collect(),
    );
    section(&mut out, "Look rules", bullets(&style.rules));
    let mut sound = Vec::new();
    if !style.sound.policy.is_empty() {
        sound.push(style.sound.policy.clone());
    }
    if !style.sound.cues.is_empty() {
        let library = crate::sound_library::list(&crate::sound_library::dir()).unwrap_or_default();
        sound.extend(style.sound.cues.iter().map(|cue| {
            match library.iter().find(|s| s.id == cue.sound) {
                Some(s) => format!(
                    "- {}: {} ({}, {:.2} s)",
                    cue.role,
                    s.id,
                    s.name,
                    s.duration_us as f64 / 1e6
                ),
                None => format!("- {}: {}, not in the sound library", cue.role, cue.sound),
            }
        }));
        sound.push(
            "Only when the user asks for sounds on graphics: place one with `library` \
             `kind: sound` `op: use` (`name` = its id, `atWord` on the graphic's event); not \
             every event gets one."
                .into(),
        );
    }
    section(&mut out, "Sound", sound);
    let folder = store::folder_of(style);
    section(
        &mut out,
        "References",
        style
            .references
            .iter()
            .map(|r| {
                let path = folder
                    .as_ref()
                    .map(|f| f.join(&r.file).display().to_string())
                    .unwrap_or_else(|| r.file.clone());
                if r.note.is_empty() {
                    format!("- {path}")
                } else {
                    format!("- {path}: {}", r.note)
                }
            })
            .collect(),
    );
    if !style.breakdown.is_empty() {
        section(&mut out, "Breakdown", vec![style.breakdown.clone()]);
    }
    let p = &style.portrait;
    let mut short = vec![format!(
        "Look × size {}, energy {}, speed {}; captions {}.",
        p.graphics.size,
        p.graphics.energy,
        p.graphics.speed,
        p.caption_style.unwrap_or(style.captions.style).label()
    )];
    short.extend(bullets(&p.motion));
    short.extend(p.kinds.iter().map(kind_line));
    short.extend(p.examples.iter().map(example_line));
    section(&mut out, "In a short (9:16)", short);
    // The parts written out above aren't repeated as fields.
    let settings = crate::style::Style {
        motion: Vec::new(),
        kinds: Vec::new(),
        examples: Vec::new(),
        rules: Vec::new(),
        references: Vec::new(),
        breakdown: String::new(),
        sound: Default::default(),
        portrait: crate::style::Portrait {
            motion: Vec::new(),
            kinds: Vec::new(),
            examples: Vec::new(),
            ..style.portrait.clone()
        },
        ..style.clone()
    };
    out.push(format!(
        "\nFields, for `set` (motion, kinds, examples, rules, references, breakdown and sound \
         as above; a list given replaces the whole list): {}\nLook presets: {}. Backgrounds: {}.",
        serde_json::to_string(&settings).unwrap_or_default(),
        crate::graphics::look::presets()
            .iter()
            .map(|p| p.name)
            .collect::<Vec<_>>()
            .join(", "),
        crate::project::layout::BACKGROUND_PRESETS.join(", ")
    ));
    out.join("\n")
}

/// What is wrong with `style` itself: fonts that aren't there, examples and sounds missing
/// from their libraries, reference files gone.
fn problems(style: &crate::style::Style) -> Vec<String> {
    let mut out: Vec<String> = style
        .fonts_status()
        .into_iter()
        .filter_map(|(which, status)| status.err().map(|why| format!("The {which} font: {why}")))
        .collect();
    let library = crate::graphics::library::dir();
    for example in style.examples.iter().chain(&style.portrait.examples) {
        match crate::graphics::library::view(&library, &example.graphic) {
            Err(_) => out.push(format!(
                "The example {} is not in the graphics library",
                example.graphic
            )),
            Ok(item) if !item.item.general => out.push(format!(
                "The example {} was saved as is (failed: {})",
                item.item.label,
                item.item.failed.join(", ")
            )),
            Ok(_) => {}
        }
    }
    let sounds = crate::sound_library::dir();
    for cue in &style.sound.cues {
        if crate::sound_library::find(&sounds, &cue.sound).is_err() {
            out.push(format!(
                "The {} cue's sound {} is not in the sound library",
                cue.role, cue.sound
            ));
        }
    }
    if let Some(folder) = crate::style::store::folder_of(style) {
        for reference in &style.references {
            if !folder.join(&reference.file).is_file() {
                out.push(format!(
                    "The reference {} is missing from the style's folder",
                    reference.file
                ));
            }
        }
    }
    out
}

fn sound(ctx: &Ctx, args: LibraryArgs) -> Result<Vec<Output>, String> {
    use crate::commands::sounds;
    match args.op {
        Op::List => {
            let found = sounds::sound_library_list_impl(args.query.clone())?;
            let mut lines = vec![format!(
                "{} (id, length, tags; * needs credit or unknown licence: not without the user's \
                 word):",
                plural(found.len(), "sound")
            )];
            lines.extend(found.iter().map(|s| {
                format!(
                    "{}{} {:.2}s {}",
                    s.id,
                    if s.licence.kind.needs_no_credit() {
                        ""
                    } else {
                        "*"
                    },
                    s.duration_us as f64 / 1e6,
                    s.tags.join(",")
                )
            }));
            text(lines.join("\n"))
        }
        Op::Use => {
            let id = args
                .name
                .ok_or("Give the sound's id as `name` (op: list)")?;
            let at = match (args.at_us, args.at_word) {
                (Some(at), _) => at,
                (None, Some(word)) => super::actions::word_edited_us(ctx, None, &word)?,
                (None, None) => return Err("Give `atUs` or `atWord`".into()),
            };
            let before = opened(ctx)?;
            let placed = sounds::project_sound_place_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                id.clone(),
                at,
                None,
            )?;
            let line = format!(
                "Placed {id} at {} as clip {} (media {}).",
                super::tools::clock(at),
                placed.clip_id,
                placed.asset_id
            );
            text(applied(ctx, "library", &before, vec![line])?)
        }
        _ => Err("Sounds take `op: list` or `op: use`; the user adds their own in the app".into()),
    }
}

fn music(ctx: &Ctx, args: LibraryArgs) -> Result<Vec<Output>, String> {
    use crate::commands::music;
    match args.op {
        Op::List => {
            let found = music::music_library_list_impl(args.query.clone())?;
            let mut lines = vec![format!(
                "{} (id, length, moods; ~ downloads on first use):",
                plural(found.len(), "track")
            )];
            lines.extend(found.iter().map(|t| {
                let s = t.duration_us / 1_000_000;
                format!(
                    "{}{} {}:{:02} {}",
                    t.id,
                    if t.downloaded { "" } else { "~" },
                    s / 60,
                    s % 60,
                    t.tags.join(",")
                )
            }));
            text(lines.join(
                "
",
            ))
        }
        Op::Use => {
            let id = args
                .name
                .ok_or("Give the track's id as `name` (op: list)")?;
            let at = match (args.at_us, args.at_word) {
                (Some(at), _) => at,
                (None, Some(word)) => super::actions::word_edited_us(ctx, None, &word)?,
                (None, None) => 0,
            };
            let before = opened(ctx)?;
            let placed = music::project_music_place_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                id.clone(),
                at,
                args.until_us,
                None,
            )?;
            let bed = &placed.bed;
            let mut line = format!(
                "{id} plays {}-{} on the Music track ({}, media {}).",
                super::tools::clock(bed.start_us),
                super::tools::clock(bed.end_us),
                plural(bed.clip_ids.len(), "clip"),
                placed.asset_id
            );
            if let Some(db) = bed.under_speech_db {
                line.push_str(&format!(" Music now sits {db} dB under the speech."));
            }
            text(applied(ctx, "library", &before, vec![line])?)
        }
        _ => Err("Music takes `op: list` or `op: use`; the user adds their own in the app".into()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn said(output: Vec<Output>) -> String {
        match output.into_iter().next() {
            Some(Output::Text(text)) => text,
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn an_agent_finds_a_sound_and_places_it() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        let list = said(library(&ctx, json!({ "kind": "sound", "query": "whoosh" })).unwrap());
        assert!(list.starts_with("4 sounds"), "{list}");
        assert!(list.contains("\nwhoosh-whip 0.38s "), "{list}");
        let placed = said(
            library(
                &ctx,
                json!({ "kind": "sound", "op": "use", "name": "whoosh-fast", "atUs": 1_000_000 }),
            )
            .unwrap(),
        );
        assert!(
            placed.starts_with("Placed whoosh-fast at 0:01.00"),
            "{placed}"
        );
        assert!(library(&ctx, json!({ "kind": "sound", "op": "use", "name": "pop" })).is_err());
    }

    #[test]
    fn an_agent_finds_a_picture_and_copies_it_into_a_graphic() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        // The lesson wears Light, so it sees the light look and the sourced pictures.
        let groups = said(library(&ctx, json!({ "kind": "image" })).unwrap());
        assert!(groups.contains("look light (style Light's"), "{groups}");
        assert!(groups.contains("\n  light/devices 6: "), "{groups}");
        assert!(groups.contains("\n  sourced/people 4*: "), "{groups}");
        assert!(!groups.contains("broadsheet/"), "{groups}");
        let laptop = said(library(&ctx, json!({ "kind": "image", "query": "a laptop" })).unwrap());
        assert!(
            laptop.contains("\nlight/devices/laptop 1024x768: "),
            "{laptop}"
        );
        assert!(!laptop.contains("neon/"), "{laptop}");
        let found = said(
            library(
                &ctx,
                json!({ "kind": "image", "query": "server cooling fan", "look": "broadsheet" }),
            )
            .unwrap(),
        );
        assert!(
            found.contains("\nbroadsheet/ai-assets/compute/server-cooling-fan 1024x768: "),
            "{found}"
        );
        assert!(library(&ctx, json!({ "kind": "image", "look": "pastel" })).is_err());
        let shown = library(
            &ctx,
            json!({ "kind": "image", "op": "show", "name": "broadsheet/ai-assets/compute" }),
        )
        .unwrap();
        assert!(matches!(&shown[1], Output::Jpeg(j) if j.starts_with(&[0xFF, 0xD8])));
        let tool = crate::mcp::tools::find("graphic_add").unwrap();
        (tool.run)(
            &ctx,
            json!({ "template": "blank", "atUs": 0, "lengthUs": 1_000_000 }),
        )
        .unwrap();
        let graphic = with_reader(ctx.state, |r| {
            Ok(r.document()
                .assets
                .iter()
                .find(|a| a.is_graphic())
                .unwrap()
                .id
                .clone())
        })
        .unwrap();
        let used = said(
            library(
                &ctx,
                json!({ "kind": "image", "op": "use", "name": "sam-altman", "graphic": graphic }),
            )
            .unwrap(),
        );
        assert!(
            used.contains("use it as src=\"images/sourced-sam-altman.webp\""),
            "{used}"
        );
        assert!(used.contains("CC BY-SA 4.0"), "{used}");
        assert!(!used.contains("look"), "{used}");
        let stranger = said(
            library(
                &ctx,
                json!({ "kind": "image", "op": "use", "name": "server-rack-open", "graphic": graphic }),
            )
            .unwrap(),
        );
        assert!(
            stranger.contains("drawn for the broadsheet look, not this project's (light)"),
            "{stranger}"
        );
        let report = with_reader(ctx.state, |r| {
            Ok(crate::check::run(r.root(), r.document(), &[]))
        })
        .unwrap();
        assert!(report
            .findings
            .iter()
            .any(|f| f.kind == crate::check::Kind::ImageLicence
                && f.detail.contains("sourced-sam-altman.webp")));
        assert!(report
            .findings
            .iter()
            .any(|f| f.kind == crate::check::Kind::ImageLook
                && f.detail.contains("broadsheet-server-rack-open.webp")));
    }

    #[test]
    fn a_style_proposal_shows_each_reference_and_saves_nothing() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("board.png");
        image::RgbaImage::from_pixel(1600, 900, image::Rgba([250, 250, 250, 255]))
            .save(&image)
            .unwrap();
        let clip = dir.path().join("reel.mp4");
        let made = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
            .arg("testsrc=size=640x360:rate=30:duration=4")
            .args(["-pix_fmt", "yuv420p"])
            .arg(&clip)
            .status()
            .unwrap();
        assert!(made.success());
        let host = crate::mcp::tests_support::BareHost::default();
        let ctx = Ctx::new(&host, "agent", false);
        let out = library(
            &ctx,
            json!({ "kind": "style", "op": "propose", "brief": "calm, lots of white",
                "references": [image.display().to_string(), clip.display().to_string(), "nothing.png"] }),
        )
        .unwrap();
        let text = match &out[0] {
            Output::Text(text) => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(
            text.contains("1. Image ") && text.contains("calm, lots of white"),
            "{text}"
        );
        // Four seconds at two frames a second.
        assert!(
            text.contains("(4.0 s): a board of 8 frames, one every 0.50 s"),
            "{text}"
        );
        assert!(text.contains("3. nothing.png: no such file"), "{text}");
        assert!(text.contains("Breakdown") && text.contains("On the user's pick only"));
        let jpegs = out[1..]
            .iter()
            .filter(|o| matches!(o, Output::Jpeg(b) if b.starts_with(&[0xFF, 0xD8])))
            .count();
        assert_eq!(jpegs, 2, "the image and the clip's board");
        assert!(library(&ctx, json!({ "kind": "style", "op": "propose" })).is_err());
    }
}
