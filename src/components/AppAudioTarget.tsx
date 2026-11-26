import type { AudioAppState } from "../types/deckforge";
import { appAudioLabel } from "../lib/appAudio";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import { deckforgeApi } from "../bridge/tauriBridge";

export function AppAudioTarget({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const [apps, setApps] = useState<AudioAppState[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const request = useRef(0);
  const inFlight = useRef(false);
  const id = useId();
  const isForeground = value.trim().toLowerCase() === "@foreground";
  const refresh = useCallback(async (force = false) => {
    if (inFlight.current) return;
    inFlight.current = true;
    const current = ++request.current;
    setBusy(true);
    setError("");
    try {
      const next = await deckforgeApi.audioApps(force);
      if (request.current === current) setApps(next);
    } catch (err) {
      if (request.current === current) setError(String(err));
    } finally {
      inFlight.current = false;
      if (request.current === current) setBusy(false);
    }
  }, []);
  useEffect(() => {
    if (isForeground) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => { await refresh(); if (active) timer = setTimeout(poll, 2000); };
    void poll();
    return () => { active = false; clearTimeout(timer); request.current++; };
  }, [refresh, isForeground]);
  const selected = apps.find(app => app.exe === value.trim().toLowerCase());
  return <div className="flex flex-col gap-2">
    <div className="flex items-center justify-between gap-2">
      <label htmlFor={id} className="text-[11px] text-[var(--text-secondary)]">Audio application</label>
      <button type="button" disabled={busy} onClick={() => void refresh(true)} className="text-[11px] font-medium text-[var(--accent-text)] hover:text-[var(--accent-hover)] hover:underline disabled:opacity-50 cursor-pointer disabled:cursor-default">{busy ? "Scanning…" : "Refresh apps"}</button>
    </div>
    <select aria-label="Choose an app with an audio session" value={isForeground ? "@foreground" : apps.some(app => app.exe === value) ? value : ""}
      onChange={event => { if (event.target.value) onChange(event.target.value); }}
      className="w-full h-8 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none cursor-pointer transition-colors">
      <option value="">Choose an audio app…</option>
      <option value="@foreground">App in foreground</option>
      {apps.map(app => <option key={app.exe} value={app.exe}>{app.exe} ({app.sessions} {app.sessions === 1 ? "session" : "sessions"})</option>)}
    </select>
    {isForeground ? <button type="button" onClick={() => onChange("")} className="self-start text-[11px] text-[var(--accent-text)] hover:underline cursor-pointer">Lock to a specific app instead</button> : <input id={id} value={value} onChange={event => onChange(event.target.value)} placeholder="Or enter spotify.exe"
      className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)] transition-colors" />}
    {!error && <p className="text-[11px] text-[var(--text-secondary)]">{isForeground ? "Uses the app in front when you press the control. Focus an app with an audio session first." : selected ? `Current: ${appAudioLabel(selected)}` : value.trim() ? "No current audio session" : "Choose an app to see its volume"}. MIX means only some sessions are muted.</p>}
    {error && <p role="alert" className="text-[11px] text-[var(--status-warning)]">{error}</p>}
    {!isForeground && !busy && !error && apps.length === 0 && <p className="text-[11px] text-[var(--text-muted)]">No audio apps found. Play audio in the app, then refresh.</p>}
    <p className="text-[11px] text-[var(--text-muted)] leading-relaxed">Targets all available sessions for {isForeground ? "the focused app" : "this executable"} across active outputs. Volume changes by 2 percentage points; mute switches its sessions together. Browser tabs may share an app. An app without an audio session reports unavailable; master volume is unchanged.</p>
  </div>;
}
