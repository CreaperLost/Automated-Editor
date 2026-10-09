import { SilenceConfig, TranscriptGapConfig } from "./types";

export const SILENCE_PRESETS = {
  gentle: { label: "Gentle", hint: "Quiet below -48 dB, pauses of 0.6 s, 80 ms room", config: { thresholdDb: -48, minDurationMs: 600, paddingMs: 80 } },
  balanced: { label: "Balanced", hint: "Quiet below -42 dB, pauses of 0.25 s, 40 ms room", config: { thresholdDb: -42, minDurationMs: 250, paddingMs: 40 } },
  tight: { label: "Tight", hint: "Quiet below -36 dB, pauses of 0.1 s, 10 ms room", config: { thresholdDb: -36, minDurationMs: 100, paddingMs: 10 } },
  aggressive: { label: "Aggressive", hint: "Quiet below -30 dB, pauses of 0.06 s, no extra room; review soft speech", config: { thresholdDb: -30, minDurationMs: 60, paddingMs: 0 } },
} satisfies Record<string, { label: string; hint: string; config: SilenceConfig }>;

export const GAP_PRESETS = {
  gentle: { label: "Gentle", hint: "Gaps of 0.3 s, 80 ms room, saved word edges", config: { minDurationMs: 300, paddingMs: 80, refineWordEdges: false, edgeThresholdDb: -42 } },
  balanced: { label: "Balanced", hint: "Gaps of 0.15 s, 40 ms room, quiet word tails refined", config: { minDurationMs: 150, paddingMs: 40, refineWordEdges: true, edgeThresholdDb: -42 } },
  tight: { label: "Tight", hint: "Gaps of 0.06 s, 10 ms room, quiet word tails refined", config: { minDurationMs: 60, paddingMs: 10, refineWordEdges: true, edgeThresholdDb: -42 } },
  aggressive: { label: "Aggressive", hint: "Gaps of 0.02 s, no extra room, quiet word tails refined; listen before applying", config: { minDurationMs: 20, paddingMs: 0, refineWordEdges: true, edgeThresholdDb: -42 } },
} satisfies Record<string, { label: string; hint: string; config: TranscriptGapConfig }>;

export type JumpCutPreset = keyof typeof SILENCE_PRESETS;
export type JumpCutSettings = SilenceConfig & Pick<TranscriptGapConfig, "refineWordEdges" | "edgeThresholdDb">;

export function gapSettings(settings: JumpCutSettings): TranscriptGapConfig {
  return { minDurationMs: settings.minDurationMs, paddingMs: settings.paddingMs, refineWordEdges: settings.refineWordEdges, edgeThresholdDb: settings.edgeThresholdDb };
}

export function presetOf(config: JumpCutSettings, gaps: boolean): JumpCutPreset | "custom" {
  const presets = gaps ? GAP_PRESETS : SILENCE_PRESETS;
  return (Object.keys(presets) as JumpCutPreset[]).find(key => {
    const preset: JumpCutSettings = { thresholdDb: -42, ...presets[key].config };
    return preset.minDurationMs === config.minDurationMs && preset.paddingMs === config.paddingMs
      && (gaps ? preset.refineWordEdges === config.refineWordEdges && preset.edgeThresholdDb === config.edgeThresholdDb
        : preset.thresholdDb === config.thresholdDb && config.autoLevel === undefined);
  }) ?? "custom";
}
