import { create } from "zustand";

/**
 * Basic or Advanced settings (UI-2), for the whole app. Basic shows what most edits change
 * and keeps the fine-tuning one click away in each panel ("More settings"); Advanced shows
 * everything, and the panels' own Advanced parts start open. Kept on this machine.
 */
const STORAGE_KEY = "aeroedits.settingsMode.v1";

function load(): boolean {
  try {
    return window.localStorage.getItem(STORAGE_KEY) === "advanced";
  } catch {
    return false;
  }
}

interface UiModeStore {
  advanced: boolean;
  setAdvanced: (advanced: boolean) => void;
}

export const useUiMode = create<UiModeStore>((set) => ({
  advanced: load(),
  setAdvanced: (advanced) => {
    try {
      window.localStorage.setItem(STORAGE_KEY, advanced ? "advanced" : "basic");
    } catch {
      // Not remembered on this machine; the mode still applies for this session.
    }
    set({ advanced });
  },
}));
