import { create } from "zustand";

/**
 * Whether the project's changes are on disk. Every edit command carries the revision it
 * expects (see invokeTauri in lib/ipc.ts); the backend writes the project before it answers,
 * so an answer means saved and a failure means that change was not saved.
 */
interface SaveStatusStore {
  /** Edits sent and not answered yet. */
  pending: number;
  /** Why the last edit failed, until the next one succeeds. */
  failed: string | null;
  savedAt: number | null;
  begin: () => void;
  succeed: () => void;
  fail: (reason: string) => void;
  clearFailure: () => void;
}

export const useSaveStatusStore = create<SaveStatusStore>((set) => ({
  pending: 0,
  failed: null,
  savedAt: null,
  begin: () => set((state) => ({ pending: state.pending + 1 })),
  succeed: () => set((state) => ({ pending: Math.max(0, state.pending - 1), failed: null, savedAt: Date.now() })),
  fail: (reason) => set((state) => ({ pending: Math.max(0, state.pending - 1), failed: reason })),
  clearFailure: () => set({ failed: null }),
}));
