import { useEffect } from "react";
import { useProjectStore } from "../stores/projectStore";
import { api } from "../lib/ipc";

export function formatWindowTitle(openedProjectName?: string | null): string {
  const trimmed = openedProjectName?.trim();
  return trimmed ? `AeroEdits \u2014 ${trimmed}` : "AeroEdits";
}

export function useWindowTitle(): void {
  const openedProjectName = useProjectStore((s) => s.openedProject?.name);

  useEffect(() => {
    const title = formatWindowTitle(openedProjectName);
    void api.setWindowTitle(title);
  }, [openedProjectName]);
}
