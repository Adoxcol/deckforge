import React from "react";
import type { DeckFaceState } from "../types/deckforge";

/** What a key shows while the PC reports a live value (volume, mic, track). */
export type LiveLook = { title: string; icon: string; level: number | null; muted: boolean };

export function liveKeyLook(actionId: string, title: string, icon: string, face: DeckFaceState | null): LiveLook {
  const still = { title, icon, level: null, muted: false };
  if (!face) return still;
  if (actionId === "media.play_pause" && (face.mediaPlaying != null || face.mediaTitle.trim())) {
    const track = face.mediaTitle.trim() || (face.mediaPlaying ? "Playing" : "Paused");
    return { title: track, icon: face.mediaPlaying ? "Pause" : "Play", level: null, muted: false };
  }
  if (actionId === "audio.mute_mic" && face.micMuted != null) {
    return face.micMuted
      ? { title: "Muted", icon: "MicOff", level: null, muted: true }
      : { title: "Mic Live", icon: "Mic", level: null, muted: false };
  }
  if (
    (actionId === "audio.volume_up" || actionId === "audio.volume_down" || actionId === "audio.volume_mute") &&
    face.volumePercent != null
  ) {
    return {
      title: face.speakerMuted ? "Muted" : `${face.volumePercent}%`,
      icon,
      level: face.volumePercent,
      muted: face.speakerMuted === true,
    };
  }
  return still;
}

export function LevelMeter({ percent, accent, muted }: { percent: number; accent: string; muted: boolean }) {
  const filled = Math.round((Math.max(0, Math.min(100, percent)) / 100) * 8);
  return (
    <span className="flex w-full gap-0.5 px-1">
      {Array.from({ length: 8 }, (_, index) => (
        <span
          key={index}
          className="h-1 flex-1 rounded-sm"
          style={{ background: index < filled && !muted ? accent : "#272D3D" }}
        />
      ))}
    </span>
  );
}

export function liveZoneValue(actionId: string, value: string, face: DeckFaceState | null) {
  if (!face) return value;
  if (actionId === "media.play_pause" && face.mediaPlaying != null) {
    return face.mediaPlaying ? face.mediaTitle || "Playing" : "Paused";
  }
  if (actionId === "audio.mute_mic" && face.micMuted != null) {
    return face.micMuted ? "Muted" : "Live";
  }
  if (
    (actionId === "audio.volume_up" || actionId === "audio.volume_down" || actionId === "audio.volume_mute") &&
    face.volumePercent != null
  ) {
    return face.speakerMuted ? `${face.volumePercent}% muted` : `${face.volumePercent}%`;
  }
  return value;
}
