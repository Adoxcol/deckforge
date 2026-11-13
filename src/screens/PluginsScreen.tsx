import React, { useState } from "react";
import { Puzzle, RefreshCw, TriangleAlert } from "lucide-react";
import { deckforgeApi } from "../bridge/tauriBridge";
import type { ActionTemplate } from "../data/actionCatalog";
import { errorText } from "../lib/format";
import type { LegacyStreamDockSummary, PluginView } from "../types/deckforge";

type Notify = (text: string, kind?: "success" | "info" | "warning") => void;

interface PluginsScreenProps {
  legacySummary: LegacyStreamDockSummary | null;
  onImportLegacy: () => void;
  plugins: PluginView[];
  onPluginsChange: (plugins: PluginView[]) => void;
  notify: Notify;
}

type PluginTab = "Installed plugins" | "Built-in actions";

/** Actions of the plugins that are turned on, for the key action picker. */
export function pluginActionTemplates(plugins: PluginView[]): ActionTemplate[] {
  return plugins
    .filter((p) => p.enabled)
    .flatMap((p) =>
      p.actions.map((a) => ({
        id: `plugin:${a.uuid}`,
        title: a.name.slice(0, 18),
        subtitle: p.name.toUpperCase().slice(0, 10),
        category: `Plugin: ${p.name}` as const,
        iconName: "Puzzle",
        actionLabel: `${p.name}: ${a.name}`,
        defaultParam: "",
        accentColor: "#A78BFA",
        bgColor: "#1E1630",
        description: a.tooltip || `From the ${p.name} plugin`,
      }))
    );
}

const RUNTIME_LABEL: Record<PluginView["runtime"], string> = {
  node: "Node.js",
  html: "HTML",
  exe: "Program",
  unknown: "Unknown type",
};

function statusOf(p: PluginView): { text: string; tone: "ok" | "warn" | "muted" } {
  if (!p.enabled) return { text: "Off", tone: "muted" };
  if (p.problem) return { text: "Problem", tone: "warn" };
  if (p.running) return { text: "Running", tone: "ok" };
  return { text: "Starting…", tone: "muted" };
}

const TONE: Record<"ok" | "warn" | "muted", string> = {
  ok: "bg-[var(--status-success)]/15 text-[var(--status-success)]",
  warn: "bg-[var(--status-warning)]/15 text-[var(--status-warning)]",
  muted: "bg-[var(--bg-sidebar)] text-[var(--text-secondary)]",
};

/** Installed plugins, which the user can turn on, and DeckForge's own integrations. */
export const PluginsScreen: React.FC<PluginsScreenProps> = ({ legacySummary, onImportLegacy, plugins, onPluginsChange, notify }) => {
  const [tab, setTab] = useState<PluginTab>("Installed plugins");
  const [confirming, setConfirming] = useState<PluginView | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  // Starting a plugin takes a moment; check back so its status settles.
  React.useEffect(() => {
    if (!plugins.some((p) => p.enabled && !p.running && !p.problem)) return;
    const timer = window.setTimeout(() => {
      deckforgeApi.pluginList().then(onPluginsChange).catch(() => undefined);
    }, 1500);
    return () => window.clearTimeout(timer);
  }, [plugins, onPluginsChange]);

  const setEnabled = async (plugin: PluginView, enabled: boolean) => {
    setBusy(plugin.uuid);
    try {
      onPluginsChange(await deckforgeApi.pluginSetEnabled(plugin.uuid, enabled));
      notify(enabled ? `Turned on ${plugin.name}. Its actions are in the key action list.` : `Turned off ${plugin.name}.`, enabled ? "success" : "info");
    } catch (error) {
      notify(errorText(error), "warning");
    } finally {
      setBusy(null);
    }
  };

  const rescan = async () => {
    try {
      onPluginsChange(await deckforgeApi.pluginRescan());
    } catch (error) {
      notify(errorText(error), "warning");
    }
  };

  const builtIns = (legacySummary?.discoveredPlugins ?? []).filter((p) => p.sourceKind === "BuiltIn");

  return (
    <div className="flex-1 p-6 overflow-y-auto max-w-6xl mx-auto w-full flex flex-col gap-5">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-bold">Plugins</h1>
          <p className="text-xs text-[var(--text-secondary)] mt-0.5">
            DeckForge runs Stream Dock and Stream Deck plugins. Turn one on, then pick its actions for a key.
          </p>
        </div>
        <div className="flex gap-1 bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] p-1 rounded-lg shrink-0">
          {(["Installed plugins", "Built-in actions"] as const).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              aria-pressed={tab === t}
              className={`px-3 py-1.5 rounded text-xs font-medium cursor-pointer ${
                tab === t ? "bg-[var(--accent-soft)] text-white font-semibold" : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
              }`}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      {tab === "Installed plugins" && (
        <>
          <div className="flex items-center justify-between gap-3 text-xs text-[var(--text-secondary)]">
            <span>
              {plugins.length} found in the Stream Dock, Stream Deck and DeckForge plugin folders.
              {legacySummary && legacySummary.namedRootProfilesCount > 0 && (
                <>
                  {" "}
                  <button onClick={onImportLegacy} className="underline text-[var(--accent-primary)] cursor-pointer">
                    Import Stream Dock profiles
                  </button>
                </>
              )}
            </span>
            <button
              onClick={rescan}
              className="h-8 px-3 rounded-md border border-[var(--border-subtle)] hover:border-[var(--border-strong)] text-xs text-[var(--text-primary)] flex items-center gap-1.5 cursor-pointer"
            >
              <RefreshCw size={13} /> Look again
            </button>
          </div>
          {plugins.length === 0 && (
            <div className="p-6 rounded-xl border border-dashed border-[var(--border-subtle)] text-sm text-[var(--text-secondary)] text-center">
              No plugins found. Install them with the Stream Dock app, or copy a <code>.sdPlugin</code> folder into
              <code> %APPDATA%\DeckForge\plugins</code>.
            </div>
          )}
          <div className="grid grid-cols-3 gap-4">
            {plugins.map((plug) => {
              const status = statusOf(plug);
              const runnable = plug.runtime !== "unknown";
              return (
                <div key={plug.uuid} className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] flex flex-col gap-3">
                  <div className="flex items-start gap-3">
                    {plug.icon ? (
                      <img src={plug.icon} alt="" className="w-9 h-9 rounded-md shrink-0 object-contain bg-black/20" />
                    ) : (
                      <span className="w-9 h-9 rounded-md shrink-0 grid place-items-center bg-[var(--bg-sidebar)] text-[var(--text-secondary)]">
                        <Puzzle size={18} />
                      </span>
                    )}
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-bold text-sm truncate">{plug.name}</span>
                        <span className={`text-[10px] px-2 py-0.5 rounded font-semibold shrink-0 ${TONE[status.tone]}`}>{status.text}</span>
                      </div>
                      <div className="text-[11px] text-[var(--text-muted)] mt-0.5 truncate">
                        {[plug.author && `By ${plug.author}`, plug.version && `v${plug.version}`, plug.source, RUNTIME_LABEL[plug.runtime]].filter(Boolean).join(" • ")}
                      </div>
                    </div>
                  </div>
                  {plug.description && <p className="text-xs text-[var(--text-secondary)] leading-relaxed line-clamp-3">{plug.description}</p>}
                  <div className="text-[11px] text-[var(--text-muted)]">
                    {plug.actions.length} action{plug.actions.length === 1 ? "" : "s"}
                    {plug.actions.length > 0 && `: ${plug.actions.slice(0, 4).map((a) => a.name).join(", ")}${plug.actions.length > 4 ? "…" : ""}`}
                  </div>
                  {plug.enabled && plug.problem && (
                    <div className="flex gap-2 text-[11px] text-[var(--status-warning)] leading-snug">
                      <TriangleAlert size={13} className="shrink-0 mt-0.5" />
                      <span>{plug.problem}</span>
                    </div>
                  )}
                  <div className="mt-auto pt-2 border-t border-[var(--border-subtle)] flex items-center justify-end">
                    {runnable ? (
                      <button
                        disabled={busy === plug.uuid}
                        onClick={() => (plug.enabled ? setEnabled(plug, false) : setConfirming(plug))}
                        className={`h-8 px-3 rounded-md text-xs font-semibold cursor-pointer disabled:opacity-50 ${
                          plug.enabled
                            ? "border border-[var(--border-subtle)] text-[var(--text-primary)] hover:border-[var(--border-strong)]"
                            : "bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white"
                        }`}
                      >
                        {plug.enabled ? "Turn off" : "Turn on"}
                      </button>
                    ) : (
                      <span className="text-[11px] text-[var(--text-muted)]">DeckForge can't run this kind of plugin</span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
          <p className="text-[11px] text-[var(--text-muted)] leading-snug">
            Plugins that need Node.js use the one installed on this PC. Plugin settings pages are not shown yet, so plugins
            that must be set up there use their default settings.
          </p>
        </>
      )}

      {tab === "Built-in actions" && (
        <div className="grid grid-cols-3 gap-4">
          {builtIns.map((plug) => (
            <div key={plug.id} className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] flex flex-col justify-between gap-3">
              <div>
                <span className="font-bold text-sm">{plug.name}</span>
                <p className="text-xs text-[var(--text-secondary)] mt-2 leading-relaxed">{plug.description}</p>
              </div>
              <div className="pt-2 border-t border-[var(--border-subtle)] text-[10.5px] text-[var(--text-muted)] truncate">Uses: {plug.permissions.join(", ")}</div>
            </div>
          ))}
        </div>
      )}

      {confirming && (
        <div className="fixed inset-0 z-[80] bg-black/50 grid place-items-center p-4" onMouseDown={() => setConfirming(null)}>
          <div
            role="dialog"
            aria-label={`Turn on ${confirming.name}`}
            className="w-full max-w-md rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] p-5 flex flex-col gap-3 shadow-2xl"
            onMouseDown={(e) => e.stopPropagation()}
          >
            <h2 className="text-sm font-bold">Turn on {confirming.name}?</h2>
            <p className="text-xs text-[var(--text-secondary)] leading-relaxed">
              A plugin is its own program. It runs on this PC with your user rights, the same as any app you install, and
              DeckForge can't limit what it does. Only turn on plugins you trust.
            </p>
            <p className="text-[11px] text-[var(--text-muted)] break-all">{confirming.dir}</p>
            <div className="flex justify-end gap-2 pt-1">
              <button onClick={() => setConfirming(null)} className="h-8 px-3 rounded-md border border-[var(--border-subtle)] text-xs cursor-pointer">
                Cancel
              </button>
              <button
                onClick={() => {
                  const plug = confirming;
                  setConfirming(null);
                  void setEnabled(plug, true);
                }}
                className="h-8 px-3 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white cursor-pointer"
              >
                Turn on
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
