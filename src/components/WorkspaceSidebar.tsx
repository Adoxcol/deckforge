import React from "react";
import { deckforgeApi } from "../bridge/tauriBridge";
import { ChevronRight, Download, Plus, RefreshCw, Search, Upload, X } from "lucide-react";
import { DeckIcon } from "./DeckIcon";
import { AppRuleList } from "./AppRuleList";
import { STARTER_PROFILES, type StarterProfile } from "../data/starterProfiles";
import type { AutoSwitchRule, DeviceDescriptor, MacroSequence, Profile } from "../types/deckforge";
import { searchBindings } from "../lib/bindingSearch";

interface WorkspaceSidebarProps {
  activeProfile: Profile;
  autoSwitchRules: AutoSwitchRule[];
  connectedDevices: DeviceDescriptor[];
  handleAddProfile: (starter: StarterProfile) => void;
  onExportProfile: () => void;
  onImportProfile: () => void;
  profiles: Profile[];
  macros: MacroSequence[];
  selectedPageIdx: number;
  setSurface: React.Dispatch<React.SetStateAction<"key" | "zone" | "strip" | "encoder">>;
  setSurfaceIndex: React.Dispatch<React.SetStateAction<number>>;
  saveRules: (rules: AutoSwitchRule[]) => void;
  setDevices: React.Dispatch<React.SetStateAction<DeviceDescriptor[]>>;
  setSelectedKeyIdx: React.Dispatch<React.SetStateAction<number>>;
  setSelectedPageIdx: React.Dispatch<React.SetStateAction<number>>;
  setSelectedProfileId: React.Dispatch<React.SetStateAction<string>>;
  triggerToast: (text: string, kind?: "success" | "info" | "warning") => void;
}

export const WorkspaceSidebar = React.memo(function WorkspaceSidebar({
  activeProfile,
  autoSwitchRules,
  connectedDevices,
  handleAddProfile,
  onExportProfile,
  onImportProfile,
  profiles,
  macros,
  selectedPageIdx,
  setSurface,
  setSurfaceIndex,
  saveRules,
  setDevices,
  setSelectedKeyIdx,
  setSelectedPageIdx,
  setSelectedProfileId,
  triggerToast,
}: WorkspaceSidebarProps) {
  const [pickerOpen, setPickerOpen] = React.useState(false);
  const [rescanning, setRescanning] = React.useState(false);
  const [bindingQuery, setBindingQuery] = React.useState("");
  const [appsOpen, setAppsOpen] = React.useState(false);
  const bindingHits = React.useMemo(
    () => searchBindings(profiles, bindingQuery, macros.map((macro) => ({ id: macro.id, name: macro.name }))),
    [profiles, bindingQuery, macros],
  );

  React.useEffect(() => {
    if (!pickerOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setPickerOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [pickerOpen]);

  const iconButtonClass =
    "w-6 h-6 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] transition-all duration-150 flex items-center justify-center cursor-pointer";
  return (
    <aside className="min-h-0 bg-[var(--bg-sidebar)] border-r border-[var(--border-subtle)] p-3.5 flex flex-col gap-4 overflow-y-auto">
      {/* Devices Section */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <span className="text-[11.5px] font-semibold tracking-wide text-[var(--text-secondary)]">Devices</span>
          <button
            type="button"
            onClick={() => {
              setRescanning(true);
              deckforgeApi.refreshUsbDevices().then(setDevices).finally(() => setRescanning(false));
              triggerToast("Rescanned USB HID bus", "info");
            }}
            title="Rescan USB Devices"
            aria-label="Rescan USB devices"
            className={iconButtonClass}
          >
            <RefreshCw size={12} className={rescanning ? "animate-spin" : ""} />
          </button>
        </div>

        <div className="flex flex-col gap-1.5">
          {connectedDevices.length === 0 ? (
            <div className="px-2.5 py-3 rounded-lg border border-dashed border-[var(--border-subtle)] bg-[var(--bg-panel)]/40 text-[11px] leading-relaxed text-[var(--text-muted)]">
              No device connected. Plug in a deck, then rescan.
            </div>
          ) : (
            connectedDevices.map((dev) => (
              <div
                key={dev.id}
                className="p-2.5 rounded-lg border flex items-center gap-2.5 bg-[var(--accent-soft)]/80 border-[var(--accent-primary)]/45 shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]"
              >
                <div
                  className="w-8 h-6 rounded bg-[var(--bg-input)] border border-[var(--accent-primary)]/60 p-1 grid gap-0.5 shrink-0"
                  style={{ gridTemplateColumns: `repeat(${Math.max(1, Math.min(dev.keys.cols, 6))}, minmax(0, 1fr))` }}
                >
                  {Array.from({ length: Math.min(dev.keys.count, 18) }).map((_, idx) => (
                    <span key={idx} className="rounded-[1px] bg-[var(--accent-primary)]/90" />
                  ))}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="text-[12px] font-semibold text-[var(--text-primary)] truncate">
                    {dev.modelName}
                  </div>
                  <div className="flex items-center gap-1.5 text-[10.5px] mt-0.5">
                    <span className="w-1.5 h-1.5 rounded-full bg-[var(--status-success)] shadow-[0_0_6px_var(--status-success)]" />
                    <span className="text-[var(--status-success)] font-medium">Connected</span>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      {/* Profiles Section */}
      <div className="relative flex-1 flex flex-col pt-3.5 border-t border-[var(--border-subtle)]">
        <div className="relative flex items-center justify-between mb-2">
          <span className="text-[11.5px] font-semibold tracking-wide text-[var(--text-secondary)]">Profiles</span>
          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={onImportProfile}
              title="Import a profile from a file"
              aria-label="Import profile"
              className={iconButtonClass}
            >
              <Download size={13} />
            </button>
            <button
              type="button"
              onClick={onExportProfile}
              title="Export this profile to a file"
              aria-label="Export profile"
              className={iconButtonClass}
            >
              <Upload size={13} />
            </button>
            <div>
              <button
                type="button"
                onClick={() => setPickerOpen((open) => !open)}
                title="New profile"
                aria-label="Create profile"
                aria-expanded={pickerOpen}
                className={iconButtonClass}
              >
                <Plus size={13} />
              </button>
              {pickerOpen && (
                <>
                  <div className="fixed inset-0 z-40" onClick={() => setPickerOpen(false)} />
                  <div
                    role="menu"
                    aria-label="Create profile from template"
                    className="absolute left-0 right-0 top-full mt-1.5 z-50 max-h-[70vh] overflow-y-auto p-1.5 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-strong)] shadow-2xl flex flex-col gap-0.5 animate-pop-in"
                  >
                    <div className="px-2 pt-1 pb-1.5 text-[11px] font-medium text-[var(--text-muted)]">Start a new profile from</div>
                    {STARTER_PROFILES.map((starter) => (
                      <button
                        key={starter.id}
                        type="button"
                        role="menuitem"
                        onClick={() => {
                          setPickerOpen(false);
                          handleAddProfile(starter);
                        }}
                        className="flex items-start gap-2 px-2 py-1.5 rounded-[var(--radius-sm)] text-left hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"
                      >
                        <span
                          className="mt-0.5 w-6 h-6 shrink-0 rounded-[var(--radius-sm)] flex items-center justify-center"
                          style={{ color: starter.color, background: `${starter.color}22` }}
                        >
                          <DeckIcon name={starter.icon} size={13} color={starter.color} />
                        </span>
                        <span className="flex flex-col min-w-0">
                          <span className="text-[12.5px] font-medium text-[var(--text-primary)]">{starter.name}</span>
                          <span className="text-[11px] text-[var(--text-muted)] leading-snug">{starter.tagline}</span>
                        </span>
                      </button>
                    ))}
                  </div>
                </>
              )}
            </div>
          </div>
        </div>

        <div className="relative mb-2">
          <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] pointer-events-none" />
          <input
            value={bindingQuery}
            onChange={(event) => setBindingQuery(event.target.value)}
            placeholder="Find a key, app, or macro"
            aria-label="Find a binding"
            className="w-full h-8 pl-8 pr-7 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)] transition-colors"
          />
          {bindingQuery && (
            <button
              type="button"
              aria-label="Clear binding search"
              onClick={() => setBindingQuery("")}
              className="absolute right-1.5 top-1/2 -translate-y-1/2 p-0.5 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"
            >
              <X size={12} />
            </button>
          )}
        </div>
        {bindingQuery.trim() && (
          <div role="listbox" aria-label="Binding results" className="mb-2.5 p-1 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-panel)] shadow-lg flex max-h-48 flex-col gap-0.5 overflow-y-auto animate-pop-in">
            {bindingHits.length === 0 ? (
              <p className="px-2.5 py-2 text-[11px] text-[var(--text-muted)]">Nothing matches.</p>
            ) : (
              <>
                <div className="px-2 py-1 text-[10px] font-medium text-[var(--text-muted)] tabular-nums border-b border-[var(--border-subtle)]/60 mb-0.5">
                  {bindingHits.length} {bindingHits.length === 1 ? "match" : "matches"}
                </div>
                {bindingHits.map((hit) => (
                <button
                  key={`${hit.profileId}:${hit.pageIndex}:${hit.surface}:${hit.index}`}
                  type="button"
                  role="option"
                  onClick={() => {
                    deckforgeApi.holdAutoSwitch().catch(() => {});
                    setSelectedProfileId(hit.profileId);
                    setSelectedPageIdx(hit.pageIndex);
                    setSurface(hit.surface);
                    setSurfaceIndex(hit.index);
                    if (hit.surface === "key") setSelectedKeyIdx(hit.index);
                    setBindingQuery("");
                  }}
                  className="px-2 py-1.5 rounded-md text-left hover:bg-[var(--bg-elevated)] focus-visible:bg-[var(--bg-elevated)] cursor-pointer transition-colors"
                >
                  <span className="flex items-center justify-between gap-2">
                    <span className="truncate text-[12px] font-medium text-[var(--text-primary)]">{hit.title}</span>
                    <span className="shrink-0 text-[9.5px] font-mono px-1.5 py-0.5 rounded bg-[var(--bg-input)] border border-[var(--border-subtle)] text-[var(--text-secondary)] capitalize">{hit.surface}</span>
                  </span>
                  <span className="block truncate text-[10.5px] text-[var(--text-muted)] mt-0.5">
                    {hit.profileName} · {hit.pageName} · {hit.where}
                  </span>
                </button>
              ))}
              </>
            )}
          </div>
        )}
        <div className="flex flex-col gap-1">
          {profiles.map((prof) => {
            const isSel = prof.id === activeProfile.id;
            const selectProfile = () => {
              deckforgeApi.holdAutoSwitch().catch(() => {});
              setSelectedProfileId(prof.id);
              setSelectedPageIdx(0);
              setSelectedKeyIdx(0);
            };
            return (
              <button
                key={prof.id}
                type="button"
                aria-current={isSel ? "true" : undefined}
                onClick={selectProfile}
                className={`w-full text-left px-2.5 py-2 rounded-lg border flex items-center justify-between gap-2 cursor-pointer transition-all duration-150 active:scale-[0.99] ${
                  isSel
                    ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/50 text-[var(--text-primary)] font-semibold shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]"
                    : "bg-transparent border-transparent text-[var(--text-secondary)] hover:bg-[var(--bg-panel)] hover:border-[var(--border-subtle)] hover:text-[var(--text-primary)]"
                }`}
              >
                <div className="flex items-center gap-2.5 truncate min-w-0">
                  <DeckIcon
                    name={prof.icon}
                    size={15}
                    color={isSel ? "var(--accent-text)" : "var(--text-secondary)"}
                  />
                  <span className="text-[12.5px] truncate">{prof.name}</span>
                </div>
                <span
                  className={`text-[10px] font-mono tabular-nums px-1.5 py-0.5 rounded shrink-0 ${
                    isSel ? "bg-[var(--accent-primary)]/20 text-[var(--accent-text)]" : "text-[var(--text-muted)]"
                  }`}
                  title={`${prof.pages.length} ${prof.pages.length === 1 ? "page" : "pages"}`}
                >
                  {prof.pages.length}p
                </span>
              </button>
            );
          })}
        </div>
      </div>

      <div className="pt-3.5 border-t border-[var(--border-subtle)]">
        <button
          type="button"
          aria-expanded={appsOpen}
          onClick={() => setAppsOpen((open) => !open)}
          className="w-full flex items-center justify-between gap-2 px-1.5 py-1 rounded-md text-left cursor-pointer text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]/60 transition-colors"
        >
          <span className="text-[11.5px] font-semibold tracking-wide">When an app opens</span>
          <ChevronRight size={14} className={`shrink-0 transition-transform ${appsOpen ? "rotate-90" : ""}`} />
        </button>
        {appsOpen && (
          <div className="mt-2">
            <AppRuleList embedded rules={autoSwitchRules} profiles={profiles} showProfileId={activeProfile?.id} currentPageIndex={selectedPageIdx} saveRules={saveRules} />
          </div>
        )}
      </div>
    </aside>
  );
});
