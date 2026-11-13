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
    <div className="min-h-13 px-4 py-3 border-b border-[var(--border-subtle)] flex items-center justify-between flex-wrap gap-3 shrink-0">
      <div className="flex items-center gap-2">
        <h1 className="text-[16px] font-bold text-[var(--text-primary)]">{activeProfile.name}</h1>
        <Edit3 size={14} className="text-[var(--text-muted)]" />
      </div>

      <div className="flex flex-wrap items-center gap-2">
        {/* Device Selector Pill */}
        {connectedDevices.length > 1 && <div className="flex items-center gap-2 bg-[var(--bg-input)] border border-[var(--border-subtle)] rounded-md px-2.5 h-9">
          <LayoutGrid size={14} className="text-[var(--text-secondary)]" />
          <span className="text-xs text-[var(--text-secondary)]">Device:</span>
          {connectedDevices.length > 1 ? (
            <select
              value={activeDevice.id}
              onChange={(e) => {
                setSelectedDeviceId(e.target.value);
                setSelectedKeyIdx(0);
                setSurface("key");
                setSurfaceIndex(0);
              }}
              className="bg-transparent text-xs font-semibold text-[var(--text-primary)] outline-none cursor-pointer"
            >
              {connectedDevices.map((d) => (
                <option key={d.id} value={d.id} className="bg-[var(--bg-panel)] text-[var(--text-primary)]">
                  {d.modelName} ({d.keys.count} keys)
                </option>
              ))}
            </select>
          ) : (
            <span className="text-xs font-semibold text-[var(--text-primary)]">
              {connectedDevices[0]?.modelName ?? "None connected"}
            </span>
          )}
        </div>}

        {/* Zoom / Fit Controls */}
        <div className="flex items-center bg-[var(--bg-input)] border border-[var(--border-subtle)] rounded-md p-0.5 h-9">
          {[90, 100, 110].map((z) => (
            <button
              key={z}
              onClick={() => setCanvasZoom(z)}
              className={`px-2 h-7 rounded text-[11px] font-medium cursor-pointer ${
                canvasZoom === z
                  ? "bg-[var(--accent-soft)] text-[var(--text-primary)]"
                  : "text-[var(--text-secondary)] hover:text-white"
              }`}
            >
              {z}%
            </button>
          ))}
        </div>

        {/* Save Locally */}
        <button
          onClick={handleSaveLocally}
          className="h-9 shrink-0 whitespace-nowrap px-3 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] text-xs font-medium text-[var(--text-primary)] flex items-center gap-1.5 cursor-pointer"
        >
          <Save size={14} />
          <span>Save</span>
        </button>

        {/* Primary Violet CTA: Apply / Deploy to Device */}
        <button
          onClick={handleApplyToDevice}
          className="h-9 shrink-0 whitespace-nowrap px-4 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white flex items-center gap-2 shadow-[0_0_20px_rgba(139,92,246,0.4)] cursor-pointer transition-all"
        >
          <Play size={13} fill="currentColor" />
          <span>Apply to Device</span>
        </button>
      </div>
    </div>
  );
};
