import React, { useEffect, useRef, useState } from "react";
import { Pause, Play, Plus, Trash2 } from "lucide-react";
import { Badge, IconButton, Notice } from "../ui";
import { useBusyRun } from "../../hooks/useBusyRun";
import { AddOwnFile, LibrarySearch, pickOwnFile, splitTags, useLibrarySearch, type OwnFile } from "./libraryParts";
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
  const { busy, error, setError, run } = useBusyRun();
  const [playing, setPlaying] = useState<string | null>(null);
  const [adding, setAdding] = useState<OwnFile | null>(null);
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
      const file = await pickOwnFile();
      if (file) setAdding(file);
    });

  const saveOwn = () =>
    run("Adding…", async () => {
      if (!adding) return;
      await api.soundLibraryAdd(adding.path, adding.name || null, splitTags(adding.tags), { kind: adding.licence, source: "My own file" });
      setAdding(null);
      await load();
    });

  const removeOwn = (sound: LibrarySound) =>
    run("Removing…", async () => {
      await api.soundLibraryRemove(sound.id);
      await load();
    });

  const shown = useLibrarySearch(sounds, search);

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <LibrarySearch
        noun="sounds"
        addLabel="Add a sound of your own to the library"
        tags={QUICK_TAGS}
        search={search}
        setSearch={setSearch}
        disabled={busy !== null}
        onAdd={() => void pickOwn()}
      />
      {adding && (
        <AddOwnFile
          noun="sounds"
          tagsHint="Tags, comma separated (whoosh, click…)"
          licences={[
            { value: "noCredit", label: "Mine / no credit" },
            { value: "cc0", label: "CC0" },
            { value: "creditRequired", label: "Needs credit" },
            { value: "unknown", label: "Unknown" },
          ]}
          adding={adding}
          setAdding={setAdding}
          warning={
            !needsNoCredit({ kind: adding.licence }) && <p className="text-meta text-suggest-fg">Check flags it before export wherever it plays.</p>
          }
          disabled={busy !== null}
          onSave={() => void saveOwn()}
        />
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
