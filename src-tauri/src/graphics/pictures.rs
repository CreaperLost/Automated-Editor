//! Pictures brought into a graphic from outside the image library (FCL-6): a screenshot or a
//! picture of the user's, or one an image model draws in the video's look. Each is kept in the
//! graphic's `images/` folder beside library pictures, and `pictures.json` records where it
//! came from, so the check before export knows what to credit and what to disclose.
use crate::image_library::{self as library, Look, GRAPHIC_DIR};
use crate::publish::generate::Draw;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The variable a picture part shows its picture from.
pub const VARIABLE: &str = "picture";
/// The graphic's record of the pictures brought in.
pub const RECORD: &str = "pictures.json";
/// A picture's longest side, at most: larger ones are scaled down when they come in.
const LONGEST: u32 = 2560;
/// A drawn picture's longest side.
const DRAWN_LONGEST: u32 = 1920;
/// Library pictures of the look sent with a brief, so the drawing follows the look.
const REFERENCES: usize = 2;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// A file: a screenshot or a picture the user gave.
    File,
    /// Drawn by an image model.
    Drawn,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Picture {
    /// Its path in the graphic's folder (`images/own-chat-window.png`).
    pub file: String,
    pub origin: Origin,
    /// A file's credit, as the description gives it; empty for the user's own.
    #[serde(default)]
    pub source: String,
    /// The file's own name, when it came from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// What a drawn picture shows, as asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brief: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

impl Picture {
    /// In a line: what it is and what it asks for.
    pub fn line(&self) -> String {
        match self.origin {
            Origin::Drawn => format!(
                "{}: drawn by {} ({})",
                self.file,
                self.model.as_deref().unwrap_or("an image model"),
                self.brief.as_deref().unwrap_or("")
            ),
            Origin::File if self.source.is_empty() => format!("{}: the user's own", self.file),
            Origin::File => format!("{}: credit {}", self.file, self.source),
        }
    }
}

/// The pictures recorded in a graphic's folder, whose files are still there.
pub fn recorded(graphic_dir: &Path) -> Vec<Picture> {
    let all: Vec<Picture> = crate::files::load_json(&graphic_dir.join(RECORD)).unwrap_or_default();
    all.into_iter()
        .filter(|p| graphic_dir.join(&p.file).is_file())
        .collect()
}

/// The record with `picture` in it (one taking the same file is replaced), as the file holds it.
pub fn record_with(graphic_dir: &Path, picture: Picture) -> Result<Vec<u8>, String> {
    let mut all = recorded(graphic_dir);
    all.retain(|p| p.file != picture.file);
    all.push(picture);
    serde_json::to_vec_pretty(&all).map_err(|e| e.to_string())
}

/// A short file name from words: lower case, letters, digits and dashes.
fn slug(words: &str) -> String {
    let mut out = String::new();
    for c in words.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= 32 {
            break;
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "picture".into()
    } else {
        out
    }
}

/// A free path for a new picture: `images/<prefix>-<slug>.<ext>`, numbered when taken.
pub fn free_path(graphic_dir: &Path, prefix: &str, words: &str, ext: &str) -> String {
    let stem = format!("{prefix}-{}", slug(words));
    (1..)
        .map(|n| {
            if n == 1 {
                format!("{GRAPHIC_DIR}/{stem}.{ext}")
            } else {
                format!("{GRAPHIC_DIR}/{stem}-{n}.{ext}")
            }
        })
        .find(|p| !graphic_dir.join(p).exists())
        .unwrap_or_default()
}

/// A picture file as a graphic keeps it: PNG or JPEG, scaled down when its longest side is
/// over [`LONGEST`]. Returns the bytes, their extension and the size.
pub fn take(bytes: &[u8]) -> Result<(Vec<u8>, &'static str, (u32, u32)), String> {
    let ext = crate::publish::image_kind(bytes)
        .ok_or("Only PNG and JPEG pictures can come in (a screenshot is PNG)")?;
    let img =
        image::load_from_memory(bytes).map_err(|e| format!("The picture can't be read: {e}"))?;
    let (w, h) = (img.width(), img.height());
    if w.max(h) <= LONGEST {
        return Ok((bytes.to_vec(), ext, (w, h)));
    }
    let small = img.resize(LONGEST, LONGEST, image::imageops::FilterType::Lanczos3);
    let size = (small.width(), small.height());
    Ok((encode(&small, ext)?, ext, size))
}

fn encode(img: &image::DynamicImage, ext: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    if ext == "png" {
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
    } else {
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
        img.to_rgb8()
            .write_with_encoder(encoder)
            .map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// What the image model is told: an illustration of `brief` in `look`, standing for an idea.
pub fn prompt(brief: &str, look: &Look, aspect: &str, with_references: bool) -> String {
    let mut out = format!(
        "An illustration for a video, {aspect}.\n\nWhat it shows: {}\n\nThe look: {} Ground \
         {}, accent {} (on the one thing that matters).",
        brief.trim(),
        look.description.trim(),
        look.ground,
        look.accent
    );
    if with_references {
        out.push_str(
            "\n\nThe pictures attached are drawn in this look: match their drawing, line, \
             colour and how much they show, not their subject.",
        );
    }
    out.push_str(
        "\n\nIt stands for the idea: not a photograph, not a real screen, product, person or \
         event. No words, letters or numbers, no logos, no watermark.",
    );
    out
}

/// A drawn picture, ready to keep.
pub struct Drawn {
    pub bytes: Vec<u8>,
    pub size: (u32, u32),
    pub cost_usd: Option<f64>,
}

/// Draws `brief` in `look` at `aspect` ("16:9", "9:16"), with the look's nearest library
/// pictures as references, and keeps it as a JPEG of at most [`DRAWN_LONGEST`].
pub fn draw(
    brief: &str,
    look: &Look,
    aspect: &str,
    model: &str,
    draw: &dyn Draw,
) -> Result<Drawn, String> {
    if brief.split_whitespace().count() < 3 {
        return Err("Describe the picture in `brief`: what it shows, in a few words".into());
    }
    let mut refs: Vec<&library::Image> = library::search(brief, Some(&[look.id.as_str()]));
    if refs.is_empty() {
        refs = library::catalog()
            .images
            .iter()
            .filter(|i| i.look == look.id)
            .collect();
    }
    let references: Vec<(String, Vec<u8>)> = refs
        .iter()
        .filter(|i| i.look == look.id)
        .take(REFERENCES)
        .map(|i| {
            let mime = if i.file.ends_with(".svg") {
                "image/svg+xml"
            } else {
                "image/webp"
            };
            (mime.to_string(), i.bytes().to_vec())
        })
        .collect();
    let made = draw.draw(
        model,
        &prompt(brief, look, aspect, !references.is_empty()),
        &references,
        aspect,
    )?;
    let img = image::load_from_memory(&made.bytes)
        .map_err(|e| format!("The image model's picture could not be read: {e}"))?;
    let img = if img.width().max(img.height()) > DRAWN_LONGEST {
        img.resize(
            DRAWN_LONGEST,
            DRAWN_LONGEST,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        img
    };
    Ok(Drawn {
        bytes: encode(&img, "jpg")?,
        size: (img.width(), img.height()),
        cost_usd: made.cost_usd,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;

    pub(crate) fn png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, _| image::Rgb([(x % 256) as u8, 40, 30]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    struct Fake(Mutex<Vec<(String, usize, String)>>);

    impl Draw for Fake {
        fn draw(
            &self,
            _model: &str,
            prompt: &str,
            references: &[(String, Vec<u8>)],
            aspect: &str,
        ) -> Result<crate::publish::generate::Drawn, String> {
            self.0
                .lock()
                .push((prompt.to_string(), references.len(), aspect.to_string()));
            Ok(crate::publish::generate::Drawn {
                bytes: png(2048, 3640),
                cost_usd: Some(0.07),
            })
        }
    }

    #[test]
    fn a_picture_comes_in_scaled_and_named_freely() {
        let (bytes, ext, size) = take(&png(400, 300)).unwrap();
        assert_eq!((ext, size), ("png", (400, 300)));
        assert_eq!(bytes, png(400, 300), "a small one is kept as it is");
        let (_, _, size) = take(&png(5120, 2880)).unwrap();
        assert_eq!(size, (2560, 1440));
        assert!(take(b"GIF89a....").is_err());

        let dir = tempfile::tempdir().unwrap();
        let first = free_path(dir.path(), "own", "Chat window, after!", "png");
        assert_eq!(first, "images/own-chat-window-after.png");
        std::fs::create_dir_all(dir.path().join("images")).unwrap();
        std::fs::write(dir.path().join(&first), b"x").unwrap();
        assert_eq!(
            free_path(dir.path(), "own", "Chat window, after!", "png"),
            "images/own-chat-window-after-2.png"
        );
        assert_eq!(
            free_path(dir.path(), "drawn", "…", "jpg"),
            "images/drawn-picture.jpg"
        );
    }

    #[test]
    fn the_record_keeps_one_entry_per_file_and_forgets_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let own = Picture {
            file: "images/own-a.png".into(),
            origin: Origin::File,
            source: String::new(),
            from: Some("a.png".into()),
            brief: None,
            model: None,
            cost_usd: None,
        };
        std::fs::create_dir_all(dir.path().join("images")).unwrap();
        std::fs::write(dir.path().join(&own.file), b"x").unwrap();
        let bytes = record_with(dir.path(), own.clone()).unwrap();
        std::fs::write(dir.path().join(RECORD), bytes).unwrap();
        let credited = Picture {
            source: "Screenshot: example.com".into(),
            ..own.clone()
        };
        std::fs::write(
            dir.path().join(RECORD),
            record_with(dir.path(), credited.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(recorded(dir.path()), vec![credited.clone()]);
        assert_eq!(
            credited.line(),
            "images/own-a.png: credit Screenshot: example.com"
        );
        std::fs::remove_file(dir.path().join(&own.file)).unwrap();
        assert!(recorded(dir.path()).is_empty());
    }

    #[test]
    fn a_drawing_follows_the_look_and_stands_for_an_idea() {
        let look = library::look("broadsheet").unwrap();
        let fake = Fake(Mutex::new(Vec::new()));
        assert!(
            draw("robot", look, "9:16", "m", &fake).is_err(),
            "too short a brief"
        );
        let made = draw("a robot reading a long contract", look, "9:16", "m", &fake).unwrap();
        assert_eq!(made.size, (1080, 1920));
        assert!(made.bytes.starts_with(&[0xFF, 0xD8, 0xFF]));
        assert_eq!(made.cost_usd, Some(0.07));
        let asked = fake.0.lock();
        let (prompt, refs, aspect) = &asked[0];
        assert_eq!((*refs, aspect.as_str()), (REFERENCES, "9:16"));
        assert!(prompt.contains(&look.ground) && prompt.contains(&look.accent));
        assert!(prompt.contains("not a photograph"));
        assert!(prompt.contains("robot reading a long contract"));
    }
}
