import { PreviewFill } from "../../lib/types";

/**
 * When the window draws the preview under the page (Windows), the page has to be see-through
 * over it: every element from the preview up to <html> stops painting its background. The
 * window paints those backgrounds instead, from the fills returned here, so the page looks
 * the same.
 */

interface Cleared {
  /** The inline background colour to put back. */
  value: string;
  priority: string;
  /** The background it painted, RGBA 0 to 1. */
  color: [number, number, number, number];
}

const cleared = new Map<HTMLElement, Cleared>();
const colors = new Map<string, [number, number, number, number]>();
let probe: CanvasRenderingContext2D | null | undefined;

/** Any CSS colour as sRGB RGBA, 0 to 1: drawn on one canvas pixel and read back. */
function rgba(css: string): [number, number, number, number] {
  const known = colors.get(css);
  if (known) return known;
  if (probe === undefined) {
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 1;
    probe = canvas.getContext("2d", { willReadFrequently: true });
  }
  let color: [number, number, number, number] = [0, 0, 0, 0];
  if (probe) {
    probe.clearRect(0, 0, 1, 1);
    probe.fillStyle = "rgba(0, 0, 0, 0)";
    probe.fillStyle = css;
    probe.fillRect(0, 0, 1, 1);
    // Image data is not premultiplied: these are the colour and its alpha as given.
    const [r, g, b, a] = probe.getImageData(0, 0, 1, 1).data;
    color = a === 0 ? [0, 0, 0, 0] : [r / 255, g / 255, b / 255, a / 255];
  }
  colors.set(css, color);
  return color;
}

function restore(element: HTMLElement, saved: Cleared) {
  element.style.setProperty("background-color", saved.value, saved.priority);
  cleared.delete(element);
}

/**
 * Clears the backgrounds from `host` up to <html> (putting back those of elements no longer on
 * that path) and returns them, outermost first, in the page's CSS pixels.
 */
export function seeThroughPath(host: HTMLElement): PreviewFill[] {
  const path: HTMLElement[] = [];
  for (let element: HTMLElement | null = host; element; element = element.parentElement) {
    path.push(element);
  }
  for (const [element, saved] of [...cleared]) {
    if (!path.includes(element)) restore(element, saved);
  }
  const fills: PreviewFill[] = [];
  for (const element of path.reverse()) {
    let saved = cleared.get(element);
    if (!saved) {
      saved = {
        value: element.style.getPropertyValue("background-color"),
        priority: element.style.getPropertyPriority("background-color"),
        color: rgba(getComputedStyle(element).backgroundColor),
      };
      cleared.set(element, saved);
      element.style.setProperty("background-color", "transparent", "important");
    }
    if (saved.color[3] <= 0) continue;
    const whole = element === document.documentElement || element === document.body;
    const box = element.getBoundingClientRect();
    fills.push({
      rect: whole ? [0, 0, window.innerWidth, window.innerHeight] : [box.left, box.top, box.width, box.height],
      color: saved.color,
      radius: whole ? 0 : parseFloat(getComputedStyle(element).borderTopLeftRadius) || 0,
    });
  }
  return fills;
}

/** Puts every cleared background back: the page paints itself again. */
export function restoreSeeThrough() {
  for (const [element, saved] of [...cleared]) restore(element, saved);
}
