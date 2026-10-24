//! The `aeroedits` skill: how an agent runs a whole edit (ask, apply, report, export), from the
//! preset for the kind of video (`crate::preset`: its method, the user's rules for it and its
//! choices). The text ships inside the app (and the user's presets), so it always matches the
//! actions, and reaches agents three ways: the `preset` tool (any client; the server's
//! instructions point to it), MCP prompts (the Claude app's + menu, slash commands in Claude
//! Code), and a skill folder installed for Claude Code and Codex from the AI agents dialog. The
//! user's answers to a method's questions are saved as the preset's choices (with `library`),
//! so the next edit asks one question and the app reads the same settings.
use super::tools::{params, schema, text, Ctx, Output, ToolDef};
use crate::preset::{self, Preset};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// The installed skill: what it is for, and a pointer to `preset`, which has the rest.
const SKILL: &str = include_str!("skill/SKILL.md");
/// The rules every edit follows, given once with the method.
const RULES: &str = include_str!("skill/rules.md");
const EXPORT: &str = include_str!("skill/export.md");
/// How to work with graphics, given with the `graphics` list (only edits with graphics need it).
const GRAPHICS: &str = include_str!("skill/graphics.md");

/// The graphics rules `graphics` gives: the `<!-- shorts -->` block (how graphics show in a
/// 9:16 short) only for a project with shorts.
pub fn graphics_guide(shorts: bool) -> String {
    let mut out = Vec::new();
    let mut dropping = false;
    for line in GRAPHICS.trim().lines() {
        match line.trim() {
            "<!-- shorts -->" => dropping = !shorts,
            "<!-- /shorts -->" => dropping = false,
            _ if !dropping => out.push(line.trim_end_matches('\r')),
            _ => {}
        }
    }
    out.join("\n").trim_end().to_string()
}
/// Files older versions installed beside SKILL.md; they are taken out so no agent reads them.
const LEGACY_FILES: [&str; 4] = [
    "export.md",
    "templates/tutorial.md",
    "templates/shorts.md",
    "templates/take-shorts.md",
];
/// The line that makes an installed skill folder AeroEdits' to update.
const MARKER: &str = "<!-- Installed by AeroEdits";

/// The preset called `name` (any case; a prompt's name, `my-gaming`, finds "My gaming").
fn find(name: &str) -> Result<Preset, String> {
    let wanted = name.trim();
    preset::store::load(wanted).or_else(|error| {
        preset::store::list()
            .into_iter()
            .find(|p| prompt_name(&p.name) == wanted.to_lowercase())
            .ok_or(error)
    })
}

/// A preset's name as a prompt's: lower case, `-` between words.
fn prompt_name(name: &str) -> String {
    crate::graphics::library::slug(name)
}

/// What a preset's method is told about choices of other names: who reviews fillers and who
/// transcribes are never kept (`review: auto`, `provider: auto` unless the user asks in the
/// conversation).
pub fn forget_review(choices: &mut serde_json::Map<String, Value>) {
    if let Some(fillers) = choices.get_mut("fillers").and_then(Value::as_object_mut) {
        fillers.remove("review");
    }
    choices.remove("transcriber");
}

/// Everything an agent needs for an edit of this kind: the rules, the method, the export
/// steps and the preset's choices (a prompt's whole guide, for a run step by step).
pub fn guide(name: &str) -> Result<String, String> {
    compose(name, false, true)
}

/// The guide `preset` gives: for an automatic run (`auto`) without what only a run step by
/// step needs (dry runs to ask about, the questions) nor the thorough review unless the
/// choices say so; the export steps only with `export` (asked for at the end). Agents pay for
/// every word of it, on every edit.
fn compose(name: &str, auto: bool, export: bool) -> Result<String, String> {
    let preset = find(name)?;
    let mut choices = preset.choices.clone();
    forget_review(&mut choices);
    let thorough = choices.get("review").and_then(Value::as_str) == Some("thorough");
    let chosen = if choices.is_empty() {
        format!(
            "## Choices\n\nNone in {} yet: ask the method's questions, then save the answers \
             as its choices with `library` (`kind: preset`, `op: set`, `set: {{\"choices\": \
             {{...}}}}`).",
            preset.name
        )
    } else {
        format!(
            "## Choices ({})\n\nThe preset's choices: run them as set without asking, one line \
             each on what ran. When the user answers otherwise, save them in this shape with \
             `library` (`kind: preset`, `op: set`, `set: {{\"choices\": {{...}}}}`; a built-in \
             preset goes into a copy the project then follows).\n\n```json\n{}\n```",
            preset.name,
            serde_json::to_string_pretty(&choices).unwrap_or_default()
        )
    };
    let own_rules = if preset.rules.trim().is_empty() {
        String::new()
    } else {
        format!(
            "## The user's rules for {}\n\nThey win over the method and the rules above.\n\n{}\n\n---\n\n",
            preset.name,
            preset.rules.trim()
        )
    };
    let export = if export {
        format!("{}\n\n---\n\n", EXPORT.trim())
    } else {
        String::new()
    };
    // A checkout with CRLF line ends embeds them.
    let whole = format!(
        "{}\n\n---\n\n{own_rules}{}\n\n---\n\n{export}{chosen}",
        RULES.trim(),
        preset.method.trim(),
    )
    .replace("\r\n", "\n");
    Ok(for_mode(&whole, auto, thorough, !choices.is_empty()))
}

/// `text` without the blocks this run doesn't need: `<!-- steps -->` (a run step by step only),
/// `<!-- auto -->` (an automatic run only), `<!-- thorough -->` (a thorough review only, kept
/// step by step since the user is asked), `<!-- nochoices -->` (only while the preset has no
/// choices: once it has, they are shown as they are). The marker lines never reach agents.
fn for_mode(text: &str, auto: bool, thorough: bool, has_choices: bool) -> String {
    let mut out = Vec::new();
    let mut dropping: Option<&str> = None;
    for line in text.lines() {
        let marker = line.trim();
        if let Some(kind) = marker
            .strip_prefix("<!-- /")
            .and_then(|m| m.strip_suffix(" -->"))
        {
            if dropping == Some(kind) {
                dropping = None;
            }
            continue;
        }
        if let Some(kind) = marker
            .strip_prefix("<!-- ")
            .and_then(|m| m.strip_suffix(" -->"))
            .filter(|k| matches!(*k, "steps" | "auto" | "thorough" | "nochoices"))
        {
            let drop = match kind {
                "steps" => auto,
                "auto" => !auto,
                "nochoices" => has_choices,
                _ => auto && !thorough,
            };
            if drop && dropping.is_none() {
                dropping = Some(kind);
            }
            continue;
        }
        if dropping.is_none() {
            out.push(line);
        }
    }
    let mut text = out.join("\n");
    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    text
}

// ---------------------------------------------------------------------------------------
// Tools.

pub static TOOLS: [ToolDef; 1] = [ToolDef {
    name: "preset",
    description: "How to run a whole edit: the preset for a kind of video (its method: the \
        steps, what to ask the user, the defaults; the user's own rules for it; its saved \
        choices) and the general rules. Call it first when the user asks for an edit, with \
        `auto: true` when they want it done automatically; `part: export` gives the export \
        steps. Without `name`, the list (the open project's preset first).",
    schema: schema::<PresetArgs>,
    read_only: true,
    run: preset_tool,
}];

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct PresetArgs {
    /// The preset, such as `tutorial`. Without it, the list of presets.
    #[serde(default)]
    pub name: Option<String>,
    /// The user wants it done automatically: the guide without the questions.
    #[serde(default)]
    pub auto: bool,
    /// `export`: only the export steps.
    #[serde(default)]
    pub part: Option<String>,
}

fn preset_tool(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: PresetArgs = params(args)?;
    // A workflow begins: what the agent changes from here is a new run, undone on its own.
    let ours = match ctx.state.opened_project.lock().as_mut() {
        Some(reader) => {
            reader.end_agent_run();
            Some(reader.document().preset.clone())
        }
        None => None,
    };
    match args.name.filter(|n| !n.trim().is_empty()) {
        Some(name) => match args.part.as_deref().map(str::trim) {
            None | Some("") => text(compose(&name, args.auto, false)?),
            Some("export") => {
                find(&name)?;
                text(EXPORT.trim().replace("\r\n", "\n"))
            }
            Some(other) => Err(format!("No part {other}: `part` is `export`")),
        },
        None => {
            let mut lines = vec!["Presets (`preset` with a name gives one):".to_string()];
            let mut all = preset::store::list();
            if let Some(ours) = &ours {
                all.sort_by_key(|p| !p.name.eq_ignore_ascii_case(ours));
            }
            lines.extend(all.iter().map(|p| {
                format!(
                    "{}: {}{}",
                    prompt_name(&p.name),
                    p.description,
                    if ours
                        .as_deref()
                        .is_some_and(|o| o.eq_ignore_ascii_case(&p.name))
                    {
                        " (the open project's)"
                    } else {
                        ""
                    }
                )
            }));
            text(lines.join("\n"))
        }
    }
}

// ---------------------------------------------------------------------------------------
// MCP prompts: a preset the user picks in the client, the whole guide in one message.

pub fn prompt_list() -> Vec<rmcp::model::Prompt> {
    use rmcp::model::{Prompt, PromptArgument};
    preset::store::list()
        .iter()
        .map(|p| {
            Prompt::new(
                prompt_name(&p.name),
                Some(p.description.clone()),
                Some(vec![
                    PromptArgument::new("recording")
                        .with_description(
                            "The recording to edit (a .aero folder). Empty: the open project.",
                        )
                        .with_required(false),
                    PromptArgument::new("name")
                        .with_description("A name for the new project.")
                        .with_required(false),
                ]),
            )
            .with_title(p.name.clone())
        })
        .collect()
}

pub fn prompt_get(
    name: &str,
    arguments: Option<&serde_json::Map<String, Value>>,
) -> Result<rmcp::model::GetPromptResult, String> {
    use rmcp::model::{GetPromptResult, PromptMessage, Role};
    let preset = find(name)?;
    let argument = |key: &str| {
        arguments
            .and_then(|a| a.get(key))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let what = match (argument("recording"), argument("name")) {
        (Some(recording), Some(project)) => {
            format!(
                "Edit the recording {recording} in AeroEdits, as a new project named \"{project}\""
            )
        }
        (Some(recording), None) => {
            format!("Edit the recording {recording} in AeroEdits (ask me what to name the project)")
        }
        (None, _) => "Edit the project open in AeroEdits".to_string(),
    };
    let message = format!(
        "{what}, following the {} preset. All of it is below, so there is no need to call \
         `preset`.\n\n{}",
        preset.name,
        guide(&preset.name)?
    );
    Ok(
        GetPromptResult::new(vec![PromptMessage::new_text(Role::User, message)])
            .with_description(preset.name),
    )
}

// ---------------------------------------------------------------------------------------
// The skill folder, for agents that read skills from disk.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkillHome {
    /// `~/.claude/skills` (Claude Code, and the Claude app's Code tab).
    ClaudeCode,
    /// `~/.codex/skills` (the Codex app and CLI).
    Codex,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkillState {
    pub home: SkillHome,
    /// The agent's settings folder exists.
    pub available: bool,
    pub installed: bool,
    /// The installed files are this app's.
    pub current: bool,
    pub path: String,
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// The agent's settings folder.
fn home_dir(home: SkillHome) -> Option<PathBuf> {
    match home {
        SkillHome::ClaudeCode => {
            env_dir("CLAUDE_CONFIG_DIR").or_else(|| Some(dirs::home_dir()?.join(".claude")))
        }
        SkillHome::Codex => {
            env_dir("CODEX_HOME").or_else(|| Some(dirs::home_dir()?.join(".codex")))
        }
    }
}

fn skill_dir(base: &Path) -> PathBuf {
    base.join("skills").join("aeroedits")
}

/// The skill folder's files, by their path inside it: only the pointer to `template`, so an
/// agent reads the rules once, from the app.
fn files() -> Vec<(PathBuf, &'static str)> {
    vec![(PathBuf::from("SKILL.md"), SKILL)]
}

/// Takes out what older versions installed beside SKILL.md, and the folder they leave empty.
fn remove_legacy(dir: &Path) {
    for path in LEGACY_FILES {
        let _ = fs::remove_file(dir.join(path));
    }
    let _ = fs::remove_dir(dir.join("templates"));
}

fn state_at(home: SkillHome, base: &Path) -> SkillState {
    let dir = skill_dir(base);
    let installed = dir.join("SKILL.md").is_file();
    let current = installed
        && files()
            .iter()
            .all(|(path, text)| fs::read_to_string(dir.join(path)).is_ok_and(|t| t == *text))
        && !LEGACY_FILES.iter().any(|path| dir.join(path).exists());
    SkillState {
        home,
        available: base.is_dir(),
        installed,
        current,
        path: dir.to_string_lossy().into_owned(),
    }
}

pub fn states() -> Vec<SkillState> {
    [SkillHome::ClaudeCode, SkillHome::Codex]
        .into_iter()
        .filter_map(|home| Some(state_at(home, &home_dir(home)?)))
        .collect()
}

fn install_at(base: &Path) -> Result<(), String> {
    let dir = skill_dir(base);
    for (path, text) in files() {
        let path = dir.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Could not make {}: {e}", parent.display()))?;
        }
        fs::write(&path, text).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    }
    remove_legacy(&dir);
    Ok(())
}

/// Takes out the files this app wrote, and the folders they leave empty; anything the user
/// added stays.
fn uninstall_at(base: &Path) -> Result<(), String> {
    let dir = skill_dir(base);
    let skill = fs::read_to_string(dir.join("SKILL.md")).unwrap_or_default();
    if !skill.is_empty() && !skill.contains("name: aeroedits") {
        return Err(format!(
            "{} holds another skill; left it alone",
            dir.display()
        ));
    }
    for (path, _) in files() {
        let _ = fs::remove_file(dir.join(path));
    }
    remove_legacy(&dir);
    let _ = fs::remove_dir(&dir);
    Ok(())
}

pub fn set_installed(home: SkillHome, installed: bool) -> Result<SkillState, String> {
    let base = home_dir(home).ok_or("Could not find the home folder")?;
    if !base.is_dir() {
        return Err(match home {
            SkillHome::ClaudeCode => "Claude Code does not seem to be installed".into(),
            SkillHome::Codex => "Codex does not seem to be installed".into(),
        });
    }
    if installed {
        install_at(&base)?;
    } else {
        uninstall_at(&base)?;
    }
    Ok(state_at(home, &base))
}

/// Brings installed skills up to this app's text, at launch, unless the user took the marker out
/// to keep changes of their own.
pub fn refresh_installed() {
    for home in [SkillHome::ClaudeCode, SkillHome::Codex] {
        let Some(base) = home_dir(home) else { continue };
        refresh_at(&base);
    }
}

fn refresh_at(base: &Path) {
    let dir = skill_dir(base);
    let ours = fs::read_to_string(dir.join("SKILL.md")).is_ok_and(|text| text.contains(MARKER));
    if ours && !state_at(SkillHome::ClaudeCode, base).current {
        if let Err(error) = install_at(base) {
            eprintln!("[mcp] {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guide_has_the_rules_the_method_the_export_and_the_choices() {
        let guide = guide("tutorial").unwrap();
        assert!(
            !guide.starts_with("---") && !guide.contains(MARKER),
            "{guide}"
        );
        assert!(guide.starts_with("# Editing in AeroEdits"), "{guide}");
        let (rules, method, export, choices) = (
            guide
                .find("## Choices")
                .unwrap_or(0)
                .min(guide.find("# Editing").unwrap()),
            guide.find("\n# Preset: Tutorial").unwrap(),
            guide.find("\n# Export").unwrap(),
            guide.find("## Choices (Tutorial)").unwrap(),
        );
        assert!(
            rules < method && method < export && export < choices,
            "{guide}"
        );
        assert!(guide.contains("\"review\": \"fast\""), "{guide}");
        assert!(
            guide::is_err(super::guide("vlog")),
            "a preset that isn't there"
        );
    }

    mod guide {
        pub fn is_err(result: Result<String, String>) -> bool {
            result.is_err_and(|e| e.contains("There is no preset"))
        }
    }

    #[test]
    fn the_preset_tool_lists_and_gives_a_method() {
        let host = crate::mcp::tests_support::BareHost::default();
        let ctx = Ctx::new(&host, "agent", false);
        let call = |name: &str, args: Value| {
            (crate::mcp::tools::find(name).unwrap().run)(&ctx, args).map(|out| match &out[0] {
                Output::Text(text) => text.clone(),
                Output::Png(_) => String::new(),
            })
        };
        let list = call("preset", serde_json::json!({})).unwrap();
        assert!(
            list.contains("\ntutorial: A screen-recorded tutorial"),
            "{list}"
        );
        let guide = call("preset", serde_json::json!({ "name": "tutorial" })).unwrap();
        // Step by step: the questions and the thorough review, no export steps yet.
        assert!(
            guide.contains("## Asking") && guide.contains("### 3b."),
            "{guide}"
        );
        assert!(
            !guide.contains("# Export") && !guide.contains("<!--"),
            "{guide}"
        );
        assert!(
            !guide.contains("You are running it automatically"),
            "{guide}"
        );
        // Automatic: none of the asking, nor the thorough review it didn't choose.
        let auto = call(
            "preset",
            serde_json::json!({ "name": "Tutorial", "auto": true }),
        )
        .unwrap();
        assert!(auto.contains("You are running it automatically"), "{auto}");
        assert!(
            !auto.contains("## Asking") && !auto.contains("### 3b."),
            "{auto}"
        );
        assert!(
            !auto.contains("1. Dry run:") && !auto.contains("<!--"),
            "{auto}"
        );
        // Under 80 % before AI-1's trims (24 Oct), which took most from the step-by-step parts.
        assert!(
            auto.len() * 100 < guide.len() * 85,
            "{} vs {}",
            auto.len(),
            guide.len()
        );
        let export = call(
            "preset",
            serde_json::json!({ "name": "tutorial", "part": "export" }),
        )
        .unwrap();
        assert!(export.starts_with("# Export"), "{export}");
        // Who reviews fillers is never kept in the choices.
        let mut choices = serde_json::json!({ "fillers": { "apply": true, "review": "in-house" },
            "transcriber": "parakeet" })
        .as_object()
        .cloned()
        .unwrap();
        forget_review(&mut choices);
        assert_eq!(
            serde_json::Value::Object(choices),
            serde_json::json!({ "fillers": { "apply": true } })
        );
    }

    #[test]
    fn a_prompt_carries_the_recording_and_the_whole_guide() {
        assert_eq!(prompt_list()[0].name, "tutorial");
        let mut arguments = serde_json::Map::new();
        arguments.insert("recording".into(), r"C:\Rec\Lesson.aero".into());
        arguments.insert("name".into(), "Lesson 1".into());
        let prompt = prompt_get("tutorial", Some(&arguments)).unwrap();
        let message = serde_json::to_value(&prompt.messages[0]).unwrap();
        let text = message["content"]["text"].as_str().unwrap();
        assert!(
            text.starts_with(r#"Edit the recording C:\Rec\Lesson.aero in AeroEdits, as a new project named "Lesson 1""#),
            "{text}"
        );
        assert!(
            text.contains("# Preset: Tutorial") && text.contains("# Export"),
            "{text}"
        );
        assert_eq!(message["role"], "user");
        let open = prompt_get("tutorial", None).unwrap();
        let text = serde_json::to_value(&open.messages[0]).unwrap()["content"]["text"].clone();
        assert!(text
            .as_str()
            .unwrap()
            .starts_with("Edit the project open in AeroEdits"));
    }

    #[test]
    fn the_skill_folder_installs_refreshes_and_leaves_the_users_files() {
        let base = tempfile::tempdir().unwrap();
        let dir = skill_dir(base.path());
        assert!(!state_at(SkillHome::Codex, base.path()).installed);
        install_at(base.path()).unwrap();
        let state = state_at(SkillHome::Codex, base.path());
        assert!(
            state.installed && state.current && state.available,
            "{state:?}"
        );
        // Only the pointer to `template`: the rules come from the app, once.
        assert!(!dir.join("templates").exists() && !dir.join("export.md").exists());

        // An older copy (with the files older versions wrote) is brought up to date at launch...
        fs::write(dir.join("export.md"), "old").unwrap();
        assert!(!state_at(SkillHome::Codex, base.path()).current);
        refresh_at(base.path());
        assert!(state_at(SkillHome::Codex, base.path()).current);
        assert!(!dir.join("export.md").exists());
        // ...unless the user took the marker out to keep their changes.
        let mine = SKILL.replace(MARKER, "<!-- mine");
        fs::write(dir.join("SKILL.md"), &mine).unwrap();
        refresh_at(base.path());
        assert_eq!(fs::read_to_string(dir.join("SKILL.md")).unwrap(), mine);

        fs::write(dir.join("notes.md"), "keep me").unwrap();
        uninstall_at(base.path()).unwrap();
        assert!(!dir.join("SKILL.md").exists() && !dir.join("templates").exists());
        assert!(dir.join("notes.md").is_file(), "the user's own file stays");

        fs::write(dir.join("SKILL.md"), "---\nname: other\n---\n").unwrap();
        assert!(uninstall_at(base.path()).is_err());
    }
}
