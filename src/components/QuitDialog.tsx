import React, { useEffect, useRef } from "react";
import { Power } from "lucide-react";

export function QuitDialog({ open, busy, dirty, error, onCancel, onQuit }: {
  open: boolean; busy: boolean; dirty: boolean; error: string | null;
  onCancel: () => void; onQuit: (save: boolean) => void;
}) {
  const panel = useRef<HTMLDivElement>(null);
  const cancel = useRef(onCancel);
  cancel.current = onCancel;
  useEffect(() => {
    if (!open) return;
    const before = document.activeElement as HTMLElement | null;
    panel.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const trap = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) { event.preventDefault(); cancel.current(); }
      if (event.key !== "Tab") return;
      const buttons = Array.from(panel.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []);
      if (!buttons.length) { event.preventDefault(); return; }
      const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = (index + (event.shiftKey ? -1 : 1) + buttons.length) % buttons.length;
      event.preventDefault(); buttons[next].focus();
    };
    document.addEventListener("keydown", trap, true);
    return () => { document.removeEventListener("keydown", trap, true); before?.focus(); };
  }, [open, busy]);
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-[100] bg-black/70 backdrop-blur-[2px] flex items-center justify-center p-6 animate-fade-in" onMouseDown={e => { if (e.target === e.currentTarget && !busy) onCancel(); }} onKeyDown={e => e.stopPropagation()}>
      <div ref={panel} role="dialog" aria-modal="true" aria-labelledby="quit-title" className="rounded-2xl border border-[var(--border-strong)] bg-[var(--bg-panel)] p-6 max-w-md w-full shadow-2xl text-[var(--text-primary)] space-y-4 animate-pop-in">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-xl bg-[var(--status-error)]/12 border border-[var(--status-error)]/25 text-[var(--status-error)] flex items-center justify-center shrink-0">
            <Power size={17} />
          </div>
          <h2 id="quit-title" className="text-lg font-bold">Quit DeckForge?</h2>
        </div>
        <p className="text-xs leading-relaxed text-[var(--text-secondary)]">Quitting stops device control. Closing the window instead keeps DeckForge running in the system tray.</p>
        {dirty && (
          <p className="text-xs leading-relaxed text-[var(--status-warning)] bg-[var(--status-warning)]/10 border border-[var(--status-warning)]/30 rounded-lg px-3 py-2">
            You have unsaved edits. Discard and quit will lose edits that have not been saved.
          </p>
        )}
        {error && <p role="alert" className="text-xs text-[var(--status-error)]">{error}</p>}
        <div className="flex flex-wrap items-center justify-end gap-2 pt-2 border-t border-[var(--border-subtle)] text-xs">
          <button type="button" autoFocus disabled={busy} onClick={onCancel} className="h-9 px-3.5 rounded-lg border border-[var(--border-strong)] hover:bg-[var(--bg-elevated)] active:scale-[0.98] font-medium text-[var(--text-primary)] cursor-pointer disabled:opacity-40 transition-all">Cancel</button>
          <button type="button" disabled={busy} onClick={() => onQuit(false)} className="h-9 px-3.5 rounded-lg border border-[var(--status-error)]/50 hover:bg-[var(--status-error)]/10 active:scale-[0.98] font-medium text-[var(--status-error)] cursor-pointer disabled:opacity-40 transition-all">{dirty ? "Discard and quit" : "Quit"}</button>
          <button type="button" disabled={busy} onClick={() => onQuit(true)} className="h-9 px-4 rounded-lg bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] active:scale-[0.98] font-semibold text-white cursor-pointer disabled:opacity-40 shadow-xs transition-all">{busy ? "Finishing…" : "Save and quit"}</button>
        </div>
      </div>
    </div>
  );
}
