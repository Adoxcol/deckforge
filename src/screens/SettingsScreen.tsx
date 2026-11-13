import React, { useEffect, useState } from "react";
import { ArrowUpRight, BookOpen, Database, Info, Monitor, Power, Usb } from "lucide-react";
import { deckforgeApi, isTauriRuntime } from "../bridge/tauriBridge";
import type { NavScreenId } from "../types/deckforge";
import { BackupPanel } from "../components/BackupPanel";
import { UpdatesPanel } from "../components/UpdatesPanel";
import packageInfo from "../../package.json";

const sections = [
  { id: "general", name: "General", hint: "Startup and background", icon: Power },
  { id: "devices", name: "Devices & integrations", hint: "Hardware and actions", icon: Usb },
  { id: "data", name: "Data & backups", hint: "Save, export and recovery", icon: Database },
  { id: "about", name: "About", hint: "Version and support", icon: Info },
] as const;
type Section = typeof sections[number]["id"];
const card = "rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-panel)] p-5 space-y-3";
const action = "inline-flex items-center gap-2 rounded-lg border border-[var(--border-strong)] px-3 py-2 text-xs font-medium text-[var(--text-primary)] hover:border-[var(--accent-primary)] hover:bg-[var(--accent-soft)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--accent-primary)]";

export const SettingsScreen: React.FC<{ onNavigate: (screen: NavScreenId) => void; onOpenGuide: () => void; onBackup: () => Promise<string>; onRestore: (contents: string) => Promise<number> }> = ({ onNavigate, onOpenGuide, onBackup, onRestore }) => {
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
    <nav aria-label="Settings sections" className="bg-[var(--bg-sidebar)] border-r border-[var(--border-subtle)] p-4 space-y-2">
      <p className="px-3 py-2 text-[10px] font-semibold uppercase tracking-widest text-[var(--text-muted)]">Preferences</p>
      {sections.map(({ id, name, hint, icon: Icon }) => <button type="button" key={id} onClick={() => setSection(id)}
        aria-current={section === id ? "page" : undefined} aria-controls="settings-content"
        className={`w-full flex items-start gap-3 text-left rounded-lg px-3 py-3 border transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--accent-primary)] ${section === id ? "border-[var(--accent-primary)]/50 bg-[var(--accent-soft)] text-white" : "border-transparent text-[var(--text-secondary)] hover:bg-[var(--bg-panel)]"}`}>
        <Icon size={16} className="mt-0.5 shrink-0" /><span><span className="block text-xs font-semibold">{name}</span><span className="block text-[10px] text-[var(--text-secondary)] mt-1">{hint}</span></span>
      </button>)}
    </nav>
    <main id="settings-content" aria-labelledby="settings-heading" className="overflow-y-auto p-6 lg:p-8">
      <div className="max-w-3xl mx-auto space-y-5 text-sm text-[var(--text-secondary)]">
        <header className="flex items-start justify-between gap-4 pb-2">
          <div><h1 id="settings-heading" className="text-xl font-bold text-[var(--text-primary)]">{selected.name}</h1><p className="text-xs mt-1">{selected.hint}</p></div>
          <span className="text-[10px] px-2.5 py-1 rounded-full border border-[var(--border-strong)] text-[var(--text-secondary)]">{desktop ? "Desktop app" : "Browser preview"}</span>
        </header>
        {section === "general" && <>
          <section className={card} aria-labelledby="startup-heading">
            <div className="flex items-center gap-2 text-[var(--text-primary)]"><Power size={17} /><h2 id="startup-heading" className="font-semibold">Windows sign-in</h2></div>
            <label className="flex gap-3 items-center py-2 text-[var(--text-primary)]">
              <input type="checkbox" className="accent-[var(--accent-primary)] w-4 h-4" checked={launchAtLogin} disabled={loading || !desktop} onChange={toggleStartup} aria-describedby="startup-help" />
              Start DeckForge in the tray when I sign in
            </label>
            <p id="startup-help" className="text-xs leading-relaxed">{desktop ? "Uses this executable path. Enable from the installed app; if you move it, enable this setting again." : "This preference is available in the Windows desktop app. Browser preview does not change Windows settings."}</p>
            {loading && desktop && <p role="status" className="text-xs">Checking startup preference…</p>}
            {error && <p role="alert" className="text-xs text-[var(--status-error)]">{error}</p>}
          </section>
          <section className={card}><h2 className="font-semibold text-[var(--text-primary)]">Close the editor, keep the deck running</h2>
            <p className="leading-relaxed">Closing the window hides DeckForge to the system tray. Use <strong className="text-[var(--text-primary)]">Open DeckForge</strong> to return, or <strong className="text-[var(--text-primary)]">Quit DeckForge</strong> to stop device control.</p>
            <p className="text-xs">Unsaved edits remain in memory while hidden. Save before restarting Windows. Quit offers Save, Discard, or Cancel.</p>
          </section>
          <button type="button" className={action} onClick={onOpenGuide}><BookOpen size={15} />Open quick-start guide</button>
        </>}
        {section === "devices" && <>
          <section className={card}><h2 className="font-semibold text-[var(--text-primary)]">Connect and switch device control</h2>
            <p>The AKP05E Pro is the primary hardware target. Check Devices for connection status, brightness, and diagnostics.</p>
            <p className="text-xs leading-relaxed">Close AJAZZ and its tray watcher, then choose Use DeckForge in the status bar. To switch back, choose Pause DeckForge and wait for the paused status before opening AJAZZ.</p>
            {link("Open Devices", "devices")}
          </section>
          <section className={card}><h2 className="font-semibold text-[var(--text-primary)]">Actions and integrations</h2>
            <p>Assign shortcuts, apps, media controls, or OBS actions in the workspace. Configure OBS connection details on an OBS action.</p>
            <p className="text-xs">Discovered AJAZZ plugins are inventory only. Their vendor actions cannot run in DeckForge.</p>
            <div className="flex flex-wrap gap-2">{link("Edit actions", "workspace")}{link("Open Macro Studio", "macros")}{link("View plugin inventory", "plugins")}</div>
          </section>
        </>}
        {section === "data" && <>
          <BackupPanel onBackup={onBackup} onRestore={onRestore} />
          <section className={card}><h2 className="font-semibold text-[var(--text-primary)]">Save locally, then apply</h2>
            <p><strong className="text-[var(--text-primary)]">Save</strong> stores profile edits on this PC. <strong className="text-[var(--text-primary)]">Apply to Device</strong> saves and sends the selected page to the deck. Browsing or editing a different page does not change the applied bindings.</p>
            <p className="text-xs">Macros, auto-switch rules, and OBS settings save as you edit. Pending or failed saves appear above the workspace with a retry control.</p>
            {link("Open workspace", "workspace")}
          </section>
          <section className={card}><h2 className="font-semibold text-[var(--text-primary)]">Profile sharing</h2>
            <p>Use Import and Export in the profile sidebar to share a profile and its referenced macros.</p>
            <div className="rounded-lg border border-[var(--status-warning)]/30 bg-[var(--status-warning)]/5 p-3 text-xs text-[var(--status-warning)]">Profile export does not bundle staged images or animations. It is not a complete backup of your setup.</div>
          </section>
          <section className={card}><h2 className="font-semibold text-[var(--text-primary)]">Local storage and recovery</h2>
            <p className="text-xs break-all font-mono text-[var(--text-primary)]">%APPDATA%\DeckForge</p>
            <p className="text-xs leading-relaxed">The first explicit save of an older profile file preserves an original copy named profiles.pre-v1-N.json. This is a migration backup, not ongoing version history. Unsaved profile edits are copied to profile-draft.json after a short pause. After an unexpected exit, choose Restore draft or Discard. The newest edits may be lost if the process stops before the recovery write finishes; recovery errors appear in the editor.</p>
          </section>
        </>}
        {section === "about" && <>
          <section className={card}><div className="flex items-center gap-3"><div className="p-3 rounded-xl bg-[var(--accent-soft)] text-[var(--accent-hover)]"><Monitor size={24} /></div><div><h2 className="text-lg font-bold text-[var(--text-primary)]">DeckForge</h2><p className="text-xs">Version {packageInfo.version} · Windows desktop controller</p></div></div>
            <p>Build and manage layouts for your deck with local profiles, macros, and media.</p>
          </section>
          <UpdatesPanel card={card} />
          <section className={card}><h2 className="font-semibold text-[var(--text-primary)]">Support and troubleshooting</h2>
            <p className="text-xs leading-relaxed">Frame deduplication reduces repeated image writes. Input polling and heartbeats still run; this does not guarantee freeze-free operation. Physical device, tray, sign-in, and installer validation are still in progress.</p>
            <div className="flex flex-wrap gap-2">{link("Device diagnostics", "devices")}<button type="button" className={action} onClick={onOpenGuide}>Quick-start guide<BookOpen size={14} /></button></div>
          </section>
        </>}
      </div>
    </main>
  </div>;
};
