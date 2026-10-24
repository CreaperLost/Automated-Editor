import React from "react";
import { FilePlus2, Search } from "lucide-react";
import { api } from "../../lib/ipc";
import { soundSources } from "../../lib/sequence";
import type {
  Asset,
  GraphicLibraryItem,
  GraphicListed,
  GraphicSlot,
  GraphicTemplate,
  OpenedProject,
  Short,
} from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import {
  addFootage,
  addShortGraphic,
  importForShort,
  setShortsError,
  useShortsStore,
} from "../../stores/shortsStore";
import { Button, Segmented } from "../ui";
import { formatTime, placement, videoLines, type VideoLine } from "./underTakeParts";

type Source = "video" | "graphic" | "file";

/** A file's stretch when nothing is selected: a still or a clip's first seconds. */
const FILE_US = 4_000_000;

/** How fast footage from the video plays under the take: more fits faster, less fills slower. */
const SPEEDS = [0.75, 1, 1.25, 1.5, 2] as const;

/** How a graphic shows in a short, chosen before it is made so it renders once. */
const MODES: { value: GraphicSlot; label: string; title: string }[] = [
  { value: "full", label: "Full", title: "The whole frame: the graphic is the point" },
  { value: "screen", label: "Split", title: "Your face above, the graphic below" },
  { value: "overFace", label: "Over face", title: "Your face fills the frame, the graphic over it" },
];

const ROW =
  "w-full flex items-baseline gap-2 rounded-control px-1.5 py-1 text-left text-meta hover:bg-studio-800 disabled:opacity-40 disabled:hover:bg-transparent";

/// Places footage from the video, a graphic or an outside file under a take, at the timeline's
/// selection or from the playhead. The same placements the agent makes (short_footage,
/// graphic_add with `short`), one undo step each.
export const UnderTake: React.FC<{ project: OpenedProject; short: Short }> = ({ project, short }) => {
  const [source, setSource] = React.useState<Source>("video");
  const busy = useShortsStore((s) => s.busy);
  const selection = useProjectStore((s) => s.timelineSelection);
  const playheadUs = useProjectStore((s) => s.currentTimeUs);
  const at = (lengthUs: number) => placement(selection, playheadUs, project.durationUs, lengthUs);
  const where = at(1_000_000);
  return (
    <div className="space-y-1.5 rounded-control border border-studio-800 p-1.5">
      <Segmented<Source>
        label="Put under the take"
        size="sm"
        className="w-full [&>button]:flex-1"
        value={source}
        onChange={setSource}
        options={[
          { value: "video", label: "Video", title: "Footage from the video, by what is said there" },
          { value: "graphic", label: "Graphic", title: "One of the video's graphics, a library item or a template" },
          { value: "file", label: "File", title: "An outside video or picture (B-roll)" },
        ]}
      />
      <p className="text-meta text-studio-500">
        {!where
          ? "Move the playhead inside the short, or select a stretch on its timeline."
          : where.selected
            ? `Goes on the selection, ${formatTime(where.atUs)}–${formatTime(where.toUs)}.`
            : `Goes from the playhead, ${formatTime(where.atUs)} (select a stretch to set its length).`}
      </p>
      {source === "video" && <FromVideo shortId={short.id} at={at} disabled={!!busy || !where} />}
      {source === "graphic" && uncut(project) && (
        <p role="alert" className="text-meta text-suggest-fg">
          The take isn't cut yet: graphics on uncut words play long, and a cut later can take their words away. Clean
          the take first, then place graphics on what it keeps.
        </p>
      )}
      {source === "graphic" && (
        <FromGraphics project={project} shortId={short.id} at={at} disabled={!!busy || !where} />
      )}
      {source === "file" && <FromFile project={project} short={short} at={at} disabled={!!busy || !where} />}
    </div>
  );
};

/** The take not cut yet: its voice is still the one clip it came in as (as `take_short::uncut`). */
function uncut(project: OpenedProject): boolean {
  return project.sequence.tracks.filter((t) => t.kind === "audio" && t.role === "mic").every((t) => t.clips.length <= 1);
}

type At = (lengthUs: number) => { atUs: number; toUs: number; selected: boolean } | null;

const FromVideo: React.FC<{ shortId: string; at: At; disabled: boolean }> = ({ shortId, at, disabled }) => {
  const main = useShortsStore((s) => s.main);
  const [lines, setLines] = React.useState<VideoLine[] | null>(null);
  const [query, setQuery] = React.useState("");
  const [speed, setSpeed] = React.useState<number>(1);
  // The video's speech: what plays on its timeline, spoken first.
  const speech = React.useMemo(() => {
    const placed = soundSources(main).filter((s) => s.placed);
    return placed.find((s) => s.speech) ?? placed[0];
  }, [main]);
  React.useEffect(() => {
    if (!main || !speech) return;
    let live = true;
    api
      .transcriptGetVideo(main.projectHandle, speech.key)
      .then((view) => live && setLines(view ? videoLines(view.words) : []))
      .catch((err) => live && (setLines([]), setShortsError(err)));
    return () => {
      live = false;
    };
  }, [main?.projectHandle, main?.revision, speech?.key]);
  if (!main || !speech) return <p className="text-meta text-studio-500">The video has no sound to search by.</p>;
  if (!lines) return <p className="text-meta text-studio-500">Reading the video's transcript…</p>;
  if (lines.length === 0)
    return <p className="text-meta text-studio-500">Transcribe the video first: its lines are how footage is found.</p>;
  const words = query.trim().toLowerCase();
  const shown = words ? lines.filter((l) => l.text.toLowerCase().includes(words)) : lines;
  return (
    <>
      <Segmented<number>
        label="How fast the footage plays"
        size="sm"
        className="w-full [&>button]:flex-1"
        value={speed}
        onChange={setSpeed}
        options={SPEEDS.map((s) => ({
          value: s,
          label: `${s}×`,
          title: s === 1 ? "As recorded" : s > 1 ? "Faster: more footage fits" : "Slower: less footage fills more",
        }))}
      />
      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-studio-500 pointer-events-none" />
        <input
          type="search"
          aria-label="Search the video's words"
          placeholder="Search what the video says…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="ui-field w-full pl-8"
        />
      </div>
      <ul className="max-h-64 overflow-y-auto" aria-label="The video's lines">
        {shown.slice(0, 300).map((line) => (
          <li key={line.startUs}>
            <button
              type="button"
              className={ROW}
              disabled={disabled}
              title="Put the video from here under the take"
              onClick={() => {
                const range = at((line.endUs - line.startUs) / speed);
                if (range)
                  addFootage(shortId, [
                    { atUs: range.atUs, toUs: range.toUs, videoUs: line.startUs, ...(speed !== 1 ? { speed } : {}) },
                  ]);
              }}
            >
              <span className="font-mono tabular-nums text-studio-500 shrink-0">{formatTime(line.startUs)}</span>
              <span className="text-studio-200 line-clamp-2">{line.text}</span>
            </button>
          </li>
        ))}
        {shown.length === 0 && <li className="text-meta text-studio-500 px-1.5">No line says that.</li>}
      </ul>
    </>
  );
};

interface GraphicChoices {
  video: GraphicListed[];
  library: GraphicLibraryItem[];
  templates: GraphicTemplate[];
}

const FromGraphics: React.FC<{ project: OpenedProject; shortId: string; at: At; disabled: boolean }> = ({
  project,
  shortId,
  at,
  disabled,
}) => {
  const [choices, setChoices] = React.useState<GraphicChoices | null>(null);
  const [mode, setMode] = React.useState<GraphicSlot>("full");
  const main = useShortsStore((s) => s.main);
  React.useEffect(() => {
    let live = true;
    Promise.all([api.projectGraphicsList(project.projectHandle), api.graphicsLibrary(), api.graphicsTemplates()])
      .then(([listed, library, templates]) => {
        if (!live) return;
        // The ones playing on the video, in its order (copies made for shorts play nowhere there).
        const video = listed.filter((g) => g.clips.length > 0).sort((a, b) => a.clips[0][1] - b.clips[0][1]);
        setChoices({ video, library, templates });
      })
      .catch((err) => live && setShortsError(err));
    return () => {
      live = false;
    };
  }, [project.projectHandle, main?.revision]);
  if (!choices) return <p className="text-meta text-studio-500">Reading the graphics…</p>;
  // Its own length unless a stretch is selected.
  const place = (from: Parameters<typeof addShortGraphic>[1]) => {
    const range = at(1_000_000);
    if (range) addShortGraphic(shortId, from, range.atUs, range.selected ? range.toUs - range.atUs : null, mode);
  };
  const group = (title: string, items: React.ReactNode[]) =>
    items.length > 0 && (
      <div>
        <p className="px-1.5 pt-1 text-meta font-medium text-studio-400">{title}</p>
        <ul>{items}</ul>
      </div>
    );
  return (
    <div className="max-h-72 overflow-y-auto">
      <Segmented<GraphicSlot>
        label="How the graphic shows"
        size="sm"
        className="w-full [&>button]:flex-1 mb-1"
        value={mode}
        onChange={setMode}
        options={MODES}
      />
      {group(
        "The video's (a copy, made 9:16)",
        choices.video.map((g) => (
          <li key={g.assetId}>
            <button type="button" className={ROW} disabled={disabled} onClick={() => place({ kind: "video", graphic: g.assetId })}>
              <span className="font-mono tabular-nums text-studio-500 shrink-0">{formatTime(g.clips[0][1])}</span>
              <span className="text-studio-200 truncate">{g.name}</span>
            </button>
          </li>
        )),
      )}
      {group(
        "Library",
        choices.library.map((item) => (
          <li key={item.id}>
            <button
              type="button"
              className={ROW}
              disabled={disabled}
              title={item.useWhen}
              onClick={() => place({ kind: "library", id: item.id })}
            >
              <span className="text-studio-200 truncate">{item.label}</span>
            </button>
          </li>
        )),
      )}
      {group(
        "Templates (change the text after)",
        choices.templates.map((t) => (
          <li key={t.name}>
            <button
              type="button"
              className={ROW}
              disabled={disabled}
              title={t.description}
              onClick={() => place({ kind: "template", template: t.name })}
            >
              <span className="text-studio-200 truncate">{t.label}</span>
            </button>
          </li>
        )),
      )}
    </div>
  );
};

/** Pictures that can go under a take: imported videos and stills, not the take itself. */
function outsideFiles(project: OpenedProject, short: Short): Asset[] {
  const take = [short.take?.camera, short.take?.voice].map((key) => key?.split(".")[0]);
  return project.assets.filter(
    (a) =>
      (a.kind === "video" || a.kind === "image") &&
      !a.missing &&
      !take.includes(a.id) &&
      a.streams.some((s) => s.kind === "picture"),
  );
}

const FromFile: React.FC<{ project: OpenedProject; short: Short; at: At; disabled: boolean }> = ({
  project,
  short,
  at,
  disabled,
}) => {
  const files = outsideFiles(project, short);
  const busy = useShortsStore((s) => s.busy);
  const importFiles = async () => {
    try {
      importForShort(await api.pickMediaFiles());
    } catch (err) {
      setShortsError(err);
    }
  };
  return (
    <>
      {files.length === 0 ? (
        <p className="text-meta text-studio-500">No outside videos or pictures in the project yet.</p>
      ) : (
        <ul className="max-h-48 overflow-y-auto">
          {files.map((file) => (
            <li key={file.id}>
              <button
                type="button"
                className={ROW}
                disabled={disabled}
                title="Put it under the take from its start (its sound ducked under the voice)"
                onClick={() => {
                  const still = file.kind === "image";
                  const range = at(still ? FILE_US : Math.min(FILE_US, file.durationUs));
                  if (!range) return;
                  // A clip can't run past its file's end.
                  const toUs = still ? range.toUs : Math.min(range.toUs, range.atUs + file.durationUs);
                  addFootage(short.id, [{ atUs: range.atUs, toUs, videoUs: 0, media: file.id }]);
                }}
              >
                <span className="text-studio-200 truncate flex-1">{file.name}</span>
                <span className="font-mono tabular-nums text-studio-500 shrink-0">
                  {file.kind === "image" ? "picture" : formatTime(file.durationUs)}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
      <Button size="sm" variant="secondary" icon={FilePlus2} className="w-full" disabled={!!busy} onClick={() => void importFiles()}>
        Import files…
      </Button>
    </>
  );
};
