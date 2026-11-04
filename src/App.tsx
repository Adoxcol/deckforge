import React, { useEffect, useMemo, useState } from "react";
import {
  Activity,
  AlertTriangle,
  Check,
  ChevronLeft,
  ChevronRight,
  Copy,
  Cpu,
  Download,
  Edit3,
  Eye,
  FolderPlus,
  HardDrive,
  HelpCircle,
  Layers,
  LayoutGrid,
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
import { deckforgeApi } from "./bridge/tauriBridge";
import { AVAILABLE_ICON_NAMES, DeckIcon } from "./components/DeckIcon";
import type {
  AutoSwitchRule,
  DeviceDescriptor,
  DiagnosticEntry,
  FrameCacheStats,
  KeyConfig,
  LegacyStreamDockSummary,
  MacroRunReport,
  MacroSequence,
  NavScreenId,
  Profile,
  SystemTelemetrySnapshot,
} from "./types/deckforge";

interface ActionTemplate {
  id: string;
  title: string;
  subtitle: string;
  category: "Custom" | "Streaming" | "OBS" | "Twitch" | "YouTube" | "Media" | "System" | "Plugins";
  iconName: string;
  actionLabel: string;
  defaultParam: string;
  accentColor: string;
  bgColor: string;
  description: string;
}

const ACTION_CATALOG: ActionTemplate[] = [
  {
    id: "custom.hotkey",
    title: "Custom Hotkey",
    subtitle: "HOTKEY",
    category: "Custom",
    iconName: "Terminal",
    actionLabel: "Custom: Keyboard Shortcut / Hotkey",
    defaultParam: "Ctrl+Shift+M",
    accentColor: "#8B5CF6",
    bgColor: "#231B38",
    description: "Record or enter any keyboard shortcut (Ctrl+Shift+M, F13–F24, Win+D, Media keys)",
  },
  {
    id: "custom.launch",
    title: "Launch App",
    subtitle: "APP",
    category: "Custom",
    iconName: "FolderPlus",
    actionLabel: "Custom: Launch Application / File (.exe)",
    defaultParam: "notepad.exe",
    accentColor: "#56B9F2",
    bgColor: "#132235",
    description: "Open any Windows executable (.exe), folder path, or game shortcut",
  },
  {
    id: "custom.shell",
    title: "Run Script",
    subtitle: "PWSH",
    category: "Custom",
    iconName: "Terminal",
    actionLabel: "Custom: PowerShell / CLI Command",
    defaultParam: "Get-Date",
    accentColor: "#35C98B",
    bgColor: "#112822",
    description: "Execute any PowerShell, CMD, Python, or CLI script on key press",
  },
  {
    id: "custom.url",
    title: "Open Website",
    subtitle: "URL",
    category: "Custom",
    iconName: "Eye",
    actionLabel: "Custom: Open Website URL",
    defaultParam: "https://github.com",
    accentColor: "#56B9F2",
    bgColor: "#132235",
    description: "Launch any URL or local dashboard in your default browser",
  },
  {
    id: "custom.text",
    title: "Paste Text",
    subtitle: "TEXT",
    category: "Custom",
    iconName: "Edit3",
    actionLabel: "Custom: Type / Paste Text Snippet",
    defaultParam: "Hello from DeckForge!",
    accentColor: "#F4B860",
    bgColor: "#2A2215",
    description: "Copy & paste any canned text response, code snippet, or prompt",
  },
  {
    id: "custom.http",
    title: "HTTP Webhook",
    subtitle: "WEBHOOK",
    category: "Custom",
    iconName: "Activity",
    actionLabel: "Custom: Trigger HTTP / Home Assistant Webhook",
    defaultParam: "http://127.0.0.1:8123/api/webhook/deck",
    accentColor: "#F16B79",
    bgColor: "#2A1521",
    description: "Fire REST API or smart-home webhook requests from any button",
  },
  {
    id: "obs.start_stream",
    title: "Start Stream",
    subtitle: "LIVE",
    category: "OBS",
    iconName: "Radio",
    actionLabel: "OBS: Start Streaming",
    defaultParam: "ws://127.0.0.1:4455 • Main",
    accentColor: "#F16B79",
    bgColor: "#2A1521",
    description: "Control OBS (stream, record, scenes, replay buffer)",
  },
  {
    id: "twitch.chat_marker",
    title: "Twitch",
    subtitle: "CHAT",
    category: "Twitch",
    iconName: "MessageSquare",
    actionLabel: "Twitch: Stream Marker & Raid",
    defaultParam: "Create Marker + Clip",
    accentColor: "#8B5CF6",
    bgColor: "#231B38",
    description: "Chat, stream info, raids, commercial breaks",
  },
  {
    id: "youtube.go_live",
    title: "YouTube",
    subtitle: "LIVE",
    category: "YouTube",
    iconName: "Play",
    actionLabel: "YouTube: Manage Broadcast",
    defaultParam: "Studio Live Control",
    accentColor: "#F16B79",
    bgColor: "#2B1419",
    description: "Go live, manage stream markers, poll viewers",
  },
  {
    id: "sys.show_desktop",
    title: "System",
    subtitle: "WIN+D",
    category: "System",
    iconName: "LayoutGrid",
    actionLabel: "System: Apps, Media & Shortcuts",
    defaultParam: "Win+D",
    accentColor: "#56B9F2",
    bgColor: "#132235",
    description: "Apps, media, volume, shortcuts, window focus",
  },
  {
    id: "macro.run_sequence",
    title: "Tools",
    subtitle: "MACRO",
    category: "Streaming",
    iconName: "Zap",
    actionLabel: "Tools: Multi-Step Macro Sequence",
    defaultParam: "macro-go-live",
    accentColor: "#F4B860",
    bgColor: "#2A2215",
    description: "Clipboard, text, delays, conditional scripts",
  },
  {
    id: "plugin.sd_bridge",
    title: "Plugins",
    subtitle: "EXT",
    category: "Plugins",
    iconName: "Puzzle",
    actionLabel: "Plugins: .sdPlugin & Elgato Bridge",
    defaultParam: "com.mirabox.streamdock.system.sdPlugin",
    accentColor: "#35C98B",
    bgColor: "#112822",
    description: "Community plugins and sandboxed extensions",
  },
  {
    id: "audio.mute_mic",
    title: "Mute Mic",
    subtitle: "MUTE",
    category: "Media",
    iconName: "MicOff",
    actionLabel: "Audio: Toggle Microphone Mute",
    defaultParam: "Default Capture Endpoint",
    accentColor: "#F16B79",
    bgColor: "#2A1521",
    description: "Instant hardware/OS microphone mute toggle",
  },
  {
    id: "media.play_pause",
    title: "Spotify",
    subtitle: "PLAY",
    category: "Media",
    iconName: "Music",
    actionLabel: "Media: Play / Pause Track",
    defaultParam: "VK_MEDIA_PLAY_PAUSE",
    accentColor: "#35C98B",
    bgColor: "#112822",
    description: "Control Spotify and system media transport",
  },
];

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
  const [inspectorTab, setInspectorTab] = useState<"Action" | "Appearance" | "Behavior">("Action");
  const [actionCategoryFilter, setActionCategoryFilter] = useState<string>("All");
  const [actionSearch, setActionSearch] = useState<string>("");
  const [canvasZoom, setCanvasZoom] = useState<number>(100);
  const [clipboardKey, setClipboardKey] = useState<KeyConfig | null>(null);

  // Sync & Persistence States (Spec Section 1 & 14: Distinguish Saved Locally vs Applied to Device)
  const [savedLocally, setSavedLocally] = useState<boolean>(true);
  const [deviceSynced, setDeviceSynced] = useState<boolean>(true);
  const [undoStack, setUndoStack] = useState<Profile[][]>([]);
  const [redoStack, setRedoStack] = useState<Profile[][]>([]);
  const [toastMessage, setToastMessage] = useState<{ text: string; kind: "success" | "info" | "warning" } | null>(null);

  // Sub-screen states
  const [deviceDetailTab, setDeviceDetailTab] = useState<"Overview" | "Controls" | "Display" | "Diagnostics" | "Advanced">("Overview");
  const [pluginFilterTab, setPluginFilterTab] = useState<"Discover" | "Installed" | "Updates" | "Built-in Actions">("Discover");
  const [selectedMacroId, setSelectedMacroId] = useState<string>("macro-go-live");
  const [selectedMacroStepIdx, setSelectedMacroStepIdx] = useState<number>(0);
  const [macroRunReport, setMacroRunReport] = useState<MacroRunReport | null>(null);
  const [settingsCategory, setSettingsCategory] = useState<string>("General");
  const [showOnboarding, setShowOnboarding] = useState<boolean>(false);
  const [livePressedKeyIdx, setLivePressedKeyIdx] = useState<number | null>(null);

  const triggerToast = (text: string, kind: "success" | "info" | "warning" = "success") => {
    setToastMessage({ text, kind });
    window.setTimeout(() => {
      setToastMessage((prev) => (prev?.text === text ? null : prev));
    }, 3200);
  };

  useEffect(() => {
    deckforgeApi.getInitialState().then((data) => {
      setDevices(data.devices);
      setProfiles(data.profiles);
      setAutoSwitchRules(data.autoSwitchRules);
      setMacros(data.macros);
      setDiagnostics(data.diagnostics);
      setFrameStats(data.frameCacheStats);
      setLegacySummary(data.legacySummary);
      setTelemetry(data.telemetry);
    });

    let unlistenFn: (() => void) | null = null;
    deckforgeApi.onHardwareInput((evt) => {
      if (evt.kind === "lcdKey") {
        setLivePressedKeyIdx(evt.pressed ? evt.controlIndex : null);
        if (evt.pressed) {
          setSelectedKeyIdx(evt.controlIndex);
          triggerToast(
            `Physical Key #${evt.controlIndex + 1} Pressed (${evt.rawHex})`,
            "success"
          );
        }
      } else {
        triggerToast(
          `Physical Input: ${evt.kind} #${evt.controlIndex + 1} (${evt.rawHex})`,
          "info"
        );
      }
    }).then((fn) => {
      unlistenFn = fn;
    });

    const timer = window.setInterval(() => {
      deckforgeApi.getTelemetry().then(setTelemetry).catch(() => {});
    }, 5000);
    return () => {
      window.clearInterval(timer);
      if (unlistenFn) unlistenFn();
    };
  }, []);

  const handleTakeOverHardware = async () => {
    const msg = await deckforgeApi.takeOverFromOfficialApp();
    const updatedDevs = await deckforgeApi.refreshUsbDevices();
    const updatedTel = await deckforgeApi.getTelemetry();
    setDevices(updatedDevs);
    setTelemetry(updatedTel);
    triggerToast(msg, "success");
  };

  const handleImportLegacyProfiles = async () => {
    const updatedProfiles = await deckforgeApi.importLegacySdProfiles();
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

  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const [customActions, setCustomActions] = useState<ActionTemplate[]>([]);

  const allActionTemplates = useMemo(
    () => [...customActions, ...ACTION_CATALOG],
    [customActions]
  );

  // Keyboard shortcuts (Ctrl+S, Ctrl+Z, Ctrl+Y) + Live Hotkey Recorder
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (isRecordingHotkey) {
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
        updateSelectedKey({
          subtitle: combo,
          actionParam: combo,
          actionId: activeKey?.actionId === "none" ? "custom.hotkey" : activeKey?.actionId,
          actionLabel:
            activeKey?.actionId === "none"
              ? "Custom: Keyboard Shortcut / Hotkey"
              : activeKey?.actionLabel,
        });
        setIsRecordingHotkey(false);
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

  const activeDevice = useMemo(
    () => devices.find((d) => d.id === selectedDeviceId) || devices[0],
    [devices, selectedDeviceId]
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

  const pushUndoSnapshot = () => {
    setUndoStack((prev) => [...prev.slice(-19), JSON.parse(JSON.stringify(profiles))]);
    setRedoStack([]);
    setSavedLocally(false);
    setDeviceSynced(false);
  };

  const handleUndo = () => {
    if (undoStack.length === 0) return;
    const previous = undoStack[undoStack.length - 1];
    setRedoStack((r) => [...r, JSON.parse(JSON.stringify(profiles))]);
    setProfiles(previous);
    setUndoStack((u) => u.slice(0, -1));
    setDeviceSynced(false);
    triggerToast("Undid last key edit", "info");
  };

  const handleRedo = () => {
    if (redoStack.length === 0) return;
    const next = redoStack[redoStack.length - 1];
    setUndoStack((u) => [...u, JSON.parse(JSON.stringify(profiles))]);
    setProfiles(next);
    setRedoStack((r) => r.slice(0, -1));
    setDeviceSynced(false);
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

  const handleSaveLocally = async () => {
    await deckforgeApi.saveProfiles(profiles);
    setSavedLocally(true);
    triggerToast("Profile saved locally to transactional store ✓", "success");
  };

  const handleApplyToDevice = async () => {
    if (!activeDevice || !activeProfile || !activePage) return;
    await deckforgeApi.saveProfiles(profiles);
    setSavedLocally(true);
    const res = await deckforgeApi.applyProfileToDevice(
      activeDevice.id,
      activeProfile.id,
      activePage.id
    );
    setDeviceSynced(true);
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
    const res = await deckforgeApi.testKeyAction(
      activeKey.actionId,
      activeKey.actionLabel,
      activeKey.actionParam
    );
    triggerToast(`${res.outputMessage} (${res.elapsedMs}ms)`, "info");
  };

  const handleClearSelectedKey = async () => {
    if (!activeDevice || !activeKey) return;
    updateSelectedKey({
      title: "Empty",
      subtitle: "",
      iconName: "Plus",
      actionId: "none",
      actionLabel: "No Action Assigned",
      actionParam: "",
      accentColor: "#394158",
      bgColor: "#151925",
    });
    await deckforgeApi.clearDeviceKey(activeDevice.id, activeKey.index + 1);
    triggerToast(`Cleared key #${activeKey.index + 1} (CRT\\0\\0CLE)`, "info");
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
    setSavedLocally(false);
    triggerToast(`Created ${newProf.name}`, "success");
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
    setSavedLocally(false);
    triggerToast(`Added ${newPage.name}`, "info");
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

  if (!activeDevice || !activeProfile || !activePage || !activeKey) {
    return (
      <div className="h-screen w-screen bg-[#0B0D14] text-[#F4F5FA] flex items-center justify-center">
        <div className="flex items-center gap-3 text-[#A6AFC4]">
          <RefreshCw className="animate-spin text-[#8B5CF6]" size={20} />
          <span>Initializing DeckForge Rust Hardware & Action Service...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen w-screen bg-[#0B0D14] text-[#F4F5FA] flex flex-col overflow-hidden select-none">
      {/* Transient Top-Right Toast (Spec Section 11.5) */}
      {toastMessage && (
        <div className="fixed top-14 right-5 z-50 flex items-center gap-2.5 px-3.5 py-2.5 rounded-lg bg-[#1C2130] border border-[#8B5CF6]/60 shadow-2xl text-xs font-medium text-[#F4F5FA]">
          <span className="w-2 h-2 rounded-full bg-[#35C98B]" />
          <span>{toastMessage.text}</span>
        </div>
      )}

      {/* 1. GLOBAL TOP NAVIGATION BAR (48px — Spec Section 3 & Reference Screenshot) */}
      <header className="h-12 bg-[#10131D] border-b border-[#272D3D] px-4 flex items-center justify-between shrink-0">
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
              OpenStreamDeck <span className="text-[#8B5CF6] font-semibold text-xs">DeckForge</span>
            </div>
            <div className="text-[10.5px] text-[#707B94] leading-tight">
              Open-source Stream Deck for All Devices
            </div>
          </div>
        </div>

        {/* Center Navigation Tabs */}
        <nav className="flex items-center gap-1">
          {(
            [
              { id: "workspace", label: "Profiles & Keys", icon: Layers },
              { id: "plugins", label: "Plugins", icon: Puzzle },
              { id: "devices", label: "Devices", icon: Usb },
              { id: "macros", label: "Macro Studio", icon: Zap },
              { id: "icons", label: "Icon Studio", icon: Palette },
              { id: "settings", label: "Settings", icon: Settings },
            ] as Array<{ id: NavScreenId; label: string; icon: React.ComponentType<{ size?: number }> }>
          ).map((tab) => {
            const Icon = tab.icon;
            const isActive = activeScreen === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveScreen(tab.id)}
                className={`h-9 px-3.5 rounded-md text-[12.5px] font-medium flex items-center gap-2 transition-all cursor-pointer border ${
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
            <button
              onClick={handleTakeOverHardware}
              title="Stop official StreamDock.exe so it stops flooding USB and give DeckForge exclusive hardware access"
              className="h-8 px-2.5 rounded-md bg-[#F4B860]/15 border border-[#F4B860]/60 text-[#F4B860] hover:bg-[#F4B860]/25 text-[11.5px] font-semibold flex items-center gap-1.5 cursor-pointer"
            >
              <Zap size={13} />
              <span>Take Over Hardware (Stop Official App)</span>
            </button>
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
            title="Quick Start & Hardware Verification Guide"
            className="w-8 h-8 rounded-md bg-[#151925] border border-[#272D3D] flex items-center justify-center text-[#A6AFC4] hover:text-white cursor-pointer"
          >
            <HelpCircle size={15} />
          </button>
        </div>
      </header>

      {/* 2. SCREEN 01: WORKSPACE / KEY EDITOR (Matches Reference Screenshot + Spec Section 4) */}
      {activeScreen === "workspace" && (
        <div className="flex-1 grid grid-cols-[240px_1fr_320px] overflow-hidden">
          {/* LEFT CONTEXTUAL SIDEBAR: Devices + Profiles */}
          <aside className="bg-[#10131D] border-r border-[#272D3D] p-3.5 flex flex-col gap-5 overflow-y-auto">
            {/* Devices Section */}
            <div>
              <div className="flex items-center justify-between mb-2">
                <span className="text-[12px] font-semibold text-[#F4F5FA]">Devices</span>
                <button
                  onClick={() => {
                    deckforgeApi.refreshUsbDevices().then(setDevices);
                    triggerToast("Rescanned USB HID bus", "info");
                  }}
                  title="Rescan USB Devices"
                  className="w-6 h-6 rounded-md bg-[#151925] border border-[#272D3D] text-[#A6AFC4] hover:text-white flex items-center justify-center cursor-pointer"
                >
                  <Plus size={13} />
                </button>
              </div>

              <div className="flex flex-col gap-1.5">
                {devices.map((dev) => {
                  const isSel = dev.id === activeDevice.id;
                  return (
                    <div
                      key={dev.id}
                      onClick={() => {
                        setSelectedDeviceId(dev.id);
                        setSelectedKeyIdx(0);
                      }}
                      className={`p-2 rounded-lg border flex items-center gap-2.5 cursor-pointer transition-all ${
                        isSel
                          ? "bg-[#30244F] border-[#8B5CF6]/60 shadow-[0_0_16px_rgba(139,92,246,0.2)]"
                          : "bg-[#151925]/60 border-transparent hover:bg-[#151925] hover:border-[#272D3D]"
                      }`}
                    >
                      {/* Mini Key Grid Thumbnail */}
                      <div
                        className={`w-8 h-6 rounded bg-[#111522] border p-1 grid grid-cols-5 gap-0.5 shrink-0 ${
                          isSel ? "border-[#8B5CF6]" : "border-[#394158]"
                        }`}
                      >
                        {Array.from({ length: 10 }).map((_, idx) => (
                          <span
                            key={idx}
                            className={`rounded-[1px] ${
                              isSel ? "bg-[#8B5CF6]" : "bg-[#707B94]/60"
                            }`}
                          />
                        ))}
                      </div>
                      <div className="min-w-0 flex-1">
                        <div className="text-[12px] font-semibold text-[#F4F5FA] truncate">
                          {dev.modelName}
                        </div>
                        <div className="flex items-center gap-1.5 text-[10.5px]">
                          <span
                            className={`w-1.5 h-1.5 rounded-full ${
                              dev.connected ? "bg-[#35C98B]" : "bg-[#707B94]"
                            }`}
                          />
                          <span className={dev.connected ? "text-[#35C98B]" : "text-[#707B94]"}>
                            {dev.connected ? "Connected" : `${dev.keys.count} keys`}
                          </span>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Profiles Section */}
            <div className="flex-1 flex flex-col">
              <div className="flex items-center justify-between mb-2">
                <span className="text-[12px] font-semibold text-[#F4F5FA]">Profiles</span>
                <button
                  onClick={handleAddProfile}
                  title="Create New Profile"
                  className="w-6 h-6 rounded-md bg-[#151925] border border-[#272D3D] text-[#A6AFC4] hover:text-white flex items-center justify-center cursor-pointer"
                >
                  <Plus size={13} />
                </button>
              </div>

              <div className="flex flex-col gap-1">
                {profiles.map((prof) => {
                  const isSel = prof.id === activeProfile.id;
                  return (
                    <div
                      key={prof.id}
                      onClick={() => {
                        setSelectedProfileId(prof.id);
                        setSelectedPageIdx(0);
                        setSelectedKeyIdx(0);
                      }}
                      className={`px-2.5 py-2 rounded-lg border flex items-center justify-between cursor-pointer transition-all ${
                        isSel
                          ? "bg-[#30244F] border-[#8B5CF6]/55 text-[#F4F5FA] font-semibold"
                          : "bg-transparent border-transparent text-[#A6AFC4] hover:bg-[#151925] hover:text-[#F4F5FA]"
                      }`}
                    >
                      <div className="flex items-center gap-2.5 truncate">
                        <DeckIcon
                          name={prof.icon}
                          size={15}
                          color={isSel ? "#8B5CF6" : "#A6AFC4"}
                        />
                        <span className="text-[12.5px] truncate">{prof.name}</span>
                      </div>
                      <span className="text-[#707B94] text-xs">•••</span>
                    </div>
                  );
                })}
              </div>
            </div>
          </aside>

          {/* CENTER WORKSPACE COLUMN: Toolbar + Hardware Bezel + Bottom Action Library Drawer */}
          <main className="flex flex-col justify-between bg-[#0B0D14] overflow-hidden">
            {/* Top Workspace Toolbar */}
            <div className="h-13 px-5 border-b border-[#272D3D] flex items-center justify-between gap-3 shrink-0">
              <div className="flex items-center gap-2">
                <h1 className="text-[16px] font-bold text-[#F4F5FA]">{activeProfile.name}</h1>
                <Edit3 size={14} className="text-[#707B94]" />
              </div>

              <div className="flex items-center gap-2.5">
                {/* Device Selector Pill */}
                <div className="flex items-center gap-2 bg-[#111522] border border-[#272D3D] rounded-md px-2.5 h-9">
                  <LayoutGrid size={14} className="text-[#A6AFC4]" />
                  <span className="text-xs text-[#A6AFC4]">Device:</span>
                  <select
                    value={activeDevice.id}
                    onChange={(e) => {
                      setSelectedDeviceId(e.target.value);
                      setSelectedKeyIdx(0);
                    }}
                    className="bg-transparent text-xs font-semibold text-[#F4F5FA] outline-none cursor-pointer"
                  >
                    {devices.map((d) => (
                      <option key={d.id} value={d.id} className="bg-[#151925] text-[#F4F5FA]">
                        {d.modelName} ({d.keys.count} keys)
                      </option>
                    ))}
                  </select>
                </div>

                {/* Zoom / Fit Controls */}
                <div className="flex items-center bg-[#111522] border border-[#272D3D] rounded-md p-0.5 h-9">
                  {[90, 100, 110].map((z) => (
                    <button
                      key={z}
                      onClick={() => setCanvasZoom(z)}
                      className={`px-2 h-7 rounded text-[11px] font-medium cursor-pointer ${
                        canvasZoom === z
                          ? "bg-[#30244F] text-[#F4F5FA]"
                          : "text-[#A6AFC4] hover:text-white"
                      }`}
                    >
                      {z}%
                    </button>
                  ))}
                </div>

                {/* Save Locally */}
                <button
                  onClick={handleSaveLocally}
                  className="h-9 px-3 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#394158] text-xs font-medium text-[#F4F5FA] flex items-center gap-1.5 cursor-pointer"
                >
                  <Save size={14} />
                  <span>Save</span>
                </button>

                {/* Primary Violet CTA: Apply / Deploy to Device */}
                <button
                  onClick={handleApplyToDevice}
                  className="h-9 px-4 rounded-md bg-[#8B5CF6] hover:bg-[#9D75F8] text-xs font-semibold text-white flex items-center gap-2 shadow-[0_0_20px_rgba(139,92,246,0.4)] cursor-pointer transition-all"
                >
                  <Play size={13} fill="currentColor" />
                  <span>Apply to Device</span>
                </button>
              </div>
            </div>

            {/* Central Hardware Bezel Stage */}
            <div className="flex-1 flex flex-col items-center justify-center px-6 py-3 gap-3 overflow-y-auto">
              <div
                style={{ transform: `scale(${canvasZoom / 100})` }}
                className="bg-[#151925] border border-[#394158] rounded-2xl p-4 shadow-[0_24px_60px_rgba(0,0,0,0.65),inset_0_1px_0_rgba(255,255,255,0.06)] transition-transform max-w-[650px] w-full flex flex-col gap-3"
              >
                {/* Dynamic Key Grid matching real hardware rows x cols */}
                <div
                  className="grid gap-2.5"
                  style={{
                    gridTemplateColumns: `repeat(${activeDevice.keys.cols}, minmax(0, 1fr))`,
                  }}
                >
                  {activePage.keys.slice(0, activeDevice.keys.count).map((k) => {
                    const isSelected = k.index === activeKey.index;
                    const isLivePressed = livePressedKeyIdx === k.index;
                    return (
                      <button
                        key={k.index}
                        onClick={() => setSelectedKeyIdx(k.index)}
                        style={{
                          borderRadius: `${k.cornerRadius}px`,
                          background:
                            k.backgroundStyle === "Gradient"
                              ? `radial-gradient(circle at 50% 28%, ${k.accentColor}44, ${k.bgColor})`
                              : k.bgColor,
                        }}
                        className={`aspect-[1/0.9] border p-2 flex flex-col items-center justify-center gap-1.5 relative cursor-pointer transition-all group ${
                          isLivePressed
                            ? "border-2 border-[#35C98B] shadow-[0_0_24px_rgba(53,201,139,0.75)] scale-95"
                            : isSelected
                            ? "border-2 border-[#8B5CF6] shadow-[0_0_20px_rgba(139,92,246,0.45)] -translate-y-0.5"
                            : "border-[#272D3D] hover:border-[#394158] hover:-translate-y-0.5"
                        }`}
                      >
                        <div
                          className="w-8 h-8 rounded-lg flex items-center justify-center transition-transform group-hover:scale-105"
                          style={{
                            backgroundColor: `${k.accentColor}22`,
                            border: `1px solid ${k.accentColor}55`,
                          }}
                        >
                          <DeckIcon name={k.iconName} size={18} color={k.accentColor} />
                        </div>
                        {k.showTitle && (
                          <span
                            className={`font-semibold text-[#F4F5FA] truncate max-w-full leading-tight ${
                              k.titleSize === "Small"
                                ? "text-[10px]"
                                : k.titleSize === "Large"
                                ? "text-[13px]"
                                : "text-[11.5px]"
                            }`}
                          >
                            {k.title}
                          </span>
                        )}
                      </button>
                    );
                  })}
                </div>

                {/* Secondary Touch Strip (if supported by device, e.g., Ajazz AKP05E Pro) */}
                {activeDevice.touchStrip && (
                  <div className="grid grid-cols-4 gap-2 bg-[#0B0D14] border border-[#272D3D] rounded-xl p-2">
                    {activePage.touchZones.slice(0, activeDevice.touchStrip.zones).map((tz) => (
                      <div
                        key={tz.index}
                        onClick={() =>
                          triggerToast(
                            `Touch Strip Zone #${tz.index + 1} (${tz.label}): CRT\\0\\0DRA ready`,
                            "info"
                          )
                        }
                        className="px-2.5 py-1.5 rounded-lg bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6]/60 cursor-pointer flex items-center justify-between"
                      >
                        <div>
                          <div className="text-[9.5px] font-bold text-[#707B94] tracking-wider">
                            {tz.label}
                          </div>
                          <div
                            className="text-[11px] font-mono font-semibold"
                            style={{ color: tz.accentColor }}
                          >
                            {tz.valueText}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}

                {/* Physical Rotary Encoders (if supported by device, e.g., Ajazz AKP05E Pro / MiraBox N3) */}
                {activeDevice.encoders && (
                  <div
                    className="grid gap-2"
                    style={{
                      gridTemplateColumns: `repeat(${activeDevice.encoders.count}, minmax(0, 1fr))`,
                    }}
                  >
                    {activePage.encoders.slice(0, activeDevice.encoders.count).map((enc) => (
                      <div
                        key={enc.index}
                        onClick={() =>
                          triggerToast(
                            `Rotary Knob K${enc.index + 1}: CCW=${enc.ccwAction} | Push=${enc.pushAction} | CW=${enc.cwAction}`,
                            "info"
                          )
                        }
                        className="px-2.5 py-1.5 rounded-xl bg-[#111522] border border-[#272D3D] hover:border-[#8B5CF6]/60 flex items-center gap-2.5 cursor-pointer"
                      >
                        <div className="w-6 h-6 rounded-full bg-gradient-to-br from-[#394158] to-[#10131D] border border-dashed border-[#A6AFC4]/60 shrink-0" />
                        <div className="min-w-0">
                          <div className="text-[11px] font-semibold text-[#F4F5FA] truncate">
                            {enc.label}
                          </div>
                          <div className="text-[9.5px] font-mono text-[#A6AFC4] truncate">
                            {enc.valueText}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Page Switcher Bar below the Deck Bezel */}
              <div className="flex items-center gap-1.5">
                <div className="flex items-center bg-[#151925] border border-[#272D3D] rounded-md h-7 px-1">
                  <button
                    onClick={() => setSelectedPageIdx((p) => Math.max(0, p - 1))}
                    className="px-1.5 text-[#A6AFC4] hover:text-white cursor-pointer"
                  >
                    <ChevronLeft size={13} />
                  </button>
                  <select
                    value={selectedPageIdx}
                    onChange={(e) => setSelectedPageIdx(Number(e.target.value))}
                    className="bg-transparent text-xs font-medium text-[#F4F5FA] px-2 outline-none cursor-pointer"
                  >
                    {activeProfile.pages.map((pg, idx) => (
                      <option key={pg.id} value={idx} className="bg-[#151925]">
                        {pg.name}
                      </option>
                    ))}
                  </select>
                  <button
                    onClick={() =>
                      setSelectedPageIdx((p) => Math.min(activeProfile.pages.length - 1, p + 1))
                    }
                    className="px-1.5 text-[#A6AFC4] hover:text-white cursor-pointer"
                  >
                    <ChevronRight size={13} />
                  </button>
                </div>

                <button
                  onClick={handleAddPage}
                  title="Add New Page"
                  className="w-7 h-7 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6] text-[#A6AFC4] hover:text-white flex items-center justify-center cursor-pointer"
                >
                  <Plus size={13} />
                </button>
              </div>
            </div>

            {/* BOTTOM ACTION LIBRARY DRAWER (Matches Reference Screenshot Bottom Panel) */}
            <div className="h-[205px] bg-[#10131D] border-t border-[#272D3D] grid grid-cols-[165px_1fr] p-3 gap-4 shrink-0">
              {/* Left Category Filter Rail */}
              <div className="border-r border-[#272D3D] pr-3 flex flex-col gap-0.5 overflow-y-auto">
                <div className="text-[11px] font-semibold text-[#A6AFC4] px-2 mb-1">Actions</div>
                {["All", "Custom", "Streaming", "OBS", "Twitch", "YouTube", "Media", "System", "Plugins"].map(
                  (cat) => {
                    const isAct = actionCategoryFilter === cat;
                    return (
                      <button
                        key={cat}
                        onClick={() => setActionCategoryFilter(cat)}
                        className={`text-left px-2.5 py-1.5 rounded-md text-[11.5px] cursor-pointer transition-colors ${
                          isAct
                            ? "bg-[#30244F] text-[#F4F5FA] font-semibold"
                            : "text-[#A6AFC4] hover:bg-[#151925] hover:text-white"
                        }`}
                      >
                        {cat === "All" ? "All Actions" : cat === "Custom" ? "★ Custom Actions" : cat}
                      </button>
                    );
                  }
                )}
              </div>

              {/* Right Search + 3x2 Action Cards Grid */}
              <div className="flex flex-col gap-2.5 overflow-hidden">
                <div className="relative">
                  <Search
                    size={14}
                    className="absolute left-3 top-1/2 -translate-y-1/2 text-[#707B94]"
                  />
                  <input
                    value={actionSearch}
                    onChange={(e) => setActionSearch(e.target.value)}
                    placeholder="Search actions..."
                    className="w-full h-8 pl-8 pr-3 rounded-md bg-[#111522] border border-[#272D3D] focus:border-[#8B5CF6] text-xs text-[#F4F5FA] outline-none"
                  />
                </div>

                <div className="grid grid-cols-3 gap-2 overflow-y-auto pr-1">
                  {filteredActions.map((act) => (
                    <div
                      key={act.id}
                      onClick={() => {
                        updateSelectedKey({
                          title: act.title,
                          subtitle: act.subtitle,
                          iconName: act.iconName,
                          actionId: act.id,
                          actionLabel: act.actionLabel,
                          actionCategory: act.category,
                          actionParam: act.defaultParam,
                          accentColor: act.accentColor,
                          bgColor: act.bgColor,
                        });
                        triggerToast(
                          `Assigned "${act.actionLabel}" to Key #${activeKey.index + 1}`,
                          "info"
                        );
                      }}
                      className="p-2.5 rounded-lg bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6]/70 hover:bg-[#1C2130] flex items-center justify-between gap-2.5 cursor-pointer transition-all"
                    >
                      <div className="flex items-center gap-2.5 min-w-0">
                        <div
                          className="w-8 h-8 rounded-lg flex items-center justify-center shrink-0"
                          style={{
                            backgroundColor: `${act.accentColor}20`,
                            border: `1px solid ${act.accentColor}50`,
                          }}
                        >
                          <DeckIcon name={act.iconName} size={16} color={act.accentColor} />
                        </div>
                        <div className="min-w-0">
                          <div className="text-[12px] font-semibold text-[#F4F5FA] truncate">
                            {act.title}
                          </div>
                          <div className="text-[10.5px] text-[#707B94] truncate">
                            {act.description}
                          </div>
                        </div>
                      </div>
                      <ChevronRight size={14} className="text-[#707B94] shrink-0" />
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </main>

          {/* RIGHT INSPECTOR PANE (320px — Button Properties matching Reference Screenshot) */}
          <aside className="bg-[#10131D] border-l border-[#272D3D] p-4 flex flex-col justify-between overflow-y-auto">
            <div className="flex flex-col gap-3.5">
              <div className="flex items-center justify-between">
                <h2 className="text-[14px] font-bold text-[#F4F5FA]">Button Properties</h2>
                <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-[#151925] border border-[#272D3D] text-[#A6AFC4]">
                  Key #{activeKey.index + 1}
                </span>
              </div>

              {/* Preview Tile + Title Input Row */}
              <div className="flex items-center gap-3">
                <div
                  style={{
                    borderRadius: `${activeKey.cornerRadius}px`,
                    background:
                      activeKey.backgroundStyle === "Gradient"
                        ? `radial-gradient(circle at 50% 30%, ${activeKey.accentColor}55, ${activeKey.bgColor})`
                        : activeKey.bgColor,
                  }}
                  className="w-15 h-15 border border-[#8B5CF6] flex items-center justify-center shrink-0 shadow-[0_0_16px_rgba(139,92,246,0.25)]"
                >
                  <DeckIcon name={activeKey.iconName} size={26} color={activeKey.accentColor} />
                </div>
                <div className="flex-1">
                  <label className="block text-[11.5px] text-[#A6AFC4] mb-1">Title</label>
                  <input
                    value={activeKey.title}
                    onChange={(e) => updateSelectedKey({ title: e.target.value })}
                    className="w-full h-9 px-2.5 rounded-md bg-[#111522] border border-[#272D3D] focus:border-[#8B5CF6] text-xs text-[#F4F5FA] outline-none"
                  />
                </div>
              </div>

              {/* Action Selector */}
              <div>
                <label className="block text-[11.5px] text-[#A6AFC4] mb-1">Action Type</label>
                <select
                  value={activeKey.actionId}
                  onChange={(e) => {
                    const tpl = allActionTemplates.find((a) => a.id === e.target.value);
                    if (tpl) {
                      updateSelectedKey({
                        actionId: tpl.id,
                        actionLabel: tpl.actionLabel,
                        actionCategory: tpl.category,
                        actionParam: tpl.defaultParam,
                        iconName: tpl.iconName,
                        accentColor: tpl.accentColor,
                        bgColor: tpl.bgColor,
                      });
                    }
                  }}
                  className="w-full h-9 px-2.5 rounded-md bg-[#111522] border border-[#272D3D] focus:border-[#8B5CF6] text-xs text-[#F4F5FA] outline-none cursor-pointer"
                >
                  {allActionTemplates.map((a) => (
                    <option key={a.id} value={a.id} className="bg-[#151925]">
                      {a.actionLabel}
                    </option>
                  ))}
                </select>
              </div>

              {/* Custom Action Parameter + Live Keybind Recorder */}
              <div className="p-2.5 rounded-lg bg-[#151925] border border-[#272D3D] flex flex-col gap-2">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] font-semibold text-[#F4F5FA]">
                    {activeKey.actionId.includes("launch")
                      ? "Application / File Path (.exe)"
                      : activeKey.actionId.includes("shell")
                      ? "PowerShell / CLI Script"
                      : activeKey.actionId.includes("url")
                      ? "Website URL"
                      : activeKey.actionId.includes("http")
                      ? "HTTP Webhook Endpoint"
                      : activeKey.actionId.includes("text")
                      ? "Text Snippet to Paste"
                      : "Shortcut / Command Parameter"}
                  </span>
                  <button
                    onClick={() => setIsRecordingHotkey((r) => !r)}
                    className={`px-2 py-0.5 rounded text-[10.5px] font-semibold cursor-pointer transition-all ${
                      isRecordingHotkey
                        ? "bg-[#F16B79] text-white animate-pulse"
                        : "bg-[#30244F] text-[#8B5CF6] border border-[#8B5CF6]/50 hover:text-white"
                    }`}
                  >
                    {isRecordingHotkey ? "Press Any Key..." : "● Record Keybind"}
                  </button>
                </div>
                <input
                  value={activeKey.actionParam}
                  onChange={(e) => updateSelectedKey({ actionParam: e.target.value })}
                  placeholder="e.g. Ctrl+Shift+M, notepad.exe, https://..."
                  className="w-full h-8 px-2.5 rounded-md bg-[#0B0D14] border border-[#272D3D] focus:border-[#8B5CF6] text-xs font-mono text-[#F4F5FA] outline-none"
                />
                <div className="flex flex-wrap gap-1">
                  {["Ctrl+Shift+M", "Win+Shift+S", "Win+D", "Alt+F4", "VK_MEDIA_PLAY_PAUSE"].map(
                    (preset) => (
                      <button
                        key={preset}
                        onClick={() =>
                          updateSelectedKey({
                            actionParam: preset,
                            subtitle: preset,
                          })
                        }
                        className="px-1.5 py-0.5 rounded bg-[#10131D] border border-[#272D3D] hover:border-[#8B5CF6] text-[10px] font-mono text-[#A6AFC4] hover:text-white cursor-pointer"
                      >
                        {preset}
                      </button>
                    )
                  )}
                </div>
              </div>

              {/* Segmented Tabs: General (Action) | Icon (Appearance) | Style (Behavior) */}
              <div className="grid grid-cols-3 bg-[#111522] border border-[#272D3D] p-0.5 rounded-md">
                {(["Action", "Appearance", "Behavior"] as const).map((tab) => (
                  <button
                    key={tab}
                    onClick={() => setInspectorTab(tab)}
                    className={`h-7 rounded text-[11.5px] font-medium cursor-pointer transition-colors ${
                      inspectorTab === tab
                        ? "bg-[#30244F] text-[#F4F5FA] font-semibold"
                        : "text-[#A6AFC4] hover:text-white"
                    }`}
                  >
                    {tab === "Action" ? "General" : tab === "Appearance" ? "Icon" : "Style"}
                  </button>
                ))}
              </div>

              {inspectorTab === "Action" && (
                <div className="flex flex-col gap-3">
                  {/* Show Title Toggle */}
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-[#A6AFC4]">Show Title</span>
                    <button
                      onClick={() => updateSelectedKey({ showTitle: !activeKey.showTitle })}
                      className={`w-10 h-5 rounded-full p-0.5 transition-colors cursor-pointer ${
                        activeKey.showTitle ? "bg-[#8B5CF6]" : "bg-[#272D3D]"
                      }`}
                    >
                      <div
                        className={`w-4 h-4 rounded-full bg-white transition-transform ${
                          activeKey.showTitle ? "translate-x-5" : "translate-x-0"
                        }`}
                      />
                    </button>
                  </div>

                  {/* Title Size */}
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-xs text-[#A6AFC4]">Title Size</span>
                    <select
                      value={activeKey.titleSize}
                      onChange={(e) =>
                        updateSelectedKey({
                          titleSize: e.target.value as "Small" | "Medium" | "Large",
                        })
                      }
                      className="w-40 h-8 px-2 rounded-md bg-[#111522] border border-[#272D3D] text-xs text-[#F4F5FA] outline-none"
                    >
                      <option value="Small">Small</option>
                      <option value="Medium">Medium</option>
                      <option value="Large">Large</option>
                    </select>
                  </div>

                  {/* Background */}
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-xs text-[#A6AFC4]">Background</span>
                    <select
                      value={activeKey.backgroundStyle}
                      onChange={(e) =>
                        updateSelectedKey({
                          backgroundStyle: e.target.value as "Gradient" | "Solid" | "Image",
                        })
                      }
                      className="w-40 h-8 px-2 rounded-md bg-[#111522] border border-[#272D3D] text-xs text-[#F4F5FA] outline-none"
                    >
                      <option value="Gradient">Gradient</option>
                      <option value="Solid">Solid Surface</option>
                    </select>
                  </div>

                  {/* Gradient Color Swatches */}
                  <div className="flex items-center justify-end gap-2">
                    {[
                      ["#F16B79", "#2A1521"],
                      ["#8B5CF6", "#231B38"],
                      ["#56B9F2", "#132235"],
                      ["#35C98B", "#112822"],
                      ["#F4B860", "#2A2215"],
                    ].map(([acc, bg]) => (
                      <button
                        key={acc}
                        onClick={() => updateSelectedKey({ accentColor: acc, bgColor: bg })}
                        style={{ backgroundColor: acc }}
                        className={`w-6 h-6 rounded-md border cursor-pointer ${
                          activeKey.accentColor === acc
                            ? "border-2 border-white scale-110"
                            : "border-transparent"
                        }`}
                      />
                    ))}
                  </div>

                  {/* Rounded Corners Slider */}
                  <div>
                    <div className="flex items-center justify-between text-xs text-[#A6AFC4] mb-1">
                      <span>Rounded Corners</span>
                      <span className="font-mono text-[#F4F5FA]">{activeKey.cornerRadius} px</span>
                    </div>
                    <input
                      type="range"
                      min={2}
                      max={18}
                      value={activeKey.cornerRadius}
                      onChange={(e) =>
                        updateSelectedKey({ cornerRadius: Number(e.target.value) })
                      }
                      className="w-full accent-[#8B5CF6] cursor-pointer"
                    />
                  </div>
                </div>
              )}

              {inspectorTab === "Appearance" && (
                <div className="flex flex-col gap-2">
                  <span className="text-xs text-[#A6AFC4]">Choose Key Icon (Lucide Vector)</span>
                  <div className="grid grid-cols-5 gap-1.5 max-h-48 overflow-y-auto p-1 bg-[#111522] border border-[#272D3D] rounded-md">
                    {AVAILABLE_ICON_NAMES.map((ico) => (
                      <button
                        key={ico}
                        onClick={() => updateSelectedKey({ iconName: ico })}
                        title={ico}
                        className={`h-9 rounded flex items-center justify-center cursor-pointer ${
                          activeKey.iconName === ico
                            ? "bg-[#30244F] border border-[#8B5CF6] text-white"
                            : "text-[#A6AFC4] hover:bg-[#1C2130] hover:text-white"
                        }`}
                      >
                        <DeckIcon name={ico} size={16} />
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {inspectorTab === "Behavior" && (
                <div className="flex flex-col gap-3">
                  <div>
                    <label className="block text-xs text-[#A6AFC4] mb-1">
                      Command / Parameter
                    </label>
                    <input
                      value={activeKey.actionParam}
                      onChange={(e) => updateSelectedKey({ actionParam: e.target.value })}
                      className="w-full h-8 px-2.5 rounded-md bg-[#111522] border border-[#272D3D] text-xs font-mono text-[#F4F5FA] outline-none"
                    />
                  </div>
                  <button
                    onClick={() => {
                      const newTpl: ActionTemplate = {
                        id: `${activeKey.actionId}.user.${Date.now()}`,
                        title: activeKey.title || "Custom Action",
                        subtitle: activeKey.subtitle || "CUSTOM",
                        category: "Custom",
                        iconName: activeKey.iconName,
                        actionLabel: `Custom: ${activeKey.title} (${activeKey.actionParam})`,
                        defaultParam: activeKey.actionParam,
                        accentColor: activeKey.accentColor,
                        bgColor: activeKey.bgColor,
                        description: `Saved custom preset → ${activeKey.actionParam}`,
                      };
                      setCustomActions((prev) => [newTpl, ...prev]);
                      setActionCategoryFilter("Custom");
                      triggerToast(`Saved "${activeKey.title}" to ★ Custom Actions catalog!`, "success");
                    }}
                    className="w-full h-8 rounded-md bg-[#30244F] border border-[#8B5CF6]/60 hover:bg-[#8B5CF6] text-xs font-semibold text-white flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
                  >
                    <Plus size={13} />
                    <span>Save to ★ Custom Actions Library</span>
                  </button>
                  <div className="flex gap-2">
                    <button
                      onClick={() => {
                        setClipboardKey(JSON.parse(JSON.stringify(activeKey)));
                        triggerToast(`Copied Key #${activeKey.index + 1}`, "info");
                      }}
                      className="flex-1 h-8 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6] text-xs flex items-center justify-center gap-1.5 cursor-pointer"
                    >
                      <Copy size={12} />
                      <span>Copy</span>
                    </button>
                    <button
                      disabled={!clipboardKey}
                      onClick={() => {
                        if (!clipboardKey) return;
                        updateSelectedKey({
                          ...clipboardKey,
                          index: activeKey.index,
                        });
                        triggerToast(`Pasted to Key #${activeKey.index + 1}`, "success");
                      }}
                      className="flex-1 h-8 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6] disabled:opacity-40 text-xs flex items-center justify-center gap-1.5 cursor-pointer"
                    >
                      <span>Paste</span>
                    </button>
                  </div>
                </div>
              )}
            </div>

            {/* Bottom Inspector Buttons: Test Action + Clear Key */}
            <div className="flex flex-col gap-2 pt-3 border-t border-[#272D3D]">
              <button
                onClick={handleTestSelectedAction}
                className="w-full h-9 rounded-md bg-[#151925] border border-[#394158] hover:border-[#8B5CF6] text-xs font-semibold text-[#F4F5FA] flex items-center justify-center gap-2 cursor-pointer transition-colors"
              >
                <Play size={13} />
                <span>Test Action</span>
              </button>
              <button
                onClick={handleClearSelectedKey}
                className="w-full h-8 rounded-md bg-transparent border border-[#272D3D] hover:border-[#F16B79] text-xs text-[#A6AFC4] hover:text-[#F16B79] flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
              >
                <Trash2 size={12} />
                <span>Clear Key</span>
              </button>
            </div>
          </aside>
        </div>
      )}

      {/* 3. SCREEN 02: DEVICE MANAGER & DIAGNOSTICS (Spec Section 5) */}
      {activeScreen === "devices" && (
        <div className="flex-1 grid grid-cols-[310px_1fr] overflow-hidden">
          <aside className="bg-[#10131D] border-r border-[#272D3D] p-4 flex flex-col gap-3 overflow-y-auto">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-bold">Detected & Simulated Hardware</h2>
              <button
                onClick={() => {
                  deckforgeApi.refreshUsbDevices().then(setDevices);
                  triggerToast("USB HID discovery refreshed", "info");
                }}
                className="p-1.5 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6] cursor-pointer"
              >
                <RefreshCw size={13} />
              </button>
            </div>
            {devices.map((dev) => (
              <div
                key={dev.id}
                onClick={() => setSelectedDeviceId(dev.id)}
                className={`p-3 rounded-lg border cursor-pointer transition-all ${
                  dev.id === activeDevice.id
                    ? "bg-[#30244F] border-[#8B5CF6]"
                    : "bg-[#151925] border-[#272D3D] hover:border-[#394158]"
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="font-bold text-xs">{dev.modelName}</span>
                  <span
                    className={`text-[10.5px] px-2 py-0.5 rounded font-medium ${
                      dev.connected
                        ? "bg-[#35C98B]/15 text-[#35C98B]"
                        : "bg-[#272D3D] text-[#A6AFC4]"
                    }`}
                  >
                    {dev.connected ? "● Connected" : "Simulator"}
                  </span>
                </div>
                <div className="text-[11px] font-mono text-[#A6AFC4] mt-1">
                  USB VID:PID {dev.vidPidHex} • {dev.protocolFamily}
                </div>
                <div className="text-[11px] text-[#707B94] mt-1">
                  {dev.keys.rows}×{dev.keys.cols} Keys ({dev.keys.pixelWidth}px)
                  {dev.touchStrip ? ` • ${dev.touchStrip.zones}-Zone Strip` : ""}
                  {dev.encoders ? ` • ${dev.encoders.count} Encoders` : ""}
                </div>
              </div>
            ))}
          </aside>

          <main className="p-6 overflow-y-auto flex flex-col gap-5">
            <div className="flex items-center justify-between border-b border-[#272D3D] pb-4">
              <div>
                <h1 className="text-xl font-bold">{activeDevice.modelName}</h1>
                <p className="text-xs text-[#A6AFC4] font-mono mt-0.5">
                  Driver: {activeDevice.driverName} • Serial: {activeDevice.serialNumber} • FW:{" "}
                  {activeDevice.firmwareVersion}
                </p>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => handleBrightnessChange(activeDevice.brightness)}
                  className="h-9 px-3.5 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6] text-xs font-medium cursor-pointer"
                >
                  Identify Device (CRT..LIG)
                </button>
                <button
                  onClick={() => setActiveScreen("workspace")}
                  className="h-9 px-4 rounded-md bg-[#8B5CF6] hover:bg-[#9D75F8] text-xs font-semibold text-white cursor-pointer"
                >
                  Open Workspace
                </button>
              </div>
            </div>

            {/* 5 Device Detail Sub-tabs per Spec Section 5 */}
            <div className="flex gap-1 bg-[#10131D] border border-[#272D3D] p-1 rounded-lg w-fit">
              {(["Overview", "Controls", "Display", "Diagnostics", "Advanced"] as const).map(
                (tab) => (
                  <button
                    key={tab}
                    onClick={() => setDeviceDetailTab(tab)}
                    className={`px-3.5 py-1.5 rounded-md text-xs font-medium cursor-pointer ${
                      deviceDetailTab === tab
                        ? "bg-[#30244F] text-white font-semibold"
                        : "text-[#A6AFC4] hover:text-white"
                    }`}
                  >
                    {tab}
                  </button>
                )
              )}
            </div>

            {deviceDetailTab === "Overview" && (
              <div className="grid grid-cols-3 gap-4">
                <div className="p-4 rounded-xl bg-[#151925] border border-[#272D3D]">
                  <div className="text-xs text-[#A6AFC4]">Hardware Verification</div>
                  <div className="text-base font-bold mt-1 text-[#35C98B]">
                    {activeDevice.verifiedDriver
                      ? "✓ Verified Physical Driver"
                      : "Simulator / Protocol Profile"}
                  </div>
                  <p className="text-xs text-[#707B94] mt-1">
                    Disassembled from SDLibrary1.dll (65-byte Windows HID reports on MI_00).
                  </p>
                </div>
                <div className="p-4 rounded-xl bg-[#151925] border border-[#272D3D]">
                  <div className="text-xs text-[#A6AFC4]">Anti-Freeze SHA-256 Deduplication</div>
                  <div className="text-base font-bold mt-1 text-[#8B5CF6]">
                    {frameStats.framesDeduplicatedSkipped} Redundant Writes Blocked
                  </div>
                  <p className="text-xs text-[#707B94] mt-1">
                    Last sync latency: {frameStats.lastTransferLatencyMs}ms • Saved{" "}
                    {(frameStats.bytesSaved / 1024).toFixed(1)} KB on USB bus.
                  </p>
                </div>
                <div className="p-4 rounded-xl bg-[#151925] border border-[#272D3D]">
                  <div className="text-xs text-[#A6AFC4]">Hardware Backlight Brightness</div>
                  <div className="flex items-center gap-3 mt-2">
                    <input
                      type="range"
                      min={0}
                      max={100}
                      value={activeDevice.brightness}
                      onChange={(e) => handleBrightnessChange(Number(e.target.value))}
                      className="flex-1 accent-[#8B5CF6] cursor-pointer"
                    />
                    <span className="font-mono text-xs font-bold w-10">
                      {activeDevice.brightness}%
                    </span>
                  </div>
                  <p className="text-xs text-[#707B94] mt-1">
                    Dispatches <code>CRT\0\0LIG</code> (RVA 0x1800225d0)
                  </p>
                </div>
              </div>
            )}

            {deviceDetailTab === "Diagnostics" && (
              <div className="bg-[#151925] border border-[#272D3D] rounded-xl p-4 flex flex-col gap-2">
                <div className="text-xs font-bold text-[#A6AFC4] mb-1">
                  USB HID & Driver Event Log
                </div>
                <div className="flex flex-col gap-1.5 max-h-80 overflow-y-auto font-mono text-xs">
                  {diagnostics.map((d) => (
                    <div
                      key={d.id}
                      className="p-2 rounded bg-[#10131D] border border-[#272D3D] flex items-center justify-between gap-4"
                    >
                      <div>
                        <span className="text-[#8B5CF6] mr-2">[{d.stage}]</span>
                        <span className="text-[#F4F5FA]">{d.message}</span>
                      </div>
                      {d.packetHex && (
                        <span className="text-[10.5px] text-[#35C98B] bg-[#151925] px-2 py-0.5 rounded">
                          {d.packetHex}
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {(deviceDetailTab === "Controls" ||
              deviceDetailTab === "Display" ||
              deviceDetailTab === "Advanced") && (
              <div className="p-4 rounded-xl bg-[#151925] border border-[#272D3D] flex flex-col gap-2">
                <div className="text-sm font-bold">
                  {deviceDetailTab} Configuration ({activeDevice.vidPidHex})
                </div>
                <div className="text-xs text-[#A6AFC4] font-mono">
                  HID Path: {activeDevice.hidPath || "Virtual Simulator Bus"}
                </div>
                <div className="text-xs text-[#A6AFC4]">
                  Key Resolution: {activeDevice.keys.pixelWidth}×{activeDevice.keys.pixelHeight}px @{" "}
                  {activeDevice.keys.rotation}° • Official StreamDock.exe Conflict:{" "}
                  {telemetry?.officialStreamdockRunning
                    ? "Running (Recommend closing official app for exclusive HID read)"
                    : "None (Clean Bus)"}
                </div>
              </div>
            )}
          </main>
        </div>
      )}

      {/* 4. SCREEN 04: PLUGINS & LEGACY .sdPlugin BRIDGE (Spec Section 7) */}
      {activeScreen === "plugins" && (
        <div className="flex-1 p-6 overflow-y-auto max-w-6xl mx-auto w-full flex flex-col gap-5">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-xl font-bold">Actions & Plugin Marketplace</h1>
              <p className="text-xs text-[#A6AFC4] mt-0.5">
                Built-in native Rust/TS integrations + discovered local{" "}
                <code>%APPDATA%\HotSpot\StreamDock\plugins</code> packages.
              </p>
            </div>
            <div className="flex gap-1 bg-[#10131D] border border-[#272D3D] p-1 rounded-lg">
              {(["Discover", "Installed", "Updates", "Built-in Actions"] as const).map((t) => (
                <button
                  key={t}
                  onClick={() => setPluginFilterTab(t)}
                  className={`px-3 py-1.5 rounded text-xs font-medium cursor-pointer ${
                    pluginFilterTab === t
                      ? "bg-[#30244F] text-white font-semibold"
                      : "text-[#A6AFC4] hover:text-white"
                  }`}
                >
                  {t}
                </button>
              ))}
            </div>
          </div>

          {legacySummary && (
            <div className="p-3.5 rounded-xl bg-[#151925] border border-[#8B5CF6]/40 flex items-center justify-between">
              <div className="text-xs">
                <span className="font-bold text-[#35C98B]">
                  ✓ Legacy StreamDock Environment Detected:
                </span>{" "}
                <span className="text-[#F4F5FA]">
                  {legacySummary.legacyProfileCount} <code>.sdProfile</code> folders &{" "}
                  {legacySummary.legacyPluginCount} <code>.sdPlugin</code> packages found on disk.
                </span>
              </div>
              <button
                onClick={handleImportLegacyProfiles}
                className="h-8 px-3 rounded-md bg-[#8B5CF6] hover:bg-[#9D75F8] text-xs font-semibold text-white cursor-pointer"
              >
                Import Legacy .sdProfiles
              </button>
            </div>
          )}

          <div className="grid grid-cols-3 gap-4">
            {(legacySummary?.discoveredPlugins || []).map((plug) => (
              <div
                key={plug.id}
                className="p-4 rounded-xl bg-[#151925] border border-[#272D3D] flex flex-col justify-between gap-3"
              >
                <div>
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-bold text-sm truncate">{plug.name}</span>
                    <span
                      className={`text-[10px] px-2 py-0.5 rounded font-mono ${
                        plug.enabled
                          ? "bg-[#35C98B]/15 text-[#35C98B]"
                          : "bg-[#F4B860]/15 text-[#F4B860]"
                      }`}
                    >
                      v{plug.version}
                    </span>
                  </div>
                  <div className="text-[11px] text-[#707B94] mt-0.5">
                    By {plug.author} • {plug.sourceKind}
                  </div>
                  <p className="text-xs text-[#A6AFC4] mt-2 leading-relaxed">
                    {plug.description}
                  </p>
                </div>
                <div className="pt-2 border-t border-[#272D3D] flex items-center justify-between">
                  <span className="text-[10.5px] text-[#707B94] truncate">
                    Permissions: {plug.permissions.join(", ")}
                  </span>
                  <span
                    className={`text-xs font-semibold ${
                      plug.enabled ? "text-[#35C98B]" : "text-[#F4B860]"
                    }`}
                  >
                    {plug.enabled ? "Active" : "Quarantined"}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 5. SCREEN 05: MACRO STUDIO (Spec Section 8) */}
      {activeScreen === "macros" && (
        <div className="flex-1 grid grid-cols-[240px_1fr_300px] overflow-hidden">
          <aside className="bg-[#10131D] border-r border-[#272D3D] p-4 flex flex-col gap-2">
            <div className="text-xs font-bold text-[#A6AFC4] mb-1">Macro Sequences</div>
            {macros.map((m) => (
              <button
                key={m.id}
                onClick={() => {
                  setSelectedMacroId(m.id);
                  setSelectedMacroStepIdx(0);
                }}
                className={`text-left p-2.5 rounded-lg border cursor-pointer ${
                  m.id === selectedMacroId
                    ? "bg-[#30244F] border-[#8B5CF6] text-white"
                    : "bg-[#151925] border-[#272D3D] text-[#A6AFC4]"
                }`}
              >
                <div className="text-xs font-bold">{m.name}</div>
                <div className="text-[10.5px] text-[#707B94] mt-0.5">{m.steps.length} steps</div>
              </button>
            ))}
          </aside>

          <main className="p-6 overflow-y-auto flex flex-col gap-4">
            {(() => {
              const activeMacro = macros.find((m) => m.id === selectedMacroId) || macros[0];
              if (!activeMacro) return null;
              return (
                <>
                  <div className="flex items-center justify-between border-b border-[#272D3D] pb-3">
                    <div>
                      <h1 className="text-lg font-bold">{activeMacro.name}</h1>
                      <p className="text-xs text-[#A6AFC4]">{activeMacro.description}</p>
                    </div>
                    <button
                      onClick={async () => {
                        const rep = await deckforgeApi.executeMacroSequence(activeMacro.id);
                        setMacroRunReport(rep);
                        triggerToast(
                          `Macro "${rep.macroName}" completed in ${rep.totalElapsedMs}ms`,
                          "success"
                        );
                      }}
                      className="h-9 px-4 rounded-md bg-[#8B5CF6] hover:bg-[#9D75F8] text-xs font-semibold text-white flex items-center gap-2 cursor-pointer"
                    >
                      <Play size={13} />
                      <span>Test Macro</span>
                    </button>
                  </div>

                  <div className="flex flex-col gap-2">
                    {activeMacro.steps.map((st, idx) => (
                      <div
                        key={st.id}
                        onClick={() => setSelectedMacroStepIdx(idx)}
                        className={`p-3 rounded-lg border flex items-center justify-between cursor-pointer ${
                          idx === selectedMacroStepIdx
                            ? "bg-[#1C2130] border-[#8B5CF6]"
                            : "bg-[#151925] border-[#272D3D]"
                        }`}
                      >
                        <div className="flex items-center gap-3">
                          <span className="w-6 h-6 rounded bg-[#30244F] text-[#8B5CF6] text-xs font-bold flex items-center justify-center">
                            {idx + 1}
                          </span>
                          <div>
                            <div className="text-xs font-bold">{st.label}</div>
                            <div className="text-[11px] font-mono text-[#A6AFC4]">
                              Type: {st.stepType} • Param: {st.parameter}
                            </div>
                          </div>
                        </div>
                        <span className="text-[11px] font-mono text-[#707B94]">
                          Timeout: {st.timeoutMs}ms
                        </span>
                      </div>
                    ))}
                  </div>

                  {macroRunReport && (
                    <div className="p-3.5 rounded-xl bg-[#10131D] border border-[#35C98B]/50 font-mono text-xs">
                      <div className="font-bold text-[#35C98B] mb-2">
                        ✓ Test Run Log: {macroRunReport.macroName} ({macroRunReport.totalElapsedMs}
                        ms)
                      </div>
                      {macroRunReport.stepResults.map((sr) => (
                        <div key={sr.stepId} className="text-[#A6AFC4] py-0.5">
                          [{sr.status}] {sr.label} — {sr.detail} ({sr.elapsedMs}ms)
                        </div>
                      ))}
                    </div>
                  )}
                </>
              );
            })()}
          </main>

          <aside className="bg-[#10131D] border-l border-[#272D3D] p-4 flex flex-col gap-3">
            <div className="text-sm font-bold">Step Inspector</div>
            <p className="text-xs text-[#A6AFC4]">
              All macro steps run with bounded timeouts, cancellation tokens, and explicit OS
              permission guards.
            </p>
            <div className="p-3 rounded-lg bg-[#151925] border border-[#272D3D] text-xs flex flex-col gap-1.5">
              <div className="text-[#A6AFC4]">Auto-Switch Foreground Rules Active:</div>
              {autoSwitchRules.map((r) => (
                <div key={r.id} className="font-mono text-[11px] text-[#35C98B]">
                  When {r.processName} → {r.targetProfileId.replace("profile-", "")}
                </div>
              ))}
            </div>
          </aside>
        </div>
      )}

      {/* 6. SCREEN 06: ICON STUDIO (Spec Section 9) */}
      {activeScreen === "icons" && (
        <div className="flex-1 grid grid-cols-[260px_1fr_300px] overflow-hidden">
          <aside className="bg-[#10131D] border-r border-[#272D3D] p-4 overflow-y-auto">
            <div className="text-xs font-bold text-[#A6AFC4] mb-2">
              Built-in Vector Icon Browser
            </div>
            <div className="grid grid-cols-4 gap-2">
              {AVAILABLE_ICON_NAMES.map((ico) => (
                <button
                  key={ico}
                  onClick={() => updateSelectedKey({ iconName: ico })}
                  className={`h-11 rounded-lg border flex flex-col items-center justify-center gap-1 cursor-pointer ${
                    activeKey.iconName === ico
                      ? "bg-[#30244F] border-[#8B5CF6]"
                      : "bg-[#151925] border-[#272D3D] hover:border-[#394158]"
                  }`}
                >
                  <DeckIcon name={ico} size={18} />
                </button>
              ))}
            </div>
          </aside>

          <main className="flex flex-col items-center justify-center gap-6 p-6">
            <div className="flex items-center gap-8">
              {/* High-Res Editor Preview */}
              <div className="flex flex-col items-center gap-2">
                <div
                  style={{
                    borderRadius: `${activeKey.cornerRadius * 2}px`,
                    background: `radial-gradient(circle at 50% 28%, ${activeKey.accentColor}55, ${activeKey.bgColor})`,
                  }}
                  className="w-40 h-40 border-2 border-[#8B5CF6] flex flex-col items-center justify-center gap-3 shadow-2xl"
                >
                  <DeckIcon name={activeKey.iconName} size={52} color={activeKey.accentColor} />
                  {activeKey.showTitle && (
                    <span className="text-base font-bold">{activeKey.title}</span>
                  )}
                </div>
                <span className="text-xs text-[#A6AFC4]">Vector Master Canvas</span>
              </div>

              {/* Hardware-Downsampled Preview (85x85px) */}
              <div className="flex flex-col items-center gap-2">
                <div
                  style={{
                    width: `${activeDevice.keys.pixelWidth}px`,
                    height: `${activeDevice.keys.pixelHeight}px`,
                    borderRadius: `${activeKey.cornerRadius}px`,
                    background: `radial-gradient(circle at 50% 28%, ${activeKey.accentColor}55, ${activeKey.bgColor})`,
                  }}
                  className="border border-[#394158] flex flex-col items-center justify-center gap-1"
                >
                  <DeckIcon name={activeKey.iconName} size={24} color={activeKey.accentColor} />
                  {activeKey.showTitle && (
                    <span className="text-[11px] font-semibold truncate max-w-full px-1">
                      {activeKey.title}
                    </span>
                  )}
                </div>
                <span className="text-xs text-[#707B94] font-mono">
                  Hardware Output ({activeDevice.keys.pixelWidth}×{activeDevice.keys.pixelHeight}px)
                </span>
              </div>
            </div>

            <button
              onClick={handleBatchApplyStyleToPage}
              className="h-9 px-4 rounded-md bg-[#8B5CF6] hover:bg-[#9D75F8] text-xs font-semibold text-white cursor-pointer"
            >
              Batch Apply Style to All Keys on Current Page
            </button>
          </main>

          <aside className="bg-[#10131D] border-l border-[#272D3D] p-4 flex flex-col gap-3">
            <div className="text-sm font-bold">Key Appearance Controls</div>
            <p className="text-xs text-[#A6AFC4]">
              Rust <code>image</code> crate encodes baseline JPEG frames at quality 88 with{" "}
              {activeDevice.keys.rotation}° hardware rotation.
            </p>
          </aside>
        </div>
      )}

      {/* 7. SCREEN 07: SETTINGS (Spec Section 10) */}
      {activeScreen === "settings" && (
        <div className="flex-1 grid grid-cols-[220px_1fr] overflow-hidden">
          <aside className="bg-[#10131D] border-r border-[#272D3D] p-4 flex flex-col gap-1">
            {[
              "General",
              "Appearance",
              "Devices",
              "Actions",
              "Plugins",
              "Data & Backups",
              "Diagnostics",
              "About",
            ].map((cat) => (
              <button
                key={cat}
                onClick={() => setSettingsCategory(cat)}
                className={`text-left px-3 py-2 rounded-md text-xs font-medium cursor-pointer ${
                  settingsCategory === cat
                    ? "bg-[#30244F] text-white font-semibold"
                    : "text-[#A6AFC4] hover:bg-[#151925] hover:text-white"
                }`}
              >
                {cat}
              </button>
            ))}
          </aside>

          <main className="p-6 overflow-y-auto max-w-3xl flex flex-col gap-4">
            <h1 className="text-lg font-bold">{settingsCategory} Settings</h1>
            <div className="p-4 rounded-xl bg-[#151925] border border-[#272D3D] flex flex-col gap-3 text-xs">
              <div className="flex items-center justify-between py-1 border-b border-[#272D3D]">
                <span>Anti-Freeze SHA-256 Frame Deduplication</span>
                <span className="text-[#35C98B] font-semibold">Enabled (Mandatory)</span>
              </div>
              <div className="flex items-center justify-between py-1 border-b border-[#272D3D]">
                <span>Idle USB Polling / Unconditional Redraws</span>
                <span className="text-[#35C98B] font-semibold">0% (Event-Driven Only)</span>
              </div>
              <div className="flex items-center justify-between py-1 border-b border-[#272D3D]">
                <span>External Telemetry / Cloud Account Requirement</span>
                <span className="text-[#35C98B] font-semibold">None (100% Local-First)</span>
              </div>
              <div className="flex items-center justify-between py-1">
                <span>Architecture</span>
                <span className="font-mono text-[#A6AFC4]">
                  Tauri v2 + Rust 1.89 + React 19 + HIDAPI Windows Native
                </span>
              </div>
            </div>
          </main>
        </div>
      )}

      {/* First-Run Onboarding / Hardware Guide Modal (Spec Section 11.1) */}
      {showOnboarding && (
        <div className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4">
          <div className="w-full max-w-lg rounded-xl bg-[#151925] border border-[#394158] p-6 flex flex-col gap-4 shadow-2xl">
            <div className="flex items-center justify-between">
              <h2 className="text-base font-bold">
                DeckForge — Universal Stream Deck Verification Guide
              </h2>
              <button
                onClick={() => setShowOnboarding(false)}
                className="text-xs text-[#A6AFC4] hover:text-white cursor-pointer"
              >
                ✕
              </button>
            </div>
            <div className="text-xs text-[#A6AFC4] space-y-2 leading-relaxed">
              <p>
                <strong className="text-white">1. Verified Hardware Detected:</strong>{" "}
                <code>Ajazz AKP05E Pro (USB\VID_0300&amp;PID_3013)</code> with 10 LCD Keys,
                Secondary Touch Strip, and 4 Rotary Encoders.
              </p>
              <p>
                <strong className="text-white">2. Zero-Freeze Guarantee:</strong> Unlike the
                official app (which re-sends all JPEG keys every 1–3 seconds), DeckForge hashes
                every frame with SHA-256 in Rust and only writes changed keys over USB.
              </p>
              <p>
                <strong className="text-white">3. Saved vs. Applied:</strong> Edits save locally
                first; click <strong>Apply to Device</strong> to push changed keys to hardware.
              </p>
            </div>
            <div className="flex justify-end">
              <button
                onClick={() => setShowOnboarding(false)}
                className="h-9 px-4 rounded-md bg-[#8B5CF6] hover:bg-[#9D75F8] text-xs font-semibold text-white cursor-pointer"
              >
                Got It
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 8. GLOBAL BOTTOM STATUS BAR (28px — Spec Section 3 & Reference Screenshot) */}
      <footer className="h-7 bg-[#10131D] border-t border-[#272D3D] px-4 flex items-center justify-between text-[11px] text-[#A6AFC4] shrink-0">
        <div className="flex items-center gap-4">
          <span className="flex items-center gap-1.5">
            <span
              className={`w-2 h-2 rounded-full ${
                activeDevice.connected ? "bg-[#35C98B]" : "bg-[#F4B860]"
              }`}
            />
            <strong className="text-[#F4F5FA]">{activeDevice.modelName}</strong>
            <span>• {activeDevice.connected ? "Connected" : "Simulator"}</span>
            <span>• {activeDevice.keys.count} keys</span>
          </span>
          <span>• Profile: {activeProfile.name}</span>
          <span className={savedLocally ? "text-[#35C98B]" : "text-[#F4B860]"}>
            • {savedLocally ? "Saved locally ✓" : "Unsaved local edits ●"}
          </span>
          <span className={deviceSynced ? "text-[#35C98B]" : "text-[#F4B860]"}>
            • {deviceSynced ? "Device synced ✓" : "Pending Apply to Device ●"}
          </span>
        </div>

        <div className="flex items-center gap-4 font-mono text-[10.5px]">
          {telemetry && (
            <span className="text-[#707B94]">
              Sys CPU: {telemetry.cpuUsagePercent}% • RAM: {telemetry.memoryUsedGb}/
              {telemetry.memoryTotalGb}GB
            </span>
          )}
          <span>v0.1.0 | Open Source (GPL)</span>
        </div>
      </footer>
    </div>
  );
};
