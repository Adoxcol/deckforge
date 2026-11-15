import { useCallback, useEffect, useId, useRef, useState } from "react";
import { deckforgeApi } from "../bridge/tauriBridge";

export function AppAudioTarget({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const [apps, setApps] = useState<{ exe: string; sessions: number }[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const request = useRef(0);
  const id = useId();
  const refresh = useCallback(async () => {
    const current = ++request.current;
    setBusy(true);
    setError("");
    try {
      const next = await deckforgeApi.audioApps();
      if (request.current === current) setApps(next);
    } catch (err) {
      if (request.current === current) setError(String(err));
    } finally {
      if (request.current === current) setBusy(false);
    }
  }, []);
  useEffect(() => { void refresh(); return () => { request.current++; }; }, [refresh]);
  return <div className="flex flex-col gap-2">
    <div className="flex items-center justify-between gap-2">
      <label htmlFor={id} className="text-[11px] text-[var(--text-secondary)]">Audio application</label>
      <button type="button" disabled={busy} onClick={() => void refresh()} className="text-[11px] text-[var(--accent-primary)] disabled:opacity-50">{busy ? "Scanning…" : "Refresh apps"}</button>
    </div>
    <select aria-label="Choose an app with an audio session" value={apps.some(app => app.exe === value) ? value : ""}
      onChange={event => { if (event.target.value) onChange(event.target.value); }}
      className="w-full h-8 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] text-xs">
      <option value="">Choose an audio app…</option>
      {apps.map(app => <option key={app.exe} value={app.exe}>{app.exe} ({app.sessions} {app.sessions === 1 ? "session" : "sessions"})</option>)}
    </select>
    <input id={id} value={value} onChange={event => onChange(event.target.value)} placeholder="Or enter spotify.exe"
      className="w-full h-8 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] text-xs" />
    {error && <p role="alert" className="text-[11px] text-[var(--status-warning)]">{error}</p>}
    {!busy && !error && apps.length === 0 && <p className="text-[11px] text-[var(--text-muted)]">No audio apps found. Play audio in the app, then refresh.</p>}
    <p className="text-[11px] text-[var(--text-muted)] leading-relaxed">Targets all available sessions for this executable across active outputs. Volume changes by 2 percentage points; mute switches its sessions together. Browser tabs may share an app. An app without an audio session reports unavailable; master volume is unchanged.</p>
  </div>;
}
