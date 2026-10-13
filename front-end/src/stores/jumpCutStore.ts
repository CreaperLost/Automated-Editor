import { create } from "zustand";
import { useProjectStore } from "./projectStore";
import { SilenceConfig, SilenceBlock, SilenceDetectionResult, TranscriptDependency, TranscriptGapConfig } from "../lib/types";
import { SILENCE_PRESETS, GAP_PRESETS } from "../lib/jumpCutConfig";

export type JumpCutPass = "silence" | "nonSpeech";
export interface JumpCutSnapshot {
  projectHandle: string;
  revision: number;
  captionsVersion: number;
  transcriptDependencies: TranscriptDependency[];
}
interface Review {
  generation: number;
  busy: boolean;
  blocks: SilenceBlock[];
  snapshot: JumpCutSnapshot | null;
  diagnostics: string[];
  thresholds: SilenceDetectionResult["thresholds"];
  error: string | null;
  notice: string | null;
  sourceMode: "one" | "two";
  chosenTrack?: string;
  secondTrack?: string;
}
const emptyReview = (): Review => ({ generation: 0, busy: false, blocks: [], snapshot: null, diagnostics: [], thresholds: [], error: null, notice: null, sourceMode: "one" });
const clearReview = (review: Review, notice: string | null = null): Review => ({ ...review, generation: review.generation + 1, busy: false, blocks: [], snapshot: null, diagnostics: [], thresholds: [], error: null, notice });

interface JumpCutStore {
  activePass: JumpCutPass;
  silenceConfig: SilenceConfig;
  gapConfig: TranscriptGapConfig;
  reviews: Record<JumpCutPass, Review>;
  setActivePass: (pass: JumpCutPass) => void;
  configureSilence: (config: SilenceConfig) => void;
  configureGaps: (config: TranscriptGapConfig) => void;
  chooseSources: (pass: JumpCutPass, sources: Partial<Pick<Review, "sourceMode" | "chosenTrack" | "secondTrack">>) => void;
  beginScan: (pass: JumpCutPass) => number;
  finishScan: (pass: JumpCutPass, generation: number, result: SilenceDetectionResult, snapshot: JumpCutSnapshot) => void;
  failScan: (pass: JumpCutPass, generation: number, error: string) => void;
  select: (pass: JumpCutPass, id: string | null, selected?: boolean) => void;
  invalidate: (notice: string) => void;
}

export const useJumpCutStore = create<JumpCutStore>((set, get) => ({
  activePass: "silence",
  silenceConfig: { ...SILENCE_PRESETS.balanced.config },
  gapConfig: { ...GAP_PRESETS.balanced.config },
  reviews: { silence: emptyReview(), nonSpeech: emptyReview() },
  setActivePass: activePass => set({ activePass }),
  configureSilence: silenceConfig => set(state => ({ silenceConfig, reviews: { ...state.reviews, silence: clearReview(state.reviews.silence) } })),
  configureGaps: gapConfig => set(state => ({ gapConfig, reviews: { ...state.reviews, nonSpeech: clearReview(state.reviews.nonSpeech) } })),
  chooseSources: (pass, sources) => set(state => ({ reviews: { ...state.reviews, [pass]: { ...clearReview(state.reviews[pass]), ...sources } } })),
  beginScan: pass => {
    const next = clearReview(get().reviews[pass]);
    set(state => ({ reviews: { ...state.reviews, [pass]: { ...next, busy: true } } }));
    return next.generation;
  },
  finishScan: (pass, generation, result, snapshot) => {
    const project = useProjectStore.getState();
    if (get().reviews[pass].generation !== generation || project.openedProject?.projectHandle !== snapshot.projectHandle
        || project.openedProject.revision !== snapshot.revision || project.captionsVersion !== snapshot.captionsVersion) return;
    set(state => ({ reviews: { ...state.reviews, [pass]: { ...state.reviews[pass], busy: false, blocks: result.suggestions, snapshot, diagnostics: result.diagnostics, thresholds: result.thresholds, error: null } } }));
  },
  failScan: (pass, generation, error) => {
    if (get().reviews[pass].generation !== generation) return;
    set(state => ({ reviews: { ...state.reviews, [pass]: { ...state.reviews[pass], busy: false, error } } }));
  },
  select: (pass, id, selected) => set(state => ({ reviews: { ...state.reviews, [pass]: {
    ...state.reviews[pass], blocks: state.reviews[pass].blocks.map(block => id === null || block.id === id ? { ...block, selected: selected ?? !block.selected } : block),
  } } })),
  invalidate: notice => set(state => ({ reviews: {
    silence: clearReview(state.reviews.silence, state.reviews.silence.blocks.length || state.reviews.silence.busy ? notice : null),
    nonSpeech: clearReview(state.reviews.nonSpeech, state.reviews.nonSpeech.blocks.length || state.reviews.nonSpeech.busy ? notice : null),
  } })),
}));

// Closing the dialog retains each review. Changes to its inputs invalidate both completed
// results and in-flight generations, including changes broadcast by another window.
useProjectStore.subscribe((state, previous) => {
  const project = state.openedProject;
  const before = previous.openedProject;
  if (project?.projectHandle !== before?.projectHandle || project?.shortView !== before?.shortView || project?.revision !== before?.revision) {
    useJumpCutStore.getState().invalidate("The timeline changed. Find suggestions again.");
  } else if (state.captionsVersion !== previous.captionsVersion) {
    useJumpCutStore.getState().invalidate("The transcript changed. Find suggestions again.");
  }
});
