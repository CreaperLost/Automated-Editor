//! A minimal OpenAI-compatible chat-completions client that asks for a JSON object back.
use super::AiProvider;
use serde_json::{json, Value};
use std::time::Duration;

const MAX_ATTEMPTS: u32 = 3;

/// Something that answers a system + user prompt with a JSON object. The detection code
/// takes this trait so tests can use canned answers.
pub trait JsonModel {
    fn complete_json(&mut self, system: &str, user: &str) -> Result<Value, String>;
    /// `provider/model`, recorded with the results.
    fn describe(&self) -> String;
}

pub struct ChatClient {
    client: reqwest::blocking::Client,
    provider: AiProvider,
    base_url: String,
    api_key: String,
    model: String,
}

impl ChatClient {
    pub fn new(provider: AiProvider, api_key: String, model: String) -> Result<Self, String> {
        Self::with_base_url(provider, provider.base_url().into(), api_key, model)
    }

    /// Talks to `base_url` instead of the provider's API; used by tests.
    pub fn with_base_url(
        provider: AiProvider,
        base_url: String,
        api_key: String,
        model: String,
    ) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err(format!("Add your {} API key first", provider.label()));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            provider,
            base_url,
            api_key: api_key.trim().to_string(),
            model,
        })
    }

    fn request(&self, body: &Value) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let mut request = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(body);
        if self.provider == AiProvider::OpenRouter {
            // OpenRouter's optional app attribution headers.
            request = request
                .header(
                    "HTTP-Referer",
                    "https://github.com/CreaperLost/Automated-Editor",
                )
                .header("X-Title", "AeroEdits");
        }
        request.send()
    }
}

/// The provider's error message from a JSON error body, if there is one.
fn error_message(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .map(|m| m.chars().take(300).collect())
}

/// The JSON object in a model reply, tolerating a Markdown code fence around it.
pub fn parse_reply(content: &str) -> Result<Value, String> {
    let trimmed = content.trim();
    let unfenced = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|rest| rest.strip_suffix("```"))
        .unwrap_or(trimmed)
        .trim();
    serde_json::from_str(unfenced).map_err(|_| "The AI reply was not valid JSON".to_string())
}

impl JsonModel for ChatClient {
    fn complete_json(&mut self, system: &str, user: &str) -> Result<Value, String> {
        let body = json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
            "response_format": { "type": "json_object" },
        });
        let mut last_error = String::new();
        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                std::thread::sleep(Duration::from_secs(2u64.pow(attempt)));
            }
            let response = match self.request(&body) {
                Ok(response) => response,
                Err(e) => {
                    last_error = format!("Could not reach {}: {e}", self.provider.label());
                    continue;
                }
            };
            let status = response.status();
            let text = response.text().unwrap_or_default();
            if status.is_success() {
                let value: Value = serde_json::from_str(&text)
                    .map_err(|_| format!("{} sent an unreadable reply", self.provider.label()))?;
                let content = value
                    .pointer("/choices/0/message/content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("{} sent an empty reply", self.provider.label()))?;
                return parse_reply(content);
            }
            let detail = error_message(&text).unwrap_or_else(|| status.to_string());
            match status.as_u16() {
                401 | 403 => {
                    return Err(format!(
                        "{} rejected the API key: {detail}",
                        self.provider.label()
                    ))
                }
                400 | 404 | 422 => {
                    return Err(format!(
                        "{} refused the request (model {}): {detail}",
                        self.provider.label(),
                        self.model
                    ))
                }
                _ => last_error = format!("{}: {detail}", self.provider.label()),
            }
        }
        Err(last_error)
    }

    fn describe(&self) -> String {
        format!("{}/{}", self.provider.label(), self.model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies_parse_with_or_without_a_code_fence() {
        assert_eq!(parse_reply("{\"a\":1}").unwrap()["a"], 1);
        assert_eq!(parse_reply("```json\n{\"a\":2}\n```").unwrap()["a"], 2);
        assert!(parse_reply("sure, here you go").is_err());
        assert_eq!(
            error_message("{\"error\":{\"message\":\"bad model\"}}").as_deref(),
            Some("bad model")
        );
    }

    /// Serves each canned (status, body) once and returns the raw requests it got.
    fn fake_server(replies: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, body) in replies {
                let (mut stream, _) = listener.accept().unwrap();
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = stream.read(&mut buf).unwrap();
                    raw.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&raw);
                    if let Some(head_end) = text.find("\r\n\r\n") {
                        let length = text[..head_end]
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if raw.len() >= head_end + 4 + length {
                            break;
                        }
                    }
                }
                requests.push(String::from_utf8_lossy(&raw).into_owned());
                let response = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
            requests
        });
        (url, handle)
    }

    #[test]
    fn sends_an_openai_style_request_and_reads_the_json_reply() {
        let reply = serde_json::json!({
            "choices": [{ "message": { "content": "{\"cuts\":[]}" } }]
        })
        .to_string();
        let (url, server) = fake_server(vec![(200, reply)]);
        let mut client = ChatClient::with_base_url(
            AiProvider::OpenRouter,
            url,
            "sk-test".into(),
            "openai/gpt-4.1-mini".into(),
        )
        .unwrap();
        let value = client.complete_json("system text", "user text").unwrap();
        assert_eq!(value, serde_json::json!({ "cuts": [] }));
        let requests = server.join().unwrap();
        let request = &requests[0];
        assert!(request.starts_with("POST /chat/completions"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer sk-test"));
        assert!(request.to_ascii_lowercase().contains("x-title: aeroedits"));
        assert!(request.contains("\"response_format\":{\"type\":\"json_object\"}"));
        assert!(request.contains("\"model\":\"openai/gpt-4.1-mini\""));
        assert!(request.contains("user text"));
        assert_eq!(client.describe(), "OpenRouter/openai/gpt-4.1-mini");
    }

    #[test]
    fn a_rejected_key_fails_at_once_with_the_provider_message() {
        let body = "{\"error\":{\"message\":\"Incorrect API key provided\"}}".to_string();
        let (url, server) = fake_server(vec![(401, body)]);
        let mut client =
            ChatClient::with_base_url(AiProvider::OpenAi, url, "sk-bad".into(), "m".into())
                .unwrap();
        let error = client.complete_json("s", "u").unwrap_err();
        assert!(error.contains("OpenAI rejected the API key"), "{error}");
        assert!(error.contains("Incorrect API key provided"));
        assert_eq!(server.join().unwrap().len(), 1);
    }
}
