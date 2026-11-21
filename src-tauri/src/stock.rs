//! Stock footage and photos from Pexels with the user's own key (FTG-6). The agent searches
//! and sees previews only; it proposes the ones it wants as a batch, and nothing is downloaded
//! until the user accepts it in the Agent panel. Each file is then saved in the project's
//! `media/stock/` folder and imported with its licence (Pexels': free to use, no credit
//! needed) and where it came from.
//!
//! The key lives in the system keychain (`PEXELS_API_KEY` overrides it), never in a project.
//! Pexels is free; its limit is about 200 requests an hour.
use crate::sequence::{Licence, LicenceKind};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const API: &str = "https://api.pexels.com";
pub const KEYS_URL: &str = "https://www.pexels.com/api/";
pub const LICENCE_URL: &str = "https://www.pexels.com/license/";
pub const KEY: crate::secrets::KeySpec = crate::secrets::KeySpec {
    env: "PEXELS_API_KEY",
    name: "pexels-api-key",
};
/// The project folder stock files are saved in.
pub const DIR: &str = "media/stock";
/// Results shown unless told otherwise, and at most.
pub const LIMIT_DEFAULT: usize = 8;
pub const LIMIT_MAX: usize = 20;
/// The largest side a downloaded video is chosen at, when Pexels has one that size or smaller.
const VIDEO_MAX_SIDE: u32 = 1920;
/// A file larger than this is refused.
const MAX_FILE_BYTES: u64 = 1 << 30;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum StockKind {
    #[default]
    Video,
    Photo,
}

/// One result: what the agent sees and what is downloaded once accepted.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockItem {
    /// `pexels-video-<n>` or `pexels-photo-<n>`.
    pub id: String,
    pub kind: StockKind,
    pub width: u32,
    pub height: u32,
    /// Seconds, for a video.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_s: Option<u32>,
    pub author: String,
    pub page_url: String,
    pub preview_url: String,
    pub file_url: String,
    /// What it shows: the photo's alt text, else the words of its page's address.
    pub about: String,
}

impl StockItem {
    /// A line for the agent and the user.
    pub fn line(&self) -> String {
        let length = self
            .duration_s
            .map(|s| format!("{s} s, "))
            .unwrap_or_default();
        format!(
            "{}: {}{}×{}, by {}: {}",
            self.id, length, self.width, self.height, self.author, self.about
        )
    }

    /// Its licence, as the imported media keeps it.
    pub fn licence(&self) -> Licence {
        Licence {
            kind: LicenceKind::NoCredit,
            source: Some(format!(
                "Pexels licence ({LICENCE_URL}), by {}: {}",
                self.author, self.page_url
            )),
        }
    }

    /// Its file name in [`DIR`].
    pub fn file_name(&self) -> String {
        let ext = match self.kind {
            StockKind::Video => "mp4",
            StockKind::Photo => {
                let path = self.file_url.split('?').next().unwrap_or_default();
                if path.to_ascii_lowercase().ends_with(".png") {
                    "png"
                } else {
                    "jpg"
                }
            }
        };
        format!("{}.{ext}", self.id)
    }
}

/// What reaches Pexels; tests answer from fixtures.
pub trait Fetch: Sync {
    /// An API answer, as JSON.
    fn json(&self, url: &str) -> Result<Value, String>;
    /// A file's bytes saved at `to`.
    fn save(&self, url: &str, to: &Path) -> Result<u64, String>;
    /// A preview picture's bytes.
    fn preview(&self, url: &str) -> Result<Vec<u8>, String>;
}

pub struct Web {
    client: reqwest::blocking::Client,
    key: String,
}

impl Web {
    pub fn new(key: String) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(600))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client, key })
    }

    /// The key in the keychain (or `PEXELS_API_KEY`).
    pub fn from_keychain() -> Result<Self, String> {
        let config = crate::transcript::settings::config_dir();
        let (key, _) = crate::secrets::get(&config, KEY).ok_or(
            "No Pexels key: the user adds theirs (free, from pexels.com/api) in Settings → \
             Accounts",
        )?;
        Self::new(key)
    }
}

impl Fetch for Web {
    fn json(&self, url: &str) -> Result<Value, String> {
        let response = self
            .client
            .get(url)
            .header("Authorization", &self.key)
            .send()
            .map_err(|e| format!("Pexels: {e}"))?;
        let status = response.status();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err("Pexels refused the key: check it in Settings → Accounts".into());
        }
        if status.as_u16() == 429 {
            return Err("Pexels' hourly limit is reached: try again later".into());
        }
        if !status.is_success() {
            return Err(format!("Pexels answered {status}"));
        }
        response.json().map_err(|e| format!("Pexels: {e}"))
    }

    fn preview(&self, url: &str) -> Result<Vec<u8>, String> {
        pexels_file(url)?;
        let response = self
            .client
            .get(url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Pexels: {e}"))?;
        Ok(response
            .bytes()
            .map_err(|e| format!("Pexels: {e}"))?
            .to_vec())
    }

    fn save(&self, url: &str, to: &Path) -> Result<u64, String> {
        pexels_file(url)?;
        let mut response = self
            .client
            .get(url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Pexels: {e}"))?;
        if response
            .content_length()
            .is_some_and(|n| n > MAX_FILE_BYTES)
        {
            return Err("That file is over 1 GB".into());
        }
        let mut file = std::fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
        response
            .copy_to(&mut file)
            .map_err(|e| format!("Pexels: {e}"))
    }
}

/// Files come from Pexels' own servers; the key is for the API only.
fn pexels_file(url: &str) -> Result<(), String> {
    let host = url_host(url);
    if url.starts_with("https://") && (host == "pexels.com" || host.ends_with(".pexels.com")) {
        Ok(())
    } else {
        Err(format!("Not a Pexels file: {url}"))
    }
}

fn url_host(url: &str) -> &str {
    url.split("://")
        .nth(1)
        .and_then(|rest| rest.split(['/', '?', ':']).next())
        .unwrap_or_default()
}

/// The words of a Pexels page address: `.../video/aerial-view-of-a-harbour-123/` → "aerial
/// view of a harbour".
fn words_of_page(url: &str) -> String {
    let slug = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    slug.split('-')
        .filter(|w| !w.chars().all(|c| c.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn number(v: &Value) -> u32 {
    v.as_u64().unwrap_or(0).min(u32::MAX as u64) as u32
}

fn video(v: &Value) -> Option<StockItem> {
    let files: Vec<&Value> = v["video_files"]
        .as_array()?
        .iter()
        .filter(|f| f["file_type"].as_str() == Some("video/mp4") && f["link"].is_string())
        .collect();
    let side = |f: &Value| number(&f["width"]).max(number(&f["height"]));
    // The largest up to 1920, else the smallest.
    let file = files
        .iter()
        .filter(|f| side(f) <= VIDEO_MAX_SIDE)
        .max_by_key(|f| side(f))
        .or_else(|| files.iter().min_by_key(|f| side(f)))?;
    let page = v["url"].as_str().unwrap_or_default().to_string();
    Some(StockItem {
        id: format!("pexels-video-{}", v["id"].as_u64()?),
        kind: StockKind::Video,
        width: number(&file["width"]),
        height: number(&file["height"]),
        duration_s: v["duration"].as_u64().map(|d| d as u32),
        author: v["user"]["name"].as_str().unwrap_or("unknown").to_string(),
        about: words_of_page(&page),
        page_url: page,
        preview_url: v["image"].as_str().unwrap_or_default().to_string(),
        file_url: file["link"].as_str()?.to_string(),
    })
}

fn photo(v: &Value) -> Option<StockItem> {
    let page = v["url"].as_str().unwrap_or_default().to_string();
    let alt = v["alt"].as_str().unwrap_or_default().trim().to_string();
    Some(StockItem {
        id: format!("pexels-photo-{}", v["id"].as_u64()?),
        kind: StockKind::Photo,
        width: number(&v["width"]),
        height: number(&v["height"]),
        duration_s: None,
        author: v["photographer"].as_str().unwrap_or("unknown").to_string(),
        about: if alt.is_empty() {
            words_of_page(&page)
        } else {
            alt
        },
        page_url: page,
        preview_url: v["src"]["medium"].as_str().unwrap_or_default().to_string(),
        file_url: v["src"]["large2x"]
            .as_str()
            .or(v["src"]["original"].as_str())?
            .to_string(),
    })
}

fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Pexels' results for `query`: videos or photos, `landscape`, `portrait` or `square` when
/// given.
pub fn search(
    fetch: &dyn Fetch,
    query: &str,
    kind: StockKind,
    orientation: Option<&str>,
    limit: usize,
) -> Result<Vec<StockItem>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Give `stock`: words for what it shows".into());
    }
    let mut url = match kind {
        StockKind::Video => format!("{API}/videos/search"),
        StockKind::Photo => format!("{API}/v1/search"),
    };
    url.push_str(&format!(
        "?query={}&per_page={}",
        encode(query),
        limit.clamp(1, LIMIT_MAX)
    ));
    if let Some(o) = orientation {
        if !["landscape", "portrait", "square"].contains(&o) {
            return Err("`orientation` is landscape, portrait or square".into());
        }
        url.push_str(&format!("&orientation={o}"));
    }
    let answer = fetch.json(&url)?;
    let (list, parse): (&str, fn(&Value) -> Option<StockItem>) = match kind {
        StockKind::Video => ("videos", video),
        StockKind::Photo => ("photos", photo),
    };
    Ok(answer[list]
        .as_array()
        .map(|a| a.iter().filter_map(parse).collect())
        .unwrap_or_default())
}

/// One item by its id (`pexels-video-<n>` or `pexels-photo-<n>`).
pub fn item(fetch: &dyn Fetch, id: &str) -> Result<StockItem, String> {
    let bad = || format!("{id} is not a stock id such as pexels-video-123 or pexels-photo-45");
    let (kind, n) = if let Some(n) = id.strip_prefix("pexels-video-") {
        (StockKind::Video, n)
    } else if let Some(n) = id.strip_prefix("pexels-photo-") {
        (StockKind::Photo, n)
    } else {
        return Err(bad());
    };
    let n: u64 = n.parse().map_err(|_| bad())?;
    let answer = match kind {
        StockKind::Video => fetch.json(&format!("{API}/videos/videos/{n}"))?,
        StockKind::Photo => fetch.json(&format!("{API}/v1/photos/{n}"))?,
    };
    match kind {
        StockKind::Video => video(&answer),
        StockKind::Photo => photo(&answer),
    }
    .ok_or_else(|| format!("Pexels has no file for {id}"))
}

/// Saves `item` in `root`'s [`DIR`] (once: a file already there is kept) and says where.
pub fn download(fetch: &dyn Fetch, root: &Path, item: &StockItem) -> Result<PathBuf, String> {
    let dir = crate::project::reader::safe_path(root, DIR)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let to = dir.join(item.file_name());
    if std::fs::metadata(&to).is_ok_and(|m| m.len() > 0) {
        return Ok(to);
    }
    let part = dir.join(format!("{}.part", item.file_name()));
    let saved = fetch.save(&item.file_url, &part).and_then(|bytes| {
        if bytes == 0 {
            return Err(format!("Pexels sent an empty file for {}", item.id));
        }
        std::fs::rename(&part, &to).map_err(|e| e.to_string())
    });
    if let Err(error) = saved {
        let _ = std::fs::remove_file(&part);
        return Err(error);
    }
    Ok(to)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use parking_lot::Mutex;
    use serde_json::json;

    /// Answers like Pexels from fixtures, and remembers what was asked.
    #[derive(Default)]
    pub struct Fake {
        pub asked: Mutex<Vec<String>>,
        /// Bytes each saved file gets.
        pub file: Vec<u8>,
    }

    pub fn video_json(id: u64) -> Value {
        json!({
            "id": id, "width": 3840, "height": 2160, "duration": 12,
            "url": format!("https://www.pexels.com/video/aerial-view-of-a-harbour-{id}/"),
            "image": format!("https://images.pexels.com/videos/{id}/pictures/preview-0.jpg"),
            "user": { "name": "Ana Lee", "url": "https://www.pexels.com/@ana" },
            "video_files": [
                { "id": 1, "quality": "uhd", "file_type": "video/mp4", "width": 3840,
                  "height": 2160, "link": format!("https://videos.pexels.com/{id}-uhd.mp4") },
                { "id": 2, "quality": "hd", "file_type": "video/mp4", "width": 1920,
                  "height": 1080, "link": format!("https://videos.pexels.com/{id}-hd.mp4") },
                { "id": 3, "quality": "sd", "file_type": "video/mp4", "width": 640,
                  "height": 360, "link": format!("https://videos.pexels.com/{id}-sd.mp4") }
            ]
        })
    }

    impl Fetch for Fake {
        fn json(&self, url: &str) -> Result<Value, String> {
            self.asked.lock().push(url.to_string());
            if url.contains("/videos/search") {
                return Ok(json!({ "videos": [video_json(11), video_json(12)] }));
            }
            if let Some(id) = url.split("/videos/videos/").nth(1) {
                return Ok(video_json(id.parse().unwrap()));
            }
            if url.contains("/v1/search") {
                return Ok(json!({ "photos": [{
                    "id": 7, "width": 4000, "height": 3000, "alt": "A red kettle on a stove",
                    "url": "https://www.pexels.com/photo/red-kettle-7/",
                    "photographer": "Sam K",
                    "src": { "medium": "https://images.pexels.com/photos/7/m.jpeg",
                             "large2x": "https://images.pexels.com/photos/7/l.jpeg" }
                }] }));
            }
            Err(format!("unexpected {url}"))
        }

        fn preview(&self, url: &str) -> Result<Vec<u8>, String> {
            self.asked.lock().push(url.to_string());
            let picture = image::RgbImage::from_pixel(64, 36, image::Rgb([200, 60, 40]));
            let mut jpeg = Vec::new();
            image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
                .encode_image(&picture)
                .map_err(|e| e.to_string())?;
            Ok(jpeg)
        }

        fn save(&self, url: &str, to: &Path) -> Result<u64, String> {
            self.asked.lock().push(url.to_string());
            std::fs::write(to, &self.file).map_err(|e| e.to_string())?;
            Ok(self.file.len() as u64)
        }
    }

    #[test]
    fn results_are_read_with_the_hd_file_and_their_licence() {
        let fake = Fake::default();
        let found = search(
            &fake,
            "harbour at dusk",
            StockKind::Video,
            Some("landscape"),
            3,
        )
        .unwrap();
        assert_eq!(found.len(), 2);
        let first = &found[0];
        assert_eq!(first.id, "pexels-video-11");
        assert_eq!((first.width, first.height), (1920, 1080));
        assert_eq!(first.file_url, "https://videos.pexels.com/11-hd.mp4");
        assert_eq!(first.about, "aerial view of a harbour");
        assert_eq!(
            first.line(),
            "pexels-video-11: 12 s, 1920×1080, by Ana Lee: aerial view of a harbour"
        );
        assert!(first.licence().kind.needs_no_credit());
        assert!(first
            .licence()
            .source
            .unwrap()
            .contains("by Ana Lee: https://www.pexels.com/video/"));
        assert_eq!(
            fake.asked.lock()[0],
            "https://api.pexels.com/videos/search?query=harbour+at+dusk&per_page=3&orientation=landscape"
        );
        let photos = search(&fake, "kettle", StockKind::Photo, None, 8).unwrap();
        assert_eq!(photos[0].about, "A red kettle on a stove");
        assert_eq!(photos[0].file_name(), "pexels-photo-7.jpg");
        assert!(search(&fake, "x", StockKind::Video, Some("wide"), 3).is_err());
        assert!(item(&fake, "shutterstock-1").is_err());
        assert!(pexels_file("https://videos.pexels.com/1.mp4").is_ok());
        assert!(pexels_file("https://evilpexels.com/1.mp4").is_err());
        assert!(pexels_file("http://videos.pexels.com/1.mp4").is_err());
        assert_eq!(
            item(&fake, "pexels-video-12").unwrap().id,
            "pexels-video-12"
        );
    }

    #[test]
    fn a_download_is_saved_once_in_the_projects_stock_folder() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Fake {
            file: b"mp4 bytes".to_vec(),
            ..Fake::default()
        };
        let it = item(&fake, "pexels-video-11").unwrap();
        let path = download(&fake, dir.path(), &it).unwrap();
        assert!(path.ends_with("media/stock/pexels-video-11.mp4"));
        assert_eq!(std::fs::read(&path).unwrap(), b"mp4 bytes");
        download(&fake, dir.path(), &it).unwrap();
        let saves = fake
            .asked
            .lock()
            .iter()
            .filter(|u| u.ends_with(".mp4"))
            .count();
        assert_eq!(saves, 1, "kept, not fetched again");
        let empty = Fake::default();
        let other = item(&empty, "pexels-video-12").unwrap();
        assert!(download(&empty, dir.path(), &other).is_err());
        assert!(!dir
            .path()
            .join("media/stock/pexels-video-12.mp4.part")
            .exists());
    }
}
