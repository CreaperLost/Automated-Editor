import React, { useEffect, useRef, useState } from "react";
import { Download, Loader2, Pause, Play, Plus, Trash2 } from "lucide-react";
import { Badge, IconButton, Notice } from "../ui";
import { useBusyRun } from "../../hooks/useBusyRun";
import { AddOwnFile, LibrarySearch, pickOwnFile, splitTags, useLibrarySearch, type OwnFile } from "./libraryParts";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import type { LibraryTrack } from "../../lib/types";

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
  const [loading, setLoading] = useState<string | null>(null);
  const [note, setNote] = useState<string>();
  const { busy, error, setError, run } = useBusyRun(() => setNote(undefined));
  const [playing, setPlaying] = useState<string | null>(null);
  const [adding, setAdding] = useState<OwnFile | null>(null);
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
      const file = await pickOwnFile();
      if (file) setAdding(file);
    });

  const saveOwn = () =>
    run("Adding…", async () => {
      if (!adding) return;
      await api.musicLibraryAdd(adding.path, adding.name || null, splitTags(adding.tags), { kind: adding.licence, source: "My own file" });
      setAdding(null);
      await load();
    });

  const removeOwn = (track: LibraryTrack) =>
    run("Removing…", async () => {
      await api.musicLibraryRemove(track.id);
      await load();
    });

  const shown = useLibrarySearch(tracks, search);
  const refused = adding ? !["cc0", "publicDomain", "noCredit"].includes(adding.licence) : false;

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <LibrarySearch
        noun="music"
        addLabel="Add a track of your own to the library"
        tags={MOODS}
        search={search}
        setSearch={setSearch}
        disabled={busy !== null}
        onAdd={() => void pickOwn()}
      />
      {adding && (
        <AddOwnFile
          noun="music"
          tagsHint="Moods, comma separated (calm, upbeat…)"
          licences={[
            { value: "noCredit", label: "Mine / no credit" },
            { value: "cc0", label: "CC0" },
            { value: "creditRequired", label: "Needs credit" },
            { value: "nonCommercial", label: "Non-commercial" },
          ]}
          adding={adding}
          setAdding={setAdding}
          warning={
            refused && (
              <p className="text-meta text-danger-fg">Music must be free for monetised videos with no credit, so this can't go in the library.</p>
            )
          }
          disabled={busy !== null || refused}
          onSave={() => void saveOwn()}
        />
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
