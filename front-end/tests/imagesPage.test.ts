import { closePage, dom } from "./dom";
import test, { after } from "node:test";
import assert from "node:assert/strict";
import React, { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { ImagesPage } from "../src/components/media/ImagesPage";
import type { ImageLibraryView, LibraryImage } from "../src/lib/types";

after(() => {
  clearMocks();
  closePage();
});

const picture = (id: string, over: Partial<LibraryImage> = {}): LibraryImage => ({
  id,
  look: id.split("/")[0],
  file: `${id}.webp`,
  depicts: `Drawing of ${id.split("/").pop()}`,
  placement: "Centre of a card.",
  licence: { kind: "noCredit" },
  width: 1024,
  height: 1024,
  ...over,
});

const library: ImageLibraryView = {
  projectLook: "light",
  catalog: {
    looks: [
      { id: "broadsheet", description: "Ink", ground: "#f4efe4", accent: "#c8361b", styles: ["Broadsheet"] },
      { id: "light", description: "Flat", ground: "#ffffff", accent: "#0071e3", styles: ["Light"] },
      { id: "sourced", description: "Real", ground: "", accent: "", styles: [] },
    ],
    topics: [
      { id: "broadsheet/ai", look: "broadsheet", description: "AI", groups: [{ id: "robots", description: "Robots", terms: [], sourced: false, count: 1 }] },
      {
        id: "light",
        look: "light",
        description: "Everyday",
        groups: [
          { id: "devices", description: "Devices", terms: ["laptop"], sourced: false, count: 2 },
          { id: "people", description: "People", terms: [], sourced: false, count: 1 },
        ],
      },
      { id: "sourced", look: "sourced", description: "Flags", groups: [{ id: "flags", description: "Flags", terms: [], sourced: true, count: 1 }] },
    ],
    images: [
      picture("broadsheet/ai/robots/robot"),
      picture("light/devices/laptop"),
      picture("light/devices/phone"),
      picture("light/people/runner"),
      picture("sourced/flags/greece", { file: "sourced/flags/greece.svg", licence: { kind: "unknown", source: "Wikimedia" }, rights: "Check the flag's use rules." }),
    ],
  },
};

const asked: string[][] = [];
mockIPC((cmd, raw) => {
  const args = (raw ?? {}) as Record<string, unknown>;
  switch (cmd) {
    case "image_library_list":
      return library;
    case "image_library_search":
      return args.query === "flag" ? ["sourced/flags/greece"] : [];
    case "image_library_pictures": {
      const ids = args.ids as string[];
      asked.push(ids);
      return ids.map((id) => ({ id, src: `data:image/webp;base64,${id}` }));
    }
    default:
      return null;
  }
});

const settle = () => act(async () => new Promise((done) => setTimeout(done, 300)));
const text = () => dom.document.body.textContent ?? "";
const tiles = () => [...dom.document.querySelectorAll<HTMLButtonElement>("[role=listitem]")];
const groupButton = (name: string) =>
  [...dom.document.querySelectorAll<HTMLButtonElement>("button[aria-expanded]")].find((b) => b.textContent?.includes(name))!;

test("the project's look opens on its first group, a picture shows its details, and search finds sourced ones", async () => {
  dom.document.body.innerHTML = '<div id="app"></div>';
  const root = createRoot(dom.document.getElementById("app")!);
  await act(async () => root.render(React.createElement(StrictMode, null, React.createElement(ImagesPage))));
  await settle();

  const select = dom.document.querySelector("select")!;
  assert.equal(select.value, "light", "the project's look");
  assert.match(select.selectedOptions[0].textContent ?? "", /this project's/);
  // Its groups and the sourced ones; another look's are not listed.
  assert.ok(groupButton("Devices") && groupButton("People") && groupButton("Flags"));
  assert.ok(!text().includes("Robots"));
  assert.equal(groupButton("Devices").getAttribute("aria-expanded"), "true");
  assert.deepEqual(
    tiles().map((t) => t.getAttribute("aria-label")),
    ["laptop", "phone"],
  );
  assert.ok(asked.some((ids) => ids.includes("light/devices/laptop")), "the open group's pictures were fetched");
  assert.ok(!asked.flat().includes("light/people/runner"), "a closed group's were not");
  assert.equal(tiles()[0].querySelector("img")?.getAttribute("src"), "data:image/webp;base64,light/devices/laptop");

  await act(async () => tiles()[1].click());
  assert.match(text(), /Drawing of phone/);
  assert.match(text(), /1024×1024/);
  assert.match(text(), /Copy id/);

  // A search shows what it finds: the sourced flag, with its licence warning.
  const search = dom.document.querySelector<HTMLInputElement>("input[type=search]")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(dom.HTMLInputElement.prototype, "value")!.set!.call(search, "flag");
    search.dispatchEvent(new dom.Event("input", { bubbles: true }));
  });
  await settle();
  assert.deepEqual(
    tiles().map((t) => t.getAttribute("aria-label")),
    ["greece"],
  );
  await act(async () => tiles()[0].click());
  assert.match(text(), /Check the flag's use rules\. Check flags it before export\./);
  assert.match(text(), /Check rights/);

  // Another look lists its own groups.
  await act(async () => {
    Object.getOwnPropertyDescriptor(dom.HTMLInputElement.prototype, "value")!.set!.call(search, "");
    search.dispatchEvent(new dom.Event("input", { bubbles: true }));
  });
  await act(async () => {
    select.value = "broadsheet";
    select.dispatchEvent(new dom.Event("change", { bubbles: true }));
  });
  await settle();
  assert.ok(groupButton("Robots"));
  assert.ok(!text().includes("Devices"));
  await act(async () => root.unmount());
});
