import React, { useEffect, useState } from "react";

import { deckforgeApi, isTauriRuntime } from "../bridge/tauriBridge";

/** Desktop lifecycle preferences and runtime information. */
export const SettingsScreen: React.FC = () => {
  const [settingsCategory, setSettingsCategory] = useState<string>("General");

  const [launchAtLogin, setLaunchAtLogin] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    deckforgeApi.getLaunchAtLogin().then(value => { if (alive) setLaunchAtLogin(value); })
      .catch(error => { if (alive) setError(String(error)); })
      .finally(() => { if (alive) setLoading(false); });
    return () => { alive = false; };
  }, []);
  const toggleStartup = async () => {
    setLoading(true); setError(null);
    try { setLaunchAtLogin(await deckforgeApi.setLaunchAtLogin(!launchAtLogin)); }
    catch (error) { setError(error instanceof Error ? error.message : String(error)); }
    finally { setLoading(false); }
  };

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
        {settingsCategory === "General" && <div className="p-4 rounded-xl bg-[#151925] border border-[#272D3D] space-y-3 text-sm">
          <p>Closing the window keeps DeckForge and your editor session running in the system tray. Use Open DeckForge to return, or Quit DeckForge to stop device control.</p>
          <label className="flex gap-3 items-center">
            <input type="checkbox" checked={launchAtLogin} disabled={loading || !isTauriRuntime()} onChange={toggleStartup} />
            Start DeckForge in the tray when I sign in to Windows
          </label>
          <p className="text-xs text-[#A6AFC4]">Uses this executable path. Enable it from the installed app; if you move the executable, enable it again. Unsaved edits stay in memory while hidden; save before restarting Windows.</p>
          {error && <p role="alert" className="text-[#F16B79]">{error}</p>}
        </div>}
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
