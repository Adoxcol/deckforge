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
          <h1 className="text-xl font-bold">Actions & Plugin Inventory</h1>
          <p className="text-xs text-[#A6AFC4] mt-0.5">
            Built-in actions are available in DeckForge. Vendor plugins are listed for reference only;
            DeckForge cannot run their actions. Importing profiles does not install a plugin runtime.
          </p>
        </div>
        <div className="flex gap-1 bg-[#10131D] border border-[#272D3D] p-1 rounded-lg">
          {(["Built-in Actions", "Discovered Vendor Plugins"] as const).map((t) => (
            <button
              key={t}
              onClick={() => setPluginFilterTab(t)}
              aria-pressed={pluginFilterTab === t}
              className={`px-3 py-1.5 rounded text-xs font-medium cursor-pointer ${
                pluginFilterTab === t
                  ? "bg-[#30244F] text-white font-semibold"
                  : "text-[#A6AFC4] hover:text-white"
              }`}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      {legacySummary && (
        <div className="p-3.5 rounded-xl bg-[#151925] border border-[#8B5CF6]/40 flex items-center justify-between">
          <div className="text-xs">
            <span className="font-bold text-[#35C98B]">
              Vendor files found:
            </span>{" "}
            <span className="text-[#F4F5FA]">
              {legacySummary.legacyProfileCount} <code>.sdProfile</code> folders &{" "}
              {legacySummary.legacyPluginCount} <code>.sdPlugin</code> packages found on disk.
            </span>
          </div>
          <button
            onClick={onImportLegacy}
            disabled={legacySummary.namedRootProfilesCount === 0}
            className="h-8 px-3 rounded-md bg-[#8B5CF6] hover:bg-[#9D75F8] text-xs font-semibold text-white cursor-pointer disabled:opacity-40 disabled:cursor-default"
          >
            Import Legacy .sdProfiles
          </button>
        </div>
      )}

      {!legacySummary && <p className="text-sm text-[#A6AFC4]">Plugin inventory is unavailable.</p>}
      {legacySummary && plugins.length === 0 && <p className="text-sm text-[#A6AFC4]">No packages found in this category.</p>}
      <div className="grid grid-cols-3 gap-4">
        {plugins.map((plug) => (
          <div
            key={plug.id}
            className="p-4 rounded-xl bg-[#151925] border border-[#272D3D] flex flex-col justify-between gap-3"
          >
            <div>
              <div className="flex items-center justify-between gap-2">
                <span className="font-bold text-sm truncate">{plug.name}</span>
                <span
                  className={`text-[10px] px-2 py-0.5 rounded font-mono ${
                    plug.sourceKind === "BuiltIn"
                      ? "bg-[#35C98B]/15 text-[#35C98B]"
                      : "bg-[#F4B860]/15 text-[#F4B860]"
                  }`}
                >
                  v{plug.version}
                </span>
              </div>
              <div className="text-[11px] text-[#707B94] mt-0.5">
                By {plug.author} • {plug.sourceKind}
              </div>
              <p className="text-xs text-[#A6AFC4] mt-2 leading-relaxed">
                {plug.description}
              </p>
            </div>
            <div className="pt-2 border-t border-[#272D3D] flex items-center justify-between">
              <span className="text-[10.5px] text-[#707B94] truncate">
                {plug.sourceKind === "BuiltIn" ? `Uses: ${plug.permissions.join(", ")}` : "Discovered on disk only"}
              </span>
              <span
                className={`text-xs font-semibold ${
                  plug.sourceKind === "BuiltIn" ? "text-[#35C98B]" : "text-[#F4B860]"
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
