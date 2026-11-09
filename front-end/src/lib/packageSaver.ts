import { api } from "./ipc";
import type { PublishPackage, PublishPackageView } from "./types";

/** How long after the last keystroke a package is saved. */
export const SAVE_AFTER_MS = 600;

type Entry = {
  /** The draft waiting for its timer, if any. */
  draft?: PublishPackage;
  timer?: number;
  /** Every save sent so far, one after another: a later one never lands before an earlier. */
  chain: Promise<unknown>;
  /** A draft whose save failed, kept so it is not lost; and why. */
  failed?: { draft: PublishPackage; error: string };
  listeners: Set<Listener>;
};

/** Told after each save of a package: the view it saved, or why it failed. */
type Listener = (result: { view: PublishPackageView } | { error: string }) => void;

const entries = new Map<string, Entry>();
const keyOf = (projectHandle: string, target: string) => `${projectHandle}\u0000${target}`;

function entry(projectHandle: string, target: string): Entry {
  const key = keyOf(projectHandle, target);
  let e = entries.get(key);
  if (!e) {
    e = { chain: Promise.resolve(), listeners: new Set() };
    entries.set(key, e);
  }
  return e;
}

/** Sends `e`'s waiting draft now, after the saves before it. Rejects when the save fails. */
function send(projectHandle: string, target: string, e: Entry): Promise<unknown> {
  window.clearTimeout(e.timer);
  e.timer = undefined;
  const draft = e.draft;
  e.draft = undefined;
  if (!draft) return e.chain;
  const save = e.chain
    .catch(() => undefined)
    .then(() => api.publishPackageSave(projectHandle, target, draft))
    .then(
      (view) => {
        // Saves land in order, so this one supersedes any that failed before it.
        e.failed = undefined;
        e.listeners.forEach((l) => l({ view }));
        return view;
      },
      (err) => {
        e.failed = { draft, error: String(err) };
        e.listeners.forEach((l) => l({ error: String(err) }));
        throw err;
      },
    );
  e.chain = save;
  return save;
}

/**
 * The saves of each video's package (PUB-1), owned by the project and target they were typed
 * for rather than by the editor on screen: closing the editor sends what is waiting instead of
 * dropping it, and Upload waits for every save to land before it sends anything (GAP-015/016).
 * A save always goes to the project it was typed in; the back end refuses it if another project
 * has been opened since (GAP-011).
 */
export const packageSaver = {
  /** Saves `draft` a moment after the last change. */
  schedule(projectHandle: string, target: string, draft: PublishPackage, delayMs = SAVE_AFTER_MS) {
    const e = entry(projectHandle, target);
    e.draft = draft;
    window.clearTimeout(e.timer);
    e.timer = window.setTimeout(() => {
      void send(projectHandle, target, e).catch(() => undefined);
    }, delayMs);
  },

  /** Sends the waiting draft now without waiting for it: an editor closing. */
  sendNow(projectHandle: string, target: string) {
    const e = entries.get(keyOf(projectHandle, target));
    if (e?.draft) void send(projectHandle, target, e).catch(() => undefined);
  },

  /** Whether a draft is waiting for its timer. */
  pending(projectHandle: string, target: string): boolean {
    const e = entries.get(keyOf(projectHandle, target));
    return !!e && (e.draft !== undefined || e.timer !== undefined);
  },

  /**
   * Sends the waiting draft now and resolves once every save sent has landed, with the package
   * as saved (read again when nothing was waiting). Rejects with the reason when a save failed:
   * nothing that depends on the package should go ahead then.
   */
  async flush(projectHandle: string, target: string): Promise<PublishPackageView> {
    const e = entry(projectHandle, target);
    await send(projectHandle, target, e);
    if (e.failed) throw new Error(e.failed.error);
    return api.publishPackage(projectHandle, target);
  },

  /** A draft whose save failed, so the editor can show it again rather than the old package. */
  unsaved(projectHandle: string, target: string): { draft: PublishPackage; error: string } | undefined {
    return entries.get(keyOf(projectHandle, target))?.failed;
  },

  /** Calls `listener` after each save of this package; returns the unsubscribe. */
  onSaved(projectHandle: string, target: string, listener: Listener) {
    const e = entry(projectHandle, target);
    e.listeners.add(listener);
    return () => e.listeners.delete(listener);
  },

  /** Forgets everything; for tests. */
  reset() {
    entries.forEach((e) => window.clearTimeout(e.timer));
    entries.clear();
  },
};
