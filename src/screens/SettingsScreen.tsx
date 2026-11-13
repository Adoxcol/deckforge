import React, { useEffect, useState } from "react";
import { ArrowUpRight, BookOpen, Database, Info, Monitor, Power, Usb } from "lucide-react";
import { deckforgeApi, isTauriRuntime } from "../bridge/tauriBridge";
import type { NavScreenId } from "../types/deckforge";
import packageInfo from "../../package.json";

const sections = [
  { id: "general", name: "General", hint: "Startup and background", icon: Power },
  { id: "devices", name: "Devices & integrations", hint: "Hardware and actions", icon: Usb },
  { id: "data", name: "Data & backups", hint: "Save, export and recovery", icon: Database },
  { id: "about", name: "About", hint: "Version and support", icon: Info },
] as const;
type Section = typeof sections[number]["id"];
const card = "rounded-xl border border-[#272D3D] bg-[#151925] p-5 space-y-3";
const action = "inline-flex items-center gap-2 rounded-lg border border-[#394158] px-3 py-2 text-xs font-medium text-[#E0E4EE] hover:border-[#8B5CF6] hover:bg-[#30244F] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[#8B5CF6]";

export const SettingsScreen: React.FC<{ onNavigate: (screen: NavScreenId) => void; onOpenGuide: () => void }> = ({ onNavigate, onOpenGuide }) => {
  const [section, setSection] = useState<Section>("general");
  const [launchAtLogin, setLaunchAtLogin] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const desktop = isTauriRuntime();
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
  const link = (label: string, screen: NavScreenId) => <button type="button" className={action} onClick={() => onNavigate(screen)}>{label}<ArrowUpRight size={14} /></button>;
  const selected = sections.find(item => item.id === section)!;

  return <div className="flex-1 min-h-0 grid grid-cols-[220px_1fr] overflow-hidden">
    <nav aria-label="Settings sections" className="bg-[#10131D] border-r border-[#272D3D] p-4 space-y-2">
      <p className="px-3 py-2 text-[10px] font-semibold uppercase tracking-widest text-[#707B94]">Preferences</p>
      {sections.map(({ id, name, hint, icon: Icon }) => <button type="button" key={id} onClick={() => setSection(id)}
        aria-current={section === id ? "page" : undefined} aria-controls="settings-content"
        className={`w-full flex items-start gap-3 text-left rounded-lg px-3 py-3 border transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-[#8B5CF6] ${section === id ? "border-[#8B5CF6]/50 bg-[#30244F] text-white" : "border-transparent text-[#A6AFC4] hover:bg-[#151925]"}`}>
        <Icon size={16} className="mt-0.5 shrink-0" /><span><span className="block text-xs font-semibold">{name}</span><span className="block text-[10px] text-[#A6AFC4] mt-1">{hint}</span></span>
      </button>)}
    </nav>
    <main id="settings-content" aria-labelledby="settings-heading" className="overflow-y-auto p-6 lg:p-8">
      <div className="max-w-3xl mx-auto space-y-5 text-sm text-[#A6AFC4]">
        <header className="flex items-start justify-between gap-4 pb-2">
          <div><h1 id="settings-heading" className="text-xl font-bold text-[#F4F5FA]">{selected.name}</h1><p className="text-xs mt-1">{selected.hint}</p></div>
          <span className="text-[10px] px-2.5 py-1 rounded-full border border-[#394158] text-[#C4CBDA]">{desktop ? "Desktop app" : "Browser preview"}</span>
        </header>
        {section === "general" && <>
          <section className={card} aria-labelledby="startup-heading">
            <div className="flex items-center gap-2 text-[#F4F5FA]"><Power size={17} /><h2 id="startup-heading" className="font-semibold">Windows sign-in</h2></div>
            <label className="flex gap-3 items-center py-2 text-[#E0E4EE]">
              <input type="checkbox" className="accent-[#8B5CF6] w-4 h-4" checked={launchAtLogin} disabled={loading || !desktop} onChange={toggleStartup} aria-describedby="startup-help" />
              Start DeckForge in the tray when I sign in
            </label>
            <p id="startup-help" className="text-xs leading-relaxed">{desktop ? "Uses this executable path. Enable from the installed app; if you move it, enable this setting again." : "This preference is available in the Windows desktop app. Browser preview does not change Windows settings."}</p>
            {loading && desktop && <p role="status" className="text-xs">Checking startup preference…</p>}
            {error && <p role="alert" className="text-xs text-[#F16B79]">{error}</p>}
          </section>
          <section className={card}><h2 className="font-semibold text-[#F4F5FA]">Close the editor, keep the deck running</h2>
            <p className="leading-relaxed">Closing the window hides DeckForge to the system tray. Use <strong className="text-[#E0E4EE]">Open DeckForge</strong> to return, or <strong className="text-[#E0E4EE]">Quit DeckForge</strong> to stop device control.</p>
            <p className="text-xs">Unsaved edits remain in memory while hidden. Save before restarting Windows. Quit offers Save, Discard, or Cancel.</p>
          </section>
          <button type="button" className={action} onClick={onOpenGuide}><BookOpen size={15} />Open quick-start guide</button>
        </>}
        {section === "devices" && <>
          <section className={card}><h2 className="font-semibold text-[#F4F5FA]">Connect and switch device control</h2>
            <p>The AKP05E Pro is the primary hardware target. Check Devices for connection status, brightness, and diagnostics.</p>
            <p className="text-xs leading-relaxed">Close AJAZZ and its tray watcher, then choose Use DeckForge in the status bar. To switch back, choose Pause DeckForge and wait for the paused status before opening AJAZZ.</p>
            {link("Open Devices", "devices")}
          </section>
          <section className={card}><h2 className="font-semibold text-[#F4F5FA]">Actions and integrations</h2>
            <p>Assign shortcuts, apps, media controls, or OBS actions in the workspace. Configure OBS connection details on an OBS action.</p>
            <p className="text-xs">Discovered AJAZZ plugins are inventory only. Their vendor actions cannot run in DeckForge.</p>
            <div className="flex flex-wrap gap-2">{link("Edit actions", "workspace")}{link("Open Macro Studio", "macros")}{link("View plugin inventory", "plugins")}</div>
          </section>
        </>}
        {section === "data" && <>
          <section className={card}><h2 className="font-semibold text-[#F4F5FA]">Save locally, then apply</h2>
            <p><strong className="text-[#E0E4EE]">Save</strong> stores profile edits on this PC. <strong className="text-[#E0E4EE]">Apply to Device</strong> saves and sends the selected page to the deck. Browsing or editing a different page does not change the applied bindings.</p>
            <p className="text-xs">Macros, auto-switch rules, and OBS settings save as you edit. Pending or failed saves appear above the workspace with a retry control.</p>
            {link("Open workspace", "workspace")}
          </section>
          <section className={card}><h2 className="font-semibold text-[#F4F5FA]">Profile sharing</h2>
            <p>Use Import and Export in the profile sidebar to share a profile and its referenced macros.</p>
            <div className="rounded-lg border border-[#F4B860]/30 bg-[#F4B860]/5 p-3 text-xs text-[#F4B860]">Profile export does not bundle staged images or animations. It is not a complete backup of your setup.</div>
          </section>
          <section className={card}><h2 className="font-semibold text-[#F4F5FA]">Local storage and recovery</h2>
            <p className="text-xs break-all font-mono text-[#E0E4EE]">%APPDATA%\DeckForge</p>
            <p className="text-xs leading-relaxed">The first explicit save of an older profile file preserves an original copy named profiles.pre-v1-N.json. This is a migration backup, not ongoing version history. Automatic recovery of unsaved edits after a crash is not available yet.</p>
          </section>
        </>}
        {section === "about" && <>
          <section className={card}><div className="flex items-center gap-3"><div className="p-3 rounded-xl bg-[#30244F] text-[#A98AF9]"><Monitor size={24} /></div><div><h2 className="text-lg font-bold text-[#F4F5FA]">DeckForge</h2><p className="text-xs">Version {packageInfo.version} · Windows desktop controller</p></div></div>
            <p>Build and manage layouts for your deck with local profiles, macros, and media.</p>
          </section>
          <section className={card}><h2 className="font-semibold text-[#F4F5FA]">Support and troubleshooting</h2>
            <p className="text-xs leading-relaxed">Frame deduplication reduces repeated image writes. Input polling and heartbeats still run; this does not guarantee freeze-free operation. Physical device, tray, sign-in, and installer validation are still in progress.</p>
            <div className="flex flex-wrap gap-2">{link("Device diagnostics", "devices")}<button type="button" className={action} onClick={onOpenGuide}>Quick-start guide<BookOpen size={14} /></button></div>
          </section>
        </>}
      </div>
    </main>
  </div>;
};
