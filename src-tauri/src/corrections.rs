//! What the user did with the agent's work on this project (LOOP-1), from the records the app
//! already keeps: proposal decisions, item by item, the agent's actions the user undid, and the
//! user's own steps after the agent's (time put back, graphics rewritten, zooms or layouts
//! changed). Only counted and shown: nothing here changes a method, style or template. The
//! agent offers a rule from a pattern (`library` `kind: preset` `op: propose_rule`) and the user
//! accepts it, or not, in the Agent panel.
use crate::project::history::{Author, Step};
use crate::proposals::{Batch, Change, ItemState, Proposals, Status};
use std::collections::BTreeMap;

/// Example words kept per kind of rejected cut.
const EXAMPLES: usize = 3;

/// Counts by kind of what the user did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Corrections {
    /// Cut kind → (accepted, rejected, a few rejected texts).
    pub cuts: BTreeMap<String, (usize, usize, Vec<String>)>,
    /// Timeline-edit batches the user rejected or undid, with their reasons.
    pub edits_turned_down: Vec<String>,
    /// The app's actions the agent ran that were undone, by action.
    pub actions_undone: BTreeMap<String, usize>,
    /// The user's own steps after the agent's, by what they changed.
    pub user_steps: BTreeMap<String, usize>,
    /// Of those, ones that put time back (a cut restored).
    pub time_put_back: Vec<String>,
}

fn kind_name<T: serde::Serialize>(kind: &T) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "cut".into())
}

/// What a user step changed, for counting: its label without numbers.
fn step_kind(step: &Step) -> String {
    if !step.files.is_empty() {
        return "graphic files".into();
    }
    let label = step.label.trim();
    if label.starts_with("Added ") && label.ends_with(" s") {
        return "time put back".into();
    }
    if label.starts_with("Cut ") && label.ends_with(" s") {
        return "time cut".into();
    }
    label
        .split(|c: char| c.is_ascii_digit())
        .next()
        .unwrap_or(label)
        .trim()
        .trim_end_matches([',', ':'])
        .to_string()
}

/// Gathers the corrections from `proposals` and the undo history's `steps`, oldest first.
pub fn gather(proposals: &Proposals, steps: &[(&Step, bool)]) -> Corrections {
    let mut out = Corrections::default();
    for batch in &proposals.batches {
        gather_batch(&mut out, batch);
    }
    // The user's own steps once an agent has worked here, and still in the edit.
    let mut agent_seen = false;
    for (step, undone) in steps {
        if step.by == Author::Agent {
            agent_seen = true;
            continue;
        }
        if !agent_seen || *undone || step.undoes_run.is_some() {
            continue;
        }
        let kind = step_kind(step);
        if kind.is_empty() {
            continue;
        }
        if kind == "time put back" {
            out.time_put_back.push(step.label.clone());
        }
        *out.user_steps.entry(kind).or_default() += 1;
    }
    out
}

fn gather_batch(out: &mut Corrections, batch: &Batch) {
    match &batch.change {
        Change::Cuts { items, .. } => {
            for item in items {
                let entry = out.cuts.entry(kind_name(&item.kind)).or_default();
                match item.state {
                    ItemState::Accepted => entry.0 += 1,
                    ItemState::Rejected => {
                        entry.1 += 1;
                        if entry.2.len() < EXAMPLES {
                            entry.2.push(item.text.chars().take(60).collect());
                        }
                    }
                    ItemState::Pending => {}
                }
            }
        }
        Change::Edits { .. } if matches!(batch.status, Status::Rejected | Status::Undone) => {
            out.edits_turned_down
                .push(batch.reason.chars().take(120).collect());
        }
        Change::Action { action } if batch.status == Status::Undone => {
            *out.actions_undone.entry(action.clone()).or_default() += 1;
        }
        _ => {}
    }
}

impl Corrections {
    pub fn is_empty(&self) -> bool {
        self.cuts.values().all(|c| c.1 == 0)
            && self.edits_turned_down.is_empty()
            && self.actions_undone.is_empty()
            && self.user_steps.is_empty()
    }

    /// The report the agent reads.
    pub fn text(&self) -> String {
        if self.is_empty() {
            return "No corrections yet: the user hasn't turned down or redone any of the \
                    agent's work here."
                .into();
        }
        let mut lines =
            vec!["What the user did with the agent's work on this project:".to_string()];
        for (kind, (accepted, rejected, examples)) in &self.cuts {
            if *rejected == 0 && *accepted == 0 {
                continue;
            }
            let mut line = format!("- {kind} cuts: {accepted} accepted, {rejected} rejected");
            if !examples.is_empty() {
                line.push_str(&format!(
                    " (rejected: {})",
                    examples
                        .iter()
                        .map(|e| format!("\"{e}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            lines.push(line);
        }
        for reason in &self.edits_turned_down {
            lines.push(format!("- timeline edits turned down: {reason}"));
        }
        for (action, n) in &self.actions_undone {
            lines.push(format!("- the agent's {action} undone {n}x"));
        }
        if !self.user_steps.is_empty() {
            let mut steps: Vec<(&String, &usize)> = self.user_steps.iter().collect();
            steps.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            lines.push(format!(
                "- the user's own changes since: {}",
                steps
                    .iter()
                    .map(|(k, n)| format!("{k} {n}x"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !self.time_put_back.is_empty() {
            lines.push(format!(
                "- time put back: {}",
                self.time_put_back.join(", ")
            ));
        }
        lines.push(
            "The same correction twice is a rule to offer: `library` `kind: preset` \
             `op: propose_rule` (their words, the example). The user accepts it; one rejection \
             is not a rule."
                .into(),
        );
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proposals::CutItem;
    use crate::transcript::TranscriptSuggestionKind as Kind;

    fn step(label: &str, by: Author, files: bool) -> Step {
        let mut s: Step = serde_json::from_value(serde_json::json!({
            "before": "a", "after": "b", "label": label, "at": "2026-11-21T00:00:00Z"
        }))
        .unwrap();
        s.by = by;
        if files {
            s.files = vec!["f".into()];
        }
        s
    }

    fn item(kind: Kind, text: &str, state: ItemState) -> CutItem {
        CutItem {
            id: "c-1".into(),
            kind,
            first_word_id: None,
            last_word_id: None,
            start_us: Some(0),
            end_us: Some(1),
            text: text.into(),
            reason: String::new(),
            state,
            revision: None,
        }
    }

    fn batch(change: Change, status: Status) -> Batch {
        Batch {
            id: "b-1".into(),
            agent: "claude-code".into(),
            reason: "Tighter opening".into(),
            created_at: String::new(),
            base_revision: 1,
            change,
            status,
            decided_at: None,
            auto: false,
            applied_revision: None,
            summary: Vec::new(),
        }
    }

    #[test]
    fn decisions_undone_actions_and_the_users_own_steps_are_counted() {
        let proposals = Proposals {
            next_id: 4,
            batches: vec![
                batch(
                    Change::Cuts {
                        stream: "rec.mic".into(),
                        words: None,
                        items: vec![
                            item(Kind::Filler, "um", ItemState::Accepted),
                            item(Kind::Filler, "so", ItemState::Rejected),
                            item(Kind::Filler, "like", ItemState::Rejected),
                        ],
                        short: None,
                    },
                    Status::Accepted,
                ),
                batch(
                    Change::Action {
                        action: "zooms".into(),
                    },
                    Status::Undone,
                ),
                batch(Change::Edits { edits: Vec::new() }, Status::Rejected),
            ],
        };
        let early = step("Zooms", Author::User, false);
        let agent = step("Cut 3.0 s", Author::Agent, false);
        let back = step("Added 1.2 s", Author::User, false);
        let graphic = step("Changed a graphic", Author::User, true);
        let zooms = step("Zooms", Author::User, false);
        let undone = step("Layouts", Author::User, false);
        let steps = vec![
            (&early, false),
            (&agent, false),
            (&back, false),
            (&graphic, false),
            (&zooms, false),
            (&undone, true),
        ];
        let found = gather(&proposals, &steps);
        assert_eq!(found.cuts["filler"].0, 1);
        assert_eq!(found.cuts["filler"].1, 2);
        assert_eq!(found.actions_undone["zooms"], 1);
        assert_eq!(found.edits_turned_down, vec!["Tighter opening".to_string()]);
        // Before the agent and undone steps don't count.
        assert_eq!(found.user_steps.get("Zooms"), Some(&1));
        assert_eq!(found.user_steps.get("Layouts"), None);
        assert_eq!(found.user_steps["graphic files"], 1);
        assert_eq!(found.time_put_back, vec!["Added 1.2 s".to_string()]);
        let text = found.text();
        assert!(
            text.contains("- filler cuts: 1 accepted, 2 rejected (rejected: \"so\", \"like\")"),
            "{text}"
        );
        assert!(text.contains("propose_rule"), "{text}");
        assert!(gather(&Proposals::default(), &[])
            .text()
            .starts_with("No corrections"));
    }
}
