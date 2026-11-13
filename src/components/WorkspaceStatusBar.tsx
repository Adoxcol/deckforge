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
    <footer aria-label="Workspace status" className="shrink-0 h-10 border-t border-[var(--border-subtle)] bg-[var(--bg-sidebar)] text-[12px] text-[var(--text-secondary)] px-4 flex items-center gap-4 min-w-0">
      <span className="flex items-center gap-1.5 min-w-0" title={`Editing ${profileName} / ${page.name}`}>
        <Layers size={13} className="shrink-0 text-[var(--text-muted)]" />
        <span className="truncate text-[var(--text-primary)]">{profileName} / {page.name}</span>
      </span>
      <span className={`flex items-center gap-1 shrink-0 ${saved ? "text-[var(--status-success)]" : "text-[var(--status-warning)]"}`}>
        {saved ? <Check size={13} /> : <span className="w-1.5 h-1.5 rounded-full bg-current" />}{saved ? "Saved" : "Unsaved"}
      </span>
      <span className="w-px h-4 bg-[var(--border-subtle)] shrink-0" aria-hidden />
      <span className="flex items-center gap-1.5 min-w-0" title={applied ? `On the deck: ${applied.profileName} / ${applied.pageName}` : "Nothing applied yet"}>
        <HardDrive size={13} className="shrink-0 text-[var(--text-muted)]" />
        <span className={`truncate ${matching ? "text-[var(--status-success)]" : ""}`}>{applyLabel}</span>
      </span>
      <div className="ml-auto flex items-center gap-3 min-w-0">
        <span className="hidden xl:inline max-w-[180px] truncate text-[var(--text-muted)]" title={deviceName}>{deviceName}</span>
        <DeckStatusBadge status={status?.deck ?? null} onNotify={onNotify} onRefresh={() => setRevision(n => n + 1)} />
      </div>
    </footer>
  );
}
