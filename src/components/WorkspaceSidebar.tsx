import React from "react";
import { deckforgeApi } from "../bridge/tauriBridge";
import { Download, Plus, Upload } from "lucide-react";
import { DeckIcon } from "./DeckIcon";
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
    <aside className="bg-[#10131D] border-r border-[#272D3D] p-3.5 flex flex-col gap-5 overflow-y-auto">
      {/* Devices Section */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <span className="text-[12px] font-semibold text-[#F4F5FA]">Devices</span>
          <button
            onClick={() => {
              deckforgeApi.refreshUsbDevices().then(setDevices);
              triggerToast("Rescanned USB HID bus", "info");
            }}
            title="Rescan USB Devices"
            className="w-6 h-6 rounded-md bg-[#151925] border border-[#272D3D] text-[#A6AFC4] hover:text-white flex items-center justify-center cursor-pointer"
          >
            <Plus size={13} />
          </button>
        </div>

        <div className="flex flex-col gap-1.5">
          {connectedDevices.length === 0 ? (
            <div className="px-2.5 py-3 rounded-lg border border-dashed border-[#272D3D] text-[11px] leading-relaxed text-[#707B94]">
              No device connected. Plug in a deck, then rescan.
            </div>
          ) : (
            connectedDevices.map((dev) => (
              <div
                key={dev.id}
                className="p-2 rounded-lg border flex items-center gap-2.5 bg-[#30244F] border-[#8B5CF6]/60 shadow-[0_0_16px_rgba(139,92,246,0.2)]"
              >
                <div className="w-8 h-6 rounded bg-[#111522] border border-[#8B5CF6] p-1 grid grid-cols-5 gap-0.5 shrink-0">
                  {Array.from({ length: 10 }).map((_, idx) => (
                    <span key={idx} className="rounded-[1px] bg-[#8B5CF6]" />
                  ))}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="text-[12px] font-semibold text-[#F4F5FA] truncate">
                    {dev.modelName}
                  </div>
                  <div className="flex items-center gap-1.5 text-[10.5px]">
                    <span className="w-1.5 h-1.5 rounded-full bg-[#35C98B]" />
                    <span className="text-[#35C98B]">Connected</span>
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
          <span className="text-[12px] font-semibold text-[#F4F5FA]">Profiles</span>
          <div className="flex items-center gap-1">
            <button
              onClick={onImportProfile}
              title="Import a profile from a file"
              aria-label="Import profile"
              className="w-6 h-6 rounded-md bg-[#151925] border border-[#272D3D] text-[#A6AFC4] hover:text-white flex items-center justify-center cursor-pointer"
            >
              <Download size={13} />
            </button>
            <button
              onClick={onExportProfile}
              title="Export this profile to a file"
              aria-label="Export profile"
              className="w-6 h-6 rounded-md bg-[#151925] border border-[#272D3D] text-[#A6AFC4] hover:text-white flex items-center justify-center cursor-pointer"
            >
              <Upload size={13} />
            </button>
            <button
              onClick={handleAddProfile}
              title="Create New Profile"
              aria-label="Create profile"
              className="w-6 h-6 rounded-md bg-[#151925] border border-[#272D3D] text-[#A6AFC4] hover:text-white flex items-center justify-center cursor-pointer"
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
                    ? "bg-[#30244F] border-[#8B5CF6]/55 text-[#F4F5FA] font-semibold"
                    : "bg-transparent border-transparent text-[#A6AFC4] hover:bg-[#151925] hover:text-[#F4F5FA]"
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
                <span className="text-[#707B94] text-xs">•••</span>
              </div>
            );
          })}
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <div className="text-[12px] font-semibold text-[#F4F5FA]">When an app is in front</div>
        <p className="text-[10.5px] leading-relaxed text-[#707B94]">
          The saved profile for that app is sent to the deck. Save after you edit a page.
        </p>
        {autoSwitchRules.map((rule) => (
          <div key={rule.id} className="flex flex-col gap-1 rounded-lg border border-[#272D3D] p-2">
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={rule.enabled}
                onChange={(event) =>
                  saveRules(autoSwitchRules.map((item) => (item.id === rule.id ? { ...item, enabled: event.target.checked } : item)))
                }
              />
              <input
                value={rule.processName}
                onChange={(event) =>
                  saveRules(autoSwitchRules.map((item) => (item.id === rule.id ? { ...item, processName: event.target.value } : item)))
                }
                className="min-w-0 flex-1 h-7 px-2 rounded-md bg-[#111522] border border-[#272D3D] text-[11px] font-mono text-[#F4F5FA] outline-none"
              />
              <button
                type="button"
                onClick={() => saveRules(autoSwitchRules.filter((item) => item.id !== rule.id))}
                className="text-[11px] text-[#A6AFC4] cursor-pointer"
              >
                Remove
              </button>
            </div>
            <input
              value={rule.windowTitleContains}
              placeholder="Window title contains (optional)"
              onChange={(event) =>
                saveRules(autoSwitchRules.map((item) => (item.id === rule.id ? { ...item, windowTitleContains: event.target.value } : item)))
              }
              className="h-7 px-2 rounded-md bg-[#111522] border border-[#272D3D] text-[11px] text-[#F4F5FA] outline-none"
            />
            <select
              value={rule.targetProfileId}
              onChange={(event) =>
                saveRules(autoSwitchRules.map((item) => (item.id === rule.id ? { ...item, targetProfileId: event.target.value } : item)))
              }
              className="h-7 px-2 rounded-md bg-[#111522] border border-[#272D3D] text-[11px] text-[#F4F5FA] outline-none"
            >
              {profiles.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.name}
                </option>
              ))}
            </select>
          </div>
        ))}
        <button
          type="button"
          onClick={() =>
            saveRules([
              ...autoSwitchRules,
              {
                id: `rule-${Date.now()}`,
                processName: "Spotify.exe",
                windowTitleContains: "",
                targetProfileId: profiles[0]?.id ?? "profile-streaming",
                priority: autoSwitchRules.length + 1,
                enabled: true,
              },
            ])
          }
          className="h-7 rounded-md border border-[#272D3D] text-[11px] text-[#A6AFC4] cursor-pointer"
        >
          Add app rule
        </button>
      </div>
    </aside>
  );
};
