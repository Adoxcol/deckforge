import React from "react";
import { deckforgeApi } from "../bridge/tauriBridge";
import { Download, Plus, Upload } from "lucide-react";
import { DeckIcon } from "./DeckIcon";
import { AppRuleList } from "./AppRuleList";
import type { AutoSwitchRule, DeviceDescriptor, Profile } from "../types/deckforge";

interface WorkspaceSidebarProps {
  activeProfile: Profile;
  autoSwitchRules: AutoSwitchRule[];
  connectedDevices: DeviceDescriptor[];
  handleAddProfile: () => void;
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
            className="w-6 h-6 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-white flex items-center justify-center cursor-pointer"
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
                className="p-2 rounded-lg border flex items-center gap-2.5 bg-[var(--accent-soft)] border-[var(--accent-primary)]/60 shadow-[0_0_16px_rgba(139,92,246,0.2)]"
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
      <div className="flex-1 flex flex-col">
        <div className="flex items-center justify-between mb-2">
          <span className="text-[12px] font-semibold text-[var(--text-primary)]">Profiles</span>
          <div className="flex items-center gap-1">
            <button
              onClick={onImportProfile}
              title="Import a profile from a file"
              aria-label="Import profile"
              className="w-6 h-6 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-white flex items-center justify-center cursor-pointer"
            >
              <Download size={13} />
            </button>
            <button
              onClick={onExportProfile}
              title="Export this profile to a file"
              aria-label="Export profile"
              className="w-6 h-6 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-white flex items-center justify-center cursor-pointer"
            >
              <Upload size={13} />
            </button>
            <button
              onClick={handleAddProfile}
              title="Create New Profile"
              aria-label="Create profile"
              className="w-6 h-6 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-white flex items-center justify-center cursor-pointer"
            >
              <Plus size={13} />
            </button>
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
                    color={isSel ? "#8B5CF6" : "#A6AFC4"}
                  />
                  <span className="text-[12.5px] truncate">{prof.name}</span>
                </div>
                <span className="text-[var(--text-muted)] text-xs">•••</span>
              </div>
            );
          })}
        </div>
      </div>

      <AppRuleList rules={autoSwitchRules} profiles={activeProfile ? [activeProfile] : []} saveRules={saveRules} />
    </aside>
  );
};
