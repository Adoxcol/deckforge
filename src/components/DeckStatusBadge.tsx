import React, { useCallback, useState } from "react";
import { RefreshCw } from "lucide-react";
import { deckforgeApi, isTauriRuntime } from "../bridge/tauriBridge";
import type { DeckStatus } from "../types/deckforge";

interface Props {
  status: DeckStatus | null;
  onRefresh: () => void;
  onNotify: (text: string, kind?: "success" | "info" | "warning") => void;
}


function clock(ms: number | null): string {
  if (!ms) return "";
  return new Date(ms).toLocaleTimeString([], { hour: "numeric", minute: "2-digit", second: "2-digit" });
}

/** What the status bar should say about the deck, and in which colour. */
export function describeDeckStatus(status: DeckStatus | null): { text: string; color: string; detail: string } {
  if (!status) {
    return { text: "Checking the deck…", color: "#707B94", detail: "" };
  }
  if (status.state === "conflict") {
    return { text: "AJAZZ running · DeckForge paused", color: "#F4B860",
      detail: "Save your work and close AJAZZ and its tray watcher, then choose Use DeckForge. No application is closed automatically." };
  }
  if (status.state === "paused") {
    return { text: "DeckForge paused", color: "#F4B860",
      detail: "Device control is released. You can use AJAZZ. Close it before choosing Use DeckForge." };
  }
  if (status.state === "pausing") {
    return { text: "Releasing the deck…", color: "#F4B860",
      detail: "Wait for DeckForge paused before opening AJAZZ." };
  }
  if (status.state === "searching") {
    return {
      text: "Looking for the deck…",
      color: "#707B94",
      detail: "Plug the deck in with a USB cable, and close the official Ajazz app if it is open.",
    };
  }
  if (status.state === "reconnecting") {
    return {
      text: "Deck lost, reconnecting…",
      color: "#F4B860",
      detail: "The deck dropped off USB. DeckForge sends the layout again as soon as it comes back.",
    };
  }
  if (status.lastPushOk === false) {
    return {
      text: `Last send failed${status.lastError ? `: ${status.lastError}` : ""}`,
      color: "#F16B79",
      detail: "Use Re-send layout to try the whole layout again.",
    };
  }
  const sent = status.lastPushAtMs ? `layout sent ${clock(status.lastPushAtMs)}` : "waiting for the first send";
  const dropped = status.reconnects > 0 ? ` · reconnected ${status.reconnects}×` : "";
  return {
    text: `Deck connected · ${sent}${dropped}`,
    color: "#35C98B",
    detail: "",
  };
}

export const DeckStatusBadge: React.FC<Props> = ({ onNotify, status, onRefresh }) => {
  const [sending, setSending] = useState(false);
  const [changingControl, setChangingControl] = useState(false);

  const resend = useCallback(async () => {
    setSending(true);
    try {
      const message = await deckforgeApi.resendLayout();
      onNotify(message, "info");
    } catch (err) {
      onNotify(typeof err === "string" ? err : "Could not ask the deck for the layout again", "warning");
    } finally {
      setSending(false);
      onRefresh();
    }
  }, [onNotify, onRefresh]);

  const view = describeDeckStatus(status);
  const suspended = status?.state === "paused" || status?.state === "conflict";
  const changeControl = async () => {
    setChangingControl(true);
    try {
      const message = suspended ? await deckforgeApi.takeOverFromOfficialApp() : await deckforgeApi.pauseDeckControl();
      onNotify(message, "info");
      onRefresh();
    } catch (error) {
      onNotify(typeof error === "string" ? error : "Could not change device control", "warning");
    } finally { setChangingControl(false); }
  };

  return (
    <span className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2" title={view.detail || undefined}>
      <span className="flex min-w-0 flex-1 items-center gap-2" style={{ color: view.color }}>
        <span className="w-2 h-2 shrink-0 rounded-full" style={{ backgroundColor: view.color }} />
        <span className="truncate" title={view.text}>{isTauriRuntime() ? view.text : `Browser preview · ${status?.state ?? "checking"}`}</span>
      </span>
      <button
        type="button"
        onClick={resend}
        disabled={sending || status?.state !== "connected"}
        title="Send every picture to the deck again, with the full connect sequence"
        className="h-8 px-3 shrink-0 whitespace-nowrap rounded-md border border-[var(--border-subtle)] bg-[var(--bg-panel)] text-[11px] text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:border-[var(--accent-primary)] disabled:opacity-40 disabled:hover:border-[var(--border-subtle)] disabled:hover:text-[var(--text-secondary)] flex items-center gap-1 cursor-pointer disabled:cursor-default"
      >
        <RefreshCw size={12} className={sending ? "animate-spin" : ""} />
        <span>Resend layout</span>
      </button>
      <button type="button" onClick={changeControl}
        disabled={!status || changingControl || status.state === "pausing"}
        className="h-8 px-3 shrink-0 whitespace-nowrap rounded-md border border-[var(--border-subtle)] text-[11px] text-[var(--status-warning)] disabled:opacity-40"
        title={suspended ? "Close AJAZZ first, then reconnect with DeckForge" : "Release the deck so you can use AJAZZ"}>
        {suspended ? "Use DeckForge" : "Pause DeckForge"}
      </button>
    </span>
  );
};
