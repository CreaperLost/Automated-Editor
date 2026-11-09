import { closePage, dom, pressKey } from "./dom";
import test, { after } from "node:test";
import assert from "node:assert/strict";
import React, { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { DialogBackdrop, Segmented, modalOpen } from "../src/components/ui";

const h = React.createElement;
// The page's timers would keep the test process running.
after(closePage);

// GAP-019, GAP-024: mounted in StrictMode, with a window listener standing in for the
// timeline's shortcuts.
test("a dialog takes focus, keeps Tab and its keys, and gives focus back", async () => {
  const document = dom.document;
  document.body.innerHTML = '<button id="outside">Clip</button><div id="app"></div>';
  const outside = document.getElementById("outside") as HTMLButtonElement;
  outside.focus();
  const reached: string[] = [];
  const editor = (event: KeyboardEvent) => {
    if (event.defaultPrevented || modalOpen()) return;
    reached.push(event.key);
  };
  dom.addEventListener("keydown", editor);
  const root = createRoot(document.getElementById("app")!);
  const view = (open: boolean) =>
    h(
      StrictMode,
      null,
      open
        ? h(
            DialogBackdrop,
            { label: "Publish" },
            h("button", { id: "first" }, "One"),
            h("button", { id: "last" }, "Two"),
          )
        : null,
    );
  await act(async () => root.render(view(true)));
  assert.equal(document.activeElement?.id, "first", "focus moves into the dialog");
  assert.ok(modalOpen());
  assert.equal(document.querySelector("[role=dialog]")?.getAttribute("aria-label"), "Publish");

  const last = document.getElementById("last")!;
  last.focus();
  const tab = pressKey(last, "Tab");
  assert.ok(tab.defaultPrevented);
  assert.equal(document.activeElement?.id, "first", "Tab wraps inside the dialog");
  pressKey(document.activeElement!, "Tab", { shiftKey: true });
  assert.equal(document.activeElement?.id, "last", "Shift+Tab wraps back");

  pressKey(last, "Delete");
  pressKey(last, " ");
  assert.deepEqual(reached, [], "no key in the dialog reaches the editor");

  await act(async () => root.render(view(false)));
  assert.equal(document.activeElement?.id, "outside", "focus goes back where it was");
  assert.ok(!modalOpen());
  pressKey(outside, "Delete");
  assert.deepEqual(reached, ["Delete"], "with the dialog closed, the editor has its keys again");
  dom.removeEventListener("keydown", editor);
  await act(async () => root.unmount());
});

// GAP-028, GAP-024
test("a choice group is one Tab stop and arrows pick without reaching the editor", async () => {
  const document = dom.document;
  document.body.innerHTML = '<div id="app"></div>';
  const reached: string[] = [];
  const editor = (event: KeyboardEvent) => {
    if (!event.defaultPrevented) reached.push(event.key);
  };
  dom.addEventListener("keydown", editor);
  const root = createRoot(document.getElementById("app")!);
  let value = "a";
  const render = () =>
    h(Segmented<string>, {
      label: "Format",
      value,
      onChange: (next) => {
        value = next;
        void root.render(render());
      },
      options: [
        { value: "a", label: "A" },
        { value: "b", label: "B", disabled: true },
        { value: "c", label: "C" },
      ],
    });
  await act(async () => root.render(render()));
  const radios = Array.from(document.querySelectorAll<HTMLButtonElement>("[role=radio]"));
  assert.deepEqual(
    radios.map((r) => r.tabIndex),
    [0, -1, -1],
  );
  radios[0].focus();
  await act(async () => void pressKey(radios[0], "ArrowRight"));
  assert.equal(value, "c", "the disabled option is skipped");
  assert.equal(document.activeElement, radios[2]);
  assert.deepEqual(
    radios.map((r) => r.tabIndex),
    [-1, -1, 0],
  );
  assert.deepEqual(reached, [], "the arrow never moved the playhead");
  dom.removeEventListener("keydown", editor);
  await act(async () => root.unmount());
});
