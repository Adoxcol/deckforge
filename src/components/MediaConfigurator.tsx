import React from "react";
import { Film, ImagePlus, Trash2 } from "lucide-react";
import type { MediaPreview, ScreenMediaConfig } from "../types/deckforge";

interface MediaConfiguratorProps {
  title: string;
  pixelLabel: string;
  config: ScreenMediaConfig | null;
  clip: MediaPreview | null;
  onUpload: () => void;
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

      <div className="flex gap-1.5">
        <button
          onClick={onUpload}
          className="flex-1 h-8 rounded-md bg-[var(--accent-soft)] border border-[var(--accent-primary)]/50 text-[11.5px] font-semibold text-white flex items-center justify-center gap-1.5 cursor-pointer"
        >
          <ImagePlus size={13} />
          Upload image or GIF
        </button>
        <button
          onClick={onSample}
          className="h-8 px-2 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] text-[11px] text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer"
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
          <div className="grid grid-cols-3 gap-1">
            {(["cover", "contain", "stretch"] as const).map((fit) => (
              <button
                key={fit}
                onClick={() => onChange({ fit })}
                className={`h-7 rounded text-[11px] capitalize cursor-pointer ${
                  config.fit === fit
                    ? "bg-[var(--accent-soft)] text-white font-semibold"
                    : "bg-[var(--bg-input)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                }`}
              >
                {fit}
              </button>
            ))}
          </div>
          <div>
            <div className="flex items-center justify-between text-[11px] text-[var(--text-secondary)] mb-1">
              <span>Playback speed</span>
              <span className="font-mono text-[var(--text-primary)]">{config.speed.toFixed(2)}×</span>
            </div>
            <input
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
              aria-label="Loop animation"
              aria-pressed={config.loopPlayback}
              onClick={() => onChange({ loopPlayback: !config.loopPlayback })}
              className={`w-10 h-5 rounded-full p-0.5 cursor-pointer ${
                config.loopPlayback ? "bg-[var(--accent-primary)]" : "bg-[var(--border-subtle)]"
              }`}
            >
              <div
                className={`w-4 h-4 rounded-full bg-white transition-transform ${
                  config.loopPlayback ? "translate-x-5" : "translate-x-0"
                }`}
              />
            </button>
          </div>
          <p className="text-[10.5px] leading-relaxed text-[var(--text-muted)]">
            The deck plays each frame at this speed, with at least 80ms between frames so the LCD bus stays stable. Press Apply to Device to push every screen.
          </p>
          <button
            onClick={onClear}
            className="h-7 rounded-md border border-[var(--border-subtle)] text-[11px] text-[var(--text-secondary)] hover:text-[var(--status-error)] hover:border-[var(--status-error)] flex items-center justify-center gap-1 cursor-pointer"
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
