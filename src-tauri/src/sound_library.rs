//! The sound effects library (SND-3): a starter set of hand-picked CC0 sounds built into the
//! app (whooshes, risers, hits, clicks, pops), and the user's own sounds beside them, each
//! tagged and carrying its licence. Placing one imports it into the project as an effects
//! sound (with its licence) and puts it on an effects track at a moment, never pushing
//! anything along.
//!
//! Built-in sounds are written once into the app's folder (`sounds/built-in`), since media
//! is used in place; the user's own are copied into `sounds/mine` with `mine.json`.
use crate::project::revision::EditDocument;
use crate::sequence::{Asset, Licence, LicenceKind, Role};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// The name of the track sounds go on when no effects track is free.
pub const TRACK_NAME: &str = "Sound effects";
const MINE_FILE: &str = "mine.json";
/// Sounds of the user's own, at most.
const MAX_MINE: usize = 500;
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

/// A sound of the user's own, as `mine.json` keeps it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Mine {
    id: String,
    name: String,
    #[serde(default)]
    tags: Vec<String>,
    duration_us: u64,
    /// The copy's file name inside `mine`.
    file: String,
    licence: Licence,
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

fn load_mine(root: &Path) -> Vec<Mine> {
    fs::read(root.join("mine").join(MINE_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_mine(root: &Path, mine: &[Mine]) -> Result<(), String> {
    let dir = root.join("mine");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(mine).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, &bytes).map_err(|e| e.to_string())?;
    temp.persist(dir.join(MINE_FILE))
        .map_err(|e| e.to_string())?;
    Ok(())
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
    sounds.extend(load_mine(root).into_iter().map(|m| Sound {
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
    let words: Vec<String> = query
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    sounds
        .into_iter()
        .filter(|s| {
            let hay = format!("{} {} {}", s.id, s.name, s.tags.join(" ")).to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .collect()
}

pub fn find(root: &Path, id: &str) -> Result<Sound, String> {
    list(root)?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("No sound {id} in the library"))
}

fn tidy_tags(tags: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag = tag.trim().to_lowercase();
        if !tag.is_empty() && tag.chars().count() <= 30 && !out.contains(&tag) {
            out.push(tag);
        }
    }
    out.truncate(12);
    out
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
    let mut mine = load_mine(root);
    if mine.len() >= MAX_MINE {
        return Err(format!(
            "The library holds {MAX_MINE} sounds of your own at most"
        ));
    }
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sound".into());
    let slug: String = stem
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("-");
    let base = format!("my-{}", if slug.is_empty() { "sound" } else { &slug });
    let taken = |id: &str| BUILTIN.iter().any(|b| b.id == id) || mine.iter().any(|m| m.id == id);
    let id = (1..)
        .map(|n| {
            if n == 1 {
                base.clone()
            } else {
                format!("{base}-{n}")
            }
        })
        .find(|id| !taken(id))
        .unwrap_or(base);
    let ext = file
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "wav".into());
    let copy = format!("{id}.{ext}");
    let dir = root.join("mine");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::copy(file, dir.join(&copy)).map_err(|e| format!("Could not copy the sound: {e}"))?;
    let entry = Mine {
        id: id.clone(),
        name: name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or(stem),
        tags: tidy_tags(tags),
        duration_us,
        file: copy,
        licence,
    };
    mine.push(entry);
    save_mine(root, &mine)?;
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
    let mut mine = load_mine(root);
    let Some(entry) = mine.iter_mut().find(|m| m.id == id) else {
        return Err(if BUILTIN.iter().any(|b| b.id == id) {
            "Built-in sounds can't be changed".into()
        } else {
            format!("No sound {id} of your own")
        });
    };
    if let Some(name) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
        entry.name = name;
    }
    if let Some(tags) = tags {
        entry.tags = tidy_tags(tags);
    }
    if let Some(licence) = licence {
        entry.licence = licence;
    }
    save_mine(root, &mine)?;
    find(root, id)
}

/// Takes a sound of the user's out of the library (projects that used it keep their media).
pub fn remove(root: &Path, id: &str) -> Result<(), String> {
    let mut mine = load_mine(root);
    let Some(at) = mine.iter().position(|m| m.id == id) else {
        return Err(format!("No sound {id} of your own"));
    };
    let entry = mine.remove(at);
    save_mine(root, &mine)?;
    let _ = fs::remove_file(root.join("mine").join(entry.file));
    Ok(())
}

/// The project's media for `sound`: the asset already holding its file, or a new one imported
/// from it (its sounds as effects, with the sound's licence). The second value says whether
/// it is new.
pub fn asset_for(
    document: &EditDocument,
    project_root: &Path,
    sound: &Sound,
) -> Result<(Asset, bool), String> {
    let same = |a: &Asset| {
        let ours = Path::new(&sound.path);
        Path::new(&a.path) == ours
            || dunce::canonicalize(ours)
                .map(|p| Path::new(&a.path) == p)
                .unwrap_or(false)
    };
    if let Some(asset) = document.assets.iter().find(|a| same(a)) {
        return Ok((asset.clone(), false));
    }
    let mut asset = crate::media_bin::import(project_root, Path::new(&sound.path))?;
    asset.name = sound.name.clone();
    for stream in &mut asset.streams {
        stream.role = Role::Effects;
    }
    asset.licence = Some(sound.licence.clone());
    Ok((asset, true))
}

/// `document` with `asset` (added when new) placed whole at `at_us` on an effects track free
/// there, or a new one named [`TRACK_NAME`]. Answers the document and the clip's id.
pub fn place(
    document: &EditDocument,
    asset: Asset,
    at_us: u64,
) -> Result<(EditDocument, String), String> {
    use crate::sequence::edit::{apply_reserving, SequenceEdit};
    use crate::sequence::{SourceRange, TrackKind};
    let mut next = document.clone();
    let asset_id = asset.id.clone();
    let length = asset.duration_us.max(1);
    if next.asset(&asset_id).is_none() {
        next.assets.push(asset);
    }
    let at = at_us.min(next.duration_us());
    let taken: Vec<String> = next.track_ids_in_use().map(String::from).collect();
    let effects = |id: &str| {
        next.asset(id)
            .is_some_and(|a| a.streams.iter().all(|s| s.role == Role::Effects))
    };
    let free = next
        .sequence
        .audio_tracks()
        .find(|t| {
            !t.locked
                && (t.role == Some(Role::Effects)
                    || t.name == TRACK_NAME
                    || (!t.clips.is_empty() && t.clips.iter().all(|c| effects(&c.asset))))
                && t.is_free(at, at + length)
        })
        .map(|t| t.id.clone());
    let mut sequence = next.sequence.clone();
    let track_id = match free {
        Some(id) => id,
        None => {
            let (added, _) = apply_reserving(
                &sequence,
                &next.assets,
                &SequenceEdit::AddTrack {
                    track_kind: TrackKind::Audio,
                },
                taken.iter().map(String::as_str),
            )?;
            let id = added
                .audio_tracks()
                .find(|t| sequence.track(&t.id).is_none())
                .map(|t| t.id.clone())
                .ok_or("Could not add a track for the sound")?;
            let (named, _) = apply_reserving(
                &added,
                &next.assets,
                &SequenceEdit::SetTrack {
                    track_id: id.clone(),
                    name: TRACK_NAME.into(),
                    hidden: false,
                    muted: false,
                    locked: false,
                    role: Some(Role::Effects),
                },
                taken.iter().map(String::as_str),
            )?;
            sequence = named;
            id
        }
    };
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
