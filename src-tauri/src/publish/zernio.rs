//! A small Zernio client (PUB-2): uploads finished videos to the YouTube channel connected in
//! the user's Zernio dashboard, so AeroEdits needs no Google sign-in of its own. Ported from
//! the explainer's `zernio.py`.
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

pub const BASE_URL: &str = "https://zernio.com/api/v1";
/// Where the user makes a key and connects channels.
pub const DASHBOARD_URL: &str = "https://zernio.com/dashboard";

pub const KEY: crate::secrets::KeySpec = crate::secrets::KeySpec {
    env: "ZERNIO_API_KEY",
    name: "zernio-api-key",
};

/// Something that speaks HTTP to Zernio; tests answer from fixtures.
pub trait Http: Sync {
    /// `method` on `url` with a JSON body; the status and the reply's JSON.
    fn json(
        &self,
        method: &str,
        url: &str,
        body: Option<&Value>,
        idempotency_key: Option<&str>,
    ) -> Result<(u16, Value), String>;
    /// PUTs a file to a presigned storage URL (which carries its own credentials).
    fn put_file(&self, url: &str, path: &Path, content_type: &str) -> Result<u16, String>;
}

pub struct Web {
    api: reqwest::blocking::Client,
    storage: reqwest::blocking::Client,
    key: String,
}

impl Web {
    pub fn new(key: String) -> Result<Self, String> {
        // Posting now uploads to YouTube inside the request, so replies can take minutes.
        let api = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(600))
            .connect_timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| e.to_string())?;
        // A large video takes as long as the connection needs.
        let storage = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(4 * 3600))
            .connect_timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { api, storage, key })
    }
}

impl Http for Web {
    fn json(
        &self,
        method: &str,
        url: &str,
        body: Option<&Value>,
        idempotency_key: Option<&str>,
    ) -> Result<(u16, Value), String> {
        let mut request = match method {
            "GET" => self.api.get(url),
            "POST" => self.api.post(url),
            "DELETE" => self.api.delete(url),
            other => return Err(format!("unsupported method {other}")),
        }
        .bearer_auth(&self.key);
        if let Some(body) = body {
            request = request.json(body);
        }
        if let Some(key) = idempotency_key {
            request = request.header("Idempotency-Key", key);
        }
        let response = request.send().map_err(|e| {
            if e.is_timeout() {
                "Zernio did not answer in time".to_string()
            } else {
                "Could not reach Zernio".to_string()
            }
        })?;
        let status = response.status().as_u16();
        let text = response.text().unwrap_or_default();
        let value = serde_json::from_str(&text).unwrap_or(Value::String(text));
        Ok((status, value))
    }

    fn put_file(&self, url: &str, path: &Path, content_type: &str) -> Result<u16, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        let response = self
            .storage
            .put(url)
            .header("Content-Type", content_type)
            .header("Content-Length", size)
            .body(reqwest::blocking::Body::sized(file, size))
            .send()
            // Never the error text: it could carry the presigned URL.
            .map_err(|_| "The upload to Zernio's storage was cut off".to_string())?;
        Ok(response.status().as_u16())
    }
}

fn error_text(label: &str, status: u16, reply: &Value) -> String {
    match status {
        401 | 403 => "Zernio refused the key: make a new one in the Zernio dashboard and paste it in Accounts".into(),
        402 => "Zernio says the plan does not cover this (HTTP 402)".into(),
        429 => "Zernio is rate limiting: try again in a minute".into(),
        _ => {
            let detail = reply
                .get("error")
                .or_else(|| reply.get("message"))
                .map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string()))
                .unwrap_or_else(|| reply.to_string());
            format!(
                "{label}: HTTP {status}: {}",
                detail.chars().take(300).collect::<String>()
            )
        }
    }
}

/// A YouTube channel connected in Zernio.
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub active: bool,
}

pub struct Client<'a> {
    pub http: &'a dyn Http,
}

impl Client<'_> {
    fn call(
        &self,
        label: &str,
        method: &str,
        path: &str,
        body: Option<&Value>,
        idempotency_key: Option<&str>,
    ) -> Result<Value, String> {
        let url = format!("{BASE_URL}{path}");
        let mut last = String::new();
        // Retries only what may pass on a second try, and never a post (its key covers that).
        let attempts = if method == "POST" && path == "/posts" {
            1
        } else {
            3
        };
        for attempt in 0..attempts {
            match self.http.json(method, &url, body, idempotency_key) {
                Ok((status, reply)) if status < 400 => return Ok(reply),
                Ok((409, reply)) if reply.to_string().contains("idempotency_conflict") => {
                    return Err("Zernio is still working on the first request for this upload: check its status in a minute".into())
                }
                Ok((status, reply)) => {
                    last = error_text(label, status, &reply);
                    if ![429, 500, 502, 503, 504].contains(&status) {
                        return Err(last);
                    }
                }
                Err(e) => last = format!("{label}: {e}"),
            }
            if attempt + 1 < attempts {
                std::thread::sleep(Duration::from_millis(1500 << attempt));
            }
        }
        Err(last)
    }

    /// The YouTube channels connected in the Zernio dashboard.
    pub fn channels(&self) -> Result<Vec<Channel>, String> {
        let reply = self.call("Listing channels", "GET", "/accounts", None, None)?;
        Ok(reply["accounts"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter(|a| a["platform"].as_str() == Some("youtube"))
            .filter_map(|a| {
                Some(Channel {
                    id: a["_id"].as_str()?.to_string(),
                    name: a["displayName"]
                        .as_str()
                        .or(a["username"].as_str())
                        .unwrap_or("YouTube channel")
                        .to_string(),
                    active: a["isActive"].as_bool().unwrap_or(true),
                })
            })
            .collect())
    }

    /// Uploads a local file to Zernio's storage; returns its public URL.
    pub fn upload_media(&self, path: &Path) -> Result<String, String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let content_type = match ext.as_str() {
            "mp4" => "video/mp4",
            "mov" => "video/quicktime",
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            _ => return Err(format!("Zernio takes MP4, MOV, JPG or PNG, not .{ext}")),
        };
        let size = std::fs::metadata(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .len();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("video.mp4");
        let presigned = self.call(
            "Preparing the upload",
            "POST",
            "/media/presign",
            Some(&json!({ "filename": name, "contentType": content_type, "size": size })),
            None,
        )?;
        let (Some(upload), Some(public)) = (
            presigned["uploadUrl"].as_str(),
            presigned["publicUrl"].as_str(),
        ) else {
            return Err("Zernio did not give an upload address".into());
        };
        let mut status = 0;
        for attempt in 0..3 {
            status = self.http.put_file(upload, path, content_type)?;
            if status < 400 || ![429, 500, 502, 503, 504].contains(&status) {
                break;
            }
            std::thread::sleep(Duration::from_millis(2000 << attempt));
        }
        if status >= 400 {
            return Err(format!("Zernio's storage refused {name} (HTTP {status})"));
        }
        Ok(public.to_string())
    }

    /// Creates the post. A 2xx can still be a failed post, so its status is checked.
    pub fn create_post(&self, body: &Value, key: &str) -> Result<Value, String> {
        let reply = self.call("Posting", "POST", "/posts", Some(body), Some(key))?;
        let post = reply
            .get("post")
            .cloned()
            .ok_or("Zernio answered without a post")?;
        let status = post["status"].as_str().unwrap_or_default();
        if body.get("isDraft").is_none() && (status == "failed" || status == "partial") {
            let reasons = post["platforms"]
                .as_array()
                .map(|p| {
                    p.iter()
                        .map(|p| {
                            p["errorMessage"]
                                .as_str()
                                .or(p["status"].as_str())
                                .unwrap_or_default()
                                .to_string()
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .unwrap_or_default();
            return Err(format!("Zernio could not post it ({status}): {reasons}"));
        }
        Ok(post)
    }

    pub fn post(&self, id: &str) -> Result<Value, String> {
        let reply = self.call(
            "Reading the post",
            "GET",
            &format!("/posts/{id}"),
            None,
            None,
        )?;
        Ok(reply.get("post").cloned().unwrap_or(reply))
    }

    /// Recent YouTube posts of a channel, to find a post a lost reply made.
    pub fn recent_posts(&self, channel: &str) -> Result<Vec<Value>, String> {
        let reply = self.call(
            "Listing posts",
            "GET",
            &format!("/posts?platform=youtube&accountId={channel}&sortBy=created-desc&limit=100"),
            None,
            None,
        )?;
        Ok(reply["posts"].as_array().cloned().unwrap_or_default())
    }
}

/// The YouTube part of a post.
pub fn youtube_target(post: &Value) -> Value {
    post["platforms"]
        .as_array()
        .and_then(|p| p.iter().find(|t| t["platform"].as_str() == Some("youtube")))
        .cloned()
        .unwrap_or(Value::Null)
}

/// The YouTube video id in a post's YouTube part, from its id or its link.
pub fn video_id(target: &Value) -> Option<String> {
    let valid = |s: &str| {
        s.len() == 11
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    };
    if let Some(id) = target["platformPostId"].as_str().filter(|s| valid(s)) {
        return Some(id.into());
    }
    let link = url::Url::parse(target["platformPostUrl"].as_str()?).ok()?;
    let host = link.host_str()?.trim_start_matches("www.");
    let parts: Vec<&str> = link.path().trim_matches('/').split('/').collect();
    let candidate = match host {
        "youtu.be" => parts.first().map(|s| s.to_string()),
        "youtube.com" | "m.youtube.com" => {
            if parts.len() == 2 && parts[0] == "shorts" {
                Some(parts[1].to_string())
            } else {
                link.query_pairs()
                    .find(|(k, _)| k == "v")
                    .map(|(_, v)| v.into_owned())
            }
        }
        _ => None,
    };
    candidate.filter(|c| valid(c))
}
