import React, { useEffect, useRef } from "react";

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
  return <div className="fixed inset-0 z-[100] bg-black/70 flex items-center justify-center" onKeyDown={e => e.stopPropagation()}>
    <div ref={panel} role="dialog" aria-modal="true" aria-labelledby="quit-title" className="rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-panel)] p-6 max-w-lg text-[var(--text-primary)] space-y-4">
      <h2 id="quit-title" className="text-lg font-bold">Quit DeckForge?</h2>
      <p className="text-sm">Quitting stops device control. Closing the window instead keeps DeckForge running in the system tray.</p>
      {dirty && <p className="text-sm text-[var(--status-warning)]">You have unsaved edits. Discard and quit will lose edits that have not been saved.</p>}
      {error && <p role="alert" className="text-sm text-[var(--status-error)]">{error}</p>}
      <div className="flex gap-3 text-sm">
        <button autoFocus disabled={busy} onClick={onCancel} className="p-2 rounded border border-[var(--text-muted)]">Cancel</button>
        <button disabled={busy} onClick={() => onQuit(false)} className="p-2 rounded border border-[var(--status-error)]">{dirty ? "Discard and quit" : "Quit"}</button>
        <button disabled={busy} onClick={() => onQuit(true)} className="p-2 rounded bg-[var(--accent-primary)]">{busy ? "Finishing…" : "Save and quit"}</button>
      </div>
    </div>
  </div>;
}
