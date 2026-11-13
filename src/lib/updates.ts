import { isTauriRuntime } from "../bridge/tauriBridge";

export interface UpdateInfo {
  version: string;
  notes: string;
  install: (onProgress?: (percent: number) => void) => Promise<void>;
}

/** Asks GitHub for a newer signed release. Null when up to date or outside the desktop app. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  if (!isTauriRuntime()) return null;
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return null;
  return {
    version: update.version,
    notes: update.body ?? "",
    install: async (onProgress) => {
      let total = 0;
      let done = 0;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          done += event.data.chunkLength;
          if (total > 0) onProgress?.(Math.min(100, Math.round((done / total) * 100)));
        }
      });
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    },
  };
}
