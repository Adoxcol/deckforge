import { PerformancePanel } from "../components/PerformancePanel";
import React, { useState } from "react";
import { RefreshCw, RotateCcw } from "lucide-react";
import {
  DEFAULT_KNOB_LED_COLORS,
  type DeviceDescriptor,
  type DiagnosticEntry,
  type EncoderConfig,
  type FrameCacheStats,
  type KnobLedEffect,
} from "../types/deckforge";

const DEVICE_KNOB_SWATCHES = ["#35C98B", "#56B9F2", "#8B5CF6", "#F4B860", "#F16B79", "#FFFFFF"] as const;

interface DevicesScreenProps {
  connectedDevices: DeviceDescriptor[];
  activeDevice: DeviceDescriptor;
  frameStats: FrameCacheStats;
  diagnostics: DiagnosticEntry[];
  onRescan: () => void;
  onSelectDevice: (deviceId: string) => void;
  onBrightness: (percent: number) => void;
  onOpenWorkspace: () => void;
  /** Restarts the deck like a replug. Resolves when it is done or has failed. */
  onRestartDeck: () => Promise<void>;
  encoders?: EncoderConfig[];
  onKnobLedChange?: (knobIndex: number | "all", colorHex: string) => void;
  onResetKnobLeds?: () => void;
  ledBrightness?: number;
  onLedBrightness?: (percent: number) => void;
  ledFollowSleep?: boolean;
  onLedSleepMode?: (followSystemSleep: boolean) => void;
  knobLedEffect?: KnobLedEffect;
  onKnobLedEffectChange?: (effect: KnobLedEffect) => void;
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
  onRestartDeck,
  encoders,
  onKnobLedChange,
  onResetKnobLeds,
  ledBrightness = 100,
  onLedBrightness,
  ledFollowSleep = true,
  onLedSleepMode,
  knobLedEffect = "static",
  onKnobLedEffectChange,
}) => {
  const [deviceDetailTab, setDeviceDetailTab] = useState<DetailTab>("Overview");
  const [restarting, setRestarting] = useState(false);
  const [rescanning, setRescanning] = useState(false);

  return (    <div className="flex-1 min-h-0 grid grid-cols-[248px_minmax(0,1fr)] lg:grid-cols-[288px_minmax(0,1fr)] overflow-hidden animate-screen-in">
      <aside className="min-h-0 bg-[var(--bg-sidebar)] border-r border-[var(--border-subtle)] p-4 flex flex-col gap-3 overflow-y-auto">
        <div className="flex items-center justify-between">
          <h2 className="text-[13px] font-bold text-[var(--text-primary)]">Connected</h2>
          <button
            type="button"
            onClick={() => {
              setRescanning(true);
              onRescan();
              window.setTimeout(() => setRescanning(false), 600);
            }}
            title="Rescan USB devices"
            aria-label="Rescan USB devices"
            className="w-7 h-7 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-elevated)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center justify-center cursor-pointer transition-colors"
          >
            <RefreshCw size={13} className={rescanning ? "animate-spin" : ""} />
          </button>
        </div>
        {connectedDevices.length === 0 ? (
          <div className="px-3 py-4 rounded-lg border border-dashed border-[var(--border-subtle)] bg-[var(--bg-panel)]/40 text-[11px] leading-relaxed text-[var(--text-muted)]">
            No device connected. Plug in a deck, then rescan.
          </div>
        ) : (
          connectedDevices.map((dev) => (
            <div
              key={dev.id}
              role="button"
              tabIndex={0}
              aria-pressed={dev.id === activeDevice.id}
              onClick={() => onSelectDevice(dev.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onSelectDevice(dev.id);
                }
              }}
              className={`p-3 rounded-xl border cursor-pointer active:scale-[0.99] transition-all ${
                dev.id === activeDevice.id
                  ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/70 shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]"
                  : "bg-[var(--bg-panel)] border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-elevated)]"
              }`}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-bold text-xs truncate">{dev.modelName}</span>
                <span className="inline-flex items-center gap-1.5 text-[10.5px] px-2 py-0.5 rounded-full font-medium bg-[var(--status-success)]/15 text-[var(--status-success)] shrink-0">
                  <span className="w-1.5 h-1.5 rounded-full bg-[var(--status-success)] shadow-[0_0_6px_var(--status-success)]" />
                  Connected
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
          <>
            <PerformancePanel />
            <div className="p-6 rounded-xl border border-dashed border-[var(--border-subtle)] bg-[var(--bg-panel)]/40 text-center text-sm text-[var(--text-secondary)]">
              Plug in a deck to see its driver, brightness, and diagnostics.
            </div>
          </>
        ) : (
        <>
        <div className="flex items-center justify-between flex-wrap gap-3 border-b border-[var(--border-subtle)] pb-4">
          <div>
            <h1 className="text-xl font-bold">{activeDevice.modelName}</h1>
            <p className="text-xs text-[var(--text-secondary)] font-mono mt-0.5">
              Driver: {activeDevice.driverName} • Serial: {activeDevice.serialNumber} • FW:{" "}
              {activeDevice.firmwareVersion}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => onBrightness(activeDevice.brightness)}
              className="h-9 px-3.5 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] active:scale-[0.98] text-xs font-medium cursor-pointer transition-all"
            >
              Identify deck
            </button>
            <button
              type="button"
              onClick={async () => {
                setRestarting(true);
                try {
                  await onRestartDeck();
                } finally {
                  setRestarting(false);
                }
              }}
              disabled={restarting}
              title="Screen stuck black or not responding? This resets the USB port the deck is on, so it disconnects and reconnects. Windows asks for permission first. If the screen is still black afterwards, unplug the deck and plug it back in."
              className="h-9 px-3.5 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] active:scale-[0.98] text-xs font-medium cursor-pointer disabled:opacity-60 disabled:cursor-wait disabled:active:scale-100 flex items-center gap-1.5 transition-all"
            >
              <RotateCcw size={13} className={restarting ? "animate-spin" : ""} />
              {restarting ? "Restarting…" : "Restart deck"}
            </button>
            <button
              type="button"
              onClick={() => onOpenWorkspace()}
              className="h-9 px-4 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white cursor-pointer df-cta active:scale-[0.98] transition-all"
            >
              Open Workspace
            </button>
          </div>
        </div>

        {/* 5 Device Detail Sub-tabs per Spec Section 5 */}
        <div role="tablist" aria-label="Device details" className="flex flex-wrap gap-1 bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] p-1 rounded-lg w-fit">
          {(["Overview", "Controls", "Display", "Diagnostics", "Advanced"] as const).map(
            (tab) => (
              <button
                key={tab}
                type="button"
                role="tab"
                aria-selected={deviceDetailTab === tab}
                onClick={() => setDeviceDetailTab(tab)}
                className={`px-3.5 py-1.5 rounded-md text-xs font-medium cursor-pointer transition-all border ${
                  deviceDetailTab === tab
                    ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-[var(--text-primary)] font-semibold shadow-xs"
                    : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
                }`}
              >
                {tab}
              </button>
            )
          )}
        </div>

        {deviceDetailTab === "Overview" && (
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] df-card-surface transition-colors">
              <div className="text-[12px] text-[var(--text-secondary)]">Driver</div>
              <div className={`text-[15px] font-semibold mt-1 ${activeDevice.verifiedDriver ? "text-[var(--status-success)]" : "text-[var(--text-primary)]"}`}>
                {activeDevice.verifiedDriver ? "Tested with this deck" : "Preview only"}
              </div>
              <p className="text-[12px] text-[var(--text-muted)] mt-1">
                {activeDevice.verifiedDriver
                  ? "DeckForge talks to this model directly over USB."
                  : "No hardware is attached; changes show in the editor only."}
              </p>
            </div>
            <div className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] df-card-surface transition-colors">
              <div className="text-[12px] text-[var(--text-secondary)]">Screen updates</div>
              <div className="text-[15px] font-semibold tabular-nums mt-1 text-[var(--text-primary)]">
                {frameStats.lastTransferLatencyMs} ms last update
              </div>
              <p className="text-[12px] text-[var(--text-muted)] mt-1 tabular-nums">
                {frameStats.framesDeduplicatedSkipped} unchanged images skipped, saving{" "}
                {(frameStats.bytesSaved / 1024).toFixed(1)} KB of USB traffic.
              </p>
            </div>
            <div className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] df-card-surface transition-colors">
              <div className="text-[12px] text-[var(--text-secondary)]">Brightness</div>
              <div className="flex items-center gap-3 mt-2">
                <input
                  aria-label="Device brightness"
                  type="range"
                  min={0}
                  max={100}
                  value={activeDevice.brightness}
                  onChange={(e) => onBrightness(Number(e.target.value))}
                  className="flex-1 accent-[var(--accent-primary)] cursor-pointer"
                />
                <span className="font-mono text-xs font-bold w-10 text-right tabular-nums">
                  {activeDevice.brightness}%
                </span>
              </div>
              <p className="text-[12px] text-[var(--text-muted)] mt-1">Applies to the deck right away.</p>
            </div>
          </div>
        )}

        {deviceDetailTab === "Overview" && activeDevice.encoders && encoders && onKnobLedChange && onResetKnobLeds && (
          <div className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] flex flex-col gap-3">
            <div className="flex items-center justify-between flex-wrap gap-3">
              <div>
                <div className="text-[13px] font-bold text-[var(--text-primary)]">Knob LED Ring Colors</div>
                <p className="text-[11.5px] text-[var(--text-muted)] mt-0.5">
                  Click a swatch or custom picker to update the hardware knob LEDs live via CRT\0\0SETLB.
                </p>
              </div>
              <div className="flex items-center flex-wrap gap-2 shrink-0">
                {onKnobLedEffectChange && (
                  <div role="group" aria-label="LED animation" className="flex items-center gap-0.5 bg-[var(--bg-input)] border border-[var(--border-subtle)] p-0.5 rounded-md">
                    {(["static", "breathe", "rainbow"] as const).map((mode) => (
                      <button
                        key={mode}
                        type="button"
                        aria-pressed={knobLedEffect === mode}
                        onClick={() => onKnobLedEffectChange(mode)}
                        className={`h-7 px-2.5 rounded-[4px] text-[11px] font-medium capitalize cursor-pointer border transition-all ${
                          knobLedEffect === mode
                            ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-[var(--text-primary)] font-semibold shadow-xs"
                            : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
                        }`}
                      >
                        {mode}
                      </button>
                    ))}
                  </div>
                )}
                <button
                  type="button"
                  onClick={() => onKnobLedChange("all", "#000000")}
                  className="h-8 px-3 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--accent-primary)] hover:bg-[var(--bg-elevated)] active:scale-[0.98] text-xs font-medium cursor-pointer transition-all"
                >
                  All Off
                </button>
                <button
                  type="button"
                  onClick={onResetKnobLeds}
                  className="h-8 px-3 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--status-warning)] hover:bg-[var(--bg-elevated)] active:scale-[0.98] text-xs font-medium cursor-pointer transition-all"
                >
                  Factory Reset
                </button>
              </div>
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
              {encoders.slice(0, activeDevice.encoders.count).map((enc) => {
                const ledColor = (enc.ledColor || DEFAULT_KNOB_LED_COLORS[enc.index] || "#35C98B").toUpperCase();
                const ledOff = ledColor === "#000000";
                return (
                  <div
                    key={enc.index}
                    className="p-3 rounded-lg bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] flex flex-col gap-2"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <div className="flex items-center gap-2 min-w-0">
                        <div
                          style={
                            ledOff
                              ? { borderColor: "var(--border-subtle)" }
                              : {
                                  borderColor: ledColor,
                                  boxShadow: `0 0 10px ${ledColor}66`,
                                }
                          }
                          className="w-5 h-5 rounded-full bg-[var(--bg-panel)] border-2 shrink-0 flex items-center justify-center"
                        >
                          <span
                            style={{ backgroundColor: ledOff ? "var(--text-muted)" : ledColor }}
                            className="w-1.5 h-1.5 rounded-full"
                          />
                        </div>
                        <span className="text-xs font-semibold truncate">{enc.label || `Knob ${enc.index + 1}`}</span>
                      </div>
                      <span className="text-[10px] font-mono text-[var(--text-muted)]">
                        {ledOff ? "OFF" : ledColor}
                      </span>
                    </div>
                    <div className="flex items-center flex-wrap gap-1.5">
                      {DEVICE_KNOB_SWATCHES.map((hex) => (
                        <button
                          key={hex}
                          type="button"
                          title={hex}
                          aria-label={`Set knob ${enc.index + 1} LED to ${hex}`}
                          aria-pressed={ledColor === hex}
                          onClick={() => onKnobLedChange(enc.index, hex)}
                          style={{ backgroundColor: hex }}
                          className={`w-5 h-5 rounded cursor-pointer transition-transform ${
                            ledColor === hex ? "ring-2 ring-[var(--accent-primary)] ring-offset-2 ring-offset-[var(--bg-sidebar)] scale-110" : "border border-white/15 hover:scale-105"
                          }`}
                        />
                      ))}
                      <label
                        title="Custom RGB color"
                        className="h-5 px-1.5 rounded bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] focus-within:border-[var(--accent-primary)] focus-within:ring-2 focus-within:ring-[var(--accent-primary)]/20 inline-flex items-center gap-1 cursor-pointer transition-all"
                      >
                        <input
                          type="color"
                          aria-label={`Custom color for knob ${enc.index + 1}`}
                          value={ledOff ? "#000000" : ledColor}
                          onChange={(e) => onKnobLedChange(enc.index, e.target.value.toUpperCase())}
                          className="w-3.5 h-3.5 bg-transparent border-0 p-0 cursor-pointer focus-ring-none"
                        />
                        <span className="text-[10px] text-[var(--text-secondary)]">Pick</span>
                      </label>
                    </div>
                  </div>
                );
              })}
            </div>
            {(onLedBrightness || onLedSleepMode) && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-2 border-t border-[var(--border-subtle)]">
                {onLedBrightness && (
                  <div className="flex flex-col gap-1.5">
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-[var(--text-secondary)]">LED Brightness</span>
                      <span className="font-mono font-bold tabular-nums">{ledBrightness}%</span>
                    </div>
                    <input
                      aria-label="Knob LED brightness"
                      type="range"
                      min={0}
                      max={100}
                      value={ledBrightness}
                      onChange={(e) => onLedBrightness(Number(e.target.value))}
                      className="w-full accent-[var(--accent-primary)] cursor-pointer"
                    />
                  </div>
                )}
                {onLedSleepMode && (
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="text-xs font-medium text-[var(--text-primary)]">LED Sleep Behavior</div>
                      <div className="text-[11px] text-[var(--text-muted)]">
                        {ledFollowSleep ? "Turns LEDs off with PC sleep" : "Keeps LEDs on continuously"}
                      </div>
                    </div>
                    <div role="group" aria-label="LED sleep behavior" className="flex items-center gap-0.5 bg-[var(--bg-input)] border border-[var(--border-subtle)] p-0.5 rounded-md shrink-0">
                      <button
                        type="button"
                        aria-pressed={ledFollowSleep}
                        onClick={() => onLedSleepMode(true)}
                        className={`h-7 px-2.5 rounded-[4px] text-[11px] font-medium cursor-pointer border transition-all ${
                          ledFollowSleep
                            ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-[var(--text-primary)] font-semibold shadow-xs"
                            : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
                        }`}
                      >
                        Follow Sleep
                      </button>
                      <button
                        type="button"
                        aria-pressed={!ledFollowSleep}
                        onClick={() => onLedSleepMode(false)}
                        className={`h-7 px-2.5 rounded-[4px] text-[11px] font-medium cursor-pointer border transition-all ${
                          !ledFollowSleep
                            ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-[var(--text-primary)] font-semibold shadow-xs"
                            : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
                        }`}
                      >
                        Always On
                      </button>
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {deviceDetailTab === "Diagnostics" && (
          <div className="flex flex-col gap-5">
            <PerformancePanel />
            <div className="bg-[var(--bg-panel)] border border-[var(--border-subtle)] rounded-xl p-4 flex flex-col gap-2">
              <div className="text-xs font-bold text-[var(--text-secondary)] mb-1">
                USB HID & Driver Event Log
              </div>
              <div className="flex flex-col gap-1.5 max-h-80 overflow-y-auto font-mono text-xs">
                {diagnostics.length === 0 ? (
                  <div className="py-6 text-center text-[var(--text-muted)] font-sans">
                    No USB HID or driver events recorded yet.
                  </div>
                ) : (
                  diagnostics.map((d) => (
                    <div
                      key={d.id}
                      className="p-2 rounded-lg bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] flex items-center justify-between gap-4 transition-colors"
                    >
                      <div className="min-w-0 flex items-center gap-2">
                        <span className="px-1.5 py-0.5 rounded bg-[var(--accent-soft)] text-[var(--accent-text)] text-[10.5px] font-semibold shrink-0">{d.stage}</span>
                        <span className="text-[var(--text-primary)] break-words">{d.message}</span>
                      </div>
                      {d.packetHex && (
                        <span className="text-[10.5px] text-[var(--status-success)] bg-[var(--bg-panel)] px-2 py-0.5 rounded shrink-0">
                          {d.packetHex}
                        </span>
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        )}

        {(deviceDetailTab === "Controls" ||
          deviceDetailTab === "Display" ||
          deviceDetailTab === "Advanced") && (
          <div className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] flex flex-col gap-3">
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm font-bold text-[var(--text-primary)]">
                {deviceDetailTab} Configuration
              </div>
              <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] text-[var(--text-secondary)]">
                VID:PID {activeDevice.vidPidHex}
              </span>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-2.5 text-xs">
              <div className="p-3 rounded-lg bg-[var(--bg-sidebar)] border border-[var(--border-subtle)]">
                <div className="text-[11px] text-[var(--text-muted)]">HID Path</div>
                <div className="font-mono text-[var(--text-primary)] mt-1 break-all">{activeDevice.hidPath || "Virtual Simulator Bus"}</div>
              </div>
              <div className="p-3 rounded-lg bg-[var(--bg-sidebar)] border border-[var(--border-subtle)]">
                <div className="text-[11px] text-[var(--text-muted)]">Key Matrix</div>
                <div className="font-mono text-[var(--text-primary)] mt-1">
                  {activeDevice.keys.count} keys ({activeDevice.keys.rows}×{activeDevice.keys.cols}) · {activeDevice.keys.pixelWidth}×{activeDevice.keys.pixelHeight}px @ {activeDevice.keys.rotation}°
                </div>
              </div>
              {activeDevice.touchStrip && (
                <div className="p-3 rounded-lg bg-[var(--bg-sidebar)] border border-[var(--border-subtle)]">
                  <div className="text-[11px] text-[var(--text-muted)]">Touch Strip</div>
                  <div className="font-mono text-[var(--text-primary)] mt-1">
                    {activeDevice.touchStrip.zones} zones · {activeDevice.touchStrip.pixelWidth}×{activeDevice.touchStrip.pixelHeight}px
                  </div>
                </div>
              )}
              {activeDevice.encoders && (
                <div className="p-3 rounded-lg bg-[var(--bg-sidebar)] border border-[var(--border-subtle)]">
                  <div className="text-[11px] text-[var(--text-muted)]">Rotary Encoders</div>
                  <div className="font-mono text-[var(--text-primary)] mt-1">
                    {activeDevice.encoders.count} dials with RGB LED rings
                  </div>
                </div>
              )}
            </div>
            <p className="text-xs text-[var(--text-secondary)] leading-relaxed">
              PNG, JPEG, WebP, and GIF uploads are fitted to those pixels. GIF playback is paced at 80ms or slower.
            </p>
          </div>
        )}
        </>
        )}
      </main>
    </div>
  );
};
