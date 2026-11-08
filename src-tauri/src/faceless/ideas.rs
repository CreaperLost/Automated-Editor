//! Ranking what could make a good video (FCL-1), first ported from the explainer's `ideas.py`,
//! reworked on 8 Nov after reading real results:
//!
//! - Reports of one event are grouped by the words they share, weighted by how rare each word
//!   is that day ("Claude" and "AI" say little; "1,122", "$11b" and "chromebook" say a lot),
//!   compared title to title, never against a group's grown word list (that joined unrelated
//!   stories). A story marked `separate` is never grouped.
//! - Each idea has an explainable score: coverage (independent outlets; X and Reddit count once
//!   each), attention (points, likes, stars and their growth), fit (the profile's hooks),
//!   freshness, a primary-source bonus and a low-value penalty. Long and short weigh them
//!   differently.
//! - An editor's read (the agent's triage, or the user's) blends in, and the user can pin.
//! - Each search keeps the top of both rankings, so the list shows how stories moved.
//!
//! A shortlist for the user or their agent, not a judgment.
use super::demand::Demand;
use super::profile::Compiled;
use super::stories::{Editorial, Snapshot, Store, Topic};
use super::{Format, Made};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

const PRIMARY_BONUS: f64 = 0.15;
const LOW_VALUE_PENALTY: f64 = 0.25;
/// Weighted share of the smaller title two reports must share to be one event.
const SAME_STORY: f64 = 0.45;
/// When they share a number ("$11b", "1,122"), a lower share is enough.
const SAME_STORY_WITH_NUMBER: f64 = 0.35;
/// How much the editor's read weighs against the code's score.
const EDITORIAL_WEIGHT: f64 = 0.4;
/// How much a checked demand adds to the long score (shorts a little more).
const DEMAND_WEIGHT: f64 = 0.15;
/// Places kept in a snapshot.
const SNAPSHOT_PLACES: usize = 30;
/// The most stories per lane the agent sees.
const SHORTLIST_PER_LANE: usize = 15;
/// The least score a story needs to reach the agent.
const SHORTLIST_MIN_SCORE: f64 = 0.3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Lane {
    Stories,
    Repositories,
    Papers,
}

fn lane_of(title: &str) -> Lane {
    if title.starts_with("Viral on GitHub: ") {
        Lane::Repositories
    } else if title.starts_with("Paper: ") {
        Lane::Papers
    } else {
        Lane::Stories
    }
}

/// Why an idea scores what it does, each part from 0 to 1.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Parts {
    /// Independent outlets reporting it (X and Reddit count once each).
    pub coverage: f64,
    /// Points, likes, stars or comments, and how fast they grow.
    pub attention: f64,
    /// The channel's hooks in its headlines, as the profile weighs them.
    pub fit: f64,
    /// 1 when new, halving every two days (a day and a half for shorts).
    pub freshness: f64,
    /// A maker's own announcement among the reports.
    pub primary: bool,
    /// A format the channel ranks lower (how-to, review, stocks...).
    pub low_value: bool,
    /// The code's score before the editor's read.
    pub code: f64,
    /// The editor's read for this format (importance × fit), when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editorial: Option<f64>,
    /// Who watches and searches for it, from the last demand check.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demand: Option<f64>,
    /// How many videos on it already came out in the last three days.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saturation: Option<f64>,
}

/// One story, as many reports of it.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Idea {
    /// The lead report's story id.
    pub id: u64,
    pub title: String,
    pub lane: Lane,
    /// Its score in the ranking asked for (long or short).
    pub score: f64,
    pub long_score: f64,
    pub short_score: f64,
    /// The parts of `score`.
    pub parts: Parts,
    /// Its place in its lane, from 1.
    pub rank: usize,
    /// Its place after the previous search, when it was in the top then.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_rank: Option<usize>,
    /// Not in the previous search's top.
    pub new: bool,
    pub event_at: DateTime<Utc>,
    /// Independent outlets (news sites, and `x`, `reddit` once each).
    pub outlets: Vec<String>,
    pub hooks: Vec<String>,
    pub urls: Vec<String>,
    pub titles: Vec<String>,
    /// Every story id grouped into this idea.
    pub ids: Vec<u64>,
    pub attention: f64,
    /// Stars, likes or upvotes gained since first seen.
    pub growth: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editorial: Option<Editorial>,
    /// The last demand check (Treg): YouTube videos on it, Google searches, reels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demand: Option<Demand>,
    pub pinned: bool,
    /// Passes the shortlist: what the agent sees (enough score, and two independent sources,
    /// a maker's own post, rising attention, an editor's read or a pin; 15 per lane at most).
    pub shortlisted: bool,
    /// Why it is not on the shortlist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_out: Option<String>,
    /// A video on this story was already made.
    pub made: bool,
}

/// Money and big numbers written one way: "$11 billion", "$11B" and "11bn" are all "11b".
fn normalize_numbers(text: &str) -> String {
    let money = regex::Regex::new(
        r"(?i)\$?\s?(\d+(?:[.,]\d+)?)\s?(trillion|tn|t|billion|bn|b|million|mn|m)\b",
    )
    .unwrap();
    money
        .replace_all(text, |c: &regex::Captures| {
            let unit = match c[2].to_lowercase().chars().next() {
                Some('t') => "t",
                Some('b') => "b",
                _ => "m",
            };
            format!(" {}{unit} ", c[1].replace(',', ""))
        })
        .to_string()
}

pub fn tokens(title: &str, stopwords: &HashSet<String>) -> BTreeSet<String> {
    let word = regex::Regex::new(r"[a-z0-9$][a-z0-9$.'-]*").unwrap();
    let lower = normalize_numbers(&title.to_lowercase())
        .replace(['\u{2019}', '\u{2018}'], "'")
        .replace(['\u{201C}', '\u{201D}'], "\"");
    // "schools" and "school", "claims" and "claim" are one word; "near-suicide" also says
    // "suicide".
    let stem = |w: &str| -> String {
        if w.len() > 3
            && w.ends_with('s')
            && !w.ends_with("ss")
            && !w.chars().any(|c| c.is_ascii_digit())
        {
            w[..w.len() - 1].to_string()
        } else {
            w.to_string()
        }
    };
    let mut out = BTreeSet::new();
    for m in word.find_iter(&lower) {
        let w = m
            .as_str()
            .trim_matches(|c| c == '.' || c == '\'' || c == '-');
        let w = w.strip_suffix("'s").unwrap_or(w);
        if w.is_empty() || stopwords.contains(w) {
            continue;
        }
        out.insert(stem(w));
        if w.contains('-') && !w.chars().any(|c| c.is_ascii_digit()) {
            for part in w
                .split('-')
                .filter(|p| p.len() > 2 && !stopwords.contains(*p))
            {
                out.insert(stem(part));
            }
        }
    }
    out
}

/// How much each word says about which event a headline is about: rare words more.
struct Weights {
    df: HashMap<String, usize>,
    n: usize,
}

impl Weights {
    fn new<'a>(titles: impl Iterator<Item = &'a BTreeSet<String>>) -> Self {
        let mut df = HashMap::new();
        let mut n = 0;
        for words in titles {
            n += 1;
            for w in words {
                *df.entry(w.clone()).or_insert(0) += 1;
            }
        }
        Self { df, n }
    }

    fn of(&self, word: &str) -> f64 {
        let df = self.df.get(word).copied().unwrap_or(0) as f64;
        let idf = ((self.n as f64 + 1.0) / (df + 1.0)).ln() + 0.3;
        if word.chars().any(|c| c.is_ascii_digit()) && word.len() >= 2 {
            idf * 1.5
        } else {
            idf
        }
    }

    fn sum(&self, words: &BTreeSet<String>) -> f64 {
        words.iter().map(|w| self.of(w)).sum()
    }

    /// Whether two headlines report one event.
    fn same(&self, a: &BTreeSet<String>, b: &BTreeSet<String>) -> bool {
        let shared: BTreeSet<String> = a.intersection(b).cloned().collect();
        if shared.len() < 2 {
            return false;
        }
        let smaller = self.sum(a).min(self.sum(b)).max(f64::EPSILON);
        let share = self.sum(&shared) / smaller;
        let number = shared
            .iter()
            .any(|w| w.chars().any(|c| c.is_ascii_digit()) && w.len() >= 2);
        share >= SAME_STORY || (number && share >= SAME_STORY_WITH_NUMBER)
    }
}

fn hooks_in(profile: &Compiled, text: &str) -> Vec<String> {
    profile
        .hooks
        .iter()
        .filter(|(_, _, re)| re.is_match(text))
        .map(|(name, _, _)| name.clone())
        .collect()
}

/// The lead's hooks, plus any a quarter of the reports share: one stray word is not a hook.
fn group_hooks(profile: &Compiled, lead: &str, titles: &[&str]) -> Vec<String> {
    let lead_hooks = hooks_in(profile, lead);
    let all: Vec<String> = titles.iter().flat_map(|t| hooks_in(profile, t)).collect();
    let need = (titles.len() as f64 / 4.0).max(2.0);
    profile
        .hooks
        .iter()
        .map(|(name, _, _)| name.clone())
        .filter(|h| lead_hooks.contains(h) || all.iter().filter(|x| *x == h).count() as f64 >= need)
        .collect()
}

fn hook_weights(profile: &Compiled, hooks: &[String]) -> Vec<f64> {
    profile
        .hooks
        .iter()
        .filter(|(name, _, _)| hooks.contains(name))
        .map(|(_, w, _)| *w)
        .collect()
}

/// The parts that don't depend on the format.
struct Base {
    coverage: f64,
    attention: f64,
    long_fit: f64,
    short_fit: f64,
    age_days: f64,
    primary: bool,
    low_value: bool,
}

#[allow(clippy::too_many_arguments)]
fn base(
    profile: &Compiled,
    outlets: usize,
    attention: f64,
    growth: f64,
    hooks: &[String],
    age_days: f64,
    low_value: bool,
    primary: bool,
) -> Base {
    // About nine outlets is full coverage.
    let coverage = ((1.0 + outlets as f64).log2() / 3.3).min(1.0);
    let reach = ((1.0 + attention.max(0.0)).log10() / 3.0).min(1.0);
    let momentum = ((1.0 + growth.max(0.0)).log10() / 2.5).min(1.0);
    // A negative hook (the profile's "mainstream": business, courts, schools) pulls fit down,
    // as the channel's scope asks.
    let weights = hook_weights(profile, hooks);
    let sum: f64 = weights.iter().sum();
    let strongest = weights.iter().copied().fold(0.0, f64::max);
    let penalty: f64 = weights.iter().filter(|w| **w < 0.0).sum();
    // Two or three strong hooks fill it; one is about half.
    let spread = (sum / 2.4).clamp(0.0, 1.0);
    Base {
        coverage,
        attention: reach.max(momentum),
        long_fit: spread,
        // A short needs one sharp thing more than many.
        short_fit: (0.7 * strongest + 0.3 * spread + penalty).clamp(0.0, 1.0),
        age_days: age_days.max(0.0),
        primary,
        low_value,
    }
}

fn score(
    b: &Base,
    format: Format,
    editorial: Option<&Editorial>,
    demand: Option<&Demand>,
) -> (f64, Parts) {
    let (coverage_w, attention_w, fit_w, fit, half_life) = match format {
        Format::Long => (0.45, 0.15, 0.40, b.long_fit, 2.0),
        Format::Short => (0.20, 0.30, 0.50, b.short_fit, 1.5),
    };
    // Demand adds when checked; many recent videos already take a little off (long more,
    // since a long video competes with them for the same search).
    let (demand_w, saturation_w) = match format {
        Format::Long => (DEMAND_WEIGHT, 0.08),
        Format::Short => (DEMAND_WEIGHT + 0.05, 0.04),
    };
    let demand_part = demand
        .map(|d| demand_w * d.score - saturation_w * d.saturation)
        .unwrap_or(0.0);
    let freshness = 0.5f64.powf(b.age_days / half_life);
    let primary = if b.primary { PRIMARY_BONUS } else { 0.0 };
    let code =
        (coverage_w * b.coverage + attention_w * b.attention + fit_w * fit + primary + demand_part)
            * freshness
            - if b.low_value { LOW_VALUE_PENALTY } else { 0.0 };
    let editorial_part = editorial.map(|e| {
        let format_fit = match format {
            Format::Long => e.long_fit,
            Format::Short => e.short_fit,
        };
        (f64::from(e.importance.clamp(1, 5)) / 5.0) * (f64::from(format_fit.clamp(1, 5)) / 5.0)
    });
    let total = match editorial_part {
        Some(e) => (1.0 - EDITORIAL_WEIGHT) * code + EDITORIAL_WEIGHT * e * freshness,
        None => code,
    };
    let round = |x: f64| (x * 1000.0).round() / 1000.0;
    (
        round(total),
        Parts {
            coverage: round(b.coverage),
            attention: round(b.attention),
            fit: round(fit),
            freshness: round(freshness),
            primary: b.primary,
            low_value: b.low_value,
            code: round(code),
            editorial: editorial_part.map(round),
            demand: demand.map(|d| round(d.score)),
            saturation: demand.map(|d| round(d.saturation)),
        },
    )
}

/// Social sources count once each, so ten subreddits are not ten outlets.
fn outlet_of(source: &str) -> String {
    if source.starts_with("reddit") {
        "reddit".into()
    } else if source == "x" || source.starts_with("x:") {
        "x".into()
    } else {
        source.to_string()
    }
}

fn attention_of(topic: &Topic) -> (f64, f64) {
    let values: Vec<f64> = topic.signals.iter().filter_map(|s| s.value).collect();
    let max = values.iter().copied().fold(0.0, f64::max);
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    (max, if min.is_finite() { min } else { 0.0 })
}

/// Groups recent reports into events. Each group is its reports, newest first.
fn group<'a>(profile: &Compiled, rows: &[&'a Topic]) -> Vec<Vec<&'a Topic>> {
    let words: Vec<BTreeSet<String>> = rows
        .iter()
        .map(|t| tokens(&t.title, &profile.stopwords))
        .collect();
    let weights = Weights::new(
        words
            .iter()
            .zip(rows)
            .filter(|(_, t)| lane_of(&t.title) == Lane::Stories)
            .map(|(w, _)| w),
    );
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        if words[i].is_empty() {
            continue;
        }
        let joinable = lane_of(&row.title) == Lane::Stories && !row.separate;
        let home = joinable
            .then(|| {
                groups.iter().position(|members| {
                    let lead = rows[members[0]];
                    // The lead, or half the group: one shared word with one member would chain
                    // events together ("valuation" joined two companies' funding news).
                    let matching = members
                        .iter()
                        .filter(|&&m| weights.same(&words[i], &words[m]))
                        .count();
                    lane_of(&lead.title) == Lane::Stories
                        && !lead.separate
                        && (weights.same(&words[i], &words[members[0]])
                            || matching * 2 >= members.len())
                })
            })
            .flatten();
        match home {
            Some(g) => groups[g].push(i),
            None => groups.push(vec![i]),
        }
    }
    // An editor's joins: a report joined to another story moves into that story's group.
    for i in 0..rows.len() {
        let Some(target) = rows[i].joined_to else {
            continue;
        };
        let (Some(from), Some(to)) = (
            groups.iter().position(|g| g.contains(&i)),
            groups
                .iter()
                .position(|g| g.iter().any(|&m| rows[m].id == target)),
        ) else {
            continue;
        };
        if from != to {
            groups[from].retain(|&m| m != i);
            groups[to].push(i);
        }
    }
    groups
        .into_iter()
        .filter(|g| !g.is_empty())
        .map(|g| g.into_iter().map(|i| rows[i]).collect())
        .collect()
}

/// The top of the ranking in `format`, per lane, as of `now`.
#[allow(clippy::too_many_arguments)]
pub fn rank(
    profile: &Compiled,
    store: &Store,
    made: &[Made],
    now: DateTime<Utc>,
    days: i64,
    limit: usize,
    format: Format,
) -> Vec<Idea> {
    let since = now - Duration::days(days);
    let mut rows: Vec<&Topic> = store
        .topics
        .iter()
        .filter(|t| t.event_at >= since && t.status == "new")
        .collect();
    rows.sort_by(|a, b| b.event_at.cmp(&a.event_at));
    let syndicators: HashSet<String> = profile
        .profile
        .syndicators
        .iter()
        .map(|s| format!("feed:{s}"))
        .collect();
    let treg_sources = profile.profile.sources.treg.as_ref();
    let primary_sources: HashSet<String> = profile
        .profile
        .primary_sources
        .iter()
        .map(|s| format!("feed:{s}"))
        .collect();

    let mut ideas: Vec<Idea> = group(profile, &rows)
        .into_iter()
        .map(|members| {
            let sources: BTreeSet<String> = members
                .iter()
                .flat_map(|m| m.signals.iter().map(|s| s.source.clone()))
                .collect();
            let outlets: BTreeSet<String> = sources
                .iter()
                .filter(|s| !syndicators.contains(*s))
                .map(|s| outlet_of(s))
                .collect();
            let event_at = members.iter().map(|m| m.event_at).min().unwrap_or(now);
            let attention = members
                .iter()
                .map(|m| attention_of(m).0)
                .fold(0.0, f64::max);
            let growth = members
                .iter()
                .map(|m| {
                    let (max, min) = attention_of(m);
                    max - min
                })
                .fold(0.0, f64::max);
            let lead = members
                .iter()
                .max_by_key(|m| {
                    (
                        m.pinned,
                        m.editorial.is_some(),
                        hooks_in(profile, &m.title).len(),
                    )
                })
                .copied()
                .unwrap_or(members[0]);
            let titles: Vec<&str> = members.iter().map(|m| m.title.as_str()).collect();
            let hooks = group_hooks(profile, &lead.title, &titles);
            let age = (now - event_at).num_seconds() as f64 / 86_400.0;
            let low_value = profile
                .low_value
                .as_ref()
                .is_some_and(|re| re.is_match(&lead.title));
            let primary = sources.iter().any(|s| {
                primary_sources.contains(s)
                    || s.starts_with("sitemap:")
                    || s.strip_prefix("x:").is_some_and(|handle| {
                        treg_sources
                            .and_then(|t| t.group_of(handle))
                            .is_some_and(|g| g.primary)
                    })
            });
            let editorial = members.iter().find_map(|m| m.editorial.clone());
            let demand = members.iter().find_map(|m| m.demand.clone());
            let b = base(
                profile,
                outlets.len(),
                attention,
                growth,
                &hooks,
                age,
                low_value,
                primary,
            );
            let (long_score, long_parts) =
                score(&b, Format::Long, editorial.as_ref(), demand.as_ref());
            let (short_score, short_parts) =
                score(&b, Format::Short, editorial.as_ref(), demand.as_ref());
            let (score, parts) = match format {
                Format::Long => (long_score, long_parts),
                Format::Short => (short_score, short_parts),
            };
            let made = members
                .iter()
                .any(|m| super::already_made(made, &m.title, Some(&m.url)).is_some());
            Idea {
                id: lead.id,
                title: lead.title.clone(),
                lane: lane_of(&lead.title),
                score,
                long_score,
                short_score,
                parts,
                rank: 0,
                previous_rank: None,
                new: true,
                event_at,
                outlets: outlets.into_iter().collect(),
                hooks,
                urls: members.iter().map(|m| m.url.clone()).take(8).collect(),
                titles: titles.iter().take(8).map(|t| t.to_string()).collect(),
                ids: members.iter().map(|m| m.id).collect(),
                attention,
                growth,
                summary: lead.summary.clone(),
                editorial,
                demand,
                pinned: members.iter().any(|m| m.pinned),
                shortlisted: false,
                left_out: None,
                made,
            }
        })
        .collect();

    // Stories by score; repositories and papers by what they gained, then by fading attention
    // (a 300K-star repository may have gained none; the first day has no history). Pinned first.
    let faded = |i: &Idea| {
        i.attention
            * 0.5f64.powf(((now - i.event_at).num_seconds() as f64 / 86_400.0).max(0.0) / 2.0)
    };
    let previous = previous_snapshot(store);
    let mut kept = Vec::new();
    for lane in [Lane::Stories, Lane::Repositories, Lane::Papers] {
        let mut members: Vec<Idea> = ideas.extract_if(.., |i| i.lane == lane).collect();
        if lane == Lane::Stories {
            members.sort_by(|a, b| b.pinned.cmp(&a.pinned).then(b.score.total_cmp(&a.score)));
        } else {
            members.sort_by(|a, b| {
                b.pinned
                    .cmp(&a.pinned)
                    .then(b.growth.total_cmp(&a.growth))
                    .then(faded(b).total_cmp(&faded(a)))
            });
        }
        for (n, idea) in members.iter_mut().enumerate() {
            idea.rank = n + 1;
            if lane == Lane::Stories {
                if let Some(snapshot) = previous {
                    let places = match format {
                        Format::Long => &snapshot.long,
                        Format::Short => &snapshot.short,
                    };
                    idea.previous_rank = places
                        .iter()
                        .position(|ids| ids.iter().any(|id| idea.ids.contains(id)))
                        .map(|p| p + 1);
                    idea.new = idea.previous_rank.is_none();
                } else {
                    idea.new = false;
                }
            } else {
                idea.new = false;
            }
        }
        // The shortlist the agent sees: enough score and a reason to trust the story matters.
        let mut places = SHORTLIST_PER_LANE;
        for idea in members.iter_mut() {
            idea.left_out = shortlist_gate(idea, lane);
            if idea.left_out.is_none() {
                if places == 0 {
                    idea.left_out = Some(format!("below the top {SHORTLIST_PER_LANE}"));
                } else {
                    places -= 1;
                }
            }
            idea.shortlisted = idea.left_out.is_none();
        }
        kept.extend(members.into_iter().take(limit));
    }
    kept
}

/// Why an idea stays off the agent's shortlist, or `None`.
fn shortlist_gate(idea: &Idea, lane: Lane) -> Option<String> {
    if idea.pinned || idea.editorial.as_ref().is_some_and(|e| e.importance >= 3) {
        return None;
    }
    if idea.made {
        return Some("already made".into());
    }
    if lane != Lane::Stories {
        // Repositories and papers are judged by what they gained.
        return (idea.growth <= 0.0 && idea.attention < 500.0)
            .then(|| "not gaining stars or upvotes".to_string());
    }
    if idea.score < SHORTLIST_MIN_SCORE {
        return Some(format!("score under {SHORTLIST_MIN_SCORE}"));
    }
    let news_outlets = idea
        .outlets
        .iter()
        .filter(|o| *o != "x" && *o != "reddit")
        .count();
    let backed = idea.outlets.len() >= 2
        || idea.parts.primary
        || idea.growth >= 100.0
        || idea.attention >= 1000.0;
    if !backed {
        return Some(if news_outlets == 0 {
            "one social post, no reporting yet".into()
        } else {
            "one source, no maker's post, no rising attention".into()
        });
    }
    None
}

/// The ranking to compare with: the last one kept before the latest search.
fn previous_snapshot(store: &Store) -> Option<&Snapshot> {
    let latest = store.last_discover_at?;
    store.rankings.iter().rev().find(|s| s.at < latest)
}

/// The top of both story rankings now, to keep after a search.
pub fn snapshot(profile: &Compiled, store: &Store, made: &[Made], now: DateTime<Utc>) -> Snapshot {
    let places = |format| {
        rank(profile, store, made, now, 3, SNAPSHOT_PLACES, format)
            .into_iter()
            .filter(|i| i.lane == Lane::Stories)
            .map(|i| i.ids)
            .collect()
    };
    Snapshot {
        at: now,
        long: places(Format::Long),
        short: places(Format::Short),
    }
}

/// A readable table of the top `n` in a format: for reviewing the ranking.
pub fn review_table(ideas: &[Idea], n: usize) -> String {
    let mut out = String::from(
        "| # | Move | Score | Story | Outlets | Coverage | Attention | Fit | Fresh | Editor | Demand |\n|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for i in ideas.iter().filter(|i| i.lane == Lane::Stories).take(n) {
        let mv = match (i.previous_rank, i.new) {
            (Some(p), _) if p > i.rank => format!("▲{}", p - i.rank),
            (Some(p), _) if p < i.rank => format!("▼{}", i.rank - p),
            (Some(_), _) => "=".into(),
            (None, true) => "new".into(),
            (None, false) => String::new(),
        };
        out.push_str(&format!(
            "| {}{} | {mv} | {:.2} | {}{} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {} | {} |\n",
            i.rank,
            if i.pinned { " 📌" } else { "" },
            i.score,
            i.title.replace('|', "/"),
            if i.made { " (made)" } else { "" },
            i.outlets.len(),
            i.parts.coverage,
            i.parts.attention,
            i.parts.fit,
            i.parts.freshness,
            i.parts
                .editorial
                .map_or("-".to_string(), |e| format!("{e:.2}")),
            i.demand.as_ref().map_or("-".to_string(), |d| format!(
                "{:.2}: {} recent videos, {:.0} views/h{}",
                d.score,
                d.recent_videos,
                d.views_per_hour,
                d.subject.as_ref().map_or(String::new(), |k| format!(
                    ", \"{}\" {:.0}/mo",
                    k.keyword, k.volume
                ))
            ))
        ));
    }
    out
}

/// What the `ideas` grouping did with a set of titles, for tests and tuning.
pub fn groups_of(profile: &Compiled, store: &Store) -> BTreeMap<u64, Vec<u64>> {
    let rows: Vec<&Topic> = store.topics.iter().collect();
    group(profile, &rows)
        .into_iter()
        .map(|g| (g[0].id, g.iter().map(|t| t.id).collect()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faceless::stories::Finding;

    fn profile() -> Compiled {
        super::super::profile::Compiled::new(
            serde_json::from_str(super::super::profile::BUILT_IN).unwrap(),
        )
        .unwrap()
    }

    fn report(source: &str, key: &str, title: &str, hours_ago: i64, now: DateTime<Utc>) -> Finding {
        Finding {
            source: source.into(),
            key: key.into(),
            title: title.into(),
            url: format!("https://e.com/{key}"),
            kind: "news".into(),
            event_at: now - Duration::hours(hours_ago),
            metric: None,
            summary: None,
        }
    }

    /// Real headlines from 7 Nov 2026 that the first grouping got wrong.
    fn real_day(now: DateTime<Utc>) -> Store {
        let mut store = Store::default();
        let rows = [
            ("feed:Verge", "a1", "Claude Code's \"Yes, and Don't Ask Again\" Option Can Leak Passwords and API Keys to GitHub"),
            ("feed:THN", "a2", "Claude Code's “Don't Ask Again” File Exposes Passwords and Keys in 1,122 Public Copies on GitHub"),
            ("feed:Wired", "a3", "Claude Code's “Don't Ask Again” File Exposes Passwords and Keys in 1,122 Public Copies on GitHub"),
            ("feed:Blog", "b1", "Claude Code cloud launches automatic dependency and API key setup"),
            ("feed:Reuters", "c1", "OpenAI closes $11 billion funding at $1.25 trillion valuation - report"),
            ("feed:CNBC", "c2", "OpenAI reportedly raises $11B in new funding at $1.25T valuation"),
            ("feed:FT", "c3", "OpenAI Hits $1.25T Valuation on $11B Round"),
            ("feed:Court", "d1", "Lawsuit: ChatGPT helped wife plan husband’s poisoning"),
            ("feed:Edu", "e1", "Lawsuit claims AI on Google’s school Chromebook helped plan teen’s near-suicide"),
            ("feed:Edu2", "e2", "Lawsuit claims Google’s AI for schools led girl into suicide plan"),
            ("feed:Anthropic", "f1", "Anthropic IPO valuation talks drop to $1.3T"),
            ("feed:Bloomberg", "f2", "Anthropic IPO Valuation Talks Fall to $1.2T After AI Stock Selloff"),
            ("feed:Ars", "g1", "Claude Code adds background agents that run while you sleep"),
        ];
        let findings: Vec<Finding> = rows
            .iter()
            .enumerate()
            .map(|(n, (s, k, t))| report(s, k, t, n as i64, now))
            .collect();
        store.add(&findings, now);
        store
    }

    fn group_titles(store: &Store) -> Vec<Vec<String>> {
        let p = profile();
        let key = |id: u64| {
            store
                .topics
                .iter()
                .find(|t| t.id == id)
                .unwrap()
                .key
                .clone()
        };
        groups_of(&p, store)
            .into_values()
            .map(|ids| {
                let mut keys: Vec<String> = ids.into_iter().map(key).collect();
                keys.sort();
                keys
            })
            .collect()
    }

    #[test]
    fn one_event_groups_and_different_events_stay_apart() {
        let now = Utc::now();
        let groups = group_titles(&real_day(now));
        let has = |keys: &[&str]| groups.iter().any(|g| g == keys);
        assert!(has(&["a1", "a2", "a3"]), "{groups:?}");
        assert!(has(&["b1"]), "{groups:?}");
        assert!(has(&["c1", "c2", "c3"]), "{groups:?}");
        assert!(has(&["d1"]), "{groups:?}");
        assert!(has(&["e1", "e2"]), "{groups:?}");
        assert!(has(&["f1", "f2"]), "{groups:?}");
        assert!(has(&["g1"]), "{groups:?}");
    }

    #[test]
    fn a_story_marked_separate_is_never_grouped() {
        let now = Utc::now();
        let mut store = real_day(now);
        let id = store.topics.iter().find(|t| t.key == "a3").unwrap().id;
        store.topic_mut(id).unwrap().separate = true;
        let groups = group_titles(&store);
        assert!(groups.iter().any(|g| g == &["a3"]), "{groups:?}");
        // And an editor can join two reports the words kept apart.
        let d1 = store.topics.iter().find(|t| t.key == "d1").unwrap().id;
        let g1 = store.topics.iter().find(|t| t.key == "g1").unwrap().id;
        store.topic_mut(g1).unwrap().joined_to = Some(d1);
        let groups = group_titles(&store);
        assert!(groups.iter().any(|g| g == &["d1", "g1"]), "{groups:?}");
    }

    #[test]
    fn money_is_written_one_way() {
        let stop = HashSet::new();
        let a = tokens("OpenAI raises $11 billion at $1.25 trillion", &stop);
        let b = tokens("OpenAI raises $11B at $1.25T", &stop);
        assert!(a.contains("11b") && a.contains("1.25t"), "{a:?}");
        assert_eq!(a, b);
    }

    #[test]
    fn coverage_counts_x_and_reddit_once_and_republishers_never() {
        let now = Utc::now();
        let p = profile();
        let mut store = Store::default();
        store.add(
            &[
                report(
                    "feed:The Verge",
                    "a",
                    "Anthropic leaks Claude source code in npm package",
                    3,
                    now,
                ),
                report(
                    "feed:Wired",
                    "b",
                    "Claude source code leak exposes Anthropic npm package",
                    2,
                    now,
                ),
                report(
                    "feed:Yahoo Finance",
                    "c",
                    "Anthropic Claude source code npm leak",
                    1,
                    now,
                ),
                report(
                    "reddit:ClaudeAI",
                    "d",
                    "Anthropic leaked the Claude source code in its npm package",
                    1,
                    now,
                ),
                report(
                    "reddit:ClaudeCode",
                    "e",
                    "Claude source code leak in the npm package",
                    1,
                    now,
                ),
                report(
                    "x",
                    "f",
                    "Anthropic just leaked Claude Code source in npm",
                    1,
                    now,
                ),
            ],
            now,
        );
        let ideas = rank(&p, &store, &[], now, 3, 30, Format::Long);
        let lead = &ideas[0];
        assert_eq!(lead.ids.len(), 6, "{ideas:#?}");
        assert_eq!(
            lead.outlets,
            ["feed:The Verge", "feed:Wired", "reddit", "x"]
        );
        assert!(lead.parts.coverage > 0.6);
    }

    #[test]
    fn long_rewards_coverage_and_short_rewards_one_sharp_hook() {
        let p = profile();
        let wide = base(&p, 8, 0.0, 0.0, &["money".into()], 0.0, false, false);
        let sharp = base(&p, 1, 2000.0, 0.0, &["exploit".into()], 0.0, false, false);
        let (wide_long, _) = score(&wide, Format::Long, None, None);
        let (sharp_long, _) = score(&sharp, Format::Long, None, None);
        let (wide_short, _) = score(&wide, Format::Short, None, None);
        let (sharp_short, _) = score(&sharp, Format::Short, None, None);
        assert!(sharp_short > wide_short, "{sharp_short} {wide_short}");
        assert!(wide_long - sharp_long > wide_short - sharp_short);
        // Older halves; a low-value format drops.
        let old = base(&p, 8, 0.0, 0.0, &["money".into()], 2.0, false, false);
        assert!((score(&old, Format::Long, None, None).0 - wide_long / 2.0).abs() < 0.002);
        let low = base(&p, 8, 0.0, 0.0, &["money".into()], 0.0, true, false);
        assert!(score(&low, Format::Long, None, None).0 < wide_long);
    }

    #[test]
    fn the_editor_moves_a_story_and_a_pin_puts_it_first() {
        let now = Utc::now();
        let p = profile();
        let mut store = real_day(now);
        let before = rank(&p, &store, &[], now, 3, 30, Format::Long);
        let last = before
            .iter()
            .filter(|i| i.lane == Lane::Stories)
            .last()
            .unwrap()
            .clone();
        let id = last.ids[0];
        store.topic_mut(id).unwrap().editorial = Some(Editorial {
            importance: 5,
            long_fit: 5,
            short_fit: 2,
            angle: "a".into(),
            reason: "r".into(),
            by: "agent".into(),
        });
        let after = rank(&p, &store, &[], now, 3, 30, Format::Long);
        let moved = after.iter().find(|i| i.ids.contains(&id)).unwrap();
        assert!(moved.rank < last.rank, "{} {}", moved.rank, last.rank);
        assert_eq!(moved.parts.editorial, Some(1.0));
        store.topic_mut(id).unwrap().pinned = true;
        let pinned = rank(&p, &store, &[], now, 3, 30, Format::Short);
        assert!(pinned[0].ids.contains(&id) && pinned[0].pinned);
    }

    #[test]
    fn movement_compares_with_the_search_before() {
        let now = Utc::now();
        let p = profile();
        let mut store = real_day(now - Duration::hours(2));
        store.last_discover_at = Some(now - Duration::hours(2));
        let first = snapshot(&p, &store, &[], now - Duration::hours(2));
        store.keep_snapshot(first);
        // A new search: a new story arrives and is reported widely.
        let fresh: Vec<Finding> = ["feed:A", "feed:B", "feed:C", "feed:D", "feed:E"]
            .iter()
            .enumerate()
            .map(|(n, s)| {
                report(
                    s,
                    &format!("z{n}"),
                    "OpenAI ships GPT-7 with a jailbreak found in hours",
                    0,
                    now,
                )
            })
            .collect();
        store.add(&fresh, now);
        store.last_discover_at = Some(now);
        let second = snapshot(&p, &store, &[], now);
        store.keep_snapshot(second);
        let ideas = rank(&p, &store, &[], now, 3, 30, Format::Long);
        let top = &ideas[0];
        assert!(top.title.contains("GPT-7") && top.new && top.previous_rank.is_none());
        let moved = ideas.iter().find(|i| i.previous_rank.is_some()).unwrap();
        assert!(!moved.new);
        let table = review_table(&ideas, 10);
        assert!(table.contains("| 1 | new |"), "{table}");
    }

    #[test]
    fn set_aside_stories_leave_and_made_ones_are_marked() {
        let now = Utc::now();
        let p = profile();
        let mut store = real_day(now);
        let lead = rank(&p, &store, &[], now, 3, 30, Format::Long)[0].clone();
        let made = vec![Made {
            topic: lead.title.clone(),
            link: None,
            format: Format::Long,
            project: "p".into(),
            kind: "test".into(),
            youtube: None,
            at: now.to_rfc3339(),
        }];
        assert!(rank(&p, &store, &made, now, 3, 30, Format::Long)[0].made);
        for id in &lead.ids {
            store.set_status(*id, "set-aside").unwrap();
        }
        let after = rank(&p, &store, &[], now, 3, 30, Format::Long);
        assert!(after.iter().all(|i| !i.ids.contains(&lead.id)));
    }
}

#[cfg(test)]
mod live {
    /// The user's real stories: `cargo test --lib live_ranking -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_ranking() {
        let config = crate::transcript::settings::config_dir();
        let profile = super::super::profile::load(&config).unwrap();
        let store = super::Store::load(&config);
        for format in [super::Format::Long, super::Format::Short] {
            let ideas = super::rank(&profile, &store, &[], chrono::Utc::now(), 3, 15, format);
            println!("== {format:?}\n{}", super::review_table(&ideas, 12));
            for i in ideas
                .iter()
                .filter(|i| i.lane == super::Lane::Stories)
                .take(12)
            {
                for t in i.titles.iter().skip(1) {
                    println!("  #{} + {t}", i.rank);
                }
            }
        }
    }
}
