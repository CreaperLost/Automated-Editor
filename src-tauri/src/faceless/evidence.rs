//! Research with checked facts (FCL-2), ported from the explainer's `research.py` and `web.py`.
//! The agent searches and proposes facts; the app fetches each page itself and keeps a fact only
//! when its quote is found word for word in that page and every number in the statement is in
//! the quote. No model is involved in the check.
//!
//! The dossier is the production's `faceless/dossier.json`; fetched page text is kept beside it
//! in `faceless/sources/` so facts can be checked again.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};

const MIN_QUOTE_CHARS: usize = 15;
const MAX_BYTES: usize = 3_000_000;
/// Facts a story needs before it can carry its own video.
pub const READY_MIN_FACTS: usize = 6;
/// Shared text that marks one source as repeating another rather than reporting on its own.
const SYNDICATED_QUOTE_CHARS: usize = 120;
/// Share of a fact's key words another origin must contain to state it too.
const CORROBORATION_WORDS: f64 = 0.6;
/// Hosts where anyone can publish: only the story's own repository, model or paper is primary.
const SHARED_PLATFORMS: [&str; 3] = ["github.com", "huggingface.co", "arxiv.org"];
const TRACKING_PARAMS: [&str; 7] = ["hl", "m", "trk", "ref", "source", "fbclid", "gclid"];
const SKIP_TAGS: [&str; 10] = [
    "script", "style", "noscript", "svg", "nav", "footer", "aside", "form", "template", "iframe",
];
const BLOCK_TAGS: [&str; 15] = [
    "p", "div", "li", "br", "tr", "section", "article", "h1", "h2", "h3", "h4", "h5", "h6", "pre",
    "td",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceStatus {
    Fetched,
    Unreadable,
    Paywalled,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub url: String,
    pub status: SourceStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub fetched_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_on: Option<String>,
    /// Who read it, when not the app itself: Treg, for a PDF, a Google News link or a page
    /// built by scripts (with the page it ended at and the cost).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    /// Correction and update notices on the page at research time, so publishing can tell a
    /// new correction from one that was already there.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<String>,
    #[serde(default)]
    pub chars: usize,
}

/// A fact the agent proposes: a statement and the exact words of the page that support it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactInput {
    /// What the video may say, in plain words. Every number in it must be in the quote.
    pub statement: String,
    /// The page's words, copied exactly (15 characters or more).
    pub quote: String,
    /// The fetched page it comes from.
    pub url: String,
    /// `fact`, `number`, `quote` (someone's words), `claim` (someone asserts it) or `date`.
    #[serde(default)]
    pub kind: Option<String>,
    /// Who or what the claim covers and doesn't: "in the lab's own tests", "US only".
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub uncertainty: Option<String>,
    /// The strongest reason it might not hold.
    #[serde(default)]
    pub counterpoint: Option<String>,
    /// For a number: its unit, base, period and what it counts.
    #[serde(default)]
    pub number_context: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Fact {
    pub id: String,
    #[serde(flatten)]
    pub input: FactInput,
    pub domain: String,
    pub primary: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_date: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rejected {
    pub statement: String,
    pub url: String,
    pub reason: String,
}

/// The agent's reading of the evidence: what the story is, for the script.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Brief {
    /// The question a viewer would ask.
    #[serde(default)]
    pub viewer_question: String,
    #[serde(default)]
    pub why_it_matters: String,
    /// The story's turns, each citing fact ids like [f3].
    #[serde(default)]
    pub beats: Vec<String>,
    #[serde(default)]
    pub counterpoints: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
    #[serde(default)]
    pub visual_ideas: Vec<String>,
    /// Why this story suits a long video or a short.
    #[serde(default)]
    pub format_fit: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Grade {
    /// Enough facts from independent origins: a long video or a short.
    Ready,
    /// Too little: a line in a news round-up at most.
    Thin,
    Empty,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Dossier {
    pub topic: String,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub facts: Vec<Fact>,
    #[serde(default)]
    pub rejected: Vec<Rejected>,
    #[serde(default)]
    pub brief: Brief,
    /// Sources grouped into independent origins: one site, or pages repeating each other.
    #[serde(default)]
    pub origins: Vec<Vec<String>>,
    /// Per fact id: how many other origins state it too.
    #[serde(default)]
    pub corroboration: BTreeMap<String, usize>,
    pub grade: Grade,
    #[serde(default)]
    pub next_fact: usize,
}

impl Dossier {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.into(),
            sources: Vec::new(),
            facts: Vec::new(),
            rejected: Vec::new(),
            brief: Brief::default(),
            origins: Vec::new(),
            corroboration: BTreeMap::new(),
            grade: Grade::Empty,
            next_fact: 0,
        }
    }
}

// ----------------------------------------------------------------------------------- text

/// Quotes, dashes and spaces made plain, lowercased, spaces collapsed: what quote matching
/// compares.
pub fn normalize(text: &str) -> String {
    let plain: String = text
        .chars()
        .map(|c| match c {
            '\u{2019}' | '\u{2018}' | '\u{201B}' | '\u{2032}' => '\'',
            '\u{201C}' | '\u{201D}' | '\u{201F}' | '\u{2033}' => '"',
            '\u{2013}' | '\u{2014}' | '\u{2212}' | '\u{2010}' | '\u{2011}' => '-',
            '\u{00A0}' | '\u{2009}' | '\u{202F}' | '\u{2007}' => ' ',
            '\u{2026}' => '.',
            _ => c,
        })
        .collect();
    plain
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The quantities in `text`, each written one way so that equal ones compare equal: "1,200"
/// and "1.2 thousand" are "1200", "$6.34 million" is "6.34 million", "40 percent" is "40%",
/// "-3" keeps its sign (GAP-026). Parts of words like "gpt-5", "covid-19" or "h100" are not
/// quantities. Only the number, its sign, its scale and whether it is a percentage or points
/// are compared: not its unit, currency or what it counts.
pub fn numbers(text: &str) -> Vec<String> {
    static QUANTITY: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = QUANTITY.get_or_init(|| {
        regex::Regex::new(
            r"(?i)(\d+(?:[.,]\d+)*)(?:\s*(%|percent\b|per cent\b|percentage points?\b|thousand\b|million\b|billion\b|trillion\b|k\b|bn\b))?",
        )
        .unwrap()
    });
    let mut out = Vec::new();
    for found in re.captures_iter(text) {
        let digits = found.get(1).unwrap();
        let mut before = text[..digits.start()].chars().rev();
        let negative = match before.next() {
            Some(c) if c.is_alphanumeric() || c == '_' || c == '.' || c == ',' => continue,
            Some('-') => match before.next() {
                // "gpt-5": part of a name.
                Some(c) if c.is_alphabetic() || c == '_' => continue,
                // "10-20": a range.
                Some(c) if c.is_ascii_digit() => false,
                _ => true,
            },
            _ => false,
        };
        let Some(mut value) = quantity(digits.as_str()) else {
            continue;
        };
        let suffix = found
            .get(2)
            .map(|m| m.as_str().to_lowercase())
            .unwrap_or_default();
        let kind = match suffix.as_str() {
            "%" | "percent" | "per cent" => "%",
            s if s.starts_with("percentage point") => " points",
            "thousand" | "k" => {
                value *= 1e3;
                ""
            }
            "million" => {
                value *= 1e6;
                ""
            }
            "billion" | "bn" => {
                value *= 1e9;
                ""
            }
            "trillion" => {
                value *= 1e12;
                ""
            }
            _ => "",
        };
        if negative {
            value = -value;
        }
        out.push(format!("{}{kind}", show_quantity(value)));
    }
    out
}

/// "1,200" is 1200 and "2.5" two and a half; "2,5" (one or two digits after the comma) is also
/// two and a half, and "1.200.000" (several dots) is grouped.
fn quantity(digits: &str) -> Option<f64> {
    let dots = digits.matches('.').count();
    let commas = digits.matches(',').count();
    let plain = if dots > 1 {
        digits.replace('.', "").replace(',', ".")
    } else if commas == 1
        && dots == 0
        && digits
            .rsplit(',')
            .next()
            .is_some_and(|tail| tail.len() != 3)
    {
        digits.replace(',', ".")
    } else {
        digits.replace(',', "")
    };
    plain.parse().ok()
}

/// A quantity written one way: millions and up by their scale word, smaller ones plainly.
fn show_quantity(value: f64) -> String {
    let trim = |x: f64| {
        let text = format!("{x:.6}");
        let text = text.trim_end_matches('0').trim_end_matches('.');
        if text == "-0" {
            "0".to_string()
        } else {
            text.to_string()
        }
    };
    for (scale, name) in [(1e12, "trillion"), (1e9, "billion"), (1e6, "million")] {
        if value.abs() >= scale {
            return format!("{} {name}", trim(value / scale));
        }
    }
    trim(value)
}

/// Why a fact fails the check, or `None` when it passes.
pub fn verify(statement: &str, quote: &str, page: &str) -> Option<String> {
    if quote.trim().chars().count() < MIN_QUOTE_CHARS {
        return Some("quote too short".into());
    }
    let quote = normalize(quote);
    if !normalize(page).contains(&quote) {
        return Some("quote not found in the page".into());
    }
    let quoted = numbers(&quote);
    for number in numbers(&normalize(statement)) {
        if !quoted.contains(&number) {
            return Some(format!("number {number} is not in the quote"));
        }
    }
    None
}

fn notice_regex() -> regex::Regex {
    regex::Regex::new(
        r"(?:\b(?:correction|clarification|editor's note|update|updated)\s*(?:\(|:|-)|\bhas been (?:corrected|updated|retracted|amended|withdrawn)\b|\bretract(?:ed|ion)\b)",
    )
    .unwrap()
}

/// Correction and update notices in a page, each with a little context.
pub fn notices(text: &str) -> Vec<String> {
    let plain = normalize(text);
    let mut found: Vec<String> = notice_regex()
        .find_iter(&plain)
        .map(|m| {
            plain[m.start()..]
                .chars()
                .take(80)
                .collect::<String>()
                .trim()
                .to_string()
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

/// A page's readable text: scripts, menus and footers out, a line per block.
pub fn html_text(html: &str) -> (String, Option<String>) {
    let mut out = String::with_capacity(html.len() / 2);
    let mut skipping = 0usize;
    let mut published = None;
    let mut rest = html;
    while let Some(at) = rest.find('<') {
        if skipping == 0 {
            out.push_str(&rest[..at]);
        }
        rest = &rest[at..];
        // Comments end at -->.
        if rest.starts_with("<!--") {
            rest = rest.find("-->").map_or("", |end| &rest[end + 3..]);
            continue;
        }
        let Some(end) = rest.find('>') else {
            break;
        };
        let tag = &rest[1..end];
        rest = &rest[end + 1..];
        let closing = tag.starts_with('/');
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        if name == "meta" && tag.contains("article:published_time") {
            if let Some(content) = tag.split("content=").nth(1) {
                let value: String = content
                    .trim_start_matches(['"', '\''])
                    .chars()
                    .take(10)
                    .collect();
                if chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d").is_ok() {
                    published = Some(value);
                }
            }
        }
        if SKIP_TAGS.contains(&name.as_str()) {
            if closing {
                skipping = skipping.saturating_sub(1);
            } else if !tag.ends_with('/') {
                skipping += 1;
            }
        } else if BLOCK_TAGS.contains(&name.as_str()) && skipping == 0 {
            out.push('\n');
        }
    }
    if skipping == 0 {
        out.push_str(rest);
    }
    let text = super::discover::unescape(&out)
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (text, published)
}

// ----------------------------------------------------------------------------------- urls

/// One address per page: tracking parameters, fragments and trailing slashes off.
pub fn canonical_url(raw: &str) -> String {
    let Ok(mut url) = url::Url::parse(raw.trim()) else {
        return raw.trim().to_string();
    };
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| {
            let k = k.to_lowercase();
            !TRACKING_PARAMS.contains(&k.as_str()) && !k.starts_with("utm_")
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(kept);
    }
    url.set_fragment(None);
    let path = url.path().trim_end_matches('/').to_string();
    url.set_path(if path.is_empty() { "/" } else { &path });
    url.to_string().trim_end_matches('/').to_string()
}

pub fn domain(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_lowercase())
        })
        .unwrap_or_default()
}

/// The repository, model, dataset or paper a shared-platform URL points at.
fn work(url: &str) -> Option<Vec<String>> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.trim_start_matches("www.").to_lowercase();
    let platform = SHARED_PLATFORMS
        .iter()
        .find(|p| host == **p || host.ends_with(&format!(".{p}")))?;
    let path: Vec<String> = parsed
        .path_segments()?
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect();
    let mut key = vec![platform.to_string()];
    match *platform {
        "arxiv.org" => {
            if path.len() >= 2 && ["abs", "pdf", "html"].contains(&path[0].as_str()) {
                let id = path[1].trim_end_matches(".pdf");
                let id = regex::Regex::new(r"v\d+$")
                    .unwrap()
                    .replace(id, "")
                    .to_string();
                key.push(id);
                return Some(key);
            }
            None
        }
        "huggingface.co"
            if path
                .first()
                .is_some_and(|p| ["datasets", "spaces", "papers"].contains(&p.as_str())) =>
        {
            if path.len() >= 3 || path[0] == "papers" {
                key.extend(path.into_iter().take(3));
                Some(key)
            } else {
                None
            }
        }
        _ if path.len() >= 2 => {
            key.extend(path.into_iter().take(2));
            Some(key)
        }
        _ => None,
    }
}

/// A source is primary for this story: the maker's own site, or on a shared platform the
/// story's own repository, model or paper (not anyone's page there).
pub fn is_primary(url: &str, story_links: &[String], primary_domains: &[String]) -> bool {
    let host = domain(url);
    if !primary_domains
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
    {
        return false;
    }
    if !SHARED_PLATFORMS
        .iter()
        .any(|p| host == *p || host.ends_with(&format!(".{p}")))
    {
        return true;
    }
    work(url).is_some_and(|w| story_links.iter().any(|l| work(l).as_ref() == Some(&w)))
}

// ----------------------------------------------------------------------------------- fetch

/// Whether an address is on the public internet (not this computer or a private network).
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || o[0] == 0
                || (o[0] == 100 && (64..128).contains(&o[1]))
                || (o[0] == 192 && o[1] == 0 && o[2] == 0)
                || (o[0] == 198 && (18..20).contains(&o[1]))
                || o[0] >= 240)
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public(IpAddr::V4(v4));
            }
            let s = v6.segments();
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (s[0] & 0xfe00) == 0xfc00
                || (s[0] & 0xffc0) == 0xfe80
                || s[0] == 0x2001 && s[1] == 0x0db8)
        }
    }
}

/// Refuses anything but http(s) on a host whose every address is public: these URLs come from
/// the web and must never reach this computer's own network.
pub fn check_public(raw: &str) -> Result<url::Url, String> {
    resolve_public(raw).map(|(url, _)| url)
}

/// [`check_public`], with the addresses it checked: the page is fetched from these and no
/// others, so a second lookup cannot send it somewhere private (GAP-013).
pub fn resolve_public(raw: &str) -> Result<(url::Url, Vec<std::net::SocketAddr>), String> {
    let url = url::Url::parse(raw).map_err(|_| format!("not a web address: {raw}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("not a web address: {raw}"));
    }
    let host = url.host_str().ok_or("no host")?.to_string();
    let port = url.port_or_known_default().unwrap_or(443);
    // An IPv6 literal comes bracketed in the URL.
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    let addresses: Vec<_> = match (bare, port).to_socket_addrs() {
        Ok(found) => found.collect(),
        // Behind a proxy with no DNS of its own (a cloud VM, some offices) the proxy looks the
        // name up, and it is what keeps the request off the local network. An address
        // written into the URL is still checked.
        Err(_) if bare.parse::<std::net::IpAddr>().is_err() && proxy_set(url.scheme()) => {
            return Ok((url, Vec::new()))
        }
        Err(_) => return Err(format!("cannot resolve {host}")),
    };
    if addresses.is_empty() || addresses.iter().any(|a| !is_public(a.ip())) {
        return Err(format!("{host} is not on the public internet"));
    }
    Ok((url, addresses))
}

/// A proxy for `scheme` is set in the environment (reqwest uses the same variables).
fn proxy_set(scheme: &str) -> bool {
    let names: &[&str] = match scheme {
        "https" => &["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"],
        _ => &["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"],
    };
    names
        .iter()
        .any(|name| std::env::var(name).is_ok_and(|v| !v.trim().is_empty()))
}

/// A fetched page's text, or why it couldn't be read.
pub enum Page {
    Text {
        text: String,
        published_on: Option<String>,
        /// Who read it, when not the app itself (Treg, for a PDF or a Google News link).
        via: Option<String>,
    },
    Unreadable(String),
    Paywalled(String),
}

/// Something that fetches a page; tests answer from fixtures.
pub trait PageFetch: Sync {
    fn page(&self, url: &str) -> Page;
}

pub struct WebPages;

impl WebPages {
    pub fn new() -> Result<Self, String> {
        Self::client(None)?;
        Ok(Self)
    }

    /// The client for one request; with `pinned`, it connects only to those addresses of that
    /// host. A proxy set for this computer still carries the request and looks the host up
    /// itself: the proxy is then what keeps it off the local network.
    fn client(
        pinned: Option<(&str, &[std::net::SocketAddr])>,
    ) -> Result<reqwest::blocking::Client, String> {
        let mut builder = reqwest::blocking::Client::builder()
            // Redirects are followed by hand, so each hop passes the address check.
            .redirect(reqwest::redirect::Policy::none())
            // Says what it is; a site that refuses it is reported unreadable, and the
            // agent finds the same words elsewhere. Never dressed up as a browser.
            .user_agent(concat!(
                "AeroEdits/",
                env!("CARGO_PKG_VERSION"),
                " (fact check of a cited page)"
            ))
            .timeout(std::time::Duration::from_secs(30));
        if let Some((host, addresses)) = pinned {
            builder = builder.resolve_to_addrs(host, addresses);
        }
        builder.build().map_err(|e| e.to_string())
    }
}

impl PageFetch for WebPages {
    fn page(&self, url: &str) -> Page {
        let mut current = url.to_string();
        for _ in 0..6 {
            let (checked, addresses) = match resolve_public(&current) {
                Ok(found) => found,
                Err(e) => return Page::Unreadable(e),
            };
            let host = checked.host_str().unwrap_or_default().to_string();
            // No addresses: the proxy looks the host up (see `resolve_public`).
            let pinned = (!addresses.is_empty()).then_some((host.as_str(), addresses.as_slice()));
            let client = match Self::client(pinned) {
                Ok(client) => client,
                Err(e) => return Page::Unreadable(e),
            };
            let response = match client.get(checked.clone()).send() {
                Ok(r) => r,
                Err(e) if e.is_timeout() => return Page::Unreadable("timed out".into()),
                Err(_) => return Page::Unreadable("could not connect".into()),
            };
            let status = response.status();
            if status.is_redirection() {
                let Some(next) = response
                    .headers()
                    .get("location")
                    .and_then(|l| l.to_str().ok())
                    .and_then(|l| checked.join(l).ok())
                else {
                    return Page::Unreadable("a redirect without a location".into());
                };
                let host = next.host_str().unwrap_or_default().to_lowercase();
                if host.starts_with("consent.") || host == "guce.yahoo.com" {
                    return Page::Unreadable(format!("sent to a cookie consent page ({host})"));
                }
                current = next.to_string();
                continue;
            }
            if status.as_u16() == 402 {
                return Page::Paywalled("payment required".into());
            }
            if !status.is_success() {
                return Page::Unreadable(format!("HTTP {}", status.as_u16()));
            }
            let kind = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_lowercase();
            let mut body = Vec::new();
            use std::io::Read;
            let mut limited = response.take(MAX_BYTES as u64 + 1);
            if limited.read_to_end(&mut body).is_err() {
                return Page::Unreadable("could not read the page".into());
            }
            return page_from(&current, &kind, &body);
        }
        Page::Unreadable("too many redirects".into())
    }
}

/// The app's own reader, with Treg reading what it can't: a PDF, a Google News link (Treg
/// follows it to the publisher), or a page whose text is built by scripts. A site that refuses
/// automated readers stays unreadable: the agent finds the same words elsewhere.
pub struct WithTreg {
    pub own: WebPages,
    pub treg: super::treg::Web,
}

/// Whether Treg should try a page the app couldn't read.
pub fn treg_can_help(reason: &str) -> bool {
    reason.starts_with("a PDF")
        || reason.starts_with("sent to a cookie consent page")
        || reason.starts_with("almost no text")
}

impl PageFetch for WithTreg {
    fn page(&self, url: &str) -> Page {
        match self.own.page(url) {
            Page::Unreadable(reason) if treg_can_help(&reason) => {
                match super::treg::extract(&self.treg, url) {
                    Ok((text, final_url, cost)) if text.chars().count() >= 200 => Page::Text {
                        text,
                        published_on: None,
                        via: Some(format!(
                            "Treg{}, {}",
                            final_url.map_or(String::new(), |u| format!(" from {u}")),
                            super::treg::dollars(cost)
                        )),
                    },
                    Ok(_) => {
                        Page::Unreadable(format!("{reason}; Treg found almost no text either"))
                    }
                    Err(e) => Page::Unreadable(format!("{reason}; Treg: {e}")),
                }
            }
            other => other,
        }
    }
}

/// The text of a page's body, by its type.
pub fn page_from(url: &str, kind: &str, body: &[u8]) -> Page {
    if kind.contains("application/pdf") || body.starts_with(b"%PDF-") {
        return Page::Unreadable(
            "a PDF: reading PDFs is not built yet; find the same words on a web page".into(),
        );
    }
    if !kind.is_empty()
        && !["html", "text", "xml", "json"]
            .iter()
            .any(|t| kind.contains(t))
    {
        return Page::Unreadable(format!(
            "not a page ({})",
            kind.chars().take(40).collect::<String>()
        ));
    }
    let text = String::from_utf8_lossy(&body[..body.len().min(MAX_BYTES)]).to_string();
    let html = kind.contains("html") || kind.is_empty();
    if html {
        let paywall =
            regex::Regex::new(r#"(?i)"isAccessibleForFree"\s*:\s*(?:false|"false")"#).unwrap();
        if paywall.is_match(&text) {
            return Page::Paywalled("the page says it is for subscribers".into());
        }
    }
    if url.to_lowercase().ends_with(".pdf") {
        return Page::Unreadable("a PDF address that sent something else".into());
    }
    let (text, published_on) = if html { html_text(&text) } else { (text, None) };
    if text.chars().count() < 200 {
        return Page::Unreadable("almost no text on the page (it may need a browser)".into());
    }
    Page::Text {
        text,
        published_on,
        via: None,
    }
}

// ----------------------------------------------------------------------------------- dossier

pub fn dossier_path(root: &Path) -> PathBuf {
    root.join(super::DIR).join("dossier.json")
}

fn sources_dir(root: &Path) -> PathBuf {
    root.join(super::DIR).join("sources")
}

fn text_path(root: &Path, url: &str) -> PathBuf {
    let digest = Sha256::digest(url.as_bytes());
    let name: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    sources_dir(root).join(format!("{name}.txt"))
}

/// A fetched page's text, as kept.
pub fn source_text(root: &Path, url: &str) -> Option<String> {
    std::fs::read_to_string(text_path(root, &canonical_url(url))).ok()
}

pub fn load(root: &Path) -> Option<Dossier> {
    crate::files::load_json(&dossier_path(root))
}

/// Groups sources that share a site or repeat each other's text into one origin, so copies of
/// one announcement count once.
pub fn origins(dossier: &Dossier, texts: &BTreeMap<String, String>) -> Vec<Vec<String>> {
    let mut urls: Vec<String> = Vec::new();
    for f in &dossier.facts {
        if !urls.contains(&f.input.url) {
            urls.push(f.input.url.clone());
        }
    }
    let mut parent: Vec<usize> = (0..urls.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let norm: Vec<String> = urls
        .iter()
        .map(|u| texts.get(u).map(|t| normalize(t)).unwrap_or_default())
        .collect();
    let quotes: Vec<Vec<String>> = urls
        .iter()
        .map(|u| {
            dossier
                .facts
                .iter()
                .filter(|f| f.input.url == *u)
                .map(|f| normalize(&f.input.quote))
                .collect()
        })
        .collect();
    for a in 0..urls.len() {
        for b in a + 1..urls.len() {
            let shared: Vec<&String> = quotes[a]
                .iter()
                .filter(|q| norm[b].contains(q.as_str()))
                .chain(quotes[b].iter().filter(|q| norm[a].contains(q.as_str())))
                .collect();
            if domain(&urls[a]) == domain(&urls[b])
                || shared.len() >= 2
                || shared
                    .iter()
                    .any(|q| q.chars().count() >= SYNDICATED_QUOTE_CHARS)
            {
                let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
                parent[rb] = ra;
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for (i, url) in urls.iter().enumerate() {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(url.clone());
    }
    groups.into_values().collect()
}

fn key_words(text: &str) -> Vec<String> {
    const COMMON: &str = "that this with from have will were been their they them than then into \
        about after before which while would could should there these those what when where also \
        only more most some such over said says report reports reported according";
    let re = regex::Regex::new(r"[a-z][a-z'-]{3,}").unwrap();
    let mut words: Vec<String> = re
        .find_iter(&normalize(text))
        .map(|m| m.as_str().to_string())
        .filter(|w| !COMMON.split_whitespace().any(|c| c == w))
        .collect();
    words.sort();
    words.dedup();
    words
}

/// For each fact, how many other origins state it too: every number of its quote and most of
/// its statement's key words appear there.
pub fn corroboration(
    dossier: &Dossier,
    groups: &[Vec<String>],
    texts: &BTreeMap<String, String>,
) -> BTreeMap<String, usize> {
    let group_texts: Vec<String> = groups
        .iter()
        .map(|g| {
            g.iter()
                .filter_map(|u| texts.get(u))
                .map(|t| normalize(t))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    dossier
        .facts
        .iter()
        .map(|fact| {
            let own = groups.iter().position(|g| g.contains(&fact.input.url));
            let nums: Vec<String> = numbers(&normalize(&fact.input.quote))
                .into_iter()
                .map(|n| n.replace(',', ""))
                .collect();
            let words = key_words(&fact.input.statement);
            let others = group_texts
                .iter()
                .enumerate()
                .filter(|(i, text)| {
                    Some(*i) != own && !text.is_empty() && !words.is_empty() && {
                        let flat = text.replace(',', "");
                        let found = numbers(&flat);
                        nums.iter().all(|n| found.contains(n))
                            && words.iter().filter(|w| text.contains(w.as_str())).count() as f64
                                / words.len() as f64
                                >= CORROBORATION_WORDS
                    }
                })
                .count();
            (fact.id.clone(), others)
        })
        .collect()
}

/// `ready` with enough facts from three independent origins, or a primary source and one more.
pub fn grade(dossier: &Dossier, independent: usize) -> Grade {
    if dossier.facts.is_empty() {
        return Grade::Empty;
    }
    let primary = dossier.facts.iter().any(|f| f.primary);
    if dossier.facts.len() >= READY_MIN_FACTS && (independent >= 3 || (primary && independent >= 2))
    {
        Grade::Ready
    } else {
        Grade::Thin
    }
}

fn all_texts(root: &Path, dossier: &Dossier) -> BTreeMap<String, String> {
    dossier
        .sources
        .iter()
        .filter(|s| s.status == SourceStatus::Fetched)
        .filter_map(|s| source_text(root, &s.url).map(|t| (s.url.clone(), t)))
        .collect()
}

/// Works out origins, corroboration and the grade, saves the dossier and records the stage.
pub fn save(root: &Path, dossier: &mut Dossier) -> Result<(), String> {
    let texts = all_texts(root, dossier);
    dossier.origins = origins(dossier, &texts);
    dossier.corroboration = corroboration(dossier, &dossier.origins, &texts);
    dossier.grade = grade(dossier, dossier.origins.len());
    crate::files::save_json(&dossier_path(root), dossier)?;
    // Research is done when the story can carry a video; a thin one needs more first.
    if dossier.grade == Grade::Ready {
        super::record_stage(root, "research")?;
    } else {
        super::clear_stage(root, "research")?;
    }
    Ok(())
}

fn open(root: &Path) -> Result<(super::Production, Dossier), String> {
    let production = super::load(root).ok_or("The open project is not a faceless video")?;
    let dossier = load(root).unwrap_or_else(|| Dossier::new(&production.topic));
    Ok((production, dossier))
}

/// Fetches pages (in parallel) and keeps their text. Pages already fetched are kept unless
/// `again`. Returns each page's line.
pub fn fetch(
    root: &Path,
    web: &dyn PageFetch,
    urls: &[String],
    again: bool,
) -> Result<Vec<Source>, String> {
    let (_, mut dossier) = open(root)?;
    let wanted: Vec<String> = urls
        .iter()
        .map(|u| canonical_url(u))
        .filter(|u| !u.is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|u| {
            again
                || !dossier
                    .sources
                    .iter()
                    .any(|s| s.url == *u && s.status == SourceStatus::Fetched)
        })
        .take(20)
        .collect();
    let pages: Vec<(String, Page)> = std::thread::scope(|scope| {
        let handles: Vec<_> = wanted
            .iter()
            .map(|url| scope.spawn(move || (url.clone(), web.page(url))))
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    std::fs::create_dir_all(sources_dir(root)).map_err(|e| e.to_string())?;
    let now = Utc::now();
    let mut out = Vec::new();
    for (url, page) in pages {
        let source = match page {
            Page::Text {
                text,
                published_on,
                via,
            } => {
                crate::files::write_atomic(&text_path(root, &url), text.as_bytes())?;
                Source {
                    url: url.clone(),
                    status: SourceStatus::Fetched,
                    reason: None,
                    fetched_at: now,
                    published_on,
                    via,
                    notices: notices(&text),
                    chars: text.chars().count(),
                }
            }
            Page::Unreadable(reason) => Source {
                url: url.clone(),
                status: SourceStatus::Unreadable,
                reason: Some(reason),
                fetched_at: now,
                published_on: None,
                via: None,
                notices: Vec::new(),
                chars: 0,
            },
            Page::Paywalled(reason) => Source {
                url: url.clone(),
                status: SourceStatus::Paywalled,
                reason: Some(reason),
                fetched_at: now,
                published_on: None,
                via: None,
                notices: Vec::new(),
                chars: 0,
            },
        };
        // A page that can't be read again keeps what was read before: unavailable is not
        // corrected. Its line still says why it failed.
        let earlier_read = dossier
            .sources
            .iter()
            .any(|s| s.url == url && s.status == SourceStatus::Fetched);
        if source.status == SourceStatus::Fetched || !earlier_read {
            dossier.sources.retain(|s| s.url != url);
            dossier.sources.push(source.clone());
        }
        if source.status == SourceStatus::Fetched {
            recheck_facts(root, &mut dossier, &url);
        }
        out.push(source);
    }
    save(root, &mut dossier)?;
    Ok(out)
}

/// Checks the facts from `url` against its page as now saved: one whose quote or numbers are
/// gone is taken out and listed as refused, so a corrected page never keeps a stale fact
/// (GAP-033). A script citing it then fails its check until the scene is rewritten.
fn recheck_facts(root: &Path, dossier: &mut Dossier, url: &str) {
    let text = source_text(root, url).unwrap_or_default();
    let published_on = dossier
        .sources
        .iter()
        .find(|s| s.url == url)
        .and_then(|s| s.published_on.clone());
    let mut gone = Vec::new();
    dossier.facts.retain_mut(|fact| {
        if fact.input.url != url {
            return true;
        }
        match verify(&fact.input.statement, &fact.input.quote, &text) {
            None => {
                fact.source_date = published_on.clone();
                true
            }
            Some(reason) => {
                gone.push(Rejected {
                    statement: fact.input.statement.clone(),
                    url: url.to_string(),
                    reason: format!(
                        "{} was taken out when the page was fetched again: {reason}",
                        fact.id
                    ),
                });
                false
            }
        }
    });
    dossier.rejected.extend(gone);
    let keep_from = dossier.rejected.len().saturating_sub(100);
    dossier.rejected.drain(..keep_from);
}

/// What adding facts did: the ids kept and the facts refused with why.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Added {
    pub kept: Vec<String>,
    pub refused: Vec<Rejected>,
    pub grade: Grade,
    pub facts: usize,
    pub origins: usize,
}

/// Checks each proposed fact against its fetched page and keeps the ones that pass.
pub fn add_facts(
    root: &Path,
    facts: Vec<FactInput>,
    primary_domains: &[String],
) -> Result<Added, String> {
    let (production, mut dossier) = open(root)?;
    let mut kept = Vec::new();
    let mut refused = Vec::new();
    for mut input in facts {
        input.url = canonical_url(&input.url);
        let reason = match dossier.sources.iter().find(|s| s.url == input.url) {
            None => Some("the page was not fetched: fetch it first".to_string()),
            Some(s) if s.status != SourceStatus::Fetched => Some(format!(
                "the page could not be read ({})",
                s.reason.clone().unwrap_or_default()
            )),
            Some(_) => match source_text(root, &input.url) {
                None => Some("the page's text is missing: fetch it again".into()),
                Some(text) => verify(&input.statement, &input.quote, &text),
            },
        };
        if let Some(reason) = reason {
            refused.push(Rejected {
                statement: input.statement.clone(),
                url: input.url.clone(),
                reason,
            });
            continue;
        }
        if dossier.facts.iter().any(|f| {
            normalize(&f.input.quote) == normalize(&input.quote) && f.input.url == input.url
        }) {
            continue;
        }
        dossier.next_fact += 1;
        let id = format!("f{}", dossier.next_fact);
        let source_date = dossier
            .sources
            .iter()
            .find(|s| s.url == input.url)
            .and_then(|s| s.published_on.clone());
        dossier.facts.push(Fact {
            id: id.clone(),
            domain: domain(&input.url),
            primary: is_primary(&input.url, &production.links, primary_domains),
            source_date,
            input,
        });
        kept.push(id);
    }
    dossier.rejected.extend(refused.iter().cloned());
    let keep_from = dossier.rejected.len().saturating_sub(100);
    dossier.rejected.drain(..keep_from);
    save(root, &mut dossier)?;
    Ok(Added {
        kept,
        refused,
        grade: dossier.grade,
        facts: dossier.facts.len(),
        origins: dossier.origins.len(),
    })
}

pub fn remove_facts(root: &Path, ids: &[String]) -> Result<Dossier, String> {
    let (_, mut dossier) = open(root)?;
    dossier.facts.retain(|f| !ids.contains(&f.id));
    save(root, &mut dossier)?;
    Ok(dossier)
}

pub fn set_brief(root: &Path, brief: Brief) -> Result<Dossier, String> {
    let (_, mut dossier) = open(root)?;
    let known: Vec<&str> = dossier.facts.iter().map(|f| f.id.as_str()).collect();
    let cited = regex::Regex::new(r"\[(f\d+)\]").unwrap();
    for beat in &brief.beats {
        for c in cited.captures_iter(beat) {
            if !known.contains(&&c[1]) {
                return Err(format!(
                    "The brief cites {} but there is no such fact",
                    &c[1]
                ));
            }
        }
    }
    dossier.brief = brief;
    save(root, &mut dossier)?;
    Ok(dossier)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str =
        "<html><head><meta property=\"article:published_time\" content=\"2026-11-05T10:00:00Z\">\
        <script>var x = 'Binance lost 99 billion';</script></head><body><nav>Menu Home</nav>\
        <article><h1>Binance says hackers took funds</h1><p>Binance said on Tuesday that \
        $6.34 billion in assets were affected, its first estimate, after an installer shipped \
        malware to 1,200 employees.</p><p>Correction: an earlier version misstated the date. \
        The exchange added that customer funds were safe and that it would cover any losses \
        from its own reserves while the investigation continues with outside help.</p></article>\
        <footer>Copyright</footer></body></html>";

    #[test]
    fn page_text_keeps_the_article_and_drops_scripts_and_menus() {
        let (text, published) = html_text(PAGE);
        assert!(text.contains("$6.34 billion"));
        assert!(!text.contains("99 billion"));
        assert!(!text.contains("Menu Home"));
        assert!(!text.contains("Copyright"));
        assert_eq!(published.as_deref(), Some("2026-11-05"));
        assert_eq!(notices(&text).len(), 1);
    }

    #[test]
    fn a_fact_needs_its_quote_on_the_page_and_its_numbers_in_the_quote() {
        let (text, _) = html_text(PAGE);
        let quote = "$6.34 billion in assets were affected, its first estimate";
        assert_eq!(
            verify(
                "Binance put the affected assets at $6.34 billion at first.",
                quote,
                &text
            ),
            None
        );
        // A planted number.
        assert_eq!(
            verify("Binance lost $6.43 billion.", quote, &text).as_deref(),
            Some("number 6.43 billion is not in the quote")
        );
        // Words the page doesn't have, and curly quotes or spacing that it does.
        assert!(verify("x", "Binance confirmed $6.34 billion was stolen", &text).is_some());
        assert_eq!(
            verify(
                "1,200 staff got it",
                "shipped  malware to 1,200 employees",
                &text
            ),
            None
        );
        assert!(verify("x", "short", &text).is_some());
        // GPT-5 and H100 are names, not numbers.
        assert!(numbers("GPT-5 on H100s costs 40 dollars") == vec!["40"]);
    }

    /// A number keeps its sign, scale and kind: a percentage is not a count, a billion not a
    /// million, and a negative number is checked like any other (GAP-026).
    #[test]
    fn a_number_keeps_its_sign_scale_and_kind() {
        let quote = "The trial included 40 people and raised $6.34 million in 2024";
        let page = format!("Story. {quote}. More.");
        assert_eq!(verify("The trial included 40 people.", quote, &page), None);
        assert_eq!(
            verify("It raised 6,340,000 dollars.", quote, &page),
            None,
            "the same amount written out"
        );
        assert!(verify("The trial included 40 percent of people.", quote, &page).is_some());
        assert!(verify("The trial raised $6.34 billion.", quote, &page).is_some());
        assert_eq!(
            verify("The trial lost -999 dollars.", quote, &page).as_deref(),
            Some("number -999 is not in the quote")
        );
        assert_eq!(
            numbers("1,000 or 1k or 1 thousand"),
            ["1000", "1000", "1000"]
        );
        assert_eq!(
            numbers("rose 5% or 5 percentage points"),
            ["5%", "5 points"]
        );
        assert_eq!(
            numbers("from 10-20, down -3 (-4)"),
            ["10", "20", "-3", "-4"]
        );
        assert_eq!(
            numbers("covid-19 and 2,5 and 1.200.000"),
            ["2.5", "1.2 million"]
        );
    }

    #[test]
    fn copies_of_one_announcement_are_one_origin() {
        let mut d = Dossier::new("t");
        let long =
            "the company said the new model passes every safety test it was given before release";
        let fact = |id: &str, url: &str, quote: &str| Fact {
            id: id.into(),
            input: FactInput {
                statement: "It passed the tests".into(),
                quote: quote.into(),
                url: url.into(),
                kind: None,
                scope: None,
                uncertainty: None,
                counterpoint: None,
                number_context: None,
            },
            domain: domain(url),
            primary: false,
            source_date: None,
        };
        d.facts = vec![
            fact("f1", "https://a.com/1", long),
            fact("f2", "https://b.com/2", long),
            fact("f3", "https://a.com/3", "another part of the same site"),
            fact(
                "f4",
                "https://c.com/4",
                "independent reporting with its own words",
            ),
        ];
        let mut texts = BTreeMap::new();
        texts.insert("https://a.com/1".to_string(), long.to_string());
        texts.insert("https://b.com/2".to_string(), format!("copy: {long}"));
        texts.insert(
            "https://a.com/3".to_string(),
            "another part of the same site".into(),
        );
        texts.insert(
            "https://c.com/4".to_string(),
            "independent reporting with its own words; the new model passed tests".into(),
        );
        let groups = origins(&d, &texts);
        assert_eq!(groups.len(), 2, "{groups:?}");
        assert_eq!(grade(&d, groups.len()), Grade::Thin);
    }

    #[test]
    fn primary_on_a_shared_platform_is_only_the_storys_own_work() {
        let domains = vec!["github.com".to_string(), "openai.com".to_string()];
        let links = vec!["https://github.com/openai/codex".to_string()];
        assert!(is_primary("https://openai.com/index/x", &links, &domains));
        assert!(is_primary(
            "https://github.com/openai/codex/releases/tag/v1",
            &links,
            &domains
        ));
        assert!(!is_primary(
            "https://github.com/someone/fork",
            &links,
            &domains
        ));
        assert!(!is_primary("https://theverge.com/x", &links, &domains));
        assert_eq!(
            canonical_url("https://www.x.com/a/?utm_source=t&id=3#top"),
            "https://www.x.com/a?id=3"
        );
    }

    #[test]
    fn private_addresses_are_refused() {
        assert!(check_public("http://127.0.0.1/admin").is_err());
        assert!(check_public("http://192.168.1.1/").is_err());
        assert!(check_public("http://[::1]/").is_err());
        assert!(check_public("file:///C:/Windows").is_err());
        // The addresses checked are the ones fetched from (GAP-013).
        let (_, addresses) = resolve_public("http://1.1.1.1/").unwrap();
        assert_eq!(addresses, ["1.1.1.1:80".parse().unwrap()]);
        let (_, addresses) = resolve_public("https://[2606:4700::1111]/").unwrap();
        assert_eq!(addresses, ["[2606:4700::1111]:443".parse().unwrap()]);
        assert!(WebPages::client(Some(("example.com", &addresses))).is_ok());
        assert!(!is_public("10.0.0.5".parse().unwrap()));
        assert!(!is_public("100.64.1.1".parse().unwrap()));
        assert!(is_public("8.8.8.8".parse().unwrap()));
    }

    struct Pages;
    impl PageFetch for Pages {
        fn page(&self, url: &str) -> Page {
            if url.contains("paper.pdf") {
                return page_from(url, "application/pdf", b"%PDF-1.7");
            }
            match page_from(url, "text/html", PAGE.as_bytes()) {
                Page::Text {
                    text,
                    published_on,
                    via,
                } => Page::Text {
                    text: format!("{text}\nPage {url}"),
                    published_on,
                    via,
                },
                other => other,
            }
        }
    }

    /// A page that says something else when fetched again loses the facts it no longer
    /// supports; one that can't be read again keeps what was read before (GAP-033).
    #[test]
    fn a_corrected_page_drops_the_facts_it_no_longer_supports() {
        struct Changing(std::sync::Mutex<&'static str>);
        impl PageFetch for Changing {
            fn page(&self, _url: &str) -> Page {
                match *self.0.lock().unwrap() {
                    "" => Page::Unreadable("timed out".into()),
                    text => Page::Text {
                        text: text.into(),
                        published_on: None,
                        via: None,
                    },
                }
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let root =
            super::super::create(dir.path(), "Trial", super::super::Format::Long, &[]).unwrap();
        let url = vec!["https://a.com/trial".to_string()];
        let web = Changing(std::sync::Mutex::new(
            "The trial included 40 people and ended in May.",
        ));
        fetch(&root, &web, &url, false).unwrap();
        let fact = |statement: &str, quote: &str| FactInput {
            statement: statement.into(),
            quote: quote.into(),
            url: url[0].clone(),
            kind: None,
            scope: None,
            uncertainty: None,
            counterpoint: None,
            number_context: None,
        };
        let added = add_facts(
            &root,
            vec![
                fact("40 people took part.", "The trial included 40 people"),
                fact("It ended in May.", "and ended in May"),
            ],
            &[],
        )
        .unwrap();
        assert_eq!(added.kept.len(), 2);
        // Unavailable is not corrected: both facts and the saved page stay.
        *web.0.lock().unwrap() = "";
        let lines = fetch(&root, &web, &url, true).unwrap();
        assert_eq!(lines[0].status, SourceStatus::Unreadable);
        let dossier = load(&root).unwrap();
        assert_eq!(dossier.facts.len(), 2);
        assert_eq!(dossier.sources[0].status, SourceStatus::Fetched);
        // Corrected: the count changed, so that fact goes; the other still holds.
        *web.0.lock().unwrap() = "The trial included 38 people and ended in May.";
        fetch(&root, &web, &url, true).unwrap();
        let dossier = load(&root).unwrap();
        assert_eq!(
            dossier
                .facts
                .iter()
                .map(|f| f.input.statement.as_str())
                .collect::<Vec<_>>(),
            ["It ended in May."]
        );
        assert!(dossier
            .rejected
            .last()
            .unwrap()
            .reason
            .contains("taken out when the page was fetched again"));
    }

    #[test]
    fn research_keeps_checked_facts_and_grades_the_story() {
        let dir = tempfile::tempdir().unwrap();
        let root = super::super::create(
            dir.path(),
            "Binance hack",
            super::super::Format::Long,
            &["https://www.binance.com/en/blog/x".to_string()],
        )
        .unwrap();
        let urls: Vec<String> = [
            "https://a.com/1",
            "https://b.com/2?utm_source=x",
            "https://c.com/paper.pdf",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let fetched = fetch(&root, &Pages, &urls, false).unwrap();
        assert_eq!(fetched.len(), 3);
        let pdf = fetched.iter().find(|s| s.url.ends_with(".pdf")).unwrap();
        assert_eq!(pdf.status, SourceStatus::Unreadable);
        let input = |statement: &str, quote: &str, url: &str| FactInput {
            statement: statement.into(),
            quote: quote.into(),
            url: url.into(),
            kind: Some("number".into()),
            scope: None,
            uncertainty: None,
            counterpoint: None,
            number_context: None,
        };
        let added = add_facts(
            &root,
            vec![
                input(
                    "At first $6.34 billion was affected.",
                    "$6.34 billion in assets were affected",
                    "https://a.com/1",
                ),
                input(
                    "$6.43 billion was stolen.",
                    "$6.34 billion in assets were affected",
                    "https://b.com/2",
                ),
                input(
                    "Staff got malware.",
                    "shipped malware to 1,200 employees",
                    "https://c.com/paper.pdf",
                ),
                input(
                    "Staff got malware.",
                    "shipped malware to 1,200 employees",
                    "https://d.com/never",
                ),
            ],
            &[],
        )
        .unwrap();
        assert_eq!(added.kept, ["f1"]);
        assert_eq!(added.refused.len(), 3);
        assert!(added.refused[0].reason.contains("6.43"));
        assert!(added.refused[1].reason.contains("could not be read"));
        assert!(added.refused[2].reason.contains("not fetched"));
        assert_eq!(added.grade, Grade::Thin);
        let summary = super::super::summary(&root).unwrap();
        // A thin story isn't done researching.
        assert_eq!(summary.stages[0].state, super::super::StageState::Todo);
        // The brief may only cite facts that exist.
        let mut brief = Brief::default();
        brief.beats = vec!["The first estimate [f9]".into()];
        assert!(set_brief(&root, brief.clone()).is_err());
        brief.beats = vec!["The first estimate [f1]".into()];
        assert!(set_brief(&root, brief).is_ok());
        let d = remove_facts(&root, &["f1".into()]).unwrap();
        assert_eq!(d.grade, Grade::Empty);
    }
}

#[cfg(test)]
mod live {
    use super::*;

    /// Real pages: `cargo test --lib live_pages -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_pages() {
        let web = WebPages::new().unwrap();
        for url in [
            "https://simonwillison.net/",
            "https://www.anthropic.com/news",
            "https://arstechnica.com/ai/",
            "https://arxiv.org/pdf/1706.03762",
        ] {
            match web.page(url) {
                Page::Text {
                    text, published_on, ..
                } => {
                    println!(
                        "{url}: {} chars, published {published_on:?}: {}",
                        text.len(),
                        text.chars()
                            .take(120)
                            .collect::<String>()
                            .replace('\n', " | ")
                    )
                }
                Page::Unreadable(r) => println!("{url}: unreadable: {r}"),
                Page::Paywalled(r) => println!("{url}: paywalled: {r}"),
            }
        }
    }
}
