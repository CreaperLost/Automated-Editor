import test from "node:test";
import assert from "node:assert/strict";
import { useNoiseStore } from "../src/stores/noiseStore";
import type { NoiseScan } from "../src/lib/types";

test("choosing another sound while listening leaves the panel usable", () => {
  useNoiseStore.setState(useNoiseStore.getInitialState(), true);
  const store = useNoiseStore.getState();
  const generation = store.begin();
  assert.equal(useNoiseStore.getState().busy, true);
  store.chooseSound("second.sound0");
  // The first sound's scan finishing late is ignored, and nothing is left listening.
  store.finish(generation, { key: "first.sound0", noises: [] } as unknown as NoiseScan);
  const state = useNoiseStore.getState();
  assert.equal(state.busy, false);
  assert.equal(state.scan, null);
  assert.equal(state.key, "second.sound0");
});
