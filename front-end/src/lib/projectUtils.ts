export function editedToSourceUs(
  retained: { startUs: number; endUs: number }[],
  editedUs: number,
): number | null {
  let accumulated = 0;
  for (const interval of retained) {
    const duration = interval.endUs - interval.startUs;
    if (editedUs < accumulated + duration) {
      return interval.startUs + (editedUs - accumulated);
    }
    accumulated += duration;
  }
  return null;
}
