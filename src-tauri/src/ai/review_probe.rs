//! Measures the filler and retake review against hand labels, the way `apply: true` would cut:
//! the rules' suggestions plus each engine's spans, merged by `edit::suggestions`. Engines:
//! `rules` (no model), `local` (the built-in model over the candidate windows) and `cloud`
//! (the OpenRouter or OpenAI review in settings, over every word).
//!
//! A case is a JSON file: `{"transcript": "<path to transcripts/*.json>", "language": "en",
//! "words": 1200, "gold": [{"from": 12, "to": 12, "type": "filler", "note": "so"}]}`.
//! `words` keeps only the first kept words (what was labelled); `gold` indexes are the review's
//! own, as the dump numbers them. The labelled cases and results are in
//! `docs/plans/review-eval`.
//!
//! `AEROEDITS_REVIEW_CASE=<case.json> AEROEDITS_REVIEW_DUMP=1 cargo test --release --lib
//! review_probe -- --ignored --nocapture` writes `<case>.words.txt` to label from. Without
//! `AEROEDITS_REVIEW_DUMP` it runs `AEROEDITS_REVIEW_ENGINES` (default `rules,local,cloud`;
//! `local` needs `--features local-llm`), prints a line per engine and writes
//! `<case>.<engine>.json` with every cut, miss and page timing. Both go to
//! `AEROEDITS_REVIEW_OUT` (default `%TEMP%\aeroedits-review-eval`).
//! `AEROEDITS_REVIEW_CLOUD_MODEL` overrides the settings' cloud model.
use super::client::JsonModel;
use super::{candidates, fillers, local};
use crate::timeline::{SourceInterval, TimelineMapper};
use crate::transcript::edit::{self, is_punctuation};
use crate::transcript::{Language, SuggestionSource, Transcript, TranscriptWord, WordKind};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(serde::Deserialize)]
struct Case {
    transcript: PathBuf,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    words: Option<usize>,
    #[serde(default)]
    gold: Vec<Gold>,
}

#[derive(serde::Deserialize)]
struct Gold {
    from: usize,
    to: usize,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    note: String,
    /// A judgement call: cutting it is not wrong and keeping it is not a miss.
    #[serde(default)]
    optional: bool,
    /// Other spans that do the same job ("volume volume": either one).
    #[serde(default)]
    alt: Vec<(usize, usize)>,
}

impl Gold {
    fn ranges(&self) -> impl Iterator<Item = std::ops::RangeInclusive<usize>> + '_ {
        std::iter::once(self.from..=self.to).chain(self.alt.iter().map(|&(a, b)| a..=b))
    }
}

/// The transcript as the review sees it: every spoken word kept (the project's own cuts are
/// ignored), no earlier review or rejection, cut to the labelled words.
fn load(case: &Case) -> (Transcript, TimelineMapper, Language) {
    let bytes = std::fs::read(&case.transcript).unwrap();
    let mut t: Transcript = serde_json::from_slice(&bytes).unwrap();
    t.ai_suggestions.clear();
    t.ai_model = None;
    t.dismissed_suggestions.clear();
    if let Some(limit) = case.words {
        let mut n = 0;
        t.words.retain(|w| {
            if w.kind != WordKind::Word {
                return n < limit;
            }
            n += 1;
            n <= limit
        });
    }
    let end = t.words.iter().map(|w| w.source_end_us).max().unwrap_or(0) + 1_000_000;
    let mapper = TimelineMapper::new(vec![SourceInterval::new("all".into(), 0, end)]);
    let language = case
        .language
        .as_deref()
        .and_then(Language::from_code)
        .unwrap_or_else(|| Language::of(&t));
    (t, mapper, language)
}

fn kept(t: &Transcript) -> Vec<&TranscriptWord> {
    t.words
        .iter()
        .filter(|w| w.kind == WordKind::Word)
        .collect()
}

/// The rules' spans as review indexes, which the candidate pass leaves alone.
fn settled(
    t: &Transcript,
    mapper: &TimelineMapper,
    words: &[&TranscriptWord],
) -> Vec<(usize, usize)> {
    let index: HashMap<&str, usize> = words
        .iter()
        .enumerate()
        .map(|(i, w)| (w.id.as_str(), i))
        .collect();
    edit::suggestions(t, mapper)
        .iter()
        .filter(|s| s.source == SuggestionSource::Rules)
        .filter_map(|s| {
            Some((
                *index.get(s.word_ids.first()?.as_str())?,
                *index.get(s.word_ids.last()?.as_str())?,
            ))
        })
        .collect()
}

#[cfg(feature = "local-llm")]
fn devices() -> String {
    llama_cpp_2::list_llama_ggml_backend_devices()
        .iter()
        .map(|d| {
            format!(
                "{} {} ({} MB)",
                d.backend,
                d.description,
                d.memory_total >> 20
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(not(feature = "local-llm"))]
fn devices() -> String {
    "not built with local-llm".into()
}

/// Times each page a model answers and records how much it read and wrote.
struct Timed<'a> {
    inner: &'a mut dyn JsonModel,
    pages: Vec<Value>,
}

impl JsonModel for Timed<'_> {
    fn complete_json(&mut self, system: &str, user: &str) -> Result<Value, String> {
        let started = Instant::now();
        let reply = self.inner.complete_json(system, user);
        self.pages.push(json!({
            "s": started.elapsed().as_secs_f64(),
            "promptChars": system.chars().count() + user.chars().count(),
            "answerChars": reply.as_ref().map(|r| r.to_string().chars().count()).unwrap_or(0),
            "cuts": reply.as_ref().ok().and_then(|r| r["cuts"].as_array().map(Vec::len)),
        }));
        reply
    }

    fn describe(&self) -> String {
        self.inner.describe()
    }
}

/// Runs one engine; returns its spans and what to record about the run.
fn review(
    engine: &str,
    t: &Transcript,
    mapper: &TimelineMapper,
    language: Language,
) -> (Vec<crate::transcript::AiSpan>, Value) {
    let words = kept(t);
    match engine {
        "rules" => (Vec::new(), json!({})),
        "local" => {
            let plan = candidates::plan(&words, &settled(t, mapper, &words));
            let started = Instant::now();
            let mut model = local::LocalModel::load(&local::default_dir()).unwrap();
            let load_s = started.elapsed().as_secs_f64();
            let mut timed = Timed {
                inner: &mut model,
                pages: Vec::new(),
            };
            let started = Instant::now();
            let spans = fillers::detect_candidates(
                &mut timed,
                language,
                &words,
                &plan,
                &mut |_| {},
                &|| true,
            )
            .unwrap();
            let review_s = started.elapsed().as_secs_f64();
            let info = json!({
                "model": timed.describe(),
                "device": devices(),
                "pages": plan.pages.len(),
                "windows": plan.windows.len(),
                "shownWords": plan.shown_words(),
                "loadS": load_s,
                "reviewS": review_s,
                "perPage": timed.pages,
                "shown": plan.windows.iter().map(|w| (w.words.start, w.words.end - 1)).collect::<Vec<_>>(),
                "flags": plan.windows.iter().flat_map(|w| &w.flags)
                    .map(|c| json!([c.from, c.to, format!("{:?}", c.flag), c.note])).collect::<Vec<_>>(),
            });
            (spans, info)
        }
        "cloud" => {
            let dir = crate::transcript::settings::config_dir();
            let settings = super::load_settings(&dir);
            let (key, _) =
                crate::secrets::get(&dir, settings.provider.key_spec()).expect("no AI key set");
            let model =
                std::env::var("AEROEDITS_REVIEW_CLOUD_MODEL").unwrap_or_else(|_| settings.model());
            let mut client = super::client::ChatClient::new(settings.provider, key, model).unwrap();
            let mut timed = Timed {
                inner: &mut client,
                pages: Vec::new(),
            };
            let started = Instant::now();
            let spans =
                fillers::detect(&mut timed, language, &words, &mut |_| {}, &|| true).unwrap();
            let info = json!({
                "model": timed.describe(),
                "perPage": timed.pages,
                "device": "remote",
                "pages": fillers::chunks(words.len()).len(),
                "shownWords": words.len(),
                "loadS": 0.0,
                "reviewS": started.elapsed().as_secs_f64(),
            });
            (spans, info)
        }
        other => panic!("unknown engine {other}"),
    }
}

/// Scores the cuts `apply: true` would make against `gold`. A labelled span (not optional) is
/// missed unless most of its words, or of one of its alternatives, are cut; a cut is
/// wrong when under half of its words are labelled (optional and alternatives included).
fn score(t: &Transcript, mapper: &TimelineMapper, all_gold: &[Gold]) -> Value {
    let words = kept(t);
    let index: HashMap<&str, usize> = words
        .iter()
        .enumerate()
        .map(|(i, w)| (w.id.as_str(), i))
        .collect();
    let spoken = |i: &usize| !is_punctuation(&words[*i].text);
    let gold_words: HashSet<usize> = all_gold
        .iter()
        .flat_map(Gold::ranges)
        .flatten()
        .filter(spoken)
        .collect();
    let gold: Vec<&Gold> = all_gold.iter().filter(|g| !g.optional).collect();
    let must_words: HashSet<usize> = gold
        .iter()
        .flat_map(|g| g.from..=g.to)
        .filter(spoken)
        .collect();
    let cuts: Vec<_> = edit::suggestions(t, mapper)
        .into_iter()
        .filter(|s| !s.dismissed)
        .collect();
    let mut cut_words: HashSet<usize> = HashSet::new();
    let mut wrong = Vec::new();
    let mut listed = Vec::new();
    for s in &cuts {
        let at: Vec<usize> = s
            .word_ids
            .iter()
            .filter_map(|id| index.get(id.as_str()).copied())
            .collect();
        cut_words.extend(at.iter().copied());
        let hit = at.iter().filter(|i| gold_words.contains(i)).count();
        let entry = json!({
            "from": at.first(), "to": at.last(), "kind": s.kind, "text": s.text,
            "source": s.source, "why": s.reason,
        });
        if hit * 2 < at.len().max(1) {
            wrong.push(entry.clone());
        }
        listed.push(entry);
    }
    let mut missed = Vec::new();
    for g in &gold {
        let covered = g.ranges().any(|r| {
            let all: Vec<usize> = r.filter(spoken).collect();
            let hit = all.iter().filter(|i| cut_words.contains(i)).count();
            hit * 2 > all.len()
        });
        if !covered {
            let text: Vec<&str> = (g.from..=g.to).map(|i| words[i].text.as_str()).collect();
            missed.push(json!({ "from": g.from, "to": g.to, "type": g.kind, "text": text.join(" "), "note": g.note }));
        }
    }
    let kind_count = |kind: &str, list: &[Value]| list.iter().filter(|v| v["type"] == kind).count();
    let gold_fillers = gold.iter().filter(|g| g.kind == "filler").count();
    let gold_retakes = gold.len() - gold_fillers;
    let wrong_words = cut_words.iter().filter(|i| !gold_words.contains(i)).count();
    let missed_words = must_words.iter().filter(|i| !cut_words.contains(i)).count();
    json!({
        "goldSpans": gold.len(), "goldFillers": gold_fillers, "goldRetakes": gold_retakes,
        "goldWords": must_words.len(), "optionalSpans": all_gold.len() - gold.len(),
        "cuts": cuts.len(), "cutWords": cut_words.len(),
        "missed": missed.len(),
        "missedFillers": kind_count("filler", &missed),
        "missedRetakes": kind_count("retake", &missed),
        "wrong": wrong.len(),
        "wrongWords": wrong_words, "missedWords": missed_words,
        "missedList": missed, "wrongList": wrong, "cutList": listed,
    })
}

/// `<case>.<suffix>` in `AEROEDITS_REVIEW_OUT`, else in a temporary folder.
fn sibling(case: &Path, suffix: &str) -> PathBuf {
    let stem = case.file_stem().unwrap().to_string_lossy();
    let dir = std::env::var_os("AEROEDITS_REVIEW_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("aeroedits-review-eval"));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{stem}.{suffix}"))
}

#[test]
#[ignore]
fn review_probe() {
    let path =
        PathBuf::from(std::env::var("AEROEDITS_REVIEW_CASE").expect("set AEROEDITS_REVIEW_CASE"));
    let mut case: Case = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    // A relative transcript sits beside the case.
    if case.transcript.is_relative() {
        case.transcript = path.parent().unwrap().join(&case.transcript);
    }
    let (mut t, mapper, language) = load(&case);
    let words = kept(&t);
    if std::env::var_os("AEROEDITS_REVIEW_DUMP").is_some() {
        let mut text = fillers::chunk_prompt(&words, 0..words.len());
        // One line per sentence end or long pause, so it can be read and labelled.
        text = text.replace(" (pause", "\n(pause");
        std::fs::write(sibling(&path, "words.txt"), text).unwrap();
        println!("{} words, {language:?}", words.len());
        return;
    }
    let engines =
        std::env::var("AEROEDITS_REVIEW_ENGINES").unwrap_or_else(|_| "rules,local,cloud".into());
    for engine in engines.split(',').map(str::trim) {
        let (spans, info) = review(engine, &t, &mapper, language);
        let ai_spans = spans.len();
        t.ai_suggestions = spans;
        t.ai_model = info["model"].as_str().map(String::from);
        let mut score = score(&t, &mapper, &case.gold);
        // For the candidate review: was a miss flagged, or at least shown to the model?
        let overlaps = |list: &Value, from: u64, to: u64, a: usize, b: usize| {
            list.as_array().into_iter().flatten().any(|r| {
                let (x, y) = (r[a].as_u64().unwrap_or(0), r[b].as_u64().unwrap_or(0));
                x <= to && from <= y
            })
        };
        if let Some(missed) = score["missedList"].as_array_mut() {
            for m in missed.iter_mut() {
                let (from, to) = (m["from"].as_u64().unwrap(), m["to"].as_u64().unwrap());
                if info.get("shown").is_some() {
                    m["shown"] = json!(overlaps(&info["shown"], from, to, 0, 1));
                    m["flagged"] = json!(overlaps(&info["flags"], from, to, 0, 1));
                }
            }
        }
        println!(
            "{engine:6} {:?} words={} gold={} (fillers {}, retakes {}) | missed={} (fillers {}, retakes {}) wrong={} \
             | words missed={} wrong={} | ai spans={ai_spans} | load {:.1}s review {:.1}s | {}",
            language,
            words_len(&t),
            score["goldSpans"], score["goldFillers"], score["goldRetakes"],
            score["missed"], score["missedFillers"], score["missedRetakes"], score["wrong"],
            score["missedWords"], score["wrongWords"],
            info["loadS"].as_f64().unwrap_or(0.0), info["reviewS"].as_f64().unwrap_or(0.0),
            info["device"].as_str().unwrap_or(""),
        );
        let out = json!({ "engine": engine, "language": language, "run": info, "score": score });
        std::fs::write(
            sibling(&path, &format!("{engine}.json")),
            serde_json::to_string_pretty(&out).unwrap(),
        )
        .unwrap();
        t.ai_suggestions.clear();
        t.ai_model = None;
    }
}

fn words_len(t: &Transcript) -> usize {
    kept(t).len()
}
