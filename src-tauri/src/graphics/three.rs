//! three.js for 3D graphics, offline, the way [`super::gsap`] does GSAP. A graphic maps the
//! module names in an import map (`"three": "./three.module.js"`, `"three/addons/":
//! "./three-addons/"`), draws its scene on HyperFrames' `hf-seek` event, and the render puts the
//! files it uses beside it: three.js itself and each addon it imports (with the addon files
//! those import), from the pinned npm package fetched once. A CDN link to three.js becomes the
//! local name when the file is written.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// three.js itself, as a graphic's import map names it; it imports `three.core.js` beside it.
pub const MODULE: &str = "three.module.js";
const CORE: &str = "three.core.js";
/// The folder the addons (`examples/jsm` in the package) are staged in.
pub const ADDONS: &str = "three-addons/";
const CDN_HOSTS: [&str; 3] = ["cdn.jsdelivr.net/", "unpkg.com/", "esm.sh/"];

/// `html` with every CDN link to three.js replaced by its local name, or `None` when it has none.
pub fn localize(html: &str) -> Option<String> {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let mut changed = false;
    while let Some(at) = rest.find("http") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail
            .find(|c: char| matches!(c, '"' | '\'' | '>' | '`') || c.is_whitespace())
            .unwrap_or(tail.len());
        let url = &tail[..end];
        match local_name(url) {
            Some(local) => {
                out.push_str(&local);
                changed = true;
            }
            None => out.push_str(url),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    changed.then_some(out)
}

/// A CDN link to three.js: a render uses the local copy, so it doesn't count as the network.
pub fn is_cdn_link(reference: &str) -> bool {
    let url = reference
        .trim_start_matches(|c: char| c != 'h')
        .trim_end_matches(['"', '\'', ',', '}']);
    local_name(url).is_some()
}

/// The local name for a CDN URL into the three package: `./three.module.js` for the build,
/// `./three-addons/<path>` for an addon (or the folder, as an import map's prefix).
fn local_name(url: &str) -> Option<String> {
    let path = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = CDN_HOSTS.iter().find(|host| path.starts_with(**host))?;
    let path = path[host.len()..].trim_start_matches("npm/");
    let after = path.strip_prefix("three")?;
    // `three@0.187.0/...` or `three/...`, never another package (`three-stdlib`).
    let after = match after.strip_prefix('@') {
        Some(versioned) => &versioned[versioned.find('/')?..],
        None if after.starts_with('/') => after,
        None => return None,
    };
    let after = after.split(['?', '#']).next()?;
    if after.starts_with("/build/three.module") {
        return Some(format!("./{MODULE}"));
    }
    let addon = after.strip_prefix("/examples/jsm/")?;
    Some(format!("./{ADDONS}{addon}"))
}

/// Whether a page or script uses three.js through the local names.
fn uses_three(text: &str) -> bool {
    text.contains(MODULE) || text.contains(ADDONS)
}

/// The addon files `text` imports (`three/addons/controls/X.js` or `./three-addons/...`), as
/// paths inside the package's `examples/jsm`.
fn addon_imports(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for marker in ["three/addons/", ADDONS] {
        let mut from = 0;
        while let Some(at) = text[from..].find(marker) {
            let start = from + at + marker.len();
            let path: String = text[start..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'))
                .collect();
            from = start;
            if let Some(clean) = clean_addon(&path) {
                found.insert(clean);
            }
        }
    }
    found
}

/// An addon path that stays inside `examples/jsm` and names a script, or `None`.
fn clean_addon(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    let joined = parts.join("/");
    joined.ends_with(".js").then_some(joined)
}

/// The relative imports (`'./x.js'`, `"../y/z.js"`) in an addon file at `path` (inside
/// `examples/jsm`), resolved to paths inside it.
fn relative_imports(source: &str, path: &str) -> Vec<String> {
    let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
    let mut found = Vec::new();
    for quote in ['\'', '"'] {
        for spec in source.split(quote).skip(1).step_by(2) {
            if (spec.starts_with("./") || spec.starts_with("../")) && spec.ends_with(".js") {
                if let Some(clean) = clean_addon(&format!("{dir}/{spec}")) {
                    found.push(clean);
                }
            }
        }
    }
    found
}

/// The page and the scripts beside it, read as text.
fn stage_texts(stage: &Path) -> Vec<String> {
    let mut texts = Vec::new();
    let Ok(entries) = std::fs::read_dir(stage) else {
        return texts;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if matches!(ext.as_str(), "html" | "js" | "mjs")
            && path.file_name().is_some_and(|n| n != MODULE && n != CORE)
        {
            if let Ok(text) = std::fs::read_to_string(&path) {
                texts.push(text);
            }
        }
    }
    texts
}

/// Puts the three.js files the graphic in `stage` loads beside it, fetching three.js once if
/// needed. A graphic that doesn't use it is left as it is (and nothing is fetched).
pub fn stage(stage: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let entry = stage.join(super::ENTRY);
    if let Ok(html) = std::fs::read_to_string(&entry) {
        if let Some(local) = localize(&html) {
            std::fs::write(&entry, local).map_err(|e| e.to_string())?;
        }
    }
    let texts = stage_texts(stage);
    if !texts.iter().any(|t| uses_three(t)) {
        return Ok(());
    }
    let package = super::runtime::three_package(cancel)?;
    copy_missing(&package.join("build"), stage, MODULE)?;
    copy_missing(&package.join("build"), stage, CORE)?;
    let jsm = package.join("examples").join("jsm");
    let mut todo: Vec<String> = texts.iter().flat_map(|t| addon_imports(t)).collect();
    let mut done = BTreeSet::new();
    while let Some(path) = todo.pop() {
        if !done.insert(path.clone()) {
            continue;
        }
        let from = jsm.join(&path);
        // Not one of three's addons: the page's import fails and the gate says so.
        let Ok(source) = std::fs::read_to_string(&from) else {
            continue;
        };
        let to: PathBuf = stage.join(ADDONS).join(&path);
        if !to.exists() {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&to, &source).map_err(|e| format!("{}: {e}", to.display()))?;
        }
        todo.extend(relative_imports(&source, &path));
    }
    Ok(())
}

fn copy_missing(from_dir: &Path, stage: &Path, name: &str) -> Result<(), String> {
    let to = stage.join(name);
    if to.exists() {
        return Ok(());
    }
    let from = from_dir.join(name);
    std::fs::copy(&from, &to).map_err(|e| format!("{}: {e}", from.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdn_links_to_three_become_local_names() {
        let html = r#"<script type="importmap">{"imports":{
"three":"https://cdn.jsdelivr.net/npm/three@0.187.0/build/three.module.js",
"three/addons/":"https://unpkg.com/three@0.170.0/examples/jsm/"}}</script>
<script type="module">import { X } from "https://cdn.jsdelivr.net/npm/three/examples/jsm/geometries/RoundedBoxGeometry.js";</script>
<script src="https://cdn.jsdelivr.net/npm/three-stdlib@2/index.js"></script>"#;
        let local = localize(html).unwrap();
        assert!(local.contains(r#""three":"./three.module.js""#), "{local}");
        assert!(
            local.contains(r#""three/addons/":"./three-addons/""#),
            "{local}"
        );
        assert!(
            local.contains(r#"from "./three-addons/geometries/RoundedBoxGeometry.js""#),
            "{local}"
        );
        // Another package stays (the gate reports it).
        assert!(local.contains("three-stdlib@2"), "{local}");
        assert!(is_cdn_link(
            "https://cdn.jsdelivr.net/npm/three@0.187.0/build/three.module.js\""
        ));
        assert!(!is_cdn_link("https://example.com/three.module.js"));
        assert_eq!(localize("<p>no links</p>"), None);
    }

    #[test]
    fn addon_imports_are_read_and_kept_inside_the_package() {
        let text = r#"import { RoundedBoxGeometry } from "three/addons/geometries/RoundedBoxGeometry.js";
import { RoomEnvironment } from './three-addons/environments/RoomEnvironment.js';
import x from "three/addons/../../../secret.js";"#;
        let found: Vec<String> = addon_imports(text).into_iter().collect();
        assert_eq!(
            found,
            vec![
                "environments/RoomEnvironment.js",
                "geometries/RoundedBoxGeometry.js"
            ]
        );
        assert_eq!(
            relative_imports(
                "import { a } from '../utils/A.js'; import b from \"./B.js\"; import 'three';",
                "loaders/GLTFLoader.js"
            ),
            vec!["utils/A.js", "loaders/B.js"]
        );
    }

    #[test]
    fn a_graphic_without_three_stages_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let html = "<html><body><script src=\"gsap.min.js\"></script></body></html>";
        std::fs::write(dir.path().join(super::super::ENTRY), html).unwrap();
        stage(dir.path(), &AtomicBool::new(false)).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
