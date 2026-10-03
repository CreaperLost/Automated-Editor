import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { FilePlus2, Film, FolderOpen, X } from "lucide-react";
import { api } from "../../lib/ipc";
import type { OpenedProject } from "../../lib/types";

interface NewProjectDialogProps {
  /** Pre-picked recording, e.g. when starting from "Edit a recording". */
  initialRecording?: string;
  /** Opens on "edit a recording" (true) or "start empty" (false). */
  startWithRecording?: boolean;
  onCreated: (project: OpenedProject) => void;
  onClose: () => void;
}

/// A new project gets its own folder. It can edit a recording folder, which is only read and
/// never written to, or start empty and be built from imported media.
export const NewProjectDialog: React.FC<NewProjectDialogProps> = ({
  initialRecording,
  startWithRecording,
  onCreated,
  onClose,
}) => {
  const [name, setName] = useState("");
  const [location, setLocation] = useState("");
  const [withRecording, setWithRecording] = useState(startWithRecording ?? !!initialRecording);
  const [recording, setRecording] = useState(initialRecording ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();

  useEffect(() => {
    void api
      .getDefaultProjectsDir()
      .then((dir) => setLocation((current) => current || dir))
      .catch(() => undefined);
  }, []);

  const pick = async (which: "location" | "recording") => {
    setError(undefined);
    try {
      const picked = which === "location" ? await api.pickProjectLocation() : await api.pickRecordingFolder();
      if (!picked) return;
      if (which === "location") setLocation(picked);
      else {
        setRecording(picked);
        setWithRecording(true);
      }
    } catch (err) {
      setError(String(err));
    }
  };

  const create = async () => {
    if (withRecording && !recording) {
      setError("Choose the recording to edit, or start an empty project.");
      return;
    }
    setBusy(true);
    setError(undefined);
    try {
      onCreated(await api.projectCreate(name.trim(), location, withRecording ? recording : undefined));
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const option = (selected: boolean) =>
    `flex-1 text-left rounded-lg border px-3 py-2.5 transition-colors ${
      selected ? "border-accent-hover/70 bg-accent-hover/10 text-white" : "border-studio-700 hover:border-studio-500 text-studio-300"
    }`;

  return createPortal(
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4 select-none">
      <div
        role="dialog"
        aria-label="New project"
        className="w-full max-w-lg bg-studio-900 border border-studio-700 rounded-2xl shadow-2xl overflow-hidden text-xs"
      >
        <div className="px-6 py-4 border-b border-studio-800 flex items-center justify-between bg-studio-850">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <FilePlus2 className="w-4 h-4 text-accent-hover" />
            New Project
          </div>
          <button aria-label="Close" onClick={onClose} className="p-1 rounded hover:bg-studio-700 text-studio-400">
            <X className="w-4 h-4" />
          </button>
        </div>

        <form
          className="p-6 space-y-5"
          onSubmit={(event) => {
            event.preventDefault();
            void create();
          }}
        >
          <label className="block space-y-1.5">
            <span className="text-studio-300 font-medium">Name</span>
            <input
              autoFocus
              aria-label="Project name"
              value={name}
              maxLength={80}
              placeholder="Untitled (today's date)"
              onChange={(event) => setName(event.target.value)}
              className="w-full bg-studio-950 border border-studio-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-accent-hover"
            />
          </label>

          <div className="space-y-1.5">
            <span className="text-studio-300 font-medium">Saved in</span>
            <div className="flex items-center gap-2">
              <span className="flex-1 truncate font-mono text-meta text-studio-400 bg-studio-950 border border-studio-800 rounded-lg px-3 py-2" title={location}>
                {location || "Documents/AeroEdits"}
              </span>
              <button
                type="button"
                onClick={() => void pick("location")}
                className="px-3 py-2 rounded-lg border border-studio-700 text-studio-200 hover:bg-studio-800"
              >
                Change…
              </button>
            </div>
            <p className="text-studio-500">The project gets its own folder here, named after it.</p>
          </div>

          <div className="space-y-1.5">
            <span className="text-studio-300 font-medium">Start with</span>
            <div className="flex gap-2">
              <button type="button" aria-pressed={!withRecording} onClick={() => setWithRecording(false)} className={option(!withRecording)}>
                <div className="font-semibold">Nothing</div>
                <div className="text-studio-400 mt-0.5">An empty timeline. Import video, images and audio.</div>
              </button>
              <button
                type="button"
                aria-pressed={withRecording}
                onClick={() => (recording ? setWithRecording(true) : void pick("recording"))}
                className={option(withRecording)}
              >
                <div className="font-semibold">A recording</div>
                <div className="text-studio-400 mt-0.5">Edit an AeroEdits recording. It is never changed.</div>
              </button>
            </div>
            {withRecording && (
              <div className="flex items-center gap-2 pt-1">
                <Film className="w-3.5 h-3.5 text-accent-hover shrink-0" />
                <span className="flex-1 truncate font-mono text-meta text-studio-300" title={recording}>
                  {recording || "No recording chosen"}
                </span>
                <button
                  type="button"
                  onClick={() => void pick("recording")}
                  className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg border border-studio-700 text-studio-200 hover:bg-studio-800"
                >
                  <FolderOpen className="w-3.5 h-3.5" />
                  Choose…
                </button>
              </div>
            )}
          </div>

          {error && (
            <p role="alert" className="text-danger-fg bg-danger/10 border border-danger/30 rounded-lg p-2">
              {error}
            </p>
          )}

          <div className="flex justify-end gap-2 pt-1">
            <button type="button" onClick={onClose} className="px-4 py-2 rounded-lg text-studio-300 hover:bg-studio-800">
              Cancel
            </button>
            <button
              type="submit"
              disabled={busy}
              className="px-4 py-2 rounded-lg bg-accent hover:bg-accent-hover text-white font-semibold disabled:opacity-50"
            >
              {busy ? "Creating…" : "Create Project"}
            </button>
          </div>
        </form>
      </div>
    </div>,
    document.body,
  );
};
