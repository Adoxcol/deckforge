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
    return { text: "Checking the deck…", color: "var(--text-muted)", detail: "" };
  }
  if (status.state === "conflict") {
    return { text: "AJAZZ running · DeckForge paused", color: "var(--status-warning)",
      detail: "Save your work and close AJAZZ and its tray watcher, then choose Use DeckForge. No application is closed automatically." };
  }
  if (status.state === "paused") {
    return { text: "DeckForge paused", color: "var(--status-warning)",
      detail: "Device control is released. You can use AJAZZ. Close it before choosing Use DeckForge." };
  }
  if (status.state === "pausing") {
    return { text: "Releasing the deck…", color: "var(--status-warning)",
      detail: "Wait for DeckForge paused before opening AJAZZ." };
  }
  if (status.state === "searching") {
    return {
      text: "Looking for the deck…",
      color: "var(--text-muted)",
      detail: "Plug the deck in with a USB cable, and close the official Ajazz app if it is open.",
    };
  }
  if (status.state === "reconnecting") {
    return {
      text: "Deck lost, reconnecting…",
      color: "var(--status-warning)",
      detail: "The deck dropped off USB. DeckForge sends the layout again as soon as it comes back.",
    };
  }
  if (status.lastPushOk === false) {
    return {
      text: `Last send failed${status.lastError ? `: ${status.lastError}` : ""}`,
      color: "var(--status-error)",
      detail: "Use Re-send layout to try the whole layout again.",
    };
  }
  const sent = status.lastPushAtMs ? `layout sent ${clock(status.lastPushAtMs)}` : "waiting for the first send";
  const dropped = status.reconnects > 0 ? ` · reconnected ${status.reconnects}×` : "";
  return {
    text: `Deck connected · ${sent}${dropped}`,
    color: "var(--status-success)",
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
    <span className="flex min-w-0 items-center gap-2" title={view.detail || undefined}>
      <span
        className="flex min-w-0 items-center gap-2 px-2.5 h-7 rounded-full bg-[var(--bg-panel)]/80 border border-[var(--border-subtle)]"
        style={{ color: view.color }}
      >
        <span
          className={`w-2 h-2 shrink-0 rounded-full ${
            status?.state === "connected" && status.lastPushOk !== false
              ? "shadow-[0_0_7px_var(--status-success)]"
              : status?.state === "searching" || status?.state === "reconnecting" || status?.state === "pausing"
              ? "animate-pulse"
              : ""
          }`}
          style={{ backgroundColor: view.color }}
        />
        <span className="truncate text-[11.5px] font-medium" title={view.text}>{isTauriRuntime() ? view.text : `Browser preview · ${status?.state ?? "checking"}`}</span>
      </span>
      <button
        type="button"
        onClick={resend}
        disabled={sending || status?.state !== "connected"}
        title="Send every picture to the deck again, with the full connect sequence"
        className="h-7 px-2.5 shrink-0 whitespace-nowrap rounded-md border border-[var(--border-subtle)] bg-[var(--bg-panel)] text-[11px] font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] active:scale-[0.97] disabled:opacity-40 disabled:hover:bg-[var(--bg-panel)] disabled:hover:border-[var(--border-subtle)] disabled:hover:text-[var(--text-secondary)] disabled:active:scale-100 flex items-center gap-1.5 cursor-pointer disabled:cursor-default transition-all"
      >
        <RefreshCw size={11} className={sending ? "animate-spin" : ""} />
        <span>Resend layout</span>
      </button>
      <button
        type="button"
        onClick={changeControl}
        disabled={!status || changingControl || status.state === "pausing"}
        className="h-7 px-2.5 shrink-0 whitespace-nowrap rounded-md border border-[var(--border-subtle)] bg-[var(--bg-panel)] text-[11px] font-medium text-[var(--status-warning)] hover:border-[var(--status-warning)]/60 hover:bg-[var(--status-warning)]/10 active:scale-[0.97] disabled:opacity-40 disabled:hover:border-[var(--border-subtle)] disabled:hover:bg-[var(--bg-panel)] disabled:active:scale-100 cursor-pointer disabled:cursor-default transition-all"
        title={suspended ? "Close AJAZZ first, then reconnect with DeckForge" : "Release the deck so you can use AJAZZ"}
      >
        {suspended ? "Use DeckForge" : "Pause DeckForge"}
      </button>
    </span>
  );
};
