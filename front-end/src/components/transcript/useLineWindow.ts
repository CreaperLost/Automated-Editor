import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { rowOffsets, visibleRows } from "../../lib/transcriptIndex";

/** Pixels of lines drawn above and below the visible part, so fast scrolling shows no gap. */
const OVERSCAN_PX = 600;
/** A line's height before it has been measured: one row of text and its padding. */
const FIRST_GUESS_PX = 40;

/**
 * Draws only the lines of a long list that are on screen. Lines differ in height (text wraps),
 * so each is measured once drawn and the rest are guessed from the average so far; spacers above
 * and below keep the scrollbar the size of the whole list.
 *
 * `keys` name the lines (stable across reloads, so heights carry over); `scroller` is the
 * scrolling element; each drawn line's wrapper takes `measure(key)` as its ref.
 */
export function useLineWindow(keys: string[], scroller: React.RefObject<HTMLElement | null>) {
  const heights = useRef(new Map<string, number>());
  const [measured, setMeasured] = useState(0);
  const [view, setView] = useState({ top: 0, height: 800 });

  // Heights of lines that were drawn; a line not drawn yet takes the average of those.
  const offsets = useMemo(() => {
    let sum = 0;
    let count = 0;
    for (const key of keys) {
      const h = heights.current.get(key);
      if (h !== undefined) {
        sum += h;
        count += 1;
      }
    }
    const guess = count > 0 ? sum / count : FIRST_GUESS_PX;
    return rowOffsets(keys.length, (i) => heights.current.get(keys[i]) ?? guess);
  }, [keys, measured]);

  // A hidden panel (another tab in front of it) has no height: it draws nothing until shown.
  const [first, last] =
    view.height > 0 ? visibleRows(offsets, view.top - OVERSCAN_PX, view.top + view.height + OVERSCAN_PX) : [0, 0];

  // Follow scrolling and resizing, at most once a frame.
  useEffect(() => {
    const element = scroller.current;
    if (!element) return;
    let frame = 0;
    const read = () => {
      frame = 0;
      setView((v) =>
        v.top === element.scrollTop && v.height === element.clientHeight
          ? v
          : { top: element.scrollTop, height: element.clientHeight },
      );
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(read);
    };
    read();
    element.addEventListener("scroll", schedule, { passive: true });
    const resized = new ResizeObserver(schedule);
    resized.observe(element);
    return () => {
      element.removeEventListener("scroll", schedule);
      resized.disconnect();
      if (frame) cancelAnimationFrame(frame);
    };
  }, [scroller]);

  // One observer for every drawn line: a new or changed height redraws the spacers once a frame.
  const pendingFrame = useRef(0);
  const observer = useMemo(
    () =>
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver((entries) => {
            let changed = false;
            for (const entry of entries) {
              const key = (entry.target as HTMLElement).dataset.lineKey;
              if (!key) continue;
              const height = entry.borderBoxSize?.[0]?.blockSize ?? (entry.target as HTMLElement).offsetHeight;
              // A hidden panel (another tab in front) measures 0: not a height.
              if (height > 0 && Math.abs((heights.current.get(key) ?? -1) - height) > 0.5) {
                heights.current.set(key, height);
                changed = true;
              }
            }
            if (changed && !pendingFrame.current) {
              pendingFrame.current = requestAnimationFrame(() => {
                pendingFrame.current = 0;
                setMeasured((n) => n + 1);
              });
            }
          }),
    [],
  );
  useEffect(() => () => observer?.disconnect(), [observer]);

  // One ref callback per line, kept, so React does not detach and attach it on every render; a
  // line scrolled away is no longer observed.
  const callbacks = useRef(new Map<string, (element: HTMLElement | null) => void>());
  const measure = useCallback(
    (key: string) => {
      let callback = callbacks.current.get(key);
      if (!callback) {
        let current: HTMLElement | null = null;
        callback = (element) => {
          if (current) observer?.unobserve(current);
          current = element;
          if (element && observer) {
            element.dataset.lineKey = key;
            observer.observe(element);
          }
        };
        callbacks.current.set(key, callback);
      }
      return callback;
    },
    [observer],
  );

  /** Brings line `index` into view (near the top third) when it is not drawn yet. */
  const scrollToLine = useCallback(
    (index: number) => {
      const element = scroller.current;
      if (!element || index < 0 || index >= keys.length) return;
      element.scrollTop = Math.max(0, offsets[index] - element.clientHeight / 3);
    },
    [scroller, keys.length, offsets],
  );

  return {
    first,
    last,
    /** Height of the spacer above the drawn lines, and below them. */
    above: offsets[first] ?? 0,
    below: Math.max(0, offsets[offsets.length - 1] - (offsets[last] ?? 0)),
    measure,
    scrollToLine,
  };
}
