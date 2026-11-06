import React, { useState } from "react";

/** Read-only facts about how DeckForge works. */
export const SettingsScreen: React.FC = () => {
  const [settingsCategory, setSettingsCategory] = useState<string>("General");

  return (    <div className="flex-1 grid grid-cols-[220px_1fr] overflow-hidden">
      <aside className="bg-[#10131D] border-r border-[#272D3D] p-4 flex flex-col gap-1">
        {[
          "General",
          "Appearance",
          "Devices",
          "Actions",
          "Plugins",
          "Data & Backups",
          "Diagnostics",
          "About",
        ].map((cat) => (
          <button
            key={cat}
            onClick={() => setSettingsCategory(cat)}
            className={`text-left px-3 py-2 rounded-md text-xs font-medium cursor-pointer ${
              settingsCategory === cat
                ? "bg-[#30244F] text-white font-semibold"
                : "text-[#A6AFC4] hover:bg-[#151925] hover:text-white"
            }`}
          >
            {cat}
          </button>
        ))}
      </aside>

      <main className="p-6 overflow-y-auto max-w-3xl flex flex-col gap-4">
        <h1 className="text-lg font-bold">{settingsCategory} Settings</h1>
        <div className="p-4 rounded-xl bg-[#151925] border border-[#272D3D] flex flex-col gap-3 text-xs">
          <div className="flex items-center justify-between py-1 border-b border-[#272D3D]">
            <span>Anti-Freeze SHA-256 Frame Deduplication</span>
            <span className="text-[#35C98B] font-semibold">Enabled (Mandatory)</span>
          </div>
          <div className="flex items-center justify-between py-1 border-b border-[#272D3D]">
            <span>Idle USB Polling / Unconditional Redraws</span>
            <span className="text-[#35C98B] font-semibold">0% (Event-Driven Only)</span>
          </div>
          <div className="flex items-center justify-between py-1 border-b border-[#272D3D]">
            <span>External Telemetry / Cloud Account Requirement</span>
            <span className="text-[#35C98B] font-semibold">None (100% Local-First)</span>
          </div>
          <div className="flex items-center justify-between py-1">
            <span>Architecture</span>
            <span className="font-mono text-[#A6AFC4]">
              Tauri v2 + Rust 1.89 + React 19 + HIDAPI Windows Native
            </span>
          </div>
        </div>
      </main>
    </div>
  );
};
