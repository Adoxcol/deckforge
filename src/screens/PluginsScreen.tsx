import React, { useState } from "react";
import { Download, ExternalLink, Puzzle, RefreshCw, Search, Star, TriangleAlert, Upload, X } from "lucide-react";
import { deckforgeApi } from "../bridge/tauriBridge";
import type { ActionTemplate } from "../data/actionCatalog";
import { errorText } from "../lib/format";
import type { GithubPlugin, LegacyStreamDockSummary, PluginInstalled, PluginView } from "../types/deckforge";

type Notify = (text: string, kind?: "success" | "info" | "warning") => void;

interface PluginsScreenProps {
  legacySummary: LegacyStreamDockSummary | null;
  onImportLegacy: () => void;
  plugins: PluginView[];
  onPluginsChange: (plugins: PluginView[]) => void;
  notify: Notify;
}

type PluginTab = "Installed plugins" | "Browse" | "Built-in actions";

const MAX_FILE_BYTES = 200 * 1024 * 1024;

function toBase64(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf);
  let out = "";
  for (let i = 0; i < bytes.length; i += 0x8000) out += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(out);
}

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
  const fileInput = React.useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<GithubPlugin[] | null>(null);
  const [searching, setSearching] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [rescanning, setRescanning] = useState(false);
  const confirmRef = React.useRef<HTMLDivElement>(null);

  const installed = (result: PluginInstalled) => {
    onPluginsChange(result.plugins);
    notify(`Installed ${result.name}. Turn it on in Installed plugins to use it.`, "success");
  };

  const installFile = async (file: File) => {
    if (file.size > MAX_FILE_BYTES) {
      notify("That file is too big for a plugin.", "warning");
      return;
    }
    setInstalling(file.name);
    try {
      installed(await deckforgeApi.pluginInstallFile(toBase64(await file.arrayBuffer())));
      setTab("Installed plugins");
    } catch (error) {
      notify(errorText(error), "warning");
    } finally {
      setInstalling(null);
    }
  };

  const search = async () => {
    setSearching(true);
    try {
      setResults(await deckforgeApi.pluginGithubSearch(query));
    } catch (error) {
      notify(errorText(error), "warning");
    } finally {
      setSearching(false);
    }
  };

  const installRepo = async (repo: string) => {
    setInstalling(repo);
    try {
      installed(await deckforgeApi.pluginGithubInstall(repo));
    } catch (error) {
      notify(errorText(error), "warning");
    } finally {
      setInstalling(null);
    }
  };

  React.useEffect(() => {
    if (tab === "Browse" && results === null && !searching) void search();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab]);

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
    setRescanning(true);
    try {
      onPluginsChange(await deckforgeApi.pluginRescan());
    } catch (error) {
      notify(errorText(error), "warning");
    } finally {
      setRescanning(false);
    }
  };

  React.useEffect(() => {
    if (!confirming) return;
    const previous = document.activeElement as HTMLElement | null;
    confirmRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        setConfirming(null);
        return;
      }
      if (e.key !== "Tab") return;
      const buttons = Array.from(confirmRef.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []);
      if (!buttons.length) return;
      const idx = buttons.indexOf(document.activeElement as HTMLButtonElement);
      e.preventDefault();
      buttons[(idx + (e.shiftKey ? -1 : 1) + buttons.length) % buttons.length].focus();
    };
    document.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      previous?.focus();
    };
  }, [confirming]);

  const builtIns = (legacySummary?.discoveredPlugins ?? []).filter((p) => p.sourceKind === "BuiltIn");

  return (
    <div className="flex-1 min-h-0 overflow-y-auto animate-screen-in">
    <div className="p-6 max-w-6xl mx-auto w-full flex flex-col gap-5">
      <div className="flex items-center justify-between flex-wrap gap-4 pb-4 border-b border-[var(--border-subtle)]">
        <div>
          <h1 className="text-xl font-bold">Plugins</h1>
          <p className="text-xs text-[var(--text-secondary)] mt-0.5">
            DeckForge runs Stream Dock and Stream Deck plugins. Turn one on, then pick its actions for a key.
          </p>
        </div>
        <div role="tablist" aria-label="Plugin sections" className="flex gap-1 bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] p-1 rounded-lg shrink-0">
          {(["Installed plugins", "Browse", "Built-in actions"] as const).map((t) => (
            <button
              key={t}
              type="button"
              role="tab"
              aria-selected={tab === t}
              onClick={() => setTab(t)}
              className={`px-3 py-1.5 rounded-md text-xs font-medium cursor-pointer transition-all border ${
                tab === t
                  ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-[var(--text-primary)] font-semibold shadow-xs"
                  : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
              }`}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      {tab === "Installed plugins" && (
        <>
          <div className="flex items-center justify-between flex-wrap gap-3 text-xs text-[var(--text-secondary)]">
            <span>
              {plugins.length} found in the Stream Dock, Stream Deck and DeckForge plugin folders.
              {legacySummary && legacySummary.namedRootProfilesCount > 0 && (
                <>
                  {" "}
                  <button type="button" onClick={onImportLegacy} className="underline text-[var(--accent-primary)] hover:text-[var(--accent-hover)] cursor-pointer transition-colors">
                    Import Stream Dock profiles
                  </button>
                </>
              )}
            </span>
            <div className="flex gap-2 shrink-0">
            <input
              ref={fileInput}
              type="file"
              accept=".streamDeckPlugin,.zip"
              className="hidden"
              onChange={(e) => {
                const file = e.target.files?.[0];
                e.target.value = "";
                if (file) void installFile(file);
              }}
            />
            <button
              type="button"
              disabled={installing !== null}
              onClick={() => fileInput.current?.click()}
              className="h-8 px-3 rounded-md border border-[var(--border-subtle)] bg-[var(--bg-panel)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] active:scale-[0.98] text-xs font-medium text-[var(--text-primary)] flex items-center gap-1.5 cursor-pointer disabled:opacity-50 transition-all"
            >
              <Upload size={13} /> {installing ? "Installing…" : "Install from file"}
            </button>
            <button
              type="button"
              disabled={rescanning}
              onClick={rescan}
              className="h-8 px-3 rounded-md border border-[var(--border-subtle)] bg-[var(--bg-panel)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] active:scale-[0.98] text-xs font-medium text-[var(--text-primary)] flex items-center gap-1.5 cursor-pointer disabled:opacity-60 transition-all"
            >
              <RefreshCw size={13} className={rescanning ? "animate-spin" : ""} /> {rescanning ? "Scanning…" : "Look again"}
            </button>
            </div>
          </div>
          {plugins.length === 0 && (
            <div className="p-6 rounded-xl border border-dashed border-[var(--border-subtle)] text-sm text-[var(--text-secondary)] text-center">
              No plugins found. Install them with the Stream Dock app, or copy a <code>.sdPlugin</code> folder into
              <code> %APPDATA%\DeckForge\plugins</code>.
            </div>
          )}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {plugins.map((plug) => {
              const status = statusOf(plug);
              const runnable = plug.runtime !== "unknown";
              return (
                <div key={plug.uuid} className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:-translate-y-[1px] hover:shadow-md df-card-surface transition-all flex flex-col gap-3">
                  <div className="flex items-start gap-3">
                    {plug.icon ? (
                      <img src={plug.icon} alt="" className="w-9 h-9 rounded-md shrink-0 object-contain bg-black/20 border border-[var(--border-subtle)] p-0.5" />
                    ) : (
                      <span className="w-9 h-9 rounded-md shrink-0 grid place-items-center bg-[var(--bg-sidebar)] text-[var(--text-secondary)]">
                        <Puzzle size={18} />
                      </span>
                    )}
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-bold text-sm truncate">{plug.name}</span>
                        <span className={`inline-flex items-center gap-1.5 text-[10px] px-2 py-0.5 rounded-full font-semibold shrink-0 ${TONE[status.tone]}`}>
                          <span className="w-1.5 h-1.5 rounded-full bg-current" aria-hidden />
                          {status.text}
                        </span>
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
                    <div className="flex gap-2 px-2.5 py-2 rounded-lg bg-[var(--status-warning)]/10 border border-[var(--status-warning)]/25 text-[11px] text-[var(--status-warning)] leading-snug">
                      <TriangleAlert size={13} className="shrink-0 mt-0.5" />
                      <span>{plug.problem}</span>
                    </div>
                  )}
                  <div className="mt-auto pt-2 border-t border-[var(--border-subtle)] flex items-center justify-end">
                    {runnable ? (
                      <button
                        type="button"
                        disabled={busy === plug.uuid}
                        onClick={() => (plug.enabled ? setEnabled(plug, false) : setConfirming(plug))}
                        className={`h-8 px-3 rounded-md text-xs font-semibold cursor-pointer active:scale-[0.98] disabled:opacity-50 disabled:active:scale-100 transition-all ${
                          plug.enabled
                            ? "border border-[var(--border-subtle)] text-[var(--text-primary)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-elevated)]"
                            : "bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white df-cta"
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

      {tab === "Browse" && (
        <>
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              void search();
            }}
          >
            <div className="flex-1 flex items-center gap-2 h-9 px-3 rounded-md bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] focus-within:border-[var(--accent-primary)] focus-within:ring-2 focus-within:ring-[var(--accent-primary)]/20 transition-all">
              <Search size={14} className="text-[var(--text-muted)] shrink-0" />
              <input
                aria-label="Search open-source plugins on GitHub"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search open-source plugins on GitHub (obs, spotify, timer…)"
                className="flex-1 bg-transparent outline-none focus-ring-none text-sm text-[var(--text-primary)] placeholder:text-[var(--text-muted)]"
              />
              {query && (
                <button
                  type="button"
                  aria-label="Clear plugin search"
                  onClick={() => setQuery("")}
                  className="p-0.5 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"
                >
                  <X size={13} />
                </button>
              )}
            </div>
            <button type="submit" disabled={searching} className="h-9 px-4 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] active:scale-[0.98] text-xs font-semibold text-white cursor-pointer disabled:opacity-50 shadow-xs transition-all">
              {searching ? "Searching…" : "Search"}
            </button>
          </form>
          <p className="text-[11px] text-[var(--text-muted)] leading-snug">
            These are GitHub projects tagged <code>streamdeck-plugin</code>. Install takes the plugin file from the
            project&apos;s releases, so projects without one can&apos;t be installed here. Downloads from GitHub can take a minute.
          </p>
          {searching && (!results || results.length === 0) && (
            <div className="p-6 rounded-xl border border-dashed border-[var(--border-subtle)] text-xs text-[var(--text-secondary)] flex items-center justify-center gap-2">
              <RefreshCw size={14} className="animate-spin text-[var(--accent-primary)]" />
              <span>Searching GitHub for Stream Deck plugins…</span>
            </div>
          )}
          {results && results.length === 0 && !searching && (
            <div className="p-6 rounded-xl border border-dashed border-[var(--border-subtle)] text-sm text-[var(--text-secondary)] text-center">Nothing found.</div>
          )}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {(results ?? []).map((r) => (
              <div key={r.repo} className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:-translate-y-[1px] hover:shadow-md transition-all flex flex-col gap-2">
                <div className="flex items-start gap-3">
                  {r.ownerAvatar ? (
                    <img src={r.ownerAvatar} alt="" className="w-8 h-8 rounded-md shrink-0" />
                  ) : (
                    <span className="w-8 h-8 rounded-md shrink-0 grid place-items-center bg-[var(--bg-sidebar)] text-[var(--text-secondary)]"><Puzzle size={16} /></span>
                  )}
                  <div className="min-w-0 flex-1">
                    <div className="font-bold text-sm truncate">{r.name}</div>
                    <div className="text-[11px] text-[var(--text-muted)] truncate flex items-center gap-1 tabular-nums">
                      {r.repo} • <Star size={10} /> {r.stars}
                    </div>
                  </div>
                </div>
                {r.description && <p className="text-xs text-[var(--text-secondary)] leading-relaxed line-clamp-2">{r.description}</p>}
                <div className="mt-auto pt-2 border-t border-[var(--border-subtle)] flex items-center justify-end gap-2">
                  <button
                    type="button"
                    onClick={() => deckforgeApi.pluginOpenPage(r.url).catch((e) => notify(errorText(e), "warning"))}
                    className="h-8 px-3 rounded-md border border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-elevated)] text-xs flex items-center gap-1.5 cursor-pointer transition-colors"
                  >
                    <ExternalLink size={12} /> Open page
                  </button>
                  <button
                    type="button"
                    disabled={installing !== null}
                    onClick={() => void installRepo(r.repo)}
                    className="h-8 px-3 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white flex items-center gap-1.5 cursor-pointer disabled:opacity-50 transition-colors"
                  >
                    <Download size={12} /> {installing === r.repo ? "Downloading…" : "Install"}
                  </button>
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      {tab === "Built-in actions" && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {builtIns.map((plug) => (
            <div key={plug.id} className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] df-card-surface flex flex-col justify-between gap-3 transition-colors">
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
        <div className="fixed inset-0 z-[80] bg-black/65 backdrop-blur-[2px] grid place-items-center p-4 animate-fade-in" onMouseDown={() => setConfirming(null)}>
          <div
            ref={confirmRef}
            role="dialog"
            aria-modal="true"
            aria-labelledby="plugin-confirm-title"
            className="w-full max-w-md rounded-2xl bg-[var(--bg-panel)] border border-[var(--border-strong)] p-5 flex flex-col gap-3 df-modal-surface animate-pop-in"
            onMouseDown={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-lg bg-[var(--accent-soft)] border border-[var(--accent-primary)]/35 text-[var(--accent-text)] flex items-center justify-center shrink-0">
                <Puzzle size={16} />
              </div>
              <h2 id="plugin-confirm-title" className="text-sm font-bold text-[var(--text-primary)]">Turn on {confirming.name}?</h2>
            </div>
            <p className="text-xs text-[var(--text-secondary)] leading-relaxed">
              A plugin is its own program. It runs on this PC with your user rights, the same as any app you install, and
              DeckForge can't limit what it does. Only turn on plugins you trust.
            </p>
            <p className="text-[11px] font-mono text-[var(--text-muted)] break-all bg-[var(--bg-sidebar)] px-2.5 py-1.5 rounded-md border border-[var(--border-subtle)]">{confirming.dir}</p>
            <div className="flex justify-end gap-2 pt-2 border-t border-[var(--border-subtle)]">
              <button type="button" autoFocus onClick={() => setConfirming(null)} className="h-8 px-3 rounded-md border border-[var(--border-subtle)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-xs font-medium cursor-pointer transition-colors">
                Cancel
              </button>
              <button
                type="button"
                onClick={() => {
                  const plug = confirming;
                  setConfirming(null);
                  void setEnabled(plug, true);
                }}
                className="h-8 px-3.5 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white cursor-pointer transition-colors"
              >
                Turn on
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
    </div>
  );
};
