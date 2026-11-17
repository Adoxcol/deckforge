import React from "react";
import { MediaLibrary } from "./MediaLibrary";
import { ScreenFace } from "./ScreenFace";
import { AVAILABLE_ICON_NAMES, DeckIcon } from "./DeckIcon";
import { BrandIconPicker } from "./BrandIconPicker";
import { LevelMeter, liveKeyLook } from "./LiveFace";
import { MediaConfigurator } from "./MediaConfigurator";
import { ActionBinding, ObsConnection, actionNeedsParam, splitStoredBinding } from "./ActionBinding";
import { ActionTemplate } from "../data/actionCatalog";
import { Copy, Play, Plus, Trash2 } from "lucide-react";
import {
  DEFAULT_KNOB_LED_COLORS,
  type DeckFaceState,
  type DeviceDescriptor,
  type EncoderConfig,
  type KeyConfig,
  type MacroSequence,
  type MediaPreview,
  type ObsSettings,
  type ScreenMediaConfig,
  type ScreenTarget,
  type TouchZoneConfig,
} from "../types/deckforge";

const KNOB_LED_PRESETS = [
  "#35C98B",
  "#56B9F2",
  "#8B5CF6",
  "#F4B860",
  "#F16B79",
  "#EC4899",
  "#00E5FF",
  "#FFFFFF",
] as const;

interface KeyInspectorProps {
  mediaLibrary: Record<string, MediaPreview>;
  onMediaStored: (clip: MediaPreview) => void;
  activeDevice: DeviceDescriptor;
  activeEncoder: EncoderConfig | null;
  activeKey: KeyConfig;
  activeZone: TouchZoneConfig | null;
  allActionTemplates: ActionTemplate[];
  assignMedia: (target: ScreenTarget, index: number, cfg: ScreenMediaConfig | null) => void;
  clipFor: (cfg?: ScreenMediaConfig | null | undefined) => MediaPreview | null;
  clipboardKey: KeyConfig | null;
  currentMedia: () => ScreenMediaConfig | null;
  face: DeckFaceState | null;
  fileRef: React.RefObject<HTMLInputElement | null>;
  handleClearSelectedKey: () => void;
  handleTestSelectedAction: () => Promise<void>;
  hotkeyTarget: "key" | "zone" | "push" | "cw" | "ccw" | null;
  ingestFile: (file: File, target: ScreenTarget, index: number) => Promise<void>;
  inspectorTab: "Action" | "Appearance" | "Behavior";
  macros: MacroSequence[];
  obsSettings: ObsSettings;
  paramOptionsFor: (actionId: string) => { value: string; label: string; }[] | undefined;
  patchCurrentMedia: (patch: Partial<ScreenMediaConfig>) => void;
  saveObs: (settings: ObsSettings) => void;
  setActionCategoryFilter: React.Dispatch<React.SetStateAction<string>>;
  setClipboardKey: React.Dispatch<React.SetStateAction<KeyConfig | null>>;
  setCustomActions: React.Dispatch<React.SetStateAction<ActionTemplate[]>>;
  setHotkeyTarget: React.Dispatch<React.SetStateAction<"key" | "zone" | "push" | "cw" | "ccw" | null>>;
  setInspectorTab: React.Dispatch<React.SetStateAction<"Action" | "Appearance" | "Behavior">>;
  surface: "key" | "zone" | "strip" | "encoder";
  surfaceIndex: number;
  triggerToast: (text: string, kind?: "success" | "info" | "warning") => void;
  updateSelectedEncoder: (index: number, patch: Partial<EncoderConfig>) => void;
  onKnobLedChange: (knobIndex: number | "all", colorHex: string) => void;
  onResetKnobLeds: () => void;
  updateSelectedKey: (patch: Partial<KeyConfig>) => void;
  updateSelectedZone: (index: number, patch: Partial<TouchZoneConfig>) => void;
  useSampleAnimation: (target: ScreenTarget, index: number) => Promise<void>;
}

export const KeyInspector: React.FC<KeyInspectorProps> = ({
  mediaLibrary,
  onMediaStored,
  activeDevice,
  activeEncoder,
  activeKey,
  activeZone,
  allActionTemplates,
  assignMedia,
  clipFor,
  clipboardKey,
  currentMedia,
  face,
  fileRef,
  handleClearSelectedKey,
  handleTestSelectedAction,
  hotkeyTarget,
  ingestFile,
  inspectorTab,
  macros,
  obsSettings,
  paramOptionsFor,
  patchCurrentMedia,
  saveObs,
  setActionCategoryFilter,
  setClipboardKey,
  setCustomActions,
  setHotkeyTarget,
  setInspectorTab,
  surface,
  surfaceIndex,
  triggerToast,
  updateSelectedEncoder,
  onKnobLedChange,
  onResetKnobLeds,
  updateSelectedKey,
  updateSelectedZone,
  useSampleAnimation,
}) => {
  const [library, setLibrary] = React.useState<{ use: (clip: MediaPreview) => void } | null>(null);
  return (
    <aside className="bg-[var(--bg-sidebar)] border-l border-[var(--border-subtle)] p-4 flex flex-col justify-between overflow-y-auto">
      {library && <MediaLibrary clips={mediaLibrary} onStored={onMediaStored} onUse={library.use} onClose={() => setLibrary(null)} />}
      <div className="flex flex-col gap-3.5">
        <div className="flex items-center justify-between">
          <h2 className="text-[14px] font-bold text-[var(--text-primary)]">
            {surface === "zone"
              ? "Touch Zone"
              : surface === "strip"
              ? "Touch Strip"
              : surface === "encoder"
              ? "Rotary Encoder"
              : "Button Properties"}
          </h2>
          <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)]">
            {surface === "zone"
              ? `Zone #${(activeZone?.index ?? 0) + 1}`
              : surface === "strip"
              ? `${activeDevice.touchStrip?.pixelWidth ?? 0}×${activeDevice.touchStrip?.pixelHeight ?? 0}`
              : surface === "encoder"
              ? `Knob #${(activeEncoder?.index ?? 0) + 1}`
              : `Key #${activeKey.index + 1}`}
          </span>
        </div>

        {surface === "key" && (
        <div className="flex items-center gap-3">
          <div
            style={{
              borderRadius: `${activeKey.cornerRadius}px`,
              background:
                clipFor(activeKey.media)
                  ? "#0B0D14"
                  : activeKey.backgroundStyle === "Gradient"
                  ? `radial-gradient(circle at 50% 30%, ${activeKey.accentColor}55, ${activeKey.bgColor})`
                  : activeKey.bgColor,
            }}
            className="relative w-16 h-16 overflow-hidden border border-[var(--accent-primary)] flex items-center justify-center shrink-0"
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => {
              event.preventDefault();
              const file = event.dataTransfer.files?.[0];
              if (file) void ingestFile(file, "key", activeKey.index);
            }}
          >
            {clipFor(activeKey.media) ? (
              <ScreenFace
                clip={clipFor(activeKey.media)}
                config={activeKey.media}
                iconName={activeKey.iconName}
                accent={activeKey.accentColor}
                title={activeKey.title}
                showTitle={activeKey.showTitle}
                titleSize={activeKey.titleSize}
                iconSize={26}
                className="absolute inset-0"
              />
            ) : (
              <DeckIcon
                name={liveKeyLook(activeKey.actionId, activeKey.title, activeKey.iconName, activeKey.media ? null : face, activeKey.actionParam).icon}
                size={26}
                color={activeKey.accentColor}
              />
            )}
          </div>
          <div className="flex-1">
            <label className="block text-[11.5px] text-[var(--text-secondary)] mb-1">Title</label>
            <input
              value={activeKey.title}
              onChange={(e) => updateSelectedKey({ title: e.target.value })}
              className="w-full h-9 px-2.5 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none"
            />
            {(() => {
              const look = liveKeyLook(activeKey.actionId, activeKey.title, activeKey.iconName, activeKey.media ? null : face, activeKey.actionParam);
              if (look.title === activeKey.title && look.level == null) return null;
              return (
                <div className="mt-1.5 flex flex-col gap-1">
                  <div className="text-[10.5px] text-[var(--status-success)] truncate">Live · {look.title}</div>
                  {look.level != null && <LevelMeter percent={look.level} accent={activeKey.accentColor} muted={look.muted} />}
                </div>
              );
            })()}
          </div>
        </div>
        )}

        {surface !== "encoder" && (
          <MediaConfigurator
            title={
              surface === "strip"
                ? "Strip image"
                : surface === "zone"
                ? "Zone image"
                : "Key screen"
            }
            pixelLabel={
              surface === "strip" || surface === "zone"
                ? `Touch strip ${activeDevice.touchStrip?.pixelWidth ?? 0}×${activeDevice.touchStrip?.pixelHeight ?? 0}px`
                : `Key ${activeDevice.keys.pixelWidth}×${activeDevice.keys.pixelHeight}px · ${activeDevice.keys.rotation}°`
            }
            config={currentMedia()}
            clip={clipFor(currentMedia())}
            onBrowse={() => {
              const target: ScreenTarget = surface === "zone" || surface === "strip" ? surface : "key";
              const index = target === "key" ? activeKey.index : surfaceIndex;
              setLibrary({ use: clip => assignMedia(target, index, { mediaId: clip.id, fit: "cover", loopPlayback: true, speed: 1 }) });
            }}
            onUpload={() => fileRef.current?.click()}
            onSample={() => {
              const target: ScreenTarget = surface === "zone" || surface === "strip" ? surface : "key";
              const index = target === "key" ? activeKey.index : surfaceIndex;
              void useSampleAnimation(target, index);
            }}
            onChange={patchCurrentMedia}
            onClear={() => {
              const target: ScreenTarget = surface === "zone" || surface === "strip" ? surface : "key";
              const index = target === "key" ? activeKey.index : surfaceIndex;
              assignMedia(target, index, null);
            }}
          />
        )}

        {surface === "zone" && activeZone && (
          <div className="flex flex-col gap-2">
            <label className="text-[11.5px] text-[var(--text-secondary)]">Zone label</label>
            <input
              value={activeZone.label}
              onChange={(e) => updateSelectedZone(activeZone.index, { label: e.target.value })}
              className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none"
            />
            <label className="text-[11.5px] text-[var(--text-secondary)]">Value text</label>
            <input
              value={activeZone.valueText}
              onChange={(e) => updateSelectedZone(activeZone.index, { valueText: e.target.value })}
              className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none"
            />
            <ActionBinding
              label="Tap"
              actionId={activeZone.tapActionId}
              actionParam={activeZone.tapActionParam ?? ""}
              choices={allActionTemplates}
              recording={hotkeyTarget === "zone"}
              onRecord={() => setHotkeyTarget("zone")}
              onChange={(actionId, actionParam) => {
                const tpl = allActionTemplates.find((item) => item.id === actionId);
                updateSelectedZone(activeZone.index, {
                  tapActionId: actionId,
                  tapActionParam: actionParam,
                  ...(tpl && actionId !== activeZone.tapActionId
                    ? { label: tpl.title, valueText: tpl.subtitle, accentColor: tpl.accentColor }
                    : {}),
                });
              }}
              paramOptions={paramOptionsFor(activeZone.tapActionId)}
            />
            {activeZone.tapActionId.startsWith("obs.") && (
              <ObsConnection obsSettings={obsSettings} onSave={saveObs} />
            )}
          </div>
        )}

        {surface === "strip" && (
          <p className="text-[11px] leading-relaxed text-[var(--text-secondary)]">
            A full-strip image covers all zones. Leave it empty to keep the zone cards, and upload a GIF on an individual zone if only that part should animate.
          </p>
        )}

        {surface === "encoder" && activeEncoder && (() => {
          const ledColor = (
            activeEncoder.ledColor ||
            DEFAULT_KNOB_LED_COLORS[activeEncoder.index] ||
            "#35C98B"
          ).toUpperCase();
          const ledOff = ledColor === "#000000";
          return (
          <div className="flex flex-col gap-2">
            <label className="text-[11.5px] text-[var(--text-secondary)]">Label</label>
            <input
              value={activeEncoder.label}
              onChange={(e) => updateSelectedEncoder(activeEncoder.index, { label: e.target.value })}
              className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none"
            />
            <div className="p-2.5 rounded-lg bg-[var(--bg-panel)] border border-[var(--border-subtle)] flex flex-col gap-2">
              <div className="flex items-center justify-between">
                <span className="text-[11.5px] font-semibold text-[var(--text-primary)]">
                  Knob LED Light
                </span>
                <span className="flex items-center gap-1.5 text-[10.5px] font-mono text-[var(--text-secondary)]">
                  <span
                    style={{
                      backgroundColor: ledOff ? "transparent" : ledColor,
                      borderColor: ledOff ? "var(--border-strong)" : ledColor,
                    }}
                    className="w-2.5 h-2.5 rounded-full border"
                  />
                  {ledOff ? "OFF" : ledColor}
                </span>
              </div>
              <div className="flex items-center flex-wrap gap-1.5">
                {KNOB_LED_PRESETS.map((hex) => {
                  const selected = ledColor === hex;
                  return (
                    <button
                      key={hex}
                      type="button"
                      title={hex}
                      aria-label={`Set knob LED to ${hex}`}
                      onClick={() => onKnobLedChange(activeEncoder.index, hex)}
                      style={{ backgroundColor: hex }}
                      className={`w-6 h-6 rounded-md cursor-pointer transition-transform ${
                        selected
                          ? "ring-2 ring-white scale-110"
                          : "border border-white/15 hover:scale-105"
                      }`}
                    />
                  );
                })}
                <button
                  type="button"
                  onClick={() => onKnobLedChange(activeEncoder.index, "#000000")}
                  className={`h-6 px-2 rounded-md text-[10.5px] font-medium border cursor-pointer ${
                    ledOff
                      ? "bg-[var(--accent-soft)] border-[var(--accent-primary)] text-white"
                      : "bg-[var(--bg-input)] border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                  }`}
                >
                  Off
                </button>
                <label
                  title="Custom RGB color"
                  className="h-6 px-1.5 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] flex items-center gap-1 cursor-pointer hover:border-[var(--accent-primary)]"
                >
                  <input
                    type="color"
                    aria-label="Custom knob LED color"
                    value={ledOff ? "#000000" : ledColor}
                    onChange={(e) => onKnobLedChange(activeEncoder.index, e.target.value.toUpperCase())}
                    className="w-4 h-4 bg-transparent border-0 p-0 cursor-pointer"
                  />
                  <span className="text-[10.5px] text-[var(--text-secondary)]">Pick</span>
                </label>
              </div>
              <div className="flex items-center gap-1.5 pt-0.5">
                <button
                  type="button"
                  onClick={() => onKnobLedChange("all", ledColor)}
                  className="flex-1 h-7 px-2 rounded bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--accent-primary)] text-[11px] font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer"
                >
                  Same color on all 4 knobs
                </button>
                <button
                  type="button"
                  onClick={onResetKnobLeds}
                  className="h-7 px-2.5 rounded bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--status-warning)] text-[11px] font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer"
                >
                  Reset LEDs
                </button>
              </div>
            </div>
            {(["cw", "ccw", "push"] as const).map((field) => {
              const stored = field === "cw" ? activeEncoder.cwAction : field === "ccw" ? activeEncoder.ccwAction : activeEncoder.pushAction;
              const param = field === "cw" ? activeEncoder.cwParam : field === "ccw" ? activeEncoder.ccwParam : activeEncoder.pushParam;
              const split = splitStoredBinding(stored, param, allActionTemplates.map((item) => item.id));
              return (
                <ActionBinding
                  key={field}
                  label={field === "cw" ? "Turn right" : field === "ccw" ? "Turn left" : "Press"}
                  actionId={split.actionId}
                  actionParam={split.actionParam}
                  choices={allActionTemplates}
                  recording={hotkeyTarget === field}
                  onRecord={() => setHotkeyTarget(field)}
                  onChange={(actionId, actionParam) => {
                    const patch =
                      field === "cw"
                        ? { cwAction: actionId, cwParam: actionParam }
                        : field === "ccw"
                        ? { ccwAction: actionId, ccwParam: actionParam }
                        : { pushAction: actionId, pushParam: actionParam };
                    updateSelectedEncoder(activeEncoder.index, patch);
                  }}
                  paramOptions={paramOptionsFor(split.actionId)}
                />
              );
            })}
            {[activeEncoder.cwAction, activeEncoder.ccwAction, activeEncoder.pushAction].some((action) =>
              action.startsWith("obs.")
            ) && <ObsConnection obsSettings={obsSettings} onSave={saveObs} />}
          </div>
          );
        })()}

        {surface === "key" && (
        <>
        <ActionBinding
          label="Action"
          actionId={activeKey.actionId}
          actionParam={activeKey.actionParam}
          choices={allActionTemplates}
          recording={hotkeyTarget === "key"}
          onRecord={() => setHotkeyTarget("key")}
          onChange={(actionId, actionParam) => {
            const tpl = allActionTemplates.find((item) => item.id === actionId);
            const macro = actionId === "macro.run" ? macros.find((item) => item.id === actionParam) : undefined;
            updateSelectedKey({
              actionId,
              actionParam,
              ...(macro
                ? {
                    title: macro.name,
                    subtitle: "MACRO",
                    actionLabel: `Macro: ${macro.name}`,
                    actionCategory: "Macro",
                    iconName: "Zap",
                    accentColor: "#8B5CF6",
                    bgColor: "#231B38",
                  }
                : tpl && actionId !== activeKey.actionId
                ? {
                    actionLabel: tpl.actionLabel,
                    actionCategory: tpl.category,
                    iconName: tpl.iconName,
                    accentColor: tpl.accentColor,
                    bgColor: tpl.bgColor,
                    subtitle: actionNeedsParam(actionId) && !actionId.startsWith("hotkey.") ? actionParam : tpl.subtitle,
                  }
                : {}),
            });
          }}
          paramOptions={paramOptionsFor(activeKey.actionId)}
        />
        {activeKey.actionId.startsWith("obs.") && (
          <ObsConnection obsSettings={obsSettings} onSave={saveObs} />
        )}

        {/* Segmented Tabs: General (Action) | Icon (Appearance) | Style (Behavior) */}
        <div role="tablist" aria-label="Button properties" className="grid grid-cols-3 bg-[var(--bg-input)] border border-[var(--border-subtle)] p-0.5 rounded-md">
          {(["Action", "Appearance", "Behavior"] as const).map((tab) => (
            <button
              key={tab}
              type="button"
              role="tab"
              aria-selected={inspectorTab === tab}
              onClick={() => setInspectorTab(tab)}
              className={`h-7 rounded text-[11.5px] font-medium cursor-pointer transition-all ${
                inspectorTab === tab
                  ? "bg-[var(--accent-soft)] text-[var(--text-primary)] font-semibold shadow-xs"
                  : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
              }`}
            >
              {tab === "Action" ? "General" : tab === "Appearance" ? "Icon" : "Style"}
            </button>
          ))}
        </div>

        {inspectorTab === "Action" && (
          <div className="flex flex-col gap-3">
            {/* Show Title Toggle */}
            <div className="flex items-center justify-between">
              <span className="text-xs text-[var(--text-secondary)]">Show Title</span>
              <button
                type="button"
                role="switch"
                aria-checked={activeKey.showTitle}
                aria-label="Show Title"
                onClick={() => updateSelectedKey({ showTitle: !activeKey.showTitle })}
                className={`w-10 h-5 rounded-full p-0.5 transition-colors cursor-pointer ${
                  activeKey.showTitle ? "bg-[var(--accent-primary)]" : "bg-[var(--border-strong)]"
                }`}
              >
                <div
                  className={`w-4 h-4 rounded-full bg-white shadow-xs transition-transform ${
                    activeKey.showTitle ? "translate-x-5" : "translate-x-0"
                  }`}
                />
              </button>
            </div>

            {/* Title Size */}
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs text-[var(--text-secondary)]">Title Size</span>
              <select
                aria-label="Title Size"
                value={activeKey.titleSize}
                onChange={(e) =>
                  updateSelectedKey({
                    titleSize: e.target.value as "Small" | "Medium" | "Large",
                  })
                }
                className="w-40 h-8 px-2 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none cursor-pointer"
              >
                <option value="Small">Small</option>
                <option value="Medium">Medium</option>
                <option value="Large">Large</option>
              </select>
            </div>

            {/* Background */}
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs text-[var(--text-secondary)]">Background</span>
              <select
                aria-label="Background style"
                value={activeKey.backgroundStyle}
                onChange={(e) =>
                  updateSelectedKey({
                    backgroundStyle: e.target.value as "Gradient" | "Solid" | "Image",
                  })
                }
                className="w-40 h-8 px-2 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none cursor-pointer"
              >
                <option value="Gradient">Gradient</option>
                <option value="Solid">Solid Surface</option>
                <option value="Image">Image / GIF</option>
              </select>
            </div>

            {/* Gradient Color Swatches */}
            <div className="flex items-center justify-end gap-2">
              {[
                ["#F16B79", "#2A1521", "Rose"],
                ["#8B5CF6", "#231B38", "Violet"],
                ["#56B9F2", "#132235", "Sky"],
                ["#35C98B", "#112822", "Mint"],
                ["#F4B860", "#2A2215", "Gold"],
              ].map(([acc, bg, label]) => (
                <button
                  key={acc}
                  type="button"
                  title={label}
                  aria-label={`Use ${label} color swatch`}
                  aria-pressed={activeKey.accentColor === acc}
                  onClick={() => updateSelectedKey({ accentColor: acc, bgColor: bg })}
                  style={{ backgroundColor: acc }}
                  className={`w-6 h-6 rounded-md border cursor-pointer transition-transform ${
                    activeKey.accentColor === acc
                      ? "border-2 border-white scale-110 shadow-sm"
                      : "border-white/15 hover:scale-105"
                  }`}
                />
              ))}
            </div>

            {/* Rounded Corners Slider */}
            <div>
              <div className="flex items-center justify-between text-xs text-[var(--text-secondary)] mb-1.5">
                <span>Rounded Corners</span>
                <span className="font-mono text-[var(--text-primary)]">{activeKey.cornerRadius} px</span>
              </div>
              <input
                aria-label="Rounded Corners"
                type="range"
                min={2}
                max={18}
                value={activeKey.cornerRadius}
                onChange={(e) =>
                  updateSelectedKey({ cornerRadius: Number(e.target.value) })
                }
                className="w-full accent-[var(--accent-primary)] cursor-pointer"
              />
            </div>
          </div>
        )}

        {inspectorTab === "Appearance" && (
          <div className="flex flex-col gap-2">
            <span className="text-xs text-[var(--text-secondary)]">Choose Key Icon (Lucide Vector)</span>
            <div className="grid grid-cols-5 gap-1.5 max-h-48 overflow-y-auto p-1.5 bg-[var(--bg-input)] border border-[var(--border-subtle)] rounded-md">
              {AVAILABLE_ICON_NAMES.map((ico) => (
                <button
                  key={ico}
                  type="button"
                  onClick={() => updateSelectedKey({ iconName: ico })}
                  title={ico}
                  aria-label={`Select ${ico} icon`}
                  aria-pressed={activeKey.iconName === ico}
                  className={`h-9 rounded-md flex items-center justify-center cursor-pointer transition-colors ${
                    activeKey.iconName === ico
                      ? "bg-[var(--accent-soft)] border border-[var(--accent-primary)] text-white"
                      : "border border-transparent text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)]"
                  }`}
                >
                  <DeckIcon name={ico} size={16} />
                </button>
              ))}
            </div>
            <BrandIconPicker
              onPick={(file, icon) => {
                void ingestFile(file, "key", activeKey.index);
                triggerToast(`Set the ${icon.title} logo on key #${activeKey.index + 1}`, "success");
              }}
            />
          </div>
        )}

        {inspectorTab === "Behavior" && (
          <div className="flex flex-col gap-3">
            <div>
              <label className="block text-xs text-[var(--text-secondary)] mb-1">
                Command / Parameter
              </label>
              <input
                value={activeKey.actionParam}
                onChange={(e) => updateSelectedKey({ actionParam: e.target.value })}
                className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs font-mono text-[var(--text-primary)] outline-none"
              />
            </div>
            <button
              type="button"
              onClick={() => {
                const newTpl: ActionTemplate = {
                  id: `${activeKey.actionId}.user.${Date.now()}`,
                  title: activeKey.title || "Custom Action",
                  subtitle: activeKey.subtitle || "CUSTOM",
                  category: "Custom",
                  iconName: activeKey.iconName,
                  actionLabel: `Custom: ${activeKey.title} (${activeKey.actionParam})`,
                  defaultParam: activeKey.actionParam,
                  accentColor: activeKey.accentColor,
                  bgColor: activeKey.bgColor,
                  description: `Saved custom preset → ${activeKey.actionParam}`,
                };
                setCustomActions((prev) => [newTpl, ...prev]);
                setActionCategoryFilter("Custom");
                triggerToast(`Saved "${activeKey.title}" to ★ Custom Actions catalog!`, "success");
              }}
              className="w-full h-8 rounded-md bg-[var(--accent-soft)] border border-[var(--accent-primary)]/60 hover:bg-[var(--accent-primary)] text-xs font-semibold text-[var(--accent-text)] hover:text-white flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
            >
              <Plus size={13} />
              <span>Save to ★ Custom Actions Library</span>
            </button>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => {
                  setClipboardKey(JSON.parse(JSON.stringify(activeKey)));
                  triggerToast(`Copied Key #${activeKey.index + 1}`, "info");
                }}
                className="flex-1 h-8 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-xs font-medium text-[var(--text-primary)] flex items-center justify-center gap-1.5 cursor-pointer"
              >
                <Copy size={12} />
                <span>Copy</span>
              </button>
              <button
                type="button"
                disabled={!clipboardKey}
                onClick={() => {
                  if (!clipboardKey) return;
                  updateSelectedKey({
                    ...clipboardKey,
                    index: activeKey.index,
                  });
                  triggerToast(`Pasted to Key #${activeKey.index + 1}`, "success");
                }}
                className="flex-1 h-8 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] disabled:opacity-40 disabled:hover:bg-[var(--bg-panel)] disabled:hover:border-[var(--border-subtle)] disabled:cursor-default text-xs font-medium text-[var(--text-primary)] flex items-center justify-center gap-1.5 cursor-pointer"
              >
                <span>Paste</span>
              </button>
            </div>
          </div>
        )}
        </>
        )}
      </div>

      {surface === "key" && (
      <div className="flex flex-col gap-2 pt-3 border-t border-[var(--border-subtle)]">
        <button
          type="button"
          onClick={handleTestSelectedAction}
          className="w-full h-9 rounded-md bg-[var(--bg-panel)] border border-[var(--border-strong)] hover:bg-[var(--bg-elevated)] hover:border-[var(--accent-primary)] text-xs font-semibold text-[var(--text-primary)] flex items-center justify-center gap-2 cursor-pointer transition-colors"
        >
          <Play size={13} />
          <span>Test Action</span>
        </button>
        <button
          type="button"
          onClick={handleClearSelectedKey}
          title="Clear this key in the editor. Apply to Device to update the physical key."
          className="w-full h-8 rounded-md bg-transparent border border-[var(--border-subtle)] hover:border-[var(--status-error)]/60 hover:bg-[var(--status-error)]/10 text-xs text-[var(--text-secondary)] hover:text-[var(--status-error)] flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
        >
          <Trash2 size={12} />
          <span>Clear Key in Editor</span>
        </button>
      </div>
      )}
    </aside>
  );
};
