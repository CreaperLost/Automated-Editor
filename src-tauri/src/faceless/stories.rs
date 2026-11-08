//! The stories found on the web (FCL-1), kept a week: `<config>/AeroEdits/faceless/stories.json`.
//! Each story keeps every time a source reported it (a signal), so wide coverage and growing
//! attention can be measured.
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FILE: &str = "stories.json";
/// Stories older than this are dropped: the news has moved on.
pub const MAX_AGE_DAYS: i64 = 7;
/// Signals kept per story; enough for momentum, small on disk.
const MAX_SIGNALS: usize = 60;

/// One report of a story by one source, before it is stored.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    /// `hackernews`, `feed:<publisher>`, `sitemap:<site>`, ...
    pub source: String,
    /// The same key means the same item (a URL, a repository, a paper).
    pub key: String,
    pub title: String,
    pub url: String,
    /// `news` or `release`.
    pub kind: String,
    pub event_at: DateTime<Utc>,
    pub metric: Option<(String, f64)>,
    pub summary: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Signal {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    pub at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Topic {
    pub id: u64,
    pub key: String,
    pub title: String,
    pub url: String,
    pub kind: String,
    pub event_at: DateTime<Utc>,
    pub first_seen_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// `new`, or `set-aside` when the user or agent rejected it.
    #[serde(default = "new_status")]
    pub status: String,
    #[serde(default)]
    pub signals: Vec<Signal>,
    /// The agent's (or user's) editorial read of the story.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editorial: Option<Editorial>,
    /// Pinned by the user: shown first, whatever its score.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
    /// Never grouped with other reports (it was grouped wrongly).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub separate: bool,
    /// Grouped with this story's reports (they are one event the words didn't show).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joined_to: Option<u64>,
    /// Who watches and searches for it, from the last demand check (Treg).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub demand: Option<super::demand::Demand>,
}

/// An editor's read of a story, which the ranking blends with the code's score.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Editorial {
    /// How much it matters to the channel's viewers, 1 to 5.
    pub importance: u8,
    /// How well it carries a 5-8 minute video (a mechanism, real shifts), 1 to 5.
    pub long_fit: u8,
    /// How well it lands as one surprising idea in under a minute, 1 to 5.
    pub short_fit: u8,
    /// The angle that would make it worth watching, in a line.
    #[serde(default)]
    pub angle: String,
    /// Why, in a line.
    #[serde(default)]
    pub reason: String,
    /// Who rated it: the agent's name or "you".
    #[serde(default)]
    pub by: String,
}

/// The top of the ranking after a search, to show how stories moved.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub at: DateTime<Utc>,
    /// Per place, the story ids grouped there (long ranking).
    pub long: Vec<Vec<u64>>,
    /// The same for the short ranking.
    pub short: Vec<Vec<u64>>,
}

fn new_status() -> String {
    "new".into()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Store {
    #[serde(default)]
    pub topics: Vec<Topic>,
    #[serde(default)]
    pub next_id: u64,
    /// Per sitemap site: every URL seen, so a new one is a new post.
    #[serde(default)]
    pub sitemap_seen: BTreeMap<String, Vec<String>>,
    /// Per funnel: when it last fetched something.
    #[serde(default)]
    pub last_success: BTreeMap<String, DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_discover_at: Option<DateTime<Utc>>,
    /// The rankings after recent searches, newest last (a few kept).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rankings: Vec<Snapshot>,
    /// Per tracked account (`x:@handle`, `yt:@handle`): what it usually gets and how often it
    /// matters, for the filters and the watchlist's hit rate.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub accounts: BTreeMap<String, AccountStats>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountStats {
    /// Likes (X) or views (YouTube) of its recent posts, newest last.
    #[serde(default)]
    pub recent: Vec<f64>,
    /// Posts of it that passed the filters.
    #[serde(default)]
    pub kept: u32,
    /// Times one of its posts was in a top 10 after a search.
    #[serde(default)]
    pub top10: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked_at: Option<DateTime<Utc>>,
}

impl AccountStats {
    pub fn remember(&mut self, value: f64) {
        self.recent.push(value);
        let excess = self.recent.len().saturating_sub(30);
        self.recent.drain(..excess);
    }

    /// Its usual: the median of what it got lately, once there are five.
    pub fn usual(&self) -> Option<f64> {
        if self.recent.len() < 5 {
            return None;
        }
        let mut sorted = self.recent.clone();
        sorted.sort_by(f64::total_cmp);
        Some(sorted[sorted.len() / 2])
    }
}

pub fn path(config: &Path) -> PathBuf {
    config.join(super::DIR).join(FILE)
}

impl Store {
    pub fn load(config: &Path) -> Self {
        crate::files::load_json(&path(config)).unwrap_or_default()
    }

    pub fn save(&self, config: &Path) -> Result<(), String> {
        crate::files::save_json(&path(config), self)
    }

    /// Adds each finding as a story (new key) and a signal; returns how many stories are new.
    pub fn add(&mut self, findings: &[Finding], now: DateTime<Utc>) -> usize {
        let mut new = 0;
        for f in findings {
            let index = match self.topics.iter().position(|t| t.key == f.key) {
                Some(index) => index,
                None => {
                    self.next_id += 1;
                    self.topics.push(Topic {
                        id: self.next_id,
                        key: f.key.clone(),
                        title: f.title.clone(),
                        url: f.url.clone(),
                        kind: f.kind.clone(),
                        event_at: f.event_at,
                        first_seen_at: now,
                        summary: f.summary.clone(),
                        status: new_status(),
                        signals: Vec::new(),
                        editorial: None,
                        pinned: false,
                        separate: false,
                        joined_to: None,
                        demand: None,
                    });
                    new += 1;
                    self.topics.len() - 1
                }
            };
            let topic = &mut self.topics[index];
            topic.signals.push(Signal {
                source: f.source.clone(),
                metric: f.metric.as_ref().map(|(name, _)| name.clone()),
                value: f.metric.as_ref().map(|(_, value)| *value),
                at: now,
            });
            if topic.signals.len() > MAX_SIGNALS {
                // Keep the first (where growth is measured from) and the latest.
                let excess = topic.signals.len() - MAX_SIGNALS;
                topic.signals.drain(1..1 + excess);
            }
        }
        new
    }

    /// Drops stories older than a week.
    pub fn prune(&mut self, now: DateTime<Utc>) {
        let oldest = now - Duration::days(MAX_AGE_DAYS);
        self.topics.retain(|t| t.event_at >= oldest);
    }

    pub fn set_status(&mut self, id: u64, status: &str) -> Result<(), String> {
        let topic = self
            .topics
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("No story {id}"))?;
        topic.status = status.into();
        Ok(())
    }

    pub fn topic_mut(&mut self, id: u64) -> Result<&mut Topic, String> {
        self.topics
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("No story {id}"))
    }

    /// Keeps a ranking to compare the next one with (the last 10).
    pub fn keep_snapshot(&mut self, snapshot: Snapshot) {
        self.rankings.push(snapshot);
        let excess = self.rankings.len().saturating_sub(10);
        self.rankings.drain(..excess);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(key: &str, source: &str, value: f64, at: DateTime<Utc>) -> Finding {
        Finding {
            source: source.into(),
            key: key.into(),
            title: format!("Story {key}"),
            url: format!("https://example.com/{key}"),
            kind: "news".into(),
            event_at: at,
            metric: Some(("points".into(), value)),
            summary: None,
        }
    }

    #[test]
    fn findings_become_stories_with_signals_and_old_ones_go() {
        let now = Utc::now();
        let mut store = Store::default();
        let new = store.add(
            &[
                finding("a", "hackernews", 10.0, now),
                finding("a", "feed:Verge", 0.0, now),
                finding("b", "hackernews", 5.0, now - Duration::days(9)),
            ],
            now,
        );
        assert_eq!(new, 2);
        assert_eq!(store.topics[0].signals.len(), 2);
        assert_eq!(store.add(&[finding("a", "hackernews", 30.0, now)], now), 0);
        assert_eq!(store.topics[0].signals.len(), 3);
        store.prune(now);
        assert_eq!(store.topics.len(), 1);
        for _ in 0..100 {
            store.add(&[finding("a", "hackernews", 1.0, now)], now);
        }
        assert_eq!(store.topics[0].signals.len(), MAX_SIGNALS);
        assert_eq!(store.topics[0].signals[0].value, Some(10.0));
        let dir = tempfile::tempdir().unwrap();
        store.save(dir.path()).unwrap();
        assert_eq!(Store::load(dir.path()).topics, store.topics);
    }
}
