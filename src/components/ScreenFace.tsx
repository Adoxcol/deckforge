import React, { useEffect, useState } from "react";
import type { MediaPreview, ScreenMediaConfig } from "../types/deckforge";
import { DeckIcon } from "./DeckIcon";

interface ScreenFaceProps {
  clip?: MediaPreview | null;
  config?: ScreenMediaConfig | null;
  iconName: string;
  accent: string;
  title?: string;
  showTitle?: boolean;
  titleSize?: string;
  iconSize?: number;
  className?: string;
}

export const ScreenFace: React.FC<ScreenFaceProps> = ({
  clip,
  config,
  iconName,
  accent,
  title,
  showTitle,
  titleSize,
  iconSize = 18,
  className = "",
}) => {
  const frames = clip?.frames ?? [];
  const nativeGif = frames.length === 1 && (frames[0].includes("image/gif") || clip?.kind === "gif");
  const [frame, setFrame] = useState(0);

  useEffect(() => {
    setFrame(0);
  }, [clip?.id]);

  useEffect(() => {
    if (frames.length < 2) return;
    const delayMs = clip?.delaysMs?.[frame] || clip?.frameDelayMs || 100;
    const speed = config?.speed && config.speed > 0 ? config.speed : 1;
    const timer = window.setTimeout(() => {
      setFrame((current) => {
        const next = current + 1;
        if (next >= frames.length) return config?.loopPlayback === false ? current : 0;
        return next;
      });
    }, Math.max(40, delayMs / speed));
    return () => window.clearTimeout(timer);
  }, [frame, frames.length, clip, config?.speed, config?.loopPlayback]);

  if (frames.length === 0) {
    return (
      <div className={`flex h-full w-full flex-col items-center justify-center gap-1 ${className || "relative"}`}>
        <DeckIcon name={iconName} size={iconSize} color={accent} />
        {showTitle && title ? (
          <span className="max-w-full truncate px-1 text-center text-[11px] font-semibold text-[var(--text-primary)]">{title}</span>
        ) : null}
      </div>
    );
  }

  const fit = config?.fit === "stretch" ? "fill" : config?.fit === "contain" ? "contain" : "cover";
  const src = nativeGif ? frames[0] : frames[Math.min(frame, frames.length - 1)];
  const titleClass =
    titleSize === "Small" ? "text-[10px]" : titleSize === "Large" ? "text-[13px]" : "text-[11px]";

  return (
    <div className={`h-full w-full overflow-hidden ${className || "relative"}`}>
      <img src={src} alt="" draggable={false} className="absolute inset-0 h-full w-full" style={{ objectFit: fit }} />
      {showTitle && title ? (
        <span
          className={`absolute bottom-1 left-1 right-1 truncate rounded bg-black/60 px-1 py-px text-center font-semibold text-white ${titleClass}`}
        >
          {title}
        </span>
      ) : null}
    </div>
  );
};
