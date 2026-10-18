import { isTauriEnvironment } from "./ipc";
import type { OpenedProject } from "./types";

/// What the backend tells the editor about agent edits (MCP): the project after an agent's
/// change, and that a transcript or its captions changed.
const EVENT = "aeroedits-project-updated";
const CAPTIONS_EVENT = "aeroedits-captions-changed";

function listenTo<T>(event: string, handler: (payload: T) => void): () => void {
  if (!isTauriEnvironment()) return () => undefined;
  let unlisten: (() => void) | undefined;
  let disposed = false;
  void import("@tauri-apps/api/event").then(({ listen }) =>
    listen<T>(event, (e) => handler(e.payload)).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    }),
  );
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** Calls `onProject` with the project an agent's edit left; returns the unsubscribe. */
export function listenForProjects(onProject: (project: OpenedProject) => void): () => void {
  return listenTo<{ project: OpenedProject }>(EVENT, (payload) => onProject(payload.project));
}

/** Calls `onChange` when an agent changed a transcript; returns the unsubscribe. */
export function listenForCaptionChanges(onChange: () => void): () => void {
  return listenTo(CAPTIONS_EVENT, onChange);
}
