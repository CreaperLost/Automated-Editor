//! The script and its review (FCL-3), checked by code, ported from the explainer's `script.py`.
//! The agent writes; the app refuses a script whose scenes cite no fact, cite facts that don't
//! exist, or say a number none of their cited quotes has, and notes what a program can judge of
//! the style (long sentences, a long first sentence, tired phrases, length).
use super::evidence::{normalize, numbers, Dossier};
use super::Format;
use serde::{Deserialize, Serialize};

/// Measured on the explainer's Codex Short: 123 words in 50.7 s before speeding up.
pub const WORDS_PER_SECOND: f64 = 2.43;
/// The channel's narration pace (the voice is sped up after it is made).
pub const DEFAULT_SPEED: f64 = 1.2;
const MAX_HOOK_WORDS: usize = 20;
const MAX_SENTENCE_WORDS: usize = 28;
const FRAGMENT_WORDS: usize = 4;
const LENGTH_TOLERANCE: f64 = 0.05;

pub fn seconds(format: Format) -> (f64, f64) {
    match format {
        Format::Long => (300.0, 480.0),
        Format::Short => (30.0, 58.0),
    }
}

/// The words a script of `format` may have at `speed`.
pub fn word_limits(format: Format, speed: f64) -> (usize, usize) {
    let (low, high) = seconds(format);
    let pace = WORDS_PER_SECOND * speed;
    (
        (low * pace).round() as usize,
        (high * pace).round() as usize,
    )
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    /// What the voice says. Measured numbers in digits ("40%", "$6.34 billion").
    pub narration: String,
    /// Facts this scene states (ids from the research). Empty only for analysis.
    #[serde(default)]
    pub fact_ids: Vec<String>,
    /// Your own inference, worded as inference. May not state new facts.
    #[serde(default)]
    pub analysis: bool,
    /// Words on screen, adding to the spoken line rather than repeating it.
    #[serde(default)]
    pub on_screen: String,
    /// "Source: <publisher>" when the scene states a specific claim, number or quote.
    #[serde(default)]
    pub source: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    pub title: String,
    /// Other titles considered.
    #[serde(default)]
    pub title_options: Vec<String>,
    /// The facts the title rests on.
    #[serde(default)]
    pub title_fact_ids: Vec<String>,
    /// The hooks considered, before choosing.
    #[serde(default)]
    pub hooks: Vec<String>,
    /// One plain question the video answers (long), or the one idea "X, but Y" (short).
    #[serde(default)]
    pub spine: String,
    /// What the first frame shows, readable with the sound off.
    #[serde(default)]
    pub first_frame: String,
    pub scenes: Vec<Scene>,
    /// Facts deliberately left out, and why.
    #[serde(default)]
    pub left_out: Vec<String>,
}

fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        current.push(*c);
        if matches!(c, '.' | '!' | '?') {
            let next = chars.get(i + 1);
            let after = chars.get(i + 2);
            if next.is_none_or(|n| n.is_whitespace())
                && after.is_none_or(|a| {
                    a.is_uppercase() || a.is_ascii_digit() || *a == '"' || *a == '$'
                })
            {
                let s = current.trim().to_string();
                if !s.is_empty() {
                    out.push(s);
                }
                current.clear();
            }
        }
    }
    let rest = current.trim();
    if !rest.is_empty() {
        out.push(rest.to_string());
    }
    out
}

fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

fn number_set(text: &str) -> Vec<String> {
    let mut found: Vec<String> = numbers(&normalize(text))
        .into_iter()
        .map(|n| n.replace(',', ""))
        .collect();
    found.sort();
    found.dedup();
    found
}

/// What blocks a script (`errors`) and what only asks for a look (`notes`).
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub errors: Vec<String>,
    pub notes: Vec<String>,
    pub words: usize,
    pub estimated_seconds: f64,
}

/// Checks a script against the research.
pub fn check(script: &Script, dossier: &Dossier, format: Format, speed: f64) -> Check {
    let mut errors = Vec::new();
    let mut notes = Vec::new();
    let facts: std::collections::BTreeMap<&str, &str> = dossier
        .facts
        .iter()
        .map(|f| (f.id.as_str(), f.input.quote.as_str()))
        .collect();
    if script.scenes.is_empty() {
        errors.push("The script has no scenes.".into());
    }
    if script
        .scenes
        .first()
        .is_some_and(|s| s.narration.trim().is_empty())
    {
        errors.push("The opening scene must have narration.".into());
    }
    if script.first_frame.trim().is_empty() {
        errors.push("Say what the first frame shows (it must make sense on mute).".into());
    }

    // Title.
    let title = script.title.trim();
    let max = if format == Format::Short { 70 } else { 55 };
    let chars = title.chars().count();
    if title.is_empty() || chars > 100 {
        errors.push(format!(
            "The title has {chars} characters: YouTube takes 1 to 100."
        ));
    } else if !(30..=max).contains(&chars) {
        notes.push(format!(
            "The title has {chars} characters; aim for 30 to {max}."
        ));
    }
    if title.matches('!').count() > 1 || title.contains("...") || title.contains('…') {
        notes.push("At most one exclamation mark, at the end; no ellipses.".into());
    }
    let title_ids: Vec<&String> = if script.title_fact_ids.is_empty() {
        script
            .scenes
            .first()
            .map(|s| s.fact_ids.iter().collect())
            .unwrap_or_default()
    } else {
        script.title_fact_ids.iter().collect()
    };
    if title_ids.is_empty() || title_ids.iter().any(|id| !facts.contains_key(id.as_str())) {
        errors.push("The title must rest on existing facts: give titleFactIds.".into());
    } else {
        let quotes = title_ids
            .iter()
            .filter_map(|id| facts.get(id.as_str()))
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        let quoted = number_set(&quotes);
        for n in number_set(title) {
            if !quoted.contains(&n) {
                errors.push(format!(
                    "The title says {n}, which is in none of its facts' quotes."
                ));
            }
        }
    }

    // Scenes: evidence.
    let all_quoted: Vec<String> = number_set(
        &dossier
            .facts
            .iter()
            .map(|f| f.input.quote.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    );
    let spelled = regex::Regex::new(
        r"(?i)\b(?:one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|\w+teen|twenty|thirty|forty|fifty|sixty|seventy|eighty|ninety|hundred|thousand)[\s-]+(?:hundred|thousand|million|billion|trillion|percent)\b",
    )
    .unwrap();
    let tired = regex::Regex::new(
        r"(?i)here's the thing|the takeaway|let's dive|in conclusion|at the end of the day|buckle up|game.chang|what does this mean|make no mistake|it's worth noting|the bottom line",
    )
    .unwrap();
    let mut said: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (i, scene) in script.scenes.iter().enumerate() {
        let n = i + 1;
        let unknown: Vec<&String> = scene
            .fact_ids
            .iter()
            .filter(|id| !facts.contains_key(id.as_str()))
            .collect();
        if !unknown.is_empty() {
            errors.push(format!(
                "Scene {n} cites facts that don't exist: {}.",
                unknown
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let known: Vec<&str> = scene
            .fact_ids
            .iter()
            .filter_map(|id| facts.get(id.as_str()).copied())
            .collect();
        if known.is_empty() && !scene.analysis {
            errors.push(format!(
                "Scene {n} cites no fact and isn't marked analysis."
            ));
        }
        let quoted = number_set(&known.join(" "));
        for number in number_set(&scene.narration) {
            if quoted.contains(&number) {
                continue;
            }
            // Analysis may use a number any fact has, never a new one.
            if scene.analysis && all_quoted.contains(&number) {
                continue;
            }
            let holders: Vec<&str> = dossier
                .facts
                .iter()
                .filter(|f| number_set(&f.input.quote).contains(&number))
                .map(|f| f.id.as_str())
                .take(5)
                .collect();
            errors.push(if holders.is_empty() {
                format!("Scene {n} says {number}, which no fact has: cut it.")
            } else {
                format!(
                    "Scene {n} says {number}, which is in none of the quotes it cites. {} {} it: cite one only if it supports this sentence.",
                    holders.join(", "),
                    if holders.len() == 1 { "has" } else { "have" }
                )
            });
        }
        if let Some(m) = spelled.find(&scene.narration) {
            errors.push(format!(
                "Scene {n} spells out \"{}\": write it in digits so it can be checked.",
                m.as_str()
            ));
        }
        // Style: notes, not refusals.
        let ss = sentences(&scene.narration);
        if n == 1 {
            if let Some(first) = ss.first() {
                if words(first) > MAX_HOOK_WORDS {
                    notes.push(format!(
                        "The first sentence has {} words; keep it to {MAX_HOOK_WORDS}.",
                        words(first)
                    ));
                }
                let lower = first.to_lowercase();
                if [
                    "hey",
                    "hi ",
                    "welcome",
                    "in this video",
                    "imagine",
                    "have you ever",
                ]
                .iter()
                .any(|p| lower.starts_with(p))
                    || first.trim_end().ends_with('?')
                {
                    notes.push(
                        "Don't open with a greeting, \"in this video\", \"imagine\" or a question: open on the news.".into(),
                    );
                }
            }
        }
        let mut run = 0;
        for s in &ss {
            if words(s) > MAX_SENTENCE_WORDS {
                notes.push(format!(
                    "Scene {n} has a {}-word sentence; split it: \"{s}\"",
                    words(s)
                ));
            }
            run = if words(s) <= FRAGMENT_WORDS {
                run + 1
            } else {
                0
            };
            if run == 3 {
                notes.push(format!(
                    "Scene {n} has three fragments in a row, ending \"{s}\": join them."
                ));
            }
            let key: String = s
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if words(&key) >= 5 {
                match said.get(&key) {
                    Some(&other) if other != n => errors.push(format!(
                        "Scene {n} repeats a sentence from scene {other}: \"{s}\". Say it once."
                    )),
                    _ => {
                        said.insert(key, n);
                    }
                }
            }
        }
        if let Some(m) = tired.find(&scene.narration) {
            notes.push(format!(
                "Scene {n} says \"{}\": say it plainly or cut it.",
                m.as_str()
            ));
        }
    }
    let analysis = script.scenes.iter().filter(|s| s.analysis).count();
    if format == Format::Long && analysis < 2 && !script.scenes.is_empty() {
        notes.push("A long video should have at least two analysis passages.".into());
    }

    // Length.
    let total: usize = script.scenes.iter().map(|s| words(&s.narration)).sum();
    let (low, high) = word_limits(format, speed);
    let ceiling = if format == Format::Short {
        high as f64
    } else {
        high as f64 * (1.0 + LENGTH_TOLERANCE)
    };
    if (total as f64) < low as f64 * (1.0 - LENGTH_TOLERANCE) || total as f64 > ceiling {
        errors.push(format!(
            "The narration has {total} words; a {} video needs {low} to {high} at this pace.{}",
            format.label(),
            if total > high {
                " Cut the longest scenes first."
            } else if format == Format::Long {
                " If the story lacks three real shifts, make it a short instead of stretching it."
            } else {
                ""
            }
        ));
    }
    Check {
        errors,
        notes,
        words: total,
        estimated_seconds: (total as f64 / (WORDS_PER_SECOND * speed) * 10.0).round() / 10.0,
    }
}

/// One review finding.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// `attention`, `coldReader` or `fairness`.
    pub review: String,
    /// The script's exact words.
    pub claim: String,
    /// For fairness: unsupported, overstated, lostLimit, misattributed or misleadingOrder.
    #[serde(default)]
    pub problem: String,
    /// `blocker` holds the video until fixed; `minor` gets one revision.
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub why: String,
    /// The words to use instead.
    #[serde(default)]
    pub change: String,
    /// Fixed in the script since.
    #[serde(default)]
    pub resolved: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    /// The script the reviews read (its hash), so a changed script needs a new review.
    pub script: String,
    pub findings: Vec<Finding>,
    /// The cold reader's answers, to compare with the evidence.
    #[serde(default)]
    pub cold_reader: serde_json::Value,
}

impl Review {
    /// Blockers still open: the video holds until each is fixed in the script.
    pub fn open_blockers(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == "blocker" && !f.resolved)
            .collect()
    }
}

/// A blocker's words still in the script: marking it resolved is refused.
pub fn still_says(script: &Script, finding: &Finding) -> bool {
    let claim = normalize(&finding.claim);
    claim.chars().count() >= 12
        && (script
            .scenes
            .iter()
            .any(|s| normalize(&s.narration).contains(&claim))
            || normalize(&script.title).contains(&claim))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faceless::evidence::{Fact, FactInput};

    fn dossier() -> Dossier {
        let mut d = Dossier::new("t");
        let fact = |id: &str, quote: &str| Fact {
            id: id.into(),
            input: FactInput {
                statement: "s".into(),
                quote: quote.into(),
                url: "https://a.com".into(),
                kind: None,
                scope: None,
                uncertainty: None,
                counterpoint: None,
                number_context: None,
            },
            domain: "a.com".into(),
            primary: false,
            source_date: None,
        };
        d.facts = vec![
            fact("f1", "Binance said $6.34 billion in assets were affected"),
            fact(
                "f2",
                "the installer reached 1,200 employees in 40 countries",
            ),
        ];
        d
    }

    fn scene(narration: &str, ids: &[&str], analysis: bool) -> Scene {
        Scene {
            narration: narration.into(),
            fact_ids: ids.iter().map(|s| s.to_string()).collect(),
            analysis,
            ..Default::default()
        }
    }

    fn short(scenes: Vec<Scene>) -> Script {
        Script {
            title: "Binance Says $6.34 Billion Was Hit by One Installer".into(),
            title_fact_ids: vec!["f1".into()],
            first_frame: "A cracked installer box".into(),
            scenes,
            ..Default::default()
        }
    }

    fn padded(text: &str, total: usize) -> String {
        let mut words: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        while words.len() < total {
            words.push("so".into());
        }
        words.join(" ")
    }

    #[test]
    fn numbers_must_come_from_the_cited_quotes() {
        let d = dossier();
        let script = short(vec![
            scene(
                &padded("Binance says $6.34 billion was affected.", 60),
                &["f1"],
                false,
            ),
            scene(
                &padded("It reached 1,200 staff in 41 countries.", 60),
                &["f2"],
                false,
            ),
            scene(
                &padded("That means about 1,200 people, so it spread fast.", 20),
                &[],
                true,
            ),
        ]);
        let c = check(&script, &d, Format::Short, DEFAULT_SPEED);
        assert_eq!(c.errors.len(), 1, "{:?}", c.errors);
        assert!(c.errors[0].contains("Scene 2 says 41"));
        // A planted number in the title.
        let mut bad = script.clone();
        bad.title = "Binance Says $6.43 Billion Was Hit by One Installer".into();
        assert!(check(&bad, &d, Format::Short, DEFAULT_SPEED)
            .errors
            .iter()
            .any(|e| e.contains("6.43")));
    }

    #[test]
    fn scenes_cite_real_facts_or_are_analysis() {
        let d = dossier();
        let script = short(vec![
            scene(
                &padded("Binance says $6.34 billion was affected.", 70),
                &["f9"],
                false,
            ),
            scene(&padded("Nobody knows who did it.", 70), &[], false),
        ]);
        let c = check(&script, &d, Format::Short, DEFAULT_SPEED);
        assert!(c.errors.iter().any(|e| e.contains("f9")));
        assert!(c.errors.iter().any(|e| e.contains("Scene 2 cites no fact")));
    }

    #[test]
    fn length_follows_the_format_and_the_pace() {
        assert_eq!(word_limits(Format::Short, 1.2), (87, 169));
        assert_eq!(word_limits(Format::Long, 1.2), (875, 1400));
        let d = dossier();
        let script = short(vec![scene(
            "Binance says $6.34 billion was affected.",
            &["f1"],
            false,
        )]);
        let c = check(&script, &d, Format::Short, DEFAULT_SPEED);
        assert!(c.errors.iter().any(|e| e.contains("needs 87 to 169")));
        assert_eq!(c.words, 6);
    }

    #[test]
    fn style_is_noted_not_refused() {
        let d = dossier();
        let script = short(vec![
            scene(
                &padded(
                    "Have you ever wondered why Binance says $6.34 billion was affected? Here's the thing.",
                    100,
                ),
                &["f1"],
                false,
            ),
            scene(&padded("It reached 1,200 staff.", 30), &["f2"], false),
        ]);
        let c = check(&script, &d, Format::Short, DEFAULT_SPEED);
        assert!(c.errors.is_empty(), "{:?}", c.errors);
        assert!(c.notes.iter().any(|n| n.contains("open on the news")));
        assert!(c
            .notes
            .iter()
            .any(|n| n.to_lowercase().contains("here's the thing")));
    }

    #[test]
    fn a_blocker_cannot_be_resolved_while_the_script_still_says_it() {
        let script = short(vec![scene(
            "Binance lost everything to one installer.",
            &["f1"],
            false,
        )]);
        let f = Finding {
            review: "fairness".into(),
            claim: "lost everything to one installer".into(),
            severity: "blocker".into(),
            ..Default::default()
        };
        assert!(still_says(&script, &f));
        let review = Review {
            script: "h".into(),
            findings: vec![f],
            cold_reader: serde_json::Value::Null,
        };
        assert_eq!(review.open_blockers().len(), 1);
    }
}

// ----------------------------------------------------------------------------------- files

pub fn script_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(super::DIR).join("script.json")
}

pub fn review_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(super::DIR).join("review.json")
}

pub fn load_script(root: &std::path::Path) -> Option<Script> {
    crate::files::load_json(&script_path(root))
}

pub fn load_review(root: &std::path::Path) -> Option<Review> {
    crate::files::load_json(&review_path(root))
}

fn script_hash(script: &Script) -> String {
    use sha2::Digest;
    let bytes = serde_json::to_vec(script).unwrap_or_default();
    sha2::Sha256::digest(&bytes)
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Checks a script against the production's research and keeps it when nothing blocks it.
/// Returns the check either way.
pub fn write(root: &std::path::Path, script: Script, speed: f64) -> Result<Check, String> {
    let production = super::load(root).ok_or("The open project is not a faceless video")?;
    let dossier = super::evidence::load(root)
        .ok_or("Research the story first: there are no checked facts yet")?;
    let check = check(&script, &dossier, production.format, speed);
    if check.errors.is_empty() {
        crate::files::save_json(&script_path(root), &script)?;
        super::record_stage(root, "script")?;
    }
    Ok(check)
}

/// Saves the three reviews of the current script. The review stage is done only when no
/// blocker is open.
pub fn save_review(
    root: &std::path::Path,
    findings: Vec<Finding>,
    cold_reader: serde_json::Value,
) -> Result<Review, String> {
    let script = load_script(root).ok_or("Write the script first")?;
    for f in &findings {
        if !["attention", "coldReader", "fairness"].contains(&f.review.as_str()) {
            return Err(format!(
                "A finding's review is attention, coldReader or fairness, not {}",
                f.review
            ));
        }
        if f.claim.trim().is_empty() {
            return Err("Each finding quotes the script's exact words in `claim`".into());
        }
    }
    let review = Review {
        script: script_hash(&script),
        findings,
        cold_reader,
    };
    crate::files::save_json(&review_path(root), &review)?;
    if !review.open_blockers().is_empty() {
        super::clear_stage(root, "review")?;
    } else {
        super::record_stage(root, "review")?;
    }
    Ok(review)
}

/// Marks findings fixed. A blocker whose words the script still says stays open.
pub fn resolve(root: &std::path::Path, indexes: &[usize]) -> Result<Review, String> {
    let script = load_script(root).ok_or("Write the script first")?;
    let mut review = load_review(root).ok_or("There is no review yet")?;
    for &i in indexes {
        let finding = review
            .findings
            .get_mut(i)
            .ok_or_else(|| format!("No finding {i}"))?;
        if finding.severity == "blocker" && still_says(&script, finding) {
            return Err(format!(
                "The script still says \"{}\": change it first",
                finding.claim
            ));
        }
        finding.resolved = true;
    }
    review.script = script_hash(&script);
    crate::files::save_json(&review_path(root), &review)?;
    if !review.open_blockers().is_empty() {
        super::clear_stage(root, "review")?;
    } else {
        super::record_stage(root, "review")?;
    }
    Ok(review)
}
