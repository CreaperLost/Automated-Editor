/**
 * Times a switch between shorts in the Shorts workspace: from the click to the new short's
 * first frame on the canvas. Each stage is kept in `window.__shortSwitches` (the last 50
 * switches) and logged in development.
 */
type Switch = { shortId: string; startedAt: number; stages: Record<string, number> };

let current: Switch | null = null;

function history(): Switch[] {
  const w = window as unknown as { __shortSwitches?: Switch[] };
  return (w.__shortSwitches ??= []);
}

export function switchStarted(shortId: string) {
  current = { shortId, startedAt: performance.now(), stages: {} };
  const all = history();
  all.push(current);
  if (all.length > 50) all.shift();
}

/** Notes when `stage` of the switch to `shortId` came; the first time only. */
export function switchStage(stage: string, shortId?: string) {
  if (!current || (shortId && shortId !== current.shortId) || stage in current.stages) return;
  current.stages[stage] = Math.round((performance.now() - current.startedAt) * 10) / 10;
  if (import.meta.env.DEV) console.debug(`[ShortSwitch] ${current.shortId} ${stage} at ${current.stages[stage]} ms`);
}

/** The short being switched to, while its first frame is on its way. */
export function switchingTo(): string | undefined {
  return current && !("first frame" in current.stages) ? current.shortId : undefined;
}
