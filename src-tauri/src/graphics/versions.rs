//! Versions of a graphic (GFX-4): a redo is made beside the graphic, not over it. A version is
//! a graphic of its own (its own folder, renders and undo-free source edits), marked in its
//! `graphic.json` as a version of the family's first graphic. One of a family plays: "use"
//! puts another where it plays, on the video's timeline and every short's own, as one undo
//! step, and its render is already there when it was looked at. A small fix (a word, a colour)
//! stays in place; a version is for trying another take of the graphic.
use super::{folder, read_source, write_source};
use crate::project::revision::EditDocument;
use crate::sequence::{Asset, Sequence};
use std::path::Path;

/// The first graphic of `asset_id`'s family: the one its versions are versions of.
pub fn family_root(root: &Path, asset_id: &str) -> String {
    read_source(&folder(root, asset_id))
        .ok()
        .and_then(|s| s.version_of)
        .unwrap_or_else(|| asset_id.to_string())
}

/// Every graphic of `asset_id`'s family in the project, the first one first, then its
/// versions in the order they were made.
pub fn family(document: &EditDocument, root: &Path, asset_id: &str) -> Vec<String> {
    let first = family_root(root, asset_id);
    let mut members: Vec<String> = document
        .assets
        .iter()
        .filter(|a| a.is_graphic() && (a.id == first || family_root(root, &a.id) == first))
        .map(|a| a.id.clone())
        .collect();
    // The first one leads even when its asset was listed later.
    if let Some(at) = members.iter().position(|id| *id == first) {
        let id = members.remove(at);
        members.insert(0, id);
    }
    members
}

/// Whether graphic `asset_id` plays anywhere: the video's timeline or a short's own.
pub fn plays(document: &EditDocument, asset_id: &str) -> bool {
    let on = |sequence: &Sequence| sequence.clips().any(|(_, c)| c.asset == asset_id);
    on(&document.sequence)
        || document
            .shorts
            .iter()
            .filter_map(|s| s.edit.as_ref())
            .any(|e| on(&e.sequence))
}

/// The graphic of `version`'s family that plays now, when `version` itself doesn't.
pub fn playing_instead(document: &EditDocument, root: &Path, version: &str) -> Option<String> {
    if plays(document, version) {
        return None;
    }
    family(document, root, version)
        .into_iter()
        .find(|id| id != version && plays(document, id))
}

/// `document` with every clip of `version`'s family playing `version` instead: the same
/// times, lengths and slots (a short keeps slots by clip).
pub fn swap(document: &EditDocument, root: &Path, version: &str) -> EditDocument {
    let family = family(document, root, version);
    let swap_in = |sequence: &mut Sequence| {
        for track in &mut sequence.tracks {
            for clip in &mut track.clips {
                if clip.asset != version && family.contains(&clip.asset) {
                    clip.asset = version.to_string();
                }
            }
        }
    };
    let mut next = document.clone();
    swap_in(&mut next.sequence);
    for short in &mut next.shorts {
        if let Some(edit) = &mut short.edit {
            swap_in(&mut edit.sequence);
        }
    }
    next
}

/// Copies graphic `from` into a new version of its family (its files, variables, style and
/// look), named like it, with `note` saying what is different. Returns its asset, not yet in
/// the document.
pub fn copy(root: &Path, from: &str, note: Option<&str>) -> Result<Asset, String> {
    let first = family_root(root, from);
    let asset = super::create_from_folder(root, &folder(root, from), None)?;
    let dir = folder(root, &asset.id);
    let made = (|| {
        let mut source = read_source(&dir)?;
        source.version_of = Some(first);
        source.note = note
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(|n| n.chars().take(200).collect());
        write_source(&dir, &source)
    })();
    if let Err(error) = made {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(error);
    }
    Ok(asset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_made_beside_the_graphic_and_used_in_its_place() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let first =
            super::super::create(root, "key-point", Some("Rule"), &serde_json::Map::new()).unwrap();
        let mut document = EditDocument::default();
        let (placed, clip) = super::super::place(&document, first.clone(), 0, 3_000_000).unwrap();
        document = placed;

        let second = copy(root, &first.id, Some("bigger text")).unwrap();
        document.assets.push(second.clone());
        // A version of a version still belongs to the first graphic.
        let third = copy(root, &second.id, None).unwrap();
        document.assets.push(third.clone());
        assert_eq!(family_root(root, &third.id), first.id);
        assert_eq!(
            family(&document, root, &third.id),
            [first.id.clone(), second.id.clone(), third.id.clone()]
        );
        let source = read_source(&folder(root, &second.id)).unwrap();
        assert_eq!(source.note.as_deref(), Some("bigger text"));
        assert_eq!(source.name, "Rule");

        // Only the first plays; the others are shown in its place.
        assert_eq!(
            playing_instead(&document, root, &second.id),
            Some(first.id.clone())
        );
        assert_eq!(playing_instead(&document, root, &first.id), None);

        let used = swap(&document, root, &second.id);
        let (_, swapped) = used.sequence.clips().find(|(_, c)| c.id == clip).unwrap();
        assert_eq!(swapped.asset, second.id, "the same clip, now the version");
        assert!(plays(&used, &second.id) && !plays(&used, &first.id));
        // And back: the first is a member like any other.
        let back = swap(&used, root, &first.id);
        assert_eq!(back.sequence, document.sequence);
    }
}
