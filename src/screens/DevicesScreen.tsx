import React, { useState } from "react";
import { RefreshCw } from "lucide-react";
import type { DeviceDescriptor, DiagnosticEntry, FrameCacheStats } from "../types/deckforge";

interface DevicesScreenProps {
  connectedDevices: DeviceDescriptor[];
  activeDevice: DeviceDescriptor;
  frameStats: FrameCacheStats;
  diagnostics: DiagnosticEntry[];
  onRescan: () => void;
  onSelectDevice: (deviceId: string) => void;
  onBrightness: (percent: number) => void;
  onOpenWorkspace: () => void;
}

type DetailTab = "Overview" | "Controls" | "Display" | "Diagnostics" | "Advanced";

/** Connected decks, their driver details, brightness and the USB event log. */
export const DevicesScreen: React.FC<DevicesScreenProps> = ({
  connectedDevices,
  activeDevice,
  frameStats,
  diagnostics,
  onRescan,
  onSelectDevice,
  onBrightness,
  onOpenWorkspace,
}) => {
  const [deviceDetailTab, setDeviceDetailTab] = useState<DetailTab>("Overview");

  return (    <div className="flex-1 grid grid-cols-[310px_1fr] overflow-hidden">
      <aside className="bg-[var(--bg-sidebar)] border-r border-[var(--border-subtle)] p-4 flex flex-col gap-3 overflow-y-auto">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-bold">Connected</h2>
          <button
            onClick={onRescan}
            className="p-1.5 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--accent-primary)] cursor-pointer"
          >
            <RefreshCw size={13} />
          </button>
        </div>
        {connectedDevices.length === 0 ? (
          <div className="px-3 py-4 rounded-lg border border-dashed border-[var(--border-subtle)] text-[11px] leading-relaxed text-[var(--text-muted)]">
            No device connected. Plug in a deck, then rescan.
          </div>
        ) : (
          connectedDevices.map((dev) => (
            <div
              key={dev.id}
              onClick={() => onSelectDevice(dev.id)}
              className={`p-3 rounded-lg border cursor-pointer transition-all ${
                dev.id === activeDevice.id
                  ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]"
                  : "bg-[var(--bg-panel)] border-[var(--border-subtle)] hover:border-[var(--border-strong)]"
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-bold text-xs">{dev.modelName}</span>
                <span className="text-[10.5px] px-2 py-0.5 rounded font-medium bg-[var(--status-success)]/15 text-[var(--status-success)]">
                  ● Connected
                </span>
              </div>
              <div className="text-[11px] font-mono text-[var(--text-secondary)] mt-1">
                USB VID:PID {dev.vidPidHex} • {dev.protocolFamily}
              </div>
              <div className="text-[11px] text-[var(--text-muted)] mt-1">
                {dev.keys.rows}×{dev.keys.cols} Keys ({dev.keys.pixelWidth}px)
                {dev.touchStrip ? ` • ${dev.touchStrip.zones}-Zone Strip` : ""}
                {dev.encoders ? ` • ${dev.encoders.count} Encoders` : ""}
              </div>
            </div>
          ))
        )}
      </aside>

      <main className="p-6 overflow-y-auto flex flex-col gap-5">
        {connectedDevices.length === 0 ? (
          <p className="text-sm text-[var(--text-secondary)]">
            Plug in a deck to see its driver, brightness, and diagnostics.
          </p>
        ) : (
        <>
        <div className="flex items-center justify-between border-b border-[var(--border-subtle)] pb-4">
          <div>
            <h1 className="text-xl font-bold">{activeDevice.modelName}</h1>
            <p className="text-xs text-[var(--text-secondary)] font-mono mt-0.5">
              Driver: {activeDevice.driverName} • Serial: {activeDevice.serialNumber} • FW:{" "}
              {activeDevice.firmwareVersion}
            </p>
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => onBrightness(activeDevice.brightness)}
              className="h-9 px-3.5 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--accent-primary)] text-xs font-medium cursor-pointer"
            >
              Identify Device (CRT..LIG)
            </button>
            <button
              onClick={() => onOpenWorkspace()}
              className="h-9 px-4 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white cursor-pointer"
            >
              Open Workspace
            </button>
          </div>
        </div>

        {/* 5 Device Detail Sub-tabs per Spec Section 5 */}
        <div className="flex gap-1 bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] p-1 rounded-lg w-fit">
          {(["Overview", "Controls", "Display", "Diagnostics", "Advanced"] as const).map(
            (tab) => (
              <button
                key={tab}
                onClick={() => setDeviceDetailTab(tab)}
                className={`px-3.5 py-1.5 rounded-md text-xs font-medium cursor-pointer ${
                  deviceDetailTab === tab
                    ? "bg-[var(--accent-soft)] text-white font-semibold"
                    : "text-[var(--text-secondary)] hover:text-white"
                }`}
              >
                {tab}
              </button>
            )
          )}
        </div>

        {deviceDetailTab === "Overview" && (
          <div className="grid grid-cols-3 gap-4">
            <div className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)]">
              <div className="text-xs text-[var(--text-secondary)]">Hardware Verification</div>
              <div className="text-base font-bold mt-1 text-[var(--status-success)]">
                {activeDevice.verifiedDriver
                  ? "✓ Verified Physical Driver"
                  : "Simulator / Protocol Profile"}
              </div>
              <p className="text-xs text-[var(--text-muted)] mt-1">
                Disassembled from SDLibrary1.dll (65-byte Windows HID reports on MI_00).
              </p>
            </div>
            <div className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)]">
              <div className="text-xs text-[var(--text-secondary)]">Anti-Freeze SHA-256 Deduplication</div>
              <div className="text-base font-bold mt-1 text-[var(--accent-primary)]">
                {frameStats.framesDeduplicatedSkipped} Redundant Writes Blocked
              </div>
              <p className="text-xs text-[var(--text-muted)] mt-1">
                Last sync latency: {frameStats.lastTransferLatencyMs}ms • Saved{" "}
                {(frameStats.bytesSaved / 1024).toFixed(1)} KB on USB bus.
              </p>
            </div>
            <div className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)]">
              <div className="text-xs text-[var(--text-secondary)]">Hardware Backlight Brightness</div>
              <div className="flex items-center gap-3 mt-2">
                <input
                  type="range"
                  min={0}
                  max={100}
                  value={activeDevice.brightness}
                  onChange={(e) => onBrightness(Number(e.target.value))}
                  className="flex-1 accent-[var(--accent-primary)] cursor-pointer"
                />
                <span className="font-mono text-xs font-bold w-10">
                  {activeDevice.brightness}%
                </span>
              </div>
              <p className="text-xs text-[var(--text-muted)] mt-1">
                Dispatches <code>CRT\0\0LIG</code> (RVA 0x1800225d0)
              </p>
            </div>
          </div>
        )}

        {deviceDetailTab === "Diagnostics" && (
          <div className="bg-[var(--bg-panel)] border border-[var(--border-subtle)] rounded-xl p-4 flex flex-col gap-2">
            <div className="text-xs font-bold text-[var(--text-secondary)] mb-1">
              USB HID & Driver Event Log
            </div>
            <div className="flex flex-col gap-1.5 max-h-80 overflow-y-auto font-mono text-xs">
              {diagnostics.map((d) => (
                <div
                  key={d.id}
                  className="p-2 rounded bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] flex items-center justify-between gap-4"
                >
                  <div>
                    <span className="text-[var(--accent-primary)] mr-2">[{d.stage}]</span>
                    <span className="text-[var(--text-primary)]">{d.message}</span>
                  </div>
                  {d.packetHex && (
                    <span className="text-[10.5px] text-[var(--status-success)] bg-[var(--bg-panel)] px-2 py-0.5 rounded">
                      {d.packetHex}
                    </span>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        {(deviceDetailTab === "Controls" ||
          deviceDetailTab === "Display" ||
          deviceDetailTab === "Advanced") && (
          <div className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] flex flex-col gap-2">
            <div className="text-sm font-bold">
              {deviceDetailTab} Configuration ({activeDevice.vidPidHex})
            </div>
            <div className="text-xs text-[var(--text-secondary)] font-mono">
              HID Path: {activeDevice.hidPath || "Virtual Simulator Bus"}
            </div>
            <div className="text-xs text-[var(--text-secondary)]">
              Key screens: {activeDevice.keys.count} × {activeDevice.keys.pixelWidth}×{activeDevice.keys.pixelHeight}px @{" "}
              {activeDevice.keys.rotation}°
              {activeDevice.touchStrip
                ? ` • Touch strip ${activeDevice.touchStrip.pixelWidth}×${activeDevice.touchStrip.pixelHeight}px`
                : ""}
              . PNG, JPEG, WebP, and GIF uploads are fitted to those pixels. GIF playback is paced at 80ms or slower.
            </div>
          </div>
        )}
        </>
        )}
      </main>
    </div>
  );
};
