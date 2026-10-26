import React, { useEffect, useMemo, useRef, useState } from "react";
import { Pause, Play, Plus, Search, Trash2, Upload } from "lucide-react";
import { Badge, Button, IconButton, Notice, Segmented } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import type { LibrarySound, Licence, LicenceKind } from "../../lib/types";

/** The tags offered as quick filters, in this order. */
const QUICK_TAGS = ["whoosh", "riser", "hit", "click", "pop", "ding"];

const LICENCE_LABEL: Record<LicenceKind, string> = {
  cc0: "CC0",
  publicDomain: "Public domain",
  noCredit: "No credit needed",
  creditRequired: "Needs credit",
  nonCommercial: "Non-commercial",
  unknown: "Unknown",
};

const seconds = (us: number) => `${(us / 1e6).toFixed(us < 1e6 ? 2 : 1)}s`;
const needsNoCredit = (licence: Licence) => ["cc0", "publicDomain", "noCredit"].includes(licence.kind);

/// The sound effects library (SND-3): the built-in CC0 set and the user's own, by tag. Play
/// one to hear it; + puts it at the playhead on an effects track (one undo step).
export const SoundsPage: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const [sounds, setSounds] = useState<LibrarySound[]>([]);
  const [search, setSearch] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const [playing, setPlaying] = useState<string | null>(null);
  const [adding, setAdding] = useState<{ path: string; name: string; tags: string; licence: LicenceKind } | null>(null);
  const audio = useRef<HTMLAudioElement | null>(null);

  const load = () =>
    api
      .soundLibraryList()
      .then(setSounds)
      .catch((err) => setError(String(err)));
  useEffect(() => {
    void load();
    return () => audio.current?.pause();
  }, []);

  const run = async (label: string, work: () => Promise<void>) => {
    if (busy) return;
    setBusy(label);
    setError(undefined);
    try {
      await work();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  };

  const play = async (sound: LibrarySound) => {
    audio.current?.pause();
    if (playing === sound.id) {
      setPlaying(null);
      return;
    }
    try {
      const player = new Audio(await api.soundLibraryPreview(sound.id));
      audio.current = player;
      player.onended = () => setPlaying((id) => (id === sound.id ? null : id));
      setPlaying(sound.id);
      await player.play();
    } catch (err) {
      setPlaying(null);
      setError(String(err));
    }
  };

  const place = (sound: LibrarySound) =>
    run("Adding…", async () => {
      const project = useProjectStore.getState().openedProject;
      if (!project) return;
      const atUs = Math.round(useProjectStore.getState().currentTimeUs);
      const placed = await api.projectSoundPlace(project.projectHandle, project.revision, sound.id, atUs);
      applyOpenedProject(placed.project);
    });

  const pickOwn = () =>
    run("Choosing…", async () => {
      const [path] = await api.pickMediaFiles();
      if (!path) return;
      const name = path.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") ?? "";
      setAdding({ path, name, tags: "", licence: "noCredit" });
    });

  const saveOwn = () =>
    run("Adding…", async () => {
      if (!adding) return;
      const tags = adding.tags.split(",").map((t) => t.trim()).filter(Boolean);
      await api.soundLibraryAdd(adding.path, adding.name || null, tags, { kind: adding.licence, source: "My own file" });
      setAdding(null);
      await load();
    });

  const removeOwn = (sound: LibrarySound) =>
    run("Removing…", async () => {
      await api.soundLibraryRemove(sound.id);
      await load();
    });

  const words = search.trim().toLowerCase().split(/[\s,]+/).filter(Boolean);
  const shown = useMemo(
    () =>
      sounds.filter((s) => {
        const hay = `${s.id} ${s.name} ${s.tags.join(" ")}`.toLowerCase();
        return words.every((w) => hay.includes(w));
      }),
    [sounds, search],
  );

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="shrink-0 space-y-2 p-3 border-b border-studio-800">
        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-studio-500 pointer-events-none" />
            <input
              type="search"
              aria-label="Search sounds"
              placeholder="Search sounds…"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="ui-field w-full pl-8"
            />
          </div>
          <IconButton
            icon={Upload}
            variant="secondary"
            label="Add a sound of your own to the library"
            disabled={busy !== null}
            onClick={() => void pickOwn()}
          />
        </div>
        <div className="flex flex-wrap gap-1">
          {QUICK_TAGS.map((tag) => (
            <button
              key={tag}
              type="button"
              onClick={() => setSearch((s) => (s.trim().toLowerCase() === tag ? "" : tag))}
              className={`h-6 px-2 rounded-control border text-meta transition-colors ${
                search.trim().toLowerCase() === tag
                  ? "border-audio/60 bg-audio/15 text-audio-fg"
                  : "border-studio-700 text-studio-300 hover:bg-studio-850"
              }`}
            >
              {tag}
            </button>
          ))}
        </div>
      </div>
      {adding && (
        <div className="m-3 space-y-2 rounded-panel border border-studio-700 p-3">
          <p className="text-label text-studio-200">Add to your sounds</p>
          <input
            aria-label="Name"
            className="ui-field w-full"
            value={adding.name}
            onChange={(e) => setAdding({ ...adding, name: e.target.value })}
          />
          <input
            aria-label="Tags"
            placeholder="Tags, comma separated (whoosh, click…)"
            className="ui-field w-full"
            value={adding.tags}
            onChange={(e) => setAdding({ ...adding, tags: e.target.value })}
          />
          <Segmented<LicenceKind>
            label="Licence"
            size="sm"
            className="w-full"
            value={adding.licence}
            onChange={(licence) => setAdding({ ...adding, licence })}
            options={[
              { value: "noCredit", label: "Mine / no credit" },
              { value: "cc0", label: "CC0" },
              { value: "creditRequired", label: "Needs credit" },
              { value: "unknown", label: "Unknown" },
            ]}
          />
          {!needsNoCredit({ kind: adding.licence }) && (
            <p className="text-meta text-suggest-fg">Check flags it before export wherever it plays.</p>
          )}
          <div className="flex justify-end gap-2">
            <Button variant="ghost" size="sm" onClick={() => setAdding(null)}>
              Cancel
            </Button>
            <Button variant="primary" size="sm" disabled={busy !== null} onClick={() => void saveOwn()}>
              Add
            </Button>
          </div>
        </div>
      )}
      {error && (
        <Notice tone="danger" className="mx-3 mt-3 rounded-control border" onDismiss={() => setError(undefined)}>
          {error}
        </Notice>
      )}
      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1">
        {shown.length === 0 && <p className="px-2 py-3 text-label text-studio-500">Nothing matches.</p>}
        {shown.map((sound) => (
          <div
            key={sound.id}
            className="group flex items-center gap-2.5 rounded-control border border-transparent px-2 py-1.5 hover:border-studio-700 hover:bg-studio-850"
            title={`${sound.name}: ${LICENCE_LABEL[sound.licence.kind]}${sound.licence.source ? `, ${sound.licence.source}` : ""}`}
          >
            <IconButton
              icon={playing === sound.id ? Pause : Play}
              size="sm"
              variant="secondary"
              label={playing === sound.id ? `Stop ${sound.name}` : `Play ${sound.name}`}
              onClick={() => void play(sound)}
            />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-1.5 min-w-0">
                <span className="truncate text-label font-medium text-studio-100">{sound.name}</span>
                {!sound.builtIn && <Badge tone="audio">Mine</Badge>}
                {!needsNoCredit(sound.licence) && (
                  <Badge tone="suggest" title="Its licence needs credit or isn't known">
                    {LICENCE_LABEL[sound.licence.kind]}
                  </Badge>
                )}
              </div>
              <div className="text-meta text-studio-500 truncate">
                {seconds(sound.durationUs)} · {sound.tags.join(", ")}
              </div>
            </div>
            <div className="flex items-center opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity">
              <IconButton
                icon={Plus}
                size="sm"
                label={`Add ${sound.name} at the playhead`}
                disabled={!openedProject || busy !== null}
                onClick={() => void place(sound)}
              />
              {!sound.builtIn && (
                <IconButton
                  icon={Trash2}
                  size="sm"
                  label={`Remove ${sound.name} from your sounds (projects keep it)`}
                  disabled={busy !== null}
                  onClick={() => void removeOwn(sound)}
                  className="hover:!text-danger-fg hover:!bg-danger/15"
                />
              )}
            </div>
          </div>
        ))}
        <p className="px-2 pt-2 text-meta text-studio-600 leading-relaxed">
          The built-in sounds are CC0: free for any video, no credit needed.
        </p>
      </div>
    </div>
  );
};
