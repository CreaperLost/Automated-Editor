//! The sound effects library (SND-3): a starter set of hand-picked CC0 sounds built into the
//! app (whooshes, risers, hits, clicks, pops), and the user's own sounds beside them, each
//! tagged and carrying its licence. Placing one imports it into the project as an effects
//! sound (with its licence) and puts it on an effects track at a moment, never pushing
//! anything along.
//!
//! Built-in sounds are written once into the app's folder (`sounds/built-in`), since media
//! is used in place; the user's own are copied into `sounds/mine` with `mine.json`.
use crate::own_library;
use crate::project::revision::EditDocument;
use crate::sequence::{Asset, Licence, LicenceKind, Role};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// The name of the track sounds go on when no effects track is free.
pub const TRACK_NAME: &str = "Sound effects";
/// A sound effect's length at most: longer is music, not an effect.
const MAX_SOUND_US: u64 = 30_000_000;

struct Builtin {
    id: &'static str,
    name: &'static str,
    tags: &'static [&'static str],
    duration_ms: u32,
    source: &'static str,
    bytes: &'static [u8],
}

macro_rules! builtin {
    ($id:literal, $name:literal, [$($tag:literal),*], $ms:literal, $source:literal) => {
        Builtin {
            id: $id,
            name: $name,
            tags: &[$($tag),*],
            duration_ms: $ms,
            source: $source,
            bytes: include_bytes!(concat!("../assets/sounds/", $id, ".flac")),
        }
    };
}

/// The starter set; where each came from is in `assets/sounds/LICENSES.md`.
static BUILTIN: [Builtin; 17] = [
    builtin!(
        "whoosh-soft",
        "Soft whoosh",
        ["whoosh", "transition", "soft", "air"],
        2250,
        "Freesound 423799 (ch_ase)"
    ),
    builtin!(
        "whoosh-short",
        "Short whoosh",
        ["whoosh", "transition", "zoom"],
        1300,
        "Freesound 449996 (DJT4NN3R)"
    ),
    builtin!(
        "whoosh-fast",
        "Fast whoosh",
        ["whoosh", "transition", "fast", "swipe"],
        450,
        "Freesound 733905 (velcronator)"
    ),
    builtin!(
        "whoosh-whip",
        "Whip whoosh",
        ["whoosh", "transition", "whip", "zoom", "fast"],
        380,
        "Freesound 486234 (BennettFilmTeacher)"
    ),
    builtin!(
        "riser-short",
        "Short riser",
        ["riser", "build", "reveal", "tension"],
        2697,
        "Freesound 685256 (syntheffects)"
    ),
    builtin!(
        "riser-hit",
        "Riser into a hit",
        ["riser", "hit", "reveal", "build", "impact"],
        4100,
        "Freesound 511871 (AudioPapkin)"
    ),
    builtin!(
        "hit-punch",
        "Punch hit",
        ["hit", "impact", "punch", "emphasis"],
        649,
        "Kenney Impact Sounds"
    ),
    builtin!(
        "hit-soft",
        "Soft hit",
        ["hit", "impact", "soft", "thud"],
        527,
        "Kenney Impact Sounds"
    ),
    builtin!(
        "hit-boom",
        "Low boom",
        ["hit", "impact", "boom", "bass", "reveal"],
        1000,
        "Kenney Sci-fi Sounds"
    ),
    builtin!(
        "click-mouse",
        "Mouse click",
        ["click", "ui", "mouse"],
        56,
        "Kenney UI Audio"
    ),
    builtin!(
        "click-soft",
        "Soft click",
        ["click", "ui", "soft"],
        97,
        "Kenney Interface Sounds"
    ),
    builtin!(
        "tick",
        "Tick",
        ["click", "tick", "ui", "count"],
        55,
        "Kenney Interface Sounds"
    ),
    builtin!(
        "pop",
        "Pop",
        ["pop", "appear", "ui"],
        188,
        "Kenney Interface Sounds"
    ),
    builtin!(
        "pop-pluck",
        "Pluck pop",
        ["pop", "pluck", "appear", "ui"],
        100,
        "Kenney Interface Sounds"
    ),
    builtin!(
        "pop-up",
        "Pop up",
        ["pop", "appear", "open", "ui"],
        258,
        "Kenney Interface Sounds"
    ),
    builtin!(
        "ding",
        "Ding",
        ["ding", "success", "done", "notification"],
        539,
        "Kenney Interface Sounds"
    ),
    builtin!(
        "toggle",
        "Toggle",
        ["click", "switch", "toggle", "ui"],
        136,
        "Kenney Interface Sounds"
    ),
];

/// One sound of the library, as the app and agents see it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub duration_us: u64,
    /// The file on disk, played in place.
    pub path: String,
    pub built_in: bool,
    pub licence: Licence,
}

/// The library's folder: the app's, or a test run's own.
pub fn dir() -> PathBuf {
    crate::style::store::config().join("sounds")
}

/// Writes the built-in sounds that are missing or changed, and answers the folder.
fn built_in_dir(root: &Path) -> Result<PathBuf, String> {
    let dir = root.join("built-in");
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    for sound in &BUILTIN {
        let path = dir.join(format!("{}.flac", sound.id));
        let current = fs::metadata(&path).map(|m| m.len()).ok();
        if current != Some(sound.bytes.len() as u64) {
            fs::write(&path, sound.bytes)
                .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        }
    }
    Ok(dir)
}

/// Every sound: the built-in set first, then the user's own, in the order they were added.
pub fn list(root: &Path) -> Result<Vec<Sound>, String> {
    let built_in = built_in_dir(root)?;
    let mut sounds: Vec<Sound> = BUILTIN
        .iter()
        .map(|b| Sound {
            id: b.id.into(),
            name: b.name.into(),
            tags: b.tags.iter().map(|t| t.to_string()).collect(),
            duration_us: b.duration_ms as u64 * 1_000,
            path: built_in
                .join(format!("{}.flac", b.id))
                .to_string_lossy()
                .into_owned(),
            built_in: true,
            licence: Licence {
                kind: LicenceKind::Cc0,
                source: Some(b.source.into()),
            },
        })
        .collect();
    let mine_dir = root.join("mine");
    sounds.extend(own_library::load(root).into_iter().map(|m| Sound {
        path: mine_dir.join(&m.file).to_string_lossy().into_owned(),
        id: m.id,
        name: m.name,
        tags: m.tags,
        duration_us: m.duration_us,
        built_in: false,
        licence: m.licence,
    }));
    Ok(sounds)
}

/// The sounds matching `query` (every word in its name, id or tags), all of them when empty.
pub fn search(sounds: Vec<Sound>, query: &str) -> Vec<Sound> {
    own_library::search(sounds, query, |s| {
        format!("{} {} {}", s.id, s.name, s.tags.join(" "))
    })
}

pub fn find(root: &Path, id: &str) -> Result<Sound, String> {
    list(root)?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("No sound {id} in the library"))
}

/// Copies a sound file of the user's into the library with its tags and licence.
pub fn add(
    root: &Path,
    file: &Path,
    name: Option<String>,
    tags: Vec<String>,
    licence: Licence,
) -> Result<Sound, String> {
    if crate::media_bin::kind_for(file) != Some(crate::sequence::AssetKind::Audio) {
        return Err(format!("{} is not a sound file", file.display()));
    }
    let duration_us = crate::media::ffmpeg::duration_us(file)?;
    if duration_us == 0 || duration_us > MAX_SOUND_US {
        return Err("A sound effect runs between a moment and 30 seconds; longer is music".into());
    }
    let mut mine = own_library::load(root);
    let id = own_library::copy_in(
        root,
        &mut mine,
        file,
        "sound",
        "wav",
        name,
        tags,
        duration_us,
        licence,
        |id| BUILTIN.iter().any(|b| b.id == id),
    )?;
    own_library::save(root, &mine)?;
    find(root, &id)
}

/// Changes a sound of the user's own: its name, tags or licence.
pub fn update(
    root: &Path,
    id: &str,
    name: Option<String>,
    tags: Option<Vec<String>>,
    licence: Option<Licence>,
) -> Result<Sound, String> {
    let mut mine = own_library::load(root);
    let Some(entry) = mine.iter_mut().find(|m| m.id == id) else {
        return Err(if BUILTIN.iter().any(|b| b.id == id) {
            "Built-in sounds can't be changed".into()
        } else {
            format!("No sound {id} of your own")
        });
    };
    own_library::change(entry, name, tags, licence);
    own_library::save(root, &mine)?;
    find(root, id)
}

/// Takes a sound of the user's out of the library (projects that used it keep their media).
pub fn remove(root: &Path, id: &str) -> Result<(), String> {
    own_library::remove(root, id, "sound")
}

/// The project's media for `sound`: the asset already holding its file, or a new one imported
/// from it (its sounds as effects, with the sound's licence). The second value says whether
/// it is new.
pub fn asset_for(
    document: &EditDocument,
    project_root: &Path,
    sound: &Sound,
) -> Result<(Asset, bool), String> {
    own_library::asset_for(
        document,
        project_root,
        Path::new(&sound.path),
        &sound.name,
        &sound.licence,
        Role::Effects,
    )
}

/// The document's sequence with an unlocked audio track for `role` free over `[from, to)`: one
/// with that role, named `name`, or holding only media of that role; else a new one named
/// `name`. Answers the sequence (with the new track, if one was added) and the track's id.
pub(crate) fn free_track<'t>(
    document: &EditDocument,
    role: Role,
    name: &str,
    from: u64,
    to: u64,
    taken: impl IntoIterator<Item = &'t str> + Clone,
) -> Result<(crate::sequence::Sequence, String), String> {
    use crate::sequence::edit::{apply_reserving, SequenceEdit};
    use crate::sequence::TrackKind;
    let only = |id: &str| {
        document
            .asset(id)
            .is_some_and(|a| a.streams.iter().all(|s| s.role == role))
    };
    let free = document
        .sequence
        .audio_tracks()
        .find(|t| {
            !t.locked
                && (t.role == Some(role)
                    || t.name == name
                    || (!t.clips.is_empty() && t.clips.iter().all(|c| only(&c.asset))))
                && t.is_free(from, to)
        })
        .map(|t| t.id.clone());
    let sequence = document.sequence.clone();
    if let Some(id) = free {
        return Ok((sequence, id));
    }
    let (added, _) = apply_reserving(
        &sequence,
        &document.assets,
        &SequenceEdit::AddTrack {
            track_kind: TrackKind::Audio,
        },
        taken.clone(),
    )?;
    let id = added
        .audio_tracks()
        .find(|t| sequence.track(&t.id).is_none())
        .map(|t| t.id.clone())
        .ok_or("Could not add a track")?;
    let (named, _) = apply_reserving(
        &added,
        &document.assets,
        &SequenceEdit::SetTrack {
            track_id: id.clone(),
            name: name.into(),
            hidden: false,
            muted: false,
            locked: false,
            role: Some(role),
        },
        taken,
    )?;
    Ok((named, id))
}

/// `document` with `asset` (added when new) placed whole at `at_us` on an effects track free
/// there, or a new one named [`TRACK_NAME`] (an id not in `taken`). Answers the document and
/// the clip's id.
pub fn place(
    document: &EditDocument,
    asset: Asset,
    at_us: u64,
    taken: &[String],
) -> Result<(EditDocument, String), String> {
    use crate::sequence::edit::{apply_reserving, SequenceEdit};
    use crate::sequence::SourceRange;
    let mut next = document.clone();
    let asset_id = asset.id.clone();
    let length = asset.duration_us.max(1);
    if next.asset(&asset_id).is_none() {
        next.assets.push(asset);
    }
    let at = at_us.min(next.duration_us());
    let (sequence, track_id) = free_track(
        &next,
        Role::Effects,
        TRACK_NAME,
        at,
        at + length,
        taken.iter().map(String::as_str),
    )?;
    let before: Vec<String> = sequence.clips().map(|(_, c)| c.id.clone()).collect();
    let (placed, _) = apply_reserving(
        &sequence,
        &next.assets,
        &SequenceEdit::PlaceAsset {
            asset_id: asset_id.clone(),
            at_us: at,
            track_id: Some(track_id),
            streams: Vec::new(),
            range: Some(SourceRange {
                start_us: 0,
                end_us: length,
            }),
        },
        taken.iter().map(String::as_str),
    )?;
    if placed.duration_us() > sequence.duration_us().max(at + length) {
        return Err("Placing the sound would have moved the video".into());
    }
    let clip_id = placed
        .clips()
        .find(|(_, c)| c.asset == asset_id && !before.contains(&c.id))
        .map(|(_, c)| c.id.clone())
        .ok_or("The sound was not placed")?;
    next.sequence = placed;
    Ok((next, clip_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_starter_set_is_written_once_and_listed_with_its_licence() {
        let dir = tempfile::tempdir().unwrap();
        let sounds = list(dir.path()).unwrap();
        assert_eq!(sounds.len(), BUILTIN.len());
        assert!((15..=20).contains(&sounds.len()));
        for sound in &sounds {
            assert!(Path::new(&sound.path).is_file(), "{}", sound.path);
            assert_eq!(sound.licence.kind, LicenceKind::Cc0);
            assert!(sound.licence.kind.needs_no_credit());
            assert!(!sound.tags.is_empty());
        }
        // Every kind the plan named is there.
        for tag in ["whoosh", "riser", "hit", "click", "pop"] {
            assert!(!search(sounds.clone(), tag).is_empty(), "no {tag}");
        }
        assert_eq!(search(sounds.clone(), "riser hit").len(), 1);
        assert_eq!(search(sounds, "").len(), BUILTIN.len());
    }

    #[test]
    fn built_in_lengths_match_their_files() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        for sound in list(dir.path()).unwrap() {
            let real = crate::media::ffmpeg::duration_us(Path::new(&sound.path)).unwrap();
            let diff = real.abs_diff(sound.duration_us);
            assert!(
                diff < 30_000,
                "{}: {real} vs {}",
                sound.id,
                sound.duration_us
            );
        }
    }

    #[test]
    fn own_sounds_are_copied_tagged_changed_and_removed() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let source = list(dir.path()).unwrap()[0].path.clone();
        let file = dir.path().join("My Swoosh!.flac");
        fs::copy(&source, &file).unwrap();
        let licence = Licence {
            kind: LicenceKind::CreditRequired,
            source: Some("a pack".into()),
        };
        let added = add(
            dir.path(),
            &file,
            None,
            vec!["Whoosh".into(), "whoosh".into(), " ".into()],
            licence,
        )
        .unwrap();
        assert_eq!(added.id, "my-my-swoosh");
        assert_eq!(added.name, "My Swoosh!");
        assert_eq!(added.tags, ["whoosh"]);
        assert!(!added.built_in);
        let again = add(dir.path(), &file, None, vec![], added.licence.clone()).unwrap();
        assert_eq!(again.id, "my-my-swoosh-2");
        let changed = update(
            dir.path(),
            &added.id,
            Some("Swoosh".into()),
            Some(vec!["air".into()]),
            None,
        )
        .unwrap();
        assert_eq!(
            (changed.name.as_str(), changed.tags.clone()),
            ("Swoosh", vec!["air".to_string()])
        );
        assert!(update(dir.path(), "pop", None, None, None).is_err());
        remove(dir.path(), &added.id).unwrap();
        assert!(find(dir.path(), &added.id).is_err());
        assert!(!Path::new(&added.path).exists());
        assert!(add(
            dir.path(),
            &dir.path().join("x.txt"),
            None,
            vec![],
            again.licence
        )
        .is_err());
    }
}
