import React from "react";
import { LevelMeter, liveKeyLook, liveZoneValue } from "./LiveFace";
import { ScreenFace } from "./ScreenFace";
import { DeckIcon } from "./DeckIcon";
import { deckforgeApi } from "../bridge/tauriBridge";
import { ACTION_DRAG_TYPE } from "./ActionDrawer";
import { PageApps } from "./AppRuleList";
import type { AutoSwitchRule } from "../types/deckforge";
import { ChevronLeft, ChevronRight, LayoutTemplate, Plus } from "lucide-react";
import type { DeckFaceState, DeviceDescriptor, KeyConfig, MediaPreview, Profile, ProfilePage, ScreenMediaConfig, ScreenTarget } from "../types/deckforge";

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
  selectedPageIdx: number;
  setSelectedKeyIdx: React.Dispatch<React.SetStateAction<number>>;
  setSelectedPageIdx: React.Dispatch<React.SetStateAction<number>>;
  setSurface: React.Dispatch<React.SetStateAction<"key" | "zone" | "strip" | "encoder">>;
  setSurfaceIndex: React.Dispatch<React.SetStateAction<number>>;
  setTemplatesOpen: React.Dispatch<React.SetStateAction<boolean>>;
  surface: "key" | "zone" | "strip" | "encoder";
  surfaceIndex: number;
  onDropAction: (keyIndex: number, actionId: string) => void;
  onMoveKey: (from: number, to: number) => void;
  autoSwitchRules: AutoSwitchRule[];
  saveRules: (rules: AutoSwitchRule[]) => void;
}

const KEY_DRAG_TYPE = "application/x-deckforge-key";

export const DeckStage: React.FC<DeckStageProps> = ({
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
  autoSwitchRules,
  saveRules,
}) => {
  const [dropTarget, setDropTarget] = React.useState<number | null>(null);
  return (
    <div className="min-h-0 flex-1 flex flex-col items-center justify-start px-6 py-3 gap-3 overflow-y-auto">
      <div
        style={{ transform: `scale(${canvasZoom / 100})` }}
        className="shrink-0 origin-top bg-[var(--bg-panel)] border border-[var(--border-subtle)] rounded-[var(--radius-lg)] p-4 transition-transform max-w-[650px] w-full flex flex-col gap-3"
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
            const look = liveKeyLook(k.actionId, k.title, k.iconName, k.media ? null : face);
            return (
              <button
                key={k.index}
                onClick={() => {
                  setSurface("key");
                  setSurfaceIndex(k.index);
                  setSelectedKeyIdx(k.index);
                }}
                draggable
                onDragStart={(event) => {
                  event.dataTransfer.setData(KEY_DRAG_TYPE, String(k.index));
                  event.dataTransfer.effectAllowed = "move";
                }}
                onDragOver={(event) => {
                  event.preventDefault();
                  if (dropTarget !== k.index) setDropTarget(k.index);
                }}
                onDragLeave={() => setDropTarget((t) => (t === k.index ? null : t))}
                onDragEnd={() => setDropTarget(null)}
                onDrop={(event) => {
                  event.preventDefault();
                  setDropTarget(null);
                  const actionId = event.dataTransfer.getData(ACTION_DRAG_TYPE);
                  const fromKey = event.dataTransfer.getData(KEY_DRAG_TYPE);
                  const file = event.dataTransfer.files?.[0];
                  if (actionId) onDropAction(k.index, actionId);
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
                className={`aspect-[1/0.9] border border-[var(--border-subtle)] p-2 flex flex-col items-center justify-center gap-1.5 relative overflow-hidden cursor-pointer active:cursor-grabbing transition-[outline-color,transform] duration-100 outline outline-2 outline-offset-2 group ${
                  dropTarget === k.index
                    ? "outline-[var(--accent-hover)] scale-[1.03]"
                    : isLivePressed
                    ? "outline-[var(--status-success)] scale-95"
                    : isSelected
                    ? "outline-[var(--accent-primary)]"
                    : "outline-transparent hover:border-[var(--border-strong)]"
                }`}
              >
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
              onClick={() => setSurface("strip")}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => {
                event.preventDefault();
                const file = event.dataTransfer.files?.[0];
                if (file) void ingestFile(file, "strip", 0);
              }}
              className={`h-7 rounded-md border text-[10.5px] font-semibold cursor-pointer ${
                surface === "strip"
                  ? "border-[var(--accent-primary)] bg-[var(--accent-soft)] text-white"
                  : "border-[var(--border-subtle)] bg-[var(--bg-input)] text-[var(--text-secondary)]"
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
                const zoneValue = liveZoneValue(tz.tapActionId, tz.valueText, tz.media ? null : face);
                return (
                  <div
                    key={tz.index}
                    onClick={() => {
                      setSurface("zone");
                      setSurfaceIndex(tz.index);
                    }}
                    onDragOver={(event) => event.preventDefault()}
                    onDrop={(event) => {
                      event.preventDefault();
                      const file = event.dataTransfer.files?.[0];
                      if (file) void ingestFile(file, "zone", tz.index);
                    }}
                    className={`relative min-h-14 overflow-hidden px-2.5 py-1.5 rounded-lg bg-[var(--bg-panel)] border cursor-pointer flex items-center justify-between ${
                      zoneSelected ? "border-[var(--accent-primary)]" : "border-[var(--border-subtle)] hover:border-[var(--accent-primary)]/60"
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
                      <div>
                        <div className="text-[9.5px] font-bold text-[var(--text-muted)] tracking-wider">
                          {tz.label}
                        </div>
                        <div
                          className="text-[11px] font-mono font-semibold"
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
              const volumeKnob = [enc.cwAction, enc.ccwAction, enc.pushAction].some((action) =>
                action.includes("volume")
              );
              const knobValue =
                volumeKnob && face?.volumePercent != null
                  ? `${face.volumePercent}%${face.speakerMuted ? " muted" : ""}`
                  : enc.valueText;
              return (
              <div
                key={enc.index}
                onClick={() => {
                  setSurface("encoder");
                  setSurfaceIndex(enc.index);
                }}
                className={`px-2.5 py-1.5 rounded-xl bg-[var(--bg-input)] border flex items-center gap-2.5 cursor-pointer ${
                  surface === "encoder" && surfaceIndex === enc.index
                    ? "border-[var(--accent-primary)]"
                    : "border-[var(--border-subtle)] hover:border-[var(--accent-primary)]/60"
                }`}
              >
                <div className="w-6 h-6 rounded-full bg-gradient-to-br from-[var(--border-strong)] to-[var(--bg-sidebar)] border border-dashed border-[var(--text-secondary)]/60 shrink-0" />
                <div className="min-w-0">
                  <div className="text-[11px] font-semibold text-[var(--text-primary)] truncate">
                    {enc.label}
                  </div>
                  <div className="text-[9.5px] font-mono text-[var(--text-secondary)] truncate">
                    {knobValue}
                  </div>
                  {volumeKnob && face?.volumePercent != null && (
                    <LevelMeter percent={face.volumePercent} accent="#35C98B" muted={face.speakerMuted === true} />
                  )}
                </div>
              </div>
              );
            })}
          </div>
        )}
      </div>

      <div className="text-[11px] text-[var(--text-muted)]">
        {activeDevice.keys.count} LCD screens · {activeDevice.keys.pixelWidth}×{activeDevice.keys.pixelHeight}px
        {activeDevice.keys.rotation ? ` · ${activeDevice.keys.rotation}°` : ""}
        {activeDevice.touchStrip
          ? ` · strip ${activeDevice.touchStrip.pixelWidth}×${activeDevice.touchStrip.pixelHeight}`
          : ""}
        . Drop a GIF onto any screen.
      </div>

      {/* Page Switcher Bar below the Deck Bezel */}
      <div className="flex items-center gap-1.5">
        <div className="flex items-center bg-[var(--bg-panel)] border border-[var(--border-subtle)] rounded-md h-7 px-1">
          <button
            onClick={() => {
              deckforgeApi.holdAutoSwitch().catch(() => {});
              setSelectedPageIdx((p) => Math.max(0, p - 1));
            }}
            className="px-1.5 text-[var(--text-secondary)] hover:text-white cursor-pointer"
          >
            <ChevronLeft size={13} />
          </button>
          <select
            value={selectedPageIdx}
            onChange={(e) => {
              deckforgeApi.holdAutoSwitch().catch(() => {});
              setSelectedPageIdx(Number(e.target.value));
            }}
            className="bg-transparent text-xs font-medium text-[var(--text-primary)] px-2 outline-none cursor-pointer"
          >
            {activeProfile.pages.map((pg, idx) => (
              <option key={pg.id} value={idx} className="bg-[var(--bg-panel)]">
                {pg.name}
              </option>
            ))}
          </select>
          <button
            onClick={() => {
              deckforgeApi.holdAutoSwitch().catch(() => {});
              setSelectedPageIdx((p) => Math.min(activeProfile.pages.length - 1, p + 1));
            }}
            className="px-1.5 text-[var(--text-secondary)] hover:text-white cursor-pointer"
          >
            <ChevronRight size={13} />
          </button>
        </div>

        <button
          onClick={handleAddPage}
          title="Add New Page"
          className="w-7 h-7 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--accent-primary)] text-[var(--text-secondary)] hover:text-white flex items-center justify-center cursor-pointer"
        >
          <Plus size={13} />
        </button>
        <button
          onClick={() => setTemplatesOpen(true)}
          title="Page templates"
          aria-label="Page templates"
          className="h-7 px-2 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--accent-primary)] text-[var(--text-secondary)] hover:text-white flex items-center justify-center gap-1.5 text-[11px] font-medium cursor-pointer"
        >
          <LayoutTemplate size={13} /> Templates
        </button>
        <PageApps rules={autoSwitchRules} profile={activeProfile} pageIndex={selectedPageIdx} saveRules={saveRules} />
      </div>
    </div>
  );
};
