import React, { useEffect, useState } from "react";
import { ArrowLeft, ChevronRight, Clapperboard, Loader2, Plus, Smartphone, MonitorPlay } from "lucide-react";
import { Button, Segmented } from "../ui";
import { api } from "../../lib/ipc";
import type { FacelessCreated, FacelessFormat, FacelessMade, FacelessProduction } from "../../lib/types";
import { StageRail } from "./StageRail";
import { IdeasList } from "./IdeasList";
import { SourcesPanel } from "./SourcesPanel";

const FORMAT_TEXT: Record<FacelessFormat, { title: string; detail: string }> = {
  long: { title: "Long", detail: "5 to 8 minutes, 16:9" },
  short: { title: "Short", detail: "30 to 58 seconds, 9:16" },
};

/**
 * The Faceless start page: make a long or short video from a topic, or carry on with one.
 * Every step after this starts from a click here or from your agent; nothing runs on its own.
 */
export const FacelessHome: React.FC<{
  busy: boolean;
  onBack: () => void;
  onOpen: (path: string) => void;
  onCreated: (created: FacelessCreated) => void;
  onError: (message: string) => void;
}> = ({ busy, onBack, onOpen, onCreated, onError }) => {
  const [format, setFormat] = useState<FacelessFormat>("long");
  const [topic, setTopic] = useState("");
  const [creating, setCreating] = useState(false);
  const [productions, setProductions] = useState<FacelessProduction[] | null>(null);
  const [made, setMade] = useState<FacelessMade[]>([]);

  useEffect(() => {
    let active = true;
    void Promise.all([api.facelessList(), api.facelessMade()])
      .then(([list, madeList]) => {
        if (!active) return;
        setProductions(list);
        setMade(madeList);
      })
      .catch((err) => active && onError(String(err)));
    return () => {
      active = false;
    };
  }, [onError]);

  const create = async (text = topic.trim(), links?: string[]) => {
    if (!text) return;
    setCreating(true);
    try {
      onCreated(await api.facelessCreate(text, format, links));
    } catch (err) {
      onError(String(err));
    } finally {
      setCreating(false);
    }
  };

  const shown = productions?.filter((p) => p.format === format) ?? [];
  const madeHere = made.filter((m) => m.format === format);

  return (
    <div className="flex-1 overflow-y-auto bg-studio-950">
      <div className="mx-auto w-full max-w-3xl px-6 py-10">
        <Button variant="ghost" size="sm" icon={ArrowLeft} onClick={onBack}>
          Back
        </Button>
        <div className="mt-4 flex items-center gap-3">
          <span className="h-11 w-11 inline-flex items-center justify-center rounded-control bg-accent text-white">
            <Clapperboard className="w-5 h-5" />
          </span>
          <div className="min-w-0 flex-1">
            <h1 className="font-display text-display text-studio-100">Faceless</h1>
            <p className="text-body text-studio-400">
              Videos made from a topic: research, script, narration and graphics, no camera.
            </p>
          </div>
          <Segmented
            label="Format"
            value={format}
            onChange={setFormat}
            options={[
              { value: "long", label: <><MonitorPlay className="w-3.5 h-3.5" /> Long</>, title: FORMAT_TEXT.long.detail },
              { value: "short", label: <><Smartphone className="w-3.5 h-3.5" /> Short</>, title: FORMAT_TEXT.short.detail },
            ]}
          />
        </div>

        <form
          className="mt-8 rounded-panel border border-studio-700 bg-studio-900 p-4"
          onSubmit={(event) => {
            event.preventDefault();
            void create();
          }}
        >
          <label htmlFor="faceless-topic" className="block text-label font-semibold text-studio-200">
            New {FORMAT_TEXT[format].title.toLowerCase()} video{" "}
            <span className="font-normal text-studio-500">({FORMAT_TEXT[format].detail})</span>
          </label>
          <div className="mt-2 flex gap-2">
            <input
              id="faceless-topic"
              value={topic}
              maxLength={200}
              onChange={(event) => setTopic(event.target.value)}
              placeholder="What is it about? e.g. The Codex installer that shipped adware"
              className="flex-1 min-w-0 h-9 rounded-control border border-studio-700 bg-studio-950 px-3 text-body text-studio-100 placeholder:text-studio-600 focus:outline-none focus:border-accent"
            />
            <Button type="submit" variant="primary" icon={creating ? Loader2 : Plus} disabled={busy || creating || !topic.trim()}>
              Create
            </Button>
          </div>
          <p className="mt-2 text-meta text-studio-500">
            Type a story, or pick one from the ideas below. Your agent can do both too.
          </p>
        </form>

        <IdeasList
          format={format}
          busy={busy || creating}
          onMake={(idea) => void create(idea.title, idea.urls)}
          onError={onError}
        />

        <SourcesPanel onError={onError} />

        <section className="mt-10">
          <h2 className="border-b border-studio-800 pb-2 text-label font-semibold text-studio-300">
            {FORMAT_TEXT[format].title} productions
          </h2>
          {productions === null ? (
            <p className="py-6 text-label text-studio-500">Loading…</p>
          ) : shown.length === 0 ? (
            <p className="py-6 text-label text-studio-500">None yet. Make one above.</p>
          ) : (
            <ul className="mt-2 divide-y divide-studio-800/70">
              {shown.map((production) => (
                <li key={production.path}>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => onOpen(production.path)}
                    className="group w-full flex items-center gap-3 rounded-control px-2 py-3 text-left hover:bg-studio-900"
                  >
                    <span className="min-w-0 flex-1">
                      <span className="block text-body font-medium text-studio-100 truncate">{production.topic}</span>
                      <span className="mt-1.5 block">
                        <StageRail stages={production.stages} next={production.next} compact />
                      </span>
                    </span>
                    <ChevronRight className="w-4 h-4 text-studio-600 group-hover:text-studio-300 shrink-0" />
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        {madeHere.length > 0 && (
          <section className="mt-10">
            <h2 className="border-b border-studio-800 pb-2 text-label font-semibold text-studio-300">
              Already made <span className="font-normal text-studio-500">(never the same story twice)</span>
            </h2>
            <ul className="mt-2 space-y-1">
              {madeHere.map((m) => (
                <li key={m.project} className="flex items-baseline gap-2 text-label text-studio-400">
                  <span className="truncate text-studio-200">{m.topic}</span>
                  <span className="text-meta text-studio-500 shrink-0">{m.kind}</span>
                </li>
              ))}
            </ul>
          </section>
        )}
      </div>
    </div>
  );
};
