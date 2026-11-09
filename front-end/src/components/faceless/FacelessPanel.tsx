import React, { useEffect, useState } from "react";
import { Bot, Clapperboard, RefreshCw } from "lucide-react";
import { Badge, IconButton, PanelHeader } from "../ui";
import { useFacelessStore } from "../../stores/facelessStore";
import { useProjectStore } from "../../stores/projectStore";
import type { FacelessStageId } from "../../lib/types";
import { StageRail } from "./StageRail";
import { ResearchView } from "./ResearchView";
import { VoiceView } from "./VoiceView";
import { VisualsView } from "./VisualsView";

/** What each stage does, who does it, and the roadmap item that builds it. */
const STAGE_TEXT: Record<FacelessStageId, { what: string; who: string; item: string }> = {
  research: {
    what: "Sources fetched and read; every fact keeps an exact quote the app finds on the page, or it is refused.",
    who: "Your agent searches; the app checks",
    item: "FCL-2",
  },
  script: {
    what: "Scenes written from the checked facts, each citing them; numbers must appear in the quotes.",
    who: "Your agent",
    item: "FCL-3",
  },
  review: {
    what: "Attention, cold-reader and fairness reviews; a blocker sends the script back.",
    who: "Your agent",
    item: "FCL-3",
  },
  voice: {
    what: "Each scene read by the channel's voice, put on the timeline as speech and transcribed; names and numbers heard differently are listed to listen to.",
    who: "The app, on your click or your agent's call (OpenRouter key)",
    item: "FCL-4",
  },
  visuals: {
    what: "Graphics on the timeline in the Broadsheet style, each landing on a spoken word; gaps, pacing, reading time and repeated kinds checked.",
    who: "Your agent places them; the app checks",
    item: "FCL-5, FCL-6",
  },
  package: {
    what: "Title, description, tags, chapters and the thumbnail.",
    who: "Your agent writes it, or you on the Export page; done with a title, description, tags and a thumbnail",
    item: "FCL-8, PUB-1",
  },
  publish: {
    what: "Upload through Zernio: private, scheduled or draft. Never public without your click.",
    who: "The app, on your click",
    item: "FCL-9, PUB-2",
  },
};

/** The open faceless production: its topic and format, and each stage with where it stands. */
export const FacelessPanel: React.FC = () => {
  const path = useProjectStore((s) => s.projectPath);
  const revision = useProjectStore((s) => s.openedProject?.revision);
  const { production, refresh } = useFacelessStore();
  // Agent calls write stage files outside the edit: read again every few seconds while shown.
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const timer = window.setInterval(() => setTick((t) => t + 1), 5000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    void refresh(path ?? null);
  }, [path, revision, refresh, tick]);

  if (!production) {
    return (
      <div className="h-full flex items-center justify-center p-6 text-label text-studio-500 bg-studio-900">
        This project is not a faceless video.
      </div>
    );
  }

  return (
    <section className="h-full flex flex-col min-h-0 bg-studio-900" aria-label="Faceless production">
      <PanelHeader
        title={production.topic}
        icon={Clapperboard}
        detail={production.format === "long" ? "Long · 16:9" : "Short · 9:16"}
        actions={
          <IconButton icon={RefreshCw} size="sm" label="Read the stages again" onClick={() => void refresh(path ?? null)} />
        }
      />
      <div className="px-3 py-2 border-b border-studio-800">
        <StageRail stages={production.stages} next={production.next} />
      </div>
      <ol className="flex-1 min-h-0 overflow-y-auto divide-y divide-studio-800/70">
        {production.stages.map((stage) => {
          const text = STAGE_TEXT[stage.id];
          return (
            <li key={stage.id} className="px-3 py-2.5">
              <div className="flex items-center gap-2">
                <span className="text-label font-semibold text-studio-100">{stage.label}</span>
                {stage.id === production.next && <Badge tone="accent">Next</Badge>}
                {stage.state === "stale" && <Badge tone="suggest">Out of date</Badge>}
                {stage.state === "done" && <Badge tone="success">Done</Badge>}
                <span className="ml-auto text-meta text-studio-600">{text.item}</span>
              </div>
              <p className="mt-1 text-meta text-studio-400">{text.what}</p>
              <p className="mt-0.5 flex items-center gap-1 text-meta text-studio-500">
                <Bot className="w-3 h-3" aria-hidden /> {text.who}
              </p>
              {stage.id === "research" && <ResearchView refreshKey={`${revision}-${production.stages[0].state}-${tick}`} />}
              {stage.id === "visuals" && (
                <VisualsView
                  refreshKey={`${revision}-${stage.state}-${tick}`}
                  ready={production.stages.find((s) => s.id === "voice")?.state === "done"}
                  onChange={() => void refresh(path ?? null)}
                />
              )}
              {stage.id === "voice" && (
                <VoiceView
                  refreshKey={`${revision}-${stage.state}-${tick}`}
                  ready={production.stages.find((s) => s.id === "review")?.state === "done"}
                  stale={stage.state === "stale"}
                  onChange={() => void refresh(path ?? null)}
                />
              )}
            </li>
          );
        })}
      </ol>
      <p className="shrink-0 px-3 py-2 border-t border-studio-800 text-meta text-studio-500">
        Nothing here runs by itself. Ask your agent (Codex, Claude Code) to work on the next stage. Package
        and Publish are in the Export dialog, for every video.
      </p>
    </section>
  );
};
