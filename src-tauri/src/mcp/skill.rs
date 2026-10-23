//! The `aeroedits` skill: how an agent runs a whole edit (ask, apply, report, export), and a
//! template for each kind of video (its steps, what to ask the user, the defaults). The text
//! ships inside the app, so it always matches the actions, and reaches agents three ways: the
//! `template` tool (any client; the server's instructions point to it), MCP prompts (the Claude
//! app's + menu, slash commands in Claude Code), and a skill folder installed for Claude Code and
//! Codex from the AI agents dialog. The user's answers to a template's questions are kept, so
//! the next edit asks one question.
use super::tools::{params, schema, text, Ctx, Output, ToolDef};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// The installed skill: what it is for, and a pointer to `template`, which has the rest.
const SKILL: &str = include_str!("skill/SKILL.md");
/// The rules every edit follows, given once with the template.
const RULES: &str = include_str!("skill/rules.md");
const EXPORT: &str = include_str!("skill/export.md");
/// How to work with graphics, given with the `graphics` list (only edits with graphics need it).
pub const GRAPHICS: &str = include_str!("skill/graphics.md");
/// Files older versions installed beside SKILL.md; they are taken out so no agent reads them.
const LEGACY_FILES: [&str; 4] = [
    "export.md",
    "templates/tutorial.md",
    "templates/shorts.md",
    "templates/take-shorts.md",
];
/// The line that makes an installed skill folder AeroEdits' to update.
const MARKER: &str = "<!-- Installed by AeroEdits";

pub struct Template {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    text: &'static str,
}

pub static TEMPLATES: [Template; 3] = [
    Template {
        name: "tutorial",
        title: "Tutorial edit",
        description: "A screen-recorded tutorial, long form: clean speech, steady pacing, \
            webcam and zooms, no captions, chapters, then export when the user says.",
        text: include_str!("skill/templates/tutorial.md"),
    },
    Template {
        name: "shorts",
        title: "Shorts from a take",
        description: "A vertical short the user recorded in a new take (camera and microphone, \
            made outside the app): the take talks, you put under it the long video's footage and \
            graphics that show each sentence, then new graphics, then export when the user says. \
            The app's Shorts tab.",
        text: include_str!("skill/templates/shorts.md"),
    },
    Template {
        name: "clips",
        title: "Clips from the video",
        description: "Vertical clips cut from the edited video: you pick the best moments from \
            its transcript, the app gives them the split-screen look and layout sections, then \
            export when the user says. The app's Clips tab.",
        text: include_str!("skill/templates/clips.md"),
    },
];

fn find(name: &str) -> Result<&'static Template, String> {
    TEMPLATES
        .iter()
        .find(|t| t.name == name.trim())
        .ok_or_else(|| format!("No template {name}. Templates: {}", names()))
}

fn names() -> String {
    TEMPLATES
        .iter()
        .map(|t| t.name)
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------------------------------
// Saved answers.

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Saved {
    saved_at: String,
    answers: Value,
}

fn answers_path(config: &Path, name: &str) -> PathBuf {
    config
        .join("agents")
        .join("answers")
        .join(format!("{name}.json"))
}

fn load_answers(config: &Path, name: &str) -> Option<Saved> {
    let mut saved: Saved =
        serde_json::from_slice(&fs::read(answers_path(config, name)).ok()?).ok()?;
    forget_review(&mut saved.answers);
    Some(saved)
}

/// Who reviews fillers and who transcribes are never saved answers: `review: auto` (the app's
/// OpenRouter review when a key is set) and `provider: auto` (online, the language detected)
/// unless the user asks otherwise in the conversation. Answers saved while the in-house review
/// or Parakeet was the default would otherwise keep them forever.
fn forget_review(answers: &mut Value) {
    if let Some(fillers) = answers.get_mut("fillers").and_then(Value::as_object_mut) {
        fillers.remove("review");
    }
    if let Some(answers) = answers.as_object_mut() {
        answers.remove("transcriber");
    }
}

fn save_answers(config: &Path, name: &str, answers: Value) -> Result<(), String> {
    let path = answers_path(config, name);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("Could not save the answers: {e}"))?;
    }
    let saved = Saved {
        saved_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        answers,
    };
    let json = serde_json::to_vec_pretty(&saved).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("Could not save the answers: {e}"))
}

/// Everything an agent needs for an edit of this kind: the rules, the template, the export
/// steps and the user's saved answers (a prompt's whole guide, for a run step by step).
pub fn guide(config: &Path, name: &str) -> Result<String, String> {
    compose(config, name, false, true)
}

/// The guide `template` gives: for an automatic run (`auto`) without what only a run step by
/// step needs (dry runs to ask about, the questions, saving answers) nor the thorough review
/// unless the saved answers chose it; the export steps only with `export` (asked for at the
/// end). Agents pay for every word of it, on every edit.
fn compose(config: &Path, name: &str, auto: bool, export: bool) -> Result<String, String> {
    let template = find(name)?;
    let loaded = load_answers(config, template.name);
    let thorough = loaded
        .as_ref()
        .and_then(|s| s.answers.get("review"))
        .and_then(Value::as_str)
        == Some("thorough");
    let saved = match loaded {
        Some(saved) => format!(
            "## Saved answers ({}, saved {})\n\nAsk once whether to use them again.\n\n```json\n{}\n```",
            template.name,
            saved.saved_at,
            serde_json::to_string_pretty(&saved.answers).unwrap_or_default()
        ),
        None => format!(
            "## Saved answers\n\nNone for {} yet: ask the template's questions, then save the \
             answers with `template_save`.",
            template.name
        ),
    };
    let export = if export {
        format!("{}\n\n---\n\n", EXPORT.trim())
    } else {
        String::new()
    };
    // A checkout with CRLF line ends embeds them.
    let whole = format!(
        "{}\n\n---\n\n{}\n\n---\n\n{export}{saved}",
        RULES.trim(),
        template.text.trim(),
    )
    .replace("\r\n", "\n");
    Ok(for_mode(&whole, auto, thorough))
}

/// `text` without the blocks this run doesn't need: `<!-- steps -->` (a run step by step only),
/// `<!-- auto -->` (an automatic run only), `<!-- thorough -->` (a thorough review only, kept
/// step by step since the user is asked). The marker lines themselves never reach agents.
fn for_mode(text: &str, auto: bool, thorough: bool) -> String {
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
            .filter(|k| matches!(*k, "steps" | "auto" | "thorough"))
        {
            let drop = match kind {
                "steps" => auto,
                "auto" => !auto,
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

pub static TOOLS: [ToolDef; 2] = [
    ToolDef {
        name: "template",
        description:
            "How to run a whole edit: the plan for a kind of video (the steps, what to ask \
            the user, the defaults), the general rules and the user's saved answers. Call it \
            first when the user asks for an edit, with `auto: true` when they want it done \
            automatically; `part: export` gives the export steps. Without `name`, the list.",
        schema: schema::<TemplateArgs>,
        read_only: true,
        run: template,
    },
    ToolDef {
        name: "template_save",
        description: "Saves the user's answers to a template's questions, in the shape the \
            template gives, so the next edit can reuse them (`template` shows them).",
        schema: schema::<SaveArgs>,
        read_only: false,
        run: template_save,
    },
];

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateArgs {
    /// The template, such as `tutorial`. Without it, the list of templates.
    #[serde(default)]
    pub name: Option<String>,
    /// The user wants it done automatically: the guide without the questions.
    #[serde(default)]
    pub auto: bool,
    /// `export`: only the export steps.
    #[serde(default)]
    pub part: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveArgs {
    pub name: String,
    /// The user's answers, in the shape the template gives.
    pub answers: Value,
}

fn template(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: TemplateArgs = params(args)?;
    // A workflow begins: what the agent changes from here is a new run, undone on its own.
    if let Some(reader) = ctx.state.opened_project.lock().as_mut() {
        reader.end_agent_run();
    }
    match args.name.filter(|n| !n.trim().is_empty()) {
        Some(name) => match args.part.as_deref().map(str::trim) {
            None | Some("") => text(compose(&ctx.host.config_dir(), &name, args.auto, false)?),
            Some("export") => {
                find(&name)?;
                text(EXPORT.trim().replace("\r\n", "\n"))
            }
            Some(other) => Err(format!("No part {other}: `part` is `export`")),
        },
        None => {
            let mut lines = vec!["Templates (`template` with a name gives one):".to_string()];
            lines.extend(
                TEMPLATES
                    .iter()
                    .map(|t| format!("{}: {}", t.name, t.description)),
            );
            text(lines.join("\n"))
        }
    }
}

fn template_save(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: SaveArgs = params(args)?;
    let template = find(&args.name)?;
    if !args.answers.is_object() {
        return Err("`answers` must be a JSON object, in the shape the template gives".into());
    }
    let mut answers = args.answers;
    forget_review(&mut answers);
    save_answers(&ctx.host.config_dir(), template.name, answers)?;
    text(format!(
        "Saved the answers for {}; the next edit offers them again.",
        template.name
    ))
}

// ---------------------------------------------------------------------------------------
// MCP prompts: a template the user picks in the client, the whole guide in one message.

pub fn prompt_list() -> Vec<rmcp::model::Prompt> {
    use rmcp::model::{Prompt, PromptArgument};
    TEMPLATES
        .iter()
        .map(|t| {
            Prompt::new(
                t.name,
                Some(t.description),
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
            .with_title(t.title)
        })
        .collect()
}

pub fn prompt_get(
    config: &Path,
    name: &str,
    arguments: Option<&serde_json::Map<String, Value>>,
) -> Result<rmcp::model::GetPromptResult, String> {
    use rmcp::model::{GetPromptResult, PromptMessage, Role};
    let template = find(name)?;
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
        "{what}, following the {} template. All of it is below, so there is no need to call \
         `template`.\n\n{}",
        template.name,
        guide(config, template.name)?
    );
    Ok(
        GetPromptResult::new(vec![PromptMessage::new_text(Role::User, message)])
            .with_description(template.title),
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
    fn the_guide_has_the_rules_the_template_the_export_and_the_saved_answers() {
        let config = tempfile::tempdir().unwrap();
        let guide = guide(config.path(), "tutorial").unwrap();
        assert!(
            !guide.starts_with("---") && !guide.contains(MARKER),
            "{guide}"
        );
        assert!(guide.starts_with("# Editing in AeroEdits"), "{guide}");
        let (rules, template, export) = (
            guide.find("## Saved answers\n\nWhen").unwrap(),
            guide.find("\n# Template: tutorial").unwrap(),
            guide.find("\n# Export").unwrap(),
        );
        assert!(rules < template && template < export, "{guide}");
        assert!(
            guide.ends_with("save the answers with `template_save`."),
            "{guide}"
        );

        save_answers(
            config.path(),
            "tutorial",
            serde_json::json!({ "silence": "tight" }),
        )
        .unwrap();
        let guide = super::guide(config.path(), "tutorial").unwrap();
        assert!(
            guide.contains("\"silence\": \"tight\"") && guide.contains("Ask once"),
            "{guide}"
        );
        assert!(super::guide(config.path(), "vlog")
            .unwrap_err()
            .contains("Templates: tutorial"));
    }

    #[test]
    fn the_template_tools_list_give_and_remember() {
        let host = crate::mcp::tests_support::BareHost::default();
        let ctx = Ctx::new(&host, "agent", false);
        let call = |name: &str, args: Value| {
            (crate::mcp::tools::find(name).unwrap().run)(&ctx, args).map(|out| match &out[0] {
                Output::Text(text) => text.clone(),
                Output::Png(_) => String::new(),
            })
        };
        let list = call("template", serde_json::json!({})).unwrap();
        assert!(
            list.contains("\ntutorial: A screen-recorded tutorial"),
            "{list}"
        );
        let guide = call("template", serde_json::json!({ "name": "tutorial" })).unwrap();
        assert!(guide.contains("None for tutorial yet"), "{guide}");
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
            "template",
            serde_json::json!({ "name": "tutorial", "auto": true }),
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
        assert!(
            auto.len() * 5 < guide.len() * 4,
            "{} vs {}",
            auto.len(),
            guide.len()
        );
        let export = call(
            "template",
            serde_json::json!({ "name": "tutorial", "part": "export" }),
        )
        .unwrap();
        assert!(export.starts_with("# Export"), "{export}");
        let saved = call(
            "template_save",
            serde_json::json!({ "name": "tutorial", "answers": { "silence": "gentle", "transcriber": "parakeet" } }),
        )
        .unwrap();
        assert!(
            saved.starts_with("Saved the answers for tutorial"),
            "{saved}"
        );
        let guide = call("template", serde_json::json!({ "name": "tutorial" })).unwrap();
        assert!(guide.contains("\"silence\": \"gentle\""), "{guide}");
        // Nor who transcribes: agents transcribe online, the language detected.
        assert!(!guide.contains("\"transcriber\""), "{guide}");
        // Who reviews fillers is never kept: the app's review runs unless the user says.
        call(
            "template_save",
            serde_json::json!({ "name": "tutorial",
                "answers": { "fillers": { "apply": true, "review": "in-house" } } }),
        )
        .unwrap();
        let guide = call("template", serde_json::json!({ "name": "tutorial" })).unwrap();
        assert!(
            guide.contains("\"apply\": true") && !guide.contains("\"review\": \"in-house\""),
            "{guide}"
        );
        assert!(answers_path(host.config.path(), "tutorial").is_file());
        assert!(call(
            "template_save",
            serde_json::json!({ "name": "tutorial", "answers": [1] })
        )
        .is_err());
        assert!(call(
            "template_save",
            serde_json::json!({ "name": "vlog", "answers": {} })
        )
        .is_err());
    }

    #[test]
    fn a_prompt_carries_the_recording_and_the_whole_guide() {
        let config = tempfile::tempdir().unwrap();
        assert_eq!(prompt_list()[0].name, "tutorial");
        let mut arguments = serde_json::Map::new();
        arguments.insert("recording".into(), r"C:\Rec\Lesson.aero".into());
        arguments.insert("name".into(), "Lesson 1".into());
        let prompt = prompt_get(config.path(), "tutorial", Some(&arguments)).unwrap();
        let message = serde_json::to_value(&prompt.messages[0]).unwrap();
        let text = message["content"]["text"].as_str().unwrap();
        assert!(
            text.starts_with(r#"Edit the recording C:\Rec\Lesson.aero in AeroEdits, as a new project named "Lesson 1""#),
            "{text}"
        );
        assert!(
            text.contains("# Template: tutorial") && text.contains("# Export"),
            "{text}"
        );
        assert_eq!(message["role"], "user");
        let open = prompt_get(config.path(), "tutorial", None).unwrap();
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
