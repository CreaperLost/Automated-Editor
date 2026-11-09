/**
 * A real page (jsdom) and the real React renderer, for tests of what the hook harness in
 * `hooks.ts` cannot show: mounting parents with their children, StrictMode, focus and how key
 * events travel (GAP-024). Import it before React DOM.
 */
import { JSDOM } from "jsdom";

const page = new JSDOM("<!doctype html><html><body></body></html>", { pretendToBeVisual: true });
const globals = globalThis as Record<string, unknown>;
for (const name of ["window", "document", "navigator", "HTMLElement", "Node", "KeyboardEvent", "Event", "getComputedStyle"]) {
  if (!(name in globals) || name === "window" || name === "document" || name === "navigator") {
    Object.defineProperty(globals, name, {
      value: name === "window" ? page.window : (page.window as unknown as Record<string, unknown>)[name],
      configurable: true,
      writable: true,
    });
  }
}
globals.IS_REACT_ACT_ENVIRONMENT = true;
// React schedules work through a MessageChannel; its ports would keep the test process open
// after the last test, so each is kept to close in `closePage`.
const NativeChannel = MessageChannel;
const channels: MessageChannel[] = [];
globals.MessageChannel = class {
  constructor() {
    const channel = new NativeChannel();
    channels.push(channel);
    return channel;
  }
};

export const dom = page.window;

/** Presses `key` on `target` as a browser does: keydown bubbling up to the window. */
export function pressKey(target: Element, key: string, init: KeyboardEventInit = {}) {
  const event = new dom.KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(event);
  return event;
}

/** Ends the page and React's channels, so the test process can exit: call it in `after`. */
export function closePage() {
  for (const channel of channels) {
    channel.port1.close();
    channel.port2.close();
  }
  page.window.close();
}
