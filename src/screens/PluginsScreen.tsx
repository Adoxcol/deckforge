import React, { useState } from "react";
import type { LegacyStreamDockSummary } from "../types/deckforge";

interface PluginsScreenProps {
  legacySummary: LegacyStreamDockSummary | null;
  onImportLegacy: () => void;
}

type PluginTab = "Built-in Actions" | "Discovered Vendor Plugins";

/** Built-in integrations and plugins found from the vendor's app. */
export const PluginsScreen: React.FC<PluginsScreenProps> = ({ legacySummary, onImportLegacy }) => {
  const [pluginFilterTab, setPluginFilterTab] = useState<PluginTab>("Built-in Actions");

  const plugins = (legacySummary?.discoveredPlugins ?? []).filter(plug =>
    pluginFilterTab === "Built-in Actions" ? plug.sourceKind === "BuiltIn" : plug.sourceKind !== "BuiltIn");

  return (    <div className="flex-1 p-6 overflow-y-auto max-w-6xl mx-auto w-full flex flex-col gap-5">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold">Plugins</h1>
          <p className="text-xs text-[var(--text-secondary)] mt-0.5">
            Built-in actions are available in DeckForge. Vendor plugins are listed for reference only;
            DeckForge cannot run their actions. Importing profiles does not install a plugin runtime.
          </p>
        </div>
        <div className="flex gap-1 bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] p-1 rounded-lg">
          {(["Built-in Actions", "Discovered Vendor Plugins"] as const).map((t) => (
            <button
              key={t}
              onClick={() => setPluginFilterTab(t)}
              aria-pressed={pluginFilterTab === t}
              className={`px-3 py-1.5 rounded text-xs font-medium cursor-pointer ${
                pluginFilterTab === t
                  ? "bg-[var(--accent-soft)] text-white font-semibold"
                  : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
              }`}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      {legacySummary && (
        <div className="p-3.5 rounded-xl bg-[var(--bg-panel)] border border-[var(--accent-primary)]/40 flex items-center justify-between">
          <div className="text-xs">
            <span className="font-bold text-[var(--status-success)]">
              Vendor files found:
            </span>{" "}
            <span className="text-[var(--text-primary)]">
              {legacySummary.legacyProfileCount} <code>.sdProfile</code> folders &{" "}
              {legacySummary.legacyPluginCount} <code>.sdPlugin</code> packages found on disk.
            </span>
          </div>
          <button
            onClick={onImportLegacy}
            disabled={legacySummary.namedRootProfilesCount === 0}
            className="h-8 px-3 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white cursor-pointer disabled:opacity-40 disabled:cursor-default"
          >
            Import Legacy .sdProfiles
          </button>
        </div>
      )}

      {!legacySummary && <p className="text-sm text-[var(--text-secondary)]">Plugin inventory is unavailable.</p>}
      {legacySummary && plugins.length === 0 && <p className="text-sm text-[var(--text-secondary)]">No packages found in this category.</p>}
      <div className="grid grid-cols-3 gap-4">
        {plugins.map((plug) => (
          <div
            key={plug.id}
            className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] flex flex-col justify-between gap-3"
          >
            <div>
              <div className="flex items-center justify-between gap-2">
                <span className="font-bold text-sm truncate">{plug.name}</span>
                <span
                  className={`text-[10px] px-2 py-0.5 rounded font-mono ${
                    plug.sourceKind === "BuiltIn"
                      ? "bg-[var(--status-success)]/15 text-[var(--status-success)]"
                      : "bg-[var(--status-warning)]/15 text-[var(--status-warning)]"
                  }`}
                >
                  v{plug.version}
                </span>
              </div>
              <div className="text-[11px] text-[var(--text-muted)] mt-0.5">
                By {plug.author} • {plug.sourceKind}
              </div>
              <p className="text-xs text-[var(--text-secondary)] mt-2 leading-relaxed">
                {plug.description}
              </p>
            </div>
            <div className="pt-2 border-t border-[var(--border-subtle)] flex items-center justify-between">
              <span className="text-[10.5px] text-[var(--text-muted)] truncate">
                {plug.sourceKind === "BuiltIn" ? `Uses: ${plug.permissions.join(", ")}` : "Discovered on disk only"}
              </span>
              <span
                className={`text-xs font-semibold ${
                  plug.sourceKind === "BuiltIn" ? "text-[var(--status-success)]" : "text-[var(--status-warning)]"
                }`}
              >
                {plug.sourceKind === "BuiltIn" ? "Built-in" : "Unsupported"}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
