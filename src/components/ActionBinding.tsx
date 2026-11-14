import React from "react";
import type { ObsSettings } from "../types/deckforge";
import { ActionPicker } from "./ActionPicker";

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
    id === "audio.cycle_output"
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
  if (actionId === "custom.hotkey" || actionId === "sys.hotkey") return "Shortcut";
  return "Parameter";
}

export const ActionBinding: React.FC<{
  label: string;
  actionId: string;
  actionParam: string;
  choices: ActionChoice[];
  recording: boolean;
  onRecord: () => void;
  onChange: (actionId: string, actionParam: string) => void;
  paramOptions?: { value: string; label: string }[];
}> = ({ label, actionId, actionParam, choices, recording, onRecord, onChange, paramOptions }) => {
  const showParam = actionNeedsParam(actionId);
  const showRecord = actionId === "custom.hotkey" || actionId === "sys.hotkey";
  const showPresets = showRecord;
  return (
    <div className="p-2.5 rounded-lg bg-[var(--bg-panel)] border border-[var(--border-subtle)] flex flex-col gap-2">
      <label className="text-[11px] font-semibold text-[var(--text-primary)]">{label}</label>
      <ActionPicker
        value={choices.some((choice) => choice.id === actionId) ? actionId : actionId || "custom.hotkey"}
        choices={choices}
        onChange={(next) => {
          const choice = choices.find((item) => item.id === next);
          onChange(next, actionNeedsParam(next) ? choice?.defaultParam ?? "" : "");
        }}
      />
      {showParam && paramOptions && paramOptions.length > 0 ? (
        <select
          value={actionParam}
          onChange={(event) => onChange(actionId, event.target.value)}
          className="w-full h-8 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] text-xs text-[var(--text-primary)] outline-none"
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
                className={`px-2 py-0.5 rounded text-[10.5px] font-semibold cursor-pointer ${
                  recording
                    ? "bg-[var(--status-error)] text-white animate-pulse"
                    : "bg-[var(--accent-soft)] text-[var(--accent-primary)] border border-[var(--accent-primary)]/50"
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
            className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] text-xs font-mono text-[var(--text-primary)] outline-none"
          />
          {showPresets && (
            <div className="flex flex-wrap gap-1">
              {PRESETS.map((preset) => (
                <button
                  key={preset}
                  type="button"
                  onClick={() => onChange(actionId === "sys.hotkey" ? "sys.hotkey" : "custom.hotkey", preset)}
                  className="px-1.5 py-0.5 rounded bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] text-[10px] font-mono text-[var(--text-secondary)] cursor-pointer"
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
      className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] text-xs font-mono text-[var(--text-primary)] outline-none"
    />
    <input
      type="password"
      value={obsSettings.password}
      placeholder="Password, if OBS has one"
      onChange={(event) => onSave({ ...obsSettings, password: event.target.value })}
      className="w-full h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] text-xs text-[var(--text-primary)] outline-none"
    />
  </div>
);
