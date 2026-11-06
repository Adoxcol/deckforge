import React from "react";
import { Edit3, LayoutGrid, Play, Save } from "lucide-react";
import type { DeviceDescriptor, Profile } from "../types/deckforge";

interface WorkspaceToolbarProps {
  activeDevice: DeviceDescriptor;
  activeProfile: Profile;
  canvasZoom: number;
  connectedDevices: DeviceDescriptor[];
  handleApplyToDevice: () => Promise<void>;
  handleSaveLocally: () => Promise<void>;
  setCanvasZoom: React.Dispatch<React.SetStateAction<number>>;
  setSelectedDeviceId: React.Dispatch<React.SetStateAction<string>>;
  setSelectedKeyIdx: React.Dispatch<React.SetStateAction<number>>;
  setSurface: React.Dispatch<React.SetStateAction<"key" | "zone" | "strip" | "encoder">>;
  setSurfaceIndex: React.Dispatch<React.SetStateAction<number>>;
}

export const WorkspaceToolbar: React.FC<WorkspaceToolbarProps> = ({
  activeDevice,
  activeProfile,
  canvasZoom,
  connectedDevices,
  handleApplyToDevice,
  handleSaveLocally,
  setCanvasZoom,
  setSelectedDeviceId,
  setSelectedKeyIdx,
  setSurface,
  setSurfaceIndex,
}) => {
  return (
    <div className="h-13 px-5 border-b border-[#272D3D] flex items-center justify-between gap-3 shrink-0">
      <div className="flex items-center gap-2">
        <h1 className="text-[16px] font-bold text-[#F4F5FA]">{activeProfile.name}</h1>
        <Edit3 size={14} className="text-[#707B94]" />
      </div>

      <div className="flex items-center gap-2.5">
        {/* Device Selector Pill */}
        <div className="flex items-center gap-2 bg-[#111522] border border-[#272D3D] rounded-md px-2.5 h-9">
          <LayoutGrid size={14} className="text-[#A6AFC4]" />
          <span className="text-xs text-[#A6AFC4]">Device:</span>
          {connectedDevices.length > 1 ? (
            <select
              value={activeDevice.id}
              onChange={(e) => {
                setSelectedDeviceId(e.target.value);
                setSelectedKeyIdx(0);
                setSurface("key");
                setSurfaceIndex(0);
              }}
              className="bg-transparent text-xs font-semibold text-[#F4F5FA] outline-none cursor-pointer"
            >
              {connectedDevices.map((d) => (
                <option key={d.id} value={d.id} className="bg-[#151925] text-[#F4F5FA]">
                  {d.modelName} ({d.keys.count} keys)
                </option>
              ))}
            </select>
          ) : (
            <span className="text-xs font-semibold text-[#F4F5FA]">
              {connectedDevices[0]?.modelName ?? "None connected"}
            </span>
          )}
        </div>

        {/* Zoom / Fit Controls */}
        <div className="flex items-center bg-[#111522] border border-[#272D3D] rounded-md p-0.5 h-9">
          {[90, 100, 110].map((z) => (
            <button
              key={z}
              onClick={() => setCanvasZoom(z)}
              className={`px-2 h-7 rounded text-[11px] font-medium cursor-pointer ${
                canvasZoom === z
                  ? "bg-[#30244F] text-[#F4F5FA]"
                  : "text-[#A6AFC4] hover:text-white"
              }`}
            >
              {z}%
            </button>
          ))}
        </div>

        {/* Save Locally */}
        <button
          onClick={handleSaveLocally}
          className="h-9 px-3 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#394158] text-xs font-medium text-[#F4F5FA] flex items-center gap-1.5 cursor-pointer"
        >
          <Save size={14} />
          <span>Save</span>
        </button>

        {/* Primary Violet CTA: Apply / Deploy to Device */}
        <button
          onClick={handleApplyToDevice}
          className="h-9 px-4 rounded-md bg-[#8B5CF6] hover:bg-[#9D75F8] text-xs font-semibold text-white flex items-center gap-2 shadow-[0_0_20px_rgba(139,92,246,0.4)] cursor-pointer transition-all"
        >
          <Play size={13} fill="currentColor" />
          <span>Apply to Device</span>
        </button>
      </div>
    </div>
  );
};
