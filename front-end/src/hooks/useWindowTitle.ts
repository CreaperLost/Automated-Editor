import { useEffect } from "react";
import { useProjectStore } from "../stores/projectStore";
import { api } from "../lib/ipc";

export function formatWindowTitle(openedProjectName?: string | null): string {
  const trimmed = openedProjectName?.trim();
  return trimmed ? `AeroShoot Editor \u2014 ${trimmed}` : "AeroShoot Editor";
}

export function useWindowTitle(): void {
  const openedProjectName = useProjectStore((s) => s.openedProject?.manifest.projectName);

  useEffect(() => {
    const title = formatWindowTitle(openedProjectName);
    void api.setWindowTitle(title);
  }, [openedProjectName]);
}
