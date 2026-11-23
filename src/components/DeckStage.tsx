import { encoderAudioLook } from "../lib/appAudio";
import React from "react";
import { LevelMeter, liveKeyLook, liveZoneValue } from "./LiveFace";
import { ScreenFace } from "./ScreenFace";
import { DeckIcon } from "./DeckIcon";
import { deckforgeApi } from "../bridge/tauriBridge";
import { ACTION_DRAG_TYPE, useCatalog, type KnobPart } from "./ActionDrawer";
import { PageApps } from "./AppRuleList";
import type { AutoSwitchRule } from "../types/deckforge";
import { ChevronLeft, ChevronRight, LayoutTemplate, Plus } from "lucide-react";
import {
  DEFAULT_KNOB_LED_COLORS,
  type DeckFaceState,
  type DeviceDescriptor,
  type KeyConfig,
  type MediaPreview,
  type Profile,
  type ProfilePage,
  type ScreenMediaConfig,
  type ScreenTarget,
} from "../types/deckforge";

interface DeckStageProps {
  activeDevice: DeviceDescriptor;
  activeKey: KeyConfig;
  activePage: ProfilePage;
  activeProfile: Profile;
  canvasZoom: number;
  clipFor: (cfg?: ScreenMediaConfig | null | undefined) => MediaPreview | null;
  face: DeckFaceState | null;
  handleAddPage: () => void;
  ingestFile: (file: File, target: ScreenTarget, index: number) => Promise<void>;
  livePressedKeyIdx: number | null;
  liveEncoderEvent?: { index: number; kind: "cw" | "ccw" | "push" } | null;
  liveTouchZoneIdx?: number | null;
  selectedPageIdx: number;
  setSelectedKeyIdx: React.Dispatch<React.SetStateAction<number>>;
  setSelectedPageIdx: React.Dispatch<React.SetStateAction<number>>;
  setSurface: React.Dispatch<React.SetStateAction<"key" | "zone" | "strip" | "encoder">>;
  setSurfaceIndex: React.Dispatch<React.SetStateAction<number>>;
  setTemplatesOpen: React.Dispatch<React.SetStateAction<boolean>>;
  surface: "key" | "zone" | "strip" | "encoder";
  surfaceIndex: number;
  onDropAction: (target: "key" | "zone" | "encoder", index: number, actionId: string, part?: KnobPart) => void;
  onMoveKey: (from: number, to: number) => void;
  onMoveKeyToPage: (keyIndex: number, pageIndex: number) => void;
  autoSwitchRules: AutoSwitchRule[];
  saveRules: (rules: AutoSwitchRule[]) => void;
}

const KEY_DRAG_TYPE = "application/x-deckforge-key";

export const DeckStage = React.memo(function DeckStage({
  activeDevice,
  activeKey,
  activePage,
  activeProfile,
  canvasZoom,
  clipFor,
  face,
  handleAddPage,
  ingestFile,
  livePressedKeyIdx,
  liveEncoderEvent,
  liveTouchZoneIdx,
  selectedPageIdx,
  setSelectedKeyIdx,
  setSelectedPageIdx,
  setSurface,
  setSurfaceIndex,
  setTemplatesOpen,
  surface,
  surfaceIndex,
  onDropAction,
  onMoveKey,
  onMoveKeyToPage,
  autoSwitchRules,
  saveRules,
}: DeckStageProps) {
  const [dropTarget, setDropTarget] = React.useState<number | null>(null);
  const [dropZone, setDropZone] = React.useState<string | null>(null);
  const [dropPart, setDropPart] = React.useState<KnobPart | null>(null);
  const [draggingKey, setDraggingKey] = React.useState<number | null>(null);
  const [dropPage, setDropPage] = React.useState<number | null>(null);
  const catalog = useCatalog();
  return (
    <div
      className="min-h-0 flex-1 flex flex-col items-center px-6 py-4 overflow-y-auto df-dot-grid"
      onClick={(event) => {
        const target = event.target as HTMLElement;
        if (target.closest("button, a, input, select, textarea, [role=button]")) return;
        catalog?.hide();
      }}
    >
      <div className="my-auto w-full flex flex-col items-center gap-3.5">
      <div
        style={{
          transform: `scale(${canvasZoom / 100})`,
          marginBlock: `${(canvasZoom - 100) * 1.8}px`,
        }}
        className="shrink-0 origin-center bg-gradient-to-b from-[var(--bg-elevated)]/90 to-[var(--bg-panel)] border border-[var(--border-strong)]/80 rounded-2xl p-5 ring-1 ring-black/40 shadow-[inset_0_1px_0_rgba(255,255,255,0.07),0_24px_56px_rgba(0,0,0,0.55)] transition-[transform,margin] duration-150 max-w-[650px] w-full flex flex-col gap-3.5"
      >
        {/* Dynamic Key Grid matching real hardware rows x cols */}
        <div
          className="grid gap-3"
          style={{
            gridTemplateColumns: `repeat(${activeDevice.keys.cols}, minmax(0, 1fr))`,
          }}
        >
          {activePage.keys.slice(0, activeDevice.keys.count).map((k) => {
            const isSelected = surface === "key" && k.index === activeKey.index;
            const isLivePressed = livePressedKeyIdx === k.index;
            const clip = clipFor(k.media);
            const baseLook = liveKeyLook(k.actionId, k.title, k.iconName, k.media ? null : face, k.actionParam);
            const lit = Boolean(
              k.latch &&
              face?.latchedKeys?.includes(k.index) &&
              face.latchedProfileId === activeProfile.id &&
              face.latchedPageId === activePage.id,
            );
            const look = lit ? { ...baseLook, title: `ON ${baseLook.title}`, icon: "Check" } : baseLook;
            return (
              <button
                key={k.index}
                type="button"
                aria-pressed={isSelected}
                aria-label={`Key ${k.index + 1}: ${look.title || "Empty"}${k.doublePress ? ", double-press" : ""}${k.hold ? ", hold" : ""}`}
                onClick={() => {
                  setSurface("key");
                  setSurfaceIndex(k.index);
                  setSelectedKeyIdx(k.index);
                }}
                draggable
                onDragStart={(event) => {
                  event.dataTransfer.setData(KEY_DRAG_TYPE, String(k.index));
                  event.dataTransfer.effectAllowed = "move";
                  setDraggingKey(k.index);
                }}
                onDragOver={(event) => {
                  event.preventDefault();
                  if (dropTarget !== k.index) setDropTarget(k.index);
                }}
                onDragLeave={() => setDropTarget((t) => (t === k.index ? null : t))}
                onDragEnd={() => {
                  setDropTarget(null);
                  setDraggingKey(null);
                  setDropPage(null);
                }}
                onDrop={(event) => {
                  event.preventDefault();
                  setDropTarget(null);
                  const actionId = event.dataTransfer.getData(ACTION_DRAG_TYPE);
                  const fromKey = event.dataTransfer.getData(KEY_DRAG_TYPE);
                  const file = event.dataTransfer.files?.[0];
                  if (actionId) onDropAction("key", k.index, actionId);
                  else if (fromKey !== "") onMoveKey(Number(fromKey), k.index);
                  else if (file) void ingestFile(file, "key", k.index);
                  setSurface("key");
                  setSurfaceIndex(k.index);
                  setSelectedKeyIdx(k.index);
                }}
                style={{
                  borderRadius: `${k.cornerRadius}px`,
                  background:
                    clip
                      ? "#0B0D14"
                      : k.backgroundStyle === "Gradient"
                      ? `radial-gradient(circle at 50% 28%, ${k.accentColor}44, ${k.bgColor})`
                      : k.bgColor,
                }}
                className={`aspect-[1/0.9] border border-[var(--border-subtle)] p-2 flex flex-col items-center justify-center gap-1.5 relative overflow-hidden cursor-pointer active:cursor-grabbing active:scale-[0.97] shadow-[inset_0_1px_0_rgba(255,255,255,0.08),0_2px_6px_rgba(0,0,0,0.35)] transition-[box-shadow,border-color,filter,transform] duration-150 group hover:brightness-110 ${
                  dropTarget === k.index
                    ? "ring-2 ring-[var(--accent-hover)] ring-offset-2 ring-offset-[var(--bg-panel)] scale-[1.03]"
                    : isLivePressed
                    ? "ring-2 ring-[var(--status-success)] ring-offset-2 ring-offset-[var(--bg-panel)] scale-95"
                    : isSelected
                    ? "ring-2 ring-[var(--accent-primary)] ring-offset-2 ring-offset-[var(--bg-panel)] shadow-[0_0_18px_rgba(124,131,247,0.24)]"
                    : lit
                    ? "ring-2 ring-[var(--status-success)]/85 ring-offset-2 ring-offset-[var(--bg-panel)]"
                    : "hover:border-[var(--border-strong)] hover:-translate-y-[1px]"
                }`}
              >
                {(k.doublePress || k.hold) && (
                  <span
                    aria-hidden
                    title={[k.doublePress && "Double-press set", k.hold && "Hold set"].filter(Boolean).join(" · ")}
                    className="absolute top-1.5 right-1.5 z-10 flex items-center gap-0.5"
                  >
                    {k.doublePress && <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-text)]/90 shadow-xs" />}
                    {k.hold && <span className="w-1.5 h-1.5 rounded-full bg-[var(--status-warning)]/90 shadow-xs" />}
                  </span>
                )}
                {clip ? (
                  <ScreenFace
                    clip={clip}
                    config={k.media}
                    iconName={look.icon}
                    accent={k.accentColor}
                    title={look.title}
                    showTitle={k.showTitle}
                    titleSize={k.titleSize}
                    className="absolute inset-0"
                  />
                ) : (
                  <>
                    <div
                      className="w-8 h-8 rounded-lg flex items-center justify-center transition-transform group-hover:scale-105"
                      style={{
                        backgroundColor: `${k.accentColor}22`,
                        border: `1px solid ${k.accentColor}55`,
                      }}
                    >
                      <DeckIcon name={look.icon} size={18} color={k.accentColor} />
                    </div>
                    {k.showTitle && (
                      <span
                        className={`font-semibold text-[var(--text-primary)] truncate max-w-full leading-tight ${
                          k.titleSize === "Small"
                            ? "text-[10px]"
                            : k.titleSize === "Large"
                            ? "text-[13px]"
                            : "text-[11.5px]"
                        }`}
                      >
                        {look.title}
                      </span>
                    )}
                    {look.level != null && (
                      <LevelMeter percent={look.level} accent={k.accentColor} muted={look.muted} />
                    )}
                  </>
                )}
              </button>
            );
          })}
        </div>

        {/* Secondary Touch Strip (if supported by device, e.g., Ajazz AKP05E Pro) */}
        {activeDevice.touchStrip && (
          <div className="flex flex-col gap-1.5">
            <button
              type="button"
              aria-pressed={surface === "strip"}
              onClick={() => setSurface("strip")}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => {
                event.preventDefault();
                const file = event.dataTransfer.files?.[0];
                if (file) void ingestFile(file, "strip", 0);
              }}
              className={`h-7 rounded-md border text-[10.5px] font-semibold cursor-pointer transition-colors ${
                surface === "strip"
                  ? "border-[var(--accent-primary)] bg-[var(--accent-soft)] text-white"
                  : "border-[var(--border-subtle)] bg-[var(--bg-input)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:border-[var(--border-strong)]"
              }`}
            >
              Whole touch strip · {activeDevice.touchStrip.pixelWidth}×{activeDevice.touchStrip.pixelHeight}
              {activePage.stripMedia ? " · image" : ""}
            </button>
            <div
              className="grid gap-2 bg-[var(--bg-app)] border border-[var(--border-subtle)] rounded-xl p-2"
              style={{ gridTemplateColumns: `repeat(${activeDevice.touchStrip.zones}, minmax(0, 1fr))` }}
            >
              {activePage.touchZones.slice(0, activeDevice.touchStrip.zones).map((tz) => {
                const zoneClip = clipFor(tz.media);
                const zoneSelected = surface === "zone" && surfaceIndex === tz.index;
                const zoneValue = liveZoneValue(tz.tapActionId, tz.valueText, tz.media ? null : face, tz.tapActionParam);
                const selectZone = () => {
                  setSurface("zone");
                  setSurfaceIndex(tz.index);
                };
                return (
                  <div
                    key={tz.index}
                    role="button"
                    tabIndex={0}
                    aria-pressed={zoneSelected}
                    aria-label={`Touch zone ${tz.index + 1}: ${tz.label}`}
                    onClick={selectZone}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        selectZone();
                      }
                    }}
                    onDragOver={(event) => {
                      event.preventDefault();
                      if (dropZone !== `zone-${tz.index}`) setDropZone(`zone-${tz.index}`);
                    }}
                    onDragLeave={() => setDropZone(null)}
                    onDrop={(event) => {
                      event.preventDefault();
                      setDropZone(null);
                      const actionId = event.dataTransfer.getData(ACTION_DRAG_TYPE);
                      const file = event.dataTransfer.files?.[0];
                      if (actionId) onDropAction("zone", tz.index, actionId);
                      else if (file) void ingestFile(file, "zone", tz.index);
                      setSurface("zone");
                      setSurfaceIndex(tz.index);
                    }}
                    className={`relative min-h-14 overflow-hidden px-2.5 py-1.5 rounded-lg bg-[var(--bg-panel)] border cursor-pointer flex items-center justify-between shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] transition-all duration-150 ${
                      dropZone === `zone-${tz.index}`
                        ? "border-[var(--accent-hover)] bg-[var(--bg-elevated)]"
                        : liveTouchZoneIdx === tz.index
                        ? "border-[var(--status-success)] ring-1 ring-[var(--status-success)]/50 scale-[0.98]"
                        : zoneSelected ? "border-[var(--accent-primary)] ring-1 ring-[var(--accent-primary)]/40" : "border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-elevated)]/60 hover:-translate-y-[0.5px]"
                    }`}
                  >
                    {zoneClip ? (
                      <ScreenFace
                        clip={zoneClip}
                        config={tz.media}
                        iconName="Image"
                        accent={tz.accentColor}
                        title={tz.label}
                        showTitle
                        className="absolute inset-0"
                      />
                    ) : (
                      <div className="min-w-0">
                        <div className="text-[10.5px] font-semibold text-[var(--text-muted)] truncate">
                          {tz.label}
                        </div>
                        <div
                          className="text-[11px] font-mono font-semibold truncate"
                          style={{ color: tz.accentColor }}
                        >
                          {zoneValue}
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Physical Rotary Encoders (if supported by device, e.g., Ajazz AKP05E Pro / MiraBox N3) */}
        {activeDevice.encoders && (
          <div
            className="grid gap-2"
            style={{
              gridTemplateColumns: `repeat(${activeDevice.encoders.count}, minmax(0, 1fr))`,
            }}
          >
            {activePage.encoders.slice(0, activeDevice.encoders.count).map((enc) => {
              const knobValue = encoderAudioLook(enc, face);
              const reactiveColor = (() => {
                if (!enc.ledReactive || !face) return null;
                const actions = [enc.pushAction, enc.cwAction, enc.ccwAction];
                if (actions.includes("audio.mute_mic") && face.micMuted) return "#F16B79";
                if (actions.some((a) => a === "audio.volume_mute" || a === "audio.volume_up" || a === "audio.volume_down") && face.speakerMuted) return "#F16B79";
                if (knobValue.muted) return "#F16B79";
                if (actions.includes("sys.game_mode") && face.gameMode) return "#35C98B";
                if (actions.some((a) => a === "media.play_pause" || a === "media.next" || a === "media.prev") && face.mediaPlaying) return "#35C98B";
                return null;
              })();
              const ledColor = (
                reactiveColor ||
                enc.ledColor ||
                DEFAULT_KNOB_LED_COLORS[enc.index] ||
                "#35C98B"
              ).toUpperCase();
              const ledOff = ledColor === "#000000";
              const isEncSelected = surface === "encoder" && surfaceIndex === enc.index;
              const selectEncoder = () => {
                setSurface("encoder");
                setSurfaceIndex(enc.index);
              };
              return (
              <div
                key={enc.index}
                role="button"
                tabIndex={0}
                aria-pressed={isEncSelected}
                aria-label={`Knob ${enc.index + 1}: ${enc.label}`}
                onClick={selectEncoder}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    selectEncoder();
                  }
                }}
                onDragOver={(event) => {
                  if (!event.dataTransfer.types.includes(ACTION_DRAG_TYPE)) return;
                  event.preventDefault();
                  if (dropZone !== `enc-${enc.index}`) setDropZone(`enc-${enc.index}`);
                }}
                onDragLeave={(event) => {
                  if (!event.currentTarget.contains(event.relatedTarget as Node)) setDropZone(null);
                }}
                onDrop={(event) => {
                  event.preventDefault();
                  setDropZone(null);
                  const actionId = event.dataTransfer.getData(ACTION_DRAG_TYPE);
                  if (!actionId) return;
                  onDropAction("encoder", enc.index, actionId, "push");
                  setSurface("encoder");
                  setSurfaceIndex(enc.index);
                }}
                className={`relative px-2.5 py-2 rounded-xl bg-[var(--bg-input)] border flex items-center gap-2.5 cursor-pointer shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] transition-all duration-150 ${
                  dropZone === `enc-${enc.index}`
                    ? "border-[var(--accent-hover)] bg-[var(--bg-elevated)]"
                    : liveEncoderEvent?.index === enc.index
                    ? `border-[var(--status-success)] ring-1 ring-[var(--status-success)]/50 ${liveEncoderEvent.kind === "push" ? "scale-[0.97]" : ""}`
                    : isEncSelected
                    ? "border-[var(--accent-primary)] ring-1 ring-[var(--accent-primary)]/40"
                    : "border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-elevated)]/60 hover:-translate-y-[0.5px]"
                }`}
              >
                {liveEncoderEvent?.index === enc.index && (
                  <span
                    aria-hidden
                    className="absolute top-1 right-1.5 text-[10px] font-bold text-[var(--status-success)]"
                  >
                    {liveEncoderEvent.kind === "cw" ? "↻" : liveEncoderEvent.kind === "ccw" ? "↺" : "●"}
                  </span>
                )}
                {dropZone === `enc-${enc.index}` && (
                  <div className="absolute inset-0 z-10 grid grid-cols-3 gap-px rounded-xl overflow-hidden bg-[var(--border-subtle)] backdrop-blur-[2px] animate-fade-in">
                    {(
                      [
                        ["ccw", "Turn left"],
                        ["push", "Press"],
                        ["cw", "Turn right"],
                      ] as Array<[KnobPart, string]>
                    ).map(([part, label]) => (
                      <div
                        key={part}
                        onDragOver={(event) => {
                          event.preventDefault();
                          if (dropPart !== part) setDropPart(part);
                        }}
                        onDrop={(event) => {
                          event.preventDefault();
                          event.stopPropagation();
                          setDropZone(null);
                          setDropPart(null);
                          const actionId = event.dataTransfer.getData(ACTION_DRAG_TYPE);
                          if (!actionId) return;
                          onDropAction("encoder", enc.index, actionId, part);
                          setSurface("encoder");
                          setSurfaceIndex(enc.index);
                        }}
                        className={`flex items-center justify-center text-[11px] font-medium transition-colors ${
                          dropPart === part ? "bg-[var(--accent-primary)] text-white" : "bg-[var(--bg-elevated)]/95 text-[var(--text-secondary)]"
                        }`}
                      >
                        {label}
                      </div>
                    ))}
                  </div>
                )}
                <div
                  style={
                    ledOff
                      ? { borderColor: "var(--border-subtle)" }
                      : {
                          borderColor: ledColor,
                          boxShadow: `0 0 10px ${ledColor}66, inset 0 0 4px ${ledColor}44`,
                        }
                  }
                  className="relative w-7 h-7 rounded-full bg-gradient-to-br from-[var(--border-strong)] to-[var(--bg-sidebar)] border-2 shrink-0 flex items-center justify-center shadow-[inset_0_1px_1px_rgba(255,255,255,0.12)]"
                >
                  <span
                    aria-hidden
                    className="absolute top-0.5 left-1/2 -translate-x-1/2 w-[2px] h-1.5 rounded-full bg-white/40"
                  />
                  <span
                    style={{ backgroundColor: ledOff ? "var(--text-muted)" : ledColor }}
                    className="w-1.5 h-1.5 rounded-full"
                  />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="text-[11px] font-semibold text-[var(--text-primary)] truncate">
                    {enc.label}
                  </div>
                  <div className="text-[10.5px] font-mono text-[var(--text-secondary)] truncate">
                    {knobValue.title}
                  </div>
                  {knobValue.level != null && (
                    <LevelMeter percent={knobValue.level} accent={ledOff ? "#35C98B" : ledColor} muted={knobValue.muted} />
                  )}
                </div>
              </div>
              );
            })}
          </div>
        )}
      </div>

      <div className="text-[11px] tabular-nums text-[var(--text-muted)] text-center">
        {activeDevice.keys.count} LCD screens · {activeDevice.keys.pixelWidth}×{activeDevice.keys.pixelHeight}px
        {activeDevice.keys.rotation ? ` · ${activeDevice.keys.rotation}°` : ""}
        {activeDevice.touchStrip
          ? ` · strip ${activeDevice.touchStrip.pixelWidth}×${activeDevice.touchStrip.pixelHeight}`
          : ""}
        {" · Drop a GIF onto any screen"}
      </div>

      {/* Page Switcher Bar below the Deck Bezel. While a key is dragged, it offers the other pages as drop targets. */}
      {draggingKey !== null && activeProfile.pages.length > 1 ? (
        <div className="flex items-center gap-1.5 flex-wrap justify-center min-h-8">
          <span className="text-[12px] text-[var(--text-muted)]">Move to</span>
          {activeProfile.pages.map((pg, idx) =>
            idx === selectedPageIdx ? null : (
              <div
                key={pg.id}
                onDragOver={(event) => {
                  event.preventDefault();
                  if (dropPage !== idx) setDropPage(idx);
                }}
                onDragLeave={() => setDropPage((p) => (p === idx ? null : p))}
                onDrop={(event) => {
                  event.preventDefault();
                  const from = Number(event.dataTransfer.getData(KEY_DRAG_TYPE));
                  setDraggingKey(null);
                  setDropPage(null);
                  if (!Number.isNaN(from)) onMoveKeyToPage(from, idx);
                }}
                className={`h-8 px-3 rounded-[var(--radius-sm)] border border-dashed flex items-center text-[12px] ${
                  dropPage === idx
                    ? "border-[var(--accent-primary)] bg-[var(--accent-soft)] text-[var(--text-primary)]"
                    : "border-[var(--border-strong)] text-[var(--text-secondary)]"
                }`}
              >
                {pg.name}
              </div>
            )
          )}
        </div>
      ) : (
      <div className="flex items-center flex-wrap justify-center gap-1.5">
        <div className="flex items-center bg-[var(--bg-panel)] border border-[var(--border-subtle)] focus-within:border-[var(--accent-primary)] focus-within:ring-2 focus-within:ring-[var(--accent-primary)]/20 rounded-[var(--radius-sm)] h-8 px-1 transition-all">
          <button
            type="button"
            disabled={selectedPageIdx === 0}
            title="Previous page (Page Up)"
            aria-label="Previous page"
            onClick={() => {
              deckforgeApi.holdAutoSwitch().catch(() => {});
              setSelectedPageIdx((p) => Math.max(0, p - 1));
            }}
            className="px-1.5 h-6 rounded text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] disabled:opacity-35 disabled:hover:bg-transparent disabled:cursor-default cursor-pointer"
          >
            <ChevronLeft size={13} />
          </button>
          <select
            aria-label="Current page"
            value={selectedPageIdx}
            onChange={(e) => {
              deckforgeApi.holdAutoSwitch().catch(() => {});
              setSelectedPageIdx(Number(e.target.value));
            }}
            className="bg-transparent text-xs font-medium text-[var(--text-primary)] px-2 outline-none focus-ring-none cursor-pointer"
          >
            {activeProfile.pages.map((pg, idx) => (
              <option key={pg.id} value={idx} className="bg-[var(--bg-panel)]">
                {pg.name}
              </option>
            ))}
          </select>
          <button
            type="button"
            disabled={selectedPageIdx >= activeProfile.pages.length - 1}
            title="Next page (Page Down)"
            aria-label="Next page"
            onClick={() => {
              deckforgeApi.holdAutoSwitch().catch(() => {});
              setSelectedPageIdx((p) => Math.min(activeProfile.pages.length - 1, p + 1));
            }}
            className="px-1.5 h-6 rounded text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] disabled:opacity-35 disabled:hover:bg-transparent disabled:cursor-default cursor-pointer"
          >
            <ChevronRight size={13} />
          </button>
        </div>

        <button
          type="button"
          onClick={handleAddPage}
          title="Add a page"
          aria-label="Add a page"
          className="w-8 h-8 rounded-[var(--radius-sm)] bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center justify-center cursor-pointer transition-colors"
        >
          <Plus size={13} />
        </button>
        <button
          type="button"
          onClick={() => setTemplatesOpen(true)}
          title="Page templates"
          aria-label="Page templates"
          className="h-8 px-2.5 rounded-[var(--radius-sm)] bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center justify-center gap-1.5 text-xs font-medium cursor-pointer transition-colors"
        >
          <LayoutTemplate size={13} /> Templates
        </button>
        <PageApps rules={autoSwitchRules} profile={activeProfile} pageIndex={selectedPageIdx} saveRules={saveRules} />
      </div>
      )}
      </div>
    </div>
  );
});
