//! Checks a faceless video again when it is released (FCL-9): words that tie it to the day it
//! was written ("today", "just announced", "this week") are refused once the release is too far
//! from that day, and every page the script cites is read again, so a quote taken down or a new
//! correction notice stops the upload.

use super::evidence::{self, Page, PageFetch, SourceStatus};
use super::script::Script;
use chrono::{DateTime, Utc};
use std::collections::BTreeSet;
use std::path::Path;

/// Words that say "the day it was written", true for about half a day.
const SAME_DAY: [&str; 11] = [
    "today",
    "tonight",
    "this morning",
    "this afternoon",
    "this evening",
    "yesterday",
    "last night",
    "right now",
    "hours ago",
    "breaking",
    "earlier today",
];
/// Words that stay true for a few days.
const SAME_WEEK: [&str; 4] = ["this week", "days ago", "last week", "this weekend"];
const SAME_DAY_HOURS: i64 = 12;
const SAME_WEEK_HOURS: i64 = 72;

/// The time words in `text` and how many hours each stays true, in the order found.
pub fn time_words(text: &str) -> Vec<(String, i64)> {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '’')
        .map(|w| {
            let w = w.to_lowercase();
            // "Today's model": the possessive is the same word.
            w.strip_suffix("'s")
                .or_else(|| w.strip_suffix("’s"))
                .unwrap_or(&w)
                .trim_matches(|c| c == '\'' || c == '’')
                .to_string()
        })
        .filter(|w| !w.is_empty())
        .collect();
    let mut found = Vec::new();
    for (i, word) in words.iter().enumerate() {
        let pair = words.get(i + 1).map(|next| format!("{word} {next}"));
        for (list, hours) in [
            (&SAME_DAY[..], SAME_DAY_HOURS),
            (&SAME_WEEK[..], SAME_WEEK_HOURS),
        ] {
            for phrase in list {
                let hit = if phrase.contains(' ') {
                    pair.as_deref() == Some(phrase)
                } else {
                    word == phrase
                };
                // "earlier today" also holds "today": say it once.
                if hit && !(*phrase == "today" && i > 0 && words[i - 1] == "earlier") {
                    found.push((phrase.to_string(), hours));
                }
            }
        }
        // "just announced", "just released": news of the moment. "Just 3%" is not.
        if word == "just" {
            if let Some(next) = words.get(i + 1) {
                if next.len() > 3 && next.ends_with("ed") {
                    found.push((format!("just {next}"), SAME_DAY_HOURS));
                }
            }
        }
    }
    found
}

/// What publishing would say that the release time makes untrue, or no longer checked: one line
/// each. `title` and `description` are the package's; `written_at` when the script was finished.
pub fn stale_wording(
    script: &Script,
    title: &str,
    description: &str,
    written_at: DateTime<Utc>,
    release_at: DateTime<Utc>,
) -> Vec<String> {
    let hours = (release_at - written_at).num_hours();
    let mut places: Vec<(String, &str)> = vec![
        ("The title".into(), title),
        ("The description".into(), description),
    ];
    for (i, scene) in script.scenes.iter().enumerate() {
        places.push((format!("Scene {}", i + 1), scene.narration.as_str()));
        places.push((
            format!("Scene {}'s on-screen words", i + 1),
            scene.on_screen.as_str(),
        ));
    }
    let mut out = Vec::new();
    for (place, text) in places {
        let stale: Vec<String> = time_words(text)
            .into_iter()
            .filter(|(_, true_for)| hours > *true_for)
            .map(|(word, _)| format!("\"{word}\""))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if !stale.is_empty() {
            out.push(format!(
                "{place} says {} but it goes out {} after the script was written",
                stale.join(", "),
                span(hours)
            ));
        }
    }
    out
}

fn span(hours: i64) -> String {
    if hours < 48 {
        format!("{hours} hours")
    } else {
        format!("{} days", hours / 24)
    }
}

/// Reads every page the script cites again: a page that can't be read, a cited quote no longer
/// on it, or a correction or update notice that wasn't there at research time is a problem.
pub fn sources_changed(root: &Path, script: &Script, web: &dyn PageFetch) -> Vec<String> {
    let Some(dossier) = evidence::load(root) else {
        return vec!["The research is missing, so the sources can't be checked again".into()];
    };
    let cited: BTreeSet<&str> = script
        .scenes
        .iter()
        .flat_map(|s| s.fact_ids.iter())
        .chain(script.title_fact_ids.iter())
        .map(String::as_str)
        .collect();
    let facts: Vec<&evidence::Fact> = dossier
        .facts
        .iter()
        .filter(|f| cited.contains(f.id.as_str()))
        .collect();
    let urls: BTreeSet<&str> = facts.iter().map(|f| f.input.url.as_str()).collect();
    let pages: Vec<(&str, Page)> = std::thread::scope(|scope| {
        let handles: Vec<_> = urls
            .iter()
            .map(|url| scope.spawn(move || (*url, web.page(url))))
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    let mut out = Vec::new();
    for (url, page) in pages {
        let text = match page {
            Page::Text { text, .. } => text,
            Page::Unreadable(reason) | Page::Paywalled(reason) => {
                out.push(format!(
                    "{url} could not be read again ({reason}): try again, or take its facts out"
                ));
                continue;
            }
        };
        for fact in facts.iter().filter(|f| f.input.url == url) {
            if let Some(reason) = evidence::verify(&fact.input.statement, &fact.input.quote, &text)
            {
                out.push(format!(
                    "{} is no longer backed by {url}: {reason}",
                    fact.id
                ));
            }
        }
        let before: Vec<&String> = dossier
            .sources
            .iter()
            .filter(|s| s.url == url && s.status == SourceStatus::Fetched)
            .flat_map(|s| s.notices.iter())
            .collect();
        for notice in evidence::notices(&text) {
            if !before.contains(&&notice) {
                out.push(format!(
                    "{url} has a new notice since the research: {notice}"
                ));
            }
        }
    }
    out
}

/// Everything that stops faceless production `root` going out at `release_at`; empty when it
/// may. An ordinary project, or one with no script, has nothing to check.
pub fn check(
    root: &Path,
    title: &str,
    description: &str,
    release_at: DateTime<Utc>,
    web: &dyn PageFetch,
) -> Vec<String> {
    let (Some(production), Some(script)) = (super::load(root), super::script::load_script(root))
    else {
        return Vec::new();
    };
    let written_at = production
        .stages
        .get("script")
        .and_then(|s| DateTime::parse_from_rfc3339(&s.at).ok())
        .map(|t| t.with_timezone(&Utc));
    let mut out = match written_at {
        Some(at) => stale_wording(&script, title, description, at, release_at),
        None => vec!["The script was never finished, so its wording can't be dated".into()],
    };
    out.extend(sources_changed(root, &script, web));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faceless::script::Scene;

    fn scene(narration: &str, facts: &[&str]) -> Scene {
        Scene {
            narration: narration.into(),
            fact_ids: facts.iter().map(|f| f.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn time_words_are_found_once_and_just_only_before_news() {
        let words: Vec<String> = time_words(
            "Earlier today OpenAI just released a model. It costs just 3% more. This week, \
             nothing. Just used? No.",
        )
        .into_iter()
        .map(|(w, _)| w)
        .collect();
        assert_eq!(
            words,
            ["earlier today", "just released", "this week", "just used"]
        );
        assert_eq!(time_words("Today's model beats yesterday’s?").len(), 2);
    }

    #[test]
    fn wording_goes_stale_after_its_window() {
        let script = Script {
            title: "t".into(),
            scenes: vec![scene("Apple just announced a robot today.", &[])],
            ..Default::default()
        };
        let written = Utc::now();
        let soon = stale_wording(
            &script,
            "",
            "",
            written,
            written + chrono::Duration::hours(3),
        );
        assert!(soon.is_empty(), "{soon:?}");
        let later = stale_wording(
            &script,
            "A robot this week",
            "",
            written,
            written + chrono::Duration::hours(30),
        );
        assert_eq!(later.len(), 1, "{later:?}");
        assert!(
            later[0]
                .starts_with("Scene 1 says \"just announced\", \"today\" but it goes out 30 hours"),
            "{}",
            later[0]
        );
        let days = stale_wording(
            &script,
            "A robot this week",
            "",
            written,
            written + chrono::Duration::days(4),
        );
        assert_eq!(days.len(), 2);
        assert!(
            days[0].starts_with("The title says \"this week\""),
            "{days:?}"
        );
    }

    struct Pages(Vec<(&'static str, Page)>);

    impl PageFetch for Pages {
        fn page(&self, url: &str) -> Page {
            match self.0.iter().find(|(u, _)| *u == url) {
                Some((_, Page::Text { text, .. })) => Page::Text {
                    text: text.clone(),
                    published_on: None,
                    via: None,
                },
                Some((_, Page::Unreadable(r))) => Page::Unreadable(r.clone()),
                Some((_, Page::Paywalled(r))) => Page::Paywalled(r.clone()),
                None => Page::Unreadable("HTTP 404".into()),
            }
        }
    }

    fn text(text: &str) -> Page {
        Page::Text {
            text: text.into(),
            published_on: None,
            via: None,
        }
    }

    const A: &str = "https://example.com/a";
    const B: &str = "https://example.org/b";
    const PAGE_A: &str = "The company said the robot walks at 4 km per hour in its own tests.";
    const PAGE_B: &str = "Reviewers found the battery lasts 2 hours under load.";

    /// A production with two researched pages, one fact from each, and a script citing both.
    fn production(dir: &Path) -> std::path::PathBuf {
        let root = crate::faceless::create(
            dir,
            "Robot",
            crate::faceless::Format::Short,
            &[A.to_string()],
        )
        .unwrap();
        let web = Pages(vec![(A, text(PAGE_A)), (B, text(PAGE_B))]);
        evidence::fetch(&root, &web, &[A.into(), B.into()], false).unwrap();
        let added = evidence::add_facts(
            &root,
            vec![
                evidence::FactInput {
                    statement: "The robot walks at 4 km per hour in the company's tests".into(),
                    quote: "the robot walks at 4 km per hour in its own tests".into(),
                    url: A.into(),
                    kind: None,
                    scope: None,
                    uncertainty: None,
                    counterpoint: None,
                    number_context: None,
                },
                evidence::FactInput {
                    statement: "The battery lasts 2 hours under load".into(),
                    quote: "the battery lasts 2 hours under load".into(),
                    url: B.into(),
                    kind: None,
                    scope: None,
                    uncertainty: None,
                    counterpoint: None,
                    number_context: None,
                },
            ],
            &[],
        )
        .unwrap();
        assert_eq!(added.kept.len(), 2, "{:?}", added.refused);
        let script = Script {
            title: "Robot".into(),
            scenes: vec![
                scene("It walks at 4 km per hour.", &[&added.kept[0]]),
                scene("The battery lasts 2 hours.", &[&added.kept[1]]),
            ],
            ..Default::default()
        };
        crate::files::save_json(&crate::faceless::script::script_path(&root), &script).unwrap();
        root
    }

    #[test]
    fn unchanged_sources_pass_and_a_changed_one_is_named() {
        let dir = tempfile::tempdir().unwrap();
        let root = production(dir.path());
        let script = crate::faceless::script::load_script(&root).unwrap();
        let same = Pages(vec![(A, text(PAGE_A)), (B, text(PAGE_B))]);
        assert!(sources_changed(&root, &script, &same).is_empty());

        let changed = Pages(vec![
            (
                A,
                text("The company said the robot walks at 3 km per hour."),
            ),
            (
                B,
                text(&format!(
                    "{PAGE_B} Correction: an earlier version of this story misstated the test."
                )),
            ),
        ]);
        let problems = sources_changed(&root, &script, &changed);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].contains("is no longer backed by https://example.com/a"));
        assert!(problems[1].contains("https://example.org/b has a new notice"));

        let gone = Pages(vec![(B, text(PAGE_B))]);
        let problems = sources_changed(&root, &script, &gone);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("could not be read again (HTTP 404)"));
    }
}
