//! Packaging and uploading exports (PUB-1, PUB-2): the Export dialog's Publish part, the
//! Accounts dialog, and the MCP `publish` tool call these.
use super::AppState;
use crate::publish::{self, zernio, Package, Release, Upload};
use base64::Engine;
use serde::Serialize;
use std::path::{Path, PathBuf};

fn config() -> PathBuf {
    crate::transcript::settings::config_dir()
}

fn project_root(state: &AppState) -> Result<PathBuf, String> {
    let guard = state.opened_project.lock();
    Ok(guard
        .as_ref()
        .ok_or(crate::mcp::tools::NO_PROJECT_MESSAGE)?
        .root()
        .to_path_buf())
}

/// The Zernio connection: whether a key is set, and the YouTube channels it reaches.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Accounts {
    pub connected: bool,
    /// `keychain`, `environment` or `file`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_source: Option<String>,
    pub channels: Vec<zernio::Channel>,
    /// The channel uploads go to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub dashboard: &'static str,
}

fn key() -> Option<(String, &'static str)> {
    crate::secrets::get(&config(), zernio::KEY)
}

/// Reads the connected channels from Zernio (a network call), and picks the only one as default.
pub fn publish_accounts_impl() -> Accounts {
    let mut settings = publish::settings(&config());
    let Some((key, source)) = key() else {
        return Accounts {
            connected: false,
            key_source: None,
            channels: Vec::new(),
            channel: None,
            error: None,
            dashboard: zernio::DASHBOARD_URL,
        };
    };
    let result = zernio::Web::new(key).and_then(|web| zernio::Client { http: &web }.channels());
    let (channels, error) = match result {
        Ok(c) => (c, None),
        Err(e) => (Vec::new(), Some(e)),
    };
    let active: Vec<&zernio::Channel> = channels.iter().filter(|c| c.active).collect();
    let known = settings
        .channel
        .as_ref()
        .is_some_and(|id| channels.iter().any(|c| &c.id == id));
    if !known && active.len() == 1 && error.is_none() {
        settings.channel = Some(active[0].id.clone());
        let _ = publish::save_settings(&config(), &settings);
    }
    Accounts {
        connected: error.is_none(),
        key_source: Some(source.to_string()),
        channels,
        channel: settings.channel,
        error,
        dashboard: zernio::DASHBOARD_URL,
    }
}

pub fn publish_set_key_impl(key: String) -> Result<Accounts, String> {
    crate::secrets::set(&config(), zernio::KEY, &key)?;
    Ok(publish_accounts_impl())
}

pub fn publish_set_channel_impl(channel: String) -> Result<(), String> {
    let mut settings = publish::settings(&config());
    settings.channel = Some(channel);
    publish::save_settings(&config(), &settings)
}

/// A video's package as the dialog shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageView {
    pub target: String,
    pub package: Package,
    /// The thumbnail as a data URL, for the preview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_size: Option<(u32, u32)>,
    /// Why it can't be uploaded yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    /// Earlier uploads of this video, newest first.
    pub uploads: Vec<Upload>,
    /// The video's chapters as YouTube reads them in a description, when it has enough.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapters: Option<String>,
}

fn chapters_text(state: &AppState, target: &str) -> Option<String> {
    if target != "video" {
        return None;
    }
    let guard = state.opened_project.lock();
    let document = guard.as_ref()?.document();
    let timeline = crate::chapters::timeline(&document.chapters, document);
    publish::youtube_chapters(&timeline, document.sequence.duration_us())
}

fn view_with_chapters(state: &AppState, root: &Path, target: &str) -> PackageView {
    let mut v = view(root, target);
    v.chapters = chapters_text(state, target);
    v
}

fn view(root: &Path, target: &str) -> PackageView {
    let mut package = publish::package(root, target);
    // Faceless videos always carry the AI label; a new package starts with it on.
    if !publish::packages(root).contains_key(target) && crate::faceless::load(root).is_some() {
        package.ai_disclosure = true;
    }
    let thumbnail = publish::thumbnail_path(root, &package).and_then(|p| std::fs::read(p).ok());
    let thumbnail_data = thumbnail.as_ref().map(|bytes| {
        let kind = if bytes.starts_with(&[0xFF, 0xD8]) {
            "image/jpeg"
        } else {
            "image/png"
        };
        format!(
            "data:{kind};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    });
    let mut uploads: Vec<Upload> = publish::uploads(root)
        .into_iter()
        .filter(|u| u.target == target)
        .collect();
    uploads.reverse();
    PackageView {
        target: target.into(),
        problem: publish::package_problem(&package),
        thumbnail_size: thumbnail.as_deref().and_then(publish::image_size),
        thumbnail_data,
        package,
        uploads,
        chapters: None,
    }
}

pub fn publish_package_impl(state: &AppState, target: String) -> Result<PackageView, String> {
    Ok(view_with_chapters(state, &project_root(state)?, &target))
}

pub fn publish_package_save_impl(
    state: &AppState,
    target: String,
    package: Package,
) -> Result<PackageView, String> {
    let root = project_root(state)?;
    publish::save_package(&root, &target, package)?;
    crate::faceless::sync_package(&root)?;
    Ok(view_with_chapters(state, &root, &target))
}

/// The thumbnail from an image file.
pub fn publish_thumbnail_file_impl(
    state: &AppState,
    target: String,
    path: String,
) -> Result<PackageView, String> {
    let root = project_root(state)?;
    let bytes = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
    publish::set_thumbnail(&root, &target, &bytes)?;
    crate::faceless::sync_package(&root)?;
    Ok(view_with_chapters(state, &root, &target))
}

/// The thumbnail from a frame of the exported video.
pub fn publish_thumbnail_frame_impl(
    state: &AppState,
    target: String,
    file: String,
    at_us: u64,
) -> Result<PackageView, String> {
    let root = project_root(state)?;
    let bytes = crate::media::ffmpeg::frame_jpeg(Path::new(&file), at_us, 1920)?;
    publish::set_thumbnail(&root, &target, &bytes)?;
    crate::faceless::sync_package(&root)?;
    Ok(view_with_chapters(state, &root, &target))
}

/// The thumbnail from a frame of the edit as it is now (graphics drawn), no export needed:
/// the video's timeline, or short `target`'s own.
pub fn publish_thumbnail_edit_impl(
    state: &AppState,
    target: String,
    at_us: u64,
) -> Result<PackageView, String> {
    let root = project_root(state)?;
    let short = (target != "video").then_some(target.as_str());
    crate::mcp::frames::wait_for_graphics(state, short, at_us, at_us)?;
    let (frame, _) = crate::mcp::frames::render(state, short, at_us, 1920)?;
    let bytes = crate::mcp::frames::png(&frame)?;
    publish::set_thumbnail(&root, &target, &bytes)?;
    crate::faceless::sync_package(&root)?;
    Ok(view_with_chapters(state, &root, &target))
}

/// Uploads a finished export with its package to the chosen channel.
pub fn publish_upload_impl(
    state: &AppState,
    target: String,
    file: String,
    release: Release,
    progress: &dyn Fn(&str),
) -> Result<Upload, String> {
    let root = project_root(state)?;
    let (key, _) = key().ok_or("Connect Zernio first: File → Accounts")?;
    let settings = publish::settings(&config());
    let channel = settings
        .channel
        .clone()
        .ok_or("Pick the YouTube channel first: File → Accounts")?;
    let web = zernio::Web::new(key)?;
    let client = zernio::Client { http: &web };
    let upload = publish::upload(
        &root,
        &client,
        &target,
        Path::new(&file),
        release,
        &channel,
        settings.category_id.as_deref().unwrap_or("28"),
        chrono::Utc::now(),
        progress,
    )?;
    crate::faceless::record_upload(&root, &upload)?;
    // A faceless video goes on the made list, so its story is never made twice.
    if let Some(production) = crate::faceless::load(&root) {
        let kind = match (&upload.release, upload.visibility.as_deref()) {
            (_, Some("public")) | (Release::Scheduled { .. }, _) => "public",
            (Release::Draft, _) => "test",
            _ => "private",
        };
        let _ = crate::faceless::add_made(
            &config(),
            crate::faceless::Made {
                topic: production.topic,
                link: production.links.first().cloned(),
                format: production.format,
                project: root.to_string_lossy().into_owned(),
                kind: kind.into(),
                youtube: upload.youtube_id.clone(),
                at: chrono::Utc::now().to_rfc3339(),
            },
        );
    }
    Ok(upload)
}

pub fn publish_refresh_impl(state: &AppState, id: String) -> Result<Upload, String> {
    let root = project_root(state)?;
    let (key, _) = key().ok_or("Connect Zernio first: File → Accounts")?;
    let web = zernio::Web::new(key)?;
    let upload = publish::refresh(&root, &zernio::Client { http: &web }, &id)?;
    // YouTube's id can arrive after the upload.
    crate::faceless::record_upload(&root, &upload)?;
    Ok(upload)
}
