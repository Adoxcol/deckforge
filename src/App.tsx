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
  Zap,
} from "lucide-react";
import { deckforgeApi, isTauriRuntime } from "./bridge/tauriBridge";
import { createProfilePersistence, profileSnapshot } from "./lib/profilePersistence";
import { QuitDialog } from "./components/QuitDialog";
import { createSaveQueue, savedStatus, type SaveStatus } from "./lib/saveQueue";
import { AVAILABLE_ICON_NAMES, DeckIcon } from "./components/DeckIcon";
import { ActionBinding, ObsConnection, actionNeedsParam, splitStoredBinding } from "./components/ActionBinding";
import { IconStudio } from "./components/IconStudio";
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
import { ActionDrawer } from "./components/ActionDrawer";
import { KeyInspector } from "./components/KeyInspector";
import { DevicesScreen } from "./screens/DevicesScreen";
import { PluginsScreen } from "./screens/PluginsScreen";
import { SettingsScreen } from "./screens/SettingsScreen";
import type {
  AutoSwitchRule,
  DeckFaceState,
  DeviceDescriptor,
  DiagnosticEntry,
  FrameCacheStats,
  EncoderConfig,
  KeyConfig,
  LegacyStreamDockSummary,
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
} from "./types/deckforge";

import { ACTION_CATALOG, type ActionTemplate } from "./data/actionCatalog";
import { bytesToBase64, buildPulseFrames, errorText } from "./lib/format";
import { buildProfileFile, parseProfileFile, pickJsonFile, placeImportedProfile, slugify } from "./lib/shareFiles";
import { LevelMeter, liveKeyLook, liveZoneValue } from "./components/LiveFace";

export const App: React.FC = () => {
  const [activeScreen, setActiveScreen] = useState<NavScreenId>("workspace");
  const [devices, setDevices] = useState<DeviceDescriptor[]>([]);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [autoSwitchRules, setAutoSwitchRules] = useState<AutoSwitchRule[]>([]);
  const [macros, setMacros] = useState<MacroSequence[]>([]);
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
  const [selectedPageIdx, setSelectedPageIdx] = useState<number>(0);
  const [selectedKeyIdx, setSelectedKeyIdx] = useState<number>(0);
  const [surface, setSurface] = useState<"key" | "zone" | "strip" | "encoder">("key");
  const [surfaceIndex, setSurfaceIndex] = useState(0);
  const [mediaLibrary, setMediaLibrary] = useState<Record<string, MediaPreview>>({});
  const fileRef = useRef<HTMLInputElement>(null);
  const [inspectorTab, setInspectorTab] = useState<"Action" | "Appearance" | "Behavior">("Action");
  const [actionCategoryFilter, setActionCategoryFilter] = useState<string>("All");
  const [actionSearch, setActionSearch] = useState<string>("");
  const [canvasZoom, setCanvasZoom] = useState<number>(100);
  const [clipboardKey, setClipboardKey] = useState<KeyConfig | null>(null);
  const [templatesOpen, setTemplatesOpen] = useState<boolean>(false);

  // Sync & Persistence States (Spec Section 1 & 14: Distinguish Saved Locally vs Applied to Device)
  const [savedSnapshot, setSavedSnapshot] = useState("[]");
  const savedLocally = profileSnapshot(profiles) === savedSnapshot;
  const [profilePersistence] = useState(() => createProfilePersistence<Profile[]>(deckforgeApi.saveProfiles, setSavedSnapshot));
  const [quitOpen, setQuitOpen] = useState(false);
  const [quitting, setQuitting] = useState(false);
  const [quitError, setQuitError] = useState<string | null>(null);
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
  const [undoStack, setUndoStack] = useState<Profile[][]>([]);
  const [redoStack, setRedoStack] = useState<Profile[][]>([]);
  const [toastMessage, setToastMessage] = useState<{ text: string; kind: "success" | "info" | "warning" } | null>(null);

  // Sub-screen states
  const [selectedMacroId, setSelectedMacroId] = useState<string>("macro-go-live");
  const [selectedMacroStepIdx, setSelectedMacroStepIdx] = useState<number>(0);
  const [macroRunReport, setMacroRunReport] = useState<MacroRunReport | null>(null);
  const [showOnboarding, setShowOnboarding] = useState<boolean>(false);
  const [livePressedKeyIdx, setLivePressedKeyIdx] = useState<number | null>(null);
  const [face, setFace] = useState<DeckFaceState | null>(null);
  const [obsSettings, setObsSettings] = useState<ObsSettings>({ url: "ws://127.0.0.1:4455", password: "" });

  const triggerToast = (text: string, kind: "success" | "info" | "warning" = "success") => {
    setToastMessage({ text, kind });
    window.setTimeout(() => {
      setToastMessage((prev) => (prev?.text === text ? null : prev));
    }, 3200);
  };

  useEffect(() => {
    let disposed = false;
    setBootstrapError(null);
    deckforgeApi.getInitialState().then((data) => {
      if (disposed) return;
      setDevices(data.devices);
      setProfiles(data.profiles);
      setSavedSnapshot(profileSnapshot(data.profiles));
      setBootstrapReady(true);
      if (data.appliedContext) {
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

    const timer = window.setInterval(() => {
      deckforgeApi.getTelemetry().then(setTelemetry).catch(() => {});
    }, 5000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
      if (unlistenFn) unlistenFn();
      if (faceUnlisten) faceUnlisten();
      if (contextUnlisten) contextUnlisten();
    };
  }, [bootstrapAttempt]);

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

  const finishQuit = async (save: boolean) => {
    setQuitting(true); setQuitError(null);
    try {
      if (save) {
        if (!bootstrapReady) throw new Error("Wait for your profiles to load before saving.");
        await profilePersistence.save(latestProfiles.current);
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
      await deckforgeApi.quitApplication();
    } catch (error) { setQuitError(errorText(error)); }
    finally { setQuitting(false); }
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
    setProfiles(updatedProfiles);
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

  const [hotkeyTarget, setHotkeyTarget] = useState<"key" | "zone" | "cw" | "ccw" | "push" | null>(null);
  const [customActions, setCustomActions] = useState<ActionTemplate[]>([]);

  const allActionTemplates = useMemo(
    () => [...customActions, ...ACTION_CATALOG],
    [customActions]
  );

  // Keyboard shortcuts (Ctrl+S, Ctrl+Z, Ctrl+Y) + Live Hotkey Recorder
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (quitOpen || showOnboarding) return;
      if (hotkeyTarget) {
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
        const keyLabel =
          keyMap[e.key] || (e.key.length === 1 ? e.key.toUpperCase() : e.key);
        parts.push(keyLabel);
        const combo = parts.join("+");
        if (hotkeyTarget === "key") {
          updateSelectedKey({
            subtitle: combo,
            actionParam: combo,
            actionId: activeKey?.actionId === "none" || activeKey?.actionId === "custom.hotkey" ? "custom.hotkey" : activeKey?.actionId,
            actionLabel:
              activeKey?.actionId === "none" || activeKey?.actionId === "custom.hotkey"
                ? "Custom: Keyboard Shortcut / Hotkey"
                : activeKey?.actionLabel,
          });
        } else if (hotkeyTarget === "zone" && activeZone) {
          updateSelectedZone(activeZone.index, { tapActionId: "custom.hotkey", tapActionParam: combo });
        } else if (activeEncoder && (hotkeyTarget === "cw" || hotkeyTarget === "ccw" || hotkeyTarget === "push")) {
          const patch =
            hotkeyTarget === "cw"
              ? { cwAction: "custom.hotkey", cwParam: combo }
              : hotkeyTarget === "ccw"
              ? { ccwAction: "custom.hotkey", ccwParam: combo }
              : { pushAction: "custom.hotkey", pushParam: combo };
          updateSelectedEncoder(activeEncoder.index, patch);
        }
        setHotkeyTarget(null);
        triggerToast(`Recorded shortcut: ${combo}`, "success");
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        handleSaveLocally();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
        e.preventDefault();
        handleUndo();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "y") {
        e.preventDefault();
        handleRedo();
      }
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  });

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
      await profilePersistence.save(nextProfiles);
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
    setUndoStack((prev) => [...prev.slice(-19), JSON.parse(JSON.stringify(profiles))]);
    setRedoStack([]);
  };

  const handleUndo = () => {
    if (undoStack.length === 0) return;
    const previous = undoStack[undoStack.length - 1];
    setRedoStack((r) => [...r, JSON.parse(JSON.stringify(profiles))]);
    setProfiles(previous);
    setUndoStack((u) => u.slice(0, -1));
    triggerToast("Undid last key edit", "info");
  };

  const handleRedo = () => {
    if (redoStack.length === 0) return;
    const next = redoStack[redoStack.length - 1];
    setUndoStack((u) => [...u, JSON.parse(JSON.stringify(profiles))]);
    setProfiles(next);
    setRedoStack((r) => r.slice(0, -1));
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
      await profilePersistence.save(profiles);
      triggerToast("Profile saved locally ✓", "success");
    } catch (error) {
      triggerToast(`Profile was not saved: ${errorText(error)}. Your edits are still here; try Save again.`, "warning");
    }
  };

  const handleApplyToDevice = async () => {
    if (!activeDevice || !activeProfile || !activePage) return;
    let res;
    try {
      await profilePersistence.save(profiles);
      res = await deckforgeApi.applyProfileToDevice(
        activeDevice.id,
        activeProfile.id,
        activePage.id
      );
    } catch (err) {
      setAppliedRefresh(n => n + 1);
      triggerToast(`Could not send to the deck: ${errorText(err)}. Try Re-send layout in the status bar.`, "warning");
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

  const handleAddProfile = () => {
    if (!profiles[0]) return;
    const id = `profile-custom-${Date.now()}`;
    const newProf: Profile = {
      ...JSON.parse(JSON.stringify(profiles[0])),
      id,
      name: `Custom Profile ${profiles.length + 1}`,
      icon: "Sparkles",
      colorStrip: "#8B5CF6",
      lastModifiedMs: Date.now(),
    };
    setProfiles((prev) => [...prev, newProf]);
    setSelectedProfileId(id);

    triggerToast(`Created ${newProf.name}`, "success");
  };

  const handleExportProfile = async () => {
    if (!activeProfile) return;
    try {
      const { file, mediaRemoved } = buildProfileFile(activeProfile, macros);
      const where = await deckforgeApi.saveExportFile(
        `deckforge-profile-${slugify(activeProfile.name)}.json`,
        JSON.stringify(file, null, 2)
      );
      const left = mediaRemoved > 0 ? ` ${mediaRemoved} picture${mediaRemoved === 1 ? "" : "s"} or animation${mediaRemoved === 1 ? " was" : "s were"} left out.` : "";
      triggerToast(`Saved "${activeProfile.name}" to ${where}.${left}`, "success");
    } catch (err) {
      triggerToast(`Could not export the profile: ${errorText(err)}`, "warning");
    }
  };

  const handleImportProfile = async () => {
    const picked = await pickJsonFile();
    if (!picked) return;
    try {
      const placed = placeImportedProfile(parseProfileFile(picked.text), profiles, macros);
      const next = [...profiles, placed.profile];
      pushUndoSnapshot();
      setProfiles(next);
      if (placed.addedMacros > 0) {
        persistMacros(placed.macros);
        if (!await saveQueues.Macros.flush()) {
          throw new Error("Imported macros were not saved. Retry the macro save, then save the profile; your edits are still here.");
        }
      }
      await profilePersistence.save(next);
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
      triggerToast(`Could not import ${picked.name}: ${errorText(err)}`, "warning");
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

  const filteredActions = useMemo(() => {
    return allActionTemplates.filter((a) => {
      const matchesCat =
        actionCategoryFilter === "All" ||
        a.category.toLowerCase() === actionCategoryFilter.toLowerCase();
      const matchesSearch =
        !actionSearch.trim() ||
        a.title.toLowerCase().includes(actionSearch.toLowerCase()) ||
        a.description.toLowerCase().includes(actionSearch.toLowerCase()) ||
        a.actionLabel.toLowerCase().includes(actionSearch.toLowerCase());
      return matchesCat && matchesSearch;
    });
  }, [allActionTemplates, actionCategoryFilter, actionSearch]);

  const quitDialog = <QuitDialog open={quitOpen} busy={quitting} error={quitError}
    dirty={!savedLocally || Object.values(saveStatuses).some(s => s.pending)}
    onCancel={() => setQuitOpen(false)} onQuit={finishQuit} />;

  if (bootstrapError) {
    return <div className="h-screen bg-[#0B0D14] text-[#F4F5FA] flex flex-col items-center justify-center gap-4 p-8" role="alert">
      {quitDialog}
      <h1 className="text-xl font-bold">Could not load DeckForge</h1>
      <p className="text-[#F4B860] max-w-xl break-words">{bootstrapError}</p>
      <button className="px-4 py-2 rounded bg-[#8B5CF6]" onClick={() => setBootstrapAttempt(n => n + 1)}>Retry loading</button>
    </div>;
  }

  if (!activeDevice || !activeProfile || !activePage || !activeKey) {
    return (
      <div className="h-screen w-screen bg-[#0B0D14] text-[#F4F5FA] flex items-center justify-center">
        <div className="flex items-center gap-3 text-[#A6AFC4]">
          <RefreshCw className="animate-spin text-[#8B5CF6]" size={20} />
          {quitDialog}
          <span>Initializing DeckForge Rust Hardware & Action Service...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen w-screen bg-[#0B0D14] text-[#F4F5FA] flex flex-col overflow-hidden select-none">
      {quitDialog}
      {Object.entries(saveQueues).map(([name, queue]) => {
        const status = saveStatuses[name] ?? savedStatus;
        if (!status.pending) return null;
        return <div key={name} role={status.error ? "alert" : "status"} className="px-4 py-2 text-xs bg-[#30244F] text-[#F4B860] flex items-center gap-3">
          <span>{status.error ? `${name} not saved: ${status.error}. Your edits remain in this session.` : `Saving ${name.toLowerCase()}…`}</span>
          {status.error && <button className="underline font-semibold" onClick={queue.retry}>Retry save</button>}
        </div>;
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
      {/* Transient Top-Right Toast (Spec Section 11.5) */}
      {toastMessage && (
        <div className="fixed top-14 right-5 z-50 flex items-center gap-2.5 px-3.5 py-2.5 rounded-lg bg-[#1C2130] border border-[#8B5CF6]/60 shadow-2xl text-xs font-medium text-[#F4F5FA]">
          <span className="w-2 h-2 rounded-full bg-[#35C98B]" />
          <span>{toastMessage.text}</span>
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

      {/* 1. GLOBAL TOP NAVIGATION BAR (48px — Spec Section 3 & Reference Screenshot) */}
      <header className="min-h-14 bg-[#10131D] border-b border-[#272D3D] px-4 py-2 flex flex-wrap items-center justify-between gap-2 shrink-0">
        {/* Brand */}
        <div className="flex items-center gap-3">
          <div className="w-7 h-7 rounded-md bg-[#30244F] border border-[#8B5CF6] p-1 grid grid-cols-2 gap-0.5 shadow-[0_0_14px_rgba(139,92,246,0.35)]">
            <span className="bg-[#8B5CF6] rounded-[1.5px]" />
            <span className="bg-[#8B5CF6] rounded-[1.5px]" />
            <span className="bg-[#8B5CF6] rounded-[1.5px]" />
            <span className="bg-[#8B5CF6] rounded-[1.5px]" />
          </div>
          <div>
            <div className="text-[14px] font-bold tracking-tight text-[#F4F5FA] leading-tight">
              DeckForge
            </div>
            <div className="text-[10.5px] text-[#707B94] leading-tight">
              Your deck, your workflow
            </div>
          </div>
        </div>

        {/* Center Navigation Tabs */}
        <nav className="order-3 flex w-full items-center gap-1 overflow-x-auto 2xl:order-none 2xl:w-auto">
          {(
            [
              { id: "workspace", label: "Profiles & Keys", icon: Layers },
              { id: "plugins", label: "Plugins", icon: Puzzle },
              { id: "devices", label: "Devices", icon: Usb },
              { id: "macros", label: "Macro Studio", icon: Zap },
              { id: "icons", label: "Icon Studio", icon: Palette },
              { id: "gamemode", label: "Game Mode", icon: Gamepad2 },
              { id: "settings", label: "Settings", icon: Settings },
            ] as Array<{ id: NavScreenId; label: string; icon: React.ComponentType<{ size?: number }> }>
          ).map((tab) => {
            const Icon = tab.icon;
            const isActive = activeScreen === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => {
                  setActiveScreen(tab.id);
                  if (tab.id === "icons") {
                    setSurface("key");
                    setSurfaceIndex(activeKey.index);
                  }
                }}
                className={`h-9 px-3.5 shrink-0 whitespace-nowrap rounded-md text-[12.5px] font-medium flex items-center gap-2 transition-all cursor-pointer border ${
                  isActive
                    ? "bg-[#30244F] text-[#F4F5FA] border-[#8B5CF6]/50 shadow-[inset_0_-2px_0_#8B5CF6]"
                    : "bg-transparent text-[#A6AFC4] border-transparent hover:text-[#F4F5FA] hover:bg-[#151925]"
                }`}
              >
                <Icon size={15} />
                <span>{tab.label}</span>
              </button>
            );
          })}
        </nav>

        {/* Right Controls */}
        <div className="flex items-center gap-2">
          {telemetry?.officialStreamdockRunning && (
            <span className="text-xs text-[#F4B860] max-w-[250px]">
              AJAZZ detected. Close it, then choose Use DeckForge in the status bar.
            </span>
          )}
          <button
            onClick={handleImportLegacyProfiles}
            title="Parse & Import Legacy .sdProfile Configurations from %APPDATA%\HotSpot\StreamDock\profiles"
            className="h-8 px-2.5 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6] text-[#A6AFC4] hover:text-white text-[11.5px] font-medium flex items-center gap-1.5 cursor-pointer"
          >
            <Download size={13} />
            <span>Import .sdProfiles</span>
          </button>
          <button
            onClick={handleUndo}
            disabled={undoStack.length === 0}
            title="Undo (Ctrl+Z)"
            className="w-8 h-8 rounded-md bg-[#151925] border border-[#272D3D] flex items-center justify-center text-[#A6AFC4] hover:text-white disabled:opacity-35 cursor-pointer"
          >
            <RotateCcw size={14} />
          </button>
          <button
            onClick={handleRedo}
            disabled={redoStack.length === 0}
            title="Redo (Ctrl+Y)"
            className="w-8 h-8 rounded-md bg-[#151925] border border-[#272D3D] flex items-center justify-center text-[#A6AFC4] hover:text-white disabled:opacity-35 cursor-pointer"
          >
            <RotateCw size={14} />
          </button>
          <button
            onClick={() => setShowOnboarding(true)}
            title="Quick-start guide"
            className="w-8 h-8 rounded-md bg-[#151925] border border-[#272D3D] flex items-center justify-center text-[#A6AFC4] hover:text-white cursor-pointer"
          >
            <HelpCircle size={15} />
          </button>
        </div>
      </header>

      {/* 2. SCREEN 01: WORKSPACE / KEY EDITOR (Matches Reference Screenshot + Spec Section 4) */}
      {activeScreen === "workspace" && (
        <div className="flex-1 grid grid-cols-[210px_minmax(0,1fr)_280px] 2xl:grid-cols-[240px_minmax(0,1fr)_320px] overflow-hidden">
          {/* LEFT CONTEXTUAL SIDEBAR: Devices + Profiles */}
          <WorkspaceSidebar activeProfile={activeProfile} autoSwitchRules={autoSwitchRules} connectedDevices={connectedDevices} handleAddProfile={handleAddProfile} onExportProfile={handleExportProfile} onImportProfile={handleImportProfile} profiles={profiles} saveRules={saveRules} setDevices={setDevices} setSelectedKeyIdx={setSelectedKeyIdx} setSelectedPageIdx={setSelectedPageIdx} setSelectedProfileId={setSelectedProfileId} triggerToast={triggerToast} />

          {/* CENTER WORKSPACE COLUMN: Toolbar + Hardware Bezel + Bottom Action Library Drawer */}
          <main className="flex flex-col justify-between bg-[#0B0D14] overflow-hidden">
            {/* Top Workspace Toolbar */}
            <WorkspaceToolbar activeDevice={activeDevice} activeProfile={activeProfile} canvasZoom={canvasZoom} connectedDevices={connectedDevices} handleApplyToDevice={handleApplyToDevice} handleSaveLocally={handleSaveLocally} setCanvasZoom={setCanvasZoom} setSelectedDeviceId={setSelectedDeviceId} setSelectedKeyIdx={setSelectedKeyIdx} setSurface={setSurface} setSurfaceIndex={setSurfaceIndex} />

            {/* Central Hardware Bezel Stage */}
            <DeckStage activeDevice={activeDevice} activeKey={activeKey} activePage={activePage} activeProfile={activeProfile} canvasZoom={canvasZoom} clipFor={clipFor} face={face} handleAddPage={handleAddPage} ingestFile={ingestFile} livePressedKeyIdx={livePressedKeyIdx} selectedPageIdx={selectedPageIdx} setSelectedKeyIdx={setSelectedKeyIdx} setSelectedPageIdx={setSelectedPageIdx} setSurface={setSurface} setSurfaceIndex={setSurfaceIndex} setTemplatesOpen={setTemplatesOpen} surface={surface} surfaceIndex={surfaceIndex} />

            {/* BOTTOM ACTION LIBRARY DRAWER (Matches Reference Screenshot Bottom Panel) */}
            <ActionDrawer actionCategoryFilter={actionCategoryFilter} actionSearch={actionSearch} activeEncoder={activeEncoder} activeKey={activeKey} activeZone={activeZone} filteredActions={filteredActions} setActionCategoryFilter={setActionCategoryFilter} setActionSearch={setActionSearch} surface={surface} triggerToast={triggerToast} updateSelectedEncoder={updateSelectedEncoder} updateSelectedKey={updateSelectedKey} updateSelectedZone={updateSelectedZone} />
          </main>

          {/* RIGHT INSPECTOR PANE (320px — Button Properties matching Reference Screenshot) */}
          <KeyInspector activeDevice={activeDevice} activeEncoder={activeEncoder} activeKey={activeKey} activeZone={activeZone} allActionTemplates={allActionTemplates} assignMedia={assignMedia} clipFor={clipFor} clipboardKey={clipboardKey} currentMedia={currentMedia} face={face} fileRef={fileRef} handleClearSelectedKey={handleClearSelectedKey} handleTestSelectedAction={handleTestSelectedAction} hotkeyTarget={hotkeyTarget} ingestFile={ingestFile} inspectorTab={inspectorTab} macros={macros} obsSettings={obsSettings} paramOptionsFor={paramOptionsFor} patchCurrentMedia={patchCurrentMedia} saveObs={saveObs} setActionCategoryFilter={setActionCategoryFilter} setClipboardKey={setClipboardKey} setCustomActions={setCustomActions} setHotkeyTarget={setHotkeyTarget} setInspectorTab={setInspectorTab} surface={surface} surfaceIndex={surfaceIndex} triggerToast={triggerToast} updateSelectedEncoder={updateSelectedEncoder} updateSelectedKey={updateSelectedKey} updateSelectedZone={updateSelectedZone} useSampleAnimation={useSampleAnimation} />
        </div>
      )}

      {/* 3. SCREEN 02: DEVICE MANAGER & DIAGNOSTICS (Spec Section 5) */}
      {activeScreen === "devices" && (
        <DevicesScreen connectedDevices={connectedDevices} activeDevice={activeDevice} frameStats={frameStats} diagnostics={diagnostics} onRescan={handleRescanDevices} onSelectDevice={handleSelectDevice} onBrightness={handleBrightnessChange} onOpenWorkspace={() => setActiveScreen("workspace")} />
      )}

      {/* 4. SCREEN 04: PLUGINS & LEGACY .sdPlugin BRIDGE (Spec Section 7) */}
      {activeScreen === "plugins" && (
        <PluginsScreen legacySummary={legacySummary} onImportLegacy={handleImportLegacyProfiles} />
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
        />
      )}

      {/* GAME MODE: close what you don't need while playing */}
      {activeScreen === "gamemode" && <GameModeStudio notify={triggerToast} />}

      {/* 7. SCREEN 07: SETTINGS (Spec Section 10) */}
      {activeScreen === "settings" && (
        <SettingsScreen onNavigate={setActiveScreen} onOpenGuide={() => setShowOnboarding(true)} />
      )}

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
