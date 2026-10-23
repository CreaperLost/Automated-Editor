//! Bringing a video's graphics in line with its look: what `graphic_check` finds a graphic
//! deciding for itself (its own colours, fonts, corners and CSS timings, and look tokens of its
//! own) rewritten to the look's tokens and the style's colours, by rule (no model).
//!
//! Only styles are rewritten (`<style>` blocks, `style="..."` attributes and `.css` files);
//! colours in scripts, and colours no role of the style is close to (a red "false" stamp is
//! meant), stay and are listed. Applying makes each changed graphic a copy with the new files
//! and points its clips at the copy, so undo brings the old one back as it was.
use super::{gate, look};
use crate::project::revision::EditDocument;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A colour is taken for a role of the style only this close (distance in RGB, 0-255 each).
const NEAR: f64 = 72.0;
/// The look's duration as a CSS time (the token is milliseconds without a unit, for scripts).
const DURATION: &str = "calc(var(--look-duration) * 1ms)";

/// What bringing one graphic in line changes, worked out from its files as they are.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    /// The graphic's asset id.
    pub graphic: String,
    pub name: String,
    /// Its files as planned on (`gate::source_hash`): a graphic changed since is not touched.
    pub stamp: String,
    /// What changes, one line each.
    pub changes: Vec<String>,
    /// What stays its own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<String>,
}

impl Plan {
    /// One line for a summary: its name, what changes and what stays.
    pub fn line(&self) -> String {
        let mut line = format!("\"{}\": {}", self.name, self.changes.join("; "));
        if !self.left.is_empty() {
            line.push_str(&format!(" (stays its own: {})", self.left.join(", ")));
        }
        line
    }
}

type Rgb = [f64; 3];

/// The style's colour roles, as a graphic would write them, and their colours.
struct Palette {
    /// (what to write, its colour, a role a see-through colour can take)
    roles: Vec<(&'static str, Rgb, bool)>,
}

fn mix(a: Rgb, b: Rgb, amount: f64) -> Rgb {
    [0, 1, 2].map(|i| a[i] * amount + b[i] * (1.0 - amount))
}

impl Palette {
    /// The roles of the kit's colours and of the app look's (graphics are often written for
    /// those), with the app preset's mixes.
    fn new(kit: Option<(&str, &str, &str)>) -> Self {
        let mut roles = Vec::new();
        let defaults = (look::BACKGROUND, look::TEXT, look::ACCENT);
        for (background, text, accent) in kit.into_iter().chain([defaults]) {
            let (Some(bg), Some(fg), Some(ac)) = (hex(background), hex(text), hex(accent)) else {
                continue;
            };
            roles.extend([
                ("var(--background)", bg, true),
                ("var(--text)", fg, true),
                ("var(--accent)", ac, true),
                ("var(--look-muted)", mix(fg, bg, 0.62), false),
                ("var(--look-line)", mix(fg, bg, 0.08), false),
                ("var(--look-tint)", mix(ac, bg, 0.10), false),
            ]);
        }
        Self { roles }
    }

    /// What to write for `colour` (with its alpha), when a role is close to it.
    fn role_for(&self, colour: Rgb, alpha: f64) -> Option<String> {
        let see_through = alpha < 0.995;
        let distance = |c: Rgb| ((0..3).map(|i| (c[i] - colour[i]).powi(2)).sum::<f64>()).sqrt();
        let (role, _, _) = self
            .roles
            .iter()
            .filter(|(_, _, takes_alpha)| !see_through || *takes_alpha)
            .map(|r| (r, distance(r.1)))
            .filter(|(_, d)| *d <= NEAR)
            .min_by(|a, b| a.1.total_cmp(&b.1))?
            .0;
        Some(if see_through {
            format!(
                "color-mix(in srgb, {role} {}%, transparent)",
                (alpha * 100.0).round()
            )
        } else {
            role.to_string()
        })
    }
}

fn hex(text: &str) -> Option<Rgb> {
    parse_colour(text).map(|(rgb, _)| rgb)
}

/// A colour literal's RGB (0-255) and alpha (0-1): `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`,
/// `rgb()`/`rgba()` and `hsl()`/`hsla()`.
fn parse_colour(text: &str) -> Option<(Rgb, f64)> {
    let text = text.trim();
    if let Some(digits) = text.strip_prefix('#') {
        let nibble = |i: usize| {
            u8::from_str_radix(&digits[i..i + 1], 16)
                .ok()
                .map(|v| v as f64 * 17.0)
        };
        let byte = |i: usize| {
            u8::from_str_radix(&digits[i..i + 2], 16)
                .ok()
                .map(f64::from)
        };
        if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        return match digits.len() {
            3 => Some(([nibble(0)?, nibble(1)?, nibble(2)?], 1.0)),
            4 => Some(([nibble(0)?, nibble(1)?, nibble(2)?], nibble(3)? / 255.0)),
            6 => Some(([byte(0)?, byte(2)?, byte(4)?], 1.0)),
            8 => Some(([byte(0)?, byte(2)?, byte(4)?], byte(6)? / 255.0)),
            _ => None,
        };
    }
    let lower = text.to_ascii_lowercase();
    let open = lower.find('(')?;
    let function = &lower[..open];
    let inner = lower[open + 1..].strip_suffix(')')?;
    let parts: Vec<&str> = inner
        .split([',', ' ', '/'])
        .filter(|p| !p.is_empty())
        .collect();
    let number = |p: &str| -> Option<f64> {
        let p = p.trim_end_matches("deg");
        match p.strip_suffix('%') {
            Some(n) => n.parse::<f64>().ok().map(|v| v / 100.0),
            None => p.parse::<f64>().ok(),
        }
    };
    let alpha = match parts.get(3) {
        Some(p) => number(p)?.clamp(0.0, 1.0),
        None => 1.0,
    };
    match function {
        "rgb" | "rgba" => {
            let channel = |p: &str| -> Option<f64> {
                Some(match p.strip_suffix('%') {
                    Some(n) => n.parse::<f64>().ok()? * 2.55,
                    None => p.parse::<f64>().ok()?,
                })
            };
            Some((
                [
                    channel(parts.first()?)?,
                    channel(parts.get(1)?)?,
                    channel(parts.get(2)?)?,
                ],
                alpha,
            ))
        }
        "hsl" | "hsla" => {
            let h = parts.first()?.trim_end_matches("deg").parse::<f64>().ok()?;
            let (s, l) = (number(parts.get(1)?)?, number(parts.get(2)?)?);
            let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
            let h = h.rem_euclid(360.0) / 60.0;
            let x = c * (1.0 - (h % 2.0 - 1.0).abs());
            let (r, g, b) = match h as u32 {
                0 => (c, x, 0.0),
                1 => (x, c, 0.0),
                2 => (0.0, c, x),
                3 => (0.0, x, c),
                4 => (x, 0.0, c),
                _ => (c, 0.0, x),
            };
            let m = l - c / 2.0;
            Some(([(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0], alpha))
        }
        _ => None,
    }
}

/// Where the next `;`, `{` or `}` of `css` is, outside comments, strings and brackets.
fn next_boundary(css: &str) -> Option<usize> {
    let bytes = css.as_bytes();
    let (mut i, mut depth) = (0, 0i32);
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = css[i + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |e| i + 2 + e + 2);
                continue;
            }
            quote @ (b'"' | b'\'') => {
                i = css[i + 1..]
                    .find(quote as char)
                    .map_or(bytes.len(), |e| i + 1 + e + 1);
                continue;
            }
            b'(' => depth += 1,
            b')' => depth -= 1,
            b';' | b'{' | b'}' if depth <= 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// `css` (a style sheet, or a `style` attribute's declarations) with its declarations
/// brought in line; each change described once in `changes`.
fn rewrite_css(css: &str, palette: &Palette, changes: &mut Vec<String>) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    loop {
        let Some(at) = next_boundary(rest) else {
            out.push_str(&declaration(rest, palette, changes));
            break;
        };
        let boundary = rest.as_bytes()[at];
        // Before a `{` is a selector or an at-rule; before a `;` or `}` a declaration.
        if boundary == b'{' {
            out.push_str(&rest[..at]);
        } else {
            out.push_str(&declaration(&rest[..at], palette, changes));
        }
        out.push(boundary as char);
        rest = &rest[at + 1..];
    }
    out
}

fn note(changes: &mut Vec<String>, change: String) {
    if !changes.contains(&change) {
        changes.push(change);
    }
}

/// One `property: value` with its value brought in line (anything else as it is).
fn declaration(text: &str, palette: &Palette, changes: &mut Vec<String>) -> String {
    let Some(colon) = text.find(':') else {
        return text.to_string();
    };
    let property = text[..colon].trim().to_ascii_lowercase();
    if property.is_empty()
        || !property
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return text.to_string();
    }
    let raw = &text[colon + 1..];
    // Keep the spacing around the value and an `!important`.
    let lead = raw.len() - raw.trim_start().len();
    let trail = raw.trim_end().len();
    let mut value = raw[lead..trail].to_string();
    let mut important = "";
    if let Some(at) = value.to_ascii_lowercase().rfind("!important") {
        if value[at..].trim().eq_ignore_ascii_case("!important") {
            important = " !important";
            value = value[..at].trim_end().to_string();
        }
    }
    let new = value_in_line(&property, &value, palette, changes);
    if new == value {
        return text.to_string();
    }
    format!(
        "{}{}{new}{important}{}",
        &text[..colon + 1],
        &raw[..lead],
        &raw[trail..]
    )
}

/// What `property`'s `value` becomes.
fn value_in_line(
    property: &str,
    value: &str,
    palette: &Palette,
    changes: &mut Vec<String>,
) -> String {
    let lower = value.to_ascii_lowercase();
    let from_vars = lower.contains("var(");
    let short = |v: &str| v.chars().take(40).collect::<String>();
    match property {
        "font-family" if !from_vars && lower != "inherit" => {
            note(changes, format!("font {} → the look's", short(value)));
            "var(--look-font)".into()
        }
        "border-radius" if !from_vars => {
            let fine =
                lower == "0" || lower == "50%" || lower.starts_with("99") || lower == "inherit";
            if fine {
                return value.to_string();
            }
            note(changes, format!("corners {} → the look's", short(value)));
            let px = lower
                .split_whitespace()
                .next()
                .and_then(|v| v.strip_suffix("px"))
                .and_then(|v| v.parse::<f64>().ok());
            if px.is_some_and(|px| px <= 8.0) {
                "var(--look-radius-small)".into()
            } else {
                "var(--look-radius)".into()
            }
        }
        "transition" | "animation" if !from_vars && lower != "none" => {
            let timed = timing(value);
            if timed != value {
                note(
                    changes,
                    format!("{property} {} → the look's timing", short(value)),
                );
            }
            replace_colours(&timed, palette, changes).0
        }
        "box-shadow" | "text-shadow" => {
            let (_, found) = replace_colours(value, palette, &mut Vec::new());
            if found == 0 {
                return value.to_string();
            }
            note(changes, format!("{property} → the look's shadow"));
            "var(--look-shadow)".into()
        }
        _ => replace_colours(value, palette, changes).0,
    }
}

/// A `transition` or `animation` with each entry's duration from the look and its easing the
/// look's (a delay, `linear` and `steps()` stay).
fn timing(value: &str) -> String {
    let eases = ["ease", "ease-in", "ease-out", "ease-in-out"];
    let mut out = Vec::new();
    for entry in split_top(value, ',') {
        let mut timed = false;
        let words: Vec<String> = split_top(&entry, ' ')
            .into_iter()
            .map(|word| {
                let lower = word.trim().to_ascii_lowercase();
                let time = lower
                    .strip_suffix("ms")
                    .or_else(|| lower.strip_suffix('s'))
                    .is_some_and(|n| !n.is_empty() && n.parse::<f64>().is_ok());
                if time && !timed {
                    timed = true;
                    DURATION.to_string()
                } else if eases.contains(&lower.as_str()) || lower.starts_with("cubic-bezier(") {
                    "var(--look-ease)".to_string()
                } else {
                    word
                }
            })
            .collect();
        let lead = entry.len() - entry.trim_start().len();
        let trail = entry.trim_end().len();
        out.push(format!(
            "{}{}{}",
            &entry[..lead],
            words.join(" "),
            &entry[trail..]
        ));
    }
    out.join(",")
}

/// `text` split at `at` outside brackets (pieces keep their spacing, except a space split).
fn split_top(text: &str, at: char) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut depth = 0i32;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if c == at && depth <= 0 {
            out.push(String::new());
        } else {
            out.last_mut().unwrap().push(c);
        }
    }
    if at == ' ' {
        out.retain(|w| !w.is_empty());
    }
    out
}

/// `value` with its colour literals (outside `var(...)`) written as the style's roles where one
/// is close, and how many literals it had.
fn replace_colours(value: &str, palette: &Palette, changes: &mut Vec<String>) -> (String, usize) {
    let mut out = String::with_capacity(value.len());
    let mut found = 0;
    let lower = value.to_ascii_lowercase();
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &lower[i..];
        // A variable (and its fallback) is the style's already.
        if rest.starts_with("var(") {
            let end = closing(value, i + 3);
            out.push_str(&value[i..end]);
            i = end;
            continue;
        }
        let literal_end = if bytes[i] == b'#' {
            let digits = value[i + 1..]
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .count();
            let after = value[i + 1 + digits..].chars().next();
            (matches!(digits, 3 | 4 | 6 | 8)
                && !after.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_'))
            .then_some(i + 1 + digits)
        } else if ["rgb(", "rgba(", "hsl(", "hsla("]
            .iter()
            .any(|f| rest.starts_with(f))
            && !value[..i]
                .chars()
                .last()
                .is_some_and(|c| c.is_alphanumeric() || c == '-')
        {
            Some(closing(value, i + rest.find('(').unwrap_or(0)))
        } else {
            None
        };
        let Some(end) = literal_end else {
            let c = value[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
            continue;
        };
        found += 1;
        let literal = &value[i..end];
        match parse_colour(literal).and_then(|(rgb, alpha)| palette.role_for(rgb, alpha)) {
            Some(role) => {
                note(changes, format!("{literal} → {}", role_name(&role)));
                out.push_str(&role);
            }
            None => out.push_str(literal),
        }
        i = end;
    }
    (out, found)
}

/// How a role reads in a change line.
fn role_name(role: &str) -> String {
    let name = [
        ("--background", "the style's background"),
        ("--text", "the style's text"),
        ("--accent", "the style's accent"),
        ("--look-muted", "the look's muted text"),
        ("--look-line", "the look's line"),
        ("--look-tint", "the look's tint"),
    ]
    .iter()
    .find(|(token, _)| role.contains(&format!("var({token})")))
    .map_or("a style colour", |(_, name)| name);
    if role.starts_with("color-mix") {
        format!("{name}, see-through")
    } else {
        name.to_string()
    }
}

/// The index just past the `)` closing the `(` at `open`.
fn closing(text: &str, open: usize) -> usize {
    let mut depth = 0i32;
    for (i, c) in text[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return open + i + 1;
                }
            }
            _ => {}
        }
    }
    text.len()
}

/// `html` with its `<style>` blocks and `style="..."` attributes brought in line.
fn rewrite_html(html: &str, palette: &Palette, changes: &mut Vec<String>) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut at = 0;
    while at < html.len() {
        let next_style = lower[at..].find("<style").map(|i| at + i);
        let next_attr = lower[at..].find("style=\"").map(|i| at + i);
        match (next_style, next_attr) {
            (Some(block), attr) if attr.is_none_or(|a| block <= a) => {
                let Some(open_end) = lower[block..].find('>').map(|i| block + i + 1) else {
                    break;
                };
                let close = lower[open_end..]
                    .find("</style")
                    .map_or(html.len(), |i| open_end + i);
                out.push_str(&html[at..open_end]);
                out.push_str(&rewrite_css(&html[open_end..close], palette, changes));
                at = close;
                // Past `</style`, so the search goes on after it.
                let skip = (close + "</style".len()).min(html.len());
                out.push_str(&html[close..skip]);
                at = at.max(skip);
            }
            (_, Some(attr)) => {
                // Only an attribute (after a space), not `data-style="..."`.
                let start = attr + "style=\"".len();
                let is_attribute = html[..attr].chars().last().is_some_and(char::is_whitespace);
                let Some(end) = html[start..].find('"').map(|i| start + i) else {
                    break;
                };
                out.push_str(&html[at..start]);
                if is_attribute {
                    out.push_str(&rewrite_css(&html[start..end], palette, changes));
                } else {
                    out.push_str(&html[start..end]);
                }
                at = end;
                out.push('"');
                at += 1;
            }
            _ => break,
        }
    }
    if at < html.len() {
        out.push_str(&html[at..]);
    }
    out
}

/// The style graphic `dir` takes (its own, else the project's), as its three colours.
fn kit_colours(dir: &Path, project_style: &str) -> Option<(String, String, String)> {
    let own = super::read_source(dir).ok()?.style;
    let style = own
        .as_deref()
        .and_then(crate::style::store::cached)
        .unwrap_or_else(|| crate::style::store::resolve(project_style));
    let c = style.brand.colors;
    Some((c.background, c.text, c.accent))
}

/// The files of graphic folder `dir` brought in line, as (relative path, new text), and what
/// changes and what stays. Only files that change are listed.
fn rewritten(
    dir: &Path,
    project_style: &str,
) -> Result<(Vec<(String, String)>, Vec<String>, Vec<String>), String> {
    let kit = kit_colours(dir, project_style);
    let palette = Palette::new(
        kit.as_ref()
            .map(|(b, t, a)| (b.as_str(), t.as_str(), a.as_str())),
    );
    let mut files = Vec::new();
    super::render::list_files(dir, dir, &mut files)?;
    let mut changes = Vec::new();
    let mut out = Vec::new();
    let mut entry = None;
    for (relative, path) in files {
        let lower = relative.to_ascii_lowercase();
        let html = lower.ends_with(".html") || lower.ends_with(".htm");
        if !html && !lower.ends_with(".css") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let new = if html {
            rewrite_html(&text, &palette, &mut changes)
        } else {
            rewrite_css(&text, &palette, &mut changes)
        };
        if relative == super::ENTRY {
            entry = Some(new.clone());
        }
        if new != text {
            out.push((relative, new));
        }
    }
    // What `graphic_check` would still see.
    let mut left = Vec::new();
    if let Some(html) = entry {
        let (colours, others) = gate::hard_coded(&html);
        left.extend(colours);
        left.extend(others);
    }
    left.truncate(6);
    Ok((out, changes, left))
}

/// What bringing each of `document`'s graphics in line would change; graphics already in
/// line are left out.
pub fn plan(root: &Path, document: &EditDocument) -> Vec<Plan> {
    document
        .assets
        .iter()
        .filter(|a| a.is_graphic())
        .filter_map(|asset| {
            let dir = super::folder(root, &asset.id);
            let source = super::read_source(&dir).ok()?;
            let (_, mut changes, left) = rewritten(&dir, &document.style).ok()?;
            if !source.look.is_empty() {
                changes.push(format!(
                    "its own look ({}) → the video's",
                    source.look.keys().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
            if changes.is_empty() {
                return None;
            }
            Some(Plan {
                graphic: asset.id.clone(),
                name: source.name,
                stamp: gate::source_hash(&dir).ok()?,
                changes,
                left,
            })
        })
        .collect()
}

/// What applying a plan made: the edit with the graphics swapped, and the new folders (to
/// remove when the edit is not kept).
pub struct Applied {
    pub document: EditDocument,
    pub made: Vec<PathBuf>,
}

impl Applied {
    pub fn discard(&self) {
        for dir in &self.made {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// Brings `plans`' graphics in line on a copy of `document`: each becomes a new graphic with
/// the rewritten files and no look of its own, and its clips (the video's and every short's)
/// play it instead. All or nothing; a graphic changed since its plan is refused.
pub fn apply(root: &Path, document: &EditDocument, plans: &[Plan]) -> Result<Applied, String> {
    let mut applied = Applied {
        document: document.clone(),
        made: Vec::new(),
    };
    let result = (|| {
        for plan in plans {
            let dir = super::folder(root, &plan.graphic);
            if gate::source_hash(&dir)? != plan.stamp {
                return Err(format!(
                    "\"{}\" changed since this was worked out: look again",
                    plan.name
                ));
            }
            let at = applied
                .document
                .assets
                .iter()
                .position(|a| a.id == plan.graphic)
                .ok_or_else(|| format!("\"{}\" is no longer in the project", plan.name))?;
            let (files, _, _) = rewritten(&dir, &document.style)?;
            let id = crate::sequence::sources::new_asset_id(false);
            let to = super::folder(root, &id);
            applied.made.push(to.clone());
            super::render::copy_dir(&dir, &to)?;
            for (relative, text) in files {
                let path = to.join(&relative);
                std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            let mut source = super::read_source(&to)?;
            source.look.clear();
            super::write_source(&to, &source)?;
            applied.document.assets[at] = super::asset_for(&id, &to)?;
            repoint(&mut applied.document, &plan.graphic, &id);
        }
        Ok(())
    })();
    match result {
        Ok(()) => Ok(applied),
        Err(error) => {
            applied.discard();
            Err(error)
        }
    }
}

/// Clips of asset `from` play asset `to`, on the video's timeline and every short's.
fn repoint(document: &mut EditDocument, from: &str, to: &str) {
    let sequences = std::iter::once(&mut document.sequence).chain(
        document
            .shorts
            .iter_mut()
            .filter_map(|s| s.edit.as_mut().map(|e| &mut e.sequence)),
    );
    for sequence in sequences {
        for clip in sequence.tracks.iter_mut().flat_map(|t| t.clips.iter_mut()) {
            if clip.asset == from {
                clip.asset = to.to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> Palette {
        Palette::new(None)
    }

    #[test]
    fn colours_close_to_a_role_take_it_and_others_stay() {
        let mut changes = Vec::new();
        let css = ".card { background: #fdfdfd; color: #222; border: 1px solid #ebebeb; } \
                   .tag { color: rgb(0, 113, 227); background: rgba(29, 29, 31, 0.4); } \
                   .false { color: #E5484D; } .ok { color: var(--accent, #123456); }";
        let out = rewrite_css(css, &palette(), &mut changes);
        assert!(out.contains("background: var(--background);"), "{out}");
        assert!(out.contains("color: var(--text);"), "{out}");
        assert!(out.contains("1px solid var(--look-line)"), "{out}");
        assert!(out.contains("color: var(--accent);"), "{out}");
        assert!(
            out.contains("color-mix(in srgb, var(--text) 40%, transparent)"),
            "{out}"
        );
        // A red stamp is meant; a fallback inside var() is the style's already.
        assert!(out.contains("color: #E5484D;"), "{out}");
        assert!(out.contains("var(--accent, #123456)"), "{out}");
        assert!(
            changes
                .iter()
                .any(|c| c == "#fdfdfd → the style's background"),
            "{changes:?}"
        );
    }

    #[test]
    fn fonts_corners_timing_and_shadows_take_the_look() {
        let mut changes = Vec::new();
        let css = "@media (orientation: portrait) { .a { font-family: Inter, sans-serif; \
                   border-radius: 16px; transition: opacity 0.4s ease-out 0.1s, transform 300ms \
                   cubic-bezier(0.2, 1, 0.3, 1); box-shadow: 0 4px 12px rgba(0,0,0,.1) !important; } \
                   .b { border-radius: 6px; } .c { border-radius: 50%; animation: none; } \
                   a:hover { color: inherit } }";
        let out = rewrite_css(css, &palette(), &mut changes);
        assert!(out.contains("font-family: var(--look-font);"), "{out}");
        assert!(out.contains(".a { font-family"), "selectors stay: {out}");
        assert!(out.contains("border-radius: var(--look-radius);"), "{out}");
        assert!(
            out.contains("border-radius: var(--look-radius-small);"),
            "{out}"
        );
        assert!(out.contains("border-radius: 50%;"), "{out}");
        assert!(
            out.contains(
                "transition: opacity calc(var(--look-duration) * 1ms) var(--look-ease) 0.1s, \
                 transform calc(var(--look-duration) * 1ms) var(--look-ease);"
            ),
            "{out}"
        );
        assert!(
            out.contains("box-shadow: var(--look-shadow) !important;"),
            "{out}"
        );
        assert!(out.contains("animation: none;"), "{out}");
        // What is left passes the gate's look check.
        let html = format!("<html><head><style>{out}</style></head></html>");
        assert!(
            gate::hard_coded(&html).1.is_empty(),
            "{:?}",
            gate::hard_coded(&html)
        );
    }

    #[test]
    fn html_styles_and_attributes_are_rewritten_and_scripts_stay() {
        let mut changes = Vec::new();
        let html = "<html><head><style>.t { color: #1d1d1f; }</style></head>\
                    <body><div style=\"background: #FFFFFF; border-radius: 20px\" \
                    data-style=\"color: #fff\"></div>\
                    <script>el.style.color = '#ff0000';</script></body></html>";
        let out = rewrite_html(html, &palette(), &mut changes);
        assert!(out.contains(".t { color: var(--text); }"), "{out}");
        assert!(
            out.contains(
                "style=\"background: var(--background); border-radius: var(--look-radius)\""
            ),
            "{out}"
        );
        assert!(out.contains("data-style=\"color: #fff\""), "{out}");
        assert!(out.contains("'#ff0000'"), "{out}");
        assert!(out.ends_with("</script></body></html>"), "{out}");
    }

    #[test]
    fn colours_parse() {
        assert_eq!(parse_colour("#fff"), Some(([255.0; 3], 1.0)));
        assert_eq!(parse_colour("#0071E3").unwrap().0, [0.0, 113.0, 227.0]);
        let (rgb, alpha) = parse_colour("rgb(255 0 0 / 50%)").unwrap();
        assert_eq!((rgb, alpha), ([255.0, 0.0, 0.0], 0.5));
        let (rgb, _) = parse_colour("hsl(120deg, 100%, 50%)").unwrap();
        assert_eq!(rgb.map(f64::round), [0.0, 255.0, 0.0]);
        assert_eq!(parse_colour("#12345"), None);
    }

    #[test]
    fn a_graphic_in_line_is_a_copy_its_clips_play_and_undo_has_the_old_one() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut document = EditDocument::default();
        let asset = super::super::create(root, "blank", Some("Card"), &Default::default()).unwrap();
        let folder = super::super::folder(root, &asset.id);
        let html_path = folder.join(super::super::ENTRY);
        let html = std::fs::read_to_string(&html_path).unwrap().replace(
            "</style>",
            ".mine { color: #222222; border-radius: 14px; }</style>",
        );
        std::fs::write(&html_path, &html).unwrap();
        let mut source = super::super::read_source(&folder).unwrap();
        source.look.insert("radius".into(), "0".into());
        super::super::write_source(&folder, &source).unwrap();
        let id = asset.id.clone();
        document.assets.push(asset);
        let mut track = crate::sequence::Track::new("t9".into(), crate::sequence::TrackKind::Video);
        track.clips.push(crate::sequence::Clip {
            id: "g1".into(),
            asset: id.clone(),
            stream: "picture".into(),
            start_us: 0,
            in_us: 0,
            duration_us: 3_000_000,
            link: None,
            fit: Default::default(),
            speed: 1.0,
        });
        document.sequence.tracks.push(track);

        let plans = plan(root, &document);
        assert_eq!(plans.len(), 1, "{plans:?}");
        assert!(plans[0]
            .changes
            .iter()
            .any(|c| c.starts_with("its own look (radius)")));
        let applied = apply(root, &document, &plans).unwrap();
        let new_id = &applied.document.assets[0].id;
        assert_ne!(new_id, &id);
        assert_eq!(applied.document.sequence.tracks[0].clips[0].asset, *new_id);
        let new_folder = super::super::folder(root, new_id);
        let new_html = std::fs::read_to_string(new_folder.join(super::super::ENTRY)).unwrap();
        assert!(
            new_html.contains(".mine { color: var(--text); border-radius: var(--look-radius); }")
        );
        assert!(super::super::read_source(&new_folder)
            .unwrap()
            .look
            .is_empty());
        // The old graphic is as it was, for undo; in line now, it plans nothing.
        assert_eq!(std::fs::read_to_string(&html_path).unwrap(), html);
        assert!(plan(root, &applied.document).is_empty());
        // A graphic changed after its plan is refused, and nothing is left behind.
        std::fs::write(&html_path, html.replace("#222222", "#333333")).unwrap();
        let before = std::fs::read_dir(root.join(super::super::GRAPHICS_DIR))
            .unwrap()
            .count();
        assert!(apply(root, &document, &plans).is_err());
        let after = std::fs::read_dir(root.join(super::super::GRAPHICS_DIR))
            .unwrap()
            .count();
        assert_eq!(before, after);
    }
}
