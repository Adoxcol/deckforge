import React, { useEffect, useState } from "react";
import { Check, HardDrive, Layers } from "lucide-react";
import { deckforgeApi } from "../bridge/tauriBridge";
import type { ProfilePage, WorkspaceStatus } from "../types/deckforge";
import { DeckStatusBadge } from "./DeckStatusBadge";

interface Props {
  deviceId: string;
  deviceName: string;
  profileId: string;
  profileName: string;
  page: ProfilePage;
  saved: boolean;
  refresh: number;
  onNotify: (text: string, kind?: "success" | "info" | "warning") => void;
}

export function WorkspaceStatusBar({ deviceId, deviceName, profileId, profileName, page, saved, refresh, onNotify }: Props) {
  const [revision, setRevision] = useState(0);
  const [result, setResult] = useState<{ key: string; status: WorkspaceStatus } | null>(null);
  const [failed, setFailed] = useState(false);
  // Include the full draft so a late response can never label newer edits as applied.
  const key = JSON.stringify([deviceId, profileId, page, refresh, revision]);
  useEffect(() => {
    let alive = true;
    let timer: ReturnType<typeof setTimeout>;
    setFailed(false);
    const read = async () => {
      try {
        const status = await deckforgeApi.getWorkspaceStatus(deviceId, profileId, page);
        if (alive) { setResult({ key, status }); setFailed(false); }
      } catch {
        if (alive) { setResult(null); setFailed(true); }
      } finally {
        if (alive) timer = setTimeout(read, 2000);
      }
    };
    timer = setTimeout(read, 150);
    return () => { alive = false; clearTimeout(timer); };
  }, [key]);
  const status = result?.key === key ? result.status : null;
  const applied = status?.applied;
  const active = applied?.layoutReady && status?.deck.state === "connected" && status.deck.lastPushOk === true;
  const matching = active && applied?.matchesEditor;
  const applyLabel = !status ? (failed ? "Status unavailable" : "Checking applied layout…")
    : !applied ? "No layout applied" : matching ? "Matches editor" : !active ? "Device not ready" : "Editor not applied";
  const pill = "inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full border px-2.5 py-1 text-[11px] font-medium";

  return (
    <footer aria-label="Workspace status" className="shrink-0 border-t border-[#2A3042] bg-[#10131D] text-xs text-[#A6AFC4]">
      <div className="grid grid-cols-2 gap-x-6 gap-y-2 border-b border-[#222838] px-4 py-2.5">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <Layers size={14} className="shrink-0 text-[#9D75F8]" />
          <span className="text-[10px] uppercase tracking-wider text-[#707B94]">Editing</span>
          <span className="min-w-0 flex-1 truncate text-[#E4E7F0]" title={`${profileName} / ${page.name}`}>{profileName} / {page.name}</span>
          <span className={`${pill} ${saved ? "border-[#35C98B]/20 bg-[#35C98B]/5 text-[#68DDB0]" : "border-[#F4B860]/20 bg-[#F4B860]/5 text-[#F4B860]"}`}>
            {saved && <Check size={12} />}{saved ? "Saved locally" : "Unsaved changes"}
          </span>
        </div>
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <HardDrive size={14} className="shrink-0 text-[#707B94]" />
          <span className="text-[10px] uppercase tracking-wider text-[#707B94]">Last applied</span>
          <span className="min-w-0 flex-1 truncate text-[#E4E7F0]" title={applied ? `${applied.profileName} / ${applied.pageName}` : undefined}>{applied ? `${applied.profileName} / ${applied.pageName}` : "—"}</span>
          <span className={`${pill} ${matching ? "border-[#35C98B]/20 bg-[#35C98B]/5 text-[#68DDB0]" : "border-[#394158] text-[#A6AFC4]"}`}>{applyLabel}</span>
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-x-5 gap-y-2 px-4 py-2">
        <span className="max-w-[220px] truncate font-medium text-[#E4E7F0]" title={deviceName}>{deviceName}</span>
        <div className="min-w-0 flex-1 basis-[480px]">
          <DeckStatusBadge status={status?.deck ?? null} onNotify={onNotify} onRefresh={() => setRevision(n => n + 1)} />
        </div>
      </div>
    </footer>
  );
}
