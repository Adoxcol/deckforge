import type React from "react";
import type { KnobPart } from "../components/ActionDrawer";
import type { StarterProfile } from "../data/starterProfiles";
import type {
  AutoSwitchRule,
  RecoveryDraft,
  EncoderConfig,
  KeyConfig,
  HotkeyCapture,
  MediaPreview,
  NavScreenId,
  Profile,
  ScreenMediaConfig,
  ScreenTarget,
  TouchZoneConfig,
} from "../types/deckforge";

export type ToastAction = { label: string; run: () => void };

export type EditorActions = {
  recoveryChecked: boolean;
  recoveryDraft: RecoveryDraft | null;
  recoveryError: string | null;
  quitOpen: boolean;
  showOnboarding: boolean;
  hotkeyTarget: HotkeyCapture | null;
  activeKey: KeyConfig | null;
  activeZone: TouchZoneConfig | null;
  activeEncoder: EncoderConfig | null;
  activeScreen: NavScreenId;
  activeProfile: Profile | undefined;
  updateSelectedKey: (patch: Partial<KeyConfig>) => void;
  updateSelectedZone: (index: number, patch: Partial<TouchZoneConfig>) => void;
  updateSelectedEncoder: (index: number, patch: Partial<EncoderConfig>) => void;
  handleSaveLocally: () => void;
  handleUndo: () => void;
  handleRedo: () => void;
  handleApplyToDevice: () => void | Promise<void>;
  triggerToast: (text: string, kind?: "success" | "info" | "warning", action?: ToastAction) => void;
  setHotkeyTarget: (target: "key" | "zone" | "cw" | "ccw" | "push" | null) => void;
  setSelectedPageIdx: React.Dispatch<React.SetStateAction<number>>;
  clipFor: (cfg?: ScreenMediaConfig | null) => MediaPreview | null;
  handleAddPage: () => void;
  ingestFile: (file: File, target: ScreenTarget, index: number) => Promise<void>;
  handleDropAction: (target: "key" | "zone" | "encoder", index: number, actionId: string, part?: KnobPart) => void;
  handleMoveKey: (from: number, to: number) => void;
  handleMoveKeyToPage: (keyIndex: number, pageIndex: number) => void;
  saveRules: (rules: AutoSwitchRule[]) => void;
  handleAddProfile: (starter: StarterProfile) => void;
  handleExportProfile: () => void | Promise<void>;
  handleImportProfile: () => void | Promise<void>;
};
