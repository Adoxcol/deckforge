import type { AudioAppState, DeckFaceState, EncoderConfig } from "../types/deckforge";

export function appAudioLabel(app: AudioAppState): string {
  if (app.muted) return "Muted";
  const volume = app.minVolume === app.maxVolume ? `${app.minVolume}%` : `${app.minVolume}-${app.maxVolume}%`;
  return app.mixedMute ? `${volume} MIX` : volume;
}
export function appAudioLook(param: string, face: DeckFaceState | null) {
  const app = face?.appAudio?.[param.trim().toLowerCase()];
  return {
    title: !param.trim() ? "Choose app" : !face?.appAudio ? "Unavailable" : app ? appAudioLabel(app) : "No session",
    level: !app ? null : app.muted ? 0 : app.minVolume === app.maxVolume && !app.mixedMute ? app.minVolume : null,
    muted: app?.muted ?? false,
  };
}
export function encoderAudioLook(enc: EncoderConfig, face: DeckFaceState | null) {
  const bindings = [[enc.cwAction, enc.cwParam], [enc.ccwAction, enc.ccwParam], [enc.pushAction, enc.pushParam]];
  const appBindings = bindings.filter(([action]) => action?.startsWith("audio.app_"));
  const master = bindings.some(([action]) => ["audio.volume_up", "audio.volume_down", "audio.volume_mute"].includes(action ?? ""));
  if (appBindings.length) {
    // One knob can mix app-specific actions with master controls. A single
    // app reading would misleadingly describe only part of its behavior.
    if (master) return { title: "Multiple targets", level: null, muted: false };
    const targets = new Set(appBindings.map(([, param]) => (param ?? "").trim().toLowerCase()));
    // A knob may intentionally target different apps on each direction.
    return targets.size === 1 ? appAudioLook(appBindings[0][1] ?? "", face) : { title: "Multiple apps", level: null, muted: false };
  }
  return master && face?.volumePercent != null
    ? { title: `${face.volumePercent}%${face.speakerMuted ? " muted" : ""}`, level: face.volumePercent, muted: face.speakerMuted === true }
    : { title: enc.valueText, level: null, muted: false };
}
