import React, { useState } from "react";
import { Download, RefreshCw } from "lucide-react";
import { checkForUpdate, type UpdateInfo } from "../lib/updates";
import { isTauriRuntime } from "../bridge/tauriBridge";
import { errorText } from "../lib/format";

const action = "inline-flex items-center gap-2 rounded-lg border border-[var(--border-strong)] bg-[var(--bg-sidebar)] px-3 py-2 text-xs font-medium text-[var(--text-primary)] hover:border-[var(--accent-primary)] hover:bg-[var(--accent-soft)] active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:border-[var(--border-strong)] disabled:hover:bg-[var(--bg-sidebar)] disabled:active:scale-100 cursor-pointer transition-all";

export const UpdatesPanel: React.FC<{ card: string }> = ({ card }) => {
  const [state, setState] = useState<"idle" | "checking" | "current" | "found" | "installing" | "error">("idle");
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [percent, setPercent] = useState(0);
  const [message, setMessage] = useState("");
  const desktop = isTauriRuntime();

  const check = async () => {
    setState("checking");
    try {
      const found = await checkForUpdate();
      setUpdate(found);
      setState(found ? "found" : "current");
    } catch (err) {
      setMessage(errorText(err));
      setState("error");
    }
  };

  const install = async () => {
    if (!update) return;
    setState("installing");
    try {
      await update.install(setPercent);
    } catch (err) {
      setMessage(errorText(err));
      setState("error");
    }
  };

  return (
    <section className={card} aria-labelledby="updates-heading">
      <div className="flex items-center gap-2 text-[var(--text-primary)]"><Download size={17} /><h2 id="updates-heading" className="font-semibold">Updates</h2></div>
      <p className="text-xs leading-relaxed">
        DeckForge looks for a new version on GitHub while it runs, downloads it in the background, and installs it when you quit from the tray. You can also install one now. Updates are signed, so only builds made from this project install.
      </p>
      {state === "current" && <p className="inline-flex w-fit items-center rounded-md border border-[var(--status-success)]/30 bg-[var(--status-success)]/10 px-2.5 py-1 text-xs font-medium text-[var(--status-success)]">You have the latest version.</p>}
      {state === "found" && update && (
        <div className="rounded-lg border border-[var(--accent-primary)]/40 bg-[var(--accent-soft)]/50 p-3 text-xs space-y-1">
          <p className="text-[var(--text-primary)] font-semibold">Version {update.version} is available.</p>
          {update.notes && <p className="whitespace-pre-line text-[var(--text-secondary)]">{update.notes}</p>}
        </div>
      )}
      {state === "installing" && (
        <div className="space-y-1.5">
          <p className="text-xs tabular-nums text-[var(--accent-text)]">Downloading… {percent}%. DeckForge restarts when it's done.</p>
          <div className="h-1.5 w-full rounded-full bg-[var(--bg-sidebar)] overflow-hidden">
            <div className="h-full rounded-full bg-[var(--accent-primary)] transition-[width] duration-150" style={{ width: `${Math.max(4, Math.min(100, percent))}%` }} />
          </div>
        </div>
      )}
      {state === "error" && <p className="rounded-md border border-[var(--status-error)]/30 bg-[var(--status-error)]/10 px-2.5 py-1.5 text-xs text-[var(--status-error)]">Could not check for updates: {message}</p>}
      <div className="flex flex-wrap gap-2">
        {state === "found" ? (
          <button type="button" className={action} onClick={install}><Download size={14} />Install and restart</button>
        ) : (
          <button type="button" className={action} disabled={!desktop || state === "checking" || state === "installing"} onClick={check}>
            <RefreshCw size={14} className={state === "checking" ? "animate-spin" : ""} />{state === "checking" ? "Checking…" : "Check for updates"}
          </button>
        )}
      </div>
      {!desktop && <p className="text-[10.5px]">Updates work in the desktop app only.</p>}
    </section>
  );
};
