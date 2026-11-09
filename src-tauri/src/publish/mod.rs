//! Packaging and uploading a finished export (PUB-1, PUB-2), for every kind of video: a
//! tutorial, a short, a clip or a faceless video. Each video (`target`: `video`, or a short's
//! id) gets its own title, description, tags and thumbnail, saved in the project's `publish/`
//! folder (outside undo). Uploads go through Zernio and are journaled before they are sent, so
//! a retry after a lost reply never posts twice. Nothing is public without the user's click.
pub mod zernio;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const DIR: &str = "publish";
const PACKAGES: &str = "package.json";
const UPLOADS: &str = "uploads.json";
const SETTINGS: &str = "publish.json";
pub const MAX_TITLE: usize = 100;
pub const MAX_DESCRIPTION: usize = 5000;
pub const MAX_TAG_CHARS: usize = 500;
/// YouTube's own limit for a custom thumbnail.
pub const MAX_THUMBNAIL_BYTES: u64 = 2 * 1024 * 1024;

/// What a video is published with.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    /// At most 100 characters, the subject named early, the way people search it.
    #[serde(default)]
    pub title: String,
    /// What it covers, the chapters and the sources (no < or >).
    #[serde(default)]
    pub description: String,
    /// Names and spelling variants.
    #[serde(default)]
    pub tags: Vec<String>,
    /// The thumbnail's file name in the project's `publish/` folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    /// YouTube's "altered or synthetic content" label. On for faceless videos.
    #[serde(default)]
    pub ai_disclosure: bool,
    /// YouTube category, 28 is Science & Technology, 27 Education.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
}

/// Why a package can't be uploaded as it is, or `None`.
pub fn package_problem(p: &Package) -> Option<String> {
    let title = p.title.trim();
    if title.is_empty() {
        return Some("Give it a title".into());
    }
    if title.chars().count() > MAX_TITLE {
        return Some(format!("Keep the title to {MAX_TITLE} characters"));
    }
    if p.description.trim().is_empty() {
        return Some("Give it a description".into());
    }
    if p.description.chars().count() > MAX_DESCRIPTION {
        return Some(format!(
            "Keep the description to {MAX_DESCRIPTION} characters"
        ));
    }
    if p.tags.iter().map(|t| t.chars().count()).sum::<usize>() > MAX_TAG_CHARS {
        return Some(format!("Tags may be {MAX_TAG_CHARS} characters in all"));
    }
    if p.description.contains('<') || p.description.contains('>') {
        return Some("YouTube refuses < and > in a description".into());
    }
    None
}

/// Chapters as YouTube reads them in a description ("0:00 Intro" a line), or `None` when YouTube
/// would ignore them: it needs the first at 0:00, at least three, each at least ten seconds.
pub fn youtube_chapters(timeline: &[(u64, String)], duration_us: u64) -> Option<String> {
    if timeline.len() < 3 || timeline.first()?.0 != 0 {
        return None;
    }
    let ends = timeline
        .iter()
        .skip(1)
        .map(|(at, _)| *at)
        .chain(std::iter::once(duration_us));
    if timeline
        .iter()
        .zip(ends)
        .any(|((start, _), end)| end.saturating_sub(*start) < 10_000_000)
    {
        return None;
    }
    let long = duration_us >= 3_600_000_000;
    Some(
        timeline
            .iter()
            .map(|(at, title)| {
                let s = at / 1_000_000;
                let stamp = if long {
                    format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
                } else {
                    format!("{}:{:02}", s / 60, s % 60)
                };
                format!("{stamp} {title}")
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn dir(root: &Path) -> PathBuf {
    root.join(DIR)
}

/// Every package in the project; empty when there are none, an error when the file is damaged
/// (it is then never written over: GAP-035).
pub fn read_packages(root: &Path) -> Result<BTreeMap<String, Package>, String> {
    Ok(crate::files::read_json(&dir(root).join(PACKAGES))?.unwrap_or_default())
}

/// [`read_packages`] for showing: a damaged file reads as empty.
pub fn packages(root: &Path) -> BTreeMap<String, Package> {
    read_packages(root).unwrap_or_default()
}

pub fn package(root: &Path, target: &str) -> Package {
    packages(root).remove(target).unwrap_or_default()
}

fn check_target(target: &str) -> Result<(), String> {
    if target.is_empty()
        || target.len() > 64
        || !target
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Not a video of this project: {target}"));
    }
    Ok(())
}

/// Saves `target`'s package (the thumbnail field is kept as it is on disk).
pub fn save_package(root: &Path, target: &str, mut package: Package) -> Result<Package, String> {
    check_target(target)?;
    if package.title.chars().count() > MAX_TITLE {
        return Err(format!("Keep the title to {MAX_TITLE} characters"));
    }
    if package.description.chars().count() > MAX_DESCRIPTION {
        return Err(format!(
            "Keep the description to {MAX_DESCRIPTION} characters"
        ));
    }
    package.tags = package
        .tags
        .iter()
        .map(|t| t.trim().trim_start_matches('#').to_string())
        .filter(|t| !t.is_empty())
        .collect();
    let mut all = read_packages(root)?;
    package.thumbnail = all.get(target).and_then(|p| p.thumbnail.clone());
    all.insert(target.into(), package.clone());
    crate::files::save_json(&dir(root).join(PACKAGES), &all)?;
    Ok(package)
}

fn image_kind(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else {
        None
    }
}

/// An image's size from its header (PNG or JPEG).
pub fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

/// Sets `target`'s thumbnail from image bytes (a PNG or JPEG): checks it and keeps a copy.
pub fn set_thumbnail(root: &Path, target: &str, bytes: &[u8]) -> Result<Package, String> {
    check_target(target)?;
    let ext = image_kind(bytes).ok_or("A thumbnail must be a JPG or PNG")?;
    let (w, h) = image_size(bytes).ok_or("That image could not be read")?;
    if w < 640 || h < 360 {
        return Err(format!(
            "The thumbnail is {w}×{h}; YouTube wants at least 1280×720"
        ));
    }
    let bytes = if bytes.len() as u64 > MAX_THUMBNAIL_BYTES {
        shrink(bytes)?
    } else {
        bytes.to_vec()
    };
    let ext = if image_kind(&bytes) == Some("jpg") {
        "jpg"
    } else {
        ext
    };
    let mut all = read_packages(root)?;
    std::fs::create_dir_all(dir(root)).map_err(|e| e.to_string())?;
    // A new file under a name of its own first, then the package pointing at it, and only then
    // the old picture goes: a failure part way leaves the old thumbnail and package as they
    // were (GAP-038).
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .take(6)
        .map(|b| format!("{b:02x}"))
        .collect();
    let name = format!("thumbnail-{target}-{digest}.{ext}");
    let path = dir(root).join(&name);
    crate::files::write_atomic(&path, &bytes)?;
    let entry = all.entry(target.into()).or_default();
    let old = entry.thumbnail.replace(name.clone());
    if let Err(e) = crate::files::save_json(&dir(root).join(PACKAGES), &all) {
        if old.as_deref() != Some(name.as_str()) {
            let _ = std::fs::remove_file(&path);
        }
        return Err(e);
    }
    retire_thumbnails(root, target, &name);
    Ok(all.remove(target).unwrap_or_default())
}

/// Removes `target`'s thumbnail files other than `keep`, once the package points at `keep`.
fn retire_thumbnails(root: &Path, target: &str, keep: &str) {
    let Ok(entries) = std::fs::read_dir(dir(root)) else {
        return;
    };
    let exact = [
        format!("thumbnail-{target}.jpg"),
        format!("thumbnail-{target}.png"),
    ];
    let prefix = format!("thumbnail-{target}-");
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let digest_named = name.strip_prefix(&prefix).is_some_and(|rest| {
            rest.split_once('.').is_some_and(|(hex, ext)| {
                hex.len() == 12
                    && hex.chars().all(|c| c.is_ascii_hexdigit())
                    && (ext == "jpg" || ext == "png")
            })
        });
        if name != keep && (exact.contains(&name) || digest_named) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// An image too large for YouTube, as a 1280×720-or-smaller JPEG under 2 MB.
fn shrink(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let img = img.resize(1920, 1080, image::imageops::FilterType::Lanczos3);
    for quality in [90u8, 80, 70, 60] {
        let mut out = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
        img.to_rgb8()
            .write_with_encoder(encoder)
            .map_err(|e| e.to_string())?;
        if out.len() as u64 <= MAX_THUMBNAIL_BYTES {
            return Ok(out);
        }
    }
    Err("The thumbnail stays over 2 MB even compressed".into())
}

pub fn thumbnail_path(root: &Path, package: &Package) -> Option<PathBuf> {
    package
        .thumbnail
        .as_ref()
        .map(|name| dir(root).join(name))
        .filter(|p| p.is_file())
}

/// App-wide upload settings: the default channel and category.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
}

pub fn settings(config: &Path) -> Settings {
    crate::files::load_json(&config.join(SETTINGS)).unwrap_or_default()
}

pub fn save_settings(config: &Path, settings: &Settings) -> Result<(), String> {
    crate::files::save_json(&config.join(SETTINGS), settings)
}

/// How a video goes up.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Release {
    /// Private on YouTube now, for checking it there.
    Private,
    /// Public at `at` (a future time). Zernio puts it on YouTube as private up to 15 minutes
    /// before, with YouTube's own release time.
    Scheduled { at: DateTime<Utc> },
    /// Kept in Zernio only; nothing reaches YouTube.
    Draft,
}

/// One upload, journaled before it is sent.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Upload {
    pub id: String,
    pub target: String,
    pub file: String,
    pub title: String,
    /// The description, tags, labels and thumbnail it was sent with (a digest).
    #[serde(default)]
    pub package_digest: String,
    pub channel: String,
    pub release: Release,
    /// Zernio's idempotency key: the same file, channel and title always give the same key.
    pub key: String,
    pub attempted_at: DateTime<Utc>,
    /// `submitting`, `submitted`, `scheduled`, `published`, `draft` or `failed`.
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub youtube_id: Option<String>,
    /// What YouTube says now (Zernio's post-level visibility is always "public").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Upload {
    pub fn studio_link(&self) -> Option<String> {
        self.youtube_id
            .as_ref()
            .map(|id| format!("https://studio.youtube.com/video/{id}/edit"))
    }
}

/// The upload journal; an error when it is damaged, so nothing is posted twice or written over.
pub fn read_uploads(root: &Path) -> Result<Vec<Upload>, String> {
    Ok(crate::files::read_json(&dir(root).join(UPLOADS))?.unwrap_or_default())
}

/// [`read_uploads`] for showing: a damaged journal reads as empty.
pub fn uploads(root: &Path) -> Vec<Upload> {
    read_uploads(root).unwrap_or_default()
}

fn save_uploads(root: &Path, all: &[Upload]) -> Result<(), String> {
    crate::files::save_json(&dir(root).join(UPLOADS), &all)
}

fn record(root: &Path, upload: &Upload) -> Result<(), String> {
    let mut all = read_uploads(root)?;
    match all.iter_mut().find(|u| u.id == upload.id) {
        Some(existing) => *existing = upload.clone(),
        None => all.push(upload.clone()),
    }
    save_uploads(root, &all)
}

fn file_digest(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// A stable key per file, channel and title, shaped like a UUID.
pub fn upload_key(digest: &str, channel: &str, title: &str) -> String {
    let hash = Sha256::digest(format!("aeroedits:{channel}:{title}:{digest}").as_bytes());
    let h: String = hash.iter().take(16).map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// The request Zernio gets, built in full before anything is sent.
pub fn post_body(
    package: &Package,
    channel: &str,
    category: &str,
    release: &Release,
    video_url: &str,
    thumbnail_url: Option<&str>,
) -> Value {
    let mut media = json!({ "type": "video", "url": video_url });
    if let Some(thumb) = thumbnail_url {
        media["thumbnail"] = json!(thumb);
    }
    let visibility = match release {
        Release::Scheduled { .. } => "public",
        _ => "private",
    };
    let mut body = json!({
        "title": package.title.trim(),
        // YouTube takes the description from the post's content.
        "content": package.description,
        "tags": package.tags,
        "mediaItems": [media],
        "platforms": [{
            "platform": "youtube",
            "accountId": channel,
            "platformSpecificData": {
                "title": package.title.trim(),
                "visibility": visibility,
                "categoryId": package.category_id.as_deref().unwrap_or(category),
                // Sent every time: YouTube may block views when it is unset.
                "madeForKids": false,
                "containsSyntheticMedia": package.ai_disclosure,
            }
        }],
    });
    match release {
        Release::Draft => body["isDraft"] = json!(true),
        Release::Scheduled { at } => {
            body["scheduledFor"] = json!(at.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        }
        Release::Private => body["publishNow"] = json!(true),
    }
    body
}

/// A digest of what an upload sends besides the file and title: description, tags, labels,
/// category and the thumbnail's bytes.
fn package_digest(root: &Path, package: &Package) -> String {
    let thumbnail = thumbnail_path(root, package)
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| Sha256::digest(&b).to_vec())
        .unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(
        serde_json::to_vec(&(
            &package.description,
            &package.tags,
            package.ai_disclosure,
            &package.category_id,
        ))
        .unwrap_or_default(),
    );
    hasher.update(&thumbnail);
    hasher
        .finalize()
        .iter()
        .take(12)
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn release_label(release: &Release) -> String {
    match release {
        Release::Private => "private".into(),
        Release::Draft => "a draft".into(),
        Release::Scheduled { at } => format!("scheduled for {}", at.format("%Y-%m-%d %H:%M UTC")),
    }
}

/// What differs between an earlier upload and this request, if anything.
fn changed_since(earlier: &Upload, release: &Release, digest: &str) -> Option<String> {
    if &earlier.release != release {
        return Some(format!("you asked for {} now", release_label(release)));
    }
    (earlier.package_digest != digest)
        .then(|| "its description, tags, labels or thumbnail changed since".to_string())
}

/// Fills an upload from Zernio's post: its status, the YouTube id and real visibility.
fn apply_post(upload: &mut Upload, post: &Value) {
    if let Some(id) = post["_id"].as_str() {
        upload.post_id = Some(id.into());
    }
    let target = zernio::youtube_target(post);
    if let Some(id) = zernio::video_id(&target) {
        upload.youtube_id = Some(id);
    }
    if let Some(v) = target["platformSpecificData"]["visibility"].as_str() {
        upload.visibility = Some(v.into());
    }
    let failed = ["failed", "partial"];
    let status = target["status"]
        .as_str()
        .filter(|s| failed.contains(s))
        .or(post["status"].as_str())
        .unwrap_or("submitted");
    upload.status = status.into();
    if failed.contains(&status) {
        upload.error = target["errorMessage"].as_str().map(str::to_string);
    }
}

/// Uploads `file` (a finished export) as `target` with its package. Refuses a title or
/// description YouTube would reject before anything is sent. An earlier upload of the same file
/// with the same title and channel is not posted again: its status is read instead.
#[allow(clippy::too_many_arguments)]
pub fn upload(
    root: &Path,
    client: &zernio::Client,
    target: &str,
    file: &Path,
    release: Release,
    channel: &str,
    category: &str,
    now: DateTime<Utc>,
    progress: &dyn Fn(&str),
) -> Result<Upload, String> {
    check_target(target)?;
    let package = read_packages(root)?.remove(target).unwrap_or_default();
    if let Some(problem) = package_problem(&package) {
        return Err(problem);
    }
    if let Release::Scheduled { at } = &release {
        if *at <= now + chrono::Duration::minutes(5) {
            return Err("Schedule it at least a few minutes ahead: a time in the past would publish it at once".into());
        }
    }
    if !file.is_file() {
        return Err(format!(
            "The export is not there any more: {}",
            file.display()
        ));
    }
    progress("Checking the file");
    let key = upload_key(&file_digest(file)?, channel, package.title.trim());
    let sent = package_digest(root, &package);
    if let Some(mut earlier) = read_uploads(root)?
        .into_iter()
        .find(|u| u.key == key && u.status != "failed")
    {
        // The same export, channel and title went up before. Only an exact retry is answered
        // from it; a new release or changed details would otherwise be dropped without a word
        // (GAP-002).
        if let Some(change) = changed_since(&earlier, &release, &sent) {
            return Err(format!(
                "This export already went to Zernio on {} as {}, and {change}. Uploading again \
                 would not change that post: change it in YouTube Studio{} or the Zernio \
                 dashboard ({}), or change the title to send a new video.",
                earlier.attempted_at.format("%Y-%m-%d %H:%M UTC"),
                release_label(&earlier.release),
                earlier
                    .studio_link()
                    .map_or(String::new(), |l| format!(" ({l})")),
                zernio::DASHBOARD_URL
            ));
        }
        // Already sent: read where it stands instead of posting twice.
        if let Some(post_id) = earlier.post_id.clone() {
            if let Ok(post) = client.post(&post_id) {
                apply_post(&mut earlier, &post);
                record(root, &earlier)?;
            }
            return Ok(earlier);
        }
        // Sent, but the reply was lost: look for the post Zernio may have made.
        let found: Vec<Value> = client
            .recent_posts(channel)?
            .into_iter()
            .filter(|p| p["title"].as_str() == Some(package.title.trim()))
            .collect();
        if found.len() == 1 {
            apply_post(&mut earlier, &found[0]);
            record(root, &earlier)?;
            return Ok(earlier);
        }
        if found.len() > 1 {
            return Err("Zernio has several posts with this title: remove the extra ones in the Zernio dashboard first".into());
        }
    }
    progress("Uploading the video to Zernio");
    let video_url = client.upload_media(file)?;
    let thumb_url = match thumbnail_path(root, &package) {
        Some(path) => {
            progress("Uploading the thumbnail");
            Some(client.upload_media(&path)?)
        }
        None => None,
    };
    let body = post_body(
        &package,
        channel,
        category,
        &release,
        &video_url,
        thumb_url.as_deref(),
    );
    let mut upload = Upload {
        id: format!("u-{}", now.format("%Y%m%d%H%M%S%3f")),
        target: target.into(),
        file: file.to_string_lossy().into_owned(),
        title: package.title.trim().into(),
        package_digest: sent,
        channel: channel.into(),
        release: release.clone(),
        key: key.clone(),
        attempted_at: now,
        status: "submitting".into(),
        post_id: None,
        youtube_id: None,
        visibility: None,
        error: None,
    };
    // Journaled before sending: a lost reply is found again, never posted twice.
    record(root, &upload)?;
    progress("Posting to YouTube");
    match client.create_post(&body, &key) {
        Ok(post) => {
            apply_post(&mut upload, &post);
            if matches!(release, Release::Draft) {
                upload.status = "draft".into();
            }
        }
        Err(e) => {
            // A reply that never came may still be a post: keep it as sent, so the next try
            // looks for it instead of posting again. A refusal is a failure.
            let lost = e.contains("in time") || e.contains("Could not reach");
            upload.status = if lost { "submitting" } else { "failed" }.into();
            upload.error = Some(e.clone());
            record(root, &upload)?;
            return Err(if lost {
                format!("{e}. It may still have gone through: upload again to check, it won't be posted twice")
            } else {
                e
            });
        }
    }
    record(root, &upload)?;
    Ok(upload)
}

/// Reads an upload's status again from Zernio.
pub fn refresh(root: &Path, client: &zernio::Client, id: &str) -> Result<Upload, String> {
    let mut upload = read_uploads(root)?
        .into_iter()
        .find(|u| u.id == id)
        .ok_or("No such upload")?;
    let post_id = upload
        .post_id
        .clone()
        .ok_or("Zernio never answered this upload: upload it again, it won't be posted twice")?;
    let post = client.post(&post_id)?;
    apply_post(&mut upload, &post);
    record(root, &upload)?;
    Ok(upload)
}

#[cfg(test)]
mod tests {
    use super::zernio::{Client, Http};
    use super::*;
    use parking_lot::Mutex;

    #[derive(Default)]
    struct Fake {
        calls: Mutex<Vec<String>>,
        posts: Mutex<usize>,
        lose_reply: Mutex<bool>,
    }

    impl Http for Fake {
        fn json(
            &self,
            method: &str,
            url: &str,
            body: Option<&Value>,
            key: Option<&str>,
        ) -> Result<(u16, Value), String> {
            self.calls.lock().push(format!("{method} {url}"));
            if url.ends_with("/media/presign") {
                let name = body.unwrap()["filename"].as_str().unwrap().to_string();
                return Ok((
                    200,
                    json!({ "uploadUrl": format!("https://s3/{name}?sig"), "publicUrl": format!("https://cdn/{name}") }),
                ));
            }
            if url.ends_with("/posts") && method == "POST" {
                assert!(key.is_some());
                *self.posts.lock() += 1;
                if *self.lose_reply.lock() {
                    return Err("Zernio did not answer in time".into());
                }
                return Ok((
                    200,
                    json!({ "post": { "_id": "p1", "status": "published", "platforms": [
                        { "platform": "youtube", "status": "published", "platformPostId": "abcdefghijk",
                          "platformSpecificData": { "visibility": "private" } }] } }),
                ));
            }
            if url.contains("/posts?") {
                return Ok((
                    200,
                    json!({ "posts": [{ "_id": "p1", "title": "My video", "status": "published", "platforms": [
                        { "platform": "youtube", "platformPostUrl": "https://youtu.be/abcdefghijk",
                          "platformSpecificData": { "visibility": "private" } }] }] }),
                ));
            }
            if url.ends_with("/posts/p1") {
                return Ok((
                    200,
                    json!({ "post": { "_id": "p1", "status": "published", "platforms": [
                        { "platform": "youtube", "platformPostId": "abcdefghijk",
                          "platformSpecificData": { "visibility": "public" } }] } }),
                ));
            }
            Ok((404, json!({ "error": "no" })))
        }

        fn put_file(&self, url: &str, _path: &Path, _kind: &str) -> Result<u16, String> {
            self.calls.lock().push(format!("PUT {url}"));
            Ok(200)
        }
    }

    fn project() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("p");
        std::fs::create_dir_all(&root).unwrap();
        let file = dir.path().join("video.mp4");
        std::fs::write(&file, b"not really a video").unwrap();
        (dir, root, file)
    }

    #[test]
    fn a_package_must_be_one_youtube_takes() {
        let mut p = Package::default();
        assert!(package_problem(&p).unwrap().contains("title"));
        p.title = "x".repeat(101);
        assert!(package_problem(&p).is_some());
        p.title = "My video".into();
        assert!(package_problem(&p).unwrap().contains("description"));
        p.description = "About <this>".into();
        assert!(package_problem(&p).is_some());
        p.description = "About this".into();
        assert_eq!(package_problem(&p), None);
        p.tags = vec!["t".repeat(501)];
        assert!(package_problem(&p).is_some());
    }

    #[test]
    fn upload_posts_once_and_a_retry_after_a_lost_reply_finds_the_post() {
        let (_dir, root, file) = project();
        save_package(
            &root,
            "video",
            Package {
                title: "My video".into(),
                description: "What it shows".into(),
                tags: vec!["#AI".into(), " ".into()],
                ai_disclosure: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(package(&root, "video").tags, ["AI"]);
        let fake = Fake::default();
        *fake.lose_reply.lock() = true;
        let client = Client { http: &fake };
        let now = Utc::now();
        // The reply is lost: the upload stays journaled as sent, with its key.
        let err = upload(
            &root,
            &client,
            "video",
            &file,
            Release::Private,
            "ch1",
            "28",
            now,
            &|_| {},
        )
        .unwrap_err();
        assert!(err.contains("won't be posted twice"));
        assert_eq!(uploads(&root)[0].status, "submitting");
        // Trying again finds the post Zernio made instead of posting again.
        *fake.lose_reply.lock() = false;
        let found = upload(
            &root,
            &client,
            "video",
            &file,
            Release::Private,
            "ch1",
            "28",
            now,
            &|_| {},
        )
        .unwrap();
        assert_eq!(*fake.posts.lock(), 1, "posted twice");
        assert_eq!(found.youtube_id.as_deref(), Some("abcdefghijk"));
        assert_eq!(found.visibility.as_deref(), Some("private"));
        // Reading its status later shows what YouTube says now.
        let later = refresh(&root, &client, &found.id).unwrap();
        assert_eq!(later.visibility.as_deref(), Some("public"));
        assert_eq!(
            later.studio_link().as_deref(),
            Some("https://studio.youtube.com/video/abcdefghijk/edit")
        );
    }

    #[test]
    fn the_request_says_private_scheduled_or_draft_and_never_kids() {
        let p = Package {
            title: " T ".into(),
            description: "D".into(),
            ai_disclosure: true,
            ..Default::default()
        };
        let at = "2026-12-01T15:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let body = post_body(
            &p,
            "ch",
            "28",
            &Release::Scheduled { at },
            "https://v",
            Some("https://t"),
        );
        let yt = &body["platforms"][0]["platformSpecificData"];
        assert_eq!(yt["visibility"], "public");
        assert_eq!(yt["madeForKids"], false);
        assert_eq!(yt["containsSyntheticMedia"], true);
        assert_eq!(body["scheduledFor"], "2026-12-01T15:00:00Z");
        assert_eq!(body["title"], "T");
        assert_eq!(body["mediaItems"][0]["thumbnail"], "https://t");
        let private = post_body(&p, "ch", "28", &Release::Private, "https://v", None);
        assert_eq!(private["publishNow"], true);
        assert_eq!(
            private["platforms"][0]["platformSpecificData"]["visibility"],
            "private"
        );
        assert_eq!(
            post_body(&p, "ch", "28", &Release::Draft, "v", None)["isDraft"],
            true
        );
        // A time in the past would publish at once.
        let (_dir, root, file) = project();
        save_package(&root, "video", p).unwrap();
        let fake = Fake::default();
        let soon = Release::Scheduled { at: Utc::now() };
        assert!(upload(
            &root,
            &Client { http: &fake },
            "video",
            &file,
            soon,
            "ch",
            "28",
            Utc::now(),
            &|_| {}
        )
        .unwrap_err()
        .contains("ahead"));
        assert_eq!(*fake.posts.lock(), 0);
    }

    #[test]
    fn thumbnails_are_checked_and_kept() {
        let (_dir, root, _) = project();
        let mut png = Vec::new();
        image::RgbImage::new(1280, 720)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let p = set_thumbnail(&root, "video", &png).unwrap();
        let name = p.thumbnail.clone().unwrap();
        assert!(
            name.starts_with("thumbnail-video-") && name.ends_with(".png"),
            "{name}"
        );
        assert!(thumbnail_path(&root, &p).is_some());
        let mut small = Vec::new();
        image::RgbImage::new(100, 100)
            .write_to(
                &mut std::io::Cursor::new(&mut small),
                image::ImageFormat::Png,
            )
            .unwrap();
        assert!(set_thumbnail(&root, "video", &small).is_err());
        assert!(set_thumbnail(&root, "video", b"GIF89a").is_err());
        assert!(set_thumbnail(&root, "../x", &png).is_err());
        // Saving the words keeps the thumbnail.
        let saved = save_package(
            &root,
            "video",
            Package {
                title: "x".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(saved.thumbnail.as_deref(), Some(name.as_str()));
    }

    fn png(width: u32) -> Vec<u8> {
        let mut out = Vec::new();
        image::RgbImage::new(width, 720)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    /// GAP-038: a replacement that fails part way leaves the old thumbnail and package usable;
    /// one that works removes the old picture.
    #[test]
    fn a_failed_thumbnail_replacement_keeps_the_old_one() {
        let (_dir, root, _) = project();
        let first = set_thumbnail(&root, "video", &png(1280)).unwrap();
        let old = thumbnail_path(&root, &first).unwrap();
        let old_bytes = std::fs::read(&old).unwrap();
        // The package can't be written: a folder stands where it goes.
        let packages_file = dir(&root).join(PACKAGES);
        let saved = std::fs::read(&packages_file).unwrap();
        std::fs::remove_file(&packages_file).unwrap();
        std::fs::create_dir(&packages_file).unwrap();
        assert!(set_thumbnail(&root, "video", &png(1300)).is_err());
        std::fs::remove_dir(&packages_file).unwrap();
        std::fs::write(&packages_file, saved).unwrap();
        assert_eq!(
            std::fs::read(&old).unwrap(),
            old_bytes,
            "the old picture stays"
        );
        assert_eq!(package(&root, "video").thumbnail, first.thumbnail);
        let thumbnails = || {
            std::fs::read_dir(dir(&root))
                .unwrap()
                .filter(|e| {
                    let e = e.as_ref().unwrap();
                    e.file_name().to_string_lossy().starts_with("thumbnail-")
                })
                .count()
        };
        assert_eq!(thumbnails(), 1, "the half-made new one is cleaned up");
        let second = set_thumbnail(&root, "video", &png(1300)).unwrap();
        assert_ne!(second.thumbnail, first.thumbnail);
        assert!(
            !old.exists(),
            "replaced once the package points at the new one"
        );
        assert_eq!(thumbnails(), 1);
    }

    /// GAP-035: a damaged package file is reported and never written over.
    #[test]
    fn a_damaged_package_file_is_never_overwritten() {
        let (_dir, root, _) = project();
        let path = dir(&root).join(PACKAGES);
        std::fs::create_dir_all(dir(&root)).unwrap();
        let damaged = br#"{"video":{"title":"Main"},"s1":{"title":"Short dra"#;
        std::fs::write(&path, damaged).unwrap();
        assert!(read_packages(&root).unwrap_err().contains("damaged"));
        let words = Package {
            title: "New".into(),
            ..Default::default()
        };
        assert!(save_package(&root, "video", words).is_err());
        assert!(set_thumbnail(&root, "video", &png(1280)).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), damaged, "left as it was");
        // A damaged journal blocks uploads and records too.
        std::fs::remove_file(&path).unwrap();
        std::fs::write(dir(&root).join(UPLOADS), b"[{").unwrap();
        assert!(read_uploads(&root).is_err());
        // A missing file is simply empty.
        std::fs::remove_file(dir(&root).join(UPLOADS)).unwrap();
        assert!(read_packages(&root).unwrap().is_empty());
        assert!(read_uploads(&root).unwrap().is_empty());
    }

    /// GAP-002: once an export is sent, only an exact retry is answered from that upload; a new
    /// release or changed details are refused with what to do, and nothing is posted.
    #[test]
    fn a_changed_release_or_package_is_not_taken_for_a_retry() {
        let (_dir, root, file) = project();
        let words = |description: &str| Package {
            title: "My video".into(),
            description: description.into(),
            ..Default::default()
        };
        save_package(&root, "video", words("What it shows")).unwrap();
        let fake = Fake::default();
        let client = Client { http: &fake };
        let now = Utc::now();
        let send = |release: Release| {
            upload(
                &root,
                &client,
                "video",
                &file,
                release,
                "ch1",
                "28",
                now,
                &|_| {},
            )
        };
        send(Release::Draft).unwrap();
        assert_eq!(*fake.posts.lock(), 1);
        // Exactly the same again: answered from the first, not posted twice.
        assert_eq!(send(Release::Draft).unwrap().release, Release::Draft);
        let at = now + chrono::Duration::days(1);
        let scheduled = send(Release::Scheduled { at }).unwrap_err();
        assert!(scheduled.contains("as a draft"), "{scheduled}");
        assert!(scheduled.contains("scheduled for"), "{scheduled}");
        assert!(send(Release::Private).unwrap_err().contains("private now"));
        save_package(&root, "video", words("Better words")).unwrap();
        assert!(send(Release::Draft).unwrap_err().contains("description"));
        assert_eq!(*fake.posts.lock(), 1, "nothing more was posted");
        assert_eq!(read_uploads(&root).unwrap().len(), 1);
    }

    #[test]
    fn chapters_are_written_the_way_youtube_reads_them() {
        let t = |s: u64, name: &str| (s * 1_000_000, name.to_string());
        let chapters = [t(0, "Intro"), t(65, "Setup"), t(600, "Result")];
        assert_eq!(
            youtube_chapters(&chapters, 900_000_000).as_deref(),
            Some("0:00 Intro\n1:05 Setup\n10:00 Result")
        );
        // Two chapters, a late first one, or a short one: YouTube would ignore them.
        assert_eq!(youtube_chapters(&chapters[..2], 900_000_000), None);
        assert_eq!(
            youtube_chapters(&[t(5, "a"), t(65, "b"), t(600, "c")], 900_000_000),
            None
        );
        assert_eq!(
            youtube_chapters(&[t(0, "a"), t(5, "b"), t(600, "c")], 900_000_000),
            None
        );
    }

    #[test]
    fn youtube_ids_come_from_the_id_or_the_link() {
        use super::zernio::video_id;
        assert_eq!(
            video_id(&json!({ "platformPostId": "abcdefghijk" })).as_deref(),
            Some("abcdefghijk")
        );
        assert_eq!(
            video_id(&json!({ "platformPostUrl": "https://www.youtube.com/shorts/abcdefghijk" }))
                .as_deref(),
            Some("abcdefghijk")
        );
        assert_eq!(
            video_id(&json!({ "platformPostUrl": "https://youtube.com/watch?v=abcdefghijk" }))
                .as_deref(),
            Some("abcdefghijk")
        );
        assert_eq!(video_id(&json!({ "platformPostId": "short" })), None);
    }
}
