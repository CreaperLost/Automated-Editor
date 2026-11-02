/** A time as minutes and seconds with `decimals` places: `1:05.3`. Never rounds up to `:60`. */
export function formatTime(us: number, decimals = 1): string {
  const scale = 10 ** decimals;
  const units = Math.round((Math.max(0, us) / 1_000_000) * scale);
  const minutes = Math.floor(units / (60 * scale));
  const seconds = (units - minutes * 60 * scale) / scale;
  return `${minutes}:${seconds.toFixed(decimals).padStart(decimals + 3, "0")}`;
}

/** What to tell the user about a failure: its own words, else `fallback`. */
export function errorMessage(err: unknown, fallback = "Something went wrong."): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return fallback;
}
