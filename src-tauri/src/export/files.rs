//! The other files of an edit: subtitles (SRT, WebVTT), the transcript (plain text, Markdown)
//! and the sound alone (WAV). Subtitles and transcript are made from the caption cues, so they
//! follow cuts, hidden words and caption breaks exactly as the burned-in captions do; the WAV
//! comes from the export's own mixer, so it sounds like the exported video.
use crate::captions::CaptionCue;
use crate::media::audio::{AudioMixer, CHANNELS, CHUNK_FRAMES, SAMPLE_RATE};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// A silence this long between captions starts a new paragraph of the transcript.
const PARAGRAPH_PAUSE_US: u64 = 2_000_000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    /// SubRip subtitles.
    Srt,
    /// WebVTT subtitles.
    Vtt,
    /// The transcript as plain text.
    Txt,
    /// The transcript as Markdown, with the chapters as headings.
    Md,
    /// The sound alone, 48 kHz stereo 16-bit.
    Wav,
}

impl FileKind {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Vtt => "vtt",
            Self::Txt => "txt",
            Self::Md => "md",
            Self::Wav => "wav",
        }
    }

    /// Subtitles show the captions as they are drawn (capitals included); a transcript reads as
    /// written.
    pub fn is_subtitles(self) -> bool {
        matches!(self, Self::Srt | Self::Vtt)
    }
}

fn cue_text(cue: &CaptionCue) -> String {
    cue.words
        .iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `h:mm:ss` under the hour or over it, `m:ss` under it, as chapter lists write them.
fn clock(us: u64) -> String {
    let s = us / 1_000_000;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// `hh:mm:ss<sep>mmm`.
fn stamp(us: u64, sep: char) -> String {
    let ms = us / 1000;
    format!(
        "{:02}:{:02}:{:02}{sep}{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

pub fn srt(cues: &[CaptionCue]) -> String {
    let mut out = String::new();
    for (i, cue) in cues.iter().enumerate() {
        out.push_str(&format!(
            "{}\r\n{} --> {}\r\n{}\r\n\r\n",
            i + 1,
            stamp(cue.start_us, ','),
            stamp(cue.end_us, ','),
            cue_text(cue)
        ));
    }
    out
}

pub fn vtt(cues: &[CaptionCue]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for cue in cues {
        // "-->" may not appear in a cue's text; nor may a blank line, which can't happen here.
        out.push_str(&format!(
            "{} --> {}\n{}\n\n",
            stamp(cue.start_us, '.'),
            stamp(cue.end_us, '.'),
            cue_text(cue).replace("-->", "->")
        ));
    }
    out
}

/// The transcript as paragraphs, each starting at a chapter or after a long pause, and each
/// paragraph's chapter (by index into `chapters`) when it opens one.
fn paragraphs(cues: &[CaptionCue], chapters: &[(u64, String)]) -> Vec<(Option<usize>, String)> {
    let mut out: Vec<(Option<usize>, String)> = Vec::new();
    let mut next_chapter = 0;
    let mut last_end = None;
    for cue in cues {
        let mut opens = None;
        while next_chapter < chapters.len() && chapters[next_chapter].0 <= cue.start_us {
            opens = Some(next_chapter);
            next_chapter += 1;
        }
        let paused = last_end.is_some_and(|end| cue.start_us.saturating_sub(end) >= PARAGRAPH_PAUSE_US);
        match out.last_mut() {
            Some((_, text)) if opens.is_none() && !paused => {
                text.push(' ');
                text.push_str(&cue_text(cue));
            }
            _ => out.push((opens, cue_text(cue))),
        }
        last_end = Some(cue.end_us);
    }
    out
}

pub fn plain(cues: &[CaptionCue], chapters: &[(u64, String)]) -> String {
    let mut out = String::new();
    for (opens, text) in paragraphs(cues, chapters) {
        if let Some(i) = opens {
            let (at, title) = &chapters[i];
            out.push_str(&format!("{} {title}\n\n", clock(*at)));
        }
        out.push_str(&text);
        out.push_str("\n\n");
    }
    out.truncate(out.trim_end().len());
    out.push('\n');
    out
}

pub fn markdown(title: &str, cues: &[CaptionCue], chapters: &[(u64, String)]) -> String {
    let mut out = format!("# {}\n\n", title.trim());
    for (opens, text) in paragraphs(cues, chapters) {
        if let Some(i) = opens {
            let (at, title) = &chapters[i];
            out.push_str(&format!("## {title} ({})\n\n", clock(*at)));
        }
        out.push_str(&text);
        out.push_str("\n\n");
    }
    out.truncate(out.trim_end().len());
    out.push('\n');
    out
}

/// Writes the mixed sound of the edit to `path` as a WAV file, calling `on_progress` with the
/// share done. Written beside the destination first, then renamed over it.
pub fn write_wav(
    mixer: &AudioMixer,
    path: &Path,
    on_progress: &mut dyn FnMut(f64),
) -> Result<(), String> {
    let block = CHANNELS as u64 * 2;
    let data_bytes = mixer.total_frames * block;
    if data_bytes + 36 > crate::project::pcm::MAX_WAV_BYTES {
        return Err("The edit is too long for a WAV file (over six hours)".into());
    }
    let partial = path.with_extension("wav.part");
    let result = (|| {
        let file = std::fs::File::create(&partial).map_err(|e| e.to_string())?;
        let mut out = std::io::BufWriter::new(file);
        let mut header = Vec::with_capacity(44);
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
        header.extend_from_slice(b"WAVEfmt ");
        header.extend_from_slice(&16u32.to_le_bytes());
        header.extend_from_slice(&1u16.to_le_bytes());
        header.extend_from_slice(&CHANNELS.to_le_bytes());
        header.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        header.extend_from_slice(&(SAMPLE_RATE * block as u32).to_le_bytes());
        header.extend_from_slice(&(block as u16).to_le_bytes());
        header.extend_from_slice(&16u16.to_le_bytes());
        header.extend_from_slice(b"data");
        header.extend_from_slice(&(data_bytes as u32).to_le_bytes());
        out.write_all(&header).map_err(|e| e.to_string())?;
        let mut frame = 0u64;
        let mut bytes = Vec::with_capacity(CHUNK_FRAMES * block as usize);
        while frame < mixer.total_frames {
            let count = CHUNK_FRAMES.min((mixer.total_frames - frame) as usize);
            bytes.clear();
            for sample in mixer.read_frames(frame, count)? {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
            out.write_all(&bytes).map_err(|e| e.to_string())?;
            frame += count as u64;
            on_progress(frame as f64 / mixer.total_frames as f64);
        }
        out.flush().map_err(|e| e.to_string())?;
        drop(out);
        std::fs::rename(&partial, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::captions::CueWord;

    fn cue(start_ms: u64, end_ms: u64, words: &[&str]) -> CaptionCue {
        CaptionCue {
            start_us: start_ms * 1000,
            end_us: end_ms * 1000,
            words: words
                .iter()
                .map(|w| CueWord {
                    id: w.to_string(),
                    text: w.to_string(),
                    start_us: start_ms * 1000,
                    end_us: end_ms * 1000,
                    emphasis: false,
                })
                .collect(),
        }
    }

    #[test]
    fn subtitles_number_and_stamp_each_cue() {
        let cues = [
            cue(0, 1_500, &["Hello", "there."]),
            cue(3_661_250, 3_662_000, &["a", "-->", "b"]),
        ];
        assert_eq!(
            srt(&cues),
            "1\r\n00:00:00,000 --> 00:00:01,500\r\nHello there.\r\n\r\n\
             2\r\n01:01:01,250 --> 01:01:02,000\r\na --> b\r\n\r\n"
        );
        assert_eq!(
            vtt(&cues),
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.500\nHello there.\n\n\
             01:01:01.250 --> 01:01:02.000\na -> b\n\n"
        );
        assert_eq!(srt(&[]), "");
    }

    #[test]
    fn transcript_breaks_at_chapters_and_long_pauses() {
        let cues = [
            cue(0, 1_000, &["One."]),
            cue(1_200, 2_000, &["Two."]),
            cue(5_000, 6_000, &["Three."]),
            cue(6_100, 7_000, &["Four."]),
        ];
        let chapters = [(0, "Intro".to_string()), (6_000_000, "Setup".to_string())];
        assert_eq!(
            markdown(" Demo ", &cues, &chapters),
            "# Demo\n\n## Intro (0:00)\n\nOne. Two.\n\nThree.\n\n## Setup (0:06)\n\nFour.\n"
        );
        assert_eq!(plain(&cues, &[]), "One. Two.\n\nThree. Four.\n");
        assert_eq!(clock(3_725_000_000), "1:02:05");
    }
}
