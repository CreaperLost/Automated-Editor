import { isTauriEnvironment } from "./ipc";
import type { OpenedProject } from "./types";

/// Keeps the editor and the Shorts Studio windows on the same project revision. Each window
/// broadcasts the project after its own edits and applies the other window's.
const EVENT = "aeroedits-project-updated";
const SOURCE = Math.random().toString(36).slice(2);

interface Payload {
  source: string;
  project: OpenedProject;
}

export function broadcastProject(project: OpenedProject) {
  if (!isTauriEnvironment()) return;
  void import("@tauri-apps/api/event")
    .then(({ emit }) => emit(EVENT, { source: SOURCE, project } satisfies Payload))
    .catch(() => undefined);
}

const CAPTIONS_EVENT = "aeroedits-captions-changed";

/** Tells the other windows a transcript or its captions changed. */
export function broadcastCaptionsChanged() {
  if (!isTauriEnvironment()) return;
  void import("@tauri-apps/api/event")
    .then(({ emit }) => emit(CAPTIONS_EVENT, { source: SOURCE }))
    .catch(() => undefined);
}

/** Calls `onChange` when another window changed a transcript; returns the unsubscribe. */
export function listenForCaptionChanges(onChange: () => void): () => void {
  if (!isTauriEnvironment()) return () => undefined;
  let unlisten: (() => void) | undefined;
  let disposed = false;
  void import("@tauri-apps/api/event").then(({ listen }) =>
    listen<{ source: string }>(CAPTIONS_EVENT, (event) => {
      if (event.payload.source !== SOURCE) onChange();
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    }),
  );
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** Calls `onProject` with projects other windows broadcast; returns the unsubscribe. */
export function listenForProjects(onProject: (project: OpenedProject) => void): () => void {
  if (!isTauriEnvironment()) return () => undefined;
  let unlisten: (() => void) | undefined;
  let disposed = false;
  void import("@tauri-apps/api/event").then(({ listen }) =>
    listen<Payload>(EVENT, (event) => {
      if (event.payload.source !== SOURCE) onProject(event.payload.project);
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    }),
  );
  return () => {
    disposed = true;
    unlisten?.();
  };
}
