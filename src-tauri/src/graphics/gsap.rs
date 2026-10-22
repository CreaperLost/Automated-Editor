//! GSAP for graphics, offline. HyperFrames' own compositions animate with GSAP and link it from a
//! CDN; graphics here may not reach the network, so the same version is fetched once from npm
//! (pinned, checked) beside the renderer and copied next to a graphic that loads it. A graphic
//! names the files it uses (`<script src="gsap.min.js">`, a plugin by its file name); a CDN link
//! to GSAP is turned into that name when the file is written, so a composition written the
//! HyperFrames way passes the gate without a rewrite.
use std::path::Path;
use std::sync::atomic::AtomicBool;

/// CDN hosts GSAP is commonly linked from.
const CDN_HOSTS: [&str; 3] = ["cdn.jsdelivr.net/", "unpkg.com/", "cdnjs.cloudflare.com/"];

/// `html` with every CDN link to a GSAP file replaced by the file's name (`gsap.min.js`,
/// `SplitText.min.js`), or `None` when it has none.
pub fn localize(html: &str) -> Option<String> {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let mut changed = false;
    while let Some(at) = rest.find("http") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail
            .find(|c: char| c == '"' || c == '\'' || c == ' ' || c == '>' || c.is_whitespace())
            .unwrap_or(tail.len());
        let url = &tail[..end];
        match gsap_file(url) {
            Some(file) => {
                out.push_str(file);
                changed = true;
            }
            None => out.push_str(url),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    changed.then_some(out)
}

/// A CDN link to one of GSAP's files: a render uses the local copy instead, so it doesn't count
/// as reaching the network.
pub fn is_cdn_link(reference: &str) -> bool {
    let url = reference
        .trim_start_matches(|c: char| c != 'h')
        .trim_end_matches(['"', '\'']);
    gsap_file(url).is_some()
}

/// The file a CDN URL to GSAP points at (`.../gsap@3.15.0/dist/gsap.min.js` → `gsap.min.js`).
fn gsap_file(url: &str) -> Option<&str> {
    let path = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    if !CDN_HOSTS.iter().any(|host| path.starts_with(host)) || !path.contains("gsap") {
        return None;
    }
    let file = path.rsplit('/').next()?;
    let file = file.split(['?', '#']).next()?;
    (file.ends_with(".js") && file.len() > 3).then_some(file)
}

/// The scripts `html` loads by a bare file name (`gsap.min.js`), the ones GSAP may supply.
fn local_scripts(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut names = Vec::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find("<script") {
        let tag_start = from + at;
        let tag_end = lower[tag_start..]
            .find('>')
            .map_or(lower.len(), |e| tag_start + e);
        let tag = &html[tag_start..tag_end];
        if let Some(src) = attribute(tag, "src") {
            if !src.contains('/') && !src.contains('\\') && src.ends_with(".js") {
                names.push(src.to_string());
            }
        }
        from = tag_end;
    }
    names
}

/// An attribute's value in `tag`, its name matched in any case.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    // ASCII lowercasing keeps every byte where it was, so positions carry over.
    let at = tag.to_ascii_lowercase().find(&format!("{name}="))? + name.len() + 1;
    let quote = tag[at..].chars().next()?;
    if quote == '"' || quote == '\'' {
        let value = &tag[at + 1..];
        value.find(quote).map(|end| &value[..end])
    } else {
        tag[at..].split([' ', '>']).next()
    }
}

/// Puts the GSAP files `stage`'s `index.html` loads beside it, fetching GSAP once if needed.
/// A graphic that loads no such file is left as it is (and nothing is fetched).
pub fn stage(stage: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let entry = stage.join(super::ENTRY);
    let Ok(html) = std::fs::read_to_string(&entry) else {
        return Ok(());
    };
    let html = match localize(&html) {
        Some(local) => {
            std::fs::write(&entry, &local).map_err(|e| e.to_string())?;
            local
        }
        None => html,
    };
    let missing: Vec<String> = local_scripts(&html)
        .into_iter()
        .filter(|name| !stage.join(name).exists())
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let dist = super::runtime::gsap_dist(cancel)?;
    for name in missing {
        let from = dist.join(&name);
        // Not one of GSAP's files: HyperFrames will say the script is missing.
        if from.is_file() {
            std::fs::copy(&from, stage.join(&name))
                .map_err(|e| format!("{}: {e}", from.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdn_links_to_gsap_become_local_names() {
        let html = r#"<script src="https://cdn.jsdelivr.net/npm/gsap@3.15.0/dist/gsap.min.js"></script>
<script src='https://unpkg.com/gsap@3/dist/SplitText.min.js'></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/gsap/3.12.5/CustomEase.min.js?v=1"></script>
<a href="https://example.com/gsap-guide">keep</a>
<script src="https://cdn.jsdelivr.net/npm/three@0.160.0/build/three.min.js"></script>"#;
        let local = localize(html).unwrap();
        assert!(local.contains(r#"<script src="gsap.min.js">"#), "{local}");
        assert!(local.contains("<script src='SplitText.min.js'>"), "{local}");
        assert!(
            local.contains(r#"<script src="CustomEase.min.js">"#),
            "{local}"
        );
        // Only GSAP on a CDN: other links stay (the gate reports them).
        assert!(local.contains("https://example.com/gsap-guide"), "{local}");
        assert!(
            local.contains("three.min.js") && local.contains("https://cdn.jsdelivr.net/npm/three"),
            "{local}"
        );
        assert_eq!(localize("<p>no links</p>"), None);
    }

    #[test]
    fn local_scripts_are_read_by_their_bare_names() {
        let html = r#"<script src="gsap.min.js"></script><SCRIPT SRC="SplitText.min.js"></SCRIPT>
<script src="lib/other.js"></script><script>const inline = 1;</script>"#;
        assert_eq!(local_scripts(html), vec!["gsap.min.js", "SplitText.min.js"]);
    }

    #[test]
    fn a_graphic_without_gsap_stages_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let html = "<html><body><div id=\"root\"></div><script>1</script></body></html>";
        std::fs::write(dir.path().join(super::super::ENTRY), html).unwrap();
        // Nothing to fetch, so this works offline and leaves the file alone.
        stage(dir.path(), &AtomicBool::new(false)).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join(super::super::ENTRY)).unwrap(),
            html
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
