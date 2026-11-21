import React, { useState } from "react";
import { Eye, EyeOff, Film, ImagePlus, Trash2, Type, Upload } from "lucide-react";
import type { KeyConfig, MediaPreview, ScreenMediaConfig } from "../types/deckforge";
import { AVAILABLE_ICON_NAMES, DeckIcon } from "./DeckIcon";
import { ScreenFace } from "./ScreenFace";
import { BrandIconPicker } from "./BrandIconPicker";

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
        className="relative overflow-hidden border border-[var(--border-strong)]"
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
                className={`max-w-[100px] truncate px-1 text-center font-semibold text-[var(--text-primary)] ${
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
  onBrandPick: (file: File) => void;
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
  onBrandPick,
}) => {
  const [layer, setLayer] = useState<LayerId>("icon");
  const face = keys.find((item) => item.index === selectedIndex) ?? keys[0];
  if (!face) return null;
  const delays = clip ? (clip.delaysMs.length === clip.frames.length ? clip.delaysMs : clip.frames.map(() => clip.frameDelayMs || 100)) : [];
  const timing = delays.reduce((sum, value) => sum + value, 0);

  return (
    <div className="flex-1 min-h-0 grid grid-cols-[288px_1fr_300px] overflow-hidden animate-screen-in">
      <aside className="min-h-0 bg-[var(--bg-sidebar)] border-r border-[var(--border-subtle)] p-4 flex flex-col gap-3 overflow-y-auto">
        <div className="text-[11px] font-semibold text-[var(--text-secondary)]">This page</div>
        <div className="grid gap-2" style={{ gridTemplateColumns: `repeat(${Math.max(1, Math.min(columns, 5))}, minmax(0, 1fr))` }}>
          {keys.map((item) => {
            const selected = item.index === face.index;
            return (
              <button
                key={item.index}
                type="button"
                title={item.title}
                aria-label={item.title || `Key ${item.index + 1}`}
                aria-pressed={selected}
                onClick={() => onSelectKey(item.index)}
                className={`relative flex aspect-square w-full items-center justify-center rounded-lg border cursor-pointer transition-all hover:brightness-110 ${
                  selected ? "border-[var(--accent-primary)] ring-1 ring-[var(--accent-primary)] shadow-sm" : "border-[var(--border-subtle)] hover:border-[var(--border-strong)]"
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
                {item.media && <span className="absolute right-1 top-1 h-1.5 w-1.5 rounded-full bg-[var(--accent-text)]" />}
              </button>
            );
          })}
        </div>
        <div className="text-xs font-semibold text-[var(--text-primary)]">{face.title || `Key ${face.index + 1}`}</div>
        <p className="text-[10.5px] leading-relaxed text-[var(--text-muted)]">
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
          <div className="pb-2 text-[11px] font-mono text-[var(--text-secondary)]">
            {pixelWidth}×{pixelHeight}
            <div className="text-[var(--text-muted)]">key {face.index + 1}</div>
          </div>
        </div>

        {clip && clip.frames.length > 0 && (
          <div className="w-full max-w-[640px] rounded-2xl border border-[var(--border-subtle)] bg-[var(--bg-sidebar)]/90 p-3">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[11px] font-semibold text-[var(--text-secondary)]">Frames</span>
              <span className="text-[11px] font-mono text-[var(--text-primary)]">
                {clip.frames.length} · {timing}ms
              </span>
            </div>
            <div className="flex h-14 items-end gap-1 overflow-x-auto pb-1">
              {clip.frames.map((src, index) => (
                <div key={`${clip.id}-${index}`} className="flex h-full min-w-[28px] flex-col justify-end" style={{ flex: delays[index] || 1 }}>
                  <img src={src} alt="" className="h-9 w-full rounded-md object-cover border border-[var(--border-subtle)]" />
                  <span className="mt-1 text-center text-[9px] font-mono text-[var(--text-muted)]">{delays[index]}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="w-full max-w-[640px] relative pl-8 flex flex-col">
          <div className="absolute left-[15px] top-3 bottom-3 w-px bg-[var(--border-strong)]" />
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
              aria-pressed={layer === item.id}
              onClick={() => setLayer(item.id)}
              className={`relative mb-2 w-full rounded-2xl border p-3 text-left cursor-pointer transition-all ${
                layer === item.id ? "border-[var(--accent-primary)] bg-[var(--bg-elevated)] shadow-md" : "border-[var(--border-subtle)] bg-[var(--bg-panel)] hover:border-[var(--border-strong)] hover:-translate-y-[0.5px]"
              }`}
            >
              <span className={`absolute -left-8 top-4 h-4 w-4 rounded-full border-2 border-[var(--bg-app)] transition-transform ${layer === item.id ? "scale-110 ring-2 ring-[var(--accent-primary)]/40" : ""}`} style={{ background: item.color }} />
              <div className="text-sm font-semibold text-[var(--text-primary)]">{item.title}</div>
              <div className="text-[11px] font-mono text-[var(--text-secondary)] truncate">{item.detail}</div>
            </button>
          ))}
        </div>
      </main>

      <aside className="min-h-0 bg-[var(--bg-sidebar)] border-l border-[var(--border-subtle)] p-4 flex flex-col gap-3 overflow-y-auto">
        <div className="text-sm font-bold text-[var(--text-primary)]">
          {layer === "picture" ? "Picture" : layer === "title" ? "Title" : layer === "icon" ? "Icon" : "Background"}
        </div>

        {layer === "picture" && (
          <>
            <button
              type="button"
              onClick={onUpload}
              className="h-9 rounded-lg bg-[var(--accent-soft)] border border-[var(--accent-primary)]/60 hover:border-[var(--accent-primary)] hover:bg-[var(--accent-primary)]/25 text-xs font-semibold text-white flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
            >
              <Upload size={14} />
              Upload image or GIF
            </button>
            <button
              type="button"
              onClick={onSample}
              className="h-8 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-panel)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
            >
              <Film size={13} />
              Sample pulse
            </button>
            {clip && face.media && (
              <>
                <div role="group" aria-label="Image fit" className="grid grid-cols-3 gap-0.5 bg-[var(--bg-input)] border border-[var(--border-subtle)] p-0.5 rounded-md">
                  {(["cover", "contain", "stretch"] as const).map((fit) => (
                    <button
                      key={fit}
                      type="button"
                      aria-pressed={face.media?.fit === fit}
                      onClick={() => onMediaPatch({ fit })}
                      className={`h-7 rounded text-[11px] capitalize cursor-pointer transition-all border ${
                        face.media?.fit === fit
                          ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-white font-semibold shadow-xs"
                          : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
                      }`}
                    >
                      {fit}
                    </button>
                  ))}
                </div>
                <div className="flex flex-col gap-1">
                  <div className="flex items-center justify-between text-[11px] text-[var(--text-secondary)]">
                    <label>Speed</label>
                    <span className="font-mono text-[var(--text-primary)]">{face.media.speed.toFixed(2)}×</span>
                  </div>
                  <input
                    aria-label="Animation speed"
                    type="range"
                    min={0.25}
                    max={3}
                    step={0.25}
                    value={face.media.speed}
                    onChange={(event) => onMediaPatch({ speed: Number(event.target.value) })}
                    className="accent-[var(--accent-primary)] cursor-pointer"
                  />
                </div>
                <button
                  type="button"
                  onClick={() => onMediaPatch({ loopPlayback: !face.media?.loopPlayback })}
                  className="h-8 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-panel)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-xs text-[var(--text-primary)] cursor-pointer transition-colors"
                >
                  {face.media.loopPlayback ? "Looping" : "Play once"}
                </button>
                <button
                  type="button"
                  onClick={onClearMedia}
                  className="h-8 rounded-lg border border-[var(--status-error)]/35 hover:border-[var(--status-error)] hover:bg-[var(--status-error)]/10 text-xs text-[var(--status-error)] flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
                >
                  <Trash2 size={13} />
                  Remove picture
                </button>
              </>
            )}
            {!clip && <p className="text-[11px] leading-relaxed text-[var(--text-muted)]">Drop a PNG, JPEG, WebP, or GIF on the canvas. A GIF plays on the key after you send the page.</p>}
          </>
        )}

        {layer === "title" && (
          <>
            <input
              aria-label="Key title"
              value={face.title}
              onChange={(event) => onChange({ title: event.target.value })}
              className="h-9 px-2.5 rounded-lg bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-sm text-[var(--text-primary)] outline-none"
            />
            <button
              type="button"
              onClick={() => onChange({ showTitle: !face.showTitle })}
              className="h-8 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-panel)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-xs text-[var(--text-primary)] flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
            >
              {face.showTitle ? <Eye size={13} /> : <EyeOff size={13} />}
              {face.showTitle ? "Title is on the key" : "Title is hidden"}
            </button>
            <div role="group" aria-label="Title size" className="grid grid-cols-3 gap-0.5 bg-[var(--bg-input)] border border-[var(--border-subtle)] p-0.5 rounded-md">
              {(["Small", "Medium", "Large"] as const).map((size) => (
                <button
                  key={size}
                  type="button"
                  aria-pressed={face.titleSize === size}
                  onClick={() => onChange({ titleSize: size })}
                  className={`h-7 rounded text-[11px] cursor-pointer transition-all border ${
                    face.titleSize === size
                      ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-white font-semibold shadow-xs"
                      : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
                  }`}
                >
                  {size}
                </button>
              ))}
            </div>
            <p className="text-[11px] text-[var(--text-muted)] flex items-center gap-1">
              <Type size={12} /> Two short words render largest on the deck.
            </p>
          </>
        )}

        {layer === "icon" && (
          <div className="grid grid-cols-5 gap-1.5 max-h-48 overflow-y-auto p-1.5 bg-[var(--bg-input)] border border-[var(--border-subtle)] rounded-md">
            {AVAILABLE_ICON_NAMES.map((name) => (
              <button
                key={name}
                type="button"
                title={name}
                aria-label={`Select ${name} icon`}
                aria-pressed={face.iconName === name}
                onClick={() => onChange({ iconName: name })}
                className={`h-9 rounded-md flex items-center justify-center cursor-pointer transition-colors border ${
                  face.iconName === name ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]" : "border-transparent hover:bg-[var(--bg-elevated)] hover:border-[var(--border-subtle)]"
                }`}
              >
                <DeckIcon name={name} size={16} color={face.accentColor} />
              </button>
            ))}
          </div>
        )}
        {layer === "icon" && <BrandIconPicker onPick={(file) => onBrandPick(file)} />}

        {layer === "background" && (
          <>
            <div className="grid grid-cols-5 gap-1.5">
              {LOOKS.map((look) => (
                <button
                  key={look.name}
                  type="button"
                  title={look.name}
                  aria-label={`Use ${look.name} look`}
                  aria-pressed={face.accentColor === look.accent}
                  onClick={() => onChange({ accentColor: look.accent, bgColor: look.bg, backgroundStyle: "Gradient" })}
                  className={`h-9 rounded-lg border cursor-pointer transition-transform ${face.accentColor === look.accent ? "border-white ring-2 ring-[var(--accent-primary)] ring-offset-2 ring-offset-[var(--bg-sidebar)] scale-105" : "border-white/10 hover:scale-105"}`}
                  style={{ background: `linear-gradient(160deg, ${look.accent}, ${look.bg})` }}
                />
              ))}
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-[11px] text-[var(--text-secondary)]">Accent</label>
              <input
                aria-label="Accent color"
                type="color"
                value={face.accentColor}
                onChange={(event) => onChange({ accentColor: event.target.value })}
                className="h-8 w-full bg-[var(--bg-input)] border border-[var(--border-subtle)] p-0.5 cursor-pointer rounded-md"
              />
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-[11px] text-[var(--text-secondary)]">Surface</label>
              <input
                aria-label="Surface color"
                type="color"
                value={face.bgColor}
                onChange={(event) => onChange({ bgColor: event.target.value })}
                className="h-8 w-full bg-[var(--bg-input)] border border-[var(--border-subtle)] p-0.5 cursor-pointer rounded-md"
              />
            </div>
            <div className="grid grid-cols-2 gap-0.5 bg-[var(--bg-input)] border border-[var(--border-subtle)] p-0.5 rounded-md">
              {(["Gradient", "Solid"] as const).map((style) => (
                <button
                  key={style}
                  type="button"
                  onClick={() => onChange({ backgroundStyle: style })}
                  className={`h-7 rounded text-[11px] cursor-pointer transition-all border ${
                    face.backgroundStyle === style
                      ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-white font-semibold shadow-xs"
                      : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
                  }`}
                >
                  {style}
                </button>
              ))}
            </div>
            <div className="flex flex-col gap-1.5">
              <div className="flex items-center justify-between text-[11px] text-[var(--text-secondary)]">
                <label>Corners</label>
                <span className="font-mono text-[var(--text-primary)]">{face.cornerRadius}px</span>
              </div>
              <input
                aria-label="Corner radius"
                type="range"
                min={2}
                max={18}
                value={face.cornerRadius}
                onChange={(event) => onChange({ cornerRadius: Number(event.target.value) })}
                className="accent-[var(--accent-primary)] cursor-pointer"
              />
            </div>
          </>
        )}

        <button
          type="button"
          onClick={onPaintPage}
          className="mt-auto h-8 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-panel)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-[11px] text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer transition-colors"
        >
          Paint this color on every key
        </button>
        <button
          type="button"
          onClick={onApply}
          className="h-10 rounded-lg bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white flex items-center justify-center gap-1.5 cursor-pointer shadow-xs transition-colors"
        >
          <ImagePlus size={14} />
          Send this page to the deck
        </button>
      </aside>
    </div>
  );
};
