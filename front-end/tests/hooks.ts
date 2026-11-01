import React from "react";

/**
 * Runs a hook or function component without a DOM: a stand-in for React's hook dispatcher
 * with the hooks the app's components use. `render` calls it again (state set by it does not
 * re-render on its own); effects whose dependencies changed run after each render, their
 * cleanups first; `unmount` runs every cleanup.
 */
export function mountHook<T>(body: () => T) {
  type Slot = { value?: unknown; deps?: unknown[]; cleanup?: void | (() => void) };
  const slots: Slot[] = [];
  let index = 0;
  let effects: (() => void)[] = [];
  const changed = (a: unknown[] | undefined, b: unknown[] | undefined) =>
    !a || !b || a.length !== b.length || a.some((x, i) => !Object.is(x, b[i]));
  const slot = () => (slots[index++] ??= {});
  const memo = <V,>(make: () => V, deps?: unknown[]) => {
    const s = slot();
    if (!("value" in s) || changed(deps, s.deps)) {
      s.value = make();
      s.deps = deps;
    }
    return s.value as V;
  };
  const effect = (run: () => void | (() => void), deps?: unknown[]) => {
    const s = slot();
    if ("value" in s && !changed(deps, s.deps)) return;
    s.value = true;
    s.deps = deps;
    effects.push(() => {
      if (typeof s.cleanup === "function") s.cleanup();
      s.cleanup = run();
    });
  };
  const dispatcher = {
    useState: <V,>(initial: V | (() => V)) => {
      const s = slot();
      if (!("value" in s)) s.value = typeof initial === "function" ? (initial as () => V)() : initial;
      return [s.value as V, (next: V | ((v: V) => V)) => {
        s.value = typeof next === "function" ? (next as (v: V) => V)(s.value as V) : next;
      }];
    },
    useRef: <V,>(initial: V) => memo(() => ({ current: initial }), []),
    useMemo: memo,
    useCallback: <F,>(fn: F, deps?: unknown[]) => memo(() => fn, deps),
    useEffect: effect,
    useLayoutEffect: effect,
    useSyncExternalStore: <V,>(_subscribe: unknown, getSnapshot: () => V) => getSnapshot(),
    useDebugValue: () => undefined,
    useContext: (context: { _currentValue: unknown }) => context._currentValue,
  };
  // Where React keeps the hook dispatcher: `H` since React 19, `ReactCurrentDispatcher` before.
  const shared = React as unknown as {
    __CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE?: { H: unknown };
    __SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED?: { ReactCurrentDispatcher: { current: unknown } };
  };
  const modern = shared.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
  const legacy = shared.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED?.ReactCurrentDispatcher;
  const internals = {
    get current(): unknown {
      return modern ? modern.H : legacy?.current;
    },
    set current(value: unknown) {
      if (modern) modern.H = value;
      else if (legacy) legacy.current = value;
    },
  };
  let result!: T;
  const render = () => {
    index = 0;
    const before = internals.current;
    internals.current = dispatcher;
    try {
      result = body();
    } finally {
      internals.current = before;
    }
    const run = effects;
    effects = [];
    run.forEach((e) => e());
    return result;
  };
  render();
  return {
    get result() {
      return result;
    },
    render,
    unmount: () => {
      for (const s of slots) if (typeof s.cleanup === "function") s.cleanup();
    },
  };
}

/** Timers run by hand: `window.setTimeout` and friends go through it once installed. */
export function fakeTimers() {
  let now = 0;
  let next = 1;
  const timers = new Map<number, { at: number; run: () => void }>();
  const g = globalThis as unknown as { window?: Record<string, unknown> };
  g.window ??= {};
  Object.assign(g.window, {
    setTimeout: (run: () => void, ms = 0) => {
      const id = next++;
      timers.set(id, { at: now + ms, run });
      return id;
    },
    clearTimeout: (id?: number) => {
      if (id !== undefined) timers.delete(id);
    },
    devicePixelRatio: 1,
  });
  return {
    pending: () => timers.size,
    /** Moves the clock on by `ms`, running timers due on the way (and ones they set). */
    advance(ms: number) {
      const until = now + ms;
      for (;;) {
        const due = [...timers.entries()].filter(([, t]) => t.at <= until).sort((a, b) => a[1].at - b[1].at)[0];
        if (!due) break;
        timers.delete(due[0]);
        now = due[1].at;
        due[1].run();
      }
      now = until;
    },
  };
}

/** Lets settled promises run their callbacks. */
export const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
