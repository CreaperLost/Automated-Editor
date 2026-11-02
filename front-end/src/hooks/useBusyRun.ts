import { useState } from "react";

/**
 * One piece of work at a time: `run` does nothing while another runs, clears the last error
 * first and keeps what fails in `error`.
 */
export function useBusyRun(onStart?: () => void) {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const run = async (label: string, work: () => Promise<void>) => {
    if (busy) return;
    setBusy(label);
    setError(undefined);
    onStart?.();
    try {
      await work();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  };
  return { busy, error, setError, run };
}
