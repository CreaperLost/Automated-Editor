//! Presets through `library`: what the app and the agent do for a kind of video, and what the
//! open project changed from its preset.
use super::actions::{applied, opened};
use super::library::{LibraryArgs, Op};
use super::tools::{pretty, text, with_reader, Ctx, Output};
use crate::commands;
use crate::commands::preset::PresetTarget;
use serde_json::json;

pub fn library(ctx: &Ctx, args: LibraryArgs) -> Result<Vec<Output>, String> {
    use crate::preset::store;
    let ours = with_reader(ctx.state, |reader| Ok(reader.document().preset.clone())).ok();
    let named = || {
        args.name
            .clone()
            .or_else(|| ours.clone())
            .unwrap_or_else(|| crate::preset::DEFAULT.into())
    };
    match args.op {
        Op::List => {
            let view = commands::preset::presets_impl(
                ctx.state,
                with_reader(ctx.state, |r| Ok(r.summary.project_handle.clone())).ok(),
            )?;
            pretty(&json!({
                "presets": view.presets.iter().map(|p| json!({
                    "name": p.name,
                    "builtIn": p.builtin,
                    "description": p.preset.description,
                    "thisProject": view.project_preset.as_deref() == Some(p.name.as_str()),
                    "thisProjectsShorts": view.shorts_preset.as_deref() == Some(p.name.as_str()),
                    "thisProjectsClips": view.clips_preset.as_deref() == Some(p.name.as_str()),
                })).collect::<Vec<_>>(),
                "changedHere": view.changes,
                "folder": view.folder,
            }))
        }
        Op::Show => {
            let preset = store::load(&named())?;
            pretty(&json!({
                "name": preset.name,
                "builtIn": crate::preset::is_builtin(&preset.name),
                "description": preset.description,
                "style": preset.style,
                "values": preset.values,
                "short": preset.short,
                "choices": preset.choices,
                "rules": preset.rules,
                "method": format!("{} characters (the `preset` tool gives it)", preset.method.len()),
            }))
        }
        Op::Use => {
            let name = args.name.ok_or("Give the preset's `name`")?;
            let name = store::load(&name)?.name;
            let before = opened(ctx)?;
            commands::preset::project_preset_set_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                name.clone(),
                args.target,
            )?;
            text(applied(
                ctx,
                "library",
                &before,
                vec![match args.target {
                    PresetTarget::Shorts => {
                        format!(
                            "New shorts (from a take) of this project start from preset {name}."
                        )
                    }
                    PresetTarget::Clips => {
                        format!("New clips of this project start from preset {name}.")
                    }
                    PresetTarget::Video => format!(
                        "The project follows preset {name}; what it changed here stays changed."
                    ),
                }],
            )?)
        }
        Op::Set => {
            let patch = args.set.ok_or("Give the fields to change in `set`")?;
            let (saved, copy) = commands::preset::preset_patch_impl(
                ctx.state,
                &named(),
                &patch,
                args.save_as.as_deref(),
            )?;
            text(match copy {
                Some(copy) if args.save_as.is_none() => format!(
                    "{} is built in, so the change went into a copy, preset {copy}, which the \
                     project follows now. Every project following it follows the change.",
                    named()
                ),
                _ => format!(
                    "Saved preset {} for every project; projects following it follow the change.",
                    saved.name
                ),
            })
        }
        Op::Save => {
            let before = opened(ctx)?;
            commands::preset::preset_save_from_project_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                args.save_as.clone(),
            )?;
            let now = with_reader(ctx.state, |r| Ok(r.document().preset.clone()))?;
            text(applied(
                ctx,
                "library",
                &before,
                vec![format!(
                    "The project's settings are preset {now}'s now, for every project that \
                     follows it."
                )],
            )?)
        }
        Op::Reset => {
            let before = opened(ctx)?;
            commands::preset::project_preset_reset_impl(
                ctx.state,
                before.handle.clone(),
                before.revision,
                args.part.clone(),
                None,
            )?;
            text(applied(
                ctx,
                "library",
                &before,
                vec![format!(
                    "{} back to the preset's values.",
                    args.part.as_deref().unwrap_or("Every setting changed here")
                )],
            )?)
        }
        Op::Remove => {
            let name = args.name.ok_or("Give the preset's `name`")?;
            commands::preset::preset_remove_impl(ctx.state, name.clone())?;
            text(format!(
                "Removed preset {name}. Projects that followed it follow Tutorial."
            ))
        }
        Op::ProposeRule => {
            let rule = args.rule.clone().unwrap_or_default();
            let example = args.example.clone().unwrap_or_default();
            let agent = ctx.agent.clone();
            let ((id, preset), _) = crate::commands::proposals::with_proposals(
                ctx.state,
                ctx.project.as_deref(),
                |reader, proposals| {
                    let id = crate::proposals::propose_rule(
                        reader,
                        proposals,
                        &agent,
                        args.name.as_deref(),
                        &rule,
                        &example,
                    )?;
                    let preset = match &proposals.get(&id)?.change {
                        crate::proposals::Change::Rule { preset, .. } => preset.clone(),
                        _ => String::new(),
                    };
                    Ok((id, preset))
                },
            )?;
            ctx.changed_proposals();
            text(format!(
                "Proposed rule {id} for {preset}. It waits for the user in the Agent panel (never \
                 autopilot); once accepted it is in {preset}'s rules for every project."
            ))
        }
        _ => Err("Presets take list, show, use, set, save, reset, remove or propose_rule".into()),
    }
}
