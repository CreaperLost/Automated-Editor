//! Finding stories on the web (FCL-1), ported from the explainer's `sources.py`. Each funnel
//! returns findings; one failing never blocks the others, and the report says why it failed
//! (rate limit, access denied, a page that didn't parse) instead of showing it as empty.
//!
//! Runs only when the user or their agent asks.
use super::profile::Compiled;
use super::stories::{Finding, Store};
use chrono::{DateTime, Duration, TimeZone, Utc};
use quick_xml::events::Event;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration as StdDuration;

/// Why a request failed.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchError {
    pub status: Option<u16>,
    pub message: String,
}

/// Something that GETs a URL; tests answer from fixtures.
pub trait Fetch: Sync {
    fn get(&self, url: &str, query: &[(&str, String)]) -> Result<String, FetchError>;
}

/// The real web.
pub struct Web {
    client: reqwest::blocking::Client,
}

impl Web {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("AeroEdits (faceless story search)")
            .timeout(StdDuration::from_secs(30))
            .connect_timeout(StdDuration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client })
    }
}

impl Fetch for Web {
    fn get(&self, url: &str, query: &[(&str, String)]) -> Result<String, FetchError> {
        let response = self
            .client
            .get(url)
            .query(query)
            .send()
            .map_err(|e| FetchError {
                status: None,
                message: if e.is_timeout() {
                    "timed out".into()
                } else {
                    "could not connect".into()
                },
            })?;
        let status = response.status();
        if !status.is_success() {
            let limited = status.as_u16() == 403
                && response
                    .headers()
                    .get("x-ratelimit-remaining")
                    .is_some_and(|v| v == "0");
            return Err(FetchError {
                status: Some(if limited { 429 } else { status.as_u16() }),
                message: String::new(),
            });
        }
        response.text().map_err(|_| FetchError {
            status: None,
            message: "could not read the reply".into(),
        })
    }
}

/// What one funnel did.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunnelReport {
    pub name: String,
    pub requests: usize,
    pub parsed: usize,
    pub found: usize,
    pub accepted: usize,
    pub new_stories: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success: Option<DateTime<Utc>>,
    pub issues: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
    /// What its paid calls cost (Treg), in micro-dollars.
    #[serde(default)]
    pub cost_micro: u64,
}

impl FunnelReport {
    fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// `ok`, `no matches`, `partial` or `error`.
    pub fn state(&self) -> &'static str {
        if !self.issues.is_empty() {
            if self.parsed > 0 {
                "partial"
            } else {
                "error"
            }
        } else if self.accepted > 0 {
            "ok"
        } else {
            "no matches"
        }
    }

    pub fn line(&self) -> String {
        let issues = if self.issues.is_empty() {
            String::new()
        } else {
            format!("; {}", self.issues.join("; "))
        };
        let cost = if self.cost_micro > 0 {
            format!(", cost {}", super::treg::dollars(self.cost_micro))
        } else {
            String::new()
        };
        if let Some(skipped) = &self.skipped {
            return format!("{}: skipped, {skipped}", self.name);
        }
        format!(
            "{}: {}; fetched {}/{}, found {}, kept {}, new {}{cost}{issues}",
            self.name,
            self.state(),
            self.parsed,
            self.requests,
            self.found,
            self.accepted,
            self.new_stories
        )
    }

    fn get(&mut self, web: &dyn Fetch, url: &str, query: &[(&str, String)]) -> Result<String, ()> {
        self.requests += 1;
        web.get(url, query).map_err(|e| self.failed(url, &e))
    }

    fn parsed(&mut self, count: usize, now: DateTime<Utc>) {
        self.parsed += 1;
        self.found += count;
        self.last_success = Some(now);
    }

    fn failed(&mut self, url: &str, error: &FetchError) {
        let reason = match error.status {
            Some(429) => "rate limit (HTTP 429)".to_string(),
            Some(code @ (401 | 403)) => format!("credentials or access denied (HTTP {code})"),
            Some(402) => "payment required (HTTP 402)".to_string(),
            Some(code) => format!("HTTP {code}"),
            None => error.message.clone(),
        };
        self.issues.push(format!("{}: {reason}", target(url)));
    }

    fn unparsed(&mut self, url: &str) {
        self.issues
            .push(format!("{}: could not parse the reply", target(url)));
    }
}

fn target(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    rest.split(['?', '#']).next().unwrap_or(rest).to_string()
}

fn parse_date(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(d) = DateTime::parse_from_rfc3339(&value.replace('Z', "+00:00")) {
        return Some(d.with_timezone(&Utc));
    }
    if let Ok(d) = DateTime::parse_from_rfc2822(value) {
        return Some(d.with_timezone(&Utc));
    }
    // A date without a zone is UTC.
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
        .ok()
        .map(|d| Utc.from_utc_datetime(&d))
}

/// `&amp;`, `&#39;`, `&#x2019;` and the common names, once (feeds often escape twice).
pub fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        // An entity is short; look a few characters on (never inside a character).
        let end = rest
            .char_indices()
            .take_while(|(i, _)| *i < 12)
            .find(|(_, c)| *c == ';')
            .map(|(i, _)| i);
        let decoded = end.and_then(|end| {
            let name = &rest[1..end];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some(' '),
                "rsquo" | "lsquo" => Some('\''),
                "rdquo" | "ldquo" => Some('"'),
                "mdash" => Some('—'),
                "ndash" => Some('–'),
                "hellip" => Some('…'),
                _ => name
                    .strip_prefix("#x")
                    .or_else(|| name.strip_prefix("#X"))
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|n| n.parse().ok()))
                    .and_then(char::from_u32),
            };
            c.map(|c| (c, end))
        });
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The words of an HTML fragment: tags out, entities decoded, spaces collapsed.
pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    unescape(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// One RSS item or Atom entry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub title: String,
    pub url: String,
    pub date: Option<DateTime<Utc>>,
    pub publisher: Option<String>,
    pub summary: Option<String>,
}

fn local(name: &str) -> String {
    let name = name.to_string();
    name.rsplit(':').next().unwrap_or(&name).to_string()
}

/// The entries of an RSS 2, RSS 1 or Atom document.
pub fn parse_feed(xml: &str) -> Result<Vec<Entry>, String> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut entries = Vec::new();
    let mut depth = 0usize;
    let mut root: Option<String> = None;
    // The depth of the item being read, its entry, the child being read and its text.
    let mut item: Option<(usize, Entry)> = None;
    let mut child: Option<(String, String, Option<String>)> = None;
    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        match event {
            Event::Start(e) => {
                depth += 1;
                let name = local(e.name().as_ref());
                if root.is_none() {
                    root = Some(name.clone());
                }
                match &item {
                    None if name == "item" || name == "entry" => {
                        item = Some((depth, Entry::default()));
                    }
                    Some((at, _)) if depth == at + 1 => {
                        let href = link_href(&e);
                        child = Some((name, String::new(), href));
                    }
                    _ => {}
                }
            }
            Event::Empty(e) => {
                if let (Some((at, entry)), true) = (&mut item, local(e.name().as_ref()) == "link") {
                    if depth == *at {
                        if let Some(href) = link_href(&e) {
                            entry.url = href;
                        }
                    }
                }
            }
            Event::Text(t) => {
                if let Some((_, text, _)) = &mut child {
                    text.push_str(&t.into_inner());
                }
            }
            Event::GeneralRef(r) => {
                if let Some((_, text, _)) = &mut child {
                    text.push('&');
                    text.push_str(&r.into_inner());
                    text.push(';');
                }
            }
            Event::CData(t) => {
                if let Some((_, text, _)) = &mut child {
                    text.push_str(&t.into_inner());
                }
            }
            Event::End(_) => {
                if let (Some((at, entry)), Some((name, text, href))) = (&mut item, &child) {
                    if depth == *at + 1 {
                        let text = text.trim();
                        match name.as_str() {
                            // Feeds often escape titles twice.
                            "title" => entry.title = unescape(&unescape(text)).trim().to_string(),
                            "link" => {
                                if let Some(href) = href {
                                    entry.url = href.clone();
                                } else if !text.is_empty() {
                                    entry.url = unescape(text);
                                }
                            }
                            "pubDate" | "published" | "date" => entry.date = parse_date(text),
                            "updated" if entry.date.is_none() => entry.date = parse_date(text),
                            "source" => entry.publisher = Some(unescape(text)),
                            "description" | "summary" | "content" if entry.summary.is_none() => {
                                let plain: String =
                                    html_to_text(&unescape(text)).chars().take(1500).collect();
                                entry.summary = Some(plain);
                            }
                            _ => {}
                        }
                        child = None;
                    }
                }
                if let Some((at, _)) = &item {
                    if depth == *at {
                        let (_, entry) = item.take().unwrap();
                        if !entry.title.is_empty() && !entry.url.is_empty() {
                            entries.push(entry);
                        }
                    }
                }
                depth = depth.saturating_sub(1);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    match root.as_deref() {
        Some("rss" | "RDF" | "feed") => Ok(entries),
        _ => Err("not an RSS or Atom feed".into()),
    }
}

/// An Atom link's `href`, when it is the page itself (`rel` absent or `alternate`).
fn link_href(e: &quick_xml::events::BytesStart) -> Option<String> {
    let mut href = None;
    let mut alternate = true;
    for attr in e.attributes().flatten() {
        let value = attr.value.to_string();
        match attr.key.as_ref() {
            "href" => href = Some(unescape(&value)),
            "rel" => alternate = value == "alternate",
            _ => {}
        }
    }
    href.filter(|_| alternate)
}

/// The `<loc>` URLs of a sitemap URL set.
pub fn parse_sitemap(xml: &str) -> Result<Vec<String>, String> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut root = None;
    let mut in_loc = false;
    let mut out = Vec::new();
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => {
                let name = local(e.name().as_ref());
                if root.is_none() {
                    root = Some(name.clone());
                }
                in_loc = name == "loc";
                if in_loc {
                    out.push(String::new());
                }
            }
            Event::Text(t) if in_loc => {
                if let Some(last) = out.last_mut() {
                    last.push_str(t.into_inner().trim());
                }
            }
            Event::End(_) => in_loc = false,
            Event::Eof => break,
            _ => {}
        }
    }
    if root.as_deref() != Some("urlset") {
        return Err("not a sitemap URL set".into());
    }
    Ok(out)
}

fn json(text: &str) -> Option<Value> {
    serde_json::from_str(text).ok()
}

fn mostly_english(text: &str) -> bool {
    let total = text.chars().count();
    total == 0 || text.chars().filter(|c| c.is_ascii()).count() * 10 >= total * 7
}

fn host_of(url: &str) -> String {
    target(url)
        .split('/')
        .next()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_string()
}

struct Run<'a> {
    web: &'a dyn Fetch,
    profile: &'a Compiled,
    now: DateTime<Utc>,
    treg: Option<&'a dyn super::treg::Call>,
    /// What paid calls cost so far in this search, and the guard's limit (micro-dollars).
    spent: std::sync::atomic::AtomicU64,
    cap: u64,
}

/// A post's words as a headline: links out, spaces collapsed, at most 140 characters.
fn post_title(text: &str) -> String {
    let plain: String = unescape(text)
        .split_whitespace()
        .filter(|w| !w.starts_with("http://") && !w.starts_with("https://"))
        .collect::<Vec<_>>()
        .join(" ");
    plain.chars().take(140).collect()
}

/// Text without hashtags, so "#Gemini" on a horoscope is not the channel's subject.
fn without_hashtags(text: &str) -> String {
    text.split_whitespace()
        .filter(|w| !w.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ")
}

/// X's search wants times as `2026-11-07_10:00:00_UTC`.
fn x_time(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d_%H:%M:%S_UTC").to_string()
}

/// `(from:a OR from:b ...)` clauses for these handles, each short enough for one X search.
pub fn from_batches(handles: &[String], max_chars: usize) -> Vec<String> {
    let mut batches = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for handle in handles {
        let clause = format!("from:{}", handle.trim_start_matches('@'));
        let length: usize = current.iter().map(|c| c.len() + 4).sum::<usize>() + clause.len() + 2;
        if !current.is_empty() && length > max_chars {
            batches.push(format!("({})", current.join(" OR ")));
            current.clear();
        }
        current.push(clause);
    }
    if !current.is_empty() {
        batches.push(format!("({})", current.join(" OR ")));
    }
    batches
}

/// The posts in an X search answer, whichever provider served it.
fn x_posts(output: &Value) -> Vec<Value> {
    output["posts"]
        .as_array()
        .or(output["data"]["items"].as_array())
        .or(output["tweets"].as_array())
        .or(output["results"].as_array())
        .cloned()
        .unwrap_or_default()
}

impl Run<'_> {
    /// A Treg call within this search's guard: past it, nothing more is paid for.
    fn paid(
        &self,
        treg: &dyn super::treg::Call,
        endpoint: &str,
        body: &Value,
        report: &mut FunnelReport,
    ) -> Result<super::treg::Reply, ()> {
        if self.spent.load(std::sync::atomic::Ordering::Relaxed) >= self.cap {
            if !report.issues.iter().any(|i| i.contains("guard")) {
                report.issues.push(format!(
                    "stopped at this search's {} guard",
                    super::treg::dollars(self.cap)
                ));
            }
            return Err(());
        }
        report.requests += 1;
        match treg.call(endpoint, body) {
            Ok(reply) => {
                self.spent
                    .fetch_add(reply.cost_micro, std::sync::atomic::Ordering::Relaxed);
                report.cost_micro += reply.cost_micro;
                Ok(reply)
            }
            Err(e) => {
                report.issues.push(e);
                Err(())
            }
        }
    }

    /// Tracked X accounts in batched searches (`from:a OR from:b`, each group with its likes
    /// floor), then the keyword searches, all only since the last search. A post passes when
    /// it is fresh, about the channel's subject (hashtags aside; labs always are), not a reply,
    /// and liked enough: a tracked account's floor, and for `relative` groups more than that
    /// account usually gets; an untracked author needs `minFollowers`.
    fn x(&self, store: &mut Store, report: &mut FunnelReport) -> Vec<Finding> {
        let Some(source) = self
            .profile
            .profile
            .sources
            .treg
            .as_ref()
            .filter(|s| s.enabled && (!s.watch.is_empty() || !s.x_searches.is_empty()))
        else {
            return Vec::new();
        };
        let Some(treg) = self.treg else {
            report.skipped = Some("needs a Treg key (File → Accounts)".into());
            return Vec::new();
        };
        let oldest = self.now - Duration::hours(source.lookback_hours);
        // Only what is new since the last X search (an hour of overlap for late counts).
        let since = store
            .last_success
            .get("x")
            .map(|at| *at - Duration::hours(1))
            .filter(|at| *at > oldest)
            .unwrap_or(oldest);
        let filters = format!("-filter:replies -filter:retweets since:{}", x_time(since));
        let mut queries: Vec<String> = Vec::new();
        for group in &source.watch {
            for batch in from_batches(&group.handles, 420) {
                queries.push(format!("{batch} min_faves:{} {filters}", group.min_likes));
            }
        }
        for search in &source.x_searches {
            queries.push(format!("{search} since:{}", x_time(since)));
        }
        let mut found = std::collections::BTreeMap::new();
        for query in queries {
            let Ok(reply) = self.paid(
                treg,
                "treg.x.search.posts",
                &serde_json::json!({ "q": query }),
                report,
            ) else {
                continue;
            };
            let posts = x_posts(&reply.output);
            for post in &posts {
                let text = post["text"]
                    .as_str()
                    .or(post["full_text"].as_str())
                    .unwrap_or_default();
                let Some(id) = post["id"]
                    .as_str()
                    .map(str::to_string)
                    .or(post["id"].as_u64().map(|n| n.to_string()))
                else {
                    continue;
                };
                let author = post["authorUsername"]
                    .as_str()
                    .or(post["author"]["username"].as_str())
                    .or(post["user"]["screen_name"].as_str())
                    .unwrap_or_default()
                    .to_string();
                let likes = super::treg::count(&post["likeCount"])
                    .or_else(|| super::treg::count(&post["favorite_count"]))
                    .or_else(|| super::treg::count(&post["likes"]))
                    .unwrap_or(0.0);
                let followers = super::treg::count(&post["authorFollowers"])
                    .or_else(|| super::treg::count(&post["author"]["followers"]))
                    .unwrap_or(0.0);
                let at = post["createdUtc"]
                    .as_i64()
                    .and_then(|s| Utc.timestamp_opt(s, 0).single())
                    .or_else(|| post["created_at"].as_str().and_then(parse_date))
                    .unwrap_or(self.now);
                let group = source.group_of(&author);
                let stats_key = format!("x:@{}", author.to_lowercase());
                // What it usually gets, before this post counts towards it.
                let usual = store.accounts.get(&stats_key).and_then(|s| s.usual());
                if group.is_some() {
                    store
                        .accounts
                        .entry(stats_key.clone())
                        .or_default()
                        .remember(likes);
                }
                let title = post_title(text);
                let on_subject = group.is_some_and(|g| g.primary)
                    || self.profile.on_subject(&without_hashtags(text));
                let liked_enough = match group {
                    Some(g) => {
                        likes >= f64::from(g.min_likes)
                            && (!g.relative || usual.is_none_or(|u| likes >= u))
                    }
                    None => followers >= source.min_followers,
                };
                if at < oldest
                    || title.chars().count() < 20
                    || post["isReply"].as_bool() == Some(true)
                    || !on_subject
                    || !liked_enough
                {
                    continue;
                }
                if group.is_some() {
                    store.accounts.entry(stats_key).or_default().kept += 1;
                }
                let key = format!("x:{id}");
                found.insert(
                    key.clone(),
                    Finding {
                        source: format!("x:@{author}"),
                        key,
                        title,
                        url: post["url"]
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| format!("https://x.com/{author}/status/{id}")),
                        kind: "news".into(),
                        event_at: at,
                        metric: Some(("likes".into(), likes)),
                        summary: Some(text.chars().take(1500).collect()),
                    },
                );
            }
            report.parsed(posts.len(), self.now);
        }
        found.into_values().collect()
    }

    /// The day's top posts of the profile's subreddits through Treg: their order is the
    /// filter (the first `subredditTop` of each).
    fn reddit(&self, report: &mut FunnelReport) -> Vec<Finding> {
        let Some(source) = self
            .profile
            .profile
            .sources
            .treg
            .as_ref()
            .filter(|s| s.enabled && !s.subreddits.is_empty())
        else {
            return Vec::new();
        };
        let Some(treg) = self.treg else {
            report.skipped = Some("needs a Treg key (File → Accounts)".into());
            return Vec::new();
        };
        let since = self.now - Duration::hours(source.lookback_hours);
        let mut out = Vec::new();
        for subreddit in &source.subreddits {
            let body = serde_json::json!({ "subreddit": subreddit, "sort": "top", "t": "day", "limit": source.subreddit_top.max(1) });
            let Ok(reply) = self.paid(treg, "crawl4ai.reddit.subreddit.posts", &body, report)
            else {
                continue;
            };
            let rows = reply.output["rows"]
                .as_array()
                .or(reply.output["posts"].as_array())
                .cloned()
                .unwrap_or_default();
            for (place, row) in rows.iter().take(source.subreddit_top.max(1)).enumerate() {
                let (Some(title), Some(url)) = (row["title"].as_str(), row["url"].as_str()) else {
                    continue;
                };
                let at = row["created"]
                    .as_str()
                    .and_then(parse_date)
                    .unwrap_or(self.now);
                if at < since || !self.profile.on_subject(&format!("{title} {subreddit}")) {
                    continue;
                }
                let clean = url.split('?').next().unwrap_or(url).trim_end_matches('/');
                out.push(Finding {
                    source: format!("reddit:{subreddit}"),
                    key: format!("url:{clean}"),
                    title: unescape(title),
                    url: clean.to_string(),
                    kind: "news".into(),
                    event_at: at,
                    // Its place among the day's top: 1st is worth more than 5th.
                    metric: Some((
                        "top place".into(),
                        (source.subreddit_top.max(1) - place) as f64 * 20.0,
                    )),
                    summary: row["text"].as_str().map(|t| t.chars().take(1500).collect()),
                });
            }
            report.parsed(rows.len(), self.now);
        }
        out
    }
}

impl Run<'_> {
    fn hackernews(&self, report: &mut FunnelReport) -> Vec<Finding> {
        let Some(source) = self
            .profile
            .profile
            .sources
            .hackernews
            .as_ref()
            .filter(|s| s.enabled)
        else {
            return Vec::new();
        };
        let since = (self.now - Duration::hours(source.lookback_hours)).timestamp();
        let numeric = format!("created_at_i>{since},points>={}", source.min_points);
        let mut searches: Vec<Vec<(&str, String)>> = source
            .queries
            .iter()
            .map(|q| {
                vec![
                    ("query", q.clone()),
                    ("tags", "story".into()),
                    ("restrictSearchableAttributes", "title".into()),
                    ("typoTolerance", "false".into()),
                    ("numericFilters", numeric.clone()),
                    ("hitsPerPage", "30".into()),
                ]
            })
            .collect();
        searches.push(vec![
            ("tags", "front_page".into()),
            ("numericFilters", numeric.clone()),
            ("hitsPerPage", "60".into()),
        ]);
        let url = "https://hn.algolia.com/api/v1/search";
        let mut found = std::collections::BTreeMap::new();
        for query in searches {
            let front_page = !query.iter().any(|(k, _)| *k == "query");
            let Ok(text) = report.get(self.web, url, &query) else {
                continue;
            };
            let Some(hits) = json(&text).and_then(|v| v["hits"].as_array().cloned()) else {
                report.unparsed(url);
                continue;
            };
            for hit in &hits {
                let (Some(title), Some(id)) = (hit["title"].as_str(), hit["objectID"].as_str())
                else {
                    continue;
                };
                if front_page && !self.profile.on_subject(title) {
                    continue;
                }
                let key = format!("hn:{id}");
                let at = Utc
                    .timestamp_opt(hit["created_at_i"].as_i64().unwrap_or(0), 0)
                    .single()
                    .unwrap_or(self.now);
                found.insert(
                    key.clone(),
                    Finding {
                        source: "hackernews".into(),
                        key,
                        title: title.into(),
                        url: hit["url"].as_str().map_or_else(
                            || format!("https://news.ycombinator.com/item?id={id}"),
                            str::to_string,
                        ),
                        kind: "news".into(),
                        event_at: at,
                        metric: Some(("points".into(), hit["points"].as_f64().unwrap_or(0.0))),
                        summary: None,
                    },
                );
            }
            report.parsed(hits.len(), self.now);
        }
        found.into_values().collect()
    }

    fn openrouter_models(&self, report: &mut FunnelReport) -> Vec<Finding> {
        let Some(source) = self
            .profile
            .profile
            .sources
            .openrouter_models
            .as_ref()
            .filter(|s| s.enabled)
        else {
            return Vec::new();
        };
        let url = "https://openrouter.ai/api/v1/models";
        let Ok(text) = report.get(self.web, url, &[]) else {
            return Vec::new();
        };
        let Some(models) = json(&text).and_then(|v| v["data"].as_array().cloned()) else {
            report.unparsed(url);
            return Vec::new();
        };
        let since = self.now - Duration::hours(source.lookback_hours);
        let dated = regex::Regex::new(r"-\d{4,8}$").unwrap();
        let out = models
            .iter()
            .filter_map(|m| {
                let id = m["id"].as_str()?;
                let created = Utc.timestamp_opt(m["created"].as_i64()?, 0).single()?;
                // ":" marks variants such as :free.
                if created < since || id.contains(':') {
                    return None;
                }
                Some(Finding {
                    source: "openrouter_models".into(),
                    key: format!("model:{}", dated.replace(id, "")),
                    title: format!("New model: {}", m["name"].as_str().unwrap_or(id)),
                    url: format!("https://openrouter.ai/{id}"),
                    kind: "release".into(),
                    event_at: created,
                    metric: None,
                    summary: None,
                })
            })
            .collect();
        report.parsed(models.len(), self.now);
        out
    }

    fn github_releases(&self, report: &mut FunnelReport) -> Vec<Finding> {
        let Some(source) = self
            .profile
            .profile
            .sources
            .github_releases
            .as_ref()
            .filter(|s| s.enabled)
        else {
            return Vec::new();
        };
        let since = self.now - Duration::hours(source.lookback_hours);
        let mut out = Vec::new();
        for repo in &source.repos {
            let url = format!("https://api.github.com/repos/{repo}/releases");
            let Ok(text) = report.get(self.web, &url, &[("per_page", "10".into())]) else {
                continue;
            };
            let Some(releases) = json(&text).and_then(|v| v.as_array().cloned()) else {
                report.unparsed(&url);
                continue;
            };
            for r in &releases {
                let Some(published) = r["published_at"].as_str().and_then(parse_date) else {
                    continue;
                };
                if r["draft"].as_bool() == Some(true)
                    || r["prerelease"].as_bool() == Some(true)
                    || published < since
                {
                    continue;
                }
                let tag = r["tag_name"].as_str().unwrap_or_default();
                let name = r["name"].as_str().filter(|n| !n.is_empty()).unwrap_or(tag);
                out.push(Finding {
                    source: "github_releases".into(),
                    key: format!("gh:{repo}:{tag}"),
                    title: format!("{repo} {name}"),
                    url: r["html_url"].as_str().unwrap_or_default().into(),
                    kind: "release".into(),
                    event_at: published,
                    metric: None,
                    summary: r["body"].as_str().map(|b| b.chars().take(1500).collect()),
                });
            }
            report.parsed(releases.len(), self.now);
        }
        out
    }

    fn feeds(&self, report: &mut FunnelReport) -> Vec<Finding> {
        let Some(source) = self
            .profile
            .profile
            .sources
            .feeds
            .as_ref()
            .filter(|s| s.enabled)
        else {
            return Vec::new();
        };
        let since = self.now - Duration::hours(source.lookback_hours);
        // Fetch in parallel: most of the time is waiting for servers.
        let fetched: Vec<(String, Result<String, FetchError>)> = std::thread::scope(|scope| {
            let chunks: Vec<_> = source
                .urls
                .chunks(source.urls.len().div_ceil(8).max(1))
                .map(|chunk| {
                    scope.spawn(move || {
                        chunk
                            .iter()
                            .map(|url| (url.clone(), self.web.get(url, &[])))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            chunks
                .into_iter()
                .flat_map(|h| h.join().unwrap_or_default())
                .collect()
        });
        let mut out = Vec::new();
        for (url, result) in fetched {
            report.requests += 1;
            let text = match result {
                Ok(text) => text,
                Err(e) => {
                    report.failed(&url, &e);
                    continue;
                }
            };
            let Ok(entries) = parse_feed(&text) else {
                report.unparsed(&url);
                continue;
            };
            report.parsed(entries.len(), self.now);
            let host = host_of(&url);
            let subject_only = source
                .subject_only_hosts
                .iter()
                .any(|h| host == *h || host.ends_with(&format!(".{h}")));
            for entry in entries {
                let Some(date) = entry.date.filter(|d| *d >= since) else {
                    continue;
                };
                let text = format!(
                    "{} {}",
                    entry.title,
                    entry.summary.as_deref().unwrap_or_default()
                );
                if !subject_only && !self.profile.on_subject(&text) {
                    continue;
                }
                let mut title = entry.title.clone();
                if let Some(publisher) = &entry.publisher {
                    if let Some(stripped) = title.strip_suffix(&format!(" - {publisher}")) {
                        title = stripped.to_string();
                    }
                }
                let clean = entry.url.split('?').next().unwrap_or(&entry.url);
                out.push(Finding {
                    // One Google News feed is many outlets, and outlets rank a story.
                    source: format!("feed:{}", entry.publisher.clone().unwrap_or(host.clone())),
                    key: format!("url:{}", clean.trim_end_matches('/')),
                    title,
                    url: entry.url,
                    kind: "news".into(),
                    event_at: date,
                    metric: None,
                    summary: entry.summary,
                });
            }
        }
        out
    }

    fn huggingface(&self, report: &mut FunnelReport) -> Vec<Finding> {
        let Some(source) = self
            .profile
            .profile
            .sources
            .huggingface
            .as_ref()
            .filter(|s| s.enabled)
        else {
            return Vec::new();
        };
        let url = "https://huggingface.co/api/models";
        let query = [
            ("sort", "trendingScore".to_string()),
            ("limit", source.limit.to_string()),
            ("full", "true".into()),
        ];
        let Ok(text) = report.get(self.web, url, &query) else {
            return Vec::new();
        };
        let Some(models) = json(&text).and_then(|v| v.as_array().cloned()) else {
            report.unparsed(url);
            return Vec::new();
        };
        let since = self.now - Duration::days(source.lookback_days);
        let out = models
            .iter()
            .filter_map(|m| {
                let id = m["id"].as_str()?;
                // A long-popular model is not news.
                let created = m["createdAt"].as_str().and_then(parse_date)?;
                (created >= since).then(|| Finding {
                    source: "huggingface".into(),
                    key: format!("hf:{id}"),
                    title: format!("Trending on Hugging Face: {id}"),
                    url: format!("https://huggingface.co/{id}"),
                    kind: "release".into(),
                    event_at: created,
                    metric: Some((
                        "trending_score".into(),
                        m["trendingScore"].as_f64().unwrap_or(0.0),
                    )),
                    summary: None,
                })
            })
            .collect();
        report.parsed(models.len(), self.now);
        out
    }

    fn repo_finding(&self, name: &str, description: &str, stars: Option<f64>) -> Finding {
        let short: String = description.chars().take(110).collect();
        Finding {
            source: "github_viral".into(),
            key: format!("repo:{}", name.to_lowercase()),
            title: format!("Viral on GitHub: {name}: {short}")
                .trim_end_matches([':', ' '])
                .to_string(),
            url: format!("https://github.com/{name}"),
            kind: "release".into(),
            // When we first saw it take off, not its creation; a known repo keeps its date.
            event_at: self.now,
            metric: stars.map(|s| ("stars".into(), s)),
            summary: (!description.is_empty()).then(|| description.chars().take(1500).collect()),
        }
    }

    fn github_viral(&self, report: &mut FunnelReport) -> Vec<Finding> {
        let Some(source) = self
            .profile
            .profile
            .sources
            .github_viral
            .as_ref()
            .filter(|s| s.enabled)
        else {
            return Vec::new();
        };
        let since = (self.now - Duration::days(source.lookback_days))
            .date_naive()
            .to_string();
        let url = "https://api.github.com/search/repositories";
        let query = [
            ("q", format!("created:>{since} stars:>{}", source.min_stars)),
            ("sort", "stars".into()),
            ("order", "desc".into()),
            ("per_page", source.limit.to_string()),
        ];
        let mut found = std::collections::BTreeMap::new();
        if let Ok(text) = report.get(self.web, url, &query) {
            match json(&text).and_then(|v| v["items"].as_array().cloned()) {
                Some(repos) => {
                    for repo in &repos {
                        let name = repo["full_name"].as_str().unwrap_or_default();
                        let description = repo["description"].as_str().unwrap_or_default();
                        let topics = repo["topics"]
                            .as_array()
                            .map(|t| {
                                t.iter()
                                    .filter_map(Value::as_str)
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                            .unwrap_or_default();
                        if name.is_empty()
                            || !self
                                .profile
                                .on_subject_repo(&format!("{name} {description} {topics}"))
                        {
                            continue;
                        }
                        let stars = repo["stargazers_count"].as_f64().unwrap_or(0.0);
                        if mostly_english(description)
                            || stars >= f64::from(source.non_english_min_stars)
                        {
                            let f = self.repo_finding(name, description, Some(stars));
                            found.insert(f.key.clone(), f);
                        }
                    }
                    report.parsed(repos.len(), self.now);
                }
                None => report.unparsed(url),
            }
        }
        // An unofficial mirror of GitHub Trending: losing it must not lose the search above.
        let mut lookups = 0;
        for feed in &source.trending_feeds {
            let Ok(text) = report.get(self.web, feed, &[]) else {
                continue;
            };
            let Ok(entries) = parse_feed(&text) else {
                report.unparsed(feed);
                continue;
            };
            report.parsed(entries.len(), self.now);
            for entry in entries {
                let name = entry.title.trim().to_string();
                let description: String = entry
                    .summary
                    .unwrap_or_default()
                    .chars()
                    .take(300)
                    .collect();
                let key = format!("repo:{}", name.to_lowercase());
                if !name.contains('/')
                    || found.contains_key(&key)
                    || !self
                        .profile
                        .on_subject_repo(&format!("{name} {description}"))
                {
                    continue;
                }
                let mut stars = None;
                // GitHub allows 60 calls an hour without a key.
                if lookups < source.max_star_lookups {
                    lookups += 1;
                    let detail = format!("https://api.github.com/repos/{name}");
                    if let Ok(text) = report.get(self.web, &detail, &[]) {
                        stars = json(&text).and_then(|v| v["stargazers_count"].as_f64());
                        if stars.is_some() {
                            report.parsed(0, self.now);
                        }
                    }
                }
                if mostly_english(&description)
                    || stars.unwrap_or(0.0) >= f64::from(source.non_english_min_stars)
                {
                    let f = self.repo_finding(&name, &description, stars);
                    found.insert(f.key.clone(), f);
                }
            }
        }
        found.into_values().collect()
    }

    fn papers(&self, report: &mut FunnelReport) -> Vec<Finding> {
        let Some(source) = self
            .profile
            .profile
            .sources
            .papers
            .as_ref()
            .filter(|s| s.enabled)
        else {
            return Vec::new();
        };
        let url = "https://huggingface.co/api/daily_papers";
        let query = [
            ("sort", "trending".to_string()),
            ("limit", source.limit.to_string()),
        ];
        let Ok(text) = report.get(self.web, url, &query) else {
            return Vec::new();
        };
        let Some(items) = json(&text).and_then(|v| v.as_array().cloned()) else {
            report.unparsed(url);
            return Vec::new();
        };
        let since = self.now - Duration::days(source.lookback_days);
        let out = items
            .iter()
            .filter_map(|item| {
                let paper = &item["paper"];
                let id = paper["id"].as_str()?;
                let published = paper["publishedAt"]
                    .as_str()
                    .or(item["publishedAt"].as_str())
                    .and_then(parse_date)?;
                let upvotes = paper["upvotes"].as_f64().unwrap_or(0.0);
                if published < since || upvotes < f64::from(source.min_upvotes) {
                    return None;
                }
                let title = item["title"].as_str().or(paper["title"].as_str())?;
                let summary = paper["ai_summary"]
                    .as_str()
                    .or(item["summary"].as_str())
                    .map(|s| s.chars().take(1500).collect());
                Some(Finding {
                    source: "papers".into(),
                    key: format!("paper:{id}"),
                    title: format!("Paper: {title}"),
                    url: format!("https://huggingface.co/papers/{id}"),
                    kind: "news".into(),
                    // When it took off for us; a known paper keeps its first date.
                    event_at: self.now,
                    metric: Some(("upvotes".into(), upvotes)),
                    summary,
                })
            })
            .collect();
        report.parsed(items.len(), self.now);
        out
    }

    /// New pages on lab sites with no feed. Their dates are rewritten in bulk, so new means a
    /// URL not seen before; the first run only records what exists.
    fn sitemaps(&self, store: &mut Store, report: &mut FunnelReport) -> Vec<Finding> {
        let Some(source) = self
            .profile
            .profile
            .sources
            .sitemaps
            .as_ref()
            .filter(|s| s.enabled)
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for site in &source.sites {
            let Ok(text) = report.get(self.web, &site.url, &[]) else {
                continue;
            };
            let Ok(locations) = parse_sitemap(&text) else {
                report.unparsed(&site.url);
                continue;
            };
            report.parsed(locations.len(), self.now);
            let urls: std::collections::BTreeSet<String> = locations
                .iter()
                .filter(|u| site.include.iter().any(|part| u.contains(part.as_str())))
                .map(|u| u.trim().trim_end_matches('/').to_string())
                .collect();
            let seen = store.sitemap_seen.get(&site.name).cloned();
            let mut all: std::collections::BTreeSet<String> =
                seen.clone().unwrap_or_default().into_iter().collect();
            all.extend(urls.iter().cloned());
            store
                .sitemap_seen
                .insert(site.name.clone(), all.into_iter().collect());
            let Some(seen) = seen else {
                continue;
            };
            for url in urls.iter().filter(|u| !seen.contains(u)) {
                let slug = url.rsplit('/').next().unwrap_or(url).replace('-', " ");
                out.push(Finding {
                    source: format!("sitemap:{}", site.name),
                    key: format!("url:{url}"),
                    title: format!("{}: {slug}", site.name),
                    url: url.clone(),
                    kind: "news".into(),
                    event_at: self.now,
                    metric: None,
                    summary: None,
                });
            }
        }
        out
    }
}

/// The funnels, in the order they run.
pub const FUNNELS: [&str; 10] = [
    "hackernews",
    "openrouter_models",
    "github_releases",
    "feeds",
    "huggingface",
    "github_viral",
    "papers",
    "sitemaps",
    "x",
    "reddit",
];

/// Runs every enabled funnel, adds what they found to `store`, drops week-old stories, and
/// says what each funnel did. `progress` hears each funnel's name as it starts.
pub fn discover(
    web: &dyn Fetch,
    profile: &Compiled,
    store: &mut Store,
    now: DateTime<Utc>,
    treg: Option<&dyn super::treg::Call>,
    progress: &dyn Fn(usize, &str),
) -> Vec<FunnelReport> {
    let run = Run {
        web,
        profile,
        now,
        treg,
        spent: std::sync::atomic::AtomicU64::new(0),
        cap: profile
            .profile
            .sources
            .treg
            .as_ref()
            .map_or(100_000, |t| (t.max_cost_per_search * 1_000_000.0) as u64),
    };
    let mut reports = Vec::new();
    for (index, name) in FUNNELS.iter().enumerate() {
        progress(index, name);
        let mut report = FunnelReport::new(name);
        let findings = match *name {
            "hackernews" => run.hackernews(&mut report),
            "openrouter_models" => run.openrouter_models(&mut report),
            "github_releases" => run.github_releases(&mut report),
            "feeds" => run.feeds(&mut report),
            "huggingface" => run.huggingface(&mut report),
            "github_viral" => run.github_viral(&mut report),
            "papers" => run.papers(&mut report),
            "x" => run.x(store, &mut report),
            "reddit" => run.reddit(&mut report),
            _ => run.sitemaps(store, &mut report),
        };
        if report.skipped.is_some() {
            reports.push(report);
            continue;
        }
        if report.requests == 0 {
            continue;
        }
        report.accepted = findings.len();
        report.new_stories = store.add(&findings, now);
        match report.last_success {
            Some(at) => {
                store.last_success.insert(name.to_string(), at);
            }
            None => report.last_success = store.last_success.get(*name).copied(),
        }
        reports.push(report);
    }
    store.prune(now);
    store.last_discover_at = Some(now);
    reports
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Canned(HashMap<&'static str, Result<String, FetchError>>);

    impl Fetch for Canned {
        fn get(&self, url: &str, _query: &[(&str, String)]) -> Result<String, FetchError> {
            self.0.get(url).cloned().unwrap_or(Err(FetchError {
                status: Some(404),
                message: String::new(),
            }))
        }
    }

    const RSS: &str = r#"<?xml version="1.0"?><rss version="2.0"><channel><title>x</title>
        <item><title>OpenAI ships GPT-7 &amp;amp; more - The Verge</title>
          <link>https://theverge.com/a?utm=1</link>
          <pubDate>Sat, 07 Nov 2026 10:00:00 GMT</pubDate>
          <source url="https://theverge.com">The Verge</source>
          <description><![CDATA[<p>The <b>new</b> model&#8217;s out</p>]]></description></item>
        <item><title>Bakery wins prize</title><link>https://x.com/b</link>
          <pubDate>Sat, 07 Nov 2026 10:00:00 GMT</pubDate></item>
        <item><title>Old Claude news</title><link>https://x.com/c</link>
          <pubDate>Sat, 01 Nov 2026 10:00:00 GMT</pubDate></item>
        </channel></rss>"#;

    const ATOM: &str = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>t</title>
        <entry><title type="html">Claude Code adds hooks</title>
          <link rel="replies" href="https://e.com/comments"/>
          <link href="https://e.com/post"/>
          <updated>2026-11-07T09:00:00Z</updated><summary>Short</summary></entry></feed>"#;

    #[test]
    fn feeds_parse_rss_and_atom() {
        let rss = parse_feed(RSS).unwrap();
        assert_eq!(rss.len(), 3);
        assert_eq!(rss[0].title, "OpenAI ships GPT-7 & more - The Verge");
        assert_eq!(rss[0].publisher.as_deref(), Some("The Verge"));
        assert_eq!(rss[0].summary.as_deref(), Some("The new model’s out"));
        let atom = parse_feed(ATOM).unwrap();
        assert_eq!(atom[0].url, "https://e.com/post");
        assert!(atom[0].date.is_some());
        assert!(parse_feed("<html><body/></html>").is_err());
        assert_eq!(
            parse_sitemap("<urlset><url><loc> https://a.com/news/x </loc></url></urlset>").unwrap(),
            ["https://a.com/news/x"]
        );
        assert!(parse_sitemap("<rss/>").is_err());
    }

    fn profile_with_feeds(urls: &[&str]) -> Compiled {
        let mut p: super::super::profile::Profile =
            serde_json::from_str(super::super::profile::BUILT_IN).unwrap();
        let mut feeds = p.sources.feeds.clone().unwrap();
        feeds.urls = urls.iter().map(|u| u.to_string()).collect();
        p.sources = super::super::profile::Sources {
            feeds: Some(feeds),
            sitemaps: p.sources.sitemaps.clone(),
            ..Default::default()
        };
        if let Some(s) = p.sources.sitemaps.as_mut() {
            s.sites.truncate(1);
        }
        Compiled::new(p).unwrap()
    }

    #[test]
    fn a_failing_funnel_says_why_and_the_rest_still_count() {
        let now = DateTime::parse_from_rfc3339("2026-11-07T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut answers = HashMap::new();
        answers.insert("https://feed.example/rss", Ok(RSS.to_string()));
        answers.insert(
            "https://limited.example/rss",
            Err(FetchError {
                status: Some(429),
                message: String::new(),
            }),
        );
        answers.insert("https://broken.example/rss", Ok("not xml at all".into()));
        answers.insert(
            "https://www.anthropic.com/sitemap.xml",
            Ok("<urlset><url><loc>https://www.anthropic.com/news/a</loc></url></urlset>".into()),
        );
        let web = Canned(answers);
        let profile = profile_with_feeds(&[
            "https://feed.example/rss",
            "https://limited.example/rss",
            "https://broken.example/rss",
        ]);
        let mut store = Store::default();
        let reports = discover(&web, &profile, &mut store, now, None, &|_, _| {});
        let feeds = reports.iter().find(|r| r.name == "feeds").unwrap();
        // Only the on-subject, recent story is kept, under its publisher.
        assert_eq!((feeds.accepted, feeds.new_stories), (1, 1));
        assert_eq!(feeds.state(), "partial");
        assert!(feeds.issues.iter().any(|i| i.contains("rate limit")));
        assert!(feeds.issues.iter().any(|i| i.contains("could not parse")));
        let story = &store.topics[0];
        assert_eq!(story.title, "OpenAI ships GPT-7 & more");
        assert_eq!(story.signals[0].source, "feed:The Verge");
        assert_eq!(story.key, "url:https://theverge.com/a");
        // The first sitemap run only learns what exists; the next finds the new page.
        let sitemap = reports.iter().find(|r| r.name == "sitemaps").unwrap();
        assert_eq!(sitemap.accepted, 0);
        let mut answers = HashMap::new();
        answers.insert(
            "https://www.anthropic.com/sitemap.xml",
            Ok(
                "<urlset><url><loc>https://www.anthropic.com/news/a</loc></url>\
                <url><loc>https://www.anthropic.com/news/new-model</loc></url>\
                <url><loc>https://www.anthropic.com/careers/x</loc></url></urlset>"
                    .into(),
            ),
        );
        let reports = discover(
            &Canned(answers),
            &profile,
            &mut store,
            now,
            None,
            &|_, _| {},
        );
        let sitemap = reports.iter().find(|r| r.name == "sitemaps").unwrap();
        assert_eq!(sitemap.accepted, 1);
        assert!(store
            .topics
            .iter()
            .any(|t| t.title == "Anthropic: new model"));
        // A funnel that fetched nothing this time keeps its last success.
        let feeds = reports.iter().find(|r| r.name == "feeds").unwrap();
        assert_eq!(feeds.state(), "error");
        assert_eq!(feeds.last_success, Some(now));
    }

    #[test]
    fn entities_and_html_become_plain_text() {
        assert_eq!(unescape("a &amp;amp; b &#39;c&#x27;"), "a &amp; b 'c'");
        assert_eq!(
            html_to_text("<p>One&nbsp;<i>two</i></p>\n three"),
            "One two three"
        );
        assert_eq!(unescape("AT&T stays"), "AT&T stays");
        assert_eq!(unescape("&nbsp; Українські &amp;"), "  Українські &");
        assert_eq!(unescape("&Укр"), "&Укр");
    }
}

#[cfg(test)]
mod live {
    /// The real web, into a scratch folder: `cargo test --lib live_discovery -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_discovery() {
        let dir = tempfile::tempdir().unwrap();
        let profile = super::super::profile::load(dir.path()).unwrap();
        let web = super::Web::new().unwrap();
        let mut store = super::Store::default();
        // X and Reddit run when TREG_API_KEY is set (billed, about a cent).
        let treg = std::env::var("TREG_API_KEY")
            .ok()
            .and_then(|k| super::super::treg::Web::new(k).ok());
        let started = std::time::Instant::now();
        let reports = super::discover(
            &web,
            &profile,
            &mut store,
            chrono::Utc::now(),
            treg.as_ref().map(|t| t as &dyn super::super::treg::Call),
            &|_, _| {},
        );
        for r in &reports {
            println!("{}", r.line());
        }
        println!("{} stories in {:?}", store.topics.len(), started.elapsed());
        let ideas = super::super::ideas::rank(
            &profile,
            &store,
            &[],
            chrono::Utc::now(),
            3,
            8,
            super::super::Format::Long,
        );
        for i in ideas {
            println!(
                "{:?} {:.2} {} | {} outlets | {:?}",
                i.lane,
                i.score,
                i.title,
                i.outlets.len(),
                i.hooks
            );
        }
    }
}

#[cfg(test)]
mod x_tests {
    use super::*;
    use crate::faceless::treg::{Call, Reply};
    use parking_lot::Mutex;

    /// Answers every X search with the same posts, and counts the searches.
    struct FakeX {
        posts: Value,
        queries: Mutex<Vec<String>>,
        cost: u64,
    }

    impl Call for FakeX {
        fn call(&self, endpoint: &str, body: &Value) -> Result<Reply, String> {
            self.queries.lock().push(format!(
                "{endpoint} {}",
                body["q"].as_str().unwrap_or_default()
            ));
            Ok(Reply {
                output: if endpoint.contains("x.search") {
                    serde_json::json!({ "posts": self.posts })
                } else {
                    serde_json::json!({ "rows": [] })
                },
                cost_micro: self.cost,
                served_by: "fake".into(),
            })
        }
    }

    fn post(
        id: &str,
        author: &str,
        likes: f64,
        followers: f64,
        text: &str,
        hours: i64,
        now: DateTime<Utc>,
    ) -> Value {
        serde_json::json!({
            "id": id, "authorUsername": author, "likeCount": likes, "authorFollowers": followers,
            "text": text, "createdUtc": (now - Duration::hours(hours)).timestamp(), "isReply": false
        })
    }

    fn profile() -> Compiled {
        let mut p: super::super::profile::Profile =
            serde_json::from_str(super::super::profile::BUILT_IN).unwrap();
        let treg = p.sources.treg.clone();
        p.sources = super::super::profile::Sources {
            treg,
            ..Default::default()
        };
        if let Some(t) = p.sources.treg.as_mut() {
            t.subreddits.clear();
        }
        Compiled::new(p).unwrap()
    }

    #[test]
    fn accounts_are_searched_in_batches_and_posts_filtered() {
        let batches = from_batches(
            &(0..60).map(|n| format!("handle{n}")).collect::<Vec<_>>(),
            420,
        );
        assert!(
            batches.len() >= 3 && batches.iter().all(|b| b.len() <= 430),
            "{batches:?}"
        );
        let now = Utc::now();
        let p = profile();
        let posts = serde_json::json!([
            post(
                "1",
                "OpenAI",
                900.0,
                4e6,
                "Introducing Sora 3 for everyone today",
                2,
                now
            ),
            post(
                "2",
                "elonmusk",
                50000.0,
                2e8,
                "Rockets are cool and so is the weather today",
                2,
                now
            ),
            post(
                "3",
                "someone",
                5000.0,
                900.0,
                "Claude Code leaked keys in a new exploit report",
                2,
                now
            ),
            post(
                "4",
                "bigaccount",
                5000.0,
                90000.0,
                "Claude Code leaked keys in a new exploit report",
                2,
                now
            ),
            post(
                "5",
                "kimmonismus",
                150.0,
                157000.0,
                "#Gemini horoscope for today, stars align",
                2,
                now
            ),
            post(
                "6",
                "rowancheung",
                300.0,
                603000.0,
                "OpenAI just shipped GPT-7 with a new agent mode",
                60,
                now
            ),
        ]);
        let fake = FakeX {
            posts,
            queries: Mutex::new(Vec::new()),
            cost: 600,
        };
        let mut store = Store::default();
        let reports = discover(&FailWeb, &p, &mut store, now, Some(&fake), &|_, _| {});
        let x = reports.iter().find(|r| r.name == "x").unwrap();
        let keys: Vec<String> = store.topics.iter().map(|t| t.key.clone()).collect();
        // A lab's post passes on its own; Musk off subject, a small unknown author, a hashtag
        // horoscope and a post older than the lookback do not.
        assert!(keys.contains(&"x:1".to_string()), "{keys:?}");
        assert!(keys.contains(&"x:4".to_string()), "{keys:?}");
        for dropped in ["x:2", "x:3", "x:5", "x:6"] {
            assert!(
                !keys.contains(&dropped.to_string()),
                "{dropped} kept: {keys:?}"
            );
        }
        let queries = fake.queries.lock().clone();
        assert!(queries.iter().all(|q| q.contains("since:")), "{queries:?}");
        assert!(queries
            .iter()
            .any(|q| q.contains("from:OpenAI") && q.contains("min_faves:100")));
        assert!(x.cost_micro > 0 && store.accounts.contains_key("x:@openai"));
        assert_eq!(
            store
                .topics
                .iter()
                .find(|t| t.key == "x:1")
                .unwrap()
                .signals[0]
                .source,
            "x:@OpenAI"
        );
    }

    #[test]
    fn a_creator_must_beat_their_usual_likes() {
        let now = Utc::now();
        let p = profile();
        let mut store = Store::default();
        let stats = store.accounts.entry("x:@mreflow".into()).or_default();
        for likes in [800.0, 900.0, 1000.0, 1100.0, 1200.0] {
            stats.remember(likes);
        }
        let posts = serde_json::json!([
            post(
                "a",
                "mreflow",
                400.0,
                121000.0,
                "OpenAI ships GPT-7 with agents for everyone",
                1,
                now
            ),
            post(
                "b",
                "mreflow",
                3000.0,
                121000.0,
                "Anthropic ships Claude 6 with a new memory",
                1,
                now
            ),
        ]);
        let fake = FakeX {
            posts,
            queries: Mutex::new(Vec::new()),
            cost: 0,
        };
        discover(&FailWeb, &p, &mut store, now, Some(&fake), &|_, _| {});
        let keys: Vec<String> = store.topics.iter().map(|t| t.key.clone()).collect();
        assert_eq!(keys, ["x:b"], "{keys:?}");
    }

    #[test]
    fn a_search_stops_paying_at_its_guard() {
        let now = Utc::now();
        let p = profile();
        let fake = FakeX {
            posts: serde_json::json!([]),
            queries: Mutex::new(Vec::new()),
            cost: 60_000,
        };
        let reports = discover(
            &FailWeb,
            &p,
            &mut Store::default(),
            now,
            Some(&fake),
            &|_, _| {},
        );
        // $0.10 guard at $0.06 a call: two calls, then it stops.
        assert_eq!(fake.queries.lock().len(), 2);
        let x = reports.iter().find(|r| r.name == "x").unwrap();
        assert!(
            x.issues.iter().any(|i| i.contains("guard")),
            "{:?}",
            x.issues
        );
    }

    /// The free sources are off in these profiles; nothing reaches the web.
    struct FailWeb;
    impl Fetch for FailWeb {
        fn get(&self, _url: &str, _query: &[(&str, String)]) -> Result<String, FetchError> {
            Err(FetchError {
                status: Some(500),
                message: String::new(),
            })
        }
    }
}
