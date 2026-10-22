import type { TranscriptViewWord } from "../../lib/types";

export function formatTime(us: number): string {
  const total = Math.max(0, us) / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

/** A sentence of the video as it plays: edited times. */
export interface VideoLine {
  startUs: number;
  endUs: number;
  text: string;
}

const PAUSE_US = 700_000;
const LONGEST_US = 15_000_000;

/**
 * The video's kept words as lines to pick footage by: a line ends at a sentence's end, a pause,
 * or after 15 s.
 */
export function videoLines(words: TranscriptViewWord[]): VideoLine[] {
  const kept = words
    .filter((w) => w.kind === "word" && w.editedStartUs !== null && w.editedEndUs !== null)
    .sort((a, b) => a.editedStartUs! - b.editedStartUs!);
  const lines: VideoLine[] = [];
  let line: VideoLine | null = null;
  let ended = false;
  for (const word of kept) {
    const start = word.editedStartUs!;
    const end = word.editedEndUs!;
    if (line && (ended || start - line.endUs > PAUSE_US || end - line.startUs > LONGEST_US)) {
      lines.push(line);
      line = null;
    }
    if (!line) line = { startUs: start, endUs: end, text: word.text.trim() };
    else {
      line.endUs = Math.max(line.endUs, end);
      line.text = `${line.text} ${word.text.trim()}`;
    }
    ended = /[.?!…]["”’)]?$/.test(word.text.trim());
  }
  if (line) lines.push(line);
  return lines;
}

/**
 * Where something placed under a take goes: the timeline's selection when there is one, else
 * from the playhead for `lengthUs`. Kept inside the short; null when nothing fits.
 */
export function placement(
  selection: { startUs: number; endUs: number } | null,
  playheadUs: number,
  shortUs: number,
  lengthUs: number,
): { atUs: number; toUs: number; selected: boolean } | null {
  const selected = !!selection && selection.endUs - selection.startUs >= 100_000;
  const atUs = Math.max(0, Math.min(selected ? selection!.startUs : playheadUs, shortUs));
  const toUs = Math.min(selected ? selection!.endUs : atUs + lengthUs, shortUs);
  return toUs - atUs >= 100_000 ? { atUs, toUs, selected } : null;
}
