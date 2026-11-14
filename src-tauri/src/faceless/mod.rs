//! Faceless videos (FCL, docs/ROADMAP.md#fcl). A production is an empty project whose picture
//! is graphics and whose sound is generated narration, long (16:9) or short (9:16). What it is
//! and how far it got lives beside the edit in `faceless/production.json`, outside the undo
//! history: undoing a cut never forgets a script.
//!
//! Each stage writes one file in the project's `faceless/` folder and records two hashes when
//! it does: what it read (the stage before it's file) and what it wrote. A stage whose input
//! changed since is stale, and so is every stage after it.
//!
//! Nothing here runs by itself: every step starts from the user's click or their agent's call.
pub mod demand;
pub mod discover;
pub mod evidence;
pub mod ideas;
pub mod profile;
pub mod script;
pub mod stories;
pub mod treg;
pub mod visuals;
pub mod voice;
pub mod watch;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The folder in a project that holds a production's files.
pub const DIR: &str = "faceless";
const PRODUCTION: &str = "production.json";
const MADE: &str = "made.json";
pub const MAX_TOPIC_CHARS: usize = 200;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    /// 5 to 8 minutes, 16:9.
    #[default]
    Long,
    /// 30 to 58 seconds, 9:16.
    Short,
}

impl Format {
    pub fn aspect_ratio(self) -> &'static str {
        match self {
            Format::Long => "16:9",
            Format::Short => "9:16",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Format::Long => "long",
            Format::Short => "short",
        }
    }
}

/// One stage of a production and the file it writes in `faceless/`.
pub struct StageDef {
    pub id: &'static str,
    pub label: &'static str,
    pub file: &'static str,
}

/// The stages, in order. Ideas come before them and belong to no production.
pub const STAGES: [StageDef; 7] = [
    StageDef {
        id: "research",
        label: "Research",
        file: "dossier.json",
    },
    StageDef {
        id: "script",
        label: "Script",
        file: "script.json",
    },
    StageDef {
        id: "review",
        label: "Review",
        file: "review.json",
    },
    StageDef {
        id: "voice",
        label: "Voice",
        file: "voice.json",
    },
    StageDef {
        id: "visuals",
        label: "Visuals",
        file: "beats.json",
    },
    StageDef {
        id: "package",
        label: "Package",
        file: "package.json",
    },
    StageDef {
        id: "publish",
        label: "Publish",
        file: "publish.json",
    },
];

/// `faceless/production.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Production {
    pub format: Format,
    /// What the video is about, as typed or picked from the ideas.
    pub topic: String,
    /// The story's reports when it was picked from the ideas: where research starts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
    pub created_at: String,
    /// Per stage id: what it read and wrote when it last finished.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub stages: BTreeMap<String, StageRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StageRecord {
    /// Hash of what the stage read: the stage before it's file, or the topic for the first.
    pub inputs: String,
    /// Hash of the file it wrote.
    pub output: String,
    pub at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StageState {
    /// Not done yet.
    Todo,
    /// Done, and nothing it read changed since.
    Done,
    /// Done, but something before it changed: do it again.
    Stale,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageStatus {
    pub id: &'static str,
    pub label: &'static str,
    pub state: StageState,
}

/// A production as the Faceless tab and agents see it.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSummary {
    /// The project folder.
    pub path: String,
    pub name: String,
    pub format: Format,
    pub topic: String,
    pub links: Vec<String>,
    pub created_at: String,
    pub stages: Vec<StageStatus>,
    /// The first stage not done or stale; none when every stage is done.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<&'static str>,
}

/// Where new productions are made: `Documents/AeroEdits/Faceless`.
pub fn home() -> PathBuf {
    crate::commands::default_projects_dir().join("Faceless")
}

fn production_path(root: &Path) -> PathBuf {
    root.join(DIR).join(PRODUCTION)
}

/// The production in project folder `root`; `None` for an ordinary project.
pub fn load(root: &Path) -> Option<Production> {
    crate::files::load_json(&production_path(root))
}

fn save(root: &Path, production: &Production) -> Result<(), String> {
    crate::files::save_json(&production_path(root), production)
}

/// The topic as typed, trimmed, or why it can't be one.
fn clean_topic(topic: &str) -> Result<String, String> {
    let topic = topic.split_whitespace().collect::<Vec<_>>().join(" ");
    if topic.is_empty() {
        return Err("Give the video a topic".into());
    }
    if topic.chars().count() > MAX_TOPIC_CHARS {
        return Err(format!("Keep the topic under {MAX_TOPIC_CHARS} characters"));
    }
    Ok(topic)
}

/// Makes a new production's project folder in `parent` (named after the topic) and returns it.
/// The caller opens it and sets the canvas to the format's shape. `links`: the story's reports.
pub fn create(
    parent: &Path,
    topic: &str,
    format: Format,
    links: &[String],
) -> Result<PathBuf, String> {
    let topic = clean_topic(topic)?;
    let folder = crate::project::folder::create_project_folder(parent, &topic, None)?;
    let production = Production {
        format,
        topic,
        links: links.iter().take(10).cloned().collect(),
        created_at: chrono::Utc::now().to_rfc3339(),
        stages: BTreeMap::new(),
    };
    if let Err(error) = save(&folder, &production) {
        let _ = std::fs::remove_dir_all(&folder);
        return Err(error);
    }
    Ok(folder)
}

fn hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().take(12).map(|b| format!("{b:02x}")).collect()
}

/// The hash of stage `index`'s file now, `None` when it isn't there.
fn file_hash(root: &Path, index: usize) -> Option<String> {
    std::fs::read(root.join(DIR).join(STAGES[index].file))
        .ok()
        .map(|bytes| hash(&bytes))
}

/// What stage `index` reads, hashed: the stage before it's file, or the format and topic.
fn input_hash(root: &Path, production: &Production, index: usize) -> Option<String> {
    if index == 0 {
        Some(hash(
            format!("{}\n{}", production.format.label(), production.topic).as_bytes(),
        ))
    } else {
        file_hash(root, index - 1)
    }
}

/// Records that stage `id` just wrote its file: what it read and what it wrote. Later stages
/// call this after writing their file.
pub fn record_stage(root: &Path, id: &str) -> Result<(), String> {
    record(root, id, None).map(|_| ())
}

/// What stage `id` reads now, when every stage before it is done: capture it when the stage's
/// work begins and hand it to [`record_stage_for`] when the work ends.
pub fn ready_inputs(root: &Path, id: &str) -> Option<String> {
    let index = STAGES.iter().position(|s| s.id == id)?;
    let production = load(root)?;
    let states = stage_states(root, &production);
    if states[..index].iter().any(|s| s.state != StageState::Done) {
        return None;
    }
    input_hash(root, &production, index)
}

/// [`record_stage`], only while what the stage reads is still `inputs` and every stage before
/// it is done. Otherwise the stage is marked not done and this returns false: its work was made
/// from inputs that have changed since.
pub fn record_stage_for(root: &Path, id: &str, inputs: &str) -> Result<bool, String> {
    if record(root, id, Some(inputs))? {
        return Ok(true);
    }
    clear_stage(root, id)?;
    Ok(false)
}

fn record(root: &Path, id: &str, expected: Option<&str>) -> Result<bool, String> {
    let index = STAGES
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| format!("No faceless stage {id}"))?;
    if expected.is_some_and(|e| ready_inputs(root, id).as_deref() != Some(e)) {
        return Ok(false);
    }
    let mut production = load(root).ok_or("This project is not a faceless video")?;
    let output = file_hash(root, index)
        .ok_or_else(|| format!("{} has not been written", STAGES[index].file))?;
    let inputs = input_hash(root, &production, index)
        .ok_or_else(|| format!("{} comes first", STAGES[index - 1].label))?;
    production.stages.insert(
        id.to_string(),
        StageRecord {
            inputs,
            output,
            at: chrono::Utc::now().to_rfc3339(),
        },
    );
    save(root, &production)?;
    Ok(true)
}

/// Marks stage `id` not done (its file stays): a review with an open blocker, say.
pub fn clear_stage(root: &Path, id: &str) -> Result<(), String> {
    let mut production = load(root).ok_or("This project is not a faceless video")?;
    if production.stages.remove(id).is_some() {
        save(root, &production)?;
    }
    Ok(())
}

/// Each stage's state, in order. A stage is stale when what it read changed since it ran, or
/// any stage before it is stale or not done.
pub fn stage_states(root: &Path, production: &Production) -> Vec<StageStatus> {
    let mut earlier_ok = true;
    STAGES
        .iter()
        .enumerate()
        .map(|(index, stage)| {
            let state = match production.stages.get(stage.id) {
                Some(record) if file_hash(root, index).is_some() => {
                    let current = input_hash(root, production, index);
                    // Its own file must still be the one it wrote: a file replaced or damaged
                    // since is not certified by the old record (GAP-027).
                    let output = file_hash(root, index);
                    if earlier_ok
                        && current.as_deref() == Some(record.inputs.as_str())
                        && output.as_deref() == Some(record.output.as_str())
                    {
                        StageState::Done
                    } else {
                        StageState::Stale
                    }
                }
                _ => StageState::Todo,
            };
            earlier_ok = state == StageState::Done;
            StageStatus {
                id: stage.id,
                label: stage.label,
                state,
            }
        })
        .collect()
}

/// The production in `root` as the tab shows it; `None` for an ordinary project.
pub fn summary(root: &Path) -> Option<ProductionSummary> {
    let production = load(root)?;
    let stages = stage_states(root, &production);
    let next = stages
        .iter()
        .find(|s| s.state != StageState::Done)
        .map(|s| s.id);
    let name = crate::project::folder::load_project_file(root)
        .ok()
        .flatten()
        .map(|file| file.name)
        .unwrap_or_else(|| production.topic.clone());
    Some(ProductionSummary {
        path: root.to_string_lossy().into_owned(),
        name,
        format: production.format,
        topic: production.topic,
        links: production.links,
        created_at: production.created_at,
        stages,
        next,
    })
}

/// Every production in `parent` (one folder deep), newest first.
pub fn list(parent: &Path) -> Vec<ProductionSummary> {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut found: Vec<ProductionSummary> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| summary(&entry.path()))
        .collect();
    found.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    found
}

/// A video already made, so no story is made twice, tests included.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Made {
    pub topic: String,
    /// The story's main source, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    pub format: Format,
    /// The project folder it was made in.
    pub project: String,
    /// `test`, `private` or `public`.
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub youtube: Option<String>,
    pub at: String,
}

/// The app-wide list of videos made: `<config>/AeroEdits/faceless/made.json`.
pub fn made_path(config: &Path) -> PathBuf {
    config.join(DIR).join(MADE)
}

pub fn load_made(config: &Path) -> Vec<Made> {
    crate::files::load_json(&made_path(config)).unwrap_or_default()
}

/// Adds `entry`, replacing an earlier entry for the same project (a private upload made
/// public, a test uploaded).
pub fn add_made(config: &Path, entry: Made) -> Result<(), String> {
    let mut made = load_made(config);
    made.retain(|m| m.project != entry.project);
    made.push(entry);
    crate::files::save_json(&made_path(config), &made)
}

/// Words of a topic, lowercased, for comparing stories loosely.
fn topic_key(topic: &str) -> String {
    topic
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A video already made on this topic or link, if any.
pub fn already_made<'a>(made: &'a [Made], topic: &str, link: Option<&str>) -> Option<&'a Made> {
    let key = topic_key(topic);
    made.iter().find(|m| {
        topic_key(&m.topic) == key
            || matches!((link, m.link.as_deref()), (Some(a), Some(b)) if a.trim() == b.trim())
    })
}

/// A production's package stage follows the video's publish package (the same for every
/// video, PUB-1): `faceless/package.json` mirrors it, and the stage is done when it has a
/// title, a description, tags and a thumbnail YouTube takes. Nothing for an ordinary project.
pub fn sync_package(root: &Path) -> Result<(), String> {
    if load(root).is_none() {
        return Ok(());
    }
    let package = crate::publish::package(root, "video");
    let thumbnail = crate::publish::thumbnail_path(root, &package)
        .and_then(|p| std::fs::read(p).ok())
        .map(|bytes| hash(&bytes));
    let mirror = serde_json::json!({
        "title": package.title,
        "description": package.description,
        "tags": package.tags,
        "thumbnail": thumbnail,
        "aiDisclosure": package.ai_disclosure,
    });
    let path = root.join(DIR).join("package.json");
    if crate::files::load_json::<serde_json::Value>(&path).as_ref() != Some(&mirror) {
        crate::files::save_json(&path, &mirror)?;
    }
    let complete = crate::publish::package_problem(&package).is_none()
        && !package.tags.is_empty()
        && thumbnail.is_some();
    // Written before the visuals exist, it waits: the stage is recorded once they do.
    let visuals = root.join(DIR).join(STAGES[4].file).is_file();
    if complete && visuals {
        record_stage(root, "package")
    } else {
        clear_stage(root, "package")
    }
}

/// A production's publish stage: done once the video is on YouTube (or scheduled), from the
/// upload `upload_id` the app just made.
pub fn record_upload(root: &Path, upload: &crate::publish::Upload) -> Result<(), String> {
    if load(root).is_none() || upload.target != "video" || upload.youtube_id.is_none() {
        return Ok(());
    }
    sync_package(root)?;
    crate::files::save_json(
        &root.join(DIR).join("publish.json"),
        &serde_json::json!({
            "upload": upload.id,
            "youtube": upload.youtube_id,
            "release": upload.release,
            "at": chrono::Utc::now().to_rfc3339(),
        }),
    )?;
    record_stage(root, "publish")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_stage(root: &Path, id: &str, contents: &str) {
        let stage = STAGES.iter().find(|s| s.id == id).unwrap();
        std::fs::write(root.join(DIR).join(stage.file), contents).unwrap();
        record_stage(root, id).unwrap();
    }

    fn states(root: &Path) -> Vec<StageState> {
        stage_states(root, &load(root).unwrap())
            .into_iter()
            .map(|s| s.state)
            .collect()
    }

    /// GAP-027: a stage's own file replaced, damaged or removed after it finished is no longer
    /// Done, and neither is what follows; the exact bytes back make it Done again.
    #[test]
    fn a_stage_whose_own_file_changed_is_not_done() {
        let dir = tempfile::tempdir().unwrap();
        let root = create(dir.path(), "Receipts", Format::Short, &[]).unwrap();
        for id in ["research", "script", "review"] {
            write_stage(&root, id, &format!("{{\"{id}\": 1}}"));
        }
        let done = vec![StageState::Done; 3];
        assert_eq!(states(&root)[..3], done[..]);
        let review = root.join(DIR).join(STAGES[2].file);
        for replaced in [Some("{\"review\": 2}"), Some("{not json"), None] {
            match replaced {
                Some(text) => std::fs::write(&review, text).unwrap(),
                None => std::fs::remove_file(&review).unwrap(),
            }
            let now = states(&root);
            assert_eq!(now[..2], done[..2]);
            assert_ne!(now[2], StageState::Done, "{replaced:?}");
        }
        std::fs::write(&review, "{\"review\": 1}").unwrap();
        assert_eq!(states(&root)[..3], done[..]);
    }

    #[test]
    fn a_new_production_is_a_project_with_every_stage_to_do() {
        let dir = tempfile::tempdir().unwrap();
        let root = create(dir.path(), "  Kimi's   audit  ", Format::Short, &[]).unwrap();
        assert_eq!(root.file_name().unwrap(), "Kimi's audit");
        assert!(crate::project::folder::load_project_file(&root)
            .unwrap()
            .is_some());
        let made = summary(&root).unwrap();
        assert_eq!(
            (made.topic.as_str(), made.format),
            ("Kimi's audit", Format::Short)
        );
        assert!(made.stages.iter().all(|s| s.state == StageState::Todo));
        assert_eq!(made.next, Some("research"));
        assert_eq!(list(dir.path()).len(), 1);
        assert!(create(dir.path(), "   ", Format::Long, &[]).is_err());
        assert!(create(dir.path(), &"x".repeat(201), Format::Long, &[]).is_err());
        // An ordinary project is not a production.
        let plain =
            crate::project::folder::create_project_folder(dir.path(), "Plain", None).unwrap();
        assert!(summary(&plain).is_none());
        assert_eq!(list(dir.path()).len(), 1);
    }

    #[test]
    fn a_changed_stage_makes_every_later_stage_stale() {
        let dir = tempfile::tempdir().unwrap();
        let root = create(dir.path(), "Astra", Format::Long, &[]).unwrap();
        write_stage(&root, "research", "facts v1");
        write_stage(&root, "script", "script v1");
        write_stage(&root, "review", "ok");
        use StageState::*;
        assert_eq!(states(&root), [Done, Done, Done, Todo, Todo, Todo, Todo]);
        assert_eq!(summary(&root).unwrap().next, Some("voice"));
        // New research: the script and its review were made from the old facts.
        write_stage(&root, "research", "facts v2");
        assert_eq!(states(&root), [Done, Stale, Stale, Todo, Todo, Todo, Todo]);
        assert_eq!(summary(&root).unwrap().next, Some("script"));
        // Writing the script again leaves the review stale until it is redone.
        write_stage(&root, "script", "script v2");
        assert_eq!(states(&root), [Done, Done, Stale, Todo, Todo, Todo, Todo]);
        write_stage(&root, "review", "ok");
        assert_eq!(states(&root), [Done, Done, Done, Todo, Todo, Todo, Todo]);
        // A stage can't be recorded before its file or the stage before it exists.
        assert!(record_stage(&root, "package").is_err());
        assert!(record_stage(&root, "nope").is_err());
    }

    #[test]
    fn the_package_stage_follows_the_videos_publish_package() {
        let dir = tempfile::tempdir().unwrap();
        let root = create(dir.path(), "Packaged", Format::Long, &[]).unwrap();
        let package = |tags: &[&str]| crate::publish::Package {
            title: "A title".into(),
            description: "What it covers.".into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        };
        let state = |root: &Path| states(root)[5];
        // Before the visuals: kept, not recorded, and saving never fails.
        crate::publish::save_package(&root, "video", package(&["ai"])).unwrap();
        sync_package(&root).unwrap();
        assert!(root.join(DIR).join("package.json").is_file());
        assert_eq!(state(&root), StageState::Todo);
        for id in ["research", "script", "review", "voice", "visuals"] {
            write_stage(&root, id, id);
        }
        sync_package(&root).unwrap();
        assert_eq!(state(&root), StageState::Todo, "no thumbnail yet");
        let mut png = Vec::new();
        image::DynamicImage::new_rgb8(1280, 720)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        crate::publish::set_thumbnail(&root, "video", &png).unwrap();
        sync_package(&root).unwrap();
        assert_eq!(state(&root), StageState::Done);
        // Emptying the video's tags undoes the stage.
        crate::publish::save_package(&root, "video", package(&[])).unwrap();
        sync_package(&root).unwrap();
        assert_eq!(state(&root), StageState::Todo);
    }

    #[test]
    fn the_made_list_finds_a_story_by_topic_or_link() {
        let dir = tempfile::tempdir().unwrap();
        let entry = |project: &str, kind: &str| Made {
            topic: "Codex adware!".into(),
            link: Some("https://example.com/a".into()),
            format: Format::Short,
            project: project.into(),
            kind: kind.into(),
            youtube: None,
            at: "2026-11-07T00:00:00Z".into(),
        };
        add_made(dir.path(), entry("p1", "test")).unwrap();
        add_made(dir.path(), entry("p1", "private")).unwrap();
        add_made(dir.path(), entry("p2", "test")).unwrap();
        let made = load_made(dir.path());
        assert_eq!(made.len(), 2);
        assert_eq!(made[0].kind, "private");
        assert!(already_made(&made, "codex  ADWARE", None).is_some());
        assert!(already_made(&made, "Other", Some("https://example.com/a")).is_some());
        assert!(already_made(&made, "Other", None).is_none());
    }
}
