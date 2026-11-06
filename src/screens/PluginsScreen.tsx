import React, { useState } from "react";
import type { LegacyStreamDockSummary } from "../types/deckforge";

interface PluginsScreenProps {
  legacySummary: LegacyStreamDockSummary | null;
  onImportLegacy: () => void;
}

type PluginTab = "Discover" | "Installed" | "Updates" | "Built-in Actions";

/** Built-in integrations and plugins found from the vendor's app. */
export const PluginsScreen: React.FC<PluginsScreenProps> = ({ legacySummary, onImportLegacy }) => {
  const [pluginFilterTab, setPluginFilterTab] = useState<PluginTab>("Discover");

  return (    <div className="flex-1 p-6 overflow-y-auto max-w-6xl mx-auto w-full flex flex-col gap-5">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold">Actions & Plugin Marketplace</h1>
          <p className="text-xs text-[#A6AFC4] mt-0.5">
            Built-in native Rust/TS integrations + discovered local{" "}
            <code>%APPDATA%\HotSpot\StreamDock\plugins</code> packages.
          </p>
        </div>
        <div className="flex gap-1 bg-[#10131D] border border-[#272D3D] p-1 rounded-lg">
          {(["Discover", "Installed", "Updates", "Built-in Actions"] as const).map((t) => (
            <button
              key={t}
              onClick={() => setPluginFilterTab(t)}
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
              ✓ Legacy StreamDock Environment Detected:
            </span>{" "}
            <span className="text-[#F4F5FA]">
              {legacySummary.legacyProfileCount} <code>.sdProfile</code> folders &{" "}
              {legacySummary.legacyPluginCount} <code>.sdPlugin</code> packages found on disk.
            </span>
          </div>
          <button
            onClick={onImportLegacy}
            className="h-8 px-3 rounded-md bg-[#8B5CF6] hover:bg-[#9D75F8] text-xs font-semibold text-white cursor-pointer"
          >
            Import Legacy .sdProfiles
          </button>
        </div>
      )}

      <div className="grid grid-cols-3 gap-4">
        {(legacySummary?.discoveredPlugins || []).map((plug) => (
          <div
            key={plug.id}
            className="p-4 rounded-xl bg-[#151925] border border-[#272D3D] flex flex-col justify-between gap-3"
          >
            <div>
              <div className="flex items-center justify-between gap-2">
                <span className="font-bold text-sm truncate">{plug.name}</span>
                <span
                  className={`text-[10px] px-2 py-0.5 rounded font-mono ${
                    plug.enabled
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
                Permissions: {plug.permissions.join(", ")}
              </span>
              <span
                className={`text-xs font-semibold ${
                  plug.enabled ? "text-[#35C98B]" : "text-[#F4B860]"
                }`}
              >
                {plug.enabled ? "Active" : "Quarantined"}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
