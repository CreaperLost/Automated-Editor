//! Sounds at graphic moments (SND-2), opt-in: the style's sound set (`Style.sound.cues`, a
//! library sound per role) played where a graphic comes on (`in`, else the style's `whoosh`),
//! at moments the composition marks itself, and where a graphic held a while goes off (`out`).
//!
//! A composition marks its moments on its root element: `data-cues="0.4:pop 1.2:hit"`
//! (seconds into the composition, then the role). Each sound is placed so its loudest moment
//! lands on the graphic's, not its start. Never many: at least [`MIN_GAP_US`] apart and at
//! most [`PER_MINUTE`] in any minute, a graphic's entrance before its marks before its exit.
//! The sounds go on their own track, ducked under the speech; placing them again replaces it.
use crate::project::revision::EditDocument;
use crate::style::Cue;
use serde::Serialize;

/// The cues' own track: placing again clears it first.
pub const TRACK_NAME: &str = "Graphic cues";
/// The root attribute a composition marks its moments with.
pub const MARKS_ATTR: &str = "data-cues";
/// How far the cues go down under speech.
pub const DUCK_DB: f32 = 8.0;
/// Cues closer together than this become the first.
pub const MIN_GAP_US: u64 = 1_500_000;
/// At most this many cues start in any minute.
pub const PER_MINUTE: usize = 6;
/// Only a graphic held this long gets a cue as it goes off.
const OUT_AFTER_US: u64 = 2_000_000;
/// A cue cut short by the next one fades out over this.
pub const TAIL_FADE_US: u64 = 40_000;
const MINUTE_US: u64 = 60_000_000;

/// A graphic moment on the timeline.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moment {
    pub at_us: u64,
    /// `in`, `out` or the role a composition's mark names.
    pub role: String,
    /// The graphic's name.
    pub graphic: String,
    /// 0 an entrance, 1 a mark, 2 an exit: the order cues are kept in.
    #[serde(skip)]
    pub rank: u8,
}

/// A moment with the library sound it gets.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Planned {
    #[serde(flatten)]
    pub moment: Moment,
    pub sound: String,
}

/// A composition's marks: (seconds into it, role), from `data-cues="0.4:pop 1.2:hit"`.
pub fn marks(html: &str) -> Result<Vec<(u64, String)>, String> {
    let Some(value) = crate::graphics::root_attribute(html, MARKS_ATTR) else {
        return Ok(Vec::new());
    };
    value
        .split([' ', ','])
        .filter(|w| !w.is_empty())
        .map(|word| {
            let bad = || format!("`{MARKS_ATTR}` takes \"seconds:role\" words, not \"{word}\"");
            let (at, role) = word.split_once(':').ok_or_else(bad)?;
            let at: f64 = at.parse().map_err(|_| bad())?;
            if !at.is_finite() || at < 0.0 || role.is_empty() {
                return Err(bad());
            }
            Ok(((at * 1e6).round() as u64, role.to_string()))
        })
        .collect()
}

/// The graphic moments on `document`'s timeline, by time. `marks_of(asset id)` gives a
/// composition's marks.
pub fn moments(
    document: &EditDocument,
    marks_of: impl Fn(&str) -> Vec<(u64, String)>,
) -> Vec<Moment> {
    let mut out = Vec::new();
    for track in document.sequence.video_tracks().filter(|t| !t.hidden) {
        for clip in &track.clips {
            let Some(asset) = document.asset(&clip.asset).filter(|a| a.is_graphic()) else {
                continue;
            };
            let moment = |at_us: u64, role: &str, rank: u8| Moment {
                at_us,
                role: role.into(),
                graphic: asset.name.clone(),
                rank,
            };
            let end = clip.start_us + clip.duration_us;
            out.push(moment(clip.start_us, "in", 0));
            let speed = if clip.speed > 0.0 { clip.speed } else { 1.0 };
            for (at, role) in marks_of(&asset.id) {
                if at < clip.in_us {
                    continue;
                }
                let on_timeline = clip.start_us + ((at - clip.in_us) as f64 / speed) as u64;
                if on_timeline > clip.start_us && on_timeline < end {
                    out.push(moment(on_timeline, &role, 1));
                }
            }
            if clip.duration_us >= OUT_AFTER_US {
                out.push(moment(end, "out", 2));
            }
        }
    }
    out.sort_by_key(|m| (m.at_us, m.rank));
    out
}

/// The sound for `role` in `cues`: its own, else for an entrance the `whoosh`.
fn sound_for<'c>(cues: &'c [Cue], role: &str) -> Option<&'c str> {
    let find = |r: &str| {
        cues.iter()
            .find(|c| c.role.eq_ignore_ascii_case(r) && !c.sound.trim().is_empty())
            .map(|c| c.sound.as_str())
    };
    find(role).or_else(|| (role == "in").then(|| find("whoosh")).flatten())
}

/// The moments that get a sound under `cues`, by time: those with a sound for their role,
/// kept [`MIN_GAP_US`] apart and at most [`PER_MINUTE`] a minute, entrances first.
pub fn plan(moments: &[Moment], cues: &[Cue]) -> Vec<Planned> {
    let mut wanted: Vec<Planned> = moments
        .iter()
        .filter_map(|m| {
            sound_for(cues, &m.role).map(|sound| Planned {
                moment: m.clone(),
                sound: sound.to_string(),
            })
        })
        .collect();
    wanted.sort_by_key(|p| (p.moment.rank, p.moment.at_us));
    let mut kept: Vec<Planned> = Vec::new();
    for p in wanted {
        let t = p.moment.at_us;
        if kept.iter().any(|k| k.moment.at_us.abs_diff(t) < MIN_GAP_US) {
            continue;
        }
        let mut starts: Vec<u64> = kept.iter().map(|k| k.moment.at_us).chain([t]).collect();
        starts.sort_unstable();
        let fits = starts.iter().all(|&w| {
            starts
                .iter()
                .filter(|&&x| x >= w && x < w + MINUTE_US)
                .count()
                <= PER_MINUTE
        });
        if fits {
            kept.push(p);
        }
    }
    kept.sort_by_key(|p| p.moment.at_us);
    kept
}

/// Where the loudest moment of interleaved `samples` is, in microseconds from their start.
pub fn peak_offset_us(samples: &[f32], channels: u16, sample_rate: u32) -> u64 {
    let channels = channels.max(1) as usize;
    let loudest = samples
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map_or(0, |(i, _)| i / channels);
    (loudest as u128 * 1_000_000 / sample_rate.max(1) as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u64 = 1_000_000;

    fn m(at_s: f64, role: &str, rank: u8) -> Moment {
        Moment {
            at_us: (at_s * 1e6) as u64,
            role: role.into(),
            graphic: "g".into(),
            rank,
        }
    }

    fn cue(role: &str, sound: &str) -> Cue {
        Cue {
            role: role.into(),
            sound: sound.into(),
        }
    }

    #[test]
    fn marks_read_from_the_root_and_bad_ones_are_refused() {
        let html = r#"<div data-composition-id="g" data-cues="0.4:pop 1.25:hit"></div>"#;
        assert_eq!(
            marks(html).unwrap(),
            vec![(400_000, "pop".into()), (1_250_000, "hit".into())]
        );
        assert!(marks(r#"<div data-composition-id="g"></div>"#)
            .unwrap()
            .is_empty());
        assert!(marks(r#"<div data-composition-id="g" data-cues="pop"></div>"#).is_err());
    }

    #[test]
    fn cues_follow_the_style_keep_apart_and_few_a_minute() {
        // No sound for a role: no cue. An entrance falls back on the style's whoosh.
        let cues = [cue("whoosh", "whoosh-soft"), cue("pop", "pop")];
        let moments = [
            m(0.0, "in", 0),
            m(0.5, "pop", 1),
            m(3.0, "pop", 1),
            m(4.0, "out", 2),
        ];
        let planned = plan(&moments, &cues);
        let got: Vec<(u64, &str)> = planned
            .iter()
            .map(|p| (p.moment.at_us, p.sound.as_str()))
            .collect();
        // The mark at 0.5 s is too close to the entrance; the exit has no sound.
        assert_eq!(got, vec![(0, "whoosh-soft"), (3 * S, "pop")]);
        assert!(plan(&moments, &[]).is_empty());

        // Twenty entrances 2 s apart: six in the first minute.
        let many: Vec<Moment> = (0..20).map(|i| m(i as f64 * 2.0, "in", 0)).collect();
        let kept = plan(&many, &cues);
        assert_eq!(kept.len(), PER_MINUTE);
        // An entrance wins over a mark near it, whichever comes first.
        let both = [m(1.0, "pop", 1), m(1.5, "in", 0)];
        assert_eq!(plan(&both, &cues)[0].moment.role, "in");
    }

    #[test]
    fn the_loudest_sample_gives_the_peak() {
        let mut samples = vec![0.0f32; 96_000];
        samples[48_000] = -0.9; // frame 24 000 of stereo at 48 kHz: half a second in
        assert_eq!(peak_offset_us(&samples, 2, 48_000), 500_000);
    }
}
