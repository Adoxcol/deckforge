import React, { useState } from "react";
import { Eye, EyeOff, Film, ImagePlus, Trash2, Type, Upload } from "lucide-react";
import type { KeyConfig, MediaPreview, ScreenMediaConfig } from "../types/deckforge";
import { AVAILABLE_ICON_NAMES, DeckIcon } from "./DeckIcon";
import { ScreenFace } from "./ScreenFace";

type LayerId = "picture" | "title" | "icon" | "background";

const LOOKS: { name: string; accent: string; bg: string }[] = [
  { name: "Violet", accent: "#8B5CF6", bg: "#231B38" },
  { name: "Sky", accent: "#56B9F2", bg: "#132235" },
  { name: "Mint", accent: "#35C98B", bg: "#112822" },
  { name: "Gold", accent: "#F4B860", bg: "#2A2215" },
  { name: "Rose", accent: "#F16B79", bg: "#2A1521" },
];

function KeyCap({
  keyConfig,
  clip,
  pixelWidth,
  pixelHeight,
  scale,
}: {
  keyConfig: KeyConfig;
  clip: MediaPreview | null;
  pixelWidth: number;
  pixelHeight: number;
  scale: number;
}) {
  const background = clip
    ? "#0B0D14"
    : keyConfig.backgroundStyle === "Gradient"
      ? `radial-gradient(circle at 50% 28%, ${keyConfig.accentColor}55, ${keyConfig.bgColor})`
      : keyConfig.bgColor;
  return (
    <div style={{ width: pixelWidth * scale, height: pixelHeight * scale }}>
      <div
        style={{
          width: pixelWidth,
          height: pixelHeight,
          transform: `scale(${scale})`,
          transformOrigin: "top left",
          borderRadius: keyConfig.cornerRadius,
          background,
        }}
        className="relative overflow-hidden border border-[#394158]"
      >
        {clip ? (
          <ScreenFace
            clip={clip}
            config={keyConfig.media}
            iconName={keyConfig.iconName}
            accent={keyConfig.accentColor}
            title={keyConfig.title}
            showTitle={keyConfig.showTitle}
            titleSize={keyConfig.titleSize}
            iconSize={28}
            className="absolute inset-0"
          />
        ) : (
          <div className="flex h-full w-full flex-col items-center justify-center gap-1">
            <span
              className="flex items-center justify-center rounded-lg"
              style={{
                width: 36,
                height: 36,
                backgroundColor: `${keyConfig.accentColor}22`,
                border: `1px solid ${keyConfig.accentColor}66`,
              }}
            >
              <DeckIcon name={keyConfig.iconName} size={20} color={keyConfig.accentColor} />
            </span>
            {keyConfig.showTitle && (
              <span
                className={`max-w-[100px] truncate px-1 text-center font-semibold text-[#F4F5FA] ${
                  keyConfig.titleSize === "Small" ? "text-[10px]" : keyConfig.titleSize === "Large" ? "text-[14px]" : "text-[12px]"
                }`}
              >
                {keyConfig.title}
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export const IconStudio: React.FC<{
  keys: KeyConfig[];
  columns: number;
  selectedIndex: number;
  clip: MediaPreview | null;
  pixelWidth: number;
  pixelHeight: number;
  onSelectKey: (index: number) => void;
  onChange: (patch: Partial<KeyConfig>) => void;
  onUpload: () => void;
  onSample: () => void;
  onClearMedia: () => void;
  onMediaPatch: (patch: Partial<ScreenMediaConfig>) => void;
  onPaintPage: () => void;
  onApply: () => void;
  onDropFile: (file: File) => void;
}> = ({
  keys,
  columns,
  selectedIndex,
  clip,
  pixelWidth,
  pixelHeight,
  onSelectKey,
  onChange,
  onUpload,
  onSample,
  onClearMedia,
  onMediaPatch,
  onPaintPage,
  onApply,
  onDropFile,
}) => {
  const [layer, setLayer] = useState<LayerId>("icon");
  const face = keys.find((item) => item.index === selectedIndex) ?? keys[0];
  if (!face) return null;
  const delays = clip ? (clip.delaysMs.length === clip.frames.length ? clip.delaysMs : clip.frames.map(() => clip.frameDelayMs || 100)) : [];
  const timing = delays.reduce((sum, value) => sum + value, 0);

  return (
    <div className="flex-1 grid grid-cols-[280px_1fr_300px] overflow-hidden">
      <aside className="bg-[#10131D] border-r border-[#272D3D] p-4 flex flex-col gap-3 overflow-y-auto">
        <div className="text-[11px] font-semibold text-[#A6AFC4]">This page</div>
        <div className="grid gap-2" style={{ gridTemplateColumns: `repeat(${Math.max(1, columns)}, 44px)` }}>
          {keys.map((item) => {
            const selected = item.index === face.index;
            return (
              <button
                key={item.index}
                type="button"
                title={item.title}
                aria-label={item.title || `Key ${item.index + 1}`}
                onClick={() => onSelectKey(item.index)}
                className={`relative flex h-11 w-11 items-center justify-center rounded-lg border cursor-pointer ${
                  selected ? "border-[#8B5CF6] shadow-[0_0_0_1px_#8B5CF6]" : "border-[#272D3D] hover:border-[#394158]"
                }`}
                style={{
                  background: item.media
                    ? "#0B0D14"
                    : item.backgroundStyle === "Gradient"
                      ? `radial-gradient(circle at 50% 28%, ${item.accentColor}55, ${item.bgColor})`
                      : item.bgColor,
                }}
              >
                <DeckIcon name={item.iconName} size={16} color={item.accentColor} />
                {item.media && <span className="absolute right-1 top-1 h-1.5 w-1.5 rounded-full bg-[#C084FC]" />}
              </button>
            );
          })}
        </div>
        <div className="text-xs font-semibold text-[#F4F5FA]">{face.title || `Key ${face.index + 1}`}</div>
        <p className="text-[10.5px] leading-relaxed text-[#707B94]">
          Pick a key. The face you build here is the picture that key sends to the deck.
        </p>
      </aside>

      <main
        className="overflow-y-auto p-5 flex flex-col items-center gap-5"
        style={{
          backgroundImage: "radial-gradient(rgba(255,255,255,0.045) 1px, transparent 1px)",
          backgroundSize: "18px 18px",
          backgroundColor: "#0B0D14",
        }}
        onDragOver={(event) => event.preventDefault()}
        onDrop={(event) => {
          event.preventDefault();
          const file = event.dataTransfer.files?.[0];
          if (file) onDropFile(file);
        }}
      >
        <div className="flex items-end gap-3">
          <KeyCap keyConfig={face} clip={clip} pixelWidth={pixelWidth} pixelHeight={pixelHeight} scale={2.4} />
          <div className="pb-2 text-[11px] font-mono text-[#A6AFC4]">
            {pixelWidth}×{pixelHeight}
            <div className="text-[#707B94]">key {face.index + 1}</div>
          </div>
        </div>

        {clip && clip.frames.length > 0 && (
          <div className="w-full max-w-[640px] rounded-2xl border border-[#272D3D] bg-[#10131D]/90 p-3">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[11px] font-semibold text-[#A6AFC4]">Frames</span>
              <span className="text-[11px] font-mono text-[#F4F5FA]">
                {clip.frames.length} · {timing}ms
              </span>
            </div>
            <div className="flex h-14 items-end gap-1">
              {clip.frames.map((src, index) => (
                <div key={`${clip.id}-${index}`} className="flex h-full min-w-0 flex-col justify-end" style={{ flex: delays[index] || 1 }}>
                  <img src={src} alt="" className="h-9 w-full rounded-md object-cover border border-[#272D3D]" />
                  <span className="mt-1 text-center text-[9px] font-mono text-[#707B94]">{delays[index]}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="w-full max-w-[640px] relative pl-8 flex flex-col">
          <div className="absolute left-[15px] top-3 bottom-3 w-px bg-gradient-to-b from-[#C084FC] via-[#56B9F2] to-[#35C98B]" />
          {(
            [
              { id: "picture" as const, title: "Picture", detail: clip ? clip.fileName : "No image yet", color: "#C084FC" },
              { id: "title" as const, title: "Title", detail: face.showTitle ? face.title || "Untitled" : "Hidden", color: "#F4F5FA" },
              { id: "icon" as const, title: "Icon", detail: face.iconName, color: face.accentColor },
              { id: "background" as const, title: "Background", detail: face.backgroundStyle, color: face.bgColor },
            ]
          ).map((item) => (
            <button
              key={item.id}
              type="button"
              onClick={() => setLayer(item.id)}
              className={`relative mb-2 w-full rounded-2xl border p-3 text-left cursor-pointer ${
                layer === item.id ? "border-[#8B5CF6] bg-[#1C2130]" : "border-[#272D3D] bg-[#151925]"
              }`}
            >
              <span className="absolute -left-8 top-4 h-4 w-4 rounded-full border-2 border-[#0B0D14]" style={{ background: item.color }} />
              <div className="text-sm font-semibold text-[#F4F5FA]">{item.title}</div>
              <div className="text-[11px] font-mono text-[#A6AFC4] truncate">{item.detail}</div>
            </button>
          ))}
        </div>
      </main>

      <aside className="bg-[#10131D] border-l border-[#272D3D] p-4 flex flex-col gap-3 overflow-y-auto">
        <div className="text-sm font-bold text-[#F4F5FA]">
          {layer === "picture" ? "Picture" : layer === "title" ? "Title" : layer === "icon" ? "Icon" : "Background"}
        </div>

        {layer === "picture" && (
          <>
            <button
              type="button"
              onClick={onUpload}
              className="h-9 rounded-lg bg-[#30244F] border border-[#8B5CF6] text-xs font-semibold text-white flex items-center justify-center gap-1.5 cursor-pointer"
            >
              <Upload size={14} />
              Upload image or GIF
            </button>
            <button
              type="button"
              onClick={onSample}
              className="h-8 rounded-lg border border-[#272D3D] text-xs text-[#A6AFC4] flex items-center justify-center gap-1.5 cursor-pointer"
            >
              <Film size={13} />
              Sample pulse
            </button>
            {clip && face.media && (
              <>
                <div className="grid grid-cols-3 gap-1">
                  {(["cover", "contain", "stretch"] as const).map((fit) => (
                    <button
                      key={fit}
                      type="button"
                      onClick={() => onMediaPatch({ fit })}
                      className={`h-7 rounded text-[11px] capitalize cursor-pointer ${
                        face.media?.fit === fit ? "bg-[#30244F] text-white font-semibold" : "bg-[#151925] text-[#A6AFC4]"
                      }`}
                    >
                      {fit}
                    </button>
                  ))}
                </div>
                <label className="text-[11px] text-[#A6AFC4]">Speed {face.media.speed.toFixed(2)}×</label>
                <input
                  type="range"
                  min={0.25}
                  max={3}
                  step={0.25}
                  value={face.media.speed}
                  onChange={(event) => onMediaPatch({ speed: Number(event.target.value) })}
                  className="accent-[#8B5CF6]"
                />
                <button
                  type="button"
                  onClick={() => onMediaPatch({ loopPlayback: !face.media?.loopPlayback })}
                  className="h-8 rounded-lg border border-[#272D3D] text-xs text-[#F4F5FA] cursor-pointer"
                >
                  {face.media.loopPlayback ? "Looping" : "Play once"}
                </button>
                <button
                  type="button"
                  onClick={onClearMedia}
                  className="h-8 rounded-lg border border-[#3A2430] text-xs text-[#F16B79] flex items-center justify-center gap-1.5 cursor-pointer"
                >
                  <Trash2 size={13} />
                  Remove picture
                </button>
              </>
            )}
            {!clip && <p className="text-[11px] leading-relaxed text-[#707B94]">Drop a PNG, JPEG, WebP, or GIF on the canvas. A GIF plays on the key after you send the page.</p>}
          </>
        )}

        {layer === "title" && (
          <>
            <input
              value={face.title}
              onChange={(event) => onChange({ title: event.target.value })}
              className="h-9 px-2.5 rounded-lg bg-[#0B0D14] border border-[#272D3D] text-sm text-[#F4F5FA] outline-none"
            />
            <button
              type="button"
              onClick={() => onChange({ showTitle: !face.showTitle })}
              className="h-8 rounded-lg border border-[#272D3D] text-xs text-[#F4F5FA] flex items-center justify-center gap-1.5 cursor-pointer"
            >
              {face.showTitle ? <Eye size={13} /> : <EyeOff size={13} />}
              {face.showTitle ? "Title is on the key" : "Title is hidden"}
            </button>
            <div className="grid grid-cols-3 gap-1">
              {(["Small", "Medium", "Large"] as const).map((size) => (
                <button
                  key={size}
                  type="button"
                  onClick={() => onChange({ titleSize: size })}
                  className={`h-7 rounded text-[11px] cursor-pointer ${
                    face.titleSize === size ? "bg-[#30244F] text-white font-semibold" : "bg-[#151925] text-[#A6AFC4]"
                  }`}
                >
                  {size}
                </button>
              ))}
            </div>
            <p className="text-[11px] text-[#707B94] flex items-center gap-1">
              <Type size={12} /> Two short words render largest on the deck.
            </p>
          </>
        )}

        {layer === "icon" && (
          <div className="grid grid-cols-5 gap-1.5">
            {AVAILABLE_ICON_NAMES.map((name) => (
              <button
                key={name}
                type="button"
                title={name}
                onClick={() => onChange({ iconName: name })}
                className={`h-9 rounded-lg flex items-center justify-center cursor-pointer ${
                  face.iconName === name ? "bg-[#30244F] border border-[#8B5CF6]" : "bg-[#151925] border border-[#272D3D]"
                }`}
              >
                <DeckIcon name={name} size={16} color={face.accentColor} />
              </button>
            ))}
          </div>
        )}

        {layer === "background" && (
          <>
            <div className="grid grid-cols-5 gap-1.5">
              {LOOKS.map((look) => (
                <button
                  key={look.name}
                  type="button"
                  title={look.name}
                  onClick={() => onChange({ accentColor: look.accent, bgColor: look.bg, backgroundStyle: "Gradient" })}
                  className={`h-9 rounded-lg border cursor-pointer ${face.accentColor === look.accent ? "border-white" : "border-transparent"}`}
                  style={{ background: `linear-gradient(160deg, ${look.accent}, ${look.bg})` }}
                />
              ))}
            </div>
            <label className="text-[11px] text-[#A6AFC4]">Accent</label>
            <input
              type="color"
              value={face.accentColor}
              onChange={(event) => onChange({ accentColor: event.target.value })}
              className="h-8 w-full bg-transparent cursor-pointer"
            />
            <label className="text-[11px] text-[#A6AFC4]">Surface</label>
            <input
              type="color"
              value={face.bgColor}
              onChange={(event) => onChange({ bgColor: event.target.value })}
              className="h-8 w-full bg-transparent cursor-pointer"
            />
            <div className="grid grid-cols-2 gap-1">
              {(["Gradient", "Solid"] as const).map((style) => (
                <button
                  key={style}
                  type="button"
                  onClick={() => onChange({ backgroundStyle: style })}
                  className={`h-7 rounded text-[11px] cursor-pointer ${
                    face.backgroundStyle === style ? "bg-[#30244F] text-white font-semibold" : "bg-[#151925] text-[#A6AFC4]"
                  }`}
                >
                  {style}
                </button>
              ))}
            </div>
            <label className="text-[11px] text-[#A6AFC4]">Corners {face.cornerRadius}px</label>
            <input
              type="range"
              min={2}
              max={18}
              value={face.cornerRadius}
              onChange={(event) => onChange({ cornerRadius: Number(event.target.value) })}
              className="accent-[#8B5CF6]"
            />
          </>
        )}

        <button
          type="button"
          onClick={onPaintPage}
          className="mt-auto h-8 rounded-lg border border-[#272D3D] text-[11px] text-[#A6AFC4] cursor-pointer"
        >
          Paint this color on every key
        </button>
        <button
          type="button"
          onClick={onApply}
          className="h-10 rounded-lg bg-[#8B5CF6] text-xs font-semibold text-white flex items-center justify-center gap-1.5 cursor-pointer"
        >
          <ImagePlus size={14} />
          Send this page to the deck
        </button>
      </aside>
    </div>
  );
};
