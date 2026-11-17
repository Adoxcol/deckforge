import React, { useEffect, useRef } from "react";
import { ArrowRight, Check, X } from "lucide-react";
import type { DeviceDescriptor } from "../types/deckforge";
import { isTauriRuntime } from "../bridge/tauriBridge";

export const OnboardingModal: React.FC<{ devices: DeviceDescriptor[]; onClose: () => void; onOpenDevices: () => void }> = ({ devices, onClose, onOpenDevices }) => {
  const panel = useRef<HTMLDivElement>(null);
  const close = useRef(onClose); close.current = onClose;
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    panel.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const keyboard = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); close.current(); }
      if (event.key !== "Tab") return;
      const buttons = Array.from(panel.current?.querySelectorAll<HTMLButtonElement>("button") ?? []);
      if (!buttons.length) return;
      const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      event.preventDefault(); buttons[(index + (event.shiftKey ? -1 : 1) + buttons.length) % buttons.length].focus();
    };
    document.addEventListener("keydown", keyboard, true);
    return () => { document.removeEventListener("keydown", keyboard, true); previous?.focus(); };
  }, []);
  const connected = devices.filter(device => device.connected && !device.isSimulator);
  const desktop = isTauriRuntime();
  const steps = [
    ["Give DeckForge control", "Close AJAZZ and its tray watcher. If DeckForge is paused, choose Use DeckForge in the status bar. Check Devices to confirm the connection."],
    ["Edit and save your page", "Choose a profile, select a key, and assign an action. Save keeps profile edits on this PC; the deck continues using its last applied page."],
    ["Apply, then test", "Choose Apply to Device to send the selected page and its bindings together. Test a key and check the status bar for transfer errors."],
    ["Keep running or switch back", "Closing the window keeps DeckForge in the tray. To use AJAZZ, choose Pause DeckForge and wait for the paused status. Use tray Quit to stop DeckForge."],
  ];
  return <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-[2px] flex items-center justify-center p-6 animate-fade-in">
    <div ref={panel} role="dialog" aria-modal="true" aria-labelledby="quick-start-title" className="w-full max-w-2xl max-h-[85vh] overflow-y-auto rounded-2xl bg-[var(--bg-panel)] border border-[var(--border-strong)] p-6 shadow-2xl text-[var(--text-primary)] animate-pop-in">
      <header className="flex items-start justify-between gap-4 mb-4"><div><p className="text-[10px] uppercase tracking-widest text-[var(--accent-hover)] mb-1">Getting started</p><h2 id="quick-start-title" className="text-xl font-bold">Your first page on the deck</h2></div>
        <button type="button" aria-label="Close quick-start guide" onClick={onClose} className="p-2 rounded-lg text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer"><X size={18} /></button>
      </header>
      <div className="rounded-lg border border-[var(--border-strong)] bg-[var(--bg-sidebar)] px-3 py-2 text-xs text-[var(--text-secondary)] mb-5">
        {!desktop ? "Browser preview — hardware status is unavailable." : connected.length ? `Detected at last scan: ${connected.map(device => device.modelName).join(", ")}. Connection does not confirm device control.` : "No physical deck detected at the last scan. Connect it by USB and rescan in Devices."}
      </div>
      <ol className="space-y-4">{steps.map(([title, description], index) => <li key={title} className="flex gap-3"><span className="shrink-0 w-7 h-7 rounded-full bg-[var(--accent-soft)] text-[var(--accent-text)] text-xs font-bold flex items-center justify-center">{index + 1}</span><div><h3 className="text-sm font-semibold">{title}</h3><p className="text-xs text-[var(--text-secondary)] leading-relaxed mt-1">{description}</p></div></li>)}</ol>
      <footer className="flex items-center justify-between gap-3 mt-6 pt-4 border-t border-[var(--border-subtle)]">
        <button type="button" onClick={onOpenDevices} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-[var(--border-strong)] hover:border-[var(--accent-primary)] hover:bg-[var(--bg-elevated)] text-xs font-medium cursor-pointer">Check devices<ArrowRight size={14} /></button>
        <button type="button" onClick={onClose} className="flex items-center gap-2 px-4 py-2 rounded-lg bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white cursor-pointer"><Check size={14} />Got it</button>
      </footer>
    </div>
  </div>;
};
