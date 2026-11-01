//! `plan`: the agent's plan for the edit, kept as `plan.md` in the project folder, so what was
//! planned and what the user locked survives between turns and chats (PLAN-1). Rows name
//! moments by word ids; reading the plan says where each of those words plays now, so a later
//! cut moves the row without the file changing.
use super::actions::{pick_sound, Want};
use super::tools::{clock, params, schema, text, with_reader, Ctx, Output, ToolDef};
use crate::transcript::store;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

pub const PLAN_FILE: &str = "plan.md";
const MAX_CHARS: usize = 24_000;

pub static TOOLS: [ToolDef; 1] = [ToolDef {
    name: "plan",
    description: "The plan for this edit, a Markdown file kept in the project (`plan.md`): \
        per moment the words (ids from `transcript_get`), purpose, what the viewer sees, \
        treatment and reason, then what the user locked. `op`: `read` (the default) gives it \
        and where each word id in it plays now (or that it was cut); `write` with `text` \
        replaces it; `edit` with `edits` (`[{oldText, newText}]`, each old text found once) \
        changes parts. Read it at the start of a turn when `status` names one, keep it \
        current as the user decides, and follow what it locks.",
    schema: schema::<PlanArgs>,
    read_only: false,
    run: plan,
}];

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PlanOp {
    #[default]
    Read,
    Write,
    Edit,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlanArgs {
    /// read (default), write or edit.
    #[serde(default)]
    pub op: PlanOp,
    /// write: the whole plan, Markdown.
    #[serde(default)]
    pub text: Option<String>,
    /// edit: each `oldText` (found once in the plan) becomes its `newText`, in order.
    #[serde(default)]
    pub edits: Vec<PlanEdit>,
}

#[derive(Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlanEdit {
    pub old_text: String,
    pub new_text: String,
}

/// The plan in project folder `root`, if one was written.
pub fn load(root: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(root.join(PLAN_FILE)) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Could not read the plan: {e}")),
    }
}

fn save(root: &Path, plan: &str) -> Result<(), String> {
    let mut temp = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, plan.as_bytes()).map_err(|e| e.to_string())?;
    temp.persist(root.join(PLAN_FILE))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// `plan` with each `old_text` replaced once by its `new_text`, in order.
pub fn apply_edits(plan: &str, edits: &[PlanEdit]) -> Result<String, String> {
    if edits.is_empty() {
        return Err("Give `edits`: `[{oldText, newText}]`".into());
    }
    let mut out = plan.to_string();
    for (n, edit) in edits.iter().enumerate() {
        let found = if edit.old_text.is_empty() {
            0
        } else {
            out.matches(edit.old_text.as_str()).count()
        };
        if found != 1 {
            return Err(format!(
                "Edit {}: its `oldText` is in the plan {found} times; it must be there once \
                 (nothing was changed)",
                n + 1
            ));
        }
        out = out.replacen(edit.old_text.as_str(), &edit.new_text, 1);
    }
    Ok(out)
}

/// The word ids (`w-12`, `w-c3`) a plan names, each once, in the order they first appear.
pub fn word_ids(plan: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    let bytes = plan.as_bytes();
    let mut i = 0;
    while let Some(found) = plan[i..].find("w-") {
        let at = i + found;
        i = at + 2;
        let mut end = at + 2;
        if bytes.get(end) == Some(&b'c') {
            end += 1;
        }
        let digits = bytes[end..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        let starts_word = at == 0 || !bytes[at - 1].is_ascii_alphanumeric();
        let ends_word = bytes
            .get(end + digits)
            .is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'-');
        if digits > 0 && starts_word && ends_word {
            let id = &plan[at..end + digits];
            if !out.contains(&id) {
                out.push(id);
            }
        }
    }
    out
}

fn plan(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: PlanArgs = params(args)?;
    let root = with_reader(ctx.state, |reader| Ok(reader.root().to_path_buf()))?;
    let had = load(&root)?;
    let next = match args.op {
        PlanOp::Read => return read(ctx, had),
        PlanOp::Write => args.text.ok_or("Give `text`: the whole plan, Markdown")?,
        PlanOp::Edit => apply_edits(
            had.as_deref()
                .ok_or("There is no plan yet: `op: write` with `text` makes one")?,
            &args.edits,
        )?,
    };
    if next.trim().is_empty() {
        return Err("A plan needs some text".into());
    }
    let chars = next.chars().count();
    if chars > MAX_CHARS {
        return Err(format!(
            "That plan is {chars} characters; keep it under {MAX_CHARS} (one row per moment)"
        ));
    }
    save(&root, &next)?;
    text(format!(
        "{} the plan ({} lines, {chars} characters), {PLAN_FILE} in the project.",
        if had.is_some() { "Saved" } else { "Wrote" },
        next.lines().count()
    ))
}

fn read(ctx: &Ctx, plan: Option<String>) -> Result<Vec<Output>, String> {
    let Some(plan) = plan else {
        return text(
            "No plan yet. Once the user agrees what to do, `plan` with `op: write` and `text` \
             keeps it in the project: a table with a row per moment (words, purpose, what the \
             viewer sees, treatment, reason), then what the user locked."
                .to_string(),
        );
    };
    let ids = word_ids(&plan);
    let mut out = plan.clone();
    if ids.is_empty() {
        return text(out);
    }
    // Where its words play now, from the transcribed speech.
    let places = pick_sound(ctx, None, Want::Transcribed).and_then(|key| {
        with_reader(ctx.state, |reader| {
            let document = reader.document();
            let transcript =
                store::load_transcript(reader.root(), &key)?.ok_or("Not transcribed")?;
            let clock_of = document.transcript_clock(&key)?;
            Ok(ids
                .iter()
                .map(|id| {
                    let place = match transcript.words.iter().find(|w| w.id == *id) {
                        None => "not in the transcript".to_string(),
                        Some(word) if crate::transcript::edit::kept(word, &clock_of) => clock_of
                            .source_to_edited_us(word.source_start_us)
                            .map_or_else(|| "cut".into(), clock),
                        Some(_) => "cut".to_string(),
                    };
                    format!("{id} {place}")
                })
                .collect::<Vec<_>>())
        })
    });
    if let Ok(places) = places {
        out.push_str(&format!(
            "\n\n---\nWhere its words play now: {}.",
            places.join(", ")
        ));
    }
    text(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(old: &str, new: &str) -> PlanEdit {
        PlanEdit {
            old_text: old.into(),
            new_text: new.into(),
        }
    }

    #[test]
    fn edits_change_text_found_once_and_nothing_else() {
        let plan = "| w-1..w-9 | title card | locked |\n| w-20..w-31 | steps |\n";
        assert_eq!(
            apply_edits(plan, &[edit("steps", "steps-link"), edit("locked", "")]).unwrap(),
            "| w-1..w-9 | title card |  |\n| w-20..w-31 | steps-link |\n"
        );
        // Twice, or not at all: refused whole.
        assert!(apply_edits(plan, &[edit("| w-", "x")]).is_err());
        assert!(apply_edits(plan, &[edit("steps", "a"), edit("nowhere", "b")]).is_err());
        assert!(apply_edits(plan, &[edit("", "b")]).is_err());
        assert!(apply_edits(plan, &[]).is_err());
    }

    #[test]
    fn word_ids_are_found_once_each_and_only_whole() {
        let plan = "w-1..w-12 hook; w-12 again; w-c3 fixed; aw-5 no; w-7x no; w- no; (w-40)";
        assert_eq!(word_ids(plan), vec!["w-1", "w-12", "w-c3", "w-40"]);
        assert!(word_ids("").is_empty());
        assert_eq!(word_ids("w-9"), vec!["w-9"]);
    }

    #[test]
    fn the_plan_is_kept_in_the_project_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), None);
        save(dir.path(), "# Plan\n").unwrap();
        assert_eq!(load(dir.path()).unwrap().as_deref(), Some("# Plan\n"));
    }
}
