//! A channel profile (FCL decision 5): where to look for stories and what the channel cares
//! about. Aero AI news is built in; a user copy at `<config>/AeroEdits/faceless/profile.json`
//! replaces it.
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const BUILT_IN: &str = include_str!("aero-ai-news.json");
const FILE: &str = "profile.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    /// What the channel covers, for the agent picking stories.
    #[serde(default)]
    pub about: String,
    /// Words that make a headline the channel's subject (alternatives of a regex).
    pub subject_terms: String,
    /// Looser words that count in terse repository descriptions.
    #[serde(default)]
    pub repo_terms: String,
    pub hooks: Vec<Hook>,
    /// Formats that rank lower (a regex).
    #[serde(default)]
    pub low_value: String,
    /// Words left out when comparing headlines, space separated.
    #[serde(default)]
    pub stopwords: String,
    /// Publishers that republish wire copy: leads, never separate outlets.
    #[serde(default)]
    pub syndicators: Vec<String>,
    /// Publishers whose own announcement is the story.
    #[serde(default)]
    pub primary_sources: Vec<String>,
    /// Sites whose pages are the story's own source (labs, makers). On GitHub, Hugging Face and
    /// arXiv only the story's own repository, model or paper counts.
    #[serde(default)]
    pub primary_domains: Vec<String>,
    pub sources: Sources,
    /// The channel's narration voice (default: Will on Eleven v4 Turbo at 1.2x).
    #[serde(default)]
    pub voice: super::voice::VoiceSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hook {
    pub name: String,
    pub weight: f64,
    pub pattern: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sources {
    #[serde(default)]
    pub hackernews: Option<HackerNews>,
    #[serde(default)]
    pub openrouter_models: Option<Lookback>,
    #[serde(default)]
    pub github_releases: Option<GithubReleases>,
    #[serde(default)]
    pub feeds: Option<Feeds>,
    #[serde(default)]
    pub huggingface: Option<HuggingFace>,
    #[serde(default)]
    pub github_viral: Option<GithubViral>,
    #[serde(default)]
    pub papers: Option<Papers>,
    #[serde(default)]
    pub sitemaps: Option<Sitemaps>,
    /// X and Reddit through Treg; needs a Treg key, and each search is billed (about a cent
    /// for the built-in set).
    #[serde(default)]
    pub treg: Option<TregSources>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TregSources {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub lookback_hours: i64,
    /// X keyword searches in X's own syntax (`min_faves:`, `-filter:replies`, `lang:`); their
    /// authors must have `minFollowers` followers. Tracked accounts are in `watch` instead.
    #[serde(default)]
    pub x_searches: Vec<String>,
    /// Subreddits whose top posts of the day are read.
    #[serde(default)]
    pub subreddits: Vec<String>,
    /// How many of each subreddit's top posts of the day count (their order is the filter).
    #[serde(default = "five")]
    pub subreddit_top: usize,
    /// Accounts tracked on X, in groups with their own likes floor.
    #[serde(default)]
    pub watch: Vec<WatchGroup>,
    /// Authors of keyword-search posts need this many followers.
    #[serde(default = "five_thousand")]
    pub min_followers: f64,
    /// YouTube channels watched for what they cover and how it does (demand checks).
    #[serde(default)]
    pub youtube_channels: Vec<Channel>,
    /// TikTok and Instagram creators watched for shorts' demand (found with "Find creators").
    #[serde(default)]
    pub tiktok_creators: Vec<Channel>,
    #[serde(default)]
    pub instagram_creators: Vec<Channel>,
    /// A guard: one search stops its paid calls past this many dollars.
    #[serde(default = "ten_cents")]
    pub max_cost_per_search: f64,
}

fn five() -> usize {
    5
}

fn five_thousand() -> f64 {
    5000.0
}

fn ten_cents() -> f64 {
    0.10
}

/// Tracked X accounts of one kind.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchGroup {
    /// `labs`, `leaders`, `research`, `news`, `security` or `creators`.
    pub group: String,
    /// The likes a post needs (in the search itself: below it nothing is paid for).
    pub min_likes: u32,
    /// Also needs to beat the account's own usual likes (creators post a lot).
    #[serde(default)]
    pub relative: bool,
    /// Its posts are the maker's own word: a primary source.
    #[serde(default)]
    pub primary: bool,
    pub handles: Vec<String>,
}

/// A channel or creator on a video platform.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    /// `@handle` as the platform writes it.
    pub handle: String,
    #[serde(default)]
    pub name: String,
    /// Followers or subscribers when last looked up.
    #[serde(default)]
    pub followers: f64,
}

impl TregSources {
    /// The group a tracked X account is in.
    pub fn group_of(&self, handle: &str) -> Option<&WatchGroup> {
        let handle = handle.trim_start_matches('@').to_lowercase();
        self.watch
            .iter()
            .find(|g| g.handles.iter().any(|h| h.to_lowercase() == handle))
    }
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HackerNews {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub min_points: u32,
    pub lookback_hours: i64,
    pub queries: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lookback {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub lookback_hours: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubReleases {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub lookback_hours: i64,
    pub repos: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feeds {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub lookback_hours: i64,
    pub urls: Vec<String>,
    /// Hosts that only publish on the channel's subject: every entry counts.
    #[serde(default)]
    pub subject_only_hosts: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HuggingFace {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub lookback_days: i64,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubViral {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub lookback_days: i64,
    pub min_stars: u32,
    pub limit: u32,
    pub non_english_min_stars: u32,
    #[serde(default)]
    pub trending_feeds: Vec<String>,
    pub max_star_lookups: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Papers {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub lookback_days: i64,
    pub min_upvotes: u32,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sitemaps {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub sites: Vec<SitemapSite>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SitemapSite {
    pub name: String,
    pub url: String,
    pub include: Vec<String>,
}

/// A profile with its patterns compiled.
pub struct Compiled {
    pub profile: Profile,
    pub subject: Regex,
    pub repo: Option<Regex>,
    pub hooks: Vec<(String, f64, Regex)>,
    pub low_value: Option<Regex>,
    pub stopwords: std::collections::HashSet<String>,
}

fn words_regex(alternatives: &str) -> Result<Regex, String> {
    RegexBuilder::new(&format!(r"\b(?:{alternatives})\b"))
        .case_insensitive(true)
        .build()
        .map_err(|e| format!("A pattern in the profile is not valid: {e}"))
}

impl Compiled {
    pub fn new(profile: Profile) -> Result<Self, String> {
        let subject = words_regex(&profile.subject_terms)?;
        let repo = (!profile.repo_terms.trim().is_empty())
            .then(|| words_regex(&profile.repo_terms))
            .transpose()?;
        let hooks = profile
            .hooks
            .iter()
            .map(|h| Ok((h.name.clone(), h.weight, words_regex(&h.pattern)?)))
            .collect::<Result<Vec<_>, String>>()?;
        let low_value = (!profile.low_value.trim().is_empty())
            .then(|| {
                RegexBuilder::new(&profile.low_value)
                    .case_insensitive(true)
                    .build()
                    .map_err(|e| format!("The profile's lowValue pattern is not valid: {e}"))
            })
            .transpose()?;
        let stopwords = profile
            .stopwords
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        Ok(Self {
            profile,
            subject,
            repo,
            hooks,
            low_value,
            stopwords,
        })
    }

    pub fn on_subject(&self, text: &str) -> bool {
        self.subject.is_match(text)
    }

    /// Repository descriptions are terse, so looser words count there.
    pub fn on_subject_repo(&self, text: &str) -> bool {
        self.on_subject(text) || self.repo.as_ref().is_some_and(|r| r.is_match(text))
    }
}

/// The profile in use: the user's copy in `config`'s faceless folder, else the built-in one.
pub fn load(config: &Path) -> Result<Compiled, String> {
    let path = config.join(super::DIR).join(FILE);
    let profile: Profile = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("{} is not a valid profile: {e}", path.display()))?,
        Err(_) => serde_json::from_str(BUILT_IN).map_err(|e| e.to_string())?,
    };
    Compiled::new(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_profile_compiles_and_knows_its_subject() {
        let dir = tempfile::tempdir().unwrap();
        let compiled = load(dir.path()).unwrap();
        assert_eq!(compiled.profile.name, "Aero AI news");
        assert!(compiled.on_subject("Anthropic ships Claude Code 3"));
        assert!(compiled.on_subject("New GPT-6 model"));
        assert!(!compiled.on_subject("Local bakery wins prize"));
        assert!(compiled.on_subject_repo("a tool for agents"));
        assert_eq!(compiled.hooks.len(), 14);
        // A user's copy replaces it.
        std::fs::create_dir_all(dir.path().join(crate::faceless::DIR)).unwrap();
        let mut mine: Profile = serde_json::from_str(BUILT_IN).unwrap();
        mine.name = "Cooking".into();
        mine.subject_terms = "recipe|baking".into();
        std::fs::write(
            dir.path().join(crate::faceless::DIR).join(FILE),
            serde_json::to_string(&mine).unwrap(),
        )
        .unwrap();
        let compiled = load(dir.path()).unwrap();
        assert!(compiled.on_subject("Local baking prize"));
    }
}
