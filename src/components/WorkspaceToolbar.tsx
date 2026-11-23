import React from "react";
import { LayoutGrid, Play, Save } from "lucide-react";
import { DeckIcon } from "./DeckIcon";
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
    <div className="min-h-13 px-4 py-2.5 border-b border-[var(--border-subtle)] flex items-center justify-between flex-wrap gap-3 shrink-0 bg-[var(--bg-app)]">
      <div className="flex items-center gap-2.5 min-w-0">
        <span className="w-7 h-7 rounded-md bg-[var(--accent-soft)] border border-[var(--accent-primary)]/30 flex items-center justify-center shrink-0">
          <DeckIcon name={activeProfile.icon} size={14} color="var(--accent-text)" />
        </span>
        <h1 className="text-[15px] font-bold tracking-tight text-[var(--text-primary)] truncate">{activeProfile.name}</h1>
        <span className="text-[11px] tabular-nums px-2 py-0.5 rounded-full bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-muted)] shrink-0">
          {activeProfile.pages.length} {activeProfile.pages.length === 1 ? "page" : "pages"}
        </span>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        {/* Device Selector Pill */}
        {connectedDevices.length > 1 && <div className="flex items-center gap-2 bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] focus-within:border-[var(--accent-primary)] focus-within:ring-2 focus-within:ring-[var(--accent-primary)]/20 rounded-md px-2.5 h-8 transition-all">
          <LayoutGrid size={14} className="text-[var(--text-secondary)] shrink-0" />
          <span className="text-xs text-[var(--text-secondary)]">Device:</span>
          <select
            aria-label="Active device"
            value={activeDevice.id}
            onChange={(e) => {
              setSelectedDeviceId(e.target.value);
              setSelectedKeyIdx(0);
              setSurface("key");
              setSurfaceIndex(0);
            }}
            className="bg-transparent text-xs font-semibold text-[var(--text-primary)] outline-none focus-ring-none cursor-pointer"
          >
            {connectedDevices.map((d) => (
              <option key={d.id} value={d.id} className="bg-[var(--bg-panel)] text-[var(--text-primary)]">
                {d.modelName} ({d.keys.count} keys)
              </option>
            ))}
          </select>
        </div>}

        {/* Zoom / Fit Controls */}
        <div role="group" aria-label="Canvas zoom" className="flex items-center bg-[var(--bg-input)] border border-[var(--border-subtle)] rounded-md p-0.5 h-8 gap-0.5">
          {[90, 100, 110].map((z) => (
            <button
              key={z}
              type="button"
              aria-pressed={canvasZoom === z}
              onClick={() => setCanvasZoom(z)}
              className={`px-2.5 h-[26px] rounded-[4px] text-[11px] tabular-nums font-medium cursor-pointer transition-all duration-150 border ${
                canvasZoom === z
                  ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-[var(--text-primary)] font-semibold shadow-xs"
                  : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
              }`}
            >
              {z}%
            </button>
          ))}
        </div>

        <span className="hidden sm:block w-px h-5 bg-[var(--border-subtle)] mx-0.5" aria-hidden />

        {/* Save Locally */}
        <button
          type="button"
          onClick={handleSaveLocally}
          title="Save profiles locally (Ctrl+S)"
          className="h-8 shrink-0 whitespace-nowrap px-3 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] active:scale-[0.98] text-xs font-medium text-[var(--text-primary)] flex items-center gap-1.5 cursor-pointer transition-all duration-150"
        >
          <Save size={13} className="text-[var(--text-secondary)]" />
          <span>Save</span>
        </button>

        {/* Primary Violet CTA: Apply / Deploy to Device */}
        <button
          type="button"
          onClick={handleApplyToDevice}
          title="Send this page to the deck (Ctrl+Enter)"
          className="h-8 shrink-0 whitespace-nowrap px-3.5 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white flex items-center gap-1.5 cursor-pointer shadow-[0_2px_10px_rgba(124,131,247,0.28)] hover:shadow-[0_3px_14px_rgba(124,131,247,0.42)] active:scale-[0.98] transition-all duration-150"
        >
          <Play size={12} fill="currentColor" />
          <span>Apply to Device</span>
        </button>
      </div>
    </div>
  );
};
