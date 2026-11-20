import React, { useState } from "react";
import { Download, Upload } from "lucide-react";
import { isTauriRuntime } from "../bridge/tauriBridge";
import { pickJsonFile } from "../lib/shareFiles";
import { errorText } from "../lib/format";

export function BackupPanel({ onBackup, onRestore }: {
  onBackup: () => Promise<string>;
  onRestore: (contents: string) => Promise<number>;
}) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [pending, setPending] = useState<{ text: string; name: string; count: number } | null>(null);
  const desktop = isTauriRuntime();
  const run = async (task: () => Promise<void>) => {
    setBusy(true); setError(""); setMessage("");
    try { await task(); } catch (error) { setError(errorText(error)); }
    finally { setBusy(false); }
  };
  const choose = () => run(async () => {
    setPending(null);
    const picked = await pickJsonFile();
    if (!picked) return;
    const file = JSON.parse(picked.text);
    if (file?.kind !== "deckforge.profiles-backup" || file.version !== 1 || !Array.isArray(file.profiles) || !file.profiles.length) {
      throw new Error("Choose a DeckForge profiles backup (version 1). Regular profile share files use Import in the workspace.");
    }
    setPending({ ...picked, count: file.profiles.length });
  });
  const button = "inline-flex items-center gap-2 rounded-lg border border-[var(--border-strong)] bg-[var(--bg-sidebar)] px-3 py-2 text-xs font-medium text-[var(--text-primary)] hover:border-[var(--accent-primary)] hover:bg-[var(--accent-soft)] disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:border-[var(--border-strong)] disabled:hover:bg-[var(--bg-sidebar)] cursor-pointer transition-colors";
  return <section className="rounded-xl border border-[var(--accent-primary)]/30 bg-[var(--bg-panel)] p-5 space-y-3">
    <h2 className="font-semibold text-[var(--text-primary)]">Profiles & media backup</h2>
    <p>Back up all editor profiles, all macros, and every image or animation those profiles use in one file.</p>
    <p className="text-xs leading-relaxed">Includes unsaved profile edits and media playback settings. Restore adds copies to the editor; Save keeps them locally and Apply sends a page to the deck. Existing profiles remain unchanged.</p>
    <div className="flex flex-wrap gap-2">
      <button type="button" className={button} disabled={!desktop || busy || !!pending} onClick={() => run(async () => setMessage(`Backup saved to ${await onBackup()}`))}><Download size={14} />Create backup</button>
      <button type="button" className={button} disabled={!desktop || busy} onClick={choose}><Upload size={14} />Restore backup…</button>
    </div>
    {!desktop && <p className="text-xs text-[var(--text-muted)]">Open the desktop app to back up or restore its media library.</p>}
    {busy && <p role="status" className="text-xs">Processing backup…</p>}
    {pending && <div className="rounded-lg border border-[var(--border-strong)] p-3 space-y-3">
      <p className="text-xs break-all">{pending.name} · {pending.count} profiles</p>
      <p className="text-xs">Media will be checked before import. Imported actions and macros may contain local paths or commands; review them before use.</p>
      <div className="flex gap-2"><button type="button" className={button} disabled={busy} onClick={() => run(async () => {
        const count = await onRestore(pending.text);
        setPending(null); setMessage(`Restored ${count} profiles as editor drafts. Open the workspace to review and save them. Macros have been queued for local save.`);
      })}>Restore as copies</button><button type="button" className={button} disabled={busy} onClick={() => setPending(null)}>Cancel</button></div>
    </div>}
    {message && <p role="status" className="text-xs text-[var(--status-success)] break-all">{message}</p>}
    {error && <p role="alert" className="text-xs text-[var(--status-error)] break-all">{error}</p>}
    <p className="text-xs text-[var(--text-muted)]">Up to 64 MB of source media per backup. Auto-switch rules, OBS connection settings, Game Modes, startup preferences, and unused media are not included.</p>
  </section>;
}
