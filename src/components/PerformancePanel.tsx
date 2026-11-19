import { useCallback, useEffect, useRef, useState } from "react";
import { deckforgeApi, isTauriRuntime } from "../bridge/tauriBridge";
import type { PerformanceSnapshot } from "../types/deckforge";

const duration = (value: number | null) => value === null ? "—" : `${value.toFixed(2)} ms`;

export function PerformancePanel() {
  const [snapshot, setSnapshot] = useState<PerformanceSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const request = useRef(0);
  const desktop = isTauriRuntime();
  const refresh = useCallback(async (reset = false) => {
    const id = ++request.current;
    setBusy(true);
    setError("");
    try {
      const next = await deckforgeApi.getPerformanceSnapshot(reset);
      if (id === request.current) setSnapshot(next);
    } catch (err) {
      if (id === request.current) setError(String(err));
    } finally {
      if (id === request.current) setBusy(false);
    }
  }, []);
  useEffect(() => {
    if (desktop) void refresh();
    return () => { request.current++; };
  }, [desktop, refresh]);

  return <section className="bg-[var(--bg-panel)] border border-[var(--border-subtle)] rounded-xl p-4 flex flex-col gap-3">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <h2 className="text-sm font-bold">Performance measurements</h2>
      {desktop && <div className="flex gap-2">
        <button type="button" disabled={busy} onClick={() => void refresh()} className="px-3 py-1.5 rounded-md border border-[var(--border-subtle)] bg-[var(--bg-sidebar)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-xs font-medium cursor-pointer disabled:opacity-50 disabled:cursor-default">{busy ? "Reading…" : "Refresh"}</button>
        <button type="button" disabled={busy} onClick={() => void refresh(true)} className="px-3 py-1.5 rounded-md border border-[var(--border-subtle)] bg-[var(--bg-sidebar)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-xs font-medium cursor-pointer disabled:opacity-50 disabled:cursor-default">Clear samples</button>
      </div>}
    </div>
    <p className="text-xs text-[var(--text-secondary)] leading-relaxed">
      {desktop ? "Clear samples, reproduce the delay, then refresh. Timings stay in memory for this app session; no key labels or action parameters are recorded." : "Open the desktop app to measure real performance. Browser preview does not generate sample timings."}
    </p>
    {error && <p role="alert" className="text-xs text-[var(--status-warning)]">Could not refresh measurements: {error}. Any table below is the last successful reading.</p>}
    {snapshot && <>
      <p className="text-xs text-[var(--text-muted)]">Snapshot after {Math.floor(snapshot.elapsedSeconds)} seconds. Statistics use the latest {snapshot.windowSize} completions per stage; count includes all completions since clearing. In-flight work is counted when it finishes.</p>
      <div className="overflow-x-auto">
        <table className="w-full text-xs text-left whitespace-nowrap">
          <thead className="text-[var(--text-muted)]"><tr>{["Stage", "Count", "Recent", "Last", "Median", "p95", "Worst"].map(label => <th key={label} scope="col" className="px-2 py-2 font-medium">{label}</th>)}</tr></thead>
          <tbody>{snapshot.timings.map(row => <tr key={row.label} className="border-t border-[var(--border-subtle)]">
            <th scope="row" className="px-2 py-2 font-medium">{row.label}</th>
            <td className="px-2 py-2 tabular-nums">{row.count}</td><td className="px-2 py-2 tabular-nums">{row.samples}</td>
            {[row.lastMs, row.medianMs, row.p95Ms, row.maxMs].map((value, index) => <td key={index} className="px-2 py-2 tabular-nums">{duration(value)}</td>)}
          </tr>)}</tbody>
        </table>
      </div>
      <p className="text-xs text-[var(--text-muted)] leading-relaxed">p95: 95% of recent samples finished within this time. A dash means no samples. Stages can overlap; do not add them together. Action handling measures dispatch before panel wake-up; asynchronous macros and Game Mode report dispatch only. USB execution includes protocol waits. Failed operations are included where measured. These are internal timings, not physical button-to-screen latency or CPU usage.</p>
    </>}
  </section>;
}
