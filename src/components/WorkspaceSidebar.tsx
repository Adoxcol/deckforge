import React from "react";
import { deckforgeApi } from "../bridge/tauriBridge";
import { Download, Plus, Upload } from "lucide-react";
import { DeckIcon } from "./DeckIcon";
import { AppRuleList } from "./AppRuleList";
import { STARTER_PROFILES, type StarterProfile } from "../data/starterProfiles";
import type { AutoSwitchRule, DeviceDescriptor, Profile } from "../types/deckforge";

interface WorkspaceSidebarProps {
  activeProfile: Profile;
  autoSwitchRules: AutoSwitchRule[];
  connectedDevices: DeviceDescriptor[];
  handleAddProfile: (starter: StarterProfile) => void;
  onExportProfile: () => void;
  onImportProfile: () => void;
  profiles: Profile[];
  saveRules: (rules: AutoSwitchRule[]) => void;
  setDevices: React.Dispatch<React.SetStateAction<DeviceDescriptor[]>>;
  setSelectedKeyIdx: React.Dispatch<React.SetStateAction<number>>;
  setSelectedPageIdx: React.Dispatch<React.SetStateAction<number>>;
  setSelectedProfileId: React.Dispatch<React.SetStateAction<string>>;
  triggerToast: (text: string, kind?: "success" | "info" | "warning") => void;
}

export const WorkspaceSidebar: React.FC<WorkspaceSidebarProps> = ({
  activeProfile,
  autoSwitchRules,
  connectedDevices,
  handleAddProfile,
  onExportProfile,
  onImportProfile,
  profiles,
  saveRules,
  setDevices,
  setSelectedKeyIdx,
  setSelectedPageIdx,
  setSelectedProfileId,
  triggerToast,
}) => {
  const [pickerOpen, setPickerOpen] = React.useState(false);
  return (
    <aside className="bg-[var(--bg-sidebar)] border-r border-[var(--border-subtle)] p-3.5 flex flex-col gap-5 overflow-y-auto">
      {/* Devices Section */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <span className="text-[12px] font-semibold text-[var(--text-primary)]">Devices</span>
          <button
            onClick={() => {
              deckforgeApi.refreshUsbDevices().then(setDevices);
              triggerToast("Rescanned USB HID bus", "info");
            }}
            title="Rescan USB Devices"
            className="w-6 h-6 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center justify-center cursor-pointer"
          >
            <Plus size={13} />
          </button>
        </div>

        <div className="flex flex-col gap-1.5">
          {connectedDevices.length === 0 ? (
            <div className="px-2.5 py-3 rounded-lg border border-dashed border-[var(--border-subtle)] text-[11px] leading-relaxed text-[var(--text-muted)]">
              No device connected. Plug in a deck, then rescan.
            </div>
          ) : (
            connectedDevices.map((dev) => (
              <div
                key={dev.id}
                className="p-2 rounded-lg border flex items-center gap-2.5 bg-[var(--accent-soft)] border-[var(--accent-primary)]/60"
              >
                <div className="w-8 h-6 rounded bg-[var(--bg-input)] border border-[var(--accent-primary)] p-1 grid grid-cols-5 gap-0.5 shrink-0">
                  {Array.from({ length: 10 }).map((_, idx) => (
                    <span key={idx} className="rounded-[1px] bg-[var(--accent-primary)]" />
                  ))}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="text-[12px] font-semibold text-[var(--text-primary)] truncate">
                    {dev.modelName}
                  </div>
                  <div className="flex items-center gap-1.5 text-[10.5px]">
                    <span className="w-1.5 h-1.5 rounded-full bg-[var(--status-success)]" />
                    <span className="text-[var(--status-success)]">Connected</span>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      {/* Profiles Section */}
      <div className="relative flex-1 flex flex-col">
        <div className="flex items-center justify-between mb-2">
          <span className="text-[12px] font-semibold text-[var(--text-primary)]">Profiles</span>
          <div className="flex items-center gap-1">
            <button
              onClick={onImportProfile}
              title="Import a profile from a file"
              aria-label="Import profile"
              className="w-6 h-6 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center justify-center cursor-pointer"
            >
              <Download size={13} />
            </button>
            <button
              onClick={onExportProfile}
              title="Export this profile to a file"
              aria-label="Export profile"
              className="w-6 h-6 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center justify-center cursor-pointer"
            >
              <Upload size={13} />
            </button>
            <div>
              <button
                onClick={() => setPickerOpen((open) => !open)}
                title="New profile"
                aria-label="Create profile"
                aria-expanded={pickerOpen}
                className="w-6 h-6 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center justify-center cursor-pointer"
              >
                <Plus size={13} />
              </button>
              {pickerOpen && (
                <>
                  <div className="fixed inset-0 z-40" onClick={() => setPickerOpen(false)} />
                  <div className="absolute left-0 right-0 top-8 z-50 p-1 rounded-[var(--radius-md)] bg-[var(--bg-panel)] border border-[var(--border-strong)] shadow-lg flex flex-col">
                    <div className="px-2 pt-1 pb-1.5 text-[11.5px] text-[var(--text-muted)]">Start a new profile from</div>
                    {STARTER_PROFILES.map((starter) => (
                      <button
                        key={starter.id}
                        type="button"
                        onClick={() => {
                          setPickerOpen(false);
                          handleAddProfile(starter);
                        }}
                        className="flex items-start gap-2 px-2 py-1.5 rounded-[var(--radius-sm)] text-left hover:bg-[var(--bg-elevated)] cursor-pointer"
                      >
                        <span
                          className="mt-0.5 w-6 h-6 shrink-0 rounded-[var(--radius-sm)] flex items-center justify-center"
                          style={{ color: starter.color, background: `${starter.color}22` }}
                        >
                          <DeckIcon name={starter.icon} size={13} color={starter.color} />
                        </span>
                        <span className="flex flex-col min-w-0">
                          <span className="text-[12.5px] text-[var(--text-primary)]">{starter.name}</span>
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

        <div className="flex flex-col gap-1">
          {profiles.map((prof) => {
            const isSel = prof.id === activeProfile.id;
            return (
              <div
                key={prof.id}
                onClick={() => {
                  deckforgeApi.holdAutoSwitch().catch(() => {});
                  setSelectedProfileId(prof.id);
                  setSelectedPageIdx(0);
                  setSelectedKeyIdx(0);
                }}
                className={`px-2.5 py-2 rounded-lg border flex items-center justify-between cursor-pointer transition-all ${
                  isSel
                    ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/55 text-[var(--text-primary)] font-semibold"
                    : "bg-transparent border-transparent text-[var(--text-secondary)] hover:bg-[var(--bg-panel)] hover:text-[var(--text-primary)]"
                }`}
              >
                <div className="flex items-center gap-2.5 truncate">
                  <DeckIcon
                    name={prof.icon}
                    size={15}
                    color={isSel ? "var(--accent-text)" : "var(--text-secondary)"}
                  />
                  <span className="text-[12.5px] truncate">{prof.name}</span>
                </div>
                <span className="text-[var(--text-muted)] text-xs">•••</span>
              </div>
            );
          })}
        </div>
      </div>

      <AppRuleList rules={autoSwitchRules} profiles={profiles} showProfileId={activeProfile?.id} saveRules={saveRules} />
    </aside>
  );
};
