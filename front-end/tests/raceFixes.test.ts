import test from "node:test";
import assert from "node:assert/strict";
import { settle } from "./hooks";
import { api } from "../src/lib/ipc";
import { useFacelessStore } from "../src/stores/facelessStore";
import { useProjectStore } from "../src/stores/projectStore";
import { updateAudio } from "../src/lib/audioQueue";
import type { AudioSettings, FacelessProduction, OpenedProject } from "../src/lib/types";

/** Answers handed out by hand, in the order the test chooses. */
function deferred<T>() {
  const pending: { resolve: (value: T) => void; reject: (err: Error) => void }[] = [];
  const call = () => new Promise<T>((resolve, reject) => pending.push({ resolve, reject }));
  return { call, pending };
}

// GAP-003
test("a late faceless status never replaces a newer project's or a cleared state", async () => {
  const answers = deferred<FacelessProduction | null>();
  api.facelessStatus = answers.call;
  const production = (topic: string) => ({ topic }) as unknown as FacelessProduction;
  const store = useFacelessStore.getState();

  const a = store.refresh("A");
  const b = store.refresh("B");
  answers.pending[1].resolve(production("B"));
  answers.pending[0].resolve(production("A"));
  await Promise.all([a, b]);
  assert.equal(useFacelessStore.getState().path, "B");
  assert.equal(useFacelessStore.getState().production?.topic, "B");

  const c = store.refresh("C");
  const cleared = store.refresh(null);
  answers.pending[2].resolve(production("C"));
  await Promise.all([c, cleared]);
  assert.deepEqual([useFacelessStore.getState().path, useFacelessStore.getState().production], [null, null]);

  // An older read of the same project loses to a newer one.
  const old = store.refresh("D");
  const fresh = store.refresh("D");
  answers.pending[4].resolve(production("D new"));
  answers.pending[3].resolve(production("D old"));
  await Promise.all([old, fresh]);
  assert.equal(useFacelessStore.getState().production?.topic, "D new");
});

// GAP-010
test("quick audio changes go one at a time on the latest revision and both land", async () => {
  const project = (revision: number, audio: Partial<AudioSettings>) =>
    ({ projectHandle: "p", revision, audio }) as unknown as OpenedProject;
  useProjectStore.setState({ openedProject: project(1, {}) });
  useProjectStore.setState({
    applyOpenedProject: async (p: OpenedProject) => {
      useProjectStore.setState({ openedProject: p });
    },
  });
  const sent: { revision: number; audio: AudioSettings }[] = [];
  const answers = deferred<OpenedProject>();
  api.projectAudioUpdate = (_handle, revision, audio) => {
    sent.push({ revision, audio });
    return answers.call();
  };
  const volume = updateAudio((a) => ({ ...a, tracks: { t1: { volumeDb: -6 } } }) as AudioSettings);
  const clicks = updateAudio((a) => ({ ...a, mouthClicks: true }));
  await settle();
  assert.equal(sent.length, 1, "the second waits for the first");
  answers.pending[0].resolve(project(2, sent[0].audio));
  await settle();
  await settle();
  assert.equal(sent.length, 2);
  assert.equal(sent[1].revision, 2, "sent on the revision the first save left");
  assert.equal(sent[1].audio.mouthClicks, true);
  assert.deepEqual(sent[1].audio.tracks, { t1: { volumeDb: -6 } }, "keeps the first change");
  answers.pending[1].resolve(project(3, sent[1].audio));
  await Promise.all([volume, clicks]);
  assert.equal(useProjectStore.getState().openedProject?.revision, 3);

  // A change queued for one project is dropped once another is open.
  const late = updateAudio((a) => ({ ...a, mouthClicks: false }));
  useProjectStore.setState({ openedProject: { ...project(1, {}), projectHandle: "other" } });
  await late;
  assert.equal(sent.length, 2);
});
