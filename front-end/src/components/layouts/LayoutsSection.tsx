import React, { useEffect, useState } from "react";
import { LayoutGrid, Sparkles, Star, Trash2, X } from "lucide-react";
import { InspectorSection, RangeRow } from "../inspector/InspectorSection";
import { Button, Field, Segmented, Switch, cn } from "../ui";
import { LayoutEditor } from "./LayoutEditor";
import { LayoutThumb } from "./LayoutThumb";
import { useProjectStore } from "../../stores/projectStore";
import {
  changeLayouts,
  loadFavourites,
  pickLayouts,
  removeFavourite,
  saveFavourite,
  selectSection,
  setLayoutsError,
  switchAt,
  useLayoutsStore,
} from "../../stores/layoutsStore";
import { formatTimeUs } from "../../hooks/useTimeline";
import { useDebouncedSave } from "../../hooks/useDebouncedSave";
import {
  BUILT_IN_LAYOUTS,
  DEFAULT_LAYOUT_TRACK,
  sameLayout,
  type Layout,
  type LayoutSection,
  type LayoutTransition,
  type PickSettings,
} from "../../lib/types";

/** How long the inspector waits after a slider moves before saving it (one undo step). */
const SAVE_MS = 200;

/**
 * A change that waits a moment, so a dragged slider saves once. One still waiting when the
 * section closes (another Inspector page, another workspace) is saved then, not lost.
 */
function useLater() {
  const { schedule } = useDebouncedSave(SAVE_MS);
  return (work: () => void) => schedule(() => work());
}

const Tile: React.FC<{
  name: string;
  layout: Layout;
  portrait: boolean;
  active: boolean;
  onPick: () => void;
  onRemove?: () => void;
  title: string;
}> = ({ name, layout, portrait, active, onPick, onRemove, title }) => (
  <div className="relative group">
    <button
      type="button"
      onClick={onPick}
      aria-pressed={active}
      title={title}
      className={cn(
        "w-full flex flex-col items-center gap-1 rounded-control border p-1.5 transition-colors",
        active ? "border-accent-hover bg-accent/10" : "border-studio-700 hover:border-studio-500 bg-studio-850/60",
      )}
    >
      <LayoutThumb layout={layout} portrait={portrait} className={portrait ? "h-10" : "w-full"} />
      <span className="text-meta text-studio-300 truncate max-w-full">{name}</span>
    </button>
    {onRemove && (
      <button
        type="button"
        aria-label={`Forget the favourite ${name}`}
        title="Forget this favourite"
        onClick={onRemove}
        className="absolute -top-1.5 -right-1.5 hidden group-hover:inline-flex h-5 w-5 items-center justify-center rounded-full bg-studio-800 border border-studio-600 text-studio-400 hover:text-danger-fg"
      >
        <X className="w-3 h-3" />
      </button>
    )}
  </div>
);

/** Where a section plays, as text. */
function placeOf(section: LayoutSection): string {
  const ranges = section.editedRanges ?? [];
  if (ranges.length === 0) return "cut from the timeline";
  const start = ranges[0].startUs;
  const end = ranges[ranges.length - 1].endUs;
  return `${formatTimeUs(start)} – ${formatTimeUs(end)}`;
}

/// Layouts of the timeline shown (the video, or the short in view): the base, the sections
/// that change it, the layouts to pick from, and picking them from the recording.
export const LayoutsSection: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const timelineSelection = useProjectStore((s) => s.timelineSelection);
  const { selectedId, favourites, busy, error, notes } = useLayoutsStore();
  const later = useLater();
  const [favouriteName, setFavouriteName] = useState<string | null>(null);
  const [showPick, setShowPick] = useState(false);
  // A slider's change shows here at once and is saved a moment later.
  const [draft, setDraft] = useState<{ key: string; layout: Layout } | null>(null);
  const [pickDraft, setPickDraft] = useState<PickSettings | null>(null);

  useEffect(() => {
    void loadFavourites();
  }, []);
  const track = project?.layouts ?? DEFAULT_LAYOUT_TRACK;
  const selected = track.sections.find((s) => s.id === selectedId) ?? null;
  // A section gone (undo, another edit) is no longer selected.
  useEffect(() => {
    if (selectedId && !selected) selectSection(null);
  }, [selectedId, selected]);
  useEffect(() => {
    setDraft(null);
    setPickDraft(null);
  }, [project?.revision]);
  if (!project) return null;

  const portrait = project.shortView ? true : project.layout?.aspectRatio === "9:16";
  const editingKey = selected ? `section:${selected.id}` : "base";
  const shownLayout =
    draft?.key === editingKey ? draft.layout : selected ? (selected.layout ?? track.base) : track.base;

  const editLayout = (layout: Layout) => {
    setDraft({ key: editingKey, layout });
    later(() =>
      selected
        ? changeLayouts({ op: "update", id: selected.id, layout, name: null }, "Changing the layout")
        : changeLayouts({ op: "base", layout }, "Changing the base layout"),
    );
  };

  /** A tile picked: the selected section takes it, or a new one over the selection or from the playhead. */
  const applyTile = (layout: Layout, name: string) => {
    if (selected) {
      changeLayouts({ op: "update", id: selected.id, layout, name }, "Changing the layout");
      return;
    }
    if (timelineSelection) {
      changeLayouts(
        { op: "add", startUs: timelineSelection.startUs, endUs: timelineSelection.endUs, layout, name },
        "Adding a layout section",
      );
      return;
    }
    switchAt(useProjectStore.getState().currentTimeUs, layout, name);
  };

  const tiles = [
    ...BUILT_IN_LAYOUTS.map((b) => ({ name: b.name, layout: b.layout, id: null as string | null })),
    ...favourites.map((f) => ({ name: f.name, layout: f.layout, id: f.id })),
  ];
  const pick = pickDraft ?? track.pick;
  const setPick = (patch: Partial<PickSettings>) => {
    const next = { ...pick, ...patch };
    setPickDraft(next);
    later(() => changeLayouts({ op: "settings", pick: next }, "Changing how layouts are picked"));
  };
  const autoCount = track.sections.filter((s) => s.auto).length;

  return (
    <InspectorSection id="layouts" title="Layouts" icon={LayoutGrid}>
      <p className="text-meta text-studio-500">
        {selected
          ? "Pick a layout for the selected section, or change it below."
          : timelineSelection
            ? "Pick a layout for the selected range of the timeline."
            : "Pick a layout to use it from the playhead to the next change. Select a range on the timeline to use it there."}
      </p>
      {error && (
        <p role="alert" className="text-meta text-danger-fg bg-danger/10 border border-danger/30 rounded p-2">
          {error}{" "}
          <button type="button" className="underline" onClick={() => setLayoutsError(undefined)}>
            Dismiss
          </button>
        </p>
      )}

      <div className={cn("grid gap-1.5", portrait ? "grid-cols-5" : "grid-cols-3")}>
        {tiles.map((tile) => (
          <Tile
            key={tile.id ?? tile.name}
            name={tile.name}
            layout={tile.layout}
            portrait={portrait}
            active={sameLayout(tile.layout, shownLayout)}
            onPick={() => applyTile(tile.layout, tile.name)}
            onRemove={tile.id ? () => void removeFavourite(tile.id!) : undefined}
            title={`${tile.name}: ${selected ? "use it for the selected section" : timelineSelection ? "use it over the selection" : "use it from the playhead"}`}
          />
        ))}
      </div>

      <div className="rounded-control border border-studio-800 p-2.5 space-y-3">
        <div className="flex items-center justify-between gap-2">
          <span className="text-label font-medium text-studio-200">
            {selected ? (
              <>
                Section · <span className="font-mono text-meta text-studio-400">{placeOf(selected)}</span>
              </>
            ) : (
              "Base layout (everywhere else)"
            )}
          </span>
          {selected && (
            <Button size="sm" variant="ghost" onClick={() => selectSection(null)}>
              Base
            </Button>
          )}
        </div>
        {selected && !selected.layout ? (
          <p className="text-meta text-studio-400">
            Shows the base layout here, and keeps picked sections away.
          </p>
        ) : (
          <LayoutEditor layout={shownLayout} portrait={portrait} onChange={editLayout} />
        )}

        {selected && (
          <>
            <Field label="Change">
              <Segmented<LayoutTransition>
                label="How it changes in and out"
                size="sm"
                className="w-full"
                value={selected.transition}
                onChange={(transition) => changeLayouts({ op: "update", id: selected.id, transition })}
                options={[
                  { value: "auto", label: "Auto", title: "A cut between full-frame shots and on cuts, a short move otherwise" },
                  { value: "cut", label: "Cut" },
                  { value: "move", label: "Move" },
                ]}
              />
            </Field>
            <div className="flex flex-wrap gap-1.5">
              <Button
                size="sm"
                variant="ghost"
                onClick={() =>
                  selected.layout
                    ? changeLayouts({ op: "update", id: selected.id, base: true })
                    : changeLayouts({ op: "update", id: selected.id, layout: track.base })
                }
                title={selected.layout ? "Show the base here and keep picked sections away" : "Give this section its own layout"}
              >
                {selected.layout ? "Keep the base here" : "Own layout"}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                icon={Trash2}
                onClick={() => {
                  changeLayouts({ op: "remove", ids: [selected.id] }, "Removing a section");
                  selectSection(null);
                }}
              >
                Remove
              </Button>
              <span className="text-meta text-studio-500 self-center">
                {selected.auto ? `Picked (${selected.rule ?? "rule"})` : "Set by hand"}
              </span>
            </div>
          </>
        )}

        {(!selected || selected.layout) &&
          (favouriteName === null ? (
            <Button size="sm" variant="ghost" icon={Star} onClick={() => setFavouriteName(selected?.name ?? "")}>
              Save as favourite
            </Button>
          ) : (
            <form
              className="flex gap-1.5"
              onSubmit={(event) => {
                event.preventDefault();
                void saveFavourite(favouriteName, shownLayout).then((ok) => ok && setFavouriteName(null));
              }}
            >
              <input
                autoFocus
                aria-label="Favourite name"
                placeholder="Name"
                value={favouriteName}
                onChange={(e) => setFavouriteName(e.target.value)}
                className="ui-field flex-1 min-w-0"
              />
              <Button size="sm" type="submit" disabled={!favouriteName.trim()}>
                Save
              </Button>
              <Button size="sm" variant="ghost" type="button" onClick={() => setFavouriteName(null)}>
                Cancel
              </Button>
            </form>
          ))}
      </div>

      {!selected && (
        <RangeRow
          label="Move length"
          value={track.transitionMs}
          min={0}
          max={1000}
          step={50}
          unit="ms"
          onChange={(transitionMs) => later(() => changeLayouts({ op: "settings", transitionMs }))}
        />
      )}

      <div className="space-y-2">
        <div className="flex gap-1.5">
          <Button
            size="sm"
            variant="secondary"
            icon={Sparkles}
            className="flex-1"
            disabled={!!busy}
            onClick={() => pickLayouts(pickDraft ?? undefined)}
            title="From the recording, no AI. Sections you set yourself stay."
          >
            {busy === "Picking layouts" ? "Picking…" : "Pick from the recording"}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setShowPick((v) => !v)} aria-expanded={showPick}>
            Rules
          </Button>
        </div>
        {autoCount > 0 && (
          <div className="flex items-center justify-between text-meta text-studio-400">
            <span>
              {autoCount} picked section{autoCount === 1 ? "" : "s"}
            </span>
            <Button size="sm" variant="ghost" onClick={() => changeLayouts({ op: "clearAll", autoOnly: true }, "Clearing picks")}>
              Clear picks
            </Button>
          </div>
        )}
        {notes.map((note) => (
          <p key={note} className="text-meta text-studio-500">
            {note}
          </p>
        ))}
        {showPick && (
          <div className="space-y-3 rounded-control border border-studio-800 p-2.5">
            <Switch checked={pick.rest} onChange={(rest) => setPick({ rest })} label="Camera while the mouse rests as I talk" />
            {pick.rest && (
              <>
                <RangeRow label="Mouse rest" value={pick.idleMs / 1000} min={0} max={60} step={0.5} unit="s" onChange={(s) => setPick({ idleMs: Math.round(s * 1000) })} />
                <RangeRow label="Shortest stretch" value={pick.minRestMs} min={500} max={15000} step={250} unit="ms" onChange={(minRestMs) => setPick({ minRestMs })} />
                <RangeRow label="Camera size" value={pick.restSizePct} min={40} max={100} unit="%" onChange={(restSizePct) => setPick({ restSizePct })} />
                <Switch checked={pick.requireSpeech} onChange={(requireSpeech) => setPick({ requireSpeech })} label="Only while I'm talking" />
                {pick.requireSpeech && !project.shortView && (
                  <>
                    <RangeRow label="Speech level" value={pick.speechThresholdDb} min={-70} max={-10} unit=" dB" onChange={(speechThresholdDb) => setPick({ speechThresholdDb })} />
                    <RangeRow label="Pause allowed" value={pick.pauseToleranceMs} min={100} max={3000} step={50} unit="ms" onChange={(pauseToleranceMs) => setPick({ pauseToleranceMs })} />
                  </>
                )}
                <Switch
                  checked={pick.cursorMovesAreActivity}
                  onChange={(cursorMovesAreActivity) => setPick({ cursorMovesAreActivity })}
                  label="Mouse movement counts as activity"
                />
              </>
            )}
            <Switch checked={pick.busy} onChange={(busy) => setPick({ busy })} label="Screen only while I click and type a lot" />
            <Switch checked={pick.hook} onChange={(hook) => setPick({ hook })} label="Camera for the opening line" />
          </div>
        )}
      </div>
    </InspectorSection>
  );
};
