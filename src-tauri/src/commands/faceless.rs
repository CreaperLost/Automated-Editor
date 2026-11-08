//! The Faceless tab's commands (FCL-0): list productions, make one, and say where the open one
//! stands. The MCP `faceless` tool calls the same functions.
use super::AppState;
use crate::faceless::{self, Format, Made, ProductionSummary};
use crate::project::OpenedProject;
use serde::Serialize;

/// A production just made and opened.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacelessCreated {
    pub opened: OpenedProject,
    pub production: ProductionSummary,
    /// A video already made on the same topic, so the user can stop before repeating a story.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub already_made: Option<Made>,
}

/// Every production in the Faceless folder, newest first.
pub fn faceless_list_impl() -> Vec<ProductionSummary> {
    faceless::list(&faceless::home())
}

/// The videos already made.
pub fn faceless_made_impl() -> Vec<Made> {
    faceless::load_made(&crate::transcript::settings::config_dir())
}

/// One discovery at a time: two would race on the stories file.
static DISCOVERING: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

/// What a discovery did.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverResult {
    pub funnels: Vec<faceless::discover::FunnelReport>,
    pub stories: usize,
    pub new_stories: usize,
    /// What the paid sources (Treg) cost, in micro-dollars.
    pub cost_micro: u64,
}

/// Searches the web for stories with the channel profile's funnels (about a minute). Never runs
/// by itself. `progress` hears the funnel starting, as a fraction and its name.
pub fn faceless_discover_impl(progress: &dyn Fn(f64, &str)) -> Result<DiscoverResult, String> {
    let _running = DISCOVERING
        .try_lock()
        .ok_or("A story search is already running")?;
    let config = crate::transcript::settings::config_dir();
    let profile = faceless::profile::load(&config)?;
    let web = faceless::discover::Web::new()?;
    let mut store = faceless::stories::Store::load(&config);
    // X and Reddit come through Treg when a key is set.
    let treg = faceless::treg::Web::from_keychain();
    let funnels = faceless::discover::discover(
        &web,
        &profile,
        &mut store,
        chrono::Utc::now(),
        treg.as_ref().map(|t| t as &dyn faceless::treg::Call),
        &|index, name| {
            progress(
                index as f64 / faceless::discover::FUNNELS.len() as f64,
                name,
            )
        },
    );
    // Keep the ranking as it now stands, so the next one shows how stories moved.
    let made = faceless::load_made(&config);
    let snapshot = faceless::ideas::snapshot(&profile, &store, &made, chrono::Utc::now());
    // Each tracked account in this top 10 gets a hit, for the watchlist's hit rate.
    let top: Vec<Vec<u64>> = snapshot
        .long
        .iter()
        .take(10)
        .chain(snapshot.short.iter().take(10))
        .cloned()
        .collect();
    faceless::watch::count_top10(&mut store, &top);
    store.keep_snapshot(snapshot);
    store.save(&config)?;
    Ok(DiscoverResult {
        new_stories: funnels.iter().map(|f| f.new_stories).sum(),
        cost_micro: funnels.iter().map(|f| f.cost_micro).sum(),
        stories: store.topics.len(),
        funnels,
    })
}

/// The ranked ideas from the stories found so far.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeasView {
    pub profile: String,
    pub about: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_discover_at: Option<chrono::DateTime<chrono::Utc>>,
    pub stories: usize,
    pub ideas: Vec<faceless::ideas::Idea>,
}

/// Stories of the last `days` (3 by default), grouped and ranked for `format`, `limit` per lane.
pub fn faceless_ideas_impl(
    days: Option<i64>,
    limit: Option<usize>,
    format: Format,
) -> Result<IdeasView, String> {
    let config = crate::transcript::settings::config_dir();
    let profile = faceless::profile::load(&config)?;
    let store = faceless::stories::Store::load(&config);
    let made = faceless::load_made(&config);
    let ideas = faceless::ideas::rank(
        &profile,
        &store,
        &made,
        chrono::Utc::now(),
        days.unwrap_or(3).clamp(1, faceless::stories::MAX_AGE_DAYS),
        limit.unwrap_or(30).clamp(1, 100),
        format,
    );
    Ok(IdeasView {
        profile: profile.profile.name.clone(),
        about: profile.profile.about.clone(),
        last_discover_at: store.last_discover_at,
        stories: store.topics.len(),
        ideas,
    })
}

/// Sets stories aside (not interesting) so they leave the ideas, or brings them back.
pub fn faceless_set_aside_impl(ids: &[u64], aside: bool) -> Result<(), String> {
    let _running = DISCOVERING
        .try_lock()
        .ok_or("Wait for the story search to finish")?;
    let config = crate::transcript::settings::config_dir();
    let mut store = faceless::stories::Store::load(&config);
    for id in ids {
        store.set_status(*id, if aside { "set-aside" } else { "new" })?;
    }
    store.save(&config)
}

/// The watchlist with what the searches learned about each account.
pub fn faceless_watchlist_impl() -> Result<Vec<faceless::watch::Watched>, String> {
    let config = crate::transcript::settings::config_dir();
    let profile = faceless::profile::load(&config)?;
    let store = faceless::stories::Store::load(&config);
    Ok(faceless::watch::list(&profile, &store))
}

/// Adds an account to the watchlist (`x`, `youtube`, `tiktok`, `instagram`).
pub fn faceless_watch_add_impl(
    platform: &str,
    handle: &str,
    group: Option<&str>,
    name: &str,
    followers: f64,
) -> Result<(), String> {
    faceless::watch::add(
        &crate::transcript::settings::config_dir(),
        platform,
        handle,
        group,
        name,
        followers,
    )
}

pub fn faceless_watch_remove_impl(platform: &str, handle: &str) -> Result<(), String> {
    faceless::watch::remove(&crate::transcript::settings::config_dir(), platform, handle)
}

/// Accounts worth tracking on X: who the tracked accounts follow (about half a cent).
pub fn faceless_suggest_x_impl() -> Result<(Vec<faceless::watch::Candidate>, u64), String> {
    let treg = faceless::treg::Web::from_keychain()
        .ok_or("Suggestions need a Treg key: File → Accounts")?;
    let profile = faceless::profile::load(&crate::transcript::settings::config_dir())?;
    Ok(faceless::watch::suggest_x(&profile, &treg, 8))
}

/// Creators worth watching on TikTok or Instagram: who posts the most-watched videos on the
/// channel's topics (about a cent).
pub fn faceless_find_creators_impl(
    platform: &str,
    queries: Option<Vec<String>>,
) -> Result<(Vec<faceless::watch::Candidate>, u64), String> {
    let treg = faceless::treg::Web::from_keychain()
        .ok_or("Finding creators needs a Treg key: File → Accounts")?;
    let profile = faceless::profile::load(&crate::transcript::settings::config_dir())?;
    let queries = queries.filter(|q| !q.is_empty()).unwrap_or_else(|| {
        [
            "AI news",
            "ChatGPT",
            "Claude AI",
            "OpenAI",
            "AI tools",
            "Gemini AI",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    });
    faceless::watch::find_creators(&profile, &treg, platform, &queries, chrono::Utc::now())
}

/// What a demand check did.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemandResult {
    pub checked: usize,
    pub problems: Vec<String>,
    pub cost_micro: u64,
}

/// Checks demand (YouTube videos and views, Google searches, reels for shorts) for the top
/// `top` stories of the `format` ranking through Treg, and keeps it with them.
pub fn faceless_demand_impl(format: Format, top: usize) -> Result<DemandResult, String> {
    let treg = faceless::treg::Web::from_keychain()
        .ok_or("The demand check needs a Treg key: File → Accounts")?;
    let _running = DISCOVERING
        .try_lock()
        .ok_or("Wait for the story search to finish")?;
    let config = crate::transcript::settings::config_dir();
    let profile = faceless::profile::load(&config)?;
    let mut store = faceless::stories::Store::load(&config);
    let made = faceless::load_made(&config);
    let now = chrono::Utc::now();
    let ideas: Vec<faceless::ideas::Idea> =
        faceless::ideas::rank(&profile, &store, &made, now, 3, top.clamp(1, 20), format)
            .into_iter()
            .filter(|i| i.lane == faceless::ideas::Lane::Stories && !i.made)
            .collect();
    // What each watched channel usually gets (refreshed at most weekly), then the stories.
    let (usual, channel_cost) =
        faceless::demand::refresh_channels(&profile, &treg, &mut store, now);
    let (found, problems, story_cost) =
        faceless::demand::check(&profile, &treg, &ideas, format, &usual, now);
    let cost_micro = channel_cost + story_cost;
    let checked = found.len();
    for (id, demand) in found {
        if let Ok(topic) = store.topic_mut(id) {
            topic.demand = Some(demand);
        }
    }
    store.save(&config)?;
    Ok(DemandResult {
        checked,
        problems,
        cost_micro,
    })
}

/// YouTube videos for a query, through Treg: what is already made on a story, and how it does.
pub fn faceless_youtube_impl(query: &str) -> Result<(Vec<faceless::treg::Video>, u64), String> {
    let treg = faceless::treg::Web::from_keychain()
        .ok_or("YouTube search needs a Treg key: File → Accounts")?;
    faceless::treg::youtube_search(&treg, query, chrono::Utc::now())
}

/// Keywords people search around a seed, with their monthly Google volume and trend, most
/// searched first: for titles, descriptions and tags.
pub fn keyword_suggestions_impl(seed: &str) -> Result<(Vec<faceless::treg::Keyword>, u64), String> {
    let treg = faceless::treg::Web::from_keychain()
        .ok_or("Keyword data needs a Treg key: File → Accounts")?;
    let seed = seed.trim().to_lowercase();
    if seed.is_empty() {
        return Err("Give a word or phrase to start from".into());
    }
    let (mut ideas, idea_cost) = faceless::treg::keyword_ideas(&treg, &seed, 15)?;
    if !ideas.contains(&seed) {
        ideas.insert(0, seed);
    }
    ideas.truncate(16);
    let (mut volumes, volume_cost) = faceless::treg::keyword_volume(&treg, &ideas)?;
    volumes.sort_by(|a, b| b.volume.total_cmp(&a.volume));
    Ok((volumes, idea_cost + volume_cost))
}

/// Changes stories under the search lock and saves them.
fn edit_stories(
    change: impl FnOnce(&mut faceless::stories::Store) -> Result<(), String>,
) -> Result<(), String> {
    let _running = DISCOVERING
        .try_lock()
        .ok_or("Wait for the story search to finish")?;
    let config = crate::transcript::settings::config_dir();
    let mut store = faceless::stories::Store::load(&config);
    change(&mut store)?;
    store.save(&config)
}

/// One editorial read, as the agent or the user gives it.
#[derive(serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Triage {
    /// A story id (an idea's id, or any of its `ids`).
    pub id: u64,
    /// `keep` (rate it) or `reject` (set it aside with the reason).
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(flatten)]
    pub editorial: faceless::stories::Editorial,
}

/// Saves editorial reads: ratings blend into the ranking; a reject sets the story aside.
pub fn faceless_triage_impl(reads: Vec<Triage>, by: &str) -> Result<usize, String> {
    let count = reads.len();
    edit_stories(|store| {
        for mut read in reads {
            for v in [
                read.editorial.importance,
                read.editorial.long_fit,
                read.editorial.short_fit,
            ] {
                if !(1..=5).contains(&v) && read.verdict.as_deref() != Some("reject") {
                    return Err(format!(
                        "Story {}: importance, longFit and shortFit are 1 to 5",
                        read.id
                    ));
                }
            }
            read.editorial.by = by.to_string();
            let topic = store.topic_mut(read.id)?;
            if read.verdict.as_deref() == Some("reject") {
                topic.status = "set-aside".into();
            }
            topic.editorial = Some(read.editorial);
        }
        Ok(())
    })?;
    Ok(count)
}

/// Pins a story first in the ranking (or unpins it).
pub fn faceless_pin_impl(id: u64, pinned: bool) -> Result<(), String> {
    edit_stories(|store| {
        store.topic_mut(id)?.pinned = pinned;
        Ok(())
    })
}

/// Takes a report out of the group it was put in wrongly (or lets it group again).
pub fn faceless_separate_impl(id: u64, separate: bool) -> Result<(), String> {
    edit_stories(|store| {
        let topic = store.topic_mut(id)?;
        topic.separate = separate;
        if separate {
            topic.joined_to = None;
        }
        Ok(())
    })
}

/// Joins stories (by their ids) to another story's group: one event worded differently.
pub fn faceless_join_impl(ids: &[u64], to: u64) -> Result<(), String> {
    edit_stories(|store| {
        store.topic_mut(to)?;
        for id in ids.iter().filter(|id| **id != to) {
            let topic = store.topic_mut(*id)?;
            topic.joined_to = Some(to);
            topic.separate = false;
        }
        Ok(())
    })
}

/// Makes a production for `topic` in the Faceless folder, opens it, and puts it on the format's
/// preset (Faceless long or short: the method, 9:16 and captions for a short; one undo step).
pub fn faceless_create_impl(
    state: &AppState,
    topic: String,
    format: Format,
    links: Vec<String>,
) -> Result<FacelessCreated, String> {
    let made = faceless_made_impl();
    let already_made = links
        .iter()
        .find_map(|link| faceless::already_made(&made, &topic, Some(link)))
        .or_else(|| faceless::already_made(&made, &topic, None))
        .cloned();
    let folder = faceless::create(&faceless::home(), &topic, format, &links)?;
    let opened = super::open_project_impl(state, folder.to_string_lossy().into_owned())?;
    let preset = match format {
        Format::Long => crate::preset::FACELESS_LONG,
        Format::Short => crate::preset::FACELESS_SHORT,
    };
    let opened = super::preset::project_preset_set_impl(
        state,
        opened.project_handle.clone(),
        opened.revision,
        preset.into(),
        super::preset::PresetTarget::Video,
    )?;
    let production = faceless::summary(&folder).ok_or("The production was not saved")?;
    Ok(FacelessCreated {
        opened,
        production,
        already_made,
    })
}

/// The open project's production; `None` when no project is open or it is an ordinary one.
pub fn faceless_status_impl(state: &AppState) -> Option<ProductionSummary> {
    let root = {
        let guard = state.opened_project.lock();
        guard.as_ref()?.root().to_path_buf()
    };
    faceless::summary(&root)
}

/// The open project's folder, when it is a faceless production.
pub fn production_root(state: &AppState) -> Result<std::path::PathBuf, String> {
    let root = {
        let guard = state.opened_project.lock();
        guard
            .as_ref()
            .ok_or(crate::mcp::tools::NO_PROJECT_MESSAGE)?
            .root()
            .to_path_buf()
    };
    faceless::load(&root).ok_or("The open project is not a faceless video")?;
    Ok(root)
}

/// The open production's research, or none yet.
pub fn faceless_dossier_impl(
    state: &AppState,
) -> Result<Option<faceless::evidence::Dossier>, String> {
    Ok(faceless::evidence::load(&production_root(state)?))
}

/// Fetches pages into the open production's research.
pub fn faceless_fetch_impl(
    state: &AppState,
    urls: &[String],
    again: bool,
) -> Result<Vec<faceless::evidence::Source>, String> {
    let root = production_root(state)?;
    let own = faceless::evidence::WebPages::new()?;
    // With a Treg key, Treg reads the PDFs and Google News links the app can't.
    match faceless::treg::Web::from_keychain() {
        Some(treg) => faceless::evidence::fetch(
            &root,
            &faceless::evidence::WithTreg { own, treg },
            urls,
            again,
        ),
        None => faceless::evidence::fetch(&root, &own, urls, again),
    }
}

/// Searches the web through Treg for the open production's research.
pub fn faceless_web_search_impl(
    state: &AppState,
    query: &str,
    limit: usize,
) -> Result<(Vec<faceless::treg::SearchResult>, u64), String> {
    production_root(state)?;
    let treg = faceless::treg::Web::from_keychain()
        .ok_or("Web search needs a Treg key: File → Accounts (or search with your own tools)")?;
    faceless::treg::web_search(&treg, query, limit)
}

/// Saves the Treg key in the keychain.
pub fn treg_set_key_impl(key: String) -> Result<(), String> {
    crate::secrets::set(
        &crate::transcript::settings::config_dir(),
        faceless::treg::KEY,
        &key,
    )
}

/// Whether a Treg key is set, and where it comes from.
pub fn treg_key_source_impl() -> Option<String> {
    crate::secrets::get(
        &crate::transcript::settings::config_dir(),
        faceless::treg::KEY,
    )
    .map(|(_, source)| source.to_string())
}

/// Checks and keeps facts in the open production's research.
pub fn faceless_add_facts_impl(
    state: &AppState,
    facts: Vec<faceless::evidence::FactInput>,
) -> Result<faceless::evidence::Added, String> {
    let root = production_root(state)?;
    let profile = faceless::profile::load(&crate::transcript::settings::config_dir())?;
    faceless::evidence::add_facts(&root, facts, &profile.profile.primary_domains)
}

/// What voicing did, for the panel and the agent.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceResult {
    pub voice: faceless::voice::Voice,
    /// The stage is done: nothing to listen to, or the user accepted it.
    pub done: bool,
    /// What the transcription said besides the words (provider fallbacks, language).
    pub diagnostics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opened: Option<OpenedProject>,
}

/// Voices the open production's reviewed script with the channel's voice (OpenRouter, the
/// user's key), puts the narration on the timeline as speech in place of an earlier one (one
/// undo step), transcribes it with the saved transcription settings and checks the names and
/// numbers heard. `redo`: scenes (1-based) to voice again even though they didn't change.
pub fn faceless_voice_impl(
    state: &AppState,
    transcripts: &super::transcript::TranscriptState,
    redo: &[usize],
    progress: &mut dyn FnMut(f64, &str),
) -> Result<VoiceResult, String> {
    use faceless::voice;
    let root = production_root(state)?;
    let production = faceless::load(&root).ok_or("Not a faceless video")?;
    let states = faceless::stage_states(&root, &production);
    let review = states.iter().find(|s| s.id == "review");
    if review.is_none_or(|s| s.state != faceless::StageState::Done) {
        return Err(
            "Voice comes after the review: write the script and review it with no \
                    blocker open"
                .into(),
        );
    }
    let script = faceless::script::load_script(&root).ok_or("Write the script first")?;
    let profile = faceless::profile::load(&crate::transcript::settings::config_dir())?;
    let speak = voice::OpenRouter::from_keychain().ok_or(
        "Voice needs an OpenRouter key: add it in Settings → Transcription (it is the same key)",
    )?;
    let redo: Vec<usize> = redo.iter().filter_map(|n| n.checked_sub(1)).collect();
    let previous = voice::load(&root);
    let mut made = {
        let progress = std::sync::Mutex::new(&mut *progress);
        voice::make(
            &root,
            &script,
            &profile.profile.voice,
            &speak,
            &redo,
            &|f, m| {
                if let Ok(mut p) = progress.lock() {
                    p(f * 0.7, m)
                }
            },
        )?
    };
    progress(0.72, "Putting the narration on the timeline");
    let same_file = previous
        .as_ref()
        .is_some_and(|v| v.narration == made.narration && v.asset_id.is_some())
        && narration_on_timeline(state, previous.as_ref().and_then(|v| v.asset_id.as_deref()));
    let (handle, sound, opened) = if same_file {
        // Nothing changed in the audio: the narration on the timeline stays.
        let p = previous.as_ref().expect("checked");
        made.asset_id = p.asset_id.clone();
        let handle = {
            let guard = state.opened_project.lock();
            guard
                .as_ref()
                .ok_or("No opened project")?
                .summary
                .project_handle
                .clone()
        };
        (handle, p.sound.clone().unwrap_or_default(), None)
    } else {
        let previous_asset = previous.as_ref().and_then(|v| v.asset_id.clone());
        let (opened, asset_id, sound) =
            place_narration(state, &root, &made.narration, previous_asset.as_deref())?;
        made.asset_id = Some(asset_id);
        (opened.project_handle.clone(), sound, Some(opened))
    };
    made.sound = Some(sound.clone());
    let mut diagnostics = Vec::new();
    let transcript = match crate::transcript::store::load_transcript(&root, &sound)? {
        Some(t) if same_file => t,
        _ => {
            progress(0.75, "Transcribing the narration");
            let result = super::transcript::transcript_run_impl(
                state,
                transcripts,
                handle,
                sound.clone(),
                Default::default(),
                &mut |p| progress(0.75 + p.fraction * 0.2, &p.message),
            )?;
            diagnostics = result.diagnostics;
            crate::transcript::store::load_transcript(&root, &sound)?
                .ok_or("The transcript was not saved")?
        }
    };
    let words: Vec<(String, u64, u64)> = transcript
        .words
        .iter()
        .filter(|w| w.kind == crate::transcript::WordKind::Word)
        .map(|w| (w.text.clone(), w.source_start_us, w.source_end_us))
        .collect();
    made.heard_words = words.len();
    made.issues = voice::heard_issues(&script, &made, &words);
    let spoken_words: usize = script
        .scenes
        .iter()
        .map(|s| s.narration.split_whitespace().count())
        .sum();
    made.length_problem = voice::length_problem(made.duration_us, production.format, spoken_words);
    // A listen already given stays with the same audio.
    made.accepted = same_file && previous.as_ref().is_some_and(|p| p.accepted);
    finish_voice(&root, made, diagnostics, opened)
}

/// Whether the open project's timeline still plays asset `id`.
fn narration_on_timeline(state: &AppState, id: Option<&str>) -> bool {
    let Some(id) = id else {
        return false;
    };
    let guard = state.opened_project.lock();
    guard
        .as_ref()
        .is_some_and(|r| r.document().sequence.clips().any(|(_, c)| c.asset == id))
}

/// Saves the voice stage's file and records the stage when there is nothing left to hear.
fn finish_voice(
    root: &std::path::Path,
    voice: faceless::voice::Voice,
    diagnostics: Vec<String>,
    opened: Option<OpenedProject>,
) -> Result<VoiceResult, String> {
    faceless::voice::save(root, &voice)?;
    let done = voice.accepted || (voice.issues.is_empty() && voice.length_problem.is_none());
    if done {
        faceless::record_stage(root, "voice")?;
    } else {
        faceless::clear_stage(root, "voice")?;
    }
    Ok(VoiceResult {
        voice,
        done,
        diagnostics,
        opened,
    })
}

/// The open production's narration, if voiced.
pub fn faceless_voice_get_impl(state: &AppState) -> Result<Option<faceless::voice::Voice>, String> {
    Ok(faceless::voice::load(&production_root(state)?))
}

/// The user listened: the names and numbers flagged are right, and the length will do.
pub fn faceless_voice_accept_impl(state: &AppState) -> Result<VoiceResult, String> {
    let root = production_root(state)?;
    let mut voice = faceless::voice::load(&root).ok_or("Voice the script first")?;
    voice.accepted = true;
    finish_voice(&root, voice, Vec::new(), None)
}

/// Imports the narration and puts it at the start of the timeline as speech, taking out an
/// earlier narration's clips and asset: one undo step. Returns the project, the asset's id and
/// its sound key.
fn place_narration(
    state: &AppState,
    root: &std::path::Path,
    narration: &str,
    previous: Option<&str>,
) -> Result<(OpenedProject, String, String), String> {
    use crate::sequence::edit::{apply, SequenceEdit};
    use crate::sequence::{Role, StreamKind};
    let file = root.join(faceless::DIR).join(narration);
    let mut asset = crate::media_bin::import(root, &file)?;
    asset.name = "Narration".into();
    let stream = asset
        .streams
        .iter_mut()
        .find(|s| s.kind == StreamKind::Sound)
        .ok_or("The narration has no sound")?;
    stream.role = Role::Mic;
    stream.name = "Narration".into();
    let sound = format!("{}.{}", asset.id, stream.id);
    let asset_id = asset.id.clone();
    let placed = (|| {
        let _guard = state.command_lock.lock();
        let mut opened = state.opened_project.lock();
        let reader = opened.as_mut().ok_or("No opened project")?;
        if reader.root() != root {
            return Err("The project changed while the narration was made".to_string());
        }
        let mut next = reader.document().clone();
        if let Some(old) = previous {
            let clip_ids: Vec<String> = next
                .sequence
                .clips()
                .filter(|(_, c)| c.asset == old)
                .map(|(_, c)| c.id.clone())
                .collect();
            if !clip_ids.is_empty() {
                next.sequence = apply(
                    &next.sequence,
                    &next.assets,
                    &SequenceEdit::Delete {
                        clip_ids,
                        ripple: Some(false),
                    },
                )?
                .0;
            }
            next.assets.retain(|a| a.id != old);
        }
        next.assets.push(asset.clone());
        next.sequence = apply(
            &next.sequence,
            &next.assets,
            &SequenceEdit::PlaceAsset {
                asset_id: asset_id.clone(),
                at_us: 0,
                track_id: None,
                streams: Vec::new(),
                range: None,
            },
        )?
        .0;
        let revision = reader.document().revision;
        reader.name_next_step("Narration".into());
        let project = reader.commit_document(revision, next)?;
        state
            .playback
            .lock()
            .apply_document(&reader.history().current)?;
        state
            .waveform_epoch
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(project)
    })();
    match placed {
        Ok(project) => Ok((project, asset_id, sound)),
        Err(error) => {
            crate::media_bin::remove_files(root, &asset);
            Err(error)
        }
    }
}

/// What the visuals check found.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualsResult {
    pub visuals: faceless::visuals::Visuals,
    /// Graphics stretched to meet the next.
    pub filled: usize,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opened: Option<OpenedProject>,
}

/// Checks the open production's graphics against its narration and, with `fill`, first
/// stretches each to meet the next (one undo step). Records the stage when nothing blocks it.
pub fn faceless_visuals_impl(state: &AppState, fill: bool) -> Result<VisualsResult, String> {
    use faceless::visuals;
    let root = production_root(state)?;
    let production = faceless::load(&root).ok_or("Not a faceless video")?;
    let voice_done = faceless::stage_states(&root, &production)
        .iter()
        .any(|s| s.id == "voice" && s.state == faceless::StageState::Done);
    if !voice_done {
        return Err(
            "Visuals come after the voice: graphics are timed on the narration's words".into(),
        );
    }
    let sound = faceless::voice::load(&root)
        .and_then(|v| v.sound)
        .ok_or("Voice the script first")?;
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    if reader.root() != root {
        return Err("The project changed".into());
    }
    let mut project = None;
    let mut filled = 0;
    if fill {
        let document = reader.document();
        let beats = visuals::gather(document, &root);
        let fills = visuals::fills(&beats, document.duration_us());
        if !fills.is_empty() {
            let mut next = document.clone();
            for (clip_id, to_us) in &fills {
                next.sequence = crate::sequence::edit::apply(
                    &next.sequence,
                    &next.assets,
                    &crate::sequence::edit::SequenceEdit::TrimClip {
                        clip_id: clip_id.clone(),
                        edge: crate::sequence::edit::Edge::End,
                        to_us: *to_us,
                        ripple: Some(false),
                    },
                )?
                .0;
            }
            let revision = document.revision;
            reader.name_next_step("Graphics meet each other".into());
            project = Some(reader.commit_document(revision, next)?);
            state
                .playback
                .lock()
                .apply_document(&reader.history().current)?;
            filled = fills.len();
        }
    }
    let document = reader.document();
    let words: Vec<(u64, String)> = match crate::transcript::store::load_transcript(&root, &sound)?
    {
        Some(transcript) => crate::transcript::edit::view(
            &transcript,
            &document.transcript_clock(&sound)?,
            document.revision,
        )
        .words
        .into_iter()
        .filter(|w| w.word.kind == crate::transcript::WordKind::Word)
        .filter_map(|w| Some((w.edited_start_us?, w.word.text)))
        .collect(),
        None => Vec::new(),
    };
    let duration_us = document.duration_us();
    let mut beats = visuals::gather(document, &root);
    visuals::caption(&mut beats, &words);
    let (errors, notes) = visuals::judge(&beats, &words, duration_us, production.format);
    let result = visuals::Visuals {
        beats,
        duration_us,
        errors,
        notes,
        at: chrono::Utc::now().to_rfc3339(),
    };
    drop(opened);
    visuals::save(&root, &result)?;
    let done = result.errors.is_empty();
    if done {
        faceless::record_stage(&root, "visuals")?;
    } else {
        faceless::clear_stage(&root, "visuals")?;
    }
    Ok(VisualsResult {
        visuals: result,
        filled,
        done,
        opened: project,
    })
}

/// The open production's last visuals check, if any.
pub fn faceless_visuals_get_impl(
    state: &AppState,
) -> Result<Option<faceless::visuals::Visuals>, String> {
    Ok(faceless::visuals::load(&production_root(state)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(path: &std::path::Path, seconds: u64) {
        let samples: Vec<i16> = (0..48_000 * seconds)
            .map(|i| ((i as f64 * 0.05).sin() * 8000.0) as i16)
            .collect();
        std::fs::write(
            path,
            crate::fixtures::generate_pcm16_wav(48_000, 1, &samples),
        )
        .unwrap();
    }

    #[test]
    fn the_narration_goes_on_the_timeline_as_speech_and_replaces_the_last() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = faceless::create(dir.path(), "Narrated", Format::Short, &[]).unwrap();
        let host = crate::mcp::tests_support::BareHost::default();
        let state = &host.state;
        super::super::open_project_impl(state, root.to_string_lossy().into()).unwrap();
        let voice_dir = faceless::voice::folder(&root);
        std::fs::create_dir_all(&voice_dir).unwrap();
        tone(&voice_dir.join("a.wav"), 3);
        tone(&voice_dir.join("b.wav"), 5);
        let (_, first, sound) = place_narration(state, &root, "voice/a.wav", None).unwrap();
        assert_eq!(sound, format!("{first}.sound0"));
        let check = |id: &str, seconds: u64| {
            let opened = state.opened_project.lock();
            let document = opened.as_ref().unwrap().document();
            let asset = document.asset(id).unwrap();
            assert_eq!(asset.streams[0].role, crate::sequence::Role::Mic);
            assert_eq!(document.assets.len(), 1, "the last narration is gone");
            let clips: Vec<_> = document.sequence.clips().collect();
            assert_eq!(clips.len(), 1);
            assert_eq!(clips[0].1.start_us, 0);
            assert!(clips.iter().all(|(_, c)| c.asset == id));
            let length = document.duration_us();
            assert!(length.abs_diff(seconds * 1_000_000) < 50_000, "{length}");
        };
        check(&first, 3);
        let (_, second, _) = place_narration(state, &root, "voice/b.wav", Some(&first)).unwrap();
        check(&second, 5);
    }

    /// A voiced production with two parts: visuals block on the gap between them until `fill`
    /// closes it, and then the stage is done.
    #[test]
    fn visuals_close_their_gaps_and_record_the_stage() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = faceless::create(dir.path(), "Visual", Format::Short, &[]).unwrap();
        let host = crate::mcp::tests_support::BareHost::default();
        let state = &host.state;
        super::super::open_project_impl(state, root.to_string_lossy().into()).unwrap();
        let voice_dir = faceless::voice::folder(&root);
        std::fs::create_dir_all(&voice_dir).unwrap();
        tone(&voice_dir.join("n.wav"), 12);
        let (_, asset, sound) = place_narration(state, &root, "voice/n.wav", None).unwrap();
        for id in ["research", "script", "review"] {
            let stage = faceless::STAGES.iter().find(|s| s.id == id).unwrap();
            std::fs::write(root.join(faceless::DIR).join(stage.file), id).unwrap();
            faceless::record_stage(&root, id).unwrap();
        }
        let voice = faceless::voice::Voice {
            asset_id: Some(asset),
            sound: Some(sound),
            ..Default::default()
        };
        faceless::voice::save(&root, &voice).unwrap();
        faceless::record_stage(&root, "voice").unwrap();
        let (handle, revision) = {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().unwrap();
            (
                reader.summary.project_handle.clone(),
                reader.summary.revision,
            )
        };
        let graphic = |template: &str, at_us: u64| super::super::graphics::VideoGraphic {
            from: super::super::graphics::ShortGraphicFrom::Template {
                template: template.into(),
            },
            at_us,
            length_us: Some(3_000_000),
            name: None,
            variables: serde_json::Map::new(),
        };
        super::super::graphics::project_graphics_add_impl(
            state,
            handle,
            revision,
            vec![graphic("headline", 0), graphic("bars", 6_000_000)],
        )
        .unwrap();
        let checked = faceless_visuals_impl(state, false).unwrap();
        assert!(!checked.done);
        assert_eq!(
            checked.visuals.errors.len(),
            2,
            "{:?}",
            checked.visuals.errors
        );
        let filled = faceless_visuals_impl(state, true).unwrap();
        assert_eq!(filled.filled, 2);
        assert!(filled.done, "{:?}", filled.visuals.errors);
        let beats = &filled.visuals.beats;
        assert_eq!(beats[0].end_us, beats[1].start_us);
        assert_eq!(beats[1].end_us, filled.visuals.duration_us);
        assert_eq!(beats[1].cue_us, Some(7_000_000), "bars' own cue, 1 s in");
        let production = faceless::load(&root).unwrap();
        assert!(faceless::stage_states(&root, &production)
            .iter()
            .any(|s| s.id == "visuals" && s.state == faceless::StageState::Done));
    }

    /// One short sentence through OpenRouter with the channel's voice, on the real key. Run by
    /// hand: `cargo test --lib live_voice -- --ignored`.
    #[test]
    #[ignore]
    fn live_voice() {
        use faceless::voice::{OpenRouter, Speak, VoiceSettings};
        let speak = OpenRouter::from_keychain().expect("an OpenRouter key");
        let settings = VoiceSettings::default();
        let audio = speak
            .speak(
                &settings.model,
                &settings.voice,
                "This is a test of the voice.",
            )
            .unwrap();
        let file = std::env::temp_dir().join("aeroedits-live-voice.mp3");
        std::fs::write(&file, &audio).unwrap();
        let length = crate::media::ffmpeg::duration_us(&file).unwrap();
        println!(
            "{} bytes, {length} us, price per char {:?}, at {}",
            audio.len(),
            speak.price_per_char(&settings.model),
            file.display()
        );
        assert!(length > 500_000);
    }
}
