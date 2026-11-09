// Canned answers for the preview build's mocked backend (see install.ts). `backend.json` and
// `frame.jpg` hold what the real commands answered for the smoke test's fixture project
// (`cd src-tauri && cargo test --lib preview_mock -- --ignored` writes them).
import data from "./backend.json";
import frameUrl from "./frame.jpg";
import { PREVIEW_FRAME_HEADER } from "../lib/ipc";

export const PROJECT_PATH = "/home/george/AeroEdits/Smoke";

const answers = data as Record<string, unknown>;

/** Commands answered with another's canned answer. */
const SAME_AS: Record<string, string> = {
  preview_layout: "preview_attach",
  preview_detach: "preview_attach",
  preview_status: "preview_attach",
};

/** Commands that answer nothing (or nothing the UI reads). */
const QUIET = new Set(["set_window_title"]);

let frame: Promise<ArrayBuffer> | undefined;

/** The preview's one picture, behind the 16-byte header (sequence 1, generation 0). */
async function previewFrame(after: number, waitMs: number): Promise<ArrayBuffer> {
  if (after >= 1) {
    // Nothing new: wait as the app would, so the preview does not spin.
    await new Promise((done) => setTimeout(done, Math.max(waitMs, 100)));
    return new ArrayBuffer(0);
  }
  frame ??= fetch(frameUrl).then((r) => r.arrayBuffer());
  const jpeg = new Uint8Array(await frame);
  const out = new Uint8Array(PREVIEW_FRAME_HEADER + jpeg.byteLength);
  new DataView(out.buffer).setBigUint64(0, 1n, true);
  out.set(jpeg, PREVIEW_FRAME_HEADER);
  return out.buffer;
}

/** The canned film-strip thumbnail nearest each time asked for. */
function thumbnails(timesUs: number[]): unknown[] {
  const canned = answers.project_clip_thumbnails as { timesUs: number[]; thumbnails: unknown[] };
  return timesUs.map((at) => {
    let best = 0;
    canned.timesUs.forEach((t, i) => {
      if (Math.abs(t - at) < Math.abs(canned.timesUs[best] - at)) best = i;
    });
    return canned.thumbnails[best];
  });
}

export function answer(cmd: string, args: Record<string, unknown>): unknown {
  if (cmd === "preview_frame") return previewFrame(Number(args.after ?? 0), Number(args.waitMs ?? 0));
  if (cmd === "project_clip_thumbnails") return thumbnails((args.timesUs as number[]) ?? []);
  if (cmd === "pick_project_folder") return PROJECT_PATH;
  // The engine marks the preview available once it has drawn a frame; here one is always ready.
  if (cmd === "playback_status") return { ...(answers.playback_status as object), previewAvailable: true };
  const name = SAME_AS[cmd] ?? cmd;
  if (name in answers) return structuredClone(answers[name]);
  if (QUIET.has(cmd)) return null;
  return undefined;
}
