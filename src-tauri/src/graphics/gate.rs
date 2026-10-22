//! The check a graphic passes to go to the library as a template, run by the app with no model.
//!
//! A general template takes any text, fits a 16:9 video and a 9:16 short, follows the length of
//! its clip, takes the brand kit's colours and needs no network. The gate looks at the graphic's
//! files, then at staged copies of it rendered through the pinned HyperFrames:
//! - `hyperframes check` (lint, runtime errors, layout overflow, contrast) at 16:9 and 9:16,
//!   with the graphic's own values and again with long test values for every string variable,
//!   and in a short's split part (1080x864, its portrait rules on, as the app renders it there);
//! - stills of a 3 s copy and an 8 s copy, eased in and out as over the full-screen camera: the
//!   first and last see-through, the card shown in between, so its timing follows the length;
//! - stills of a 3 s copy the app cuts in and out (`data-entrance="cut"`, as everywhere else):
//!   the card shown whole on the first and last frames, and whether it fills the frame matches
//!   its `data-overlay` mark;
//! - one copy per string variable with another value, whose stills must differ from the plain
//!   ones.
//!
//! The answer is a pass, warning or fail per check with the reason, and a contact sheet of both
//! shapes. Results are kept by the source's hash in the graphic's render cache, so an unchanged
//! graphic is not checked twice.
use super::{render, runtime, VariableDecl};
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Bumped when the checks change, so older results are not reused.
const GATE_VERSION: u32 = 2;
/// The two lengths the gate plays a graphic at.
pub const SHORT_US: u64 = 3_000_000;
pub const LONG_US: u64 = 8_000_000;
const WIDE: (u32, u32) = (1920, 1080);
const TALL: (u32, u32) = (1080, 1920);
/// The screen's part of a 9:16 short split with the camera at 55 %.
const SPLIT: (u32, u32) = (1080, 864);
/// Chrome instances at once.
const PARALLEL: usize = 3;
const RUN_TIMEOUT: Duration = Duration::from_secs(180);
/// A frame is see-through when at most this share of it is covered (alpha 96 or more).
const SEE_THROUGH_MAX: f64 = 0.005;
/// The card shows when at least this share of the frame is nearly opaque (alpha 200 or more).
const SHOWN_MIN: f64 = 0.02;
/// A graphic fills the frame when its card covers at least this share of it.
const FILLS_MIN: f64 = 0.9;
/// A graphic that covers less than this share is an overlay (a lower third, a callout).
const OVERLAY_MAX: f64 = 0.6;
/// Cut in, the first and last frames show at least this share of what the card covers.
const CUT_SHOWN_SHARE: f64 = 0.5;
/// A variable changes the picture when this many pixels differ.
const CHANGED_PIXELS: usize = 100;
/// What a string variable is set to, to see that it changes the picture.
const OTHER_VALUE: &str = "Wxyz 0123";
/// Words long test values are made of when a variable's default is empty.
const FILLER: &str = "Configure the continuous integration pipeline before deploying the workspace";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    /// Worth fixing, but not a reason to refuse it as a template.
    Warn,
    Fail,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub id: String,
    pub label: String,
    pub status: Status,
    /// Why, for a warning or a failure; what was checked, for a pass.
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// No check failed.
    pub passed: bool,
    pub checks: Vec<Check>,
    /// Of the source checked; a changed graphic is checked again.
    pub source_hash: String,
    pub checked_at: String,
    pub seconds: f64,
    /// The contact sheet (PNG): 16:9 with its values and with long ones, then 9:16 the same,
    /// then a short's split part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
}

impl Report {
    /// One line per check: ✓ passed, ! warning, ✗ failed (with why).
    pub fn lines(&self) -> Vec<String> {
        self.checks
            .iter()
            .map(|c| match c.status {
                Status::Pass => format!("✓ {}", c.label),
                Status::Warn => format!("! {}: {}", c.label, c.reason),
                Status::Fail => format!("✗ {}: {}", c.label, c.reason),
            })
            .collect()
    }

    /// The labels of the checks that failed.
    pub fn failed(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter(|c| c.status == Status::Fail)
            .map(|c| c.label.clone())
            .collect()
    }
}

fn check(id: &str, label: &str, status: Status, reason: impl Into<String>) -> Check {
    Check {
        id: id.into(),
        label: label.into(),
        status,
        reason: reason.into(),
    }
}

/// What names a gate result: every source file, the gate and HyperFrames versions.
pub fn source_hash(dir: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    render::list_files(dir, dir, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    hasher.update(format!(
        "gate {GATE_VERSION} {}",
        runtime::HYPERFRAMES_VERSION
    ));
    for (relative, path) in files {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    let digest = hasher.finalize();
    Ok(digest.iter().take(8).map(|b| format!("{b:02x}")).collect())
}

fn results_dir(cache: &Path) -> PathBuf {
    cache.join("gate")
}

/// The gate's last result for the graphic in `dir` as its files are now, if there is one.
pub fn cached(dir: &Path, cache: &Path) -> Option<Report> {
    let hash = source_hash(dir).ok()?;
    load(cache, &hash)
}

fn load(cache: &Path, hash: &str) -> Option<Report> {
    let results = results_dir(cache);
    let text = std::fs::read_to_string(results.join(format!("{hash}.json"))).ok()?;
    let mut report: Report = serde_json::from_str(&text).ok()?;
    let sheet = results.join(format!("{hash}.png"));
    report.sheet = sheet
        .is_file()
        .then(|| sheet.to_string_lossy().into_owned());
    Some(report)
}

// ---------------------------------------------------------------------------------------
// What the files say.

/// The checks made on the files alone: no network, the brand kit's colour variables, a
/// portrait block, no fixed text.
pub fn static_checks(
    html: &str,
    declared: &[VariableDecl],
    files: &[(String, String)],
) -> Vec<Check> {
    let mut out = Vec::new();
    let network = network_refs(files);
    out.push(if network.is_empty() {
        check(
            "network",
            "No network",
            Status::Pass,
            "no http(s) references",
        )
    } else {
        check(
            "network",
            "No network",
            Status::Fail,
            format!(
                "it reaches the network ({}); put files beside index.html instead",
                network
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    });
    let missing: Vec<&str> = ["background", "text", "accent"]
        .into_iter()
        .filter(|id| !declared.iter().any(|d| d.id == *id && d.kind == "color"))
        .collect();
    out.push(if missing.is_empty() {
        check(
            "brandColours",
            "Brand kit colours",
            Status::Pass,
            "background, text and accent are color variables",
        )
    } else {
        check(
            "brandColours",
            "Brand kit colours",
            Status::Fail,
            format!(
                "declare {} as color variables and use them for every colour, so a brand kit applies",
                missing.join(", ")
            ),
        )
    });
    let lower = html.to_ascii_lowercase();
    out.push(
        if lower.contains("orientation: portrait") || lower.contains("orientation:portrait") {
            check(
                "portrait",
                "Portrait layout",
                Status::Pass,
                "has an @media (orientation: portrait) block",
            )
        } else {
            check(
                "portrait",
                "Portrait layout",
                Status::Warn,
                "no @media (orientation: portrait) block; the 9:16 renders decide whether it fits",
            )
        },
    );
    out.push(look_check(html));
    let fixed = fixed_text(html);
    out.push(if fixed.is_empty() {
        check(
            "fixedText",
            "No fixed text",
            Status::Pass,
            "every text comes from a variable or the script",
        )
    } else {
        check(
            "fixedText",
            "No fixed text",
            Status::Warn,
            format!(
                "text not from a variable: {}",
                fixed
                    .iter()
                    .take(4)
                    .map(|t| format!("\"{t}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    });
    out
}

/// `http://` and `https://` references in the graphic's text files (XML namespaces aside), and
/// protocol-relative ones.
fn network_refs(files: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in files {
        let lower = text.to_ascii_lowercase();
        for pattern in [
            "http://",
            "https://",
            "src=\"//",
            "src='//",
            "href=\"//",
            "href='//",
            "url(//",
            "url(\"//",
            "url('//",
        ] {
            let mut from = 0;
            while let Some(at) = lower[from..].find(pattern) {
                let at = from + at;
                from = at + pattern.len();
                let reference: String = text[at..]
                    .chars()
                    .take_while(|c| !c.is_whitespace() && !matches!(c, ')' | '<' | '>'))
                    .take(80)
                    .collect();
                // XML namespaces, and GSAP from a CDN (the render uses the local copy).
                if reference.contains("www.w3.org/") || super::gsap::is_cdn_link(&reference) {
                    continue;
                }
                found.push(format!("{path}: {reference}"));
            }
        }
    }
    found
}

/// Whether the graphic takes the video's look: a colour of its own fails (the brand kit must
/// reach every colour); a font, corner radius or animation timing of its own is a warning (the
/// look sets them for every graphic at once; `graphic_templatize` turns them into tokens).
fn look_check(html: &str) -> Check {
    let (colours, others) = hard_coded(html);
    let list = |found: &[String]| {
        found
            .iter()
            .take(4)
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !colours.is_empty() {
        check(
            "look",
            "The video's look",
            Status::Fail,
            format!(
                "colours of its own ({}): use var(--background), var(--text), var(--accent) \
                 or the roles var(--look-muted), var(--look-line), var(--look-tint), \
                 var(--look-wash), var(--look-shadow)",
                list(&colours)
            ),
        )
    } else if !others.is_empty() {
        check(
            "look",
            "The video's look",
            Status::Warn,
            format!(
                "fixed where the look should decide ({}): use var(--look-font), \
                 var(--look-radius), var(--look-duration) and the other --look-* tokens",
                list(&others)
            ),
        )
    } else {
        check(
            "look",
            "The video's look",
            Status::Pass,
            "colours from the brand kit, type, corners and motion from the look",
        )
    }
}

/// Colours, and fonts, corner radii and CSS timings, written into `html`'s styles and scripts
/// rather than taken from variables (anything inside `var(...)` counts as taken, so a
/// fallback is fine).
pub fn hard_coded(html: &str) -> (Vec<String>, Vec<String>) {
    // Comments first: a guide in one may show a `<script>` tag.
    let mut html = html.to_string();
    while let Some(at) = html.find("<!--") {
        let end = html[at..].find("-->").map_or(html.len(), |e| at + e + 3);
        html.replace_range(at..end, "");
    }
    let html = html.as_str();
    let code = strip_vars(&format!(
        "{}\n{}\n{}",
        blocks(html, "style"),
        blocks(html, "script"),
        inline_styles(html)
    ));
    let mut colours: Vec<String> = Vec::new();
    let mut others: Vec<String> = Vec::new();
    let push = |list: &mut Vec<String>, found: String| {
        if !list.contains(&found) {
            list.push(found);
        }
    };
    for (at, _) in code.match_indices('#') {
        let hex: String = code[at + 1..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .collect();
        // A colour in a value, not an id selector (`#card-back`, `#fade {`).
        let next = code[at + 1 + hex.len()..].chars().next();
        if matches!(hex.len(), 3 | 4 | 6 | 8)
            && !next.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_')
            && after_colon(&code, at)
        {
            push(&mut colours, format!("#{hex}"));
        }
    }
    let lower = code.to_ascii_lowercase();
    for function in ["rgb(", "rgba(", "hsl(", "hsla("] {
        for (at, _) in lower.match_indices(function) {
            let end = lower[at..].find(')').map_or(lower.len(), |e| at + e + 1);
            push(&mut colours, code[at..end].to_string());
        }
    }
    for property in [
        "font-family:",
        "border-radius:",
        "transition:",
        "animation:",
    ] {
        for (at, _) in lower.match_indices(property) {
            let end = lower[at..]
                .find([';', '}', '"', '\n'])
                .map_or(lower.len(), |e| at + e);
            let value = lower[at + property.len()..end].trim();
            // Taken from the look, or a shape no look sets: a circle or a pill, square corners.
            let fine = value.is_empty()
                || value.contains("var(")
                || value == "inherit"
                || value == "none"
                || (property == "border-radius:"
                    && (value == "0" || value == "50%" || value.starts_with("99")));
            if !fine {
                push(&mut others, code[at..end].trim().to_string());
            }
        }
    }
    (colours, others)
}

/// Whether `at` is in a declaration's value (after a `:` since the last `;`, `{` or `}`), as
/// opposed to a selector.
fn after_colon(code: &str, at: usize) -> bool {
    let before = &code[..at];
    let boundary = before.rfind([';', '{', '}']).map_or(0, |b| b + 1);
    before[boundary..].contains(':')
}

/// `code` without its `var(...)` expressions.
fn strip_vars(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut rest = code;
    while let Some(at) = rest.find("var(") {
        out.push_str(&rest[..at]);
        let mut depth = 0;
        let mut end = rest.len();
        for (i, c) in rest[at..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = at + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push_str("var()");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Every `style="..."` attribute's value in `html`, joined.
fn inline_styles(html: &str) -> String {
    html.match_indices("style=\"")
        .filter_map(|(at, m)| {
            let start = at + m.len();
            html[start..].find('"').map(|e| &html[start..start + e])
        })
        .collect::<Vec<_>>()
        .join(";\n")
}

const VOID: [&str; 10] = [
    "br", "img", "hr", "input", "meta", "link", "source", "wbr", "area", "col",
];

/// Visible text in the body that no variable fills: neither inside an element with a
/// `data-var-*` attribute nor inside one the script looks up by id (and so fills itself).
fn fixed_text(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let scripts = blocks(html, "script");
    let filled_by_script = |tag: &str| {
        id_of(tag).is_some_and(|id| {
            scripts.contains(&format!("\"{id}\""))
                || scripts.contains(&format!("'{id}'"))
                || scripts.contains(&format!("#{id}"))
        })
    };
    let mut out = Vec::new();
    // Open elements: name, and whether their text comes from elsewhere.
    let mut open: Vec<(String, bool)> = Vec::new();
    let mut i = lower.find("<body").unwrap_or(0);
    while i < html.len() {
        if html[i..].starts_with("<!--") {
            i = html[i..].find("-->").map_or(html.len(), |e| i + e + 3);
            continue;
        }
        if html[i..].starts_with('<') {
            let Some(end) = html[i..].find('>').map(|e| i + e) else {
                break;
            };
            let tag = &html[i + 1..end];
            i = end + 1;
            if let Some(name) = tag.strip_prefix('/') {
                let name = name.trim().to_ascii_lowercase();
                if let Some(at) = open.iter().rposition(|(n, _)| *n == name) {
                    open.truncate(at);
                }
                continue;
            }
            let name: String = tag
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect::<String>()
                .to_ascii_lowercase();
            if name.is_empty() {
                continue;
            }
            if name == "script" || name == "style" {
                let close = format!("</{name}");
                i = lower[i..].find(&close).map_or(html.len(), |e| i + e);
                continue;
            }
            if !VOID.contains(&name.as_str()) && !tag.trim_end().ends_with('/') {
                let filled = tag.contains("data-var-") || filled_by_script(tag);
                open.push((name, filled));
            }
            continue;
        }
        let next = html[i..].find('<').map_or(html.len(), |e| i + e);
        let text = html[i..next]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        i = next;
        if text.chars().any(char::is_alphanumeric) && !open.iter().any(|(_, filled)| *filled) {
            out.push(text.chars().take(40).collect());
        }
    }
    out
}

/// Everything inside `<name ...>...</name>` blocks, joined.
fn blocks(html: &str, name: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let (open, close) = (format!("<{name}"), format!("</{name}"));
    let mut out = String::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find(&open) {
        let start = match html[from + at..].find('>') {
            Some(e) => from + at + e + 1,
            None => break,
        };
        let end = lower[start..]
            .find(&close)
            .map_or(html.len(), |e| start + e);
        out.push_str(&html[start..end]);
        out.push('\n');
        from = end;
    }
    out
}

fn id_of(tag: &str) -> Option<String> {
    let at = tag.find(" id=").or_else(|| tag.find("\tid="))? + 4;
    let quote = tag[at..]
        .chars()
        .next()
        .filter(|c| *c == '"' || *c == '\'')?;
    let end = tag[at + 1..].find(quote)?;
    Some(tag[at + 1..at + 1 + end].to_string())
}

/// A long but plausible value in place of `default`: each of its lines (the same number of
/// them) about twice as long, made of its own words.
pub fn long_value(default: &str) -> String {
    default
        .split('\n')
        .map(|line| {
            let length = line.chars().count();
            let want = (length * 2).max(length + 16).clamp(24, 140);
            let words: Vec<&str> = line.split_whitespace().collect();
            let words = if words.is_empty() {
                FILLER.split(' ').collect()
            } else {
                words
            };
            let mut out = String::new();
            for word in words.iter().cycle() {
                if out.chars().count() >= want {
                    break;
                }
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(word);
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Another value for a string variable, with as many lines as `default`.
fn other_value(default: &str) -> String {
    let lines = default.split('\n').count().max(1);
    (0..lines)
        .map(|n| format!("{OTHER_VALUE}{n}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------------------
// What the renders show.

/// The share of `frame`'s pixels with alpha at least `alpha`.
pub fn coverage(frame: &RgbaImage, alpha: u8) -> f64 {
    let total = (frame.width() as u64 * frame.height() as u64).max(1);
    let covered = frame.pixels().filter(|p| p.0[3] >= alpha).count() as u64;
    covered as f64 / total as f64
}

/// How many pixels differ by more than a little between `a` and `b` (all of them when the sizes
/// differ).
pub fn differing_pixels(a: &RgbaImage, b: &RgbaImage) -> usize {
    if a.dimensions() != b.dimensions() {
        return (a.width() * a.height()) as usize;
    }
    a.pixels()
        .zip(b.pixels())
        .filter(|(p, q)| p.0.iter().zip(q.0.iter()).any(|(x, y)| x.abs_diff(*y) > 32))
        .count()
}

/// The findings of one section of `hyperframes check --json` with `severity`, as lines.
fn findings(report: &Value, section: &str, severity: &str) -> Vec<String> {
    report[section]["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["severity"].as_str() == Some(severity))
        .map(|f| {
            let mut line = f["message"]
                .as_str()
                .or(f["code"].as_str())
                .unwrap_or("problem")
                .to_string();
            if let Some(selector) = f["selector"].as_str() {
                line = format!("{selector}: {line}");
            }
            line
        })
        .collect()
}

/// Layout findings that something leaves its box or the frame. HyperFrames skips the browser
/// when its lint finds errors, and then nothing was measured: that is a finding too.
fn overflows(report: &Value) -> Vec<String> {
    let samples = report["layout"]["samples"].as_array().map_or(0, Vec::len);
    if report["browserSkipped"].as_bool() == Some(true) || samples == 0 {
        return vec![
            "not measured: HyperFrames stopped before rendering (fix the HyperFrames check first)"
                .into(),
        ];
    }
    report["layout"]["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| {
            f["severity"].as_str() == Some("error")
                || f["code"].as_str().is_some_and(|c| c.contains("overflow"))
        })
        .map(|f| {
            let mut line = format!(
                "{} {}",
                f["selector"].as_str().unwrap_or("something"),
                f["message"].as_str().unwrap_or("overflows")
            );
            if let Some(text) = f["text"].as_str() {
                let text: String = text.chars().take(40).collect();
                line.push_str(&format!(" (\"{text}\")"));
            }
            if let Some(hint) = f["fixHint"].as_str() {
                let hint: String = hint.chars().take(160).collect();
                line.push_str(&format!(" {hint}"));
            }
            line
        })
        .collect()
}

/// One staged copy of the graphic, rendered for the gate.
struct Run {
    /// Short: it names the folder.
    key: String,
    size: (u32, u32),
    length_us: u64,
    values: Map<String, Value>,
    check: bool,
    contrast: bool,
    /// How the app brings it on and off in this run.
    entrance: super::Entrance,
    /// As a part of a short's split (`data-mode="split"`, laid out as a phone frame).
    split: bool,
    /// Stills to take, in seconds.
    at: Vec<f64>,
}

#[derive(Default)]
struct RunResult {
    check: Option<Value>,
    frames: Vec<RgbaImage>,
    error: Option<String>,
}

fn times(length_us: u64, fractions: &[f64], last: bool) -> Vec<f64> {
    let length = length_us as f64 / 1e6;
    let mut at: Vec<f64> = fractions.iter().map(|f| length * f).collect();
    if last {
        at.push(length - 1.0 / render::RENDER_FPS as f64);
    }
    at
}

/// Only one gate runs at a time; it already runs several Chromes.
static RUNNING: Mutex<()> = Mutex::new(());

/// Runs the gate on the graphic in folder `dir`, keeping the result in `cache` (its render
/// cache). An unchanged graphic answers from there at once.
pub fn run(
    dir: &Path,
    cache: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f64, &str),
) -> Result<Report, String> {
    let _one = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    let hash = source_hash(dir)?;
    if let Some(report) = load(cache, &hash) {
        return Ok(report);
    }
    let paths = runtime::paths()?;
    let started = Instant::now();
    let entry = dir.join(super::ENTRY);
    let html = std::fs::read_to_string(&entry).map_err(|e| format!("{}: {e}", entry.display()))?;
    let declared = super::parse_declarations(&html)?;
    let source = super::read_source(dir)?;
    let mut files = Vec::new();
    render::list_files(dir, dir, &mut files)?;
    let texts: Vec<(String, String)> = files
        .iter()
        .filter(|(relative, _)| {
            let lower = relative.to_ascii_lowercase();
            [
                ".html", ".htm", ".css", ".js", ".mjs", ".svg", ".json", ".txt",
            ]
            .iter()
            .any(|ext| lower.ends_with(ext))
        })
        .filter_map(|(relative, path)| {
            Some((relative.clone(), std::fs::read_to_string(path).ok()?))
        })
        .collect();
    let mut checks = static_checks(&html, &declared, &texts);

    // The values it shows: its own over the declared defaults.
    let mut values = Map::new();
    for decl in &declared {
        let value = source
            .variables
            .get(&decl.id)
            .cloned()
            .unwrap_or_else(|| decl.default.clone());
        values.insert(decl.id.clone(), value);
    }
    let strings: Vec<&VariableDecl> = declared.iter().filter(|d| d.kind == "string").collect();
    let mut long = values.clone();
    for decl in &strings {
        let now = values[&decl.id].as_str().unwrap_or_default().to_string();
        long.insert(decl.id.clone(), Value::String(long_value(&now)));
    }
    let mut runs = vec![
        Run {
            key: "w".into(),
            size: WIDE,
            length_us: SHORT_US,
            values: values.clone(),
            check: true,
            contrast: true,
            entrance: super::Entrance::Ease,
            split: false,
            at:times(SHORT_US, &[0.0, 0.5, 0.75], true),
        },
        Run {
            key: "t".into(),
            size: TALL,
            length_us: LONG_US,
            values: values.clone(),
            check: true,
            contrast: true,
            entrance: super::Entrance::Ease,
            split: false,
            at:times(LONG_US, &[0.0, 0.5, 0.75], true),
        },
        Run {
            key: "wl".into(),
            size: WIDE,
            length_us: SHORT_US,
            values: long.clone(),
            check: true,
            contrast: false,
            entrance: super::Entrance::Ease,
            split: false,
            at:times(SHORT_US, &[0.5], false),
        },
        Run {
            key: "tl".into(),
            size: TALL,
            length_us: LONG_US,
            values: long,
            check: true,
            contrast: false,
            entrance: super::Entrance::Ease,
            split: false,
            at:times(LONG_US, &[0.5], false),
        },
    ];
    for (n, decl) in strings.iter().enumerate() {
        let mut changed = values.clone();
        let now = values[&decl.id].as_str().unwrap_or_default();
        changed.insert(decl.id.clone(), Value::String(other_value(now)));
        runs.push(Run {
            key: format!("v{n}"),
            size: WIDE,
            length_us: SHORT_US,
            values: changed,
            check: false,
            contrast: false,
            entrance: super::Entrance::Ease,
            split: false,
            at:times(SHORT_US, &[0.5, 0.75], false),
        });
    }
    // As the app places it everywhere but over the full-screen camera: cut in and out.
    runs.push(Run {
        key: "c".into(),
        size: WIDE,
        length_us: SHORT_US,
        values: values.clone(),
        check: false,
        contrast: false,
        entrance: super::Entrance::Cut,
        split: false,
        at: times(SHORT_US, &[0.0, 0.5], true),
    });
    // In the screen's part of a short's split, as the app lays it out there (the camera at
    // 55 %: the widest part that still takes the portrait rules is close to this).
    runs.push(Run {
        key: "s".into(),
        size: SPLIT,
        length_us: SHORT_US,
        values: values.clone(),
        check: true,
        contrast: false,
        entrance: super::Entrance::Cut,
        split: true,
        at: times(SHORT_US, &[0.5], false),
    });

    let work = runtime::runtime_dir()
        .join("work")
        .join(format!("gate-{}", &hash[..8]));
    let _ = std::fs::remove_dir_all(&work);
    let results = execute(&paths, dir, &html, &work, &runs, cancel, progress);
    let _ = std::fs::remove_dir_all(&work);
    if cancel.load(Ordering::Relaxed) {
        return Err("Cancelled".into());
    }
    let result = |key: &str| {
        runs.iter()
            .position(|r| r.key == key)
            .map(|at| &results[at])
            .expect("every run has a result")
    };
    let (wide, tall, wide_long, tall_long) = (result("w"), result("t"), result("wl"), result("tl"));
    let split = result("s");

    // HyperFrames' own check.
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for run in [wide, tall] {
        match (&run.check, &run.error) {
            (Some(report), _) => {
                for section in ["lint", "runtime", "contrast"] {
                    if section == "lint" && !std::ptr::eq(run, wide) {
                        continue;
                    }
                    errors.extend(findings(report, section, "error"));
                    warnings.extend(findings(report, section, "warning"));
                }
            }
            (None, Some(error)) => errors.push(error.clone()),
            (None, None) => errors.push("no answer from hyperframes check".into()),
        }
    }
    errors.dedup();
    warnings.dedup();
    checks.insert(
        0,
        if !errors.is_empty() {
            check(
                "hyperframes",
                "HyperFrames check",
                Status::Fail,
                errors
                    .iter()
                    .take(4)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        } else if !warnings.is_empty() {
            check(
                "hyperframes",
                "HyperFrames check",
                Status::Warn,
                warnings
                    .iter()
                    .take(4)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        } else {
            check(
                "hyperframes",
                "HyperFrames check",
                Status::Pass,
                "lint, runtime and contrast clean",
            )
        },
    );

    // Fits each shape, with long text too.
    for (id, label, plain, long) in [
        ("fitsWide", "Fits 16:9, long text too", wide, wide_long),
        ("fitsTall", "Fits 9:16, long text too", tall, tall_long),
    ] {
        let mut problems = Vec::new();
        for (run, which) in [(plain, "its text"), (long, "long text")] {
            match (&run.check, &run.error) {
                (Some(report), _) => problems.extend(
                    overflows(report)
                        .into_iter()
                        .map(|line| format!("with {which}: {line}")),
                ),
                (None, error) => problems.push(format!(
                    "with {which}: {}",
                    error.clone().unwrap_or_else(|| "not checked".into())
                )),
            }
        }
        checks.push(if problems.is_empty() {
            check(
                id,
                label,
                Status::Pass,
                "nothing leaves its box or the frame",
            )
        } else {
            check(
                id,
                label,
                Status::Fail,
                problems
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        });
    }
    let problems = match (&split.check, &split.error) {
        (Some(report), _) => overflows(report),
        (None, error) => vec![error.clone().unwrap_or_else(|| "not checked".into())],
    };
    checks.push(if problems.is_empty() {
        check(
            "fitsSplit",
            "Fits a short's split part (1080x864)",
            Status::Pass,
            "its portrait layout fits beside the camera",
        )
    } else {
        check(
            "fitsSplit",
            "Fits a short's split part (1080x864)",
            Status::Fail,
            format!(
                "in a part of a 9:16 short its portrait rules apply: {}",
                problems
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        )
    });

    // Follows the clip's length. A graphic that holds to the end (an end card) may stay shown.
    let holds = holds_end(&html);
    for (id, label, run, length_us) in [
        ("lengthShort", "Follows a 3 s length", wide, SHORT_US),
        ("lengthLong", "Follows an 8 s length", tall, LONG_US),
    ] {
        checks.push(timing(id, label, run, length_us, holds));
    }

    // Cut in and out where the app wants a cut, and marked an overlay only when it is one.
    checks.push(cut(result("c"), wide));
    checks.push(shape(wide, super::is_overlay(&html)));

    // Each string variable changes the picture.
    checks.push(if strings.is_empty() {
        check(
            "variables",
            "Text variables change the picture",
            Status::Warn,
            "it declares no string variables, so it can't take new text",
        )
    } else {
        let mut unchanged = Vec::new();
        for (n, decl) in strings.iter().enumerate() {
            let changed = result(&format!("v{n}"));
            let differs = wide.frames.len() >= 3
                && changed.frames.len() == 2
                && (differing_pixels(&wide.frames[1], &changed.frames[0]) >= CHANGED_PIXELS
                    || differing_pixels(&wide.frames[2], &changed.frames[1]) >= CHANGED_PIXELS);
            if !differs {
                unchanged.push(match &changed.error {
                    Some(error) => format!("{} (not rendered: {error})", decl.id),
                    None => decl.id.clone(),
                });
            }
        }
        if unchanged.is_empty() {
            check(
                "variables",
                "Text variables change the picture",
                Status::Pass,
                format!(
                    "{} each change it",
                    strings
                        .iter()
                        .map(|d| d.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        } else {
            check(
                "variables",
                "Text variables change the picture",
                Status::Fail,
                format!(
                    "another value of {} changes nothing on screen: show it (data-var-text or the script) or remove it",
                    unchanged.join(", ")
                ),
            )
        }
    });

    let order = [
        "hyperframes",
        "fitsWide",
        "fitsTall",
        "fitsSplit",
        "lengthShort",
        "lengthLong",
        "cut",
        "overlay",
        "variables",
        "network",
        "brandColours",
        "portrait",
        "fixedText",
    ];
    checks.sort_by_key(|c| order.iter().position(|o| *o == c.id).unwrap_or(order.len()));
    let mut report = Report {
        passed: checks.iter().all(|c| c.status != Status::Fail),
        checks,
        source_hash: hash.clone(),
        checked_at: chrono::Utc::now().to_rfc3339(),
        seconds: (started.elapsed().as_secs_f64() * 10.0).round() / 10.0,
        sheet: None,
    };

    let results_dir = results_dir(cache);
    std::fs::create_dir_all(&results_dir).map_err(|e| format!("{}: {e}", results_dir.display()))?;
    let tiles: Vec<(&str, Option<&RgbaImage>)> = vec![
        ("16:9", wide.frames.get(1)),
        ("16:9 long text", wide_long.frames.first()),
        ("9:16", tall.frames.get(1)),
        ("9:16 long", tall_long.frames.first()),
        ("Split part", split.frames.first()),
    ];
    let sheet_path = results_dir.join(format!("{hash}.png"));
    contact_sheet(&tiles)
        .save(&sheet_path)
        .map_err(|e| format!("{}: {e}", sheet_path.display()))?;
    let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    let json = results_dir.join(format!("{hash}.json"));
    std::fs::write(&json, text).map_err(|e| format!("{}: {e}", json.display()))?;
    report.sheet = Some(sheet_path.to_string_lossy().into_owned());
    Ok(report)
}

/// Whether the root composition element says the graphic holds to the end of its clip
/// (`data-holds-end`, as an end card does): its last frame may stay covered.
pub fn holds_end(html: &str) -> bool {
    let Some(at) = super::root_at(html) else {
        return false;
    };
    let start = html[..at].rfind('<').unwrap_or(0);
    let end = html[at..].find('>').map_or(html.len(), |e| at + e);
    html[start..end].contains("data-holds-end")
}

/// Whether `run`'s stills (at the start, half, three quarters and the last frame of
/// `length_us`) start and end see-through with the card shown in between; with `holds_end`
/// the last frame may stay covered.
fn timing(id: &str, label: &str, run: &RunResult, length_us: u64, holds_end: bool) -> Check {
    if run.frames.len() < 4 {
        return check(
            id,
            label,
            Status::Fail,
            run.error
                .clone()
                .unwrap_or_else(|| "its stills were not made".into()),
        );
    }
    let at = times(length_us, &[0.0, 0.5, 0.75], true);
    let mut problems = Vec::new();
    let covered = coverage(&run.frames[0], 96);
    if covered > SEE_THROUGH_MAX {
        problems.push(format!(
            "the first frame is {:.0}% covered: fade the card in from nothing",
            covered * 100.0
        ));
    }
    for n in [1, 2] {
        // A 3 s graphic may have faded out a little before three quarters.
        if n == 2 && length_us < LONG_US {
            continue;
        }
        let shown = coverage(&run.frames[n], 200);
        if shown < SHOWN_MIN {
            problems.push(format!(
                "nothing shows at {:.1} s: time it from the root's data-duration, not fixed times",
                at[n]
            ));
        }
    }
    let last = coverage(&run.frames[3], 96);
    if last > SEE_THROUGH_MAX && !holds_end {
        problems.push(format!(
            "the last frame ({:.2} s) is {:.0}% covered: fade out by the end of data-duration",
            at[3],
            last * 100.0
        ));
    }
    if problems.is_empty() {
        check(
            id,
            label,
            Status::Pass,
            if holds_end {
                "see-through at the start, shown from then on (it holds to the end)"
            } else {
                "see-through at both ends, shown in between"
            },
        )
    } else {
        check(id, label, Status::Fail, problems.join("; "))
    }
}

/// Whether `cut`'s stills (the first frame, the middle and the last, rendered with
/// `data-entrance="cut"`) show the card whole at both ends: a graphic that fills the frame never
/// eases, so it must not fade or slide in when the app cuts it in. `eased` is the 16:9 run whose
/// middle still says how much the card covers.
fn cut(cut: &RunResult, eased: &RunResult) -> Check {
    let (id, label) = ("cut", "Cuts in and out");
    if cut.frames.len() < 3 {
        return check(
            id,
            label,
            Status::Fail,
            cut.error
                .clone()
                .unwrap_or_else(|| "its stills were not made".into()),
        );
    }
    let shown = eased
        .frames
        .get(1)
        .map_or_else(|| coverage(&cut.frames[1], 200), |f| coverage(f, 200));
    let need = (shown * CUT_SHOWN_SHARE).max(SHOWN_MIN);
    let mut problems = Vec::new();
    for (frame, which) in [(&cut.frames[0], "first"), (&cut.frames[2], "last")] {
        let covered = coverage(frame, 96);
        if covered < need {
            problems.push(format!(
                "with data-entrance=\"cut\" the {which} frame is {:.0}% covered (the card covers {:.0}%)",
                covered * 100.0,
                shown * 100.0
            ));
        }
    }
    if problems.is_empty() {
        check(
            id,
            label,
            Status::Pass,
            "shown whole on its first and last frames when the app cuts it in",
        )
    } else {
        check(
            id,
            label,
            Status::Fail,
            format!(
                "{}: fade or slide the card in and out only when root.dataset.entrance is \"ease\"; otherwise show it whole from the first frame to the last",
                problems.join("; ")
            ),
        )
    }
}

/// Whether the graphic's `data-overlay` mark matches how much of the frame its card covers in
/// `eased`'s middle still: a graphic that fills the frame never eases, so it must not say it is
/// an overlay; one that covers only part of it should, so it eases over the full-screen camera.
fn shape(eased: &RunResult, overlay: bool) -> Check {
    let (id, label) = ("overlay", "Full screen or overlay");
    let Some(middle) = eased.frames.get(1) else {
        return check(id, label, Status::Fail, "its stills were not made");
    };
    let shown = coverage(middle, 200);
    let percent = shown * 100.0;
    if overlay && shown >= FILLS_MIN {
        check(
            id,
            label,
            Status::Fail,
            format!(
                "it covers {percent:.0}% of the frame but says data-overlay: a graphic that fills the frame never eases, so remove data-overlay"
            ),
        )
    } else if !overlay && shown < OVERLAY_MAX {
        check(
            id,
            label,
            Status::Warn,
            format!(
                "it covers {percent:.0}% of the frame: if it is an overlay (a lower third, a callout), put data-overlay on the root so it eases over the full-screen camera"
            ),
        )
    } else if overlay {
        check(
            id,
            label,
            Status::Pass,
            format!(
                "an overlay ({percent:.0}% of the frame): it eases over the full-screen camera and cuts elsewhere"
            ),
        )
    } else {
        check(
            id,
            label,
            Status::Pass,
            format!("fills the frame ({percent:.0}%): it always cuts in and out"),
        )
    }
}

/// Stages and renders every run, `PARALLEL` at a time.
fn execute(
    paths: &runtime::RuntimePaths,
    dir: &Path,
    html: &str,
    work: &Path,
    runs: &[Run],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f64, &str),
) -> Vec<RunResult> {
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<RunResult>>> =
        Mutex::new((0..runs.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..PARALLEL.min(runs.len()) {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::SeqCst);
                let Some(run) = runs.get(index) else {
                    break;
                };
                let result = if cancel.load(Ordering::Relaxed) {
                    RunResult {
                        error: Some("Cancelled".into()),
                        ..Default::default()
                    }
                } else {
                    one_run(paths, dir, html, work, run, cancel)
                };
                results.lock().unwrap_or_else(|e| e.into_inner())[index] = Some(result);
                done.fetch_add(1, Ordering::SeqCst);
            });
        }
        let mut shown = usize::MAX;
        loop {
            let finished = done.load(Ordering::SeqCst);
            if finished != shown {
                shown = finished;
                progress(
                    finished as f64 / runs.len() as f64,
                    &format!("Checking the graphic: {finished} of {} renders", runs.len()),
                );
            }
            if finished >= runs.len() {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
    results
        .into_inner()
        .unwrap_or_else(|e| e.into_inner())
        .into_iter()
        .map(Option::unwrap_or_default)
        .collect()
}

fn one_run(
    paths: &runtime::RuntimePaths,
    dir: &Path,
    html: &str,
    work: &Path,
    run: &Run,
    cancel: &AtomicBool,
) -> RunResult {
    let stage = work.join(&run.key);
    let stills = work.join(format!("{}-s", run.key));
    let staged = (|| {
        render::copy_dir(dir, &stage)?;
        let sized = super::size_composition(
            html,
            run.size.0,
            run.size.1,
            run.length_us as f64 / 1e6,
            run.entrance,
        );
        let mut html = super::with_defaults(&sized, &run.values)?;
        if run.split {
            let marked = super::with_root_attribute(&html, "data-mode", "split");
            html = super::as_split_part(&marked, run.size.0, run.size.1);
        }
        std::fs::write(stage.join(super::ENTRY), html).map_err(|e| e.to_string())?;
        // GSAP, when the composition loads it, beside it (fetched once).
        super::gsap::stage(&stage, cancel)
    })();
    if let Err(error) = staged {
        return RunResult {
            error: Some(error),
            ..Default::default()
        };
    }
    let mut result = RunResult::default();
    if run.check {
        let mut args: Vec<OsString> = vec![
            "check".into(),
            stage.clone().into(),
            "--json".into(),
            "--frame-check".into(),
        ];
        if !run.contrast {
            args.push("--no-contrast".into());
        }
        match run_cli(paths, args, cancel) {
            Ok((_, stdout, stderr)) => {
                match stdout
                    .find('{')
                    .and_then(|at| serde_json::from_str::<Value>(&stdout[at..]).ok())
                {
                    Some(report) => result.check = Some(report),
                    None => result.error = Some(tail(&stderr, &stdout)),
                }
            }
            Err(error) => result.error = Some(error),
        }
    }
    let at = run
        .at
        .iter()
        .map(|t| format!("{t:.3}"))
        .collect::<Vec<_>>()
        .join(",");
    let args: Vec<OsString> = vec![
        "snapshot".into(),
        stage.into(),
        "--output".into(),
        stills.clone().into(),
        "--at".into(),
        at.into(),
        "--no-end".into(),
        // Never send frames to a vision model, whatever is in the environment.
        "--describe".into(),
        "false".into(),
    ];
    match run_cli(paths, args, cancel) {
        Ok((ok, stdout, stderr)) => {
            let mut pngs: Vec<PathBuf> = std::fs::read_dir(&stills)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("frame-") && n.ends_with(".png"))
                })
                .collect();
            pngs.sort();
            for png in pngs {
                match image::open(&png) {
                    Ok(frame) => result.frames.push(frame.to_rgba8()),
                    Err(e) => {
                        result
                            .error
                            .get_or_insert(format!("{}: {e}", png.display()));
                    }
                }
            }
            if result.frames.len() != run.at.len() {
                result.error.get_or_insert(if ok {
                    format!(
                        "{} of {} stills were made",
                        result.frames.len(),
                        run.at.len()
                    )
                } else {
                    tail(&stderr, &stdout)
                });
            }
        }
        Err(error) => {
            result.error.get_or_insert(error);
        }
    }
    result
}

/// The last lines of a failed command's output, for a reason.
fn tail(stderr: &str, stdout: &str) -> String {
    let text = render::strip_ansi(&format!("{stderr}\n{stdout}"));
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains("[INFO]") && !l.starts_with("[hyperframes]"))
        .collect();
    let start = lines.len().saturating_sub(4);
    let joined = lines[start..].join(" / ");
    if joined.is_empty() {
        "HyperFrames stopped without saying why".into()
    } else {
        joined
    }
}

/// Runs the pinned HyperFrames with `args`: (succeeded, stdout, stderr).
/// Stills of the composition staged in `stage` at `at` (seconds, comma-separated) as PNGs in
/// `output`.
pub fn snapshot(
    paths: &runtime::RuntimePaths,
    stage: &Path,
    at: &str,
    output: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let args: Vec<OsString> = vec![
        "snapshot".into(),
        stage.into(),
        "--output".into(),
        output.into(),
        "--at".into(),
        at.into(),
        "--no-end".into(),
        // Never send frames to a vision model, whatever is in the environment.
        "--describe".into(),
        "false".into(),
    ];
    let (ok, stdout, stderr) = run_cli(paths, args, cancel)?;
    if ok {
        Ok(())
    } else {
        Err(tail(&stderr, &stdout))
    }
}

fn run_cli(
    paths: &runtime::RuntimePaths,
    args: Vec<OsString>,
    cancel: &AtomicBool,
) -> Result<(bool, String, String), String> {
    let mut command = Command::new(&paths.node);
    command
        .arg(&paths.cli)
        .args(args)
        .env("HYPERFRAMES_BROWSER_PATH", &paths.chrome)
        .env("HYPERFRAMES_FONT_CACHE_DIR", paths.cache.join("fonts"))
        .env("HYPERFRAMES_NO_TELEMETRY", "1")
        .env("HYPERFRAMES_SKIP_SKILLS", "1")
        .env("NO_COLOR", "1")
        .env_remove("GEMINI_API_KEY")
        .env("PATH", runtime::render_path()?)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Ok(ffmpeg) = crate::media::ffmpeg::ffmpeg_path() {
        command.env("HYPERFRAMES_FFMPEG_PATH", ffmpeg);
    }
    if let Ok(ffprobe) = crate::media::ffmpeg::ffprobe_path() {
        command.env("HYPERFRAMES_FFPROBE_PATH", ffprobe);
    }
    runtime::hide_window(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start the graphics renderer: {e}"))?;
    let read = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_string(&mut text);
            }
            text
        })
    };
    let stdout = read(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let stderr = read(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if cancel.load(Ordering::Relaxed) || started.elapsed() > RUN_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(if cancel.load(Ordering::Relaxed) {
                "Cancelled".into()
            } else {
                "HyperFrames took too long".into()
            });
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let stdout = stdout.join().unwrap_or_default();
    let stderr = stderr.join().unwrap_or_default();
    Ok((status.success(), stdout, stderr))
}

/// The tiles side by side over a checkerboard (so see-through parts show), each 270 px high
/// with its label above.
fn contact_sheet(tiles: &[(&str, Option<&RgbaImage>)]) -> RgbaImage {
    const HEIGHT: u32 = 270;
    const GAP: u32 = 12;
    const LABEL: u32 = 26;
    let sized: Vec<(&str, RgbaImage)> = tiles
        .iter()
        .map(|(label, frame)| {
            let tile = match frame {
                Some(frame) => {
                    let width = ((frame.width() as f64 * HEIGHT as f64
                        / frame.height().max(1) as f64)
                        .round() as u32)
                        .max(1);
                    image::imageops::resize(
                        *frame,
                        width,
                        HEIGHT,
                        image::imageops::FilterType::Triangle,
                    )
                }
                // Not rendered: a dark red square.
                None => RgbaImage::from_pixel(HEIGHT, HEIGHT, Rgba([90, 20, 20, 255])),
            };
            (*label, tile)
        })
        .collect();
    let width = GAP + sized.iter().map(|(_, t)| t.width() + GAP).sum::<u32>();
    let height = GAP + LABEL + HEIGHT + GAP;
    let mut sheet = RgbaImage::from_pixel(width, height, Rgba([24, 24, 27, 255]));
    let mut x = GAP;
    for (label, tile) in &sized {
        let y = GAP + LABEL;
        for ty in 0..tile.height() {
            for tx in 0..tile.width() {
                let checker = if ((tx / 10) + (ty / 10)) % 2 == 0 {
                    63
                } else {
                    82
                };
                let p = tile.get_pixel(tx, ty).0;
                let a = p[3] as u32;
                let mix = |c: u8| ((c as u32 * a + checker * (255 - a)) / 255) as u8;
                sheet.put_pixel(x + tx, y + ty, Rgba([mix(p[0]), mix(p[1]), mix(p[2]), 255]));
            }
        }
        draw_text(&mut sheet, x, GAP + LABEL - 8, label, 16.0);
        x += tile.width() + GAP;
    }
    sheet
}

/// Light text with its baseline at `baseline`.
fn draw_text(image: &mut RgbaImage, x: u32, baseline: u32, text: &str, size: f32) {
    let font = crate::captions::font();
    let mut pen = x as f32;
    for c in text.chars() {
        let (metrics, coverage) = font.rasterize(c, size);
        let gx = (pen + metrics.xmin as f32).round() as i64;
        let gy = baseline as i64 - metrics.height as i64 - metrics.ymin as i64;
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let (px, py) = (gx + col as i64, gy + row as i64);
                if px < 0 || py < 0 || px >= image.width() as i64 || py >= image.height() as i64 {
                    continue;
                }
                let a = coverage[row * metrics.width + col] as u32;
                let under = image.get_pixel(px as u32, py as u32).0;
                let mix = |c: u8| ((c as u32 * (255 - a) + 228 * a) / 255) as u8;
                image.put_pixel(
                    px as u32,
                    py as u32,
                    Rgba([mix(under[0]), mix(under[1]), mix(under[2]), 255]),
                );
            }
        }
        pen += metrics.advance_width;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decls(json: Value) -> Vec<VariableDecl> {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn the_bundled_templates_pass_the_file_checks() {
        for template in super::super::templates::all() {
            let declared = super::super::parse_declarations(template.html()).unwrap();
            let files = vec![("index.html".to_string(), template.html().to_string())];
            for c in static_checks(template.html(), &declared, &files) {
                assert_eq!(
                    c.status,
                    Status::Pass,
                    "{} {}: {}",
                    template.name,
                    c.id,
                    c.reason
                );
            }
        }
    }

    #[test]
    fn a_graphic_of_its_own_colours_fails_the_look_and_fixed_style_warns() {
        let check = |style: &str| {
            look_check(&format!(
                "<html><head><style>{style}</style></head><body><div id=\"card-back\"></div></body></html>"
            ))
        };
        // Kit colours and the look's tokens (with fallbacks) pass; selectors are no colours.
        let fine = check(
            "#card-back { background: var(--background, #fff); color: var(--look-muted); \
             border-radius: var(--look-radius); font-family: var(--look-font); } \
             #fade { border-radius: 99vmin; } .dot { border-radius: 50%; }",
        );
        assert_eq!(fine.status, Status::Pass, "{}", fine.reason);
        let accent = check(".title { color: #FACC15; }");
        assert_eq!(accent.status, Status::Fail);
        assert!(accent.reason.contains("#FACC15"), "{}", accent.reason);
        assert_eq!(
            check(".a { background: rgba(0, 0, 0, 0.4); }").status,
            Status::Fail
        );
        let fixed = check(".card { border-radius: 24px; font-family: Inter; }");
        assert_eq!(fixed.status, Status::Warn);
        assert!(fixed.reason.contains("24px") && fixed.reason.contains("Inter"));
        // In a script too.
        let html = "<html><body><script>gsap.to('#t', { color: '#ff0000' })</script></body></html>";
        assert_eq!(look_check(html).status, Status::Fail);
    }

    #[test]
    fn the_network_and_missing_colours_fail() {
        let html = r#"<html><body><script src="https://cdn.example.com/gsap.js"></script>
            <svg xmlns="http://www.w3.org/2000/svg"></svg>
            <div style="background:url(//fonts.example.com/a.png)"></div></body></html>"#;
        let declared = decls(serde_json::json!([
            {"id":"title","type":"string","default":"Hi"},
            {"id":"accent","type":"color","default":"#fff"}
        ]));
        let checks = static_checks(html, &declared, &[("index.html".into(), html.into())]);
        let by = |id: &str| checks.iter().find(|c| c.id == id).unwrap();
        assert_eq!(by("network").status, Status::Fail);
        assert!(
            by("network").reason.contains("cdn.example.com"),
            "{}",
            by("network").reason
        );
        assert!(by("network").reason.contains("//fonts.example.com"));
        assert!(!by("network").reason.contains("w3.org"));
        assert_eq!(by("brandColours").status, Status::Fail);
        assert!(by("brandColours").reason.contains("background, text"));
        assert_eq!(by("portrait").status, Status::Warn);
    }

    #[test]
    fn fixed_text_is_text_no_variable_or_script_fills() {
        let html = r#"<html data-composition-variables='[]'><head><style>h1{}</style></head>
            <body><div id="root" data-composition-id="g">
              <h1 data-var-text="title">Filled</h1>
              <div class="tag">NEW</div>
              <span id="count">Step 3</span>
              <!-- a note -->
              <img src="logo.png"><p>Made by <b>us</b></p>
            </div><script>document.getElementById("count").textContent = "x";</script></body></html>"#;
        assert_eq!(fixed_text(html), vec!["NEW", "Made by", "us"]);
    }

    #[test]
    fn long_values_keep_their_lines_and_grow() {
        assert_eq!(long_value("Part 2"), "Part 2 Part 2 Part 2 Part");
        let items = long_value("Install the tools\nRun it");
        assert_eq!(items.lines().count(), 2);
        assert!(items.lines().all(|l| l.chars().count() >= 24), "{items}");
        assert!(long_value("").starts_with("Configure the continuous"));
        assert!(long_value(&"word ".repeat(100)).chars().count() <= 145);
        assert_eq!(other_value("a\nb\nc").lines().count(), 3);
    }

    #[test]
    fn defaults_are_set_in_the_declarations() {
        let html = r#"<html data-composition-variables='[{"id":"title","type":"string","default":"Hi"},{"id":"n","type":"number","default":1}]'>"#;
        let values = serde_json::json!({"title": "It's <b> & \"quoted\"", "n": 4});
        let staged = super::super::with_defaults(html, values.as_object().unwrap()).unwrap();
        let declared = super::super::parse_declarations(&staged).unwrap();
        assert_eq!(declared[0].default, values["title"]);
        assert_eq!(declared[1].default, 4);
        assert!(staged.ends_with("'>"), "{staged}");
        // A double-quoted attribute stays one.
        let double = "<html data-composition-variables=\"[{&quot;id&quot;:&quot;t&quot;,&quot;type&quot;:&quot;string&quot;,&quot;default&quot;:&quot;a&quot;}]\">";
        let staged = super::super::with_defaults(double, values.as_object().unwrap()).unwrap();
        assert!(staged.contains("&quot;id&quot;"));
    }

    #[test]
    fn coverage_and_differences_are_counted() {
        let mut frame = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 0]));
        assert_eq!(coverage(&frame, 96), 0.0);
        for x in 0..10 {
            frame.put_pixel(x, 0, Rgba([255, 255, 255, 255]));
        }
        assert!((coverage(&frame, 200) - 0.1).abs() < 1e-9);
        let blank = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 0]));
        assert_eq!(differing_pixels(&frame, &blank), 10);
        assert_eq!(differing_pixels(&frame, &frame), 0);
        assert_eq!(differing_pixels(&frame, &RgbaImage::new(5, 5)), 100);
    }

    #[test]
    fn layout_overflows_and_errors_are_read_from_the_check() {
        let report = serde_json::json!({
            "lint": {"findings": [{"severity": "error", "code": "x", "message": "Bad root"}]},
            "layout": {"findings": [
                {"severity": "warning", "code": "canvas_overflow", "selector": "#title",
                 "message": "Text extends outside the composition canvas.", "text": "Long",
                 "fixHint": "Shrink it."},
                {"severity": "warning", "code": "low_coverage", "message": "fine"}
            ], "samples": [0.5, 1.0]}
        });
        let found = overflows(&report);
        assert_eq!(found.len(), 1);
        assert!(found[0].starts_with("#title Text extends") && found[0].contains("Shrink it."));
        assert_eq!(findings(&report, "lint", "error"), vec!["Bad root"]);
        // Lint errors stop HyperFrames before the browser: nothing measured is no pass.
        let skipped =
            serde_json::json!({"browserSkipped": true, "layout": {"findings": [], "samples": []}});
        assert!(overflows(&skipped)[0].starts_with("not measured"));
    }

    #[test]
    fn the_sheet_holds_every_tile() {
        let wide = RgbaImage::from_pixel(192, 108, Rgba([255, 0, 0, 255]));
        let tall = RgbaImage::from_pixel(108, 192, Rgba([0, 0, 255, 128]));
        let sheet = contact_sheet(&[("16:9", Some(&wide)), ("9:16", Some(&tall)), ("x", None)]);
        assert_eq!(sheet.height(), 12 + 26 + 270 + 12);
        assert_eq!(sheet.width(), 12 + (480 + 12) + (152 + 12) + (270 + 12));
        assert_eq!(sheet.get_pixel(12 + 10, 12 + 26 + 10).0, [255, 0, 0, 255]);
    }

    /// A graphic written for one spot: fixed 4 s timing, a px title that never wraps, colours
    /// written in, a font from the web, and a subtitle variable nothing shows.
    const ONE_OFF: &str = r##"<!doctype html>
<html data-composition-variables='[
  {"id":"title","type":"string","label":"Title","default":"Launch day"},
  {"id":"subtitle","type":"string","label":"Subtitle","default":"Coming soon"}
]'>
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=1920, height=1080" />
<link rel="stylesheet" href="https://fonts.example.com/css?family=Display" />
<style>
  html, body { margin: 0; background: transparent; }
  #root { position: relative; width: 100%; height: 100%; overflow: hidden; }
  #card { position: absolute; inset: 0; background: #1E3A8A; }
  h1 { position: absolute; left: 120px; top: 400px; margin: 0; font-size: 180px;
       white-space: nowrap; color: #FFFFFF; font-family: sans-serif; }
</style>
</head>
<body>
<div id="root" data-composition-id="graphic" data-no-timeline data-start="0" data-width="1920" data-height="1080" data-duration="4">
  <div id="card"></div>
  <h1 id="title" data-var-text="title">Launch day</h1>
</div>
<script>
  for (const id of ["card", "title"]) {
    document.getElementById(id).animate(
      [{ opacity: 0 }, { opacity: 1, offset: 0.1 }, { opacity: 1, offset: 0.85 }, { opacity: 0 }],
      { duration: 4000, fill: "both", iterations: 1 },
    ).pause();
  }
</script>
</body>
</html>
"##;

    /// The gate through the installed runtime: every bundled template passes, a one-off graphic
    /// fails for its reasons, and an unchanged graphic answers from the cache.
    /// `AEROEDITS_GRAPHICS_RUNTIME=<installed runtime> cargo test --lib graphics_gate_real --
    /// --ignored --nocapture` (without it, the app's own runtime is used).
    #[test]
    #[ignore]
    fn a_gsap_graphic_passes_the_gate_real() {
        // Written the HyperFrames way, GSAP from its CDN: the file gets the local name, the
        // render fetches GSAP once and stages it, and the graphic passes as it is.
        let root = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let asset = super::super::create(root.path(), "blank", None, &Map::new()).unwrap();
        let dir = super::super::folder(root.path(), &asset.id);
        let html = std::fs::read_to_string(dir.join(super::super::ENTRY))
            .unwrap()
            .replace(
                r#"<script src="gsap.min.js"></script>"#,
                r#"<script src="https://cdn.jsdelivr.net/npm/gsap@3.15.0/dist/gsap.min.js"></script>"#,
            );
        assert!(html.contains("cdn.jsdelivr.net"));
        super::super::write_file(&dir, super::super::ENTRY, html.as_bytes()).unwrap();
        let written = std::fs::read_to_string(dir.join(super::super::ENTRY)).unwrap();
        assert!(written.contains(r#"<script src="gsap.min.js">"#) && !written.contains("cdn."));
        let cache = super::super::cache_folder(root.path(), &asset.id);
        let report = run(&dir, &cache, &cancel, &mut |_: f64, _: &str| {}).unwrap();
        eprintln!(
            "gsap blank ({} s):\n  {}",
            report.seconds,
            report.lines().join("\n  ")
        );
        assert!(report.passed, "the GSAP blank fails the gate");
        // GSAP lives beside the renderer, not in the project.
        assert!(!dir.join("gsap.min.js").exists());
    }

    #[test]
    #[ignore]
    fn graphics_gate_real() {
        let root = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let mut quiet = |_: f64, _: &str| {};
        for template in super::super::templates::all() {
            if template.name == "blank" {
                continue;
            }
            let asset =
                super::super::create(root.path(), template.name, None, &Map::new()).unwrap();
            let dir = super::super::folder(root.path(), &asset.id);
            let cache = super::super::cache_folder(root.path(), &asset.id);
            let report = run(&dir, &cache, &cancel, &mut quiet).unwrap();
            eprintln!(
                "{} ({} s):\n  {}",
                template.name,
                report.seconds,
                report.lines().join("\n  ")
            );
            assert!(report.passed, "{} fails the gate", template.name);
            assert!(Path::new(report.sheet.as_deref().unwrap()).is_file());
            if template.name == "title" {
                let started = Instant::now();
                let again = run(&dir, &cache, &cancel, &mut quiet).unwrap();
                assert_eq!(again.checks, report.checks);
                assert!(
                    started.elapsed() < Duration::from_secs(2),
                    "answered from the cache"
                );
                assert_eq!(
                    cached(&dir, &cache).unwrap().source_hash,
                    report.source_hash
                );
            }
        }

        let asset =
            super::super::create(root.path(), "blank", Some("Launch"), &Map::new()).unwrap();
        let dir = super::super::folder(root.path(), &asset.id);
        super::super::write_file(&dir, "index.html", ONE_OFF.as_bytes()).unwrap();
        let cache = super::super::cache_folder(root.path(), &asset.id);
        let report = run(&dir, &cache, &cancel, &mut |f, m| {
            eprintln!("{:.0}% {m}", f * 100.0)
        })
        .unwrap();
        eprintln!(
            "one-off ({} s):\n  {}",
            report.seconds,
            report.lines().join("\n  ")
        );
        assert!(!report.passed);
        let status = |id: &str| report.checks.iter().find(|c| c.id == id).unwrap().status;
        for failing in [
            "network",
            "brandColours",
            "fitsTall",
            "lengthShort",
            "lengthLong",
            "variables",
        ] {
            assert_eq!(status(failing), Status::Fail, "{failing}");
        }
        let variables = report.checks.iter().find(|c| c.id == "variables").unwrap();
        assert!(
            variables.reason.contains("subtitle") && !variables.reason.contains("title,"),
            "{}",
            variables.reason
        );
        // The sheet shows both shapes.
        let sheet = image::open(report.sheet.as_deref().unwrap()).unwrap();
        assert!(sheet.width() > sheet.height() * 3);
    }

    #[test]
    fn timing_needs_see_through_ends_and_a_shown_middle() {
        let clear = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 0]));
        let card = RgbaImage::from_pixel(10, 10, Rgba([9, 9, 9, 255]));
        let run = |frames: Vec<&RgbaImage>| RunResult {
            frames: frames.into_iter().cloned().collect(),
            ..Default::default()
        };
        let good = run(vec![&clear, &card, &card, &clear]);
        assert_eq!(timing("l", "L", &good, LONG_US, false).status, Status::Pass);
        // A graphic timed for 4 s: gone by the middle of an 8 s clip.
        let fixed = run(vec![&clear, &clear, &clear, &clear]);
        let failed = timing("l", "L", &fixed, LONG_US, false);
        assert_eq!(failed.status, Status::Fail);
        assert!(failed.reason.contains("data-duration"), "{}", failed.reason);
        // And still showing at the end of a 3 s one.
        let late = run(vec![&clear, &card, &card, &card]);
        assert!(timing("s", "S", &late, SHORT_US, false)
            .reason
            .contains("last frame"));
        // Unless it holds to the end; its first frame must still be see-through.
        assert_eq!(timing("s", "S", &late, SHORT_US, true).status, Status::Pass);
        let covered = run(vec![&card, &card, &card, &card]);
        assert!(timing("s", "S", &covered, SHORT_US, true)
            .reason
            .contains("first frame"));
        assert!(holds_end(
            "<div id=\"root\" data-composition-id=\"g\"
  data-holds-end
>"
        ));
        assert!(!holds_end(
            "<div data-composition-id=\"g\"><p data-holds-end></p>"
        ));
    }

    /// A 10x10 frame with its bottom two rows covered: a lower third.
    fn lower_third() -> RgbaImage {
        let mut third = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 0]));
        for x in 0..10 {
            for y in 8..10 {
                third.put_pixel(x, y, Rgba([9, 9, 9, 255]));
            }
        }
        third
    }

    #[test]
    fn a_cut_shows_the_card_whole_at_both_ends() {
        let clear = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 0]));
        let card = RgbaImage::from_pixel(10, 10, Rgba([9, 9, 9, 255]));
        let run = |frames: Vec<&RgbaImage>| RunResult {
            frames: frames.into_iter().cloned().collect(),
            ..Default::default()
        };
        let eased = run(vec![&clear, &card, &card, &clear]);
        // A full-screen card cut in: whole on the first and last frames.
        let whole = run(vec![&card, &card, &card]);
        assert_eq!(cut(&whole, &eased).status, Status::Pass);
        // Still fading in and out although the app asked for a cut.
        let fades = run(vec![&clear, &card, &clear]);
        let failed = cut(&fades, &eased);
        assert_eq!(failed.status, Status::Fail);
        assert!(
            failed.reason.contains("first frame") && failed.reason.contains("last frame"),
            "{}",
            failed.reason
        );
        assert!(failed.reason.contains("entrance"), "{}", failed.reason);
        // A lower third cut in is measured against its own size.
        let third = lower_third();
        let eased_third = run(vec![&clear, &third, &third, &clear]);
        assert_eq!(
            cut(&run(vec![&third, &third, &third]), &eased_third).status,
            Status::Pass
        );
        assert_eq!(
            cut(&run(vec![&clear, &third, &third]), &eased_third).status,
            Status::Fail
        );
    }

    #[test]
    fn the_overlay_mark_matches_what_the_card_covers() {
        let clear = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 0]));
        let card = RgbaImage::from_pixel(10, 10, Rgba([9, 9, 9, 255]));
        let run = |middle: &RgbaImage| RunResult {
            frames: vec![clear.clone(), middle.clone(), clear.clone(), clear.clone()],
            ..Default::default()
        };
        // A full-screen card is not an overlay: it never eases.
        assert_eq!(shape(&run(&card), false).status, Status::Pass);
        let claimed = shape(&run(&card), true);
        assert_eq!(claimed.status, Status::Fail);
        assert!(claimed.reason.contains("never eases"), "{}", claimed.reason);
        // A lower third should say it is one, so it eases over the camera.
        assert_eq!(shape(&run(&lower_third()), true).status, Status::Pass);
        let unmarked = shape(&run(&lower_third()), false);
        assert_eq!(unmarked.status, Status::Warn);
        assert!(
            unmarked.reason.contains("data-overlay"),
            "{}",
            unmarked.reason
        );
    }
}
