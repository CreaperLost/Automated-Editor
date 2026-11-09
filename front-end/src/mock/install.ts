// Preview builds only (`npm run build:preview`): stands in for the desktop app's backend, so
// the UI runs in a plain browser with a canned project. Not part of the app's build.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { answer, PROJECT_PATH } from "./backend";
import { useProjectStore } from "../stores/projectStore";

mockWindows("main");
mockIPC(
  (cmd, args) => {
    const reply = answer(cmd, (args ?? {}) as Record<string, unknown>);
    if (reply === undefined) {
      console.warn(`[mock] no answer for ${cmd}`, args);
      return null;
    }
    return reply;
  },
  { shouldMockEvents: true },
);

// The start screen lists the canned project as a recent one.
useProjectStore.setState({ recentProjects: [PROJECT_PATH] });
