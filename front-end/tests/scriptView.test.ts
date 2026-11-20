import { closePage, dom } from "./dom";
import test, { after } from "node:test";
import assert from "node:assert/strict";
import React, { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { ScriptDetails } from "../src/components/faceless/ScriptView";
import type { FacelessScriptView } from "../src/lib/types";

const h = React.createElement;
after(closePage);

const scene = (narration: string, factIds: string[]) => ({ narration, factIds, analysis: false, onScreen: "", source: "" });

function view(over: Partial<FacelessScriptView> = {}): FacelessScriptView {
  return {
    script: {
      title: "Apple's robot walks",
      titleOptions: [],
      titleFactIds: [],
      hooks: [],
      spine: "",
      firstFrame: "",
      scenes: [scene("It walks at 4 km per hour.", ["f1"]), scene("Nobody tested it outside.", ["f1", "f9"])],
      leftOut: [],
    },
    check: { errors: [], notes: ["Scene 2 is short."], words: 120, estimatedSeconds: 44 },
    targetSeconds: [30, 58],
    facts: [{ id: "f1", statement: "It walks at 4 km per hour", quote: "the robot walks at 4 km per hour", url: "https://example.com/a", primary: true }],
    missingFacts: ["f9"],
    review: {
      script: "abc",
      findings: [
        { review: "fairness", claim: "Nobody tested it outside.", problem: "overstated", severity: "blocker", why: "The paper says only indoor tests.", change: "It was tested indoors only.", resolved: false },
        { review: "attention", claim: "It walks", problem: "", severity: "minor", why: "", change: "", resolved: true },
      ],
    },
    reviewCurrent: true,
    ...over,
  };
}

// CL-13 (FCL-3): mounted in StrictMode.
test("the Script panel shows length, cited facts, findings and whether this revision is accepted", async () => {
  const document = dom.document;
  document.body.innerHTML = '<div id="app"></div>';
  const root = createRoot(document.getElementById("app")!);
  const show = async (v: FacelessScriptView) => act(async () => root.render(h(StrictMode, null, h(ScriptDetails, { view: v }))));
  await show(view());
  const text = () => document.body.textContent ?? "";
  assert.match(text(), /2 scenes · 120 words · about 44 s \(fits 30 s to 58 s\)/);
  assert.match(text(), /Not accepted: 1 blocker open\./);
  assert.match(text(), /f9 is cited but no longer in the research\./);
  assert.match(text(), /Scene 2 is short\./);
  assert.match(text(), /Instead: “It was tested indoors only\.”/);
  assert.ok(!text().includes("the robot walks at 4 km"), "quotes stay folded");

  const scenes = document.querySelectorAll("[aria-label=Scenes] > li > button");
  assert.equal(scenes.length, 2);
  await act(async () => (scenes[0] as HTMLButtonElement).click());
  assert.equal(scenes[0].getAttribute("aria-expanded"), "true");
  assert.match(text(), /f1: It walks at 4 km per hour/);
  assert.match(text(), /“the robot walks at 4 km per hour”/);
  assert.match(text(), /Primary source · https:\/\/example.com\/a/);

  await show(view({ reviewCurrent: false }));
  assert.match(text(), /Changed since the review/);
  assert.equal(document.querySelector("[aria-label='Review findings']"), null, "an old review's findings are not shown");

  await show(view({ acceptedAt: "2026-11-20T10:00:00Z", check: { errors: [], notes: [], words: 200, estimatedSeconds: 75 } }));
  assert.match(text(), /This revision was accepted/);
  assert.match(text(), /\(outside 30 s to 58 s\)/);
  await act(async () => root.unmount());
});
