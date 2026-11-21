import { AppAudioTarget } from "./AppAudioTarget";
import React from "react";
import type { ObsSettings } from "../types/deckforge";
import { ActionPicker } from "./ActionPicker";
import { joinOutputPair, splitOutputPair } from "../lib/audioOutputs";
import { joinSound, splitSound } from "../lib/soundboard";
import { deckforgeApi } from "../bridge/tauriBridge";

export interface ActionChoice {
  id: string;
  actionLabel: string;
  defaultParam: string;
  title?: string;
  category?: string;
  description?: string;
  iconName?: string;
}

const PRESETS = ["Ctrl+Shift+M", "Win+Shift+S", "Win+D", "Alt+F4", "VK_MEDIA_PLAY_PAUSE"];

export function actionNeedsParam(id: string): boolean {
  return (
    id === "custom.hotkey" ||
    id === "sys.hotkey" ||
    id.startsWith("custom.") ||
    id.startsWith("hotkey.") ||
    id === "obs.set_scene" ||
    id === "obs.toggle_mute" ||
    id === "profile.switch_page" ||
    id === "profile.switch" ||
    id === "macro.run" ||
    id.startsWith("audio.app_") ||
    id === "audio.cycle_output" ||
    id === "audio.play_sound"
  );
}

export function splitStoredBinding(
  stored: string,
  param: string | undefined,
  knownIds: string[]
): { actionId: string; actionParam: string } {
  const extra = param ?? "";
  if (knownIds.includes(stored)) return { actionId: stored, actionParam: extra };
  return { actionId: "custom.hotkey", actionParam: stored || extra };
}

function paramLabel(actionId: string): string {
  if (actionId.startsWith("hotkey.")) return "Shortcut";
  if (actionId === "obs.set_scene") return "OBS scene name";
  if (actionId === "obs.toggle_mute") return "OBS audio source name";
  if (actionId.includes("launch")) return "Application / file path";
  if (actionId.includes("shell")) return "PowerShell / command";
  if (actionId.includes("url")) return "Website URL";
  if (actionId.includes("http")) return "Webhook URL";
  if (actionId.includes("text")) return "Text to paste";
  if (actionId === "profile.switch_page") return "Page";
  if (actionId === "profile.switch") return "Profile";
  if (actionId === "macro.run") return "Macro";
  if (actionId === "audio.cycle_output") return "Outputs";
  if (actionId === "audio.play_sound") return "Sound file";
  if (actionId === "custom.hotkey" || actionId === "sys.hotkey") return "Shortcut";
  return "Parameter";
}

/** Two pickers: the outputs a key flips between. Both blank goes through every output. */
const OutputPair: React.FC<{ value: string; options: { value: string; label: string }[]; onChange: (value: string) => void }> = ({ value, options, onChange }) => {
  const [a, b] = splitOutputPair(value);
  const select = (current: string, set: (next: string) => void, label: string) => (
    <label className="flex flex-col gap-1">
      <span className="text-[10.5px] text-[var(--text-secondary)]">{label}</span>
      <select
        value={current}
        onChange={(event) => set(event.target.value)}
        className="w-full h-8 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none cursor-pointer transition-colors"
      >
        <option value="">Not set</option>
        {current && !options.some((o) => o.value === current) && <option value={current}>{current} (not plugged in)</option>}
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </label>
  );
  return (
    <div className="flex flex-col gap-2">
      {select(a, (next) => onChange(joinOutputPair(next, b)), "Switch between")}
      {select(b, (next) => onChange(joinOutputPair(a, next)), "and")}
      <span className="text-[10.5px] text-[var(--text-muted)] leading-snug">
        {!a && !b
          ? "Leave both empty to go through every output in turn."
          : a && b
          ? "Each press moves sound to the other one."
          : "Pick a second output, or the key always uses this one."}
      </span>
    </div>
  );
};

const SoundClip: React.FC<{ value: string; onChange: (value: string) => void }> = ({ value, onChange }) => {
  const { volume, path } = splitSound(value);
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <span className="text-[10.5px] text-[var(--text-secondary)]">Sound file</span>
        <button
          type="button"
          onClick={() => {
            void deckforgeApi.pickSoundFile().then((picked) => {
              if (picked) onChange(joinSound(volume, picked));
            });
          }}
          className="px-2 py-0.5 rounded text-[10.5px] font-semibold cursor-pointer bg-[var(--accent-soft)] text-[var(--accent-text)] border border-[var(--accent-primary)]/45 hover:bg-[var(--accent-primary)] hover:text-white transition-colors"
        >
          Browse
        </button>
      </div>
      <input
        value={path}
        aria-label="Sound file"
        onChange={(event) => onChange(joinSound(volume, event.target.value))}
        placeholder="C:\sounds\horn.wav"
        className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs font-mono text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)]"
      />
      <label className="flex items-center gap-2">
        <span className="text-[10.5px] text-[var(--text-secondary)] w-16 shrink-0">Volume {volume}</span>
        <input
          type="range"
          min={0}
          max={100}
          value={volume}
          aria-label="Sound volume"
          onChange={(event) => onChange(joinSound(Number(event.target.value), path))}
          className="flex-1 accent-[var(--accent-primary)]"
        />
      </label>
      <span className="text-[10.5px] text-[var(--text-muted)] leading-snug">
        wav, mp3, or ogg, up to 20 MB. A new sound replaces the one that is playing.
      </span>
    </div>
  );
};

export const ActionBinding: React.FC<{
  label: string;
  actionId: string;
  actionParam: string;
  choices: ActionChoice[];
  recording: boolean;
  onRecord: () => void;
  onChange: (actionId: string, actionParam: string) => void;
  paramOptions?: { value: string; label: string }[];
  onBrowse?: () => void;
}> = ({ label, actionId, actionParam, choices, recording, onRecord, onChange, paramOptions, onBrowse }) => {
  const showParam = actionNeedsParam(actionId);
  const showRecord = actionId === "custom.hotkey" || actionId === "sys.hotkey";
  const showPresets = showRecord;
  return (
    <div className="p-2.5 rounded-lg bg-[var(--bg-panel)] border border-[var(--border-subtle)] flex flex-col gap-2">
      <label className="text-[11px] font-semibold text-[var(--text-primary)]">{label}</label>
      <ActionPicker
        value={choices.some((choice) => choice.id === actionId) ? actionId : actionId || "custom.hotkey"}
        choices={choices}
        onOpen={onBrowse}
        onChange={(next) => {
          const choice = choices.find((item) => item.id === next);
          onChange(next, next.startsWith("audio.app_") && actionId.startsWith("audio.app_")
            ? actionParam : actionNeedsParam(next) ? choice?.defaultParam ?? "" : "");
        }}
      />
      {actionId.startsWith("audio.app_") ? (
        <AppAudioTarget value={actionParam} onChange={next => onChange(actionId, next)} />
      ) : actionId === "audio.cycle_output" ? (
        <OutputPair value={actionParam} options={paramOptions ?? []} onChange={(next) => onChange(actionId, next)} />
      ) : actionId === "audio.play_sound" ? (
        <SoundClip value={actionParam} onChange={(next) => onChange(actionId, next)} />
      ) : showParam && paramOptions && paramOptions.length > 0 ? (
        <select
          value={actionParam}
          onChange={(event) => onChange(actionId, event.target.value)}
          className="w-full h-8 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none cursor-pointer transition-colors"
        >
          {!paramOptions.some((option) => option.value === actionParam) && actionParam && (
          <option value={actionParam}>{actionParam}</option>
        )}
        {paramOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      ) : showParam ? (
        <>
          <div className="flex items-center justify-between gap-2">
            <span className="text-[10.5px] text-[var(--text-secondary)]">{paramLabel(actionId)}</span>
            {showRecord && (
              <button
                type="button"
                onClick={onRecord}
                className={`px-2 py-0.5 rounded text-[10.5px] font-semibold cursor-pointer transition-colors ${
                  recording
                    ? "bg-[var(--status-error)] text-white animate-pulse"
                    : "bg-[var(--accent-soft)] text-[var(--accent-text)] border border-[var(--accent-primary)]/45 hover:bg-[var(--accent-primary)] hover:text-white"
                }`}
              >
                {recording ? "Press any key..." : "Record keybind"}
              </button>
            )}
          </div>
          <input
            value={actionParam}
            onChange={(event) => onChange(actionId, event.target.value)}
            placeholder="Ctrl+Shift+M, notepad.exe, https://..."
            className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs font-mono text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)] transition-colors"
          />
          {showPresets && (
            <div className="flex flex-wrap gap-1">
              {PRESETS.map((preset) => (
                <button
                  key={preset}
                  type="button"
                  onClick={() => onChange(actionId === "sys.hotkey" ? "sys.hotkey" : "custom.hotkey", preset)}
                  className="px-1.5 py-0.5 rounded bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:text-[var(--text-primary)] text-[10px] font-mono text-[var(--text-secondary)] cursor-pointer transition-colors"
                >
                  {preset}
                </button>
              ))}
            </div>
          )}
        </>
      ) : null}
    </div>
  );
};

export const ObsConnection: React.FC<{
  obsSettings: ObsSettings;
  onSave: (settings: ObsSettings) => void;
}> = ({ obsSettings, onSave }) => (
  <div className="p-2.5 rounded-lg bg-[var(--bg-panel)] border border-[var(--border-subtle)] flex flex-col gap-2">
    <div className="text-[11px] font-semibold text-[var(--text-primary)]">OBS WebSocket</div>
    <p className="text-[10.5px] leading-relaxed text-[var(--text-secondary)]">
      In OBS, open Tools, then WebSocket Server Settings, and turn the server on. The usual address is ws://127.0.0.1:4455.
    </p>
    <input
      value={obsSettings.url}
      onChange={(event) => onSave({ ...obsSettings, url: event.target.value })}
      aria-label="OBS WebSocket URL"
      className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs font-mono text-[var(--text-primary)] outline-none"
    />
    <input
      type="password"
      value={obsSettings.password}
      placeholder="Password, if OBS has one"
      aria-label="OBS WebSocket password"
      onChange={(event) => onSave({ ...obsSettings, password: event.target.value })}
      className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)]"
    />
  </div>
);
