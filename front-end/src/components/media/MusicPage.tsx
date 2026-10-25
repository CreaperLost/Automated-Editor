import React, { useEffect, useMemo, useRef, useState } from "react";
import { Download, Loader2, Pause, Play, Plus, Search, Trash2, Upload } from "lucide-react";
import { Badge, Button, IconButton, Notice, Segmented } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import type { LibraryTrack, LicenceKind } from "../../lib/types";

/** The moods offered as quick filters, in this order. */
const MOODS = ["calm", "focus", "happy", "dreamy", "nostalgic", "upbeat", "game"];

const length = (us: number) => {
  const s = Math.round(us / 1e6);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

/// The music library (SND-3): built-in CC0 tracks (downloaded the first time they're used) and
/// the user's own, by mood. Play one to hear it; + fills from the playhead to the end of the
/// video on a music track, kept under the speech (one undo step). Tracks that need credit or
/// forbid monetised use are refused.
export const MusicPage: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const [tracks, setTracks] = useState<LibraryTrack[]>([]);
  const [search, setSearch] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [loading, setLoading] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const [note, setNote] = useState<string>();
  const [playing, setPlaying] = useState<string | null>(null);
  const [adding, setAdding] = useState<{ path: string; name: string; tags: string; licence: LicenceKind } | null>(null);
  const audio = useRef<HTMLAudioElement | null>(null);

  const load = () =>
    api
      .musicLibraryList()
      .then(setTracks)
      .catch((err) => setError(String(err)));
  useEffect(() => {
    void load();
    return () => audio.current?.pause();
  }, []);

  const run = async (label: string, work: () => Promise<void>) => {
    if (busy) return;
    setBusy(label);
    setError(undefined);
    setNote(undefined);
    try {
      await work();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  };

  const play = async (track: LibraryTrack) => {
    audio.current?.pause();
    if (playing === track.id) {
      setPlaying(null);
      return;
    }
    setLoading(track.id);
    try {
      const player = new Audio(await api.musicLibraryPreview(track.id));
      player.currentTime = track.startUs / 1e6;
      audio.current = player;
      player.onended = () => setPlaying((id) => (id === track.id ? null : id));
      setPlaying(track.id);
      if (!track.downloaded) void load();
      await player.play();
    } catch (err) {
      setPlaying(null);
      setError(String(err));
    } finally {
      setLoading(null);
    }
  };

  const place = (track: LibraryTrack) =>
    run("Adding…", async () => {
      const project = useProjectStore.getState().openedProject;
      if (!project) return;
      setLoading(track.id);
      try {
        const atUs = Math.round(useProjectStore.getState().currentTimeUs);
        const placed = await api.projectMusicPlace(project.projectHandle, project.revision, track.id, atUs);
        applyOpenedProject(placed.project);
        const repeats = placed.clipIds.length > 1 ? `, played ${placed.clipIds.length} times` : "";
        setNote(
          `${track.name} plays from ${length(placed.startUs)} to ${length(placed.endUs)}${repeats}.` +
            (placed.underSpeechDb ? ` Music now sits ${placed.underSpeechDb} dB under the speech (Audio panel).` : ""),
        );
        if (!track.downloaded) void load();
      } finally {
        setLoading(null);
      }
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
      await api.musicLibraryAdd(adding.path, adding.name || null, tags, { kind: adding.licence, source: "My own file" });
      setAdding(null);
      await load();
    });

  const removeOwn = (track: LibraryTrack) =>
    run("Removing…", async () => {
      await api.musicLibraryRemove(track.id);
      await load();
    });

  const words = search.trim().toLowerCase().split(/[\s,]+/).filter(Boolean);
  const shown = useMemo(
    () =>
      tracks.filter((t) => {
        const hay = `${t.id} ${t.name} ${t.tags.join(" ")}`.toLowerCase();
        return words.every((w) => hay.includes(w));
      }),
    [tracks, search],
  );
  const refused = adding ? !["cc0", "publicDomain", "noCredit"].includes(adding.licence) : false;

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="shrink-0 space-y-2 p-3 border-b border-studio-800">
        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-studio-500 pointer-events-none" />
            <input
              type="search"
              aria-label="Search music"
              placeholder="Search music…"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="ui-field w-full pl-8"
            />
          </div>
          <IconButton
            icon={Upload}
            variant="secondary"
            label="Add a track of your own to the library"
            disabled={busy !== null}
            onClick={() => void pickOwn()}
          />
        </div>
        <div className="flex flex-wrap gap-1">
          {MOODS.map((tag) => (
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
          <p className="text-label text-studio-200">Add to your music</p>
          <input
            aria-label="Name"
            className="ui-field w-full"
            value={adding.name}
            onChange={(e) => setAdding({ ...adding, name: e.target.value })}
          />
          <input
            aria-label="Tags"
            placeholder="Moods, comma separated (calm, upbeat…)"
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
              { value: "nonCommercial", label: "Non-commercial" },
            ]}
          />
          {refused && (
            <p className="text-meta text-danger-fg">
              Music must be free for monetised videos with no credit, so this can't go in the library.
            </p>
          )}
          <div className="flex justify-end gap-2">
            <Button variant="ghost" size="sm" onClick={() => setAdding(null)}>
              Cancel
            </Button>
            <Button variant="primary" size="sm" disabled={busy !== null || refused} onClick={() => void saveOwn()}>
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
      {note && (
        <Notice tone="accent" className="mx-3 mt-3 rounded-control border" onDismiss={() => setNote(undefined)}>
          {note}
        </Notice>
      )}
      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1">
        {shown.length === 0 && <p className="px-2 py-3 text-label text-studio-500">Nothing matches.</p>}
        {shown.map((track) => (
          <div
            key={track.id}
            className="group flex items-center gap-2.5 rounded-control border border-transparent px-2 py-1.5 hover:border-studio-700 hover:bg-studio-850"
            title={`${track.name}${track.licence.source ? `: ${track.licence.source}` : ""}`}
          >
            <IconButton
              icon={loading === track.id ? Loader2 : playing === track.id ? Pause : Play}
              size="sm"
              variant="secondary"
              label={playing === track.id ? `Stop ${track.name}` : `Play ${track.name}`}
              disabled={loading !== null && loading !== track.id}
              onClick={() => void play(track)}
              className={loading === track.id ? "[&_svg]:animate-spin" : undefined}
            />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-1.5 min-w-0">
                <span className="truncate text-label font-medium text-studio-100">{track.name}</span>
                {!track.builtIn && <Badge tone="audio">Mine</Badge>}
                {track.builtIn && !track.downloaded && (
                  <Download className="w-3 h-3 shrink-0 text-studio-600" aria-label="Downloads the first time it's used" />
                )}
              </div>
              <div className="text-meta text-studio-500 truncate">
                {length(track.durationUs)} · {track.tags.join(", ")}
              </div>
            </div>
            <div className="flex items-center opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity">
              <IconButton
                icon={Plus}
                size="sm"
                label={`Play ${track.name} under the video from the playhead to the end`}
                disabled={!openedProject || busy !== null}
                onClick={() => void place(track)}
              />
              {!track.builtIn && (
                <IconButton
                  icon={Trash2}
                  size="sm"
                  label={`Remove ${track.name} from your music (projects keep it)`}
                  disabled={busy !== null}
                  onClick={() => void removeOwn(track)}
                  className="hover:!text-danger-fg hover:!bg-danger/15"
                />
              )}
            </div>
          </div>
        ))}
        <p className="px-2 pt-2 text-meta text-studio-600 leading-relaxed">
          The built-in music is CC0 (by HoliznaCC0 and Komiku): free for monetised videos, no credit needed. Each track downloads
          the first time you play or add it.
        </p>
      </div>
    </div>
  );
};
