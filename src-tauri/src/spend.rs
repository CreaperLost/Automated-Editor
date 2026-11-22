//! What a video cost: each paid call the app makes for a project (narration, Treg searches and
//! page reads, drawn pictures, generated thumbnails, online transcription) written to the
//! project's `spend.json` as it happens, outside the undo history (money spent stays spent).
//! A cost the service didn't report is kept as unknown, never as nothing. The agent's own
//! tokens and calls made before the project existed (discovering stories) are not here.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FILE: &str = "spend.json";

/// What the report never covers, said with every total.
pub const NOT_COUNTED: &str = "Not counted: your agent's own tokens, story discovery before \
    the video was made, and the AI review passes (fillers, chapters, clips).";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub at: String,
    /// What was paid for: "Narration", "Web search", "Drawn picture"...
    pub step: String,
    /// Who was paid: OpenRouter, Treg, ElevenLabs.
    pub provider: String,
    /// In dollars; none when the service didn't say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// A few words on what it was (the query, how many scenes).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

pub fn path(root: &Path) -> PathBuf {
    root.join(FILE)
}

pub fn load(root: &Path) -> Vec<Entry> {
    crate::files::load_json(&path(root)).unwrap_or_default()
}

/// Adds a paid call to the project's record. A failure to write is not the call's failure:
/// it is reported to the log and the call's result stands.
pub fn record(root: &Path, step: &str, provider: &str, cost_usd: Option<f64>, note: &str) {
    let mut all = load(root);
    all.push(Entry {
        at: chrono::Utc::now().to_rfc3339(),
        step: step.into(),
        provider: provider.into(),
        cost_usd,
        note: note.chars().take(120).collect(),
    });
    if let Err(e) = crate::files::save_json(&path(root), &all) {
        eprintln!("spend: {e}");
    }
}

/// Dollars from Treg's micro-dollars.
pub fn from_micro(micro: u64) -> f64 {
    micro as f64 / 1e6
}

/// One line of the report: a provider's or a step's total.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Total {
    pub calls: usize,
    /// What the calls with a known cost add up to.
    pub known_usd: f64,
    /// Calls whose cost the service didn't report.
    pub unknown: usize,
}

impl Total {
    fn add(&mut self, entry: &Entry) {
        self.calls += 1;
        match entry.cost_usd {
            Some(c) => self.known_usd += c,
            None => self.unknown += 1,
        }
    }

    fn line(&self) -> String {
        let mut out = format!(
            "${:.3} over {} call{}",
            self.known_usd,
            self.calls,
            if self.calls == 1 { "" } else { "s" }
        );
        if self.unknown > 0 {
            out.push_str(&format!(" ({} of unknown cost)", self.unknown));
        }
        out
    }
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub total: Total,
    pub by_provider: BTreeMap<String, Total>,
    pub by_step: BTreeMap<String, Total>,
    pub first_at: Option<String>,
    pub last_at: Option<String>,
}

pub fn report(entries: &[Entry]) -> Report {
    let mut out = Report::default();
    for entry in entries {
        out.total.add(entry);
        out.by_provider
            .entry(entry.provider.clone())
            .or_default()
            .add(entry);
        out.by_step
            .entry(entry.step.clone())
            .or_default()
            .add(entry);
    }
    out.first_at = entries.first().map(|e| e.at.clone());
    out.last_at = entries.last().map(|e| e.at.clone());
    out
}

impl Report {
    /// The report as an agent or the user reads it.
    pub fn text(&self) -> String {
        if self.total.calls == 0 {
            return format!("No paid calls recorded for this video yet. {NOT_COUNTED}");
        }
        let mut lines = vec![format!("Spent on this video: {}.", self.total.line())];
        lines.push("By provider:".into());
        for (provider, total) in &self.by_provider {
            lines.push(format!("- {provider}: {}", total.line()));
        }
        lines.push("By step:".into());
        for (step, total) in &self.by_step {
            lines.push(format!("- {step}: {}", total.line()));
        }
        lines.push(NOT_COUNTED.into());
        lines.join("\n")
    }

    /// In a few words, for a status line.
    pub fn short(&self) -> String {
        match self.total.calls {
            0 => "nothing spent yet".into(),
            _ if self.total.unknown > 0 => format!(
                "${:.3} spent, {} call{} of unknown cost",
                self.total.known_usd,
                self.total.unknown,
                if self.total.unknown == 1 { "" } else { "s" }
            ),
            _ => format!("${:.3} spent", self.total.known_usd),
        }
    }
}

/// What the UI shows: the report, the latest calls (newest first) and what is never counted.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub report: Report,
    pub latest: Vec<Entry>,
    pub not_counted: &'static str,
}

/// The project's spending as the UI shows it, with at most `latest` calls listed.
pub fn view(root: &Path, latest: usize) -> View {
    let entries = load(root);
    View {
        report: report(&entries),
        latest: entries.iter().rev().take(latest).cloned().collect(),
        not_counted: NOT_COUNTED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paid_calls_add_up_by_provider_and_step_and_unknown_costs_stay_unknown() {
        let dir = tempfile::tempdir().unwrap();
        assert!(report(&load(dir.path()))
            .text()
            .starts_with("No paid calls"));
        record(
            dir.path(),
            "Web search",
            "Treg",
            Some(from_micro(4_000)),
            "gpt-5 launch",
        );
        record(
            dir.path(),
            "Narration",
            "OpenRouter",
            Some(0.042),
            "9 scenes",
        );
        record(dir.path(), "Transcription", "ElevenLabs", None, "narration");
        record(
            dir.path(),
            "Web search",
            "Treg",
            Some(from_micro(2_500)),
            "",
        );
        let entries = load(dir.path());
        assert_eq!(entries.len(), 4);
        let r = report(&entries);
        assert_eq!(r.total.calls, 4);
        assert_eq!(r.total.unknown, 1);
        assert!((r.total.known_usd - 0.0485).abs() < 1e-9);
        assert_eq!(r.by_provider["Treg"].calls, 2);
        assert!((r.by_step["Web search"].known_usd - 0.0065).abs() < 1e-9);
        // The UI's view: newest first, as many as asked, the same totals.
        let v = view(dir.path(), 2);
        assert_eq!(v.report, r);
        assert_eq!(v.latest.len(), 2);
        assert_eq!(v.latest[0].step, "Web search");
        assert_eq!(v.latest[1].provider, "ElevenLabs");
        let json = serde_json::to_value(&v).unwrap();
        assert!(json["report"]["byProvider"]["Treg"]["knownUsd"].is_number());
        assert!(json["notCounted"]
            .as_str()
            .unwrap()
            .starts_with("Not counted"));
        let text = r.text();
        assert!(
            text.contains("Spent on this video: $0.049 over 4 calls (1 of unknown cost)."),
            "{text}"
        );
        assert!(
            text.contains("- ElevenLabs: $0.000 over 1 call (1 of unknown cost)"),
            "{text}"
        );
        assert!(text.contains("Not counted"));
        assert_eq!(r.short(), "$0.049 spent, 1 call of unknown cost");
    }
}
