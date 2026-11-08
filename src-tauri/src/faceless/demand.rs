//! Demand for a story, from Treg (8 Nov): who is already making videos about it and how many
//! watch them (YouTube), how many people search for its subject on Google and whether that is
//! growing, and for shorts, the reels about it (Instagram). Checked on request for the top of
//! a ranking, a few cents for ten stories, and kept with the story.
//!
//! Three numbers come out, each from 0 to 1:
//! - `audience`: views per hour of recent videos on this story (people watch it), or on the
//!   subject when nobody has covered the story yet;
//! - `interest`: monthly Google searches for the subject, scaled by their trend;
//! - `saturation`: how many videos on this story came out in the last three days.
//!
//! `score` blends audience and interest; the ranking adds it and takes off a little for
//! saturation. A rough signal, not a forecast.
use super::ideas::{tokens, Idea};
use super::profile::Compiled;
use super::treg::{self, Keyword, Reel, Video};
use super::Format;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Videos kept per story, for the review.
const KEEP_VIDEOS: usize = 5;
/// About a thousand views an hour on a video is as strong as audience gets.
const FULL_VIEWS_PER_HOUR: f64 = 3.0;
/// About a million monthly searches is as strong as interest gets.
const FULL_SEARCHES: f64 = 6.0;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Demand {
    pub checked_at: DateTime<Utc>,
    /// What YouTube was searched for.
    pub query: String,
    /// Recent videos on this story, most watched first.
    #[serde(default)]
    pub videos: Vec<Video>,
    /// Videos on this story from the last three days.
    pub recent_videos: usize,
    /// The best views per hour among them (or the subject's, when none cover it).
    pub views_per_hour: f64,
    /// The subject searched for on Google, with its monthly searches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<Keyword>,
    /// Instagram reels and TikTok videos about it (short ranking only), most viewed first.
    #[serde(default)]
    pub reels: Vec<Reel>,
    /// Videos on it from the watched YouTube channels, against what each usually gets.
    #[serde(default)]
    pub watched: Vec<WatchedHit>,
    pub audience: f64,
    pub interest: f64,
    pub saturation: f64,
    /// 0 to 1: audience and interest together.
    pub score: f64,
    pub cost_micro: u64,
}

/// The headline as a search: quotes, publisher suffixes and punctuation out, ten words at most.
pub fn query_of(title: &str) -> String {
    let title = title.split(" - ").next().unwrap_or(title);
    let title = title.split(" | ").next().unwrap_or(title);
    let cleaned: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '$' || c == '.' || c == '-' {
                c
            } else {
                ' '
            }
        })
        .collect();
    cleaned
        .split_whitespace()
        .take(10)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The subject a headline is about: the longest of the profile's subject terms in it
/// ("claude code" before "claude").
pub fn subject_of(profile: &Compiled, title: &str) -> Option<String> {
    // Each alternative on its own: one regex stops at the first that matches ("claude").
    profile
        .profile
        .subject_terms
        .split('|')
        .filter_map(|term| {
            regex::RegexBuilder::new(&format!(r"\b(?:{term})\b"))
                .case_insensitive(true)
                .build()
                .ok()?
                .find(title)
                .map(|m| m.as_str().to_lowercase())
        })
        .max_by_key(|s| s.len())
        .filter(|s| s.len() >= 3 && s != "ai" && s != "a.i.")
}

/// Whether a video is about the same story: beyond the subject's own words ("claude code"),
/// it shares at least two of the headline's words.
fn covers(
    story: &BTreeSet<String>,
    subject: &BTreeSet<String>,
    video_title: &str,
    profile: &Compiled,
) -> bool {
    let words = tokens(video_title, &profile.stopwords);
    story
        .intersection(&words)
        .filter(|w| !subject.contains(*w))
        .count()
        >= 2
}

fn scale_log(value: f64, full: f64) -> f64 {
    ((1.0 + value.max(0.0)).log10() / full).clamp(0.0, 1.0)
}

/// Works out the numbers from what the searches found.
/// A watched channel's video on the story.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchedHit {
    pub channel: String,
    pub title: String,
    pub views: f64,
    /// What the channel's recent videos usually get.
    pub usual: f64,
    /// `views / usual`: 2 or more means the story pulls more than the channel's usual.
    pub ratio: f64,
    pub url: String,
}

/// `usual`: per watched channel (lowercase name), the median views of its recent videos.
#[allow(clippy::too_many_arguments)]
pub fn measure(
    profile: &Compiled,
    title: &str,
    query: String,
    videos: Vec<Video>,
    subject: Option<Keyword>,
    mut reels: Vec<Reel>,
    usual: &BTreeMap<String, f64>,
    now: DateTime<Utc>,
    cost_micro: u64,
) -> Demand {
    let story = tokens(title, &profile.stopwords);
    let subject_words = subject_of(profile, title)
        .map(|s| tokens(&s, &profile.stopwords))
        .unwrap_or_default();
    let per_hour = |v: &Video| v.views / v.hours.unwrap_or(24.0 * 30.0).max(1.0);
    let mut covering: Vec<Video> = videos
        .iter()
        .filter(|v| v.hours.is_none_or(|h| h <= 24.0 * 30.0))
        .filter(|v| covers(&story, &subject_words, &v.title, profile))
        .cloned()
        .collect();
    covering.sort_by(|a, b| b.views.total_cmp(&a.views));
    let recent_videos = covering
        .iter()
        .filter(|v| v.hours.is_some_and(|h| h <= 72.0))
        .count();
    // Nobody has covered the story yet: what the subject's videos get is the audience waiting.
    let views_per_hour = if covering.is_empty() {
        videos.iter().map(per_hour).fold(0.0, f64::max) * 0.5
    } else {
        covering.iter().map(per_hour).fold(0.0, f64::max)
    };
    // A watched channel's video on it, against that channel's usual.
    let mut watched: Vec<WatchedHit> = covering
        .iter()
        .filter_map(|v| {
            let usual = *usual.get(&v.channel.to_lowercase())?;
            (usual > 0.0).then(|| WatchedHit {
                channel: v.channel.clone(),
                title: v.title.clone(),
                views: v.views,
                usual,
                ratio: (v.views / usual * 100.0).round() / 100.0,
                url: v.url.clone(),
            })
        })
        .collect();
    watched.sort_by(|a, b| b.ratio.total_cmp(&a.ratio));
    let best_ratio = watched.first().map_or(0.0, |w| w.ratio);
    let mut audience = scale_log(views_per_hour, FULL_VIEWS_PER_HOUR);
    if best_ratio >= 2.0 {
        audience = audience.max(0.9);
    } else if best_ratio >= 1.0 {
        audience = audience.max(0.7);
    }
    reels.sort_by(|a, b| b.views.total_cmp(&a.views));
    let interest = subject.as_ref().map_or(0.0, |k| {
        (scale_log(k.volume, FULL_SEARCHES) * k.trend.unwrap_or(1.0).clamp(0.7, 1.3)).min(1.0)
    });
    let reel_signal = reels
        .iter()
        .filter(|r| r.hours.is_none_or(|h| h <= 24.0 * 14.0))
        .map(|r| r.views.max(r.likes * 20.0))
        .fold(0.0, f64::max);
    let reel_part = scale_log(reel_signal, 7.0);
    let score = if reels.is_empty() {
        0.6 * audience + 0.4 * interest
    } else {
        0.5 * audience + 0.3 * interest + 0.2 * reel_part
    };
    let round = |x: f64| (x * 1000.0).round() / 1000.0;
    Demand {
        checked_at: now,
        query,
        videos: if covering.is_empty() {
            Vec::new()
        } else {
            covering.into_iter().take(KEEP_VIDEOS).collect()
        },
        recent_videos,
        views_per_hour: views_per_hour.round(),
        subject,
        reels: reels.into_iter().take(4).collect(),
        watched,
        audience: round(audience),
        interest: round(interest),
        saturation: round((recent_videos as f64 / 6.0).min(1.0)),
        score: round(score),
        cost_micro,
    }
}

/// Refreshes what each watched YouTube channel usually gets (its recent videos' views), at
/// most once a week per channel. Returns per channel (lowercase name) its median views, and
/// the cost.
pub fn refresh_channels(
    profile: &Compiled,
    treg: &dyn treg::Call,
    store: &mut super::stories::Store,
    now: DateTime<Utc>,
) -> (BTreeMap<String, f64>, u64) {
    let channels = profile
        .profile
        .sources
        .treg
        .as_ref()
        .map(|t| t.youtube_channels.clone())
        .unwrap_or_default();
    let stale: Vec<&super::profile::Channel> = channels
        .iter()
        .filter(|c| {
            store
                .accounts
                .get(&format!("yt:{}", c.handle.to_lowercase()))
                .and_then(|s| s.checked_at)
                .is_none_or(|at| now - at > chrono::Duration::days(7))
        })
        .collect();
    type Fetched = (String, Result<(Vec<f64>, u64), String>);
    let fetched: Vec<Fetched> = std::thread::scope(|scope| {
        let handles: Vec<_> = stale
            .iter()
            .map(|c| scope.spawn(move || (c.handle.clone(), treg::channel_views(treg, &c.handle))))
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    let mut cost = 0;
    for (handle, result) in fetched {
        if let Ok((views, c)) = result {
            cost += c;
            let stats = store
                .accounts
                .entry(format!("yt:{}", handle.to_lowercase()))
                .or_default();
            stats.recent.clear();
            for v in views {
                stats.remember(v);
            }
            stats.checked_at = Some(now);
        }
    }
    let usual = channels
        .iter()
        .filter_map(|c| {
            let stats = store
                .accounts
                .get(&format!("yt:{}", c.handle.to_lowercase()))?;
            let mut views = stats.recent.clone();
            if views.is_empty() {
                return None;
            }
            views.sort_by(f64::total_cmp);
            Some((c.name.to_lowercase(), views[views.len() / 2]))
        })
        .collect();
    (usual, cost)
}

/// Checks demand for these stories through Treg, in parallel: a YouTube search each (and
/// Instagram reels and TikTok for shorts), and one Google volume call for all their subjects.
/// `usual` is what each watched channel usually gets. Returns each idea's lead id with its
/// demand, problems, and the total cost.
pub fn check(
    profile: &Compiled,
    treg: &dyn treg::Call,
    ideas: &[Idea],
    format: Format,
    usual: &BTreeMap<String, f64>,
    now: DateTime<Utc>,
) -> (Vec<(u64, Demand)>, Vec<String>, u64) {
    let subjects: Vec<String> = ideas
        .iter()
        .filter_map(|i| subject_of(profile, &i.title))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut problems = Vec::new();
    let (volumes, volume_cost) = match treg::keyword_volume(treg, &subjects) {
        Ok(v) => v,
        Err(e) => {
            problems.push(format!("Google volume: {e}"));
            (Vec::new(), 0)
        }
    };
    let volumes: BTreeMap<String, Keyword> = volumes
        .into_iter()
        .map(|k| (k.keyword.to_lowercase(), k))
        .collect();
    let searched: Vec<(u64, Result<Demand, String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = ideas
            .iter()
            .map(|idea| {
                let volumes = &volumes;
                scope.spawn(move || {
                    let query = query_of(&idea.title);
                    let (videos, mut cost) = treg::youtube_search(treg, &query, now)?;
                    let mut reels = Vec::new();
                    if format == Format::Short {
                        if let Ok((found, c)) = treg::instagram_reels(treg, &query, now) {
                            cost += c;
                            reels.extend(found);
                        }
                        if let Ok((found, c)) = treg::tiktok_search(treg, &query, now) {
                            cost += c;
                            reels.extend(found);
                        }
                    }
                    let subject =
                        subject_of(profile, &idea.title).and_then(|s| volumes.get(&s).cloned());
                    Ok(measure(
                        profile,
                        &idea.title,
                        query,
                        videos,
                        subject,
                        reels,
                        usual,
                        now,
                        cost,
                    ))
                })
            })
            .collect();
        handles
            .into_iter()
            .zip(ideas)
            .map(|(h, idea)| (idea.id, h.join().unwrap_or_else(|_| Err("crashed".into()))))
            .collect()
    });
    let mut total = volume_cost;
    let mut out = Vec::new();
    for (id, result) in searched {
        match result {
            Ok(d) => {
                total += d.cost_micro;
                out.push((id, d));
            }
            Err(e) => problems.push(format!("story {id}: {e}")),
        }
    }
    (out, problems, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Compiled {
        Compiled::new(serde_json::from_str(super::super::profile::BUILT_IN).unwrap()).unwrap()
    }

    fn video(title: &str, views: f64, hours: f64) -> Video {
        Video {
            id: title.len().to_string(),
            title: title.into(),
            channel: "c".into(),
            views,
            hours: Some(hours),
            url: "u".into(),
        }
    }

    #[test]
    fn queries_and_subjects_come_from_the_headline() {
        let p = profile();
        assert_eq!(
            query_of("Claude Code's \"Don't Ask Again\" File Exposes Keys - The Verge"),
            "Claude Code s Don t Ask Again File Exposes Keys"
        );
        assert_eq!(
            subject_of(&p, "Claude Code's Don't Ask Again file leaks keys").as_deref(),
            Some("claude code")
        );
        assert_eq!(subject_of(&p, "Bakery wins prize"), None);
    }

    #[test]
    fn covering_videos_set_audience_and_saturation() {
        let p = profile();
        let title = "Claude Code's Don't Ask Again file leaks API keys on GitHub";
        let videos = vec![
            video(
                "Claude Code Don't Ask Again leaked API keys explained",
                20_000.0,
                10.0,
            ),
            video(
                "Claude Code leaks your API keys (Don't Ask Again)",
                500.0,
                30.0,
            ),
            video("Best pasta recipe", 1_000_000.0, 5.0),
        ];
        let subject = Some(Keyword {
            keyword: "claude code".into(),
            volume: 368_000.0,
            cpc: None,
            difficulty: None,
            trend: Some(1.6),
        });
        let mut usual = BTreeMap::new();
        usual.insert("c".to_string(), 5_000.0);
        let d = measure(
            &p,
            title,
            "q".into(),
            videos,
            subject,
            Vec::new(),
            &usual,
            Utc::now(),
            1000,
        );
        // A watched channel ("c") at 4x its usual on the top video.
        assert_eq!(d.watched.first().map(|w| w.ratio), Some(4.0));
        assert_eq!(d.videos.len(), 2, "{:?}", d.videos);
        assert_eq!(d.recent_videos, 2);
        assert_eq!(d.views_per_hour, 2000.0);
        assert!(
            d.audience > 0.9 && d.interest > 0.9 && d.saturation > 0.3,
            "{d:?}"
        );
        // Nobody has covered it yet: open, with the subject's audience at half weight.
        let open = measure(
            &p,
            title,
            "q".into(),
            vec![video("Claude Code tutorial for beginners", 50_000.0, 100.0)],
            None,
            Vec::new(),
            &BTreeMap::new(),
            Utc::now(),
            0,
        );
        assert_eq!((open.recent_videos, open.saturation), (0, 0.0));
        assert!(open.videos.is_empty() && open.audience > 0.0);
    }

    #[test]
    fn counts_and_ages_read_every_way() {
        let now = Utc::now();
        assert_eq!(treg::count(&serde_json::json!("1.2K views")), Some(1200.0));
        assert_eq!(treg::count(&serde_json::json!("3,405")), Some(3405.0));
        assert_eq!(treg::count(&serde_json::json!(7)), Some(7.0));
        assert_eq!(
            treg::hours_ago(&serde_json::json!("27 minutes ago"), now),
            Some(0.45)
        );
        assert_eq!(
            treg::hours_ago(&serde_json::json!("Streamed 2 days ago"), now),
            Some(48.0)
        );
        let ts = now.timestamp() - 7200;
        assert_eq!(treg::hours_ago(&serde_json::json!(ts), now), Some(2.0));
    }
}
