//! The watchlist (8 Nov): which X accounts, YouTube channels, TikTok and Instagram creators the
//! research follows. Edits go into the user's copy of the channel profile
//! (`<config>/AeroEdits/faceless/profile.json`); the built-in one stays as shipped.
//!
//! Two ways to grow it, both paid through Treg and both proposals the user approves:
//! - `suggest_x`: who the tracked accounts follow, ranked by how many of them follow each;
//! - `find_creators`: who posts the most-watched TikTok videos or reels on the channel's topics.
use super::profile::{Channel, Compiled, Profile, BUILT_IN};
use super::stories::Store;
use super::treg::{self, Call};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

const FILE: &str = "profile.json";

/// The profile as stored: the user's copy, else the built-in one.
pub fn current(config: &Path) -> Result<Profile, String> {
    let path = config.join(super::DIR).join(FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("{} is not a valid profile: {e}", path.display())),
        Err(_) => serde_json::from_str(BUILT_IN).map_err(|e| e.to_string()),
    }
}

/// Saves the user's copy (checked by compiling it first).
pub fn save(config: &Path, profile: &Profile) -> Result<(), String> {
    Compiled::new(profile.clone())?;
    crate::files::save_json(&config.join(super::DIR).join(FILE), profile)
}

fn clean(handle: &str) -> Result<String, String> {
    let h = handle.trim().trim_start_matches('@').to_string();
    if h.is_empty()
        || h.len() > 64
        || !h
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
    {
        return Err(format!("Not a handle: {handle}"));
    }
    Ok(h)
}

/// `x`, `youtube`, `tiktok` or `instagram`.
fn channels_of<'a>(
    t: &'a mut super::profile::TregSources,
    platform: &str,
) -> Result<&'a mut Vec<Channel>, String> {
    match platform {
        "youtube" => Ok(&mut t.youtube_channels),
        "tiktok" => Ok(&mut t.tiktok_creators),
        "instagram" => Ok(&mut t.instagram_creators),
        other => Err(format!("No watchlist for {other}")),
    }
}

/// Adds an account. On X it joins `group` (made with the creators' floor if new).
pub fn add(
    config: &Path,
    platform: &str,
    handle: &str,
    group: Option<&str>,
    name: &str,
    followers: f64,
) -> Result<(), String> {
    let handle = clean(handle)?;
    let mut profile = current(config)?;
    let t = profile
        .sources
        .treg
        .as_mut()
        .ok_or("The profile has no Treg sources")?;
    if platform == "x" {
        if t.group_of(&handle).is_some() {
            return Err(format!("@{handle} is already tracked"));
        }
        let group = group.unwrap_or("creators");
        match t.watch.iter_mut().find(|g| g.group == group) {
            Some(g) => g.handles.push(handle),
            None => t.watch.push(super::profile::WatchGroup {
                group: group.into(),
                min_likes: 200,
                relative: true,
                primary: false,
                handles: vec![handle],
            }),
        }
    } else {
        let list = channels_of(t, platform)?;
        let at = format!("@{handle}");
        if list.iter().any(|c| c.handle.eq_ignore_ascii_case(&at)) {
            return Err(format!("{at} is already watched"));
        }
        list.push(Channel {
            handle: at,
            name: name.into(),
            followers,
        });
    }
    save(config, &profile)
}

pub fn remove(config: &Path, platform: &str, handle: &str) -> Result<(), String> {
    let handle = clean(handle)?;
    let mut profile = current(config)?;
    let t = profile
        .sources
        .treg
        .as_mut()
        .ok_or("The profile has no Treg sources")?;
    let before;
    let after;
    if platform == "x" {
        before = t.watch.iter().map(|g| g.handles.len()).sum::<usize>();
        for g in &mut t.watch {
            g.handles.retain(|h| !h.eq_ignore_ascii_case(&handle));
        }
        after = t.watch.iter().map(|g| g.handles.len()).sum::<usize>();
    } else {
        let list = channels_of(t, platform)?;
        before = list.len();
        list.retain(|c| {
            !c.handle
                .trim_start_matches('@')
                .eq_ignore_ascii_case(&handle)
        });
        after = list.len();
    }
    if before == after {
        return Err(format!("@{handle} is not on the {platform} watchlist"));
    }
    save(config, &profile)
}

/// A proposed account.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub platform: String,
    pub handle: String,
    pub name: String,
    pub followers: f64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bio: String,
    /// X: the tracked accounts that follow it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub followed_by: Vec<String>,
    /// TikTok and Instagram: its posts found, their views together, and one example.
    pub posts: usize,
    pub views: f64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub example: String,
}

/// Who the tracked accounts follow (the news, research, security and leaders groups, `seeds`
/// of them), ranked by how many follow each; already tracked, small (under the profile's
/// `minFollowers`) and off-subject accounts are left out.
pub fn suggest_x(profile: &Compiled, treg: &dyn Call, seeds: usize) -> (Vec<Candidate>, u64) {
    let Some(t) = profile.profile.sources.treg.as_ref() else {
        return (Vec::new(), 0);
    };
    let order = ["news", "research", "security", "leaders", "creators"];
    let mut seed_handles: Vec<String> = Vec::new();
    for group in order {
        if let Some(g) = t.watch.iter().find(|g| g.group == group) {
            seed_handles.extend(g.handles.iter().cloned());
        }
    }
    seed_handles.truncate(seeds.clamp(1, 20));
    let answers: Vec<(String, Result<treg::Reply, String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = seed_handles
            .iter()
            .map(|h| {
                scope.spawn(move || {
                    (
                        h.clone(),
                        treg.call(
                            "anyapi.twitter.following",
                            &serde_json::json!({ "username": h, "limit": 200 }),
                        ),
                    )
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    let mut cost = 0;
    let mut found: BTreeMap<String, Candidate> = BTreeMap::new();
    for (seed, answer) in answers {
        let Ok(reply) = answer else { continue };
        cost += reply.cost_micro;
        let users = reply.output["data"]["items"]
            .as_array()
            .or(reply.output["users"].as_array())
            .or(reply.output["following"].as_array())
            .cloned()
            .unwrap_or_default();
        for u in users {
            let Some(handle) = u["username"].as_str().or(u["screen_name"].as_str()) else {
                continue;
            };
            let entry = found
                .entry(handle.to_lowercase())
                .or_insert_with(|| Candidate {
                    platform: "x".into(),
                    handle: handle.into(),
                    name: u["name"].as_str().unwrap_or_default().into(),
                    followers: treg::count(&u["followers"])
                        .or_else(|| treg::count(&u["followers_count"]))
                        .unwrap_or(0.0),
                    bio: u["bio"]
                        .as_str()
                        .or(u["description"].as_str())
                        .unwrap_or_default()
                        .chars()
                        .take(160)
                        .collect(),
                    followed_by: Vec::new(),
                    posts: 0,
                    views: 0.0,
                    example: String::new(),
                });
            entry.followed_by.push(seed.clone());
        }
    }
    let mut out: Vec<Candidate> = found
        .into_values()
        .filter(|c| {
            t.group_of(&c.handle).is_none()
                && c.followed_by.len() >= 2
                && c.followers >= t.min_followers
                && profile.on_subject(&format!("{} {}", c.name, c.bio))
        })
        .collect();
    out.sort_by(|a, b| {
        b.followed_by
            .len()
            .cmp(&a.followed_by.len())
            .then(b.followers.total_cmp(&a.followers))
    });
    out.truncate(25);
    (out, cost)
}

/// Who posts the most-watched TikTok videos (or Instagram reels) on these searches, already
/// watched creators left out.
pub fn find_creators(
    profile: &Compiled,
    treg: &dyn Call,
    platform: &str,
    queries: &[String],
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(Vec<Candidate>, u64), String> {
    let t = profile
        .profile
        .sources
        .treg
        .as_ref()
        .ok_or("The profile has no Treg sources")?;
    let watched: Vec<String> = match platform {
        "tiktok" => &t.tiktok_creators,
        "instagram" => &t.instagram_creators,
        other => {
            return Err(format!(
                "Find creators works on tiktok or instagram, not {other}"
            ))
        }
    }
    .iter()
    .map(|c| c.handle.trim_start_matches('@').to_lowercase())
    .collect();
    let results: Vec<Result<(Vec<treg::Reel>, u64), String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = queries
            .iter()
            .map(|q| {
                scope.spawn(move || match platform {
                    "tiktok" => treg::tiktok_search(treg, q, now),
                    _ => treg::instagram_reels(treg, q, now),
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err("crashed".into())))
            .collect()
    });
    let mut cost = 0;
    let mut by_author: BTreeMap<String, Candidate> = BTreeMap::new();
    for result in results {
        let Ok((reels, c)) = result else { continue };
        cost += c;
        for r in reels {
            if r.author.is_empty()
                || watched.contains(&r.author.to_lowercase())
                || !profile.on_subject(&r.caption)
            {
                continue;
            }
            let entry = by_author
                .entry(r.author.to_lowercase())
                .or_insert_with(|| Candidate {
                    platform: platform.into(),
                    handle: r.author.clone(),
                    name: String::new(),
                    followers: r.author_followers,
                    bio: String::new(),
                    followed_by: Vec::new(),
                    posts: 0,
                    views: 0.0,
                    example: r.url.clone(),
                });
            entry.posts += 1;
            entry.views += r.views;
            entry.followers = entry.followers.max(r.author_followers);
        }
    }
    let mut out: Vec<Candidate> = by_author
        .into_values()
        .filter(|c| c.posts >= 2 || c.views >= 50_000.0)
        .collect();
    // Who posts on these topics again and again first; one viral video says less.
    out.sort_by(|a, b| b.posts.cmp(&a.posts).then(b.views.total_cmp(&a.views)));
    out.truncate(25);
    Ok((out, cost))
}

/// One watched account as the sources panel shows it.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Watched {
    pub platform: String,
    pub handle: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub group: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub followers: f64,
    /// Its posts that passed the filters, and how often one was in a top 10.
    pub kept: u32,
    pub top10: u32,
    /// What it usually gets (likes on X, views on YouTube), once known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usual: Option<f64>,
}

/// Everything watched, with what the searches learned about each.
pub fn list(profile: &Compiled, store: &Store) -> Vec<Watched> {
    let Some(t) = profile.profile.sources.treg.as_ref() else {
        return Vec::new();
    };
    let stats = |key: String| store.accounts.get(&key).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for g in &t.watch {
        for h in &g.handles {
            let s = stats(format!("x:@{}", h.to_lowercase()));
            out.push(Watched {
                platform: "x".into(),
                handle: h.clone(),
                group: g.group.clone(),
                name: String::new(),
                followers: 0.0,
                kept: s.kept,
                top10: s.top10,
                usual: s.usual(),
            });
        }
    }
    for (platform, list) in [
        ("youtube", &t.youtube_channels),
        ("tiktok", &t.tiktok_creators),
        ("instagram", &t.instagram_creators),
    ] {
        for c in list {
            let key = match platform {
                "youtube" => format!("yt:{}", c.handle.to_lowercase()),
                other => format!("{other}:{}", c.handle.to_lowercase()),
            };
            let s = stats(key);
            let usual = {
                let mut v = s.recent.clone();
                v.sort_by(f64::total_cmp);
                (!v.is_empty()).then(|| v[v.len() / 2])
            };
            out.push(Watched {
                platform: platform.into(),
                handle: c.handle.clone(),
                group: String::new(),
                name: c.name.clone(),
                followers: c.followers,
                kept: s.kept,
                top10: s.top10,
                usual,
            });
        }
    }
    out
}

/// After a search: each tracked X account in a story of the long or short top 10 gets a hit.
pub fn count_top10(store: &mut Store, ideas_ids: &[Vec<u64>]) {
    let mut hits: Vec<String> = Vec::new();
    for ids in ideas_ids {
        for id in ids {
            if let Some(t) = store.topics.iter().find(|t| t.id == *id) {
                for s in &t.signals {
                    if let Some(handle) = s.source.strip_prefix("x:") {
                        let key = format!("x:{}", handle.to_lowercase());
                        if !hits.contains(&key) {
                            hits.push(key);
                        }
                    }
                }
            }
        }
    }
    for key in hits {
        store.accounts.entry(key).or_default().top10 += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_go_into_the_users_copy() {
        let dir = tempfile::tempdir().unwrap();
        add(dir.path(), "x", "@NewVoice", Some("news"), "", 0.0).unwrap();
        assert!(add(dir.path(), "x", "newvoice", None, "", 0.0).is_err());
        let p = current(dir.path()).unwrap();
        let t = p.sources.treg.as_ref().unwrap();
        assert_eq!(t.group_of("newvoice").unwrap().group, "news");
        add(
            dir.path(),
            "tiktok",
            "aicreator",
            None,
            "AI Creator",
            90000.0,
        )
        .unwrap();
        assert_eq!(
            current(dir.path())
                .unwrap()
                .sources
                .treg
                .unwrap()
                .tiktok_creators
                .len(),
            1
        );
        remove(dir.path(), "x", "NEWVOICE").unwrap();
        assert!(remove(dir.path(), "x", "newvoice").is_err());
        assert!(add(dir.path(), "x", "bad handle!", None, "", 0.0).is_err());
        // The built-in profile is untouched.
        let built_in: Profile = serde_json::from_str(BUILT_IN).unwrap();
        assert!(built_in.sources.treg.unwrap().tiktok_creators.is_empty());
    }

    struct Following;
    impl Call for Following {
        fn call(&self, _e: &str, body: &serde_json::Value) -> Result<treg::Reply, String> {
            let who = body["username"].as_str().unwrap_or_default();
            let items = serde_json::json!([
                { "username": "AIResearcher", "name": "AI Researcher", "followers": 80000, "bio": "LLM evals and agents" },
                { "username": "Chef", "name": "Chef", "followers": 900000, "bio": "Cooking" },
                { "username": "emollick", "name": "Ethan", "followers": 400000, "bio": "AI" },
                { "username": format!("only{who}"), "name": "x", "followers": 90000, "bio": "AI agents" }
            ]);
            Ok(treg::Reply {
                output: serde_json::json!({ "data": { "items": items } }),
                cost_micro: 450,
                served_by: "fake".into(),
            })
        }
    }

    #[test]
    fn suggestions_are_followed_by_several_on_subject_and_new() {
        let p = Compiled::new(serde_json::from_str(BUILT_IN).unwrap()).unwrap();
        let (found, cost) = suggest_x(&p, &Following, 3);
        assert_eq!(cost, 1350);
        let handles: Vec<&str> = found.iter().map(|c| c.handle.as_str()).collect();
        // Followed by all three seeds, on subject, not tracked yet; the chef, a tracked
        // account and accounts only one seed follows are left out.
        assert_eq!(handles, ["AIResearcher"], "{found:?}");
        assert_eq!(found[0].followed_by.len(), 3);
    }
}
