import { closePage, dom } from "./dom";
import test, { after, beforeEach } from "node:test";
import assert from "node:assert/strict";
import React, { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { PublishDialog } from "../src/components/export/PublishDialog";
import { packageSaver } from "../src/lib/packageSaver";
import type { PublishPackage, PublishPackageView } from "../src/lib/types";

const h = React.createElement;
after(() => {
  clearMocks();
  closePage();
});

/** The fake back end: each project's saved packages, every call in order, and how saves go. */
type Backend = {
  saved: Record<string, PublishPackage>;
  calls: string[];
  failSave?: string;
  /** Holds each save until released, to see what waits on it. */
  holdSaves?: boolean;
  release: () => void;
};

function backend(): Backend {
  let waiting: (() => void)[] = [];
  const b: Backend = {
    saved: {
      A: { title: "Saved A", description: "About A", tags: ["a"], aiDisclosure: false },
      B: { title: "Saved B", description: "About B", tags: ["b"], aiDisclosure: false },
    },
    calls: [],
    release: () => {
      const all = waiting;
      waiting = [];
      all.forEach((go) => go());
    },
  };
  const view = (handle: string): PublishPackageView =>
    ({
      target: "video",
      package: b.saved[handle],
      thumbnailData: "data:image/png;base64,AAAA",
      thumbnailSize: [1280, 720],
      uploads: [],
    }) as unknown as PublishPackageView;
  mockIPC(async (cmd, raw) => {
    const args = (raw ?? {}) as Record<string, unknown>;
    const handle = String(args.projectHandle ?? "");
    switch (cmd) {
      case "publish_accounts":
        return { connected: true, source: "environment", channels: [{ id: "ch", name: "Aero" }], channel: "ch" };
      case "publish_package":
        b.calls.push(`read ${handle}`);
        return view(handle);
      case "publish_package_save": {
        const pkg = args.package as PublishPackage;
        b.calls.push(`save ${handle} "${pkg.title}"`);
        if (b.holdSaves) await new Promise<void>((go) => waiting.push(go));
        if (b.failSave) throw b.failSave;
        b.saved[handle] = pkg;
        return view(handle);
      }
      case "publish_upload":
        b.calls.push(`upload ${handle} "${b.saved[handle].title}"`);
        return { id: "u1", status: "submitted", release: { kind: "private" }, attemptedAt: new Date().toISOString() };
      case "chapters_list":
      case "project_chapters":
        return { chapters: [] };
      default:
        return null;
    }
  });
  return b;
}

/** Types `text` into the input or textarea `field` as a browser does. */
async function type(field: HTMLInputElement | HTMLTextAreaElement, text: string) {
  const proto = field instanceof dom.HTMLTextAreaElement ? dom.HTMLTextAreaElement.prototype : dom.HTMLInputElement.prototype;
  await act(async () => {
    Object.getOwnPropertyDescriptor(proto, "value")!.set!.call(field, text);
    field.dispatchEvent(new dom.Event("input", { bubbles: true }));
  });
}

const settle = () => act(async () => new Promise((done) => setTimeout(done, 20)));
const title = () => dom.document.querySelector<HTMLInputElement>("input[placeholder^='Name the subject']")!;
const button = (name: RegExp) =>
  [...dom.document.querySelectorAll<HTMLButtonElement>("button")].find((b) => name.test(b.getAttribute("aria-label") ?? b.textContent ?? ""))!;

function mount() {
  dom.document.body.innerHTML = '<div id="app"></div>';
  const root = createRoot(dom.document.getElementById("app")!);
  let closed = 0;
  const show = (handle: string | null) =>
    act(async () =>
      root.render(
        h(
          StrictMode,
          null,
          handle
            ? h(PublishDialog, {
                key: handle,
                projectHandle: handle,
                target: "video",
                label: "Video",
                file: "/tmp/a.mp4",
                onClose: () => closed++,
                onOpenAccounts: () => undefined,
              })
            : null,
        ),
      ),
    );
  return { root, show, closed: () => closed };
}

beforeEach(() => packageSaver.reset());

// GAP-015, GAP-024: the change typed just before Close is not lost.
test("closing the dialog right after typing saves the last change to its project", async () => {
  const b = backend();
  const { root, show } = mount();
  await show("A");
  await settle();
  assert.equal(title().value, "Saved A");
  await type(title(), "Typed in A");
  assert.ok(packageSaver.pending("A", "video"), "the save waits for its timer");
  await show(null);
  await settle();
  assert.deepEqual(b.calls.filter((c) => c.startsWith("save")), ['save A "Typed in A"']);
  assert.equal(b.saved.A.title, "Typed in A");
  await act(async () => root.unmount());
});

// GAP-016: Upload sends what was typed and waits for it; a failed save sends nothing.
test("Upload waits for the last change to be saved, and a failed save stops it", async () => {
  const b = backend();
  const { root, show } = mount();
  await show("A");
  await settle();
  b.holdSaves = true;
  await type(title(), "Final title");
  await act(async () => button(/^Upload$/).click());
  await settle();
  assert.ok(!b.calls.some((c) => c.startsWith("upload")), "nothing goes up while the save is out");
  assert.equal(title().disabled, true, "the fields lock during the upload");
  b.release();
  await settle();
  const order = b.calls.filter((c) => !c.startsWith("read"));
  assert.deepEqual(order, ['save A "Final title"', 'upload A "Final title"']);

  b.holdSaves = false;
  b.failSave = "disk full";
  await type(title(), "Never uploaded");
  await act(async () => button(/^Upload$/).click());
  await settle();
  assert.equal(b.calls.filter((c) => c.startsWith("upload")).length, 1, "no second upload");
  assert.match(dom.document.body.textContent ?? "", /disk full/);
  await act(async () => root.unmount());
});

// GAP-011, GAP-024: switching projects keeps each package with its own project.
test("opening another project's dialog saves A's draft to A and shows B's package", async () => {
  const b = backend();
  const { root, show } = mount();
  await show("A");
  await settle();
  await type(title(), "Draft for A");
  await show("B");
  await settle();
  assert.equal(title().value, "Saved B", "B's own package, not A's draft");
  assert.equal(b.saved.A.title, "Draft for A");
  assert.equal(b.saved.B.title, "Saved B");
  assert.ok(!b.calls.some((c) => c.startsWith("save B")), "nothing written to B");
  await act(async () => root.unmount());
});
