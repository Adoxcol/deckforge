import React from "react";
import { LevelMeter, liveKeyLook, liveZoneValue } from "./LiveFace";
import { ScreenFace } from "./ScreenFace";
import { DeckIcon } from "./DeckIcon";
import { deckforgeApi } from "../bridge/tauriBridge";
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
}

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
}) => {
  return (
    <div className="flex-1 flex flex-col items-center justify-center px-6 py-3 gap-3 overflow-y-auto">
      <div
        style={{ transform: `scale(${canvasZoom / 100})` }}
        className="bg-[#151925] border border-[#394158] rounded-2xl p-4 shadow-[0_24px_60px_rgba(0,0,0,0.65),inset_0_1px_0_rgba(255,255,255,0.06)] transition-transform max-w-[650px] w-full flex flex-col gap-3"
      >
        {/* Dynamic Key Grid matching real hardware rows x cols */}
        <div
          className="grid gap-2.5"
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
                onDragOver={(event) => event.preventDefault()}
                onDrop={(event) => {
                  event.preventDefault();
                  const file = event.dataTransfer.files?.[0];
                  if (file) void ingestFile(file, "key", k.index);
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
                className={`aspect-[1/0.9] border p-2 flex flex-col items-center justify-center gap-1.5 relative overflow-hidden cursor-pointer transition-all group ${
                  isLivePressed
                    ? "border-2 border-[#35C98B] shadow-[0_0_24px_rgba(53,201,139,0.75)] scale-95"
                    : isSelected
                    ? "border-2 border-[#8B5CF6] shadow-[0_0_20px_rgba(139,92,246,0.45)] -translate-y-0.5"
                    : "border-[#272D3D] hover:border-[#394158] hover:-translate-y-0.5"
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
                        className={`font-semibold text-[#F4F5FA] truncate max-w-full leading-tight ${
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
                  ? "border-[#8B5CF6] bg-[#30244F] text-white"
                  : "border-[#272D3D] bg-[#111522] text-[#A6AFC4]"
              }`}
            >
              Whole touch strip · {activeDevice.touchStrip.pixelWidth}×{activeDevice.touchStrip.pixelHeight}
              {activePage.stripMedia ? " · image" : ""}
            </button>
            <div
              className="grid gap-2 bg-[#0B0D14] border border-[#272D3D] rounded-xl p-2"
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
                    className={`relative min-h-14 overflow-hidden px-2.5 py-1.5 rounded-lg bg-[#151925] border cursor-pointer flex items-center justify-between ${
                      zoneSelected ? "border-[#8B5CF6]" : "border-[#272D3D] hover:border-[#8B5CF6]/60"
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
                        <div className="text-[9.5px] font-bold text-[#707B94] tracking-wider">
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
                className={`px-2.5 py-1.5 rounded-xl bg-[#111522] border flex items-center gap-2.5 cursor-pointer ${
                  surface === "encoder" && surfaceIndex === enc.index
                    ? "border-[#8B5CF6]"
                    : "border-[#272D3D] hover:border-[#8B5CF6]/60"
                }`}
              >
                <div className="w-6 h-6 rounded-full bg-gradient-to-br from-[#394158] to-[#10131D] border border-dashed border-[#A6AFC4]/60 shrink-0" />
                <div className="min-w-0">
                  <div className="text-[11px] font-semibold text-[#F4F5FA] truncate">
                    {enc.label}
                  </div>
                  <div className="text-[9.5px] font-mono text-[#A6AFC4] truncate">
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

      <div className="text-[11px] text-[#707B94]">
        {activeDevice.keys.count} LCD screens · {activeDevice.keys.pixelWidth}×{activeDevice.keys.pixelHeight}px
        {activeDevice.keys.rotation ? ` · ${activeDevice.keys.rotation}°` : ""}
        {activeDevice.touchStrip
          ? ` · strip ${activeDevice.touchStrip.pixelWidth}×${activeDevice.touchStrip.pixelHeight}`
          : ""}
        . Drop a GIF onto any screen.
      </div>

      {/* Page Switcher Bar below the Deck Bezel */}
      <div className="flex items-center gap-1.5">
        <div className="flex items-center bg-[#151925] border border-[#272D3D] rounded-md h-7 px-1">
          <button
            onClick={() => {
              deckforgeApi.holdAutoSwitch().catch(() => {});
              setSelectedPageIdx((p) => Math.max(0, p - 1));
            }}
            className="px-1.5 text-[#A6AFC4] hover:text-white cursor-pointer"
          >
            <ChevronLeft size={13} />
          </button>
          <select
            value={selectedPageIdx}
            onChange={(e) => {
              deckforgeApi.holdAutoSwitch().catch(() => {});
              setSelectedPageIdx(Number(e.target.value));
            }}
            className="bg-transparent text-xs font-medium text-[#F4F5FA] px-2 outline-none cursor-pointer"
          >
            {activeProfile.pages.map((pg, idx) => (
              <option key={pg.id} value={idx} className="bg-[#151925]">
                {pg.name}
              </option>
            ))}
          </select>
          <button
            onClick={() => {
              deckforgeApi.holdAutoSwitch().catch(() => {});
              setSelectedPageIdx((p) => Math.min(activeProfile.pages.length - 1, p + 1));
            }}
            className="px-1.5 text-[#A6AFC4] hover:text-white cursor-pointer"
          >
            <ChevronRight size={13} />
          </button>
        </div>

        <button
          onClick={handleAddPage}
          title="Add New Page"
          className="w-7 h-7 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6] text-[#A6AFC4] hover:text-white flex items-center justify-center cursor-pointer"
        >
          <Plus size={13} />
        </button>
        <button
          onClick={() => setTemplatesOpen(true)}
          title="Page templates"
          aria-label="Page templates"
          className="h-7 px-2 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6] text-[#A6AFC4] hover:text-white flex items-center justify-center gap-1.5 text-[11px] font-medium cursor-pointer"
        >
          <LayoutTemplate size={13} /> Templates
        </button>
      </div>
    </div>
  );
};
