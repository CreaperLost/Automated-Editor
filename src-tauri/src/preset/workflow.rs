//! A saved workflow: a preset the user approved on a real edit, so the next video of that kind
//! runs the same way. Nothing here learns by itself: exceptions and checks are the user's words,
//! saved at their yes, and an agent reads them with the method (`for_the_guide`).
use super::Preset;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Exceptions or checks one preset holds, and how long one is: an agent reads them all, each edit.
const MAX_LINES: usize = 30;
const MAX_LINE_CHARS: usize = 300;

/// The edit a workflow was approved on.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Approved {
    /// The project's name.
    pub from: String,
    /// The day, as `YYYY-MM-DD`.
    pub on: String,
    /// What the user approved, in a line.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// What the user adds when saving an edit as a workflow; what is left out stays as it was.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Workflow {
    pub exceptions: Option<Vec<String>>,
    pub checks: Option<Vec<String>>,
    pub note: Option<String>,
}

fn cleaned(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

impl Workflow {
    /// `preset` as approved today on the project called `from`.
    pub fn apply_to(self, preset: &mut Preset, from: &str) {
        if let Some(exceptions) = self.exceptions {
            preset.exceptions = cleaned(exceptions);
        }
        if let Some(checks) = self.checks {
            preset.checks = cleaned(checks);
        }
        preset.approved = Some(Approved {
            from: from.to_string(),
            on: chrono::Local::now().format("%Y-%m-%d").to_string(),
            note: self.note.unwrap_or_default().trim().to_string(),
        });
    }
}

pub fn validate(preset: &Preset) -> Result<(), String> {
    for (what, lines) in [
        ("exceptions", &preset.exceptions),
        ("checks", &preset.checks),
    ] {
        if lines.len() > MAX_LINES {
            return Err(format!("Keep at most {MAX_LINES} {what}"));
        }
        if lines.iter().any(|l| l.chars().count() > MAX_LINE_CHARS) {
            return Err(format!(
                "Keep each of the {what} to {MAX_LINE_CHARS} characters"
            ));
        }
    }
    Ok(())
}

/// `minHoldMs` as words: `min hold ms`.
fn words(key: &str) -> String {
    let mut out = String::new();
    for c in key.chars() {
        if c.is_uppercase() {
            out.push(' ');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn said(value: &Value) -> String {
    match value {
        Value::Bool(true) => "on".into(),
        Value::Bool(false) => "off".into(),
        Value::String(text) => text.clone(),
        Value::Null => "default".into(),
        Value::Array(items) => items.iter().map(said).collect::<Vec<_>>().join(", "),
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| match value {
                Value::Object(_) => said(value),
                _ => format!("{} {}", words(key), said(value)),
            })
            .collect::<Vec<_>>()
            .join(", "),
        Value::Number(n) => n.to_string(),
    }
}

/// `{"zooms": {"apply": true, "settings": {...}}}` as `zooms on (...)`: a choice's `apply` says
/// whether it runs at all, so it comes first and an off one says nothing more.
fn choice(key: &str, value: &Value) -> String {
    let name = words(key);
    let Value::Object(map) = value else {
        return format!("{name} {}", said(value));
    };
    let mut rest = map.clone();
    match rest.remove("apply") {
        Some(Value::Bool(false)) => format!("{name} off"),
        Some(Value::Bool(true)) if rest.is_empty() => format!("{name} on"),
        Some(Value::Bool(true)) => format!("{name} on ({})", said(&Value::Object(rest))),
        _ => format!("{name} {}", said(value)),
    }
}

/// All of a preset in a few readable lines: what someone checks before trusting it with the next
/// video. The method and rules are named only by size, as they have their own place to read.
pub fn summary(preset: &Preset) -> String {
    let mut lines = vec![if preset.version > 0 {
        format!("{}, version {}", preset.name, preset.version)
    } else {
        format!("{} (built in)", preset.name)
    }];
    if !preset.description.trim().is_empty() {
        lines.push(format!("For: {}", preset.description.trim()));
    }
    if let Some(approved) = &preset.approved {
        let note = if approved.note.is_empty() {
            String::new()
        } else {
            format!(": {}", approved.note)
        };
        lines.push(format!(
            "Approved on {} on \"{}\"{note}",
            approved.on, approved.from
        ));
    }
    if !preset.style.is_empty() {
        lines.push(format!("Look: style {}", preset.style));
    }
    let list = |map: &serde_json::Map<String, Value>, each: &dyn Fn(&str, &Value) -> String| {
        map.iter()
            .map(|(key, value)| each(key, value))
            .collect::<Vec<_>>()
            .join("; ")
    };
    if !preset.choices.is_empty() {
        lines.push(format!("Steps: {}", list(&preset.choices, &choice)));
    }
    if !preset.values.is_empty() {
        lines.push(format!(
            "Settings: {}",
            list(&preset.values, &|key, value| format!(
                "{} {}",
                words(key),
                said(value)
            ))
        ));
    }
    if !preset.short.is_empty() {
        lines.push(format!(
            "A new short starts with: {}",
            list(&preset.short, &|key, value| format!(
                "{} {}",
                words(key),
                said(value)
            ))
        ));
    }
    let size = |text: &str| text.lines().filter(|l| !l.trim().is_empty()).count();
    lines.push(format!(
        "Method: {} lines; the user's rules: {} lines",
        size(&preset.method),
        size(&preset.rules)
    ));
    let bullets = |title: &str, items: &[String], lines: &mut Vec<String>| {
        if !items.is_empty() {
            lines.push(format!("{title}:"));
            lines.extend(items.iter().map(|item| format!("- {item}")));
        }
    };
    bullets("Exceptions", &preset.exceptions, &mut lines);
    bullets("Checks before export", &preset.checks, &mut lines);
    lines.join("\n")
}

/// What the agent's guide says of a workflow: nothing for a preset with none of it.
pub fn for_the_guide(preset: &Preset) -> String {
    if preset.exceptions.is_empty() && preset.checks.is_empty() {
        return String::new();
    }
    let mut out = format!("## The user's workflow {}\n", preset.name);
    if !preset.exceptions.is_empty() {
        out.push_str("\nExceptions, which win over the method:\n\n");
        for line in &preset.exceptions {
            out.push_str(&format!("- {line}\n"));
        }
    }
    if !preset.checks.is_empty() {
        out.push_str(
            "\nChecks before you ask about export, besides `check`. Go through each and report it \
             under **Looked at**, or under **Still open** if you could not verify it:\n\n",
        );
        for line in &preset.checks {
            out.push_str(&format!("- {line}\n"));
        }
    }
    out.push_str("\n---\n\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(value: Value) -> serde_json::Map<String, Value> {
        value.as_object().cloned().unwrap()
    }

    fn workflow() -> Preset {
        let mut preset = Preset {
            name: "My tutorials".into(),
            description: "Long tutorials".into(),
            style: "My library".into(),
            version: 3,
            method: "# One\n\nTwo\n".into(),
            rules: "Keep the intro.\n".into(),
            choices: map(json!({
                "review": "thorough",
                "gaps": { "apply": false, "preset": "balanced" },
                "fillers": { "apply": true },
                "zooms": { "apply": true, "settings": { "minHoldMs": 5000 } },
            })),
            values: map(json!({ "audio": { "normalize": true } })),
            ..Preset::default()
        };
        Workflow {
            exceptions: Some(vec![
                " A take under a minute gets no chapters ".into(),
                "  ".into(),
            ]),
            checks: Some(vec!["The intro is under 20 s".into()]),
            note: Some(" The 29 Oct edit, as exported ".into()),
        }
        .apply_to(&mut preset, "Untitled 29 Oct 2026");
        preset
    }

    #[test]
    fn a_summary_reads_as_what_the_workflow_does() {
        let text = summary(&workflow());
        for part in [
            "My tutorials, version 3",
            "For: Long tutorials",
            "on \"Untitled 29 Oct 2026\": The 29 Oct edit, as exported",
            "Look: style My library",
            "fillers on",
            "gaps off",
            "review thorough",
            "zooms on (min hold ms 5000)",
            "Settings: audio normalize on",
            "Method: 2 lines; the user's rules: 1 lines",
            "- A take under a minute gets no chapters\n",
            "Checks before export:\n- The intro is under 20 s",
        ] {
            assert!(text.contains(part), "{part}\n{text}");
        }
        // An off step says only that.
        assert!(
            !text.contains("gaps off (") && !text.contains("balanced"),
            "{text}"
        );
        assert!(summary(&crate::preset::builtin()[0]).starts_with("Tutorial (built in)"));
    }

    #[test]
    fn the_guide_has_the_workflow_only_when_there_is_one() {
        let text = for_the_guide(&workflow());
        assert!(
            text.contains("wins") || text.contains("win over the method"),
            "{text}"
        );
        assert!(
            text.contains("- A take under a minute gets no chapters"),
            "{text}"
        );
        assert!(text.contains("**Looked at**"), "{text}");
        assert!(for_the_guide(&Preset::default()).is_empty());
    }

    #[test]
    fn what_a_save_leaves_out_stays_and_too_much_is_refused() {
        let mut preset = workflow();
        Workflow {
            note: Some("Second edit".into()),
            ..Workflow::default()
        }
        .apply_to(&mut preset, "Another");
        assert_eq!(preset.checks, ["The intro is under 20 s"]);
        assert_eq!(preset.approved.as_ref().unwrap().from, "Another");
        assert_eq!(preset.approved.as_ref().unwrap().note, "Second edit");
        preset.validate().unwrap();
        preset.checks = vec!["x".into(); MAX_LINES + 1];
        assert!(preset.validate().unwrap_err().contains("at most"));
        preset.checks = vec!["x".repeat(MAX_LINE_CHARS + 1)];
        assert!(preset.validate().unwrap_err().contains("characters"));
    }

    /// The whole thing on a second tutorial's document: a workflow's settings and words reach a
    /// project that only follows it.
    #[test]
    fn a_workflow_saved_on_one_edit_runs_the_next() {
        let dir = tempfile::tempdir().unwrap();
        let presets = dir.path();
        let mut first = crate::project::revision::EditDocument::default();
        first.captions.enabled = true;
        let mut approved = crate::preset::builtin()[0].clone();
        approved.values = crate::preset::captured(&first);
        Workflow {
            checks: Some(vec!["The intro is under 20 s".into()]),
            ..Workflow::default()
        }
        .apply_to(&mut approved, "First tutorial");
        let saved = crate::preset::store::save_in(
            presets,
            &crate::preset::store::PresetInput {
                name: "My tutorials".into(),
                previous: None,
                method: approved.method.clone(),
                rules: String::new(),
                preset: approved,
            },
        )
        .unwrap();
        // A second tutorial starts from it, with nothing changed of its own.
        let inherited = saved.inherited();
        assert_eq!(inherited["captions"]["enabled"], true);
        assert!(for_the_guide(&saved).contains("The intro is under 20 s"));
        assert_eq!(saved.approved.as_ref().unwrap().from, "First tutorial");
    }
}
