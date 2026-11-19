import { startPolling } from "./lib/polling";
import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  Activity,
  AlertTriangle,
  Check,
  ChevronLeft,
  ChevronRight,
  Copy,
  Cpu,
  Gamepad2,
  Download,
  Edit3,
  Eye,
  FolderPlus,
  HardDrive,
  HelpCircle,
  Layers,
  LayoutGrid,
  LayoutTemplate,
  Maximize2,
  Palette,
  Play,
  Plus,
  Puzzle,
  RefreshCw,
  RotateCcw,
  RotateCw,
  Save,
  Search,
  Settings,
  Shield,
  Sliders,
  Sparkles,
  Terminal,
  Trash2,
  Upload,
  Usb,
  Wand2,
  X,
  Zap,
} from "lucide-react";
import { deckforgeApi, isTauriRuntime } from "./bridge/tauriBridge";
import { createProfilePersistence, profileSnapshot } from "./lib/profilePersistence";
import { beginEdit, captureLibrary, initialClock, isClean, noteRestored, noteSaved, pushEntry, type HistoryEntry, type ProfileClock } from "./lib/profileHistory";
import { buildStarterProfile, type StarterProfile } from "./data/starterProfiles";
import { createDraftRecovery, type DraftStatus } from "./lib/draftRecovery";
import { QuitDialog } from "./components/QuitDialog";
import { createSaveQueue, savedStatus, type SaveStatus } from "./lib/saveQueue";
import { AVAILABLE_ICON_NAMES, DeckIcon } from "./components/DeckIcon";
import { ActionBinding, ObsConnection, actionNeedsParam, splitStoredBinding } from "./components/ActionBinding";
import { IconStudio } from "./components/IconStudio";
import { ThemesScreen, type KeyFrames } from "./screens/ThemesScreen";
import { GameModeStudio } from "./components/GameModeStudio";
import { MacroStudio } from "./components/MacroStudio";
import { PageTemplates } from "./components/PageTemplates";
import { MediaConfigurator } from "./components/MediaConfigurator";
import { ScreenFace } from "./components/ScreenFace";
import { WorkspaceStatusBar } from "./components/WorkspaceStatusBar";
import { OnboardingModal } from "./components/OnboardingModal";
import { WorkspaceToolbar } from "./components/WorkspaceToolbar";
import { DeckStage } from "./components/DeckStage";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import { ActionDrawer, keyPatchForAction, zonePatchForAction, encoderPatchForAction, type KnobPart } from "./components/ActionDrawer";
import { KeyInspector } from "./components/KeyInspector";
import { DevicesScreen } from "./screens/DevicesScreen";
import { PluginsScreen, pluginActionTemplates } from "./screens/PluginsScreen";
import { SettingsScreen } from "./screens/SettingsScreen";
import {
  DEFAULT_KNOB_LED_COLORS,
  type AutoSwitchRule,
  type RecoveryDraft,
  DraftSnapshot,
  DeckFaceState,
  DeviceDescriptor,
  DiagnosticEntry,
  FrameCacheStats,
  EncoderConfig,
  KeyConfig,
  type HotkeyCapture,
  type KnobLedEffect,
  LegacyStreamDockSummary,
  PluginView,
  MacroRunReport,
  MacroSequence,
  MediaPreview,
  ObsSettings,
  NavScreenId,
  Profile,
  ProfilePage,
  ScreenMediaConfig,
  ScreenTarget,
  SystemTelemetrySnapshot,
  TouchZoneConfig,
  AudioOutput,
} from "./types/deckforge";
import { outputParamOptions } from "./lib/audioOutputs";

import { ACTION_CATALOG, type ActionTemplate } from "./data/actionCatalog";
import { bytesToBase64, buildPulseFrames, errorText } from "./lib/format";
import { macroIdsUsed, placeBackup, parseProfileFile, pickJsonFile, placeImportedProfile, slugify } from "./lib/shareFiles";
import { LevelMeter, liveKeyLook, liveZoneValue } from "./components/LiveFace";
import { onUpdateReady } from "./lib/updates";

type ToastAction = { label: string; run: () => void };

type EditorActions = {
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

const NAV_ITEMS = [
    { id: "workspace", label: "Profiles and keys", icon: Layers },
    { id: "macros", label: "Macro studio", icon: Zap },
    { id: "icons", label: "Icon studio", icon: Palette },
    { id: "themes", label: "Themes", icon: Sparkles },
    { id: "gamemode", label: "Game mode", icon: Gamepad2 },
    { id: "plugins", label: "Plugins", icon: Puzzle },
    { id: "devices", label: "Devices", icon: Usb },
  ] as const;

export const App: React.FC = () => {
  const [activeScreen, setActiveScreen] = useState<NavScreenId>("workspace");
  const [devices, setDevices] = useState<DeviceDescriptor[]>([]);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [autoSwitchRules, setAutoSwitchRules] = useState<AutoSwitchRule[]>([]);
  const [macros, setMacros] = useState<MacroSequence[]>([]);
  const [audioOutputs, setAudioOutputs] = useState<AudioOutput[]>([]);

  // Sound outputs come and go (headsets, docks), so look again whenever the window gets focus.
  useEffect(() => {
    const load = () => deckforgeApi.audioOutputs().then(setAudioOutputs).catch(() => undefined);
    load();
    window.addEventListener("focus", load);
    return () => window.removeEventListener("focus", load);
  }, []);
  const [diagnostics, setDiagnostics] = useState<DiagnosticEntry[]>([]);
  const [frameStats, setFrameStats] = useState<FrameCacheStats>({
    framesEvaluated: 0,
    framesTransmitted: 0,
    framesDeduplicatedSkipped: 0,
    bytesTransmitted: 0,
    bytesSaved: 0,
    lastTransferLatencyMs: 0,
  });
  const [legacySummary, setLegacySummary] = useState<LegacyStreamDockSummary | null>(null);
  const [telemetry, setTelemetry] = useState<SystemTelemetrySnapshot | null>(null);

  const [selectedDeviceId, setSelectedDeviceId] = useState<string>("ajazz-akp05e-pro");
  const [selectedProfileId, setSelectedProfileId] = useState<string>("profile-streaming");
  const [startupProfileId, setStartupProfileId] = useState<string | null>(null);
  const [selectedPageIdx, setSelectedPageIdx] = useState<number>(0);
  const [selectedKeyIdx, setSelectedKeyIdx] = useState<number>(0);
  const [surface, setSurface] = useState<"key" | "zone" | "strip" | "encoder">("key");
  const [surfaceIndex, setSurfaceIndex] = useState(0);
  const [mediaLibrary, setMediaLibrary] = useState<Record<string, MediaPreview>>({});
  const fileRef = useRef<HTMLInputElement>(null);
  const [inspectorTab, setInspectorTab] = useState<"Action" | "Appearance" | "Behavior">("Action");
  const [actionCategoryFilter, setActionCategoryFilter] = useState<string>("All");
  const [canvasZoom, setCanvasZoom] = useState<number>(100);
  const [clipboardKey, setClipboardKey] = useState<KeyConfig | null>(null);
  const [templatesOpen, setTemplatesOpen] = useState<boolean>(false);

  // A revision counter tracks unsaved edits. Undo restores the revision, so
  // returning to the saved profile is clean without stringifying the library.
  const clockRef = useRef<ProfileClock>(initialClock());
  const [clock, setClock] = useState<ProfileClock>(initialClock);
  const savedProfilesRef = useRef<Profile[]>([]);
  const savedLocally = isClean(clock);
  const [profilePersistence] = useState(() => createProfilePersistence<Profile[]>(deckforgeApi.saveProfiles, (snapshot) => {
    savedProfilesRef.current = JSON.parse(snapshot) as Profile[];
  }));
  const commitClock = (next: ProfileClock) => {
    clockRef.current = next;
    setClock(next);
  };
  const saveEditorProfiles = (value: Profile[]) => {
    const at = clockRef.current.current;
    return profilePersistence.save(value).then((result) => {
      commitClock(noteSaved(clockRef.current, at));
      return result;
    });
  };
  const editorRef = useRef<EditorActions | null>(null);
  const [recoveryChecked, setRecoveryChecked] = useState(false);
  const [recoveryDraft, setRecoveryDraft] = useState<RecoveryDraft | null>(null);
  const [recoveryError, setRecoveryError] = useState<string | null>(null);
  const [recoveryBusy, setRecoveryBusy] = useState(false);
  const [draftStatus, setDraftStatus] = useState<DraftStatus>({ pending: false, error: null });
  const [draftWriter] = useState(() => createDraftRecovery<DraftSnapshot>(deckforgeApi.persistProfileDraft, setDraftStatus));
  const [quitOpen, setQuitOpen] = useState(false);
  const [quitting, setQuitting] = useState(false);
  const [quitError, setQuitError] = useState<string | null>(null);
  const latestMacros = useRef(macros);
  latestMacros.current = macros;
  const latestProfiles = useRef(profiles);
  latestProfiles.current = profiles;
  const [saveStatuses, setSaveStatuses] = useState<Record<string, SaveStatus>>({});
  const [bootstrapReady, setBootstrapReady] = useState(false);
  const [bootstrapError, setBootstrapError] = useState<string | null>(null);
  const [bootstrapAttempt, setBootstrapAttempt] = useState(0);
  const [saveQueues] = useState(() => {
    const status = (name: string) => (value: SaveStatus) => setSaveStatuses(prev => ({ ...prev, [name]: value }));
    return {
      Macros: createSaveQueue(deckforgeApi.saveMacros, status("Macros")),
      "Auto-switch rules": createSaveQueue(deckforgeApi.saveAutoSwitchRules, status("Auto-switch rules")),
      "OBS settings": createSaveQueue(deckforgeApi.saveObsSettings, status("OBS settings")),
    };
  });
  const [appliedRefresh, setAppliedRefresh] = useState(0);
  const [undoStack, setUndoStack] = useState<HistoryEntry<Profile[]>[]>([]);
  const [redoStack, setRedoStack] = useState<HistoryEntry<Profile[]>[]>([]);
  const [toastMessage, setToastMessage] = useState<{ text: string; kind: "success" | "info" | "warning"; action?: ToastAction } | null>(null);

  // Sub-screen states
  const [selectedMacroId, setSelectedMacroId] = useState<string>("macro-go-live");
  const [selectedMacroStepIdx, setSelectedMacroStepIdx] = useState<number>(0);
  const [macroRunReport, setMacroRunReport] = useState<MacroRunReport | null>(null);
  const [showOnboarding, setShowOnboarding] = useState<boolean>(false);
  const [livePressedKeyIdx, setLivePressedKeyIdx] = useState<number | null>(null);
  const [liveEncoderEvent, setLiveEncoderEvent] = useState<{ index: number; kind: "cw" | "ccw" | "push" } | null>(null);
  const [liveTouchZoneIdx, setLiveTouchZoneIdx] = useState<number | null>(null);
  const liveEncoderTimerRef = useRef<number | null>(null);
  const liveZoneTimerRef = useRef<number | null>(null);
  const [face, setFace] = useState<DeckFaceState | null>(null);
  const [obsSettings, setObsSettings] = useState<ObsSettings>({ url: "ws://127.0.0.1:4455", password: "" });

  /** A toast with an action button stays longer so there is time to press it. */
  const triggerToast = (text: string, kind: "success" | "info" | "warning" = "success", action?: ToastAction) => {
    setToastMessage({ text, kind, action });
    window.setTimeout(() => {
      setToastMessage((prev) => (prev?.text === text ? null : prev));
    }, action ? 7000 : 3200);
  };
  // Toast buttons run later than the render that made them, so they call through refs.
  const undoRef = React.useRef<() => void>(() => {});
  const applyRef = React.useRef<() => void>(() => {});
  const undoAction: ToastAction = { label: "Undo", run: () => undoRef.current() };

  useEffect(() => {
    let disposed = false;
    setBootstrapError(null);
    deckforgeApi.getInitialState().then(async (data) => {
      let draft: RecoveryDraft | null = null;
      let recoveryFailure: string | null = null;
      try { draft = await deckforgeApi.getProfileDraft(); }
      catch (error) { recoveryFailure = errorText(error); }
      if (disposed) return;
      setRecoveryDraft(draft); setRecoveryError(recoveryFailure); setRecoveryChecked(true);
      setDevices(data.devices);
      savedProfilesRef.current = data.profiles;
      const clean = initialClock();
      clockRef.current = clean;
      setClock(clean);
      setProfiles(data.profiles);
      setBootstrapReady(true);
      setStartupProfileId(data.startupProfileId ?? null);
      if (data.startupProfileId && data.profiles.some(p => p.id === data.startupProfileId)) {
        setSelectedProfileId(data.startupProfileId);
        setSelectedPageIdx(0);
      } else if (data.appliedContext) {
        setSelectedProfileId(data.appliedContext.profileId);
        setSelectedPageIdx(data.appliedContext.pageIndex);
      }
      setAutoSwitchRules(data.autoSwitchRules);
      setMacros(data.macros);
      setDiagnostics(data.diagnostics);
      setFrameStats(data.frameCacheStats);
      setLegacySummary(data.legacySummary);
      setTelemetry(data.telemetry);
      setMediaLibrary(Object.fromEntries((data.mediaLibrary || []).map((item) => [item.id, item])));
      if (data.face) setFace(data.face);
      if (data.obs) setObsSettings(data.obs);
    }).catch(error => { if (!disposed) setBootstrapError(errorText(error)); });

    let unlistenFn: (() => void) | null = null;
    deckforgeApi.onHardwareInput((evt) => {
      if (evt.kind === "unknown" || !evt.pressed) {
        if (evt.kind === "lcdKey") setLivePressedKeyIdx(null);
        return;
      }
      if (evt.kind === "lcdKey") {
        setLivePressedKeyIdx(evt.controlIndex);
        setSelectedKeyIdx(evt.controlIndex);
        triggerToast(
          `Physical Key #${evt.controlIndex + 1} Pressed (${evt.rawHex})`,
          "success"
        );
      } else if (
        evt.kind === "rotaryKnobTurnCw" ||
        evt.kind === "rotaryKnobTurnCcw" ||
        evt.kind === "rotaryKnobPush"
      ) {
        const kind =
          evt.kind === "rotaryKnobTurnCw" ? "cw" : evt.kind === "rotaryKnobTurnCcw" ? "ccw" : "push";
        setLiveEncoderEvent({ index: evt.controlIndex, kind });
        if (liveEncoderTimerRef.current !== null) window.clearTimeout(liveEncoderTimerRef.current);
        liveEncoderTimerRef.current = window.setTimeout(() => setLiveEncoderEvent(null), 350);
        triggerToast(
          `Physical Input: ${evt.kind} #${evt.controlIndex + 1} (${evt.rawHex})`,
          "info"
        );
      } else if (evt.kind === "touchStripTap") {
        setLiveTouchZoneIdx(evt.controlIndex);
        if (liveZoneTimerRef.current !== null) window.clearTimeout(liveZoneTimerRef.current);
        liveZoneTimerRef.current = window.setTimeout(() => setLiveTouchZoneIdx(null), 350);
        triggerToast(
          `Physical Input: ${evt.kind} #${evt.controlIndex + 1} (${evt.rawHex})`,
          "info"
        );
      } else {
        triggerToast(
          `Physical Input: ${evt.kind} #${evt.controlIndex + 1} (${evt.rawHex})`,
          "info"
        );
      }
    }).then((fn) => {
      if (disposed) fn(); else unlistenFn = fn;
    });
    let faceUnlisten: (() => void) | null = null;
    let contextUnlisten: (() => void) | null = null;
    deckforgeApi.onDeckFace((next) => setFace(next)).then((fn) => {
      if (disposed) fn(); else faceUnlisten = fn;
    });
    deckforgeApi.onDeckContext((ctx) => {
      setSelectedProfileId(ctx.profileId);
      setSelectedPageIdx(ctx.pageIndex);
      triggerToast(ctx.message, "info");
    }).then((fn) => {
      if (disposed) fn(); else contextUnlisten = fn;
    });

    const stopTelemetry = startPolling(() => deckforgeApi.getTelemetry(), setTelemetry, 5000);
    return () => {
      disposed = true;
      stopTelemetry();
      if (unlistenFn) unlistenFn();
      if (faceUnlisten) faceUnlisten();
      if (contextUnlisten) contextUnlisten();
    };
  }, [bootstrapAttempt]);

  useEffect(() => {
    // The desktop app downloads new releases by itself and installs them when it quits.
    return onUpdateReady((version) =>
      triggerToast(`DeckForge ${version} is downloaded. It installs when you quit DeckForge.`, "info")
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let disposed = false;
    let stop: (() => void) | undefined;
    deckforgeApi.onQuitRequested(() => { setQuitError(null); setQuitOpen(true); })
      .then(fn => { if (disposed) fn(); else stop = fn; });
    return () => { disposed = true; stop?.(); };
  }, []);

  useEffect(() => {
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (!savedLocally || profilePersistence.isPending() || Object.values(saveQueues).some(q => q.isPending())) {
        event.preventDefault(); event.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", beforeUnload);
    return () => window.removeEventListener("beforeunload", beforeUnload);
  }, [savedLocally, profilePersistence, saveQueues]);

  useEffect(() => {
    draftWriter.resume();
    return () => draftWriter.pause();
  }, [draftWriter]);

  useEffect(() => {
    if (!bootstrapReady || !recoveryChecked || recoveryDraft || recoveryError || quitting) return;
    draftWriter.schedule(savedLocally ? null : { profiles, baseProfiles: savedProfilesRef.current });
  }, [profiles, savedLocally, bootstrapReady, recoveryChecked, recoveryDraft, recoveryError, quitting, draftWriter]);

  const resolveRecovery = async (restore: boolean) => {
    setRecoveryBusy(true);
    try {
      if (restore && recoveryDraft) {
        const step = beginEdit(clockRef.current);
        clockRef.current = step.clock;
        setClock(step.clock);
        setUndoStack([{ revision: step.undoRevision, state: captureLibrary(latestProfiles.current) }]); setRedoStack([]);
        setProfiles(recoveryDraft.draft.profiles);
        setSelectedProfileId(recoveryDraft.draft.profiles[0].id);
        setSelectedPageIdx(0); setSelectedKeyIdx(0);
      } else if (!await draftWriter.clear()) {
        throw new Error("Could not discard the stored draft. Retry; the recovery copy is still available.");
      }
      setRecoveryDraft(null); setRecoveryError(null);
    } catch (error) { setRecoveryError(errorText(error)); }
    finally { setRecoveryBusy(false); }
  };

  const finishQuit = async (save: boolean) => {
    setQuitting(true); setQuitError(null); draftWriter.pause();
    try {
      if (save && (!recoveryChecked || recoveryDraft || recoveryError)) throw new Error("Resolve draft recovery before saving and quitting.");
      if (save) {
        if (!bootstrapReady) throw new Error("Wait for your profiles to load before saving.");
        await saveEditorProfiles(latestProfiles.current);
        for (const queue of Object.values(saveQueues)) queue.retry();
        const results = await Promise.all(Object.values(saveQueues).map(queue => queue.flush()));
        if (results.some(ok => !ok) || Object.values(saveQueues).some(q => q.isPending())) {
          throw new Error("Some settings could not be saved. Retry, or cancel to keep editing.");
        }
      } else {
        // Let writes already in progress settle before stopping the process.
        await Promise.all(Object.values(saveQueues).map(queue => queue.flush()));
      }
      await profilePersistence.flush();
      if (save && (profilePersistence.savedSnapshot() !== profileSnapshot(latestProfiles.current)
        || Object.values(saveQueues).some(q => q.isPending()))) {
        throw new Error("Edits changed while saving. Review them and try again.");
      }
      if (!await draftWriter.clear()) throw new Error("Could not clear the recovery draft. Retry quitting, or cancel to keep editing.");
      await deckforgeApi.quitApplication();
    } catch (error) { setQuitError(errorText(error)); }
    finally { draftWriter.resume(); setQuitting(false); }
  };

  const handleRescanDevices = () => {
    deckforgeApi.refreshUsbDevices().then(setDevices);
    triggerToast("USB HID discovery refreshed", "info");
  };

  const handleSelectDevice = (deviceId: string) => {
    setSelectedDeviceId(deviceId);
    setSelectedKeyIdx(0);
    setSurface("key");
    setSurfaceIndex(0);
  };

  const handleImportLegacyProfiles = async () => {
    // Import into the latest editor snapshot, without discarding local edits.
    const imported = await deckforgeApi.importLegacySdProfiles();
    const current = latestProfiles.current;
    const updatedProfiles = [...current, ...imported.filter(p => !current.some(existing => existing.id === p.id))];
    if (updatedProfiles.length !== current.length) {
      const step = beginEdit(clockRef.current);
      clockRef.current = step.clock;
      setClock(step.clock);
      setProfiles(updatedProfiles);
    }
    const firstImported = updatedProfiles.find((p) => p.id.startsWith("imported-"));
    if (firstImported) {
      setSelectedProfileId(firstImported.id);
      setSelectedPageIdx(0);
      setSelectedKeyIdx(0);
    }
    triggerToast(
      `Imported ${updatedProfiles.filter((p) => p.id.startsWith("imported-")).length} legacy .sdProfile configurations with Win32 hotkeys!`,
      "success"
    );
  };

  const [hotkeyTarget, setHotkeyTarget] = useState<HotkeyCapture | null>(null);
  const [customActions, setCustomActions] = useState<ActionTemplate[]>([]);

  const [plugins, setPlugins] = useState<PluginView[]>([]);
  useEffect(() => {
    deckforgeApi.pluginList().then(setPlugins).catch(() => setPlugins([]));
  }, []);
  const pluginActions = useMemo(() => pluginActionTemplates(plugins), [plugins]);

  const allActionTemplates = useMemo(
    () => [...customActions, ...ACTION_CATALOG, ...pluginActions],
    [customActions, pluginActions]
  );

  // One listener for the life of the editor. Live face ticks must not remove and
  // re-add it; the handler reads the latest editor through editorRef.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const editor = editorRef.current;
      if (!editor) return;
      if (document.querySelector("dialog[open]")) return;
      if (!editor.recoveryChecked || editor.recoveryDraft || editor.recoveryError || editor.quitOpen || editor.showOnboarding) return;
      if (editor.hotkeyTarget) {
        e.preventDefault();
        e.stopPropagation();
        if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) {
          return;
        }
        const parts: string[] = [];
        if (e.ctrlKey) parts.push("Ctrl");
        if (e.shiftKey) parts.push("Shift");
        if (e.altKey) parts.push("Alt");
        if (e.metaKey) parts.push("Win");
        const keyMap: Record<string, string> = {
          " ": "Space",
          ArrowUp: "Up",
          ArrowDown: "Down",
          ArrowLeft: "Left",
          ArrowRight: "Right",
          Escape: "Esc",
        };
        const keyLabel = keyMap[e.key] || (e.key.length === 1 ? e.key.toUpperCase() : e.key);
        parts.push(keyLabel);
        const combo = parts.join("+");
        const { hotkeyTarget, activeKey, activeZone, activeEncoder } = editor;
        if (hotkeyTarget === "key") {
          editor.updateSelectedKey({
            subtitle: combo,
            actionParam: combo,
            actionId: activeKey?.actionId === "none" || activeKey?.actionId === "custom.hotkey" ? "custom.hotkey" : activeKey?.actionId,
            actionLabel:
              activeKey?.actionId === "none" || activeKey?.actionId === "custom.hotkey"
                ? "Custom: Keyboard Shortcut / Hotkey"
                : activeKey?.actionLabel,
          });
        } else if ((hotkeyTarget === "double" || hotkeyTarget === "hold") && activeKey) {
          const which = hotkeyTarget === "double" ? "doublePress" : "hold";
          const prev = activeKey[which];
          editor.updateSelectedKey({
            [which]: {
              actionId: prev?.actionId === "sys.hotkey" ? "sys.hotkey" : "custom.hotkey",
              actionLabel: prev?.actionLabel || "Custom: Keyboard Shortcut / Hotkey",
              actionParam: combo,
            },
          });
        } else if (hotkeyTarget === "zone" && activeZone) {
          editor.updateSelectedZone(activeZone.index, { tapActionId: "custom.hotkey", tapActionParam: combo });
        } else if (activeEncoder && (hotkeyTarget === "cw" || hotkeyTarget === "ccw" || hotkeyTarget === "push")) {
          const patch =
            hotkeyTarget === "cw"
              ? { cwAction: "custom.hotkey", cwParam: combo }
              : hotkeyTarget === "ccw"
              ? { ccwAction: "custom.hotkey", ccwParam: combo }
              : { pushAction: "custom.hotkey", pushParam: combo };
          editor.updateSelectedEncoder(activeEncoder.index, patch);
        }
        editor.setHotkeyTarget(null);
        editor.triggerToast(`Recorded shortcut: ${combo}`, "success");
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        editor.handleSaveLocally();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
        e.preventDefault();
        editor.handleUndo();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "y") {
        e.preventDefault();
        editor.handleRedo();
      } else if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
        e.preventDefault();
        void editor.handleApplyToDevice();
      } else if ((e.key === "PageUp" || e.key === "PageDown") && editor.activeScreen === "workspace" && editor.activeProfile) {
        const el = e.target as HTMLElement | null;
        if (el && (el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName))) return;
        e.preventDefault();
        const last = editor.activeProfile.pages.length - 1;
        editor.setSelectedPageIdx((p) => (e.key === "PageUp" ? Math.max(0, p - 1) : Math.min(last, p + 1)));
      }
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, []);

  const connectedDevices = useMemo(() => devices.filter((d) => d.connected), [devices]);

  // The list is read once at start. A deck that was still rebooting then would stay "not
  // connected" until someone pressed rescan, so keep looking while none is found.
  const hasConnectedDevice = connectedDevices.length > 0;
  useEffect(() => {
    if (!isTauriRuntime() || hasConnectedDevice) return;
    const timer = window.setInterval(() => {
      deckforgeApi.refreshUsbDevices().then(setDevices).catch(() => {});
    }, 4000);
    return () => window.clearInterval(timer);
  }, [hasConnectedDevice]);

  const activeDevice = useMemo(
    () =>
      connectedDevices.find((d) => d.id === selectedDeviceId) ||
      connectedDevices[0] ||
      devices.find((d) => d.id === selectedDeviceId) ||
      devices[0],
    [connectedDevices, devices, selectedDeviceId]
  );

  const activeProfile = useMemo(
    () => profiles.find((p) => p.id === selectedProfileId) || profiles[0],
    [profiles, selectedProfileId]
  );

  const activePage = useMemo(() => {
    if (!activeProfile) return null;
    return activeProfile.pages[selectedPageIdx] || activeProfile.pages[0] || null;
  }, [activeProfile, selectedPageIdx]);

  const activeKey = useMemo(() => {
    if (!activePage) return null;
    return activePage.keys[selectedKeyIdx] || activePage.keys[0] || null;
  }, [activePage, selectedKeyIdx]);

  const paramOptionsFor = (actionId: string) => {
    if (actionId === "profile.switch_page" && activeProfile) {
      return activeProfile.pages.map((page) => ({ value: page.name, label: page.name }));
    }
    if (actionId === "profile.switch") {
      return profiles.map((profile) => ({ value: profile.id, label: profile.name }));
    }
    if (actionId === "macro.run") {
      return macros.map((macro) => ({ value: macro.id, label: macro.name }));
    }
    if (actionId === "audio.cycle_output") {
      return outputParamOptions(audioOutputs);
    }
    return undefined;
  };

  const saveRules = (rules: AutoSwitchRule[]) => {
    setAutoSwitchRules(rules);
    saveQueues["Auto-switch rules"].save(rules);
  };

  const saveObs = (settings: ObsSettings) => {
    setObsSettings(settings);
    saveQueues["OBS settings"].save(settings);
  };

  const persistMacros = (next: MacroSequence[]) => {
    setMacros(next);
    saveQueues.Macros.save(next);
  };

  const assignMacroToKey = async (macro: MacroSequence) => {
    if (!activeProfile || !activePage || !activeKey) {
      triggerToast("Select a key on the deck first", "warning");
      return;
    }
    pushUndoSnapshot();
    const nextProfiles = profiles.map((prof) => {
      if (prof.id !== activeProfile.id) return prof;
      return {
        ...prof,
        lastModifiedMs: Date.now(),
        pages: prof.pages.map((page, pageIndex) => {
          if (pageIndex !== selectedPageIdx) return page;
          return {
            ...page,
            keys: page.keys.map((key) =>
              key.index === activeKey.index
                ? {
                    ...key,
                    title: macro.name,
                    subtitle: "MACRO",
                    iconName: "Zap",
                    actionId: "macro.run",
                    actionLabel: `Macro: ${macro.name}`,
                    actionCategory: "Macro",
                    actionParam: macro.id,
                    accentColor: "#8B5CF6",
                    bgColor: "#231B38",
                  }
                : key
            ),
          };
        }),
      };
    });
    setProfiles(nextProfiles);
    saveQueues.Macros.save(macros);
    if (!await saveQueues.Macros.flush()) return;
    try {
      await saveEditorProfiles(nextProfiles);
      triggerToast(`${macro.name} is on key ${activeKey.index + 1}`, "success");
    } catch (error) {
      triggerToast(`Key assignment was not saved: ${errorText(error)}. Try Save again.`, "warning");
    }
  };

  const activeZone =
    activePage?.touchZones.find((zone) => zone.index === surfaceIndex) ||
    activePage?.touchZones[0] ||
    null;
  const activeEncoder =
    activePage?.encoders.find((enc) => enc.index === surfaceIndex) ||
    activePage?.encoders[0] ||
    null;

  const pushUndoSnapshot = () => {
    const step = beginEdit(clockRef.current);
    commitClock(step.clock);
    setUndoStack((prev) => pushEntry(prev, { revision: step.undoRevision, state: captureLibrary(profiles) }));
    setRedoStack([]);
  };

  const syncHardwareKnobLedsAfterHistory = (nextProfiles: Profile[]) => {
    if (!activeDevice || !activeProfile || !activePage) return;
    const nextPage = nextProfiles.find((p) => p.id === activeProfile.id)?.pages[selectedPageIdx];
    if (!nextPage) return;
    const rawBefore = [0, 1, 2, 3].map((i) => activePage.encoders.find((e) => e.index === i)?.ledColor?.trim() ?? "");
    const rawAfter = [0, 1, 2, 3].map((i) => nextPage.encoders.find((e) => e.index === i)?.ledColor?.trim() ?? "");
    if (rawBefore.join("|") === rawAfter.join("|")) return;
    if (rawAfter.some((c) => c !== "")) {
      const resolved = rawAfter.map((c, i) => (c || DEFAULT_KNOB_LED_COLORS[i] || "#35C98B").toUpperCase());
      void flushKnobLedSend(activeDevice.id, resolved);
    } else {
      void deckforgeApi.resetKnobLeds(activeDevice.id).catch(() => {});
    }
  };

  const handleUndo = () => {
    const entry = undoStack[undoStack.length - 1];
    if (!entry) return;
    const redo = { revision: clockRef.current.current, state: captureLibrary(profiles) };
    commitClock(noteRestored(clockRef.current, entry.revision));
    lastKnobUndoRef.current = null;
    setRedoStack((r) => [...r, redo]);
    setProfiles(entry.state);
    setUndoStack((u) => u.slice(0, -1));
    syncHardwareKnobLedsAfterHistory(entry.state);
    triggerToast("Undid last key edit", "info");
  };

  undoRef.current = handleUndo;

  const handleRedo = () => {
    const entry = redoStack[redoStack.length - 1];
    if (!entry) return;
    const undo = { revision: clockRef.current.current, state: captureLibrary(profiles) };
    commitClock(noteRestored(clockRef.current, entry.revision));
    lastKnobUndoRef.current = null;
    setUndoStack((u) => pushEntry(u, undo));
    setProfiles(entry.state);
    setRedoStack((r) => r.slice(0, -1));
    syncHardwareKnobLedsAfterHistory(entry.state);
    triggerToast("Redid key edit", "info");
  };

  const updateSelectedKey = (patch: Partial<KeyConfig>) => {
    if (!activeProfile || !activePage || !activeKey) return;
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) => {
        if (prof.id !== activeProfile.id) return prof;
        return {
          ...prof,
          lastModifiedMs: Date.now(),
          pages: prof.pages.map((pg, pIdx) => {
            if (pIdx !== selectedPageIdx) return pg;
            return {
              ...pg,
              keys: pg.keys.map((k) => (k.index === activeKey.index ? { ...k, ...patch } : k)),
            };
          }),
        };
      })
    );
  };

  const patchKeys = (change: (keys: KeyConfig[]) => KeyConfig[]) => {
    if (!activeProfile || !activePage) return;
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) =>
        prof.id !== activeProfile.id
          ? prof
          : {
              ...prof,
              lastModifiedMs: Date.now(),
              pages: prof.pages.map((pg, pIdx) => (pIdx === selectedPageIdx ? { ...pg, keys: change(pg.keys) } : pg)),
            }
      )
    );
  };

  /** Moves a key to another page of this profile: the same slot if it is empty there, else the first empty one. */
  const handleMoveKeyToPage = (keyIndex: number, pageIndex: number) => {
    if (!activeProfile || pageIndex === selectedPageIdx) return;
    const source = activeProfile.pages[selectedPageIdx]?.keys.find((k) => k.index === keyIndex);
    const target = activeProfile.pages[pageIndex];
    if (!source || !target) return;
    const gestureSet = (gesture?: { actionId: string } | null) => !!gesture && gesture.actionId !== "" && gesture.actionId !== "none";
    const isEmpty = (k: KeyConfig) => k.actionId === "none" && !k.customImageDataUrl && !gestureSet(k.doublePress) && !gestureSet(k.hold);
    const slot = target.keys.find((k) => k.index === keyIndex && isEmpty(k)) ?? target.keys.find(isEmpty);
    if (!slot) {
      triggerToast(`${target.name} has no empty key. Clear one there first.`, "warning");
      return;
    }
    const blank: KeyConfig = {
      ...source,
      title: "Empty",
      subtitle: "",
      iconName: "Plus",
      actionId: "none",
      actionLabel: "No Action Assigned",
      actionCategory: "System",
      customImageDataUrl: undefined,
      actionParam: "",
      doublePress: null,
      hold: null,
      accentColor: "#394158",
      bgColor: "#151925",
    };
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) =>
        prof.id !== activeProfile.id
          ? prof
          : {
              ...prof,
              lastModifiedMs: Date.now(),
              pages: prof.pages.map((pg, pIdx) =>
                pIdx === selectedPageIdx
                  ? { ...pg, keys: pg.keys.map((k) => (k.index === keyIndex ? blank : k)) }
                  : pIdx === pageIndex
                  ? { ...pg, keys: pg.keys.map((k) => (k.index === slot.index ? { ...source, index: slot.index } : k)) }
                  : pg
              ),
            }
      )
    );
    triggerToast(`${source.title} moved to ${target.name}, key ${slot.index + 1}`, "info", undoAction);
  };

  /** Dropping an action card on a key binds that key, whichever key is selected. */
  const handleDropAction = (target: "key" | "zone" | "encoder", index: number, actionId: string, part: KnobPart = "push") => {
    const act = allActionTemplates.find((a) => a.id === actionId);
    if (!act) return;
    if (target === "zone") {
      updateSelectedZone(index, zonePatchForAction(act));
      triggerToast(`${act.title} is on touch zone ${index + 1}`, "info", undoAction);
    } else if (target === "encoder") {
      updateSelectedEncoder(index, encoderPatchForAction(act, part));
      const where = part === "cw" ? "turn right" : part === "ccw" ? "turn left" : "press";
      triggerToast(`${act.title} is on knob ${index + 1} ${where}`, "info", undoAction);
    } else {
      patchKeys((keys) => keys.map((k) => (k.index === index ? { ...k, ...keyPatchForAction(act) } : k)));
      triggerToast(`${act.title} is on key ${index + 1}`, "info", undoAction);
    }
  };

  /** Dragging one key onto another swaps them, so an empty slot receives the key. */
  const handleMoveKey = (from: number, to: number) => {
    if (from === to || Number.isNaN(from)) return;
    patchKeys((keys) => {
      const a = keys.find((k) => k.index === from);
      const b = keys.find((k) => k.index === to);
      if (!a || !b) return keys;
      return keys.map((k) => (k.index === from ? { ...b, index: from } : k.index === to ? { ...a, index: to } : k));
    });
  };

  const clipFor = (cfg?: ScreenMediaConfig | null) => (cfg ? mediaLibrary[cfg.mediaId] ?? null : null);

  const assignMedia = (target: ScreenTarget, index: number, cfg: ScreenMediaConfig | null) => {
    if (!activeProfile || !activePage) return;
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) => {
        if (prof.id !== activeProfile.id) return prof;
        return {
          ...prof,
          lastModifiedMs: Date.now(),
          pages: prof.pages.map((pg, pIdx) => {
            if (pIdx !== selectedPageIdx) return pg;
            if (target === "strip") return { ...pg, stripMedia: cfg };
            if (target === "zone") {
              return {
                ...pg,
                touchZones: pg.touchZones.map((zone) =>
                  zone.index === index ? { ...zone, media: cfg } : zone
                ),
              };
            }
            return {
              ...pg,
              keys: pg.keys.map((key) =>
                key.index === index
                  ? {
                      ...key,
                      media: cfg,
                      backgroundStyle: cfg
                        ? "Image"
                        : key.backgroundStyle === "Image"
                        ? "Gradient"
                        : key.backgroundStyle,
                    }
                  : key
              ),
            };
          }),
        };
      })
    );
    setSurface(target);
    setSurfaceIndex(index);
    if (target === "key") setSelectedKeyIdx(index);
  };

  const currentMedia = (): ScreenMediaConfig | null => {
    if (!activePage) return null;
    if (surface === "zone") return activePage.touchZones.find((zone) => zone.index === surfaceIndex)?.media ?? null;
    if (surface === "strip") return activePage.stripMedia ?? null;
    return activeKey?.media ?? null;
  };

  const patchCurrentMedia = (patch: Partial<ScreenMediaConfig>) => {
    const current = currentMedia();
    if (!current || surface === "encoder") return;
    const index = surface === "key" ? activeKey?.index ?? selectedKeyIdx : surfaceIndex;
    assignMedia(surface, index, { ...current, ...patch });
  };

  const ingestFile = async (file: File, target: ScreenTarget, index: number) => {
    if (!file.type.startsWith("image/") && !/\.(gif|png|jpe?g|webp|bmp)$/i.test(file.name)) {
      triggerToast("Use a PNG, JPEG, WebP, or GIF", "warning");
      return;
    }
    if (file.size > 8 * 1024 * 1024) {
      triggerToast("Image must be 8 MB or smaller", "warning");
      return;
    }
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const preview = await deckforgeApi.stageScreenMedia(
        `media-${Date.now()}`,
        file.name,
        bytesToBase64(bytes)
      );
      setMediaLibrary((prev) => ({ ...prev, [preview.id]: preview }));
      assignMedia(target, index, {
        mediaId: preview.id,
        fit: "cover",
        loopPlayback: true,
        speed: 1,
      });
      triggerToast(
        `${preview.fileName} fitted · ${preview.frameCount} frame${preview.frameCount === 1 ? "" : "s"}`,
        "success"
      );
    } catch (err) {
      triggerToast(errorText(err), "warning");
    }
  };

  /** Stages animated frames for several keys and puts them on the page in one undo step. */
  const applyKeyFrames = async (items: KeyFrames[], label: string) => {
    if (!activeProfile || !activePage || items.length === 0) return;
    const staged: Array<{ index: number; preview: MediaPreview }> = [];
    try {
      for (const item of items) {
        const preview = await deckforgeApi.stageFrameSequence(`theme-${Date.now()}-${item.index}`, item.name, item.frames, item.delayMs);
        staged.push({ index: item.index, preview });
      }
    } catch (err) {
      triggerToast(errorText(err), "warning");
      return;
    }
    setMediaLibrary((prev) => ({ ...prev, ...Object.fromEntries(staged.map((s) => [s.preview.id, s.preview])) }));
    const byIndex = new Map(staged.map((s) => [s.index, s.preview.id]));
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) =>
        prof.id !== activeProfile.id
          ? prof
          : {
              ...prof,
              lastModifiedMs: Date.now(),
              pages: prof.pages.map((pg, pIdx) =>
                pIdx !== selectedPageIdx
                  ? pg
                  : {
                      ...pg,
                      keys: pg.keys.map((key) => {
                        const mediaId = byIndex.get(key.index);
                        return mediaId
                          ? { ...key, media: { mediaId, fit: "cover", loopPlayback: true, speed: 1 }, backgroundStyle: "Image" }
                          : key;
                      }),
                    }
              ),
            }
      )
    );
    triggerToast(`${label} added to ${staged.length} key${staged.length === 1 ? "" : "s"}. Apply to Device to see it.`, "success", {
      label: "Apply now",
      run: () => applyRef.current(),
    });
  };

  const useSampleAnimation = async (target: ScreenTarget, index: number) => {
    try {
      const frames = buildPulseFrames();
      const preview = await deckforgeApi.stageFrameSequence(
        `pulse-${Date.now()}`,
        "Pulse.gif",
        frames.map((frame) => frame.split(",").pop() || ""),
        90
      );
      setMediaLibrary((prev) => ({ ...prev, [preview.id]: preview }));
      assignMedia(target, index, {
        mediaId: preview.id,
        fit: "cover",
        loopPlayback: true,
        speed: 1,
      });
      triggerToast("Sample pulse animation added. Apply to Device to play it on the screens.", "success");
    } catch (err) {
      triggerToast(errorText(err), "warning");
    }
  };

  const updateSelectedZone = (index: number, patch: Partial<TouchZoneConfig>) => {
    if (!activeProfile) return;
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) => {
        if (prof.id !== activeProfile.id) return prof;
        return {
          ...prof,
          pages: prof.pages.map((pg, pIdx) => {
            if (pIdx !== selectedPageIdx) return pg;
            return {
              ...pg,
              touchZones: pg.touchZones.map((zone) =>
                zone.index === index ? { ...zone, ...patch } : zone
              ),
            };
          }),
        };
      })
    );
  };

  const updateSelectedEncoder = (index: number, patch: Partial<EncoderConfig>) => {
    if (!activeProfile) return;
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) => {
        if (prof.id !== activeProfile.id) return prof;
        return {
          ...prof,
          pages: prof.pages.map((pg, pIdx) => {
            if (pIdx !== selectedPageIdx) return pg;
            return {
              ...pg,
              encoders: pg.encoders.map((enc) => (enc.index === index ? { ...enc, ...patch } : enc)),
            };
          }),
        };
      })
    );
  };

  const handleSaveLocally = async () => {
    try {
      await saveEditorProfiles(profiles);
      triggerToast("Profile saved locally ✓", "success");
    } catch (error) {
      triggerToast(`Profile was not saved: ${errorText(error)}. Your edits are still here; try Save again.`, "warning");
    }
  };

  const handleApplyToDevice = async () => {
    if (!activeDevice || !activeProfile || !activePage) return;
    let res;
    try {
      await saveEditorProfiles(profiles);
      res = await deckforgeApi.applyProfileToDevice(
        activeDevice.id,
        activeProfile.id,
        activePage.id
      );
    } catch (err) {
      setAppliedRefresh(n => n + 1);
      triggerToast(`Could not send to the deck: ${errorText(err)}`, "warning", { label: "Retry", run: () => applyRef.current() });
      return;
    }
    setAppliedRefresh(n => n + 1);
    setFrameStats((prev) => ({
      ...prev,
      framesEvaluated: prev.framesEvaluated + res.keysEvaluated,
      framesTransmitted: prev.framesTransmitted + res.keysTransmitted,
      framesDeduplicatedSkipped: prev.framesDeduplicatedSkipped + res.keysDeduplicatedSkipped,
      bytesTransmitted: prev.bytesTransmitted + res.bytesTransmitted,
      lastTransferLatencyMs: res.elapsedMs,
    }));
    triggerToast(res.statusMessage, "success");
  };

  applyRef.current = () => void handleApplyToDevice();

  const handleTestSelectedAction = async () => {
    if (!activeKey) return;
    try {
      const res = await deckforgeApi.testKeyAction(activeKey.actionId, activeKey.actionLabel, activeKey.actionParam);
      triggerToast(`${res.outputMessage} (${res.elapsedMs}ms)`, res.success ? "success" : "warning");
    } catch (error) {
      triggerToast(`Could not test the action: ${errorText(error)}`, "warning");
    }
  };

  const handleClearSelectedKey = () => {
    if (!activeKey) return;
    updateSelectedKey({
      title: "Empty",
      subtitle: "",
      iconName: "Plus",
      actionId: "none",
      actionLabel: "No Action Assigned",
      actionCategory: "System",
      customImageDataUrl: undefined,
      actionParam: "",
      doublePress: null,
      hold: null,
      accentColor: "#394158",
      bgColor: "#151925",
      media: null,
      backgroundStyle: "Gradient",
    });
    triggerToast(`Cleared key #${activeKey.index + 1} in the editor. Apply to Device to update its image and action together.`, "info");
  };

  const handleBrightnessChange = async (percent: number) => {
    if (!activeDevice) return;
    setDevices((prev) =>
      prev.map((d) => (d.id === activeDevice.id ? { ...d, brightness: percent } : d))
    );
    const msg = await deckforgeApi.setDeviceBrightness(activeDevice.id, percent);
    triggerToast(msg, "info");
  };

  const knobLedWriteInFlight = useRef(false);
  const queuedKnobLedSend = useRef<{ deviceId: string; colors: string[] } | null>(null);
  const lastKnobUndoRef = useRef<{ target: number | "all"; atMs: number } | null>(null);

  const flushKnobLedSend = async (deviceId: string, colors: string[]) => {
    queuedKnobLedSend.current = { deviceId, colors };
    if (knobLedWriteInFlight.current) return;
    knobLedWriteInFlight.current = true;
    try {
      while (queuedKnobLedSend.current) {
        const next = queuedKnobLedSend.current;
        queuedKnobLedSend.current = null;
        await deckforgeApi.setKnobLeds(next.deviceId, next.colors);
      }
    } catch (err) {
      triggerToast(`Could not update knob LEDs: ${errorText(err)}`, "warning");
    } finally {
      knobLedWriteInFlight.current = false;
    }
  };

  const handleKnobLedChange = (knobIndex: number | "all", colorHex: string) => {
    if (!activeProfile || !activePage || !activeDevice) return;
    const normalized = colorHex.toUpperCase();
    const updatedEncoders = activePage.encoders.map((enc) =>
      knobIndex === "all" || enc.index === knobIndex ? { ...enc, ledColor: normalized } : enc
    );
    const now = Date.now();
    const last = lastKnobUndoRef.current;
    if (!last || last.target !== knobIndex || now - last.atMs > 600) {
      pushUndoSnapshot();
    }
    lastKnobUndoRef.current = { target: knobIndex, atMs: now };
    setProfiles((prev) =>
      prev.map((prof) =>
        prof.id !== activeProfile.id
          ? prof
          : {
              ...prof,
              lastModifiedMs: Date.now(),
              pages: prof.pages.map((pg, pIdx) =>
                pIdx !== selectedPageIdx ? pg : { ...pg, encoders: updatedEncoders }
              ),
            }
      )
    );
    const colors = [0, 1, 2, 3].map((i) => {
      const enc = updatedEncoders.find((e) => e.index === i);
      return (enc?.ledColor || DEFAULT_KNOB_LED_COLORS[i] || "#35C98B").toUpperCase();
    });
    void flushKnobLedSend(activeDevice.id, colors);
  };

  const handleResetKnobLeds = async () => {
    if (!activeProfile || !activePage || !activeDevice) return;
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) =>
        prof.id !== activeProfile.id
          ? prof
          : {
              ...prof,
              lastModifiedMs: Date.now(),
              pages: prof.pages.map((pg, pIdx) =>
                pIdx !== selectedPageIdx
                  ? pg
                  : {
                      ...pg,
                      encoders: pg.encoders.map((enc) => ({ ...enc, ledColor: "" })),
                    }
              ),
            }
      )
    );
    try {
      const msg = await deckforgeApi.resetKnobLeds(activeDevice.id);
      triggerToast(msg, "info");
    } catch (err) {
      triggerToast(`Could not reset knob LEDs: ${errorText(err)}`, "warning");
    }
  };

  const handleApplyKnobLedPalette = (colors: readonly string[], label: string) => {
    if (!activeProfile || !activePage || !activeDevice) return;
    const normalized = [0, 1, 2, 3].map((i) => (colors[i] || DEFAULT_KNOB_LED_COLORS[i] || "#35C98B").toUpperCase());
    lastKnobUndoRef.current = null;
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) =>
        prof.id !== activeProfile.id
          ? prof
          : {
              ...prof,
              lastModifiedMs: Date.now(),
              pages: prof.pages.map((pg, pIdx) =>
                pIdx !== selectedPageIdx
                  ? pg
                  : {
                      ...pg,
                      encoders: pg.encoders.map((enc) => ({
                        ...enc,
                        ledColor: normalized[enc.index] ?? normalized[0],
                      })),
                    }
              ),
            }
      )
    );
    void flushKnobLedSend(activeDevice.id, normalized);
    triggerToast(`Applied ${label} knob LED palette`, "info");
  };

  const handleKnobLedEffectChange = async (effect: KnobLedEffect) => {
    if (!activeProfile || !activePage || !activeDevice) return;
    lastKnobUndoRef.current = null;
    pushUndoSnapshot();
    const stored = effect === "static" ? undefined : effect;
    setProfiles((prev) =>
      prev.map((prof) =>
        prof.id !== activeProfile.id
          ? prof
          : {
              ...prof,
              lastModifiedMs: Date.now(),
              pages: prof.pages.map((pg, pIdx) =>
                pIdx !== selectedPageIdx ? pg : { ...pg, knobLedEffect: stored }
              ),
            }
      )
    );
    const colors = [0, 1, 2, 3].map((i) => {
      const enc = activePage.encoders.find((e) => e.index === i);
      return (enc?.ledColor || DEFAULT_KNOB_LED_COLORS[i] || "#35C98B").toUpperCase();
    });
    try {
      const msg = await deckforgeApi.setKnobLedEffect(activeDevice.id, effect, colors);
      triggerToast(msg, "info");
    } catch (err) {
      triggerToast(`Could not set knob LED effect: ${errorText(err)}`, "warning");
    }
  };

  const handleApplyKnobLedsToAllPages = () => {
    if (!activeProfile || !activePage) return;
    const byIndex = new Map(activePage.encoders.map((enc) => [enc.index, enc.ledColor ?? ""]));
    lastKnobUndoRef.current = null;
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) =>
        prof.id !== activeProfile.id
          ? prof
          : {
              ...prof,
              lastModifiedMs: Date.now(),
              pages: prof.pages.map((pg) => ({
                ...pg,
                encoders: pg.encoders.map((enc) => ({
                  ...enc,
                  ledColor: byIndex.get(enc.index) ?? enc.ledColor ?? "",
                })),
              })),
            }
      )
    );
    triggerToast(`Copied knob LED colors to all ${activeProfile.pages.length} pages in ${activeProfile.name}`, "success");
  };

  const [ledBrightness, setLedBrightness] = useState(100);
  const [ledFollowSleep, setLedFollowSleep] = useState(true);

  const handleLedBrightnessChange = async (percent: number) => {
    if (!activeDevice) return;
    setLedBrightness(percent);
    try {
      await deckforgeApi.setLedBrightness(activeDevice.id, percent);
    } catch (err) {
      triggerToast(`Could not set LED brightness: ${errorText(err)}`, "warning");
    }
  };

  const handleLedSleepModeChange = async (followSystemSleep: boolean) => {
    if (!activeDevice) return;
    setLedFollowSleep(followSystemSleep);
    try {
      const msg = await deckforgeApi.setLedSleepMode(activeDevice.id, followSystemSleep);
      triggerToast(msg, "info");
    } catch (err) {
      triggerToast(`Could not set LED sleep mode: ${errorText(err)}`, "warning");
    }
  };

  const handleRestartDeck = async () => {
    triggerToast("Restarting the deck. Approve the Windows permission prompt if it appears.", "info");
    try {
      triggerToast(await deckforgeApi.restartDeck(), "success");
    } catch (err) {
      triggerToast(`Could not restart the deck: ${errorText(err)} Unplugging and replugging it does the same.`, "warning");
    }
  };

  const handleAddProfile = (starter: StarterProfile) => {
    const built = buildStarterProfile(starter, profiles, autoSwitchRules);
    const newProf: Profile = { ...built.profile, targetDeviceId: profiles[0]?.targetDeviceId ?? "" };
    pushUndoSnapshot();
    setProfiles((prev) => [...prev, newProf]);
    if (built.rules.length > 0) saveRules([...autoSwitchRules, ...built.rules]);
    setSelectedProfileId(newProf.id);
    setSelectedPageIdx(0);
    setSelectedKeyIdx(0);
    const apps = built.rules.length > 0 ? ` It opens with ${built.rules.length} app${built.rules.length === 1 ? "" : "s"}.` : "";
    const skipped = built.skippedApps.length > 0 ? ` ${built.skippedApps.join(", ")} already open another page, so they were left alone.` : "";
    triggerToast(`Created ${newProf.name}.${apps}${skipped}`, "success");
  };

  const handleBackup = () => deckforgeApi.exportProfilesBackup(latestProfiles.current, latestMacros.current);
  const handleRestoreBackup = async (contents: string) => {
    const restored = await deckforgeApi.restoreProfilesBackup(contents);
    const current = latestProfiles.current;
    const placed = placeBackup(restored.profiles, restored.macros, current, latestMacros.current);
    const step = beginEdit(clockRef.current);
    clockRef.current = step.clock;
    setClock(step.clock);
    setUndoStack(prev => pushEntry(prev, { revision: step.undoRevision, state: captureLibrary(current) }));
    setRedoStack([]);
    setMediaLibrary(prev => ({ ...prev, ...Object.fromEntries(restored.mediaLibrary.map(media => [media.id, media])) }));
    setProfiles([...current, ...placed.profiles]);
    if (placed.macros.length) persistMacros([...latestMacros.current, ...placed.macros]);
    setSelectedProfileId(placed.profiles[0].id);
    setSelectedPageIdx(0); setSelectedKeyIdx(0);
    return placed.profiles.length;
  };

  const handleExportProfile = async () => {
    if (!activeProfile) return;
    try {
      const used = new Set(macroIdsUsed(activeProfile));
      const where = await deckforgeApi.exportProfilePack(
        activeProfile,
        macros.filter(macro => used.has(macro.id)),
        autoSwitchRules.filter(rule => rule.targetProfileId === activeProfile.id && rule.processName.trim()),
      );
      triggerToast(`Saved portable profile "${activeProfile.name}" with its media to ${where}.`, "success");
    } catch (err) {
      triggerToast(`Could not export the profile: ${errorText(err)}`, "warning");
    }
  };

  const handleImportProfile = async () => {
    let picked: { name: string; text: string } | null = null;
    try {
      picked = await pickJsonFile();
      if (!picked) return;
      let kind: string | undefined;
      try { kind = JSON.parse(picked.text)?.kind; } catch { /* Legacy parser reports invalid JSON. */ }
      const restoredPack = kind === "deckforge.profile-pack" ? await deckforgeApi.restoreProfilePack(picked.text) : null;
      const parsed = restoredPack ? {
        profile: restoredPack.profiles[0],
        macros: restoredPack.macros,
        appRules: (restoredPack.appRules ?? []).map(rule => ({
          processName: rule.processName, windowTitleContains: rule.windowTitleContains,
          pageId: rule.targetPageId ?? "", enabled: rule.enabled,
        })),
      } : parseProfileFile(picked.text);
      const placed = placeImportedProfile(parsed, profiles, macros);
      const next = [...profiles, placed.profile];
      pushUndoSnapshot();
      if (restoredPack) setMediaLibrary(prev => ({ ...prev, ...Object.fromEntries(restoredPack.mediaLibrary.map(media => [media.id, media])) }));
      setProfiles(next);
      if (placed.addedMacros > 0) {
        persistMacros(placed.macros);
        if (!await saveQueues.Macros.flush()) {
          throw new Error("Imported macros were not saved. Retry the macro save, then save the profile; your edits are still here.");
        }
      }
      await saveEditorProfiles(next);
      if (parsed.appRules.length > 0) {
        const stamp = Date.now();
        saveRules([
          ...autoSwitchRules,
          ...parsed.appRules.map((rule, i) => ({
            id: `rule-${stamp}-${i}`,
            processName: rule.processName,
            windowTitleContains: rule.windowTitleContains,
            targetProfileId: placed.profile.id,
            targetPageId: rule.pageId,
            priority: autoSwitchRules.length + i + 1,
            enabled: rule.enabled,
          })),
        ]);
      }
      setSelectedProfileId(placed.profile.id);
      setSelectedPageIdx(0);
      setSelectedKeyIdx(0);
      const pages = placed.profile.pages.length;
      const extra = placed.addedMacros > 0 ? ` and ${placed.addedMacros} macro${placed.addedMacros === 1 ? "" : "s"}` : "";
      triggerToast(
        `Imported "${placed.profile.name}" (${pages} page${pages === 1 ? "" : "s"}${extra}). Press Apply to Device to send it to the deck.`,
        "success"
      );
    } catch (err) {
      triggerToast(`Could not import ${picked?.name ?? "file"}: ${errorText(err)}`, "warning");
    }
  };

  const handleAddPage = () => {
    if (!activeProfile || !activePage) return;
    const newPage = {
      ...JSON.parse(JSON.stringify(activePage)),
      id: `page-${activeProfile.pages.length + 1}-${Date.now()}`,
      name: `Page ${activeProfile.pages.length + 1}`,
    };
    setProfiles((prev) =>
      prev.map((p) =>
        p.id === activeProfile.id ? { ...p, pages: [...p.pages, newPage] } : p
      )
    );
    setSelectedPageIdx(activeProfile.pages.length);
    triggerToast(`Added ${newPage.name}`, "info");
  };

  const handleAddPageFromTemplate = (page: ProfilePage) => {
    if (!activeProfile) return;
    pushUndoSnapshot();
    const unique: ProfilePage = {
      ...page,
      id: `${page.id}-${Date.now()}`,
    };
    setProfiles((prev) =>
      prev.map((p) =>
        p.id === activeProfile.id ? { ...p, lastModifiedMs: Date.now(), pages: [...p.pages, unique] } : p
      )
    );
    setSelectedPageIdx(activeProfile.pages.length);
    setSelectedKeyIdx(0);
    setSurface("key");
    setTemplatesOpen(false);
    triggerToast(`Added "${unique.name}". Press Apply to Device to see it on the deck.`, "success");
  };

  const handleReplacePageFromTemplate = (page: ProfilePage) => {
    if (!activeProfile || !activePage) return;
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((p) =>
        p.id === activeProfile.id
          ? {
              ...p,
              lastModifiedMs: Date.now(),
              pages: p.pages.map((pg, idx) => (idx === selectedPageIdx ? { ...page, id: pg.id } : pg)),
            }
          : p
      )
    );
    setSelectedKeyIdx(0);
    setSurface("key");
    setTemplatesOpen(false);
    triggerToast(`Replaced the page with "${page.name}". Ctrl+Z undoes it.`, "success");
  };

  const handleBatchApplyStyleToPage = () => {
    if (!activeProfile || !activePage || !activeKey) return;
    pushUndoSnapshot();
    setProfiles((prev) =>
      prev.map((prof) => {
        if (prof.id !== activeProfile.id) return prof;
        return {
          ...prof,
          pages: prof.pages.map((pg, idx) => {
            if (idx !== selectedPageIdx) return pg;
            return {
              ...pg,
              keys: pg.keys.map((k) => ({
                ...k,
                accentColor: activeKey.accentColor,
                bgColor: activeKey.bgColor,
                cornerRadius: activeKey.cornerRadius,
                titleSize: activeKey.titleSize,
                backgroundStyle: activeKey.backgroundStyle,
              })),
            };
          }),
        };
      })
    );
    triggerToast("Applied current key's appearance style to all keys on this page ✓", "success");
  };

  // Stable wrappers. Live face ticks re-render this component, and these callbacks
  // keep the memoized deck, sidebar, and action list from rebuilding with them.
  editorRef.current = {
    recoveryChecked, recoveryDraft, recoveryError, quitOpen, showOnboarding, hotkeyTarget,
    activeKey, activeZone, activeEncoder, activeScreen, activeProfile,
    updateSelectedKey, updateSelectedZone, updateSelectedEncoder,
    handleSaveLocally, handleUndo, handleRedo, handleApplyToDevice, triggerToast,
    setHotkeyTarget, setSelectedPageIdx, clipFor, handleAddPage, ingestFile,
    handleDropAction, handleMoveKey, handleMoveKeyToPage, saveRules,
    handleAddProfile, handleExportProfile, handleImportProfile,
  };
  const drawerCallbacks = useMemo(
    () => ({
      triggerToast: (...args: Parameters<typeof triggerToast>) => editorRef.current?.triggerToast(...args),
      updateSelectedKey: (patch: Partial<KeyConfig>) => editorRef.current?.updateSelectedKey(patch),
      updateSelectedZone: (index: number, patch: Partial<TouchZoneConfig>) => editorRef.current?.updateSelectedZone(index, patch),
      updateSelectedEncoder: (index: number, patch: Partial<EncoderConfig>) => editorRef.current?.updateSelectedEncoder(index, patch),
    }),
    []
  );
  const stageCallbacks = useMemo(() => ({
    clipFor: (cfg?: ScreenMediaConfig | null) => editorRef.current?.clipFor(cfg) ?? null,
    handleAddPage: () => editorRef.current?.handleAddPage(),
    ingestFile: (file: File, target: ScreenTarget, index: number) => editorRef.current?.ingestFile(file, target, index) ?? Promise.resolve(),
    onDropAction: (target: "key" | "zone" | "encoder", index: number, actionId: string, part?: KnobPart) => editorRef.current?.handleDropAction(target, index, actionId, part),
    onMoveKey: (from: number, to: number) => editorRef.current?.handleMoveKey(from, to),
    onMoveKeyToPage: (keyIndex: number, pageIndex: number) => editorRef.current?.handleMoveKeyToPage(keyIndex, pageIndex),
    saveRules: (rules: AutoSwitchRule[]) => editorRef.current?.saveRules(rules),
  }), []);
  const sidebarCallbacks = useMemo(() => ({
    handleAddProfile: (starter: StarterProfile) => editorRef.current?.handleAddProfile(starter),
    onExportProfile: () => { void editorRef.current?.handleExportProfile(); },
    onImportProfile: () => { void editorRef.current?.handleImportProfile(); },
    saveRules: (rules: AutoSwitchRule[]) => editorRef.current?.saveRules(rules),
    triggerToast: (text: string, kind?: "success" | "info" | "warning") => editorRef.current?.triggerToast(text, kind),
  }), []);

  const quitDialog = <QuitDialog open={quitOpen} busy={quitting} error={quitError}
    dirty={!savedLocally || !!recoveryDraft || !!recoveryError || Object.values(saveStatuses).some(s => s.pending)}
    onCancel={() => setQuitOpen(false)} onQuit={finishQuit} />;

  if (bootstrapError) {
    return (
      <div className="h-screen bg-[var(--bg-app)] text-[var(--text-primary)] flex items-center justify-center p-8" role="alert">
        {quitDialog}
        <div className="max-w-md w-full rounded-2xl border border-[var(--border-strong)] bg-[var(--bg-panel)] p-6 shadow-2xl space-y-4 text-center animate-pop-in">
          <div className="w-11 h-11 mx-auto rounded-xl bg-[var(--status-warning)]/15 text-[var(--status-warning)] flex items-center justify-center">
            <AlertTriangle size={22} />
          </div>
          <h1 className="text-lg font-bold">Could not load DeckForge</h1>
          <p className="text-xs leading-relaxed text-[var(--status-warning)] break-words">{bootstrapError}</p>
          <button
            type="button"
            className="h-9 px-5 rounded-lg bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white cursor-pointer"
            onClick={() => setBootstrapAttempt(n => n + 1)}
          >
            Retry loading
          </button>
        </div>
      </div>
    );
  }

  if (recoveryChecked && (recoveryDraft || recoveryError)) {
    return (
      <div className="h-screen bg-[var(--bg-app)] text-[var(--text-primary)] flex items-center justify-center p-8">
        {quitDialog}
        <section aria-labelledby="recovery-title" className="max-w-lg w-full rounded-2xl border border-[var(--border-strong)] bg-[var(--bg-panel)] p-6 shadow-2xl space-y-4 animate-pop-in">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-[var(--accent-soft)] text-[var(--accent-text)] flex items-center justify-center shrink-0">
              <AlertTriangle size={20} />
            </div>
            <h1 id="recovery-title" className="text-lg font-bold">{recoveryDraft ? "Recover unsaved edits?" : "Could not read recovery draft"}</h1>
          </div>
          {recoveryDraft && <p className="text-xs leading-relaxed text-[var(--text-secondary)]">{recoveryDraft.draft.profiles.length} profiles captured {new Date(recoveryDraft.draft.updatedAtMs).toLocaleString()}. Restore opens them as unsaved editor drafts. Saved profiles and the deck stay unchanged until you Save or Apply.</p>}
          {recoveryDraft?.savedProfilesChanged && <p className="text-xs leading-relaxed text-[var(--status-warning)] bg-[var(--status-warning)]/10 border border-[var(--status-warning)]/30 rounded-lg px-3 py-2">Saved profiles have changed since this draft began. Restoring replaces the editor contents with the recovered version; review it before saving. Undo returns to the loaded saved profiles.</p>}
          {recoveryError && <p role="alert" className="text-xs text-[var(--status-error)] break-words">{recoveryError}</p>}
          {!recoveryDraft && <p className="text-xs leading-relaxed text-[var(--text-secondary)]">The file is preserved at <code className="text-[var(--text-primary)]">%APPDATA%\DeckForge\profile-draft.json</code>. Retry reading it, or discard this recovery copy to open saved profiles.</p>}
          <div className="flex flex-wrap gap-2.5 pt-1 text-xs">
            {recoveryDraft ? (
              <button type="button" autoFocus disabled={recoveryBusy} className="h-9 px-4 rounded-lg bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] font-semibold text-white cursor-pointer disabled:opacity-40" onClick={() => resolveRecovery(true)}>Restore draft</button>
            ) : (
              <button type="button" disabled={recoveryBusy} className="h-9 px-4 rounded-lg border border-[var(--border-strong)] hover:border-[var(--accent-primary)] font-medium cursor-pointer disabled:opacity-40" onClick={() => { setRecoveryChecked(false); setRecoveryError(null); setBootstrapAttempt(n => n + 1); }}>Retry recovery</button>
            )}
            <button type="button" disabled={recoveryBusy} className="h-9 px-4 rounded-lg border border-[var(--border-strong)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-elevated)] font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer disabled:opacity-40" onClick={() => resolveRecovery(false)}>Discard draft</button>
          </div>
        </section>
      </div>
    );
  }

  if (!recoveryChecked || !activeDevice || !activeProfile || !activePage || !activeKey) {
    return (
      <div className="h-screen w-screen bg-[var(--bg-app)] text-[var(--text-primary)] flex items-center justify-center p-6">
        {quitDialog}
        <div className="flex flex-col items-center gap-4 text-center animate-fade-in">
          <div aria-hidden className="w-10 h-10 rounded-xl bg-[var(--accent-primary)] p-2 grid grid-cols-2 gap-1 shadow-[0_4px_20px_rgba(124,131,247,0.35)]">
            <span className="bg-[var(--bg-app)] rounded-[2px]" />
            <span className="bg-[var(--bg-app)] rounded-[2px]" />
            <span className="bg-[var(--bg-app)] rounded-[2px]" />
            <span className="bg-[var(--bg-app)] rounded-[2px] opacity-50" />
          </div>
          <div className="flex items-center gap-2.5 text-xs text-[var(--text-secondary)]">
            <RefreshCw className="animate-spin text-[var(--accent-primary)]" size={15} />
            <span>Initializing DeckForge Hardware &amp; Action Service…</span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen w-screen bg-[var(--bg-app)] text-[var(--text-primary)] flex flex-col overflow-hidden select-none">
      {quitDialog}
      {(!savedLocally || draftStatus.error) && (
        <div role={draftStatus.error ? "alert" : "status"} className="px-4 py-1.5 text-[11.5px] bg-[var(--bg-panel)] border-b border-[var(--border-subtle)] text-[var(--text-secondary)] flex items-center gap-2.5">
          <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${draftStatus.error ? "bg-[var(--status-warning)]" : draftStatus.pending ? "bg-[var(--accent-primary)] animate-pulse" : "bg-[var(--status-success)]"}`} />
          <span>{draftStatus.error ? `Recovery copy could not update: ${draftStatus.error}. Retry recovery or save your profiles.` : draftStatus.pending ? "Updating recovery copy…" : "Recovery copy saved · Profile edits are still unsaved"}</span>
          {draftStatus.error && <button type="button" className="underline text-[var(--status-warning)] hover:text-[var(--text-primary)] cursor-pointer font-medium transition-colors" onClick={draftWriter.retry}>Retry recovery copy</button>}
        </div>
      )}
      {Object.entries(saveQueues).map(([name, queue]) => {
        const status = saveStatuses[name] ?? savedStatus;
        if (!status.pending) return null;
        return (
          <div key={name} role={status.error ? "alert" : "status"} className="px-4 py-1.5 text-[11.5px] bg-[var(--accent-soft)] border-b border-[var(--accent-primary)]/30 text-[var(--status-warning)] flex items-center gap-2.5">
            <span>{status.error ? `${name} not saved: ${status.error}. Your edits remain in this session.` : `Saving ${name.toLowerCase()}…`}</span>
            {status.error && <button type="button" className="underline font-semibold cursor-pointer hover:text-white transition-colors" onClick={queue.retry}>Retry save</button>}
          </div>
        );
      })}
      <input
        ref={fileRef}
        type="file"
        accept="image/png,image/jpeg,image/gif,image/webp,image/bmp,.gif"
        className="hidden"
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = "";
          if (!file) return;
          if (surface === "encoder") {
            triggerToast("Choose a key or touch screen first", "warning");
            return;
          }
          const target: ScreenTarget = surface === "zone" || surface === "strip" ? surface : "key";
          const index = target === "key" ? activeKey.index : surfaceIndex;
          void ingestFile(file, target, index);
        }}
      />
      {/* Transient Bottom-Right Toast */}
      {toastMessage && (
        <div
          role="status"
          aria-live="polite"
          className={`fixed bottom-14 right-5 z-50 flex items-center gap-2.5 pl-3.5 pr-2.5 py-2.5 rounded-xl bg-[var(--bg-elevated)]/95 backdrop-blur-md border shadow-2xl text-[12.5px] font-medium text-[var(--text-primary)] max-w-md animate-slide-up ${
            toastMessage.kind === "warning"
              ? "border-[var(--status-warning)]/45"
              : toastMessage.kind === "info"
              ? "border-[var(--accent-primary)]/40"
              : "border-[var(--status-success)]/40"
          }`}
        >
          <span
            className={`w-2 h-2 rounded-full shrink-0 ${
              toastMessage.kind === "warning" ? "bg-[var(--status-warning)]" : toastMessage.kind === "info" ? "bg-[var(--accent-primary)]" : "bg-[var(--status-success)]"
            }`}
          />
          <span className="leading-snug">{toastMessage.text}</span>
          {toastMessage.action && (
            <button
              type="button"
              onClick={() => {
                toastMessage.action?.run();
                setToastMessage(null);
              }}
              className="ml-1 px-2.5 h-6 rounded-[var(--radius-sm)] bg-[var(--accent-soft)] text-[var(--accent-text)] hover:bg-[var(--accent-primary)] hover:text-white font-semibold cursor-pointer shrink-0"
            >
              {toastMessage.action.label}
            </button>
          )}
          <button
            type="button"
            aria-label="Dismiss notification"
            onClick={() => setToastMessage(null)}
            className="p-1 rounded-md text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)] cursor-pointer shrink-0"
          >
            <X size={13} />
          </button>
        </div>
      )}
      {templatesOpen && (
        <PageTemplates
          currentPage={activePage}
          profileName={activeProfile.name}
          onAdd={handleAddPageFromTemplate}
          onReplace={handleReplacePageFromTemplate}
          onClose={() => setTemplatesOpen(false)}
          notify={triggerToast}
        />
      )}

      <div className="flex-1 min-h-0 flex">
      {/* Icon rail: one place to switch screens, labels on hover. */}
      <nav aria-label="Screens" className="w-14 shrink-0 bg-[var(--bg-sidebar)] border-r border-[var(--border-subtle)] flex flex-col items-center gap-1 py-3">
        <div aria-hidden className="w-7 h-7 mb-3 rounded-[var(--radius-sm)] bg-[var(--accent-primary)] p-[5px] grid grid-cols-2 gap-[3px] shadow-[0_2px_10px_rgba(124,131,247,0.3)]">
          <span className="bg-[var(--bg-app)] rounded-[1.5px]" />
          <span className="bg-[var(--bg-app)] rounded-[1.5px]" />
          <span className="bg-[var(--bg-app)] rounded-[1.5px]" />
          <span className="bg-[var(--bg-app)] rounded-[1.5px] opacity-50" />
        </div>
        {(
          [...(NAV_ITEMS), { id: "settings", label: "Settings", icon: Settings }] as Array<{ id: NavScreenId; label: string; icon: React.ComponentType<{ size?: number; strokeWidth?: number }> }>
        ).map((tab) => {
          const Icon = tab.icon;
          const isActive = activeScreen === tab.id;
          return (
            <React.Fragment key={tab.id}>
              {tab.id === "settings" && <div aria-hidden className="mt-auto mb-1 w-6 h-px bg-[var(--border-subtle)]" />}
              <button
                type="button"
                title={tab.label}
                aria-label={tab.label}
                aria-current={isActive ? "page" : undefined}
                onClick={() => {
                  setActiveScreen(tab.id);
                  if (tab.id === "icons") {
                    setSurface("key");
                    setSurfaceIndex(activeKey.index);
                  }
                }}
                className={`group relative w-10 h-10 rounded-[var(--radius-md)] flex items-center justify-center transition-all duration-150 cursor-pointer ${
                  isActive
                    ? "bg-[var(--bg-elevated)] text-[var(--text-primary)] border border-[var(--border-strong)]/60 shadow-xs"
                    : "border border-transparent text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
                }`}
              >
                {isActive && <span aria-hidden className="absolute -left-2 top-2.5 bottom-2.5 w-[3px] rounded-r bg-[var(--accent-primary)] shadow-[0_0_8px_rgba(124,131,247,0.65)]" />}
                <Icon size={19} strokeWidth={isActive ? 2 : 1.75} />
                <span
                  aria-hidden
                  className="pointer-events-none absolute left-full ml-2.5 z-50 px-2.5 py-1 rounded-md bg-[var(--bg-elevated)] border border-[var(--border-strong)] shadow-xl text-[11.5px] font-medium text-[var(--text-primary)] whitespace-nowrap opacity-0 -translate-x-1 group-hover:opacity-100 group-hover:translate-x-0 group-focus-visible:opacity-100 group-focus-visible:translate-x-0 transition-all duration-150"
                >
                  {tab.label}
                </span>
              </button>
            </React.Fragment>
          );
        })}
      </nav>
      <div className="flex-1 min-w-0 flex flex-col">
      <header className="h-12 bg-[var(--bg-app)] border-b border-[var(--border-subtle)] px-4 flex items-center justify-between gap-3 shrink-0">
        <div className="flex items-center gap-2.5 min-w-0">
          <h1 className="text-[14px] font-semibold tracking-tight text-[var(--text-primary)] truncate">
            {[...NAV_ITEMS, { id: "settings", label: "Settings" }].find((n) => n.id === activeScreen)?.label ?? "DeckForge"}
          </h1>
        </div>
        {/* Right Controls */}
        <div className="flex items-center gap-1.5">
          {telemetry?.officialStreamdockRunning && (
            <span
              title="AJAZZ detected. Close it, then choose Use DeckForge in the status bar."
              className="hidden sm:inline-flex items-center gap-1.5 px-2.5 h-7 rounded-full bg-[var(--status-warning)]/12 border border-[var(--status-warning)]/35 text-[11px] font-medium text-[var(--status-warning)] max-w-[260px] truncate"
            >
              <AlertTriangle size={12} className="shrink-0" />
              <span className="truncate">AJAZZ running — close &amp; choose Use DeckForge</span>
            </span>
          )}
          <button
            type="button"
            onClick={handleImportLegacyProfiles}
            title="Parse & Import Legacy .sdProfile Configurations from %APPDATA%\HotSpot\StreamDock\profiles"
            className="h-8 px-2.5 rounded-[var(--radius-sm)] border border-[var(--border-subtle)] bg-[var(--bg-panel)]/60 hover:border-[var(--border-strong)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] text-[12px] font-medium flex items-center gap-1.5 cursor-pointer transition-colors"
          >
            <Download size={13} />
            <span>Import from Stream Dock</span>
          </button>
          <span className="w-px h-4 bg-[var(--border-subtle)] mx-0.5" aria-hidden />
          <button
            type="button"
            onClick={handleUndo}
            disabled={undoStack.length === 0}
            title="Undo (Ctrl+Z)"
            aria-label="Undo"
            className="w-8 h-8 rounded-[var(--radius-sm)] border border-transparent hover:border-[var(--border-subtle)] flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)] disabled:opacity-35 disabled:hover:border-transparent disabled:hover:bg-transparent disabled:cursor-default cursor-pointer transition-colors"
          >
            <RotateCcw size={14} />
          </button>
          <button
            type="button"
            onClick={handleRedo}
            disabled={redoStack.length === 0}
            title="Redo (Ctrl+Y)"
            aria-label="Redo"
            className="w-8 h-8 rounded-[var(--radius-sm)] border border-transparent hover:border-[var(--border-subtle)] flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)] disabled:opacity-35 disabled:hover:border-transparent disabled:hover:bg-transparent disabled:cursor-default cursor-pointer transition-colors"
          >
            <RotateCw size={14} />
          </button>
          <button
            type="button"
            onClick={() => setShowOnboarding(true)}
            title="Quick-start guide"
            aria-label="Quick-start guide"
            className="w-8 h-8 rounded-[var(--radius-sm)] border border-transparent hover:border-[var(--border-subtle)] flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)] cursor-pointer transition-colors"
          >
            <HelpCircle size={15} />
          </button>
        </div>
      </header>

      {/* 2. SCREEN 01: WORKSPACE / KEY EDITOR (Matches Reference Screenshot + Spec Section 4) */}
      {activeScreen === "workspace" && (
        <div className="flex-1 grid grid-cols-[232px_minmax(0,1fr)_292px] 2xl:grid-cols-[256px_minmax(0,1fr)_324px] overflow-hidden">
          {/* LEFT CONTEXTUAL SIDEBAR: Devices + Profiles */}
          <WorkspaceSidebar activeProfile={activeProfile} autoSwitchRules={autoSwitchRules} connectedDevices={connectedDevices} handleAddProfile={sidebarCallbacks.handleAddProfile} onExportProfile={sidebarCallbacks.onExportProfile} onImportProfile={sidebarCallbacks.onImportProfile} profiles={profiles} selectedPageIdx={selectedPageIdx} saveRules={sidebarCallbacks.saveRules} setDevices={setDevices} setSelectedKeyIdx={setSelectedKeyIdx} setSelectedPageIdx={setSelectedPageIdx} setSelectedProfileId={setSelectedProfileId} triggerToast={sidebarCallbacks.triggerToast} />

          {/* CENTER WORKSPACE COLUMN: Toolbar + Hardware Bezel + Bottom Action Library Drawer */}
          <main className="flex flex-col justify-between bg-[var(--bg-app)] overflow-hidden">
            {/* Top Workspace Toolbar */}
            <WorkspaceToolbar activeDevice={activeDevice} activeProfile={activeProfile} canvasZoom={canvasZoom} connectedDevices={connectedDevices} handleApplyToDevice={handleApplyToDevice} handleSaveLocally={handleSaveLocally} setCanvasZoom={setCanvasZoom} setSelectedDeviceId={setSelectedDeviceId} setSelectedKeyIdx={setSelectedKeyIdx} setSurface={setSurface} setSurfaceIndex={setSurfaceIndex} />

            {/* Central Hardware Bezel Stage */}
            <DeckStage activeDevice={activeDevice} activeKey={activeKey} activePage={activePage} activeProfile={activeProfile} canvasZoom={canvasZoom} clipFor={stageCallbacks.clipFor} face={face} handleAddPage={stageCallbacks.handleAddPage} ingestFile={stageCallbacks.ingestFile} livePressedKeyIdx={livePressedKeyIdx} liveEncoderEvent={liveEncoderEvent} liveTouchZoneIdx={liveTouchZoneIdx} selectedPageIdx={selectedPageIdx} setSelectedKeyIdx={setSelectedKeyIdx} setSelectedPageIdx={setSelectedPageIdx} setSurface={setSurface} setSurfaceIndex={setSurfaceIndex} setTemplatesOpen={setTemplatesOpen} surface={surface} surfaceIndex={surfaceIndex} onDropAction={stageCallbacks.onDropAction} onMoveKey={stageCallbacks.onMoveKey} onMoveKeyToPage={stageCallbacks.onMoveKeyToPage} autoSwitchRules={autoSwitchRules} saveRules={stageCallbacks.saveRules} />

            {/* BOTTOM ACTION LIBRARY DRAWER (Matches Reference Screenshot Bottom Panel) */}
            <ActionDrawer actionCategoryFilter={actionCategoryFilter} activeEncoder={activeEncoder} activeKey={activeKey} activeZone={activeZone} actions={allActionTemplates} setActionCategoryFilter={setActionCategoryFilter} surface={surface} {...drawerCallbacks} />
          </main>

          {/* RIGHT INSPECTOR PANE (320px — Button Properties matching Reference Screenshot) */}
          <KeyInspector mediaLibrary={mediaLibrary} onMediaStored={clip => setMediaLibrary(prev => ({ ...prev, [clip.id]: clip }))} activeDevice={activeDevice} activeEncoder={activeEncoder} activeKey={activeKey} activeZone={activeZone} allActionTemplates={allActionTemplates} assignMedia={assignMedia} clipFor={clipFor} clipboardKey={clipboardKey} currentMedia={currentMedia} face={face} fileRef={fileRef} handleClearSelectedKey={handleClearSelectedKey} handleTestSelectedAction={handleTestSelectedAction} hotkeyTarget={hotkeyTarget} ingestFile={ingestFile} inspectorTab={inspectorTab} macros={macros} obsSettings={obsSettings} paramOptionsFor={paramOptionsFor} patchCurrentMedia={patchCurrentMedia} saveObs={saveObs} setActionCategoryFilter={setActionCategoryFilter} setClipboardKey={setClipboardKey} setCustomActions={setCustomActions} setHotkeyTarget={setHotkeyTarget} setInspectorTab={setInspectorTab} surface={surface} surfaceIndex={surfaceIndex} triggerToast={triggerToast} updateSelectedEncoder={updateSelectedEncoder} onKnobLedChange={handleKnobLedChange} onResetKnobLeds={() => void handleResetKnobLeds()} profileColorStrip={activeProfile.colorStrip} onApplyKnobLedPalette={handleApplyKnobLedPalette} onApplyKnobLedsToAllPages={handleApplyKnobLedsToAllPages} knobLedEffect={activePage.knobLedEffect ?? "static"} onKnobLedEffectChange={(effect) => void handleKnobLedEffectChange(effect)} updateSelectedKey={updateSelectedKey} updateSelectedZone={updateSelectedZone} useSampleAnimation={useSampleAnimation} />
        </div>
      )}

      {/* 3. SCREEN 02: DEVICE MANAGER & DIAGNOSTICS (Spec Section 5) */}
      {activeScreen === "devices" && (
        <DevicesScreen connectedDevices={connectedDevices} activeDevice={activeDevice} frameStats={frameStats} diagnostics={diagnostics} onRescan={handleRescanDevices} onSelectDevice={handleSelectDevice} onBrightness={handleBrightnessChange} onOpenWorkspace={() => setActiveScreen("workspace")} onRestartDeck={handleRestartDeck} encoders={activePage.encoders} onKnobLedChange={handleKnobLedChange} onResetKnobLeds={() => void handleResetKnobLeds()} ledBrightness={ledBrightness} onLedBrightness={(pct) => void handleLedBrightnessChange(pct)} ledFollowSleep={ledFollowSleep} onLedSleepMode={(follow) => void handleLedSleepModeChange(follow)} knobLedEffect={activePage.knobLedEffect ?? "static"} onKnobLedEffectChange={(effect) => void handleKnobLedEffectChange(effect)} />
      )}

      {/* 4. SCREEN 04: PLUGINS & LEGACY .sdPlugin BRIDGE (Spec Section 7) */}
      {activeScreen === "plugins" && (
        <PluginsScreen legacySummary={legacySummary} onImportLegacy={handleImportLegacyProfiles} plugins={plugins} onPluginsChange={setPlugins} notify={triggerToast} />
      )}

      {activeScreen === "macros" && (
        <MacroStudio
          macros={macros}
          profiles={profiles}
          selectedMacroId={selectedMacroId}
          selectedStepIdx={selectedMacroStepIdx}
          runReport={macroRunReport}
          activeKeyLabel={activeKey ? `key ${activeKey.index + 1}` : null}
          onSelectMacro={setSelectedMacroId}
          onSelectStep={setSelectedMacroStepIdx}
          onChange={persistMacros}
          onRun={async (macro) => {
            try {
              const next = macros.some((item) => item.id === macro.id)
                ? macros.map((item) => (item.id === macro.id ? macro : item)) : [macro, ...macros];
              persistMacros(next);
              if (!await saveQueues.Macros.flush()) return;
              const report = await deckforgeApi.executeMacroSequence(macro.id);
              setMacroRunReport(report);
              triggerToast(`${report.macroName} finished in ${report.totalElapsedMs}ms`,
                report.stepResults.some(step => step.status !== "Passed") ? "warning" : "success");
            } catch (error) {
              triggerToast(typeof error === "string" ? error : "Could not run the macro", "warning");
            }
          }}
          onStop={async () => {
            try { triggerToast(await deckforgeApi.stopMacroExecution(), "info"); }
            catch { triggerToast("Could not stop the running macro", "warning"); }
          }}
          onAssign={(macro) => {
            void assignMacroToKey(macro);
          }}
        />
      )}

      {/* 6. SCREEN 06: ICON STUDIO (Spec Section 9) */}
      {activeScreen === "icons" && (
        <IconStudio
          keys={activePage.keys.slice(0, activeDevice.keys.count)}
          columns={activeDevice.keys.cols}
          selectedIndex={activeKey.index}
          clip={clipFor(activeKey.media)}
          pixelWidth={activeDevice.keys.pixelWidth}
          pixelHeight={activeDevice.keys.pixelHeight}
          onSelectKey={(index) => {
            setSurface("key");
            setSurfaceIndex(index);
            setSelectedKeyIdx(index);
          }}
          onChange={updateSelectedKey}
          onUpload={() => fileRef.current?.click()}
          onSample={() => void useSampleAnimation("key", activeKey.index)}
          onClearMedia={() => assignMedia("key", activeKey.index, null)}
          onMediaPatch={(patch) => {
            if (!activeKey.media) return;
            assignMedia("key", activeKey.index, { ...activeKey.media, ...patch });
          }}
          onPaintPage={handleBatchApplyStyleToPage}
          onApply={() => void handleApplyToDevice()}
          onDropFile={(file) => void ingestFile(file, "key", activeKey.index)}
          onBrandPick={(file) => void ingestFile(file, "key", activeKey.index)}
        />
      )}

      {activeScreen === "themes" && (
        <ThemesScreen
          keys={activePage.keys.slice(0, activeDevice.keys.count)}
          columns={activeDevice.keys.cols}
          rows={activeDevice.keys.rows}
          pixelWidth={activeDevice.keys.pixelWidth}
          pixelHeight={activeDevice.keys.pixelHeight}
          selectedIndex={activeKey.index}
          onApplyFrames={applyKeyFrames}
          notify={triggerToast}
        />
      )}

      {/* GAME MODE: close what you don't need while playing */}
      {activeScreen === "gamemode" && <GameModeStudio notify={triggerToast} />}

      {/* 7. SCREEN 07: SETTINGS (Spec Section 10) */}
      {activeScreen === "settings" && (
        <SettingsScreen profiles={profiles} startupProfileId={startupProfileId} onStartupProfileChange={async id => { await deckforgeApi.saveStartupProfile(id); setStartupProfileId(id); }} onBackup={handleBackup} onRestore={handleRestoreBackup} onNavigate={setActiveScreen} onOpenGuide={() => setShowOnboarding(true)} />
      )}

      </div>
      </div>

      {/* First-Run Onboarding / Hardware Guide Modal (Spec Section 11.1) */}
      {showOnboarding && (
        <OnboardingModal devices={devices} onClose={() => setShowOnboarding(false)} onOpenDevices={() => { setShowOnboarding(false); setActiveScreen("devices"); }} />
      )}

      <WorkspaceStatusBar
        deviceId={activeDevice.id} deviceName={activeDevice.modelName}
        profileId={activeProfile.id} profileName={activeProfile.name} page={activePage}
        saved={savedLocally && !Object.values(saveStatuses).some(s => s.pending)}
        refresh={appliedRefresh} onNotify={triggerToast}
      />
    </div>
  );
};
