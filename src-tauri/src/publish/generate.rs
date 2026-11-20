//! Generated thumbnails (PUB-1): an image model draws one from a brief in the project's style
//! (its colours, its thumbnail rules and the references it marks as thumbnails), on the user's
//! OpenRouter key. The picture is cut to 16:9 at 1280×720 and kept as the package's thumbnail
//! like any other; a picture that would be hard to read at feed size is said so.
use crate::style::Style;
use base64::Engine;
use image::GenericImageView;
use std::path::Path;
use std::time::Duration;

/// The image model asked by default: Google's Gemini 3.1 Flash Image, about 7 cents a picture
/// (1,120 image tokens at OpenRouter's price, Nov 2026).
pub const DEFAULT_MODEL: &str = "google/gemini-3.1-flash-image";
const CHAT_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
/// The size the model is asked to compose for; YouTube's own is 1280×720.
const ASKED: (u32, u32) = (1536, 864);
const OUT: (u32, u32) = (1280, 720);
/// The smallest picture worth enlarging to 1280×720.
const SMALLEST: (u32, u32) = (640, 360);
/// Thumbnail references sent with the brief, at most.
const MAX_REFERENCES: usize = 3;
/// Below this contrast (the spread of brightness at 160×90, 0 to 1) a feed-size thumbnail
/// reads as one flat colour.
const MIN_SMALL_CONTRAST: f64 = 0.12;

/// One picture an image model drew, and what it cost when the service said.
pub struct Drawn {
    pub bytes: Vec<u8>,
    pub cost_usd: Option<f64>,
}

/// Something that draws a picture from words and example pictures; tests answer from fixtures.
pub trait Draw: Sync {
    /// `references`: (MIME type, bytes) of pictures to follow the look of.
    fn draw(
        &self,
        model: &str,
        prompt: &str,
        references: &[(String, Vec<u8>)],
    ) -> Result<Drawn, String>;
}

/// OpenRouter's chat endpoint with image output, on the key kept for transcription and review.
pub struct OpenRouter {
    client: reqwest::blocking::Client,
    key: String,
}

impl OpenRouter {
    pub fn new(key: String) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(180))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client, key })
    }

    /// The OpenRouter key in the keychain (or `OPENROUTER_API_KEY`), if one is set.
    pub fn from_keychain() -> Option<Self> {
        let config = crate::transcript::settings::config_dir();
        let (key, _) = crate::transcript::settings::openrouter_key(&config)?;
        Self::new(key).ok()
    }
}

/// The request OpenRouter gets: the brief, then each reference as an image.
pub fn request_body(
    model: &str,
    prompt: &str,
    references: &[(String, Vec<u8>)],
) -> serde_json::Value {
    let mut content = vec![serde_json::json!({ "type": "text", "text": prompt })];
    for (mime, bytes) in references {
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        content.push(serde_json::json!({
            "type": "image_url",
            "image_url": { "url": format!("data:{mime};base64,{data}") }
        }));
    }
    serde_json::json!({
        "model": model,
        "modalities": ["image", "text"],
        "image_config": { "aspect_ratio": "16:9" },
        "messages": [{ "role": "user", "content": content }],
    })
}

/// The picture in an OpenRouter reply (a `data:` URL in the message's images).
pub fn picture_in(reply: &serde_json::Value) -> Result<Drawn, String> {
    let message = &reply["choices"][0]["message"];
    let url = message["images"][0]["image_url"]["url"]
        .as_str()
        .ok_or_else(|| {
            let said: String = message["content"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(200)
                .collect();
            if said.trim().is_empty() {
                "The image model sent no picture".to_string()
            } else {
                format!("The image model sent no picture; it said: {said}")
            }
        })?;
    let data = url
        .split_once(";base64,")
        .map(|(_, d)| d)
        .ok_or("The image model's picture is not inline data")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data.trim())
        .map_err(|e| format!("The image model's picture could not be read: {e}"))?;
    Ok(Drawn {
        bytes,
        cost_usd: reply["usage"]["cost"].as_f64(),
    })
}

impl Draw for OpenRouter {
    fn draw(
        &self,
        model: &str,
        prompt: &str,
        references: &[(String, Vec<u8>)],
    ) -> Result<Drawn, String> {
        let response = self
            .client
            .post(CHAT_URL)
            .bearer_auth(&self.key)
            .header("X-Title", "AeroEdits")
            .json(&request_body(model, prompt, references))
            .send()
            .map_err(|e| format!("Could not reach OpenRouter: {e}"))?;
        let status = response.status().as_u16();
        let body = response.text().unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(match status {
                401 | 403 => {
                    "OpenRouter refused the key: check it in transcription settings".into()
                }
                402 => "The OpenRouter credits are used up: top them up at openrouter.ai".into(),
                429 => "OpenRouter is busy: try again in a minute".into(),
                _ => format!(
                    "OpenRouter answered HTTP {status}: {}",
                    body.chars().take(300).collect::<String>()
                ),
            });
        }
        let reply: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("OpenRouter's reply: {e}"))?;
        picture_in(&reply)
    }
}

/// What the image model is told: the brief, the style's colours and thumbnail rules, and what
/// every thumbnail needs.
pub fn prompt(brief: &str, style: &Style, with_references: bool) -> String {
    let c = &style.brand.colors;
    let mut out = format!(
        "A YouTube thumbnail, 16:9, composed for {}×{}.\n\nWhat it shows: {}\n\nColours: \
         background {}, text {}, accent {} (the one thing that matters).",
        ASKED.0,
        ASKED.1,
        brief.trim(),
        c.background,
        c.text,
        c.accent
    );
    if !style.thumbnail.is_empty() {
        out.push_str("\n\nThe channel's thumbnail rules:");
        for rule in &style.thumbnail {
            out.push_str(&format!("\n- {rule}"));
        }
    }
    if with_references {
        out.push_str(
            "\n\nThe pictures attached are earlier thumbnails of the channel: match their \
             look (layout, type, colour), not their subject or words.",
        );
    }
    out.push_str(
        "\n\nIt must read at 160 pixels wide in a phone feed: few words, very large, strong \
         contrast against the background, one clear subject. Spell every word exactly as \
         given; no other text, no logos you were not asked for, no watermark.",
    );
    out
}

/// The style's thumbnail references (files in its folder), as (MIME type, bytes).
pub fn references(style: &Style, folder: Option<&Path>) -> Vec<(String, Vec<u8>)> {
    let Some(folder) = folder else {
        return Vec::new();
    };
    style
        .references
        .iter()
        .filter(|r| r.thumbnail)
        .filter_map(|r| {
            let bytes = std::fs::read(folder.join(&r.file)).ok()?;
            let mime = match super::image_kind(&bytes)? {
                "png" => "image/png",
                _ => "image/jpeg",
            };
            Some((mime.to_string(), bytes))
        })
        .take(MAX_REFERENCES)
        .collect()
}

/// A drawn picture as a thumbnail: the middle 16:9 of it at 1280×720, as a JPEG.
pub fn fit(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("The image model's picture could not be read: {e}"))?;
    let (w, h) = img.dimensions();
    if w < SMALLEST.0 || h < SMALLEST.1 {
        return Err(format!(
            "The image model drew {w}×{h}; a thumbnail needs at least {}×{}",
            SMALLEST.0, SMALLEST.1
        ));
    }
    // The widest 16:9 box in the middle.
    let (cw, ch) = if w * 9 >= h * 16 {
        (h * 16 / 9, h)
    } else {
        (w, w * 9 / 16)
    };
    let cropped = img.crop_imm((w - cw) / 2, (h - ch) / 2, cw, ch);
    let sized = cropped.resize_exact(OUT.0, OUT.1, image::imageops::FilterType::Lanczos3);
    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 92);
    sized
        .to_rgb8()
        .write_with_encoder(encoder)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// How much the brightness varies across the picture at feed size (160×90): the standard
/// deviation of luminance, 0 (one flat colour) to 0.5.
pub fn small_contrast(bytes: &[u8]) -> Result<f64, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let small = img
        .resize_exact(160, 90, image::imageops::FilterType::Triangle)
        .to_luma8();
    let values: Vec<f64> = small.pixels().map(|p| p.0[0] as f64 / 255.0).collect();
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    Ok(var.sqrt())
}

/// A thumbnail drawn for a brief, ready to keep.
pub struct Generated {
    pub bytes: Vec<u8>,
    pub cost_usd: Option<f64>,
    /// What to look at before using it.
    pub notes: Vec<String>,
}

/// Draws a thumbnail for `brief` in `style` and fits it.
pub fn generate(
    brief: &str,
    style: &Style,
    style_folder: Option<&Path>,
    model: &str,
    draw: &dyn Draw,
) -> Result<Generated, String> {
    if brief.trim().chars().count() < 10 {
        return Err("Describe the thumbnail: what it shows and its few words".into());
    }
    let refs = references(style, style_folder);
    let drawn = draw.draw(model, &prompt(brief, style, !refs.is_empty()), &refs)?;
    let bytes = fit(&drawn.bytes)?;
    let mut notes = Vec::new();
    let contrast = small_contrast(&bytes)?;
    if contrast < MIN_SMALL_CONTRAST {
        notes.push(format!(
            "At 160 px it is nearly one flat tone (contrast {contrast:.2}): it may not read in a feed"
        ));
    }
    Ok(Generated {
        bytes,
        cost_usd: drawn.cost_usd,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;

    /// A picture `w`×`h`: a dark ground with a light block (or flat, when `flat`), as PNG.
    fn picture(w: u32, h: u32, flat: bool) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            if !flat && x < w / 2 && y > h / 4 && y < h * 3 / 4 {
                image::Rgb([240, 233, 218])
            } else {
                image::Rgb([28, 26, 22])
            }
        });
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    /// Answers with a fixed picture, as OpenRouter's reply would carry it, and keeps the asks.
    struct Fake {
        reply: serde_json::Value,
        asked: Mutex<Vec<serde_json::Value>>,
    }

    impl Fake {
        fn with(bytes: &[u8]) -> Self {
            let data = base64::engine::general_purpose::STANDARD.encode(bytes);
            Self {
                reply: serde_json::json!({
                    "choices": [{ "message": { "content": "", "images": [
                        { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{data}") } }
                    ] } }],
                    "usage": { "cost": 0.0665 }
                }),
                asked: Mutex::new(Vec::new()),
            }
        }
    }

    impl Draw for Fake {
        fn draw(
            &self,
            model: &str,
            prompt: &str,
            references: &[(String, Vec<u8>)],
        ) -> Result<Drawn, String> {
            self.asked
                .lock()
                .push(request_body(model, prompt, references));
            picture_in(&self.reply)
        }
    }

    #[test]
    fn a_drawn_picture_becomes_a_1280_by_720_thumbnail_under_2_mb() {
        let style = crate::style::builtin()
            .into_iter()
            .find(|s| s.name == crate::style::BROADSHEET)
            .unwrap();
        // What the model really sent in a live check: 1376×768.
        let fake = Fake::with(&picture(1376, 768, false));
        let made = generate(
            "A small humanoid robot mid-stride; the words WALKS FROM VIDEO",
            &style,
            None,
            DEFAULT_MODEL,
            &fake,
        )
        .unwrap();
        assert_eq!(super::super::image_size(&made.bytes), Some((1280, 720)));
        assert!((made.bytes.len() as u64) < super::super::MAX_THUMBNAIL_BYTES);
        assert_eq!(made.cost_usd, Some(0.0665));
        assert!(made.notes.is_empty(), "{:?}", made.notes);
        let asked = fake.asked.lock()[0].clone();
        assert_eq!(asked["model"], DEFAULT_MODEL);
        assert_eq!(asked["image_config"]["aspect_ratio"], "16:9");
        let text = asked["messages"][0]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("composed for 1536×864"));
        assert!(text.contains("WALKS FROM VIDEO"));
        assert!(text.contains("#C8321F"), "the style's accent");
        assert!(text.contains("one word in vermilion"), "the style's rules");
        assert!(!text.contains("pictures attached"));
    }

    #[test]
    fn the_style_s_thumbnail_references_go_with_the_brief() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("thumb-1.png"), picture(64, 36, false)).unwrap();
        std::fs::write(dir.path().join("card.png"), picture(64, 36, false)).unwrap();
        let mut style = crate::style::Style::default();
        style.references = vec![
            crate::style::Reference {
                file: "card.png".into(),
                ..Default::default()
            },
            crate::style::Reference {
                file: "thumb-1.png".into(),
                thumbnail: true,
                ..Default::default()
            },
        ];
        let fake = Fake::with(&picture(1920, 1080, false));
        generate(
            "A robot and two words",
            &style,
            Some(dir.path()),
            "m",
            &fake,
        )
        .unwrap();
        let asked = fake.asked.lock()[0].clone();
        let parts = asked["messages"][0]["content"].as_array().unwrap().clone();
        assert_eq!(parts.len(), 2, "the brief and the one thumbnail reference");
        assert!(parts[1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert!(parts[0]["text"]
            .as_str()
            .unwrap()
            .contains("pictures attached"));
    }

    #[test]
    fn odd_shapes_are_cut_to_16_9_and_bad_pictures_are_refused_or_flagged() {
        // A square picture keeps its middle.
        let square = fit(&picture(1024, 1024, false)).unwrap();
        assert_eq!(super::super::image_size(&square), Some((1280, 720)));
        // Too small to enlarge.
        assert!(fit(&picture(600, 338, false))
            .unwrap_err()
            .contains("at least 640×360"));
        // A flat picture is kept but said to be hard to read at feed size.
        let flat = generate(
            "A flat dark nothing",
            &Style::default(),
            None,
            "m",
            &Fake::with(&picture(1376, 768, true)),
        )
        .unwrap();
        assert!(flat.notes[0].contains("160 px"), "{:?}", flat.notes);
        // A reply without a picture says what the model said instead.
        let refused = picture_in(
            &serde_json::json!({ "choices": [{ "message": { "content": "I can't draw that." } }] }),
        );
        assert!(refused.err().unwrap().contains("I can't draw that."));
        assert!(generate(
            "short",
            &Style::default(),
            None,
            "m",
            &Fake::with(&picture(1376, 768, false))
        )
        .is_err());
    }
}
