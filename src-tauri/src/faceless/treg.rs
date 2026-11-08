//! Treg (https://treg.to), the research provider the explainer planned (its roadmap step 7):
//! X and Reddit for story search, web search and page reading for research. One HTTP call per
//! catalog endpoint (`POST /call/<id>`), billed per answer at the provider's price; a failed
//! call costs nothing. Every call carries a cost ceiling.
//!
//! The key lives in the system keychain (`TREG_API_KEY` overrides it), never in a project.
use serde_json::{json, Value};
use std::time::Duration;

pub const BASE_URL: &str = "https://treg.to/call";
pub const KEYS_URL: &str = "https://treg.to/app#orgs/keys";
/// The most one call may cost, in dollars (Treg refuses dearer routes without charging).
pub const MAX_COST_USD: &str = "0.05";

pub const KEY: crate::secrets::KeySpec = crate::secrets::KeySpec {
    env: "TREG_API_KEY",
    name: "treg-api-key",
};

/// An answer: Treg's `output`, what it cost and which provider gave it.
#[derive(Clone, Debug, Default)]
pub struct Reply {
    pub output: Value,
    pub cost_micro: u64,
    pub served_by: String,
}

/// Something that calls a Treg endpoint; tests answer from fixtures.
pub trait Call: Sync {
    fn call(&self, endpoint: &str, body: &Value) -> Result<Reply, String>;
}

pub struct Web {
    client: reqwest::blocking::Client,
    key: String,
}

impl Web {
    pub fn new(key: String) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(90))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client, key })
    }

    /// The key in the keychain (or `TREG_API_KEY`), if one is set.
    pub fn from_keychain() -> Option<Self> {
        let config = crate::transcript::settings::config_dir();
        let (key, _) = crate::secrets::get(&config, KEY)?;
        Self::new(key).ok()
    }
}

fn refusal(status: u16, body: &Value) -> String {
    let code = body["detail"]["error"]
        .as_str()
        .or(body["error"].as_str())
        .unwrap_or_default();
    match (status, code) {
        (401 | 403, _) if code != "capability_pinned" => {
            "Treg refused the key: make a new one at treg.to and paste it in File → Accounts".into()
        }
        (402, "route_max_cost") => format!(
            "Every Treg provider for this costs more than ${MAX_COST_USD} a call; nothing was charged"
        ),
        (402, _) => "The Treg balance is used up: top it up at treg.to (nothing was charged)".into(),
        (429, _) => format!(
            "Treg is busy ({}): try again in {} s",
            if code.is_empty() { "rate limit" } else { code },
            body["detail"]["retry_after_s"]
                .as_u64()
                .or(body["retry_after_s"].as_u64())
                .unwrap_or(30)
        ),
        (502, _) => "No Treg provider found an answer".into(),
        _ => format!(
            "Treg answered HTTP {status}{}",
            if code.is_empty() {
                String::new()
            } else {
                format!(" ({code})")
            }
        ),
    }
}

impl Call for Web {
    fn call(&self, endpoint: &str, body: &Value) -> Result<Reply, String> {
        let response = self
            .client
            .post(format!("{BASE_URL}/{endpoint}"))
            .header("X-Treg-Token", &self.key)
            .header("X-Treg-Route-Max-Cost", MAX_COST_USD)
            .json(body)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    "Treg did not answer in time".to_string()
                } else {
                    "Could not reach Treg".to_string()
                }
            })?;
        let status = response.status().as_u16();
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        let cost_micro = header("x-treg-cost-micro")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let served_by = header("x-treg-served-by").unwrap_or_else(|| endpoint.to_string());
        let reply: Value = response.json().unwrap_or(Value::Null);
        if status >= 400 {
            return Err(refusal(status, &reply));
        }
        // Routed endpoints answer {output, raw, _treg}; direct ones the provider's own shape.
        let output = reply.get("output").cloned().unwrap_or(reply);
        Ok(Reply {
            output,
            cost_micro,
            served_by,
        })
    }
}

/// Markdown made plain enough for quote matching: links keep their words, emphasis marks and
/// heading marks go.
pub fn markdown_text(markdown: &str) -> String {
    let link = regex::Regex::new(r"!?\[([^\]]*)\]\([^)]*\)").unwrap();
    let text = link.replace_all(markdown, "$1");
    text.lines()
        .map(|line| {
            line.trim_start_matches(['#', '>', ' '])
                .replace("**", "")
                .replace("__", "")
                .replace('`', "")
        })
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// One web search result.
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Searches the open web through Treg (`treg.web.search`).
pub fn web_search(
    treg: &dyn Call,
    query: &str,
    limit: usize,
) -> Result<(Vec<SearchResult>, u64), String> {
    let reply = treg.call(
        "treg.web.search",
        &json!({ "q": query, "limit": limit.clamp(1, 20) }),
    )?;
    let rows = reply.output["results"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let results = rows
        .iter()
        .filter_map(|r| {
            let url = r["url"].as_str().or(r["link"].as_str())?.to_string();
            Some(SearchResult {
                title: r["title"].as_str().unwrap_or_default().to_string(),
                snippet: r["snippet"]
                    .as_str()
                    .or(r["description"].as_str())
                    .or(r["content"].as_str())
                    .unwrap_or_default()
                    .chars()
                    .take(400)
                    .collect(),
                url,
            })
        })
        .take(limit)
        .collect();
    Ok((results, reply.cost_micro))
}

/// A page's text read by Treg (`treg.web.extract`): PDFs, Google News links and pages built by
/// scripts. Returns the text, the address it ended at, and what it cost.
pub fn extract(treg: &dyn Call, url: &str) -> Result<(String, Option<String>, u64), String> {
    let reply = treg.call("treg.web.extract", &json!({ "url": url }))?;
    let page = reply.output["pages"]
        .as_array()
        .and_then(|p| p.first())
        .cloned()
        .unwrap_or(reply.output.clone());
    let raw = page["markdown"]
        .as_str()
        .or(page["text"].as_str())
        .or(page["content"].as_str())
        .or(page["raw_content"].as_str())
        .unwrap_or_default();
    if raw.trim().is_empty() {
        return Err("Treg read no text on the page".into());
    }
    let final_url = page["final_url"]
        .as_str()
        .or(page["url"].as_str())
        .filter(|u| *u != url)
        .map(str::to_string);
    Ok((markdown_text(raw), final_url, reply.cost_micro))
}

/// Dollars from Treg's micro-dollars, for messages.
pub fn dollars(micro: u64) -> String {
    format!("${:.4}", micro as f64 / 1_000_000.0)
}

/// A count written any way providers write it: 1234, "1,234", "1.2K views", "3M".
pub fn count(value: &Value) -> Option<f64> {
    if let Some(n) = value.as_f64() {
        return Some(n);
    }
    let text = value.as_str()?.trim().to_lowercase().replace(',', "");
    let number: String = text
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let n: f64 = number.parse().ok()?;
    let rest = text[number.len()..].trim_start();
    Some(match rest.chars().next() {
        Some('k') => n * 1e3,
        Some('m') => n * 1e6,
        Some('b') => n * 1e9,
        _ => n,
    })
}

/// Hours since a "27 minutes ago" / "3 days ago" / "Streamed 2 weeks ago" / ISO time.
pub fn hours_ago(value: &Value, now: chrono::DateTime<chrono::Utc>) -> Option<f64> {
    if let Some(secs) = value.as_i64().filter(|s| *s > 1_000_000_000) {
        return Some((now.timestamp() - secs) as f64 / 3600.0);
    }
    let text = value.as_str()?.trim().to_lowercase();
    if let Ok(at) = chrono::DateTime::parse_from_rfc3339(&text.replace('z', "+00:00")) {
        return Some((now - at.with_timezone(&chrono::Utc)).num_minutes() as f64 / 60.0);
    }
    let re =
        regex::Regex::new(r"(\d+)\s*(second|minute|hour|day|week|month|year)s?\s+ago").unwrap();
    let c = re.captures(&text)?;
    let n: f64 = c[1].parse().ok()?;
    Some(
        n * match &c[2] {
            "second" => 1.0 / 3600.0,
            "minute" => 1.0 / 60.0,
            "hour" => 1.0,
            "day" => 24.0,
            "week" => 168.0,
            "month" => 730.0,
            _ => 8760.0,
        },
    )
}

/// A YouTube video found by a search.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub views: f64,
    /// Hours since it was published, when the provider says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<f64>,
    pub url: String,
}

/// Searches YouTube through Treg (`treg.youtube.search.videos`).
pub fn youtube_search(
    treg: &dyn Call,
    query: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(Vec<Video>, u64), String> {
    let reply = treg.call("treg.youtube.search.videos", &json!({ "q": query }))?;
    let rows = reply.output["videos"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let videos = rows
        .iter()
        .filter_map(|v| {
            let id = v["video_id"]
                .as_str()
                .or(v["videoId"].as_str())
                .or(v["id"].as_str())?
                .to_string();
            Some(Video {
                title: v["title"].as_str().unwrap_or_default().to_string(),
                channel: v["author"]
                    .as_str()
                    .or(v["channel"].as_str())
                    .or(v["channelTitle"].as_str())
                    .or(v["channel"]["name"].as_str())
                    .unwrap_or_default()
                    .to_string(),
                views: count(&v["number_of_views"])
                    .or_else(|| count(&v["views"]))
                    .or_else(|| count(&v["viewCount"]))
                    .or_else(|| count(&v["view_count"]))
                    .unwrap_or(0.0),
                hours: hours_ago(&v["published_time"], now)
                    .or_else(|| hours_ago(&v["publishedTime"], now))
                    .or_else(|| hours_ago(&v["published"], now))
                    .or_else(|| hours_ago(&v["publishedAt"], now))
                    .or_else(|| hours_ago(&v["publish_date"], now)),
                url: format!("https://www.youtube.com/watch?v={id}"),
                id,
            })
        })
        .collect();
    Ok((videos, reply.cost_micro))
}

/// An Instagram reel found by a search.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reel {
    pub caption: String,
    pub views: f64,
    pub likes: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<f64>,
    pub url: String,
    /// Who posted it.
    #[serde(default)]
    pub author: String,
    /// Their followers, when the platform says.
    #[serde(default)]
    pub author_followers: f64,
}

/// Searches Instagram reels through Treg (`treg.instagram.search.reels`).
pub fn instagram_reels(
    treg: &dyn Call,
    query: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(Vec<Reel>, u64), String> {
    let reply = treg.call("treg.instagram.search.reels", &json!({ "q": query }))?;
    let rows = reply.output["reels"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let reels = rows
        .iter()
        .map(|r| Reel {
            caption: r["caption"]
                .as_str()
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect(),
            views: count(&r["views"])
                .or_else(|| count(&r["play_count"]))
                .unwrap_or(0.0),
            likes: count(&r["likes"])
                .or_else(|| count(&r["like_count"]))
                .unwrap_or(0.0),
            hours: hours_ago(&r["createdUtc"], now).or_else(|| hours_ago(&r["taken_at"], now)),
            author: r["username"]
                .as_str()
                .or(r["owner"]["username"].as_str())
                .unwrap_or_default()
                .to_string(),
            author_followers: count(&r["followers"])
                .or_else(|| count(&r["owner"]["follower_count"]))
                .unwrap_or(0.0),
            url: r["url"].as_str().map(str::to_string).unwrap_or_else(|| {
                format!(
                    "https://www.instagram.com/reel/{}/",
                    r["shortcode"].as_str().unwrap_or_default()
                )
            }),
        })
        .collect();
    Ok((reels, reply.cost_micro))
}

/// Searches TikTok videos through Treg (`treg.tiktok.search.videos`), as reels (caption, plays,
/// likes, age, link).
pub fn tiktok_search(
    treg: &dyn Call,
    query: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(Vec<Reel>, u64), String> {
    let reply = treg.call(
        "treg.tiktok.search.videos",
        &json!({ "q": query, "limit": 10 }),
    )?;
    let rows = reply.output["videos"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let videos = rows
        .iter()
        .map(|row| {
            // Some providers wrap each video in `aweme_info`.
            let v = if row["aweme_info"].is_object() {
                &row["aweme_info"]
            } else {
                row
            };
            let stats = if v["statistics"].is_object() {
                &v["statistics"]
            } else {
                &v["stats"]
            };
            Reel {
                caption: v["desc"]
                    .as_str()
                    .or(v["title"].as_str())
                    .or(v["caption"].as_str())
                    .unwrap_or_default()
                    .chars()
                    .take(200)
                    .collect(),
                views: count(&stats["play_count"])
                    .or_else(|| count(&stats["playCount"]))
                    .or_else(|| count(&v["play_count"]))
                    .or_else(|| count(&v["views"]))
                    .unwrap_or(0.0),
                likes: count(&stats["digg_count"])
                    .or_else(|| count(&stats["diggCount"]))
                    .or_else(|| count(&v["likes"]))
                    .unwrap_or(0.0),
                hours: hours_ago(&v["create_time"], now)
                    .or_else(|| hours_ago(&v["createTime"], now)),
                author: v["author"]["unique_id"]
                    .as_str()
                    .or(v["author"]["uniqueId"].as_str())
                    .or(v["author"].as_str())
                    .unwrap_or_default()
                    .to_string(),
                author_followers: count(&v["author"]["follower_count"])
                    .or_else(|| count(&v["author"]["followerCount"]))
                    .unwrap_or(0.0),
                url: v["share_url"]
                    .as_str()
                    .or(v["url"].as_str())
                    .map(|u| u.split('?').next().unwrap_or(u).to_string())
                    .unwrap_or_default(),
            }
        })
        .collect();
    Ok((videos, reply.cost_micro))
}

/// A YouTube channel's recent videos' views, through Treg (`crawl4ai.youtube.channel.videos`):
/// what it usually gets.
pub fn channel_views(treg: &dyn Call, channel: &str) -> Result<(Vec<f64>, u64), String> {
    let reply = treg.call(
        "crawl4ai.youtube.channel.videos",
        &json!({ "channel": channel, "limit": 15 }),
    )?;
    let rows = reply.output["rows"]
        .as_array()
        .or(reply.output["videos"].as_array())
        .cloned()
        .unwrap_or_default();
    let views = rows
        .iter()
        .filter_map(|r| count(&r["views"]).or_else(|| count(&r["viewCount"])))
        .collect();
    Ok((views, reply.cost_micro))
}

/// Monthly Google searches for a keyword, and how they moved.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Keyword {
    pub keyword: String,
    pub volume: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpc: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<f64>,
    /// The last three months against the three before (1.5 = up half), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trend: Option<f64>,
}

/// Google search volume for keywords through Treg (`treg.google.keywords.volume`, one call).
pub fn keyword_volume(treg: &dyn Call, keywords: &[String]) -> Result<(Vec<Keyword>, u64), String> {
    if keywords.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let reply = treg.call(
        "treg.google.keywords.volume",
        &json!({ "keywords": keywords, "country": "us", "language": "en" }),
    )?;
    let rows = reply.output["rows"].as_array().cloned().unwrap_or_default();
    let found = rows
        .iter()
        .filter(|r| r["is_data_found"].as_bool() != Some(false))
        .filter_map(|r| {
            let keyword = r["keyword"].as_str()?.to_string();
            let trend = r["history_trend"].as_object().and_then(|h| {
                let mut months: Vec<(&String, f64)> = h
                    .iter()
                    .filter_map(|(m, v)| Some((m, v.as_f64()?)))
                    .collect();
                months.sort_by(|a, b| a.0.cmp(b.0));
                let values: Vec<f64> = months.iter().map(|(_, v)| *v).collect();
                (values.len() >= 6).then(|| {
                    let n = values.len();
                    let recent: f64 = values[n - 3..].iter().sum();
                    let before: f64 = values[n - 6..n - 3].iter().sum();
                    if before > 0.0 {
                        recent / before
                    } else {
                        1.0
                    }
                })
            });
            Some(Keyword {
                keyword,
                volume: count(&r["volume"])
                    .or_else(|| count(&r["search_volume"]))
                    .unwrap_or(0.0),
                cpc: r["cpc"].as_f64(),
                difficulty: r["difficulty"]
                    .as_f64()
                    .or(r["keyword_difficulty"].as_f64()),
                trend,
            })
        })
        .collect();
    Ok((found, reply.cost_micro))
}

/// Keyword ideas people search around a seed, through Treg (`treg.google.keywords.ideas`).
pub fn keyword_ideas(
    treg: &dyn Call,
    seed: &str,
    limit: usize,
) -> Result<(Vec<String>, u64), String> {
    let reply = treg.call(
        "treg.google.keywords.ideas",
        &json!({ "keyword": seed, "limit": limit.clamp(1, 30), "country": "us", "language": "en" }),
    )?;
    let rows = reply.output["keywords"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut ideas: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            r["value"]
                .as_str()
                .or(r["keyword"].as_str())
                .or(r.as_str())
                .map(|s| s.trim().to_lowercase())
        })
        .filter(|s| !s.is_empty())
        .collect();
    ideas.dedup();
    Ok((ideas, reply.cost_micro))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_keeps_the_words() {
        assert_eq!(
            markdown_text(
                "# Title\n\n**Bold** words and a [link text](https://x.com/a) here.\n> quoted"
            ),
            "Title\nBold words and a link text here.\nquoted"
        );
    }

    #[test]
    fn refusals_say_what_to_do() {
        assert!(refusal(401, &Value::Null).contains("refused the key"));
        assert!(
            refusal(402, &json!({"detail": {"error": "insufficient_balance"}}))
                .contains("top it up")
        );
        assert!(
            refusal(402, &json!({"detail": {"error": "route_max_cost"}})).contains("costs more")
        );
        assert!(refusal(
            429,
            &json!({"detail": {"error": "share_busy", "retry_after_s": 7}})
        )
        .contains("7 s"));
    }

    struct Canned(Value);
    impl Call for Canned {
        fn call(&self, _endpoint: &str, _body: &Value) -> Result<Reply, String> {
            Ok(Reply {
                output: self.0.clone(),
                cost_micro: 150,
                served_by: "x".into(),
            })
        }
    }

    #[test]
    fn search_and_extract_read_the_answers() {
        let search = Canned(json!({ "results": [
            { "title": "T", "url": "https://a.com/x", "snippet": "S" },
            { "title": "no url" }
        ]}));
        let (results, cost) = web_search(&search, "q", 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!((results[0].url.as_str(), cost), ("https://a.com/x", 150));
        let page = Canned(
            json!({ "pages": [{ "markdown": "## Hi\nThere", "final_url": "https://pub.com/a" }] }),
        );
        let (text, final_url, _) = extract(&page, "https://news.google.com/x").unwrap();
        assert_eq!(text, "Hi\nThere");
        assert_eq!(final_url.as_deref(), Some("https://pub.com/a"));
        assert!(extract(&Canned(json!({ "pages": [{}] })), "https://a.com").is_err());
    }
}
