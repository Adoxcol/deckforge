import React from "react";
import { Film, ImagePlus, Trash2 } from "lucide-react";
import type { MediaPreview, ScreenMediaConfig } from "../types/deckforge";

interface MediaConfiguratorProps {
  title: string;
  pixelLabel: string;
  config: ScreenMediaConfig | null;
  clip: MediaPreview | null;
  onUpload: () => void;
  onBrowse: () => void;
  onSample: () => void;
  onChange: (patch: Partial<ScreenMediaConfig>) => void;
  onClear: () => void;
}

export const MediaConfigurator: React.FC<MediaConfiguratorProps> = ({
  title,
  pixelLabel,
  config,
  clip,
  onUpload,
  onBrowse,
  onSample,
  onChange,
  onClear,
}) => {
  return (
    <div className="rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-panel)] p-2.5 flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <div>
          <div className="text-[11.5px] font-semibold text-[var(--text-primary)]">{title}</div>
          <div className="text-[10.5px] font-mono text-[var(--text-muted)]">{pixelLabel}</div>
        </div>
        {clip && clip.frameCount > 1 ? (
          <span className="inline-flex items-center gap-1 rounded bg-[var(--accent-soft)] px-1.5 py-0.5 text-[10px] font-semibold text-[var(--accent-text)]">
            <Film size={11} />
            GIF
          </span>
        ) : null}
      </div>

      <button
        type="button"
        onClick={onBrowse}
        className="h-9 rounded-md bg-[var(--accent-soft)] border border-[var(--accent-primary)]/45 hover:border-[var(--accent-primary)] hover:bg-[var(--accent-primary)]/20 active:scale-[0.98] text-xs font-semibold text-[var(--accent-text)] hover:text-white cursor-pointer transition-all"
      >
        Browse media library
      </button>
      <div className="flex gap-1.5">
        <button
          type="button"
          onClick={onUpload}
          className="flex-1 h-8 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--accent-primary)]/60 hover:bg-[var(--bg-elevated)] active:scale-[0.98] text-[11.5px] font-semibold text-[var(--text-primary)] flex items-center justify-center gap-1.5 cursor-pointer transition-all"
        >
          <ImagePlus size={13} />
          Upload image or GIF
        </button>
        <button
          type="button"
          onClick={onSample}
          className="h-8 px-2.5 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-elevated)] active:scale-[0.98] text-[11px] text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer transition-all"
        >
          Sample
        </button>
      </div>

      {config && clip ? (
        <>
          <div className="text-[10.5px] text-[var(--text-secondary)] truncate">
            {clip.fileName}
            {clip.width > 0 ? ` · source ${clip.width}×${clip.height}` : ""}
            {` · ${clip.frameCount} frame${clip.frameCount === 1 ? "" : "s"}`}
            {clip.sourceFrameCount > clip.frameCount ? ` (from ${clip.sourceFrameCount})` : ""}
          </div>
          <div role="group" aria-label="Image fit" className="grid grid-cols-3 gap-0.5 bg-[var(--bg-input)] border border-[var(--border-subtle)] p-0.5 rounded-md">
            {(["cover", "contain", "stretch"] as const).map((fit) => (
              <button
                key={fit}
                type="button"
                aria-pressed={config.fit === fit}
                onClick={() => onChange({ fit })}
                className={`h-7 rounded-[4px] text-[11px] capitalize cursor-pointer transition-all border ${
                  config.fit === fit
                    ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-[var(--text-primary)] font-semibold shadow-xs"
                    : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
                }`}
              >
                {fit}
              </button>
            ))}
          </div>
          <div>
            <div className="flex items-center justify-between text-[11px] text-[var(--text-secondary)] mb-1">
              <span>Playback speed</span>
              <span className="font-mono tabular-nums text-[var(--text-primary)]">{config.speed.toFixed(2)}×</span>
            </div>
            <input
              aria-label="Playback speed"
              type="range"
              min={0.25}
              max={3}
              step={0.25}
              value={config.speed}
              onChange={(e) => onChange({ speed: Number(e.target.value) })}
              className="w-full accent-[var(--accent-primary)] cursor-pointer"
            />
          </div>
          <div className="flex items-center justify-between">
            <span className="text-[11.5px] text-[var(--text-secondary)]">Loop animation</span>
            <button
              type="button"
              role="switch"
              aria-label="Loop animation"
              aria-checked={config.loopPlayback}
              onClick={() => onChange({ loopPlayback: !config.loopPlayback })}
              className={`w-9 h-5 rounded-full p-0.5 cursor-pointer transition-colors shrink-0 ${
                config.loopPlayback ? "bg-[var(--accent-primary)]" : "bg-[var(--border-strong)]"
              }`}
            >
              <div
                className={`w-4 h-4 rounded-full bg-white shadow-xs transition-transform ${
                  config.loopPlayback ? "translate-x-4" : "translate-x-0"
                }`}
              />
            </button>
          </div>
          <p className="text-[10.5px] leading-relaxed text-[var(--text-muted)]">
            The deck plays each frame at this speed, with at least 80ms between frames so the LCD bus stays stable. Press Apply to Device to push every screen.
          </p>
          <button
            type="button"
            onClick={onClear}
            className="h-7 rounded-md border border-[var(--border-subtle)] text-[11px] text-[var(--text-secondary)] hover:text-[var(--status-error)] hover:border-[var(--status-error)]/60 hover:bg-[var(--status-error)]/10 active:scale-[0.98] flex items-center justify-center gap-1 cursor-pointer transition-all"
          >
            <Trash2 size={12} />
            Remove image
          </button>
        </>
      ) : (
        <p className="text-[10.5px] leading-relaxed text-[var(--text-muted)]">
          PNG, JPEG, WebP, or GIF. It is fitted to this screen’s pixel size. GIFs animate in the editor and on the hardware after Apply.
        </p>
      )}
      {config && !clip ? (
        <p className="text-[10.5px] text-[var(--status-warning)]">This screen’s file is missing. Upload it again.</p>
      ) : null}
    </div>
  );
};
