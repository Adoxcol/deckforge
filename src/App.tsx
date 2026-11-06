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
import { AVAILABLE_ICON_NAMES, DeckIcon } from "./components/DeckIcon";
import { ActionBinding, ObsConnection, actionNeedsParam, splitStoredBinding } from "./components/ActionBinding";
import { IconStudio } from "./components/IconStudio";
import { GameModeStudio } from "./components/GameModeStudio";
import { MacroStudio } from "./components/MacroStudio";
import { PageTemplates } from "./components/PageTemplates";
import { MediaConfigurator } from "./components/MediaConfigurator";
import { ScreenFace } from "./components/ScreenFace";
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
  SystemTelemetrySnapshot,
  TouchZoneConfig,
} from "./types/deckforge";

interface ActionTemplate {
  id: string;
  title: string;
  subtitle: string;
  category: "Custom" | "OBS" | "Media" | "System" | "Navigation" | "Macro";
  iconName: string;
  actionLabel: string;
  defaultParam: string;
  accentColor: string;
  bgColor: string;
  description: string;
}

type ScreenTarget = "key" | "zone" | "strip";

function errorText(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return "Could not read that image";
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

function buildPulseFrames(): string[] {
  const frames: string[] = [];
  for (let i = 0; i < 8; i += 1) {
    const canvas = document.createElement("canvas");
    canvas.width = 112;
    canvas.height = 112;
    const ctx = canvas.getContext("2d");
    if (!ctx) continue;
    ctx.fillStyle = "#1A1028";
    ctx.fillRect(0, 0, 112, 112);
    const radius = 22 + Math.sin((i / 8) * Math.PI * 2) * 10;
    ctx.fillStyle = `hsl(${262 + i * 8} 85% 64%)`;
    ctx.beginPath();
    ctx.arc(56, 46, Math.max(8, radius), 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#F4F5FA";
    ctx.font = "bold 14px sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("PULSE", 56, 96);
    frames.push(canvas.toDataURL("image/jpeg", 0.86));
  }
  return frames;
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
    title: "Stream",
    subtitle: "LIVE",
    category: "OBS",
    iconName: "Radio",
    actionLabel: "OBS: Toggle stream",
    defaultParam: "",
    accentColor: "#F16B79",
    bgColor: "#2A1521",
    description: "Start or stop the OBS stream",
  },
  {
    id: "obs.toggle_record",
    title: "Record",
    subtitle: "REC",
    category: "OBS",
    iconName: "CircleDot",
    actionLabel: "OBS: Toggle recording",
    defaultParam: "",
    accentColor: "#F16B79",
    bgColor: "#2A1521",
    description: "Start or stop OBS recording",
  },
  {
    id: "obs.next_scene",
    title: "Scene +",
    subtitle: "NEXT",
    category: "OBS",
    iconName: "Clapperboard",
    actionLabel: "OBS: Next scene",
    defaultParam: "",
    accentColor: "#56B9F2",
    bgColor: "#132235",
    description: "Move to the next OBS scene",
  },
  {
    id: "obs.prev_scene",
    title: "Scene -",
    subtitle: "PREV",
    category: "OBS",
    iconName: "Film",
    actionLabel: "OBS: Previous scene",
    defaultParam: "",
    accentColor: "#56B9F2",
    bgColor: "#132235",
    description: "Move to the previous OBS scene",
  },
  {
    id: "obs.save_replay",
    title: "Clip",
    subtitle: "CLIP",
    category: "OBS",
    iconName: "Scissors",
    actionLabel: "OBS: Save replay",
    defaultParam: "",
    accentColor: "#8B5CF6",
    bgColor: "#231B38",
    description: "Save the OBS replay buffer",
  },
  {
    id: "media.play_pause",
    title: "Play",
    subtitle: "PLAY",
    category: "Media",
    iconName: "Music",
    actionLabel: "Media: Play / pause",
    defaultParam: "",
    accentColor: "#35C98B",
    bgColor: "#112822",
    description: "Play or pause Spotify, or the current media app",
  },
  {
    id: "media.next",
    title: "Next",
    subtitle: "NEXT",
    category: "Media",
    iconName: "Play",
    actionLabel: "Media: Next track",
    defaultParam: "",
    accentColor: "#35C98B",
    bgColor: "#112822",
    description: "Skip to the next track",
  },
  {
    id: "media.prev",
    title: "Previous",
    subtitle: "PREV",
    category: "Media",
    iconName: "Play",
    actionLabel: "Media: Previous track",
    defaultParam: "",
    accentColor: "#35C98B",
    bgColor: "#112822",
    description: "Go back to the previous track",
  },
  {
    id: "audio.mute_mic",
    title: "Mute Mic",
    subtitle: "MIC",
    category: "Media",
    iconName: "MicOff",
    actionLabel: "Audio: Mute microphone",
    defaultParam: "",
    accentColor: "#F16B79",
    bgColor: "#2A1521",
    description: "Mute or unmute the microphone",
  },
  {
    id: "audio.volume_mute",
    title: "Mute Sound",
    subtitle: "MUTE",
    category: "Media",
    iconName: "Volume2",
    actionLabel: "Audio: Mute speakers",
    defaultParam: "",
    accentColor: "#F4B860",
    bgColor: "#2A2215",
    description: "Mute or unmute the speakers",
  },
  {
    id: "audio.volume_up",
    title: "Vol Up",
    subtitle: "VOL",
    category: "Media",
    iconName: "Volume2",
    actionLabel: "Audio: Volume up",
    defaultParam: "",
    accentColor: "#35C98B",
    bgColor: "#112822",
    description: "Raise the speaker volume",
  },
  {
    id: "audio.volume_down",
    title: "Vol Down",
    subtitle: "VOL",
    category: "Media",
    iconName: "Volume2",
    actionLabel: "Audio: Volume down",
    defaultParam: "",
    accentColor: "#35C98B",
    bgColor: "#112822",
    description: "Lower the speaker volume",
  },
  {
    id: "audio.open_mixer",
    title: "Mixer",
    subtitle: "MIX",
    category: "Media",
    iconName: "Sliders",
    actionLabel: "Audio: Open volume mixer",
    defaultParam: "",
    accentColor: "#8B5CF6",
    bgColor: "#231B38",
    description: "Open the Windows volume mixer",
  },
  {
    id: "profile.next_page",
    title: "Next Page",
    subtitle: "PAGE",
    category: "Navigation",
    iconName: "Layers",
    actionLabel: "Page: Next page",
    defaultParam: "",
    accentColor: "#8B5CF6",
    bgColor: "#231B38",
    description: "Show the next page on the deck",
  },
  {
    id: "profile.prev_page",
    title: "Previous Page",
    subtitle: "PAGE",
    category: "Navigation",
    iconName: "Layers",
    actionLabel: "Page: Previous page",
    defaultParam: "",
    accentColor: "#8B5CF6",
    bgColor: "#231B38",
    description: "Show the previous page on the deck",
  },
  {
    id: "profile.switch_page",
    title: "Open Page",
    subtitle: "PAGE",
    category: "Navigation",
    iconName: "LayoutGrid",
    actionLabel: "Page: Open a page",
    defaultParam: "Page 2",
    accentColor: "#8B5CF6",
    bgColor: "#231B38",
    description: "Jump to a page by name",
  },
  {
    id: "profile.switch",
    title: "Open Profile",
    subtitle: "PROFILE",
    category: "Navigation",
    iconName: "Layers",
    actionLabel: "Profile: Switch profile",
    defaultParam: "profile-streaming",
    accentColor: "#56B9F2",
    bgColor: "#132235",
    description: "Switch the whole deck to another profile",
  },
  {
    id: "macro.run",
    title: "Run Macro",
    subtitle: "MACRO",
    category: "Macro",
    iconName: "Zap",
    actionLabel: "Macro: Run sequence",
    defaultParam: "macro-go-live",
    accentColor: "#8B5CF6",
    bgColor: "#231B38",
    description: "Run a saved macro chain",
  },
  {
    id: "sys.show_desktop",
    title: "Desktop",
    subtitle: "WIN+D",
    category: "System",
    iconName: "Monitor",
    actionLabel: "System: Show desktop",
    defaultParam: "Win+D",
    accentColor: "#56B9F2",
    bgColor: "#132235",
    description: "Show the desktop",
  },
  {
    id: "sys.screenshot",
    title: "Screenshot",
    subtitle: "SNAP",
    category: "System",
    iconName: "Camera",
    actionLabel: "System: Screenshot",
    defaultParam: "Win+Shift+S",
    accentColor: "#56B9F2",
    bgColor: "#132235",
    description: "Open the snipping tool",
  },
  {
    id: "sys.task_manager",
    title: "Task Manager",
    subtitle: "TASK",
    category: "System",
    iconName: "Terminal",
    actionLabel: "System: Task Manager",
    defaultParam: "Ctrl+Shift+Esc",
    accentColor: "#8B5CF6",
    bgColor: "#231B38",
    description: "Open Task Manager",
  },
  {
    id: "sys.lock_pc",
    title: "Lock",
    subtitle: "LOCK",
    category: "System",
    iconName: "Shield",
    actionLabel: "System: Lock PC",
    defaultParam: "",
    accentColor: "#F4B860",
    bgColor: "#2A2215",
    description: "Lock Windows",
  },
  {
    id: "sys.game_mode",
    title: "Game Mode",
    subtitle: "BOOST",
    category: "System",
    iconName: "Zap",
    actionLabel: "System: Game Mode",
    defaultParam: "",
    accentColor: "#F4B860",
    bgColor: "#2A2215",
    description: "Close the apps you picked, keep Bitsum on, and put it all back when you press again",
  },
  {
    id: "brightness.up",
    title: "Brighter",
    subtitle: "LIGHT",
    category: "System",
    iconName: "Lightbulb",
    actionLabel: "Deck: Brightness up",
    defaultParam: "",
    accentColor: "#F4B860",
    bgColor: "#2A2215",
    description: "Raise the deck backlight",
  },
  {
    id: "brightness.down",
    title: "Dimmer",
    subtitle: "LIGHT",
    category: "System",
    iconName: "Lightbulb",
    actionLabel: "Deck: Brightness down",
    defaultParam: "",
    accentColor: "#F4B860",
    bgColor: "#2A2215",
    description: "Lower the deck backlight",
  },
  {
    id: "brightness.toggle",
    title: "Backlight",
    subtitle: "LIGHT",
    category: "System",
    iconName: "Lightbulb",
    actionLabel: "Deck: Toggle backlight",
    defaultParam: "",
    accentColor: "#F4B860",
    bgColor: "#2A2215",
    description: "Turn the deck backlight on or off",
  },
];

type LiveLook = { title: string; icon: string; level: number | null; muted: boolean };

function liveKeyLook(actionId: string, title: string, icon: string, face: DeckFaceState | null): LiveLook {
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

function LevelMeter({ percent, accent, muted }: { percent: number; accent: string; muted: boolean }) {
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

function liveZoneValue(actionId: string, value: string, face: DeckFaceState | null) {
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
  const [face, setFace] = useState<DeckFaceState | null>(null);
  const [obsSettings, setObsSettings] = useState<ObsSettings>({ url: "ws://127.0.0.1:4455", password: "" });

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
      setMediaLibrary(Object.fromEntries((data.mediaLibrary || []).map((item) => [item.id, item])));
      if (data.face) setFace(data.face);
      if (data.obs) setObsSettings(data.obs);
    });

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
      unlistenFn = fn;
    });
    let faceUnlisten: (() => void) | null = null;
    let contextUnlisten: (() => void) | null = null;
    deckforgeApi.onDeckFace((next) => setFace(next)).then((fn) => {
      faceUnlisten = fn;
    });
    deckforgeApi.onDeckContext((ctx) => {
      setSelectedProfileId(ctx.profileId);
      setSelectedPageIdx(ctx.pageIndex);
      triggerToast(ctx.message, "info");
    }).then((fn) => {
      contextUnlisten = fn;
    });

    const timer = window.setInterval(() => {
      deckforgeApi.getTelemetry().then(setTelemetry).catch(() => {});
    }, 5000);
    return () => {
      window.clearInterval(timer);
      if (unlistenFn) unlistenFn();
      if (faceUnlisten) faceUnlisten();
      if (contextUnlisten) contextUnlisten();
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    deckforgeApi.setActiveContext(selectedProfileId, selectedPageIdx).catch(() => {});
  }, [selectedProfileId, selectedPageIdx]);

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

  const [hotkeyTarget, setHotkeyTarget] = useState<"key" | "zone" | "cw" | "ccw" | "push" | null>(null);
  const [customActions, setCustomActions] = useState<ActionTemplate[]>([]);

  const allActionTemplates = useMemo(
    () => [...customActions, ...ACTION_CATALOG],
    [customActions]
  );

  // Keyboard shortcuts (Ctrl+S, Ctrl+Z, Ctrl+Y) + Live Hotkey Recorder
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
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
    deckforgeApi.saveAutoSwitchRules(rules).catch(() => {});
  };

  const saveObs = (settings: ObsSettings) => {
    setObsSettings(settings);
    deckforgeApi.saveObsSettings(settings).catch(() => {});
  };

  const persistMacros = (next: MacroSequence[]) => {
    setMacros(next);
    deckforgeApi.saveMacros(next).catch(() => {});
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
    await deckforgeApi.saveMacros(macros);
    await deckforgeApi.saveProfiles(nextProfiles);
    setSavedLocally(true);
    triggerToast(`${macro.name} is on key ${activeKey.index + 1}`, "success");
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
      media: null,
      backgroundStyle: "Gradient",
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
                {connectedDevices.length === 0 ? (
                  <div className="px-2.5 py-3 rounded-lg border border-dashed border-[#272D3D] text-[11px] leading-relaxed text-[#707B94]">
                    No device connected. Plug in a deck, then rescan.
                  </div>
                ) : (
                  connectedDevices.map((dev) => (
                    <div
                      key={dev.id}
                      className="p-2 rounded-lg border flex items-center gap-2.5 bg-[#30244F] border-[#8B5CF6]/60 shadow-[0_0_16px_rgba(139,92,246,0.2)]"
                    >
                      <div className="w-8 h-6 rounded bg-[#111522] border border-[#8B5CF6] p-1 grid grid-cols-5 gap-0.5 shrink-0">
                        {Array.from({ length: 10 }).map((_, idx) => (
                          <span key={idx} className="rounded-[1px] bg-[#8B5CF6]" />
                        ))}
                      </div>
                      <div className="min-w-0 flex-1">
                        <div className="text-[12px] font-semibold text-[#F4F5FA] truncate">
                          {dev.modelName}
                        </div>
                        <div className="flex items-center gap-1.5 text-[10.5px]">
                          <span className="w-1.5 h-1.5 rounded-full bg-[#35C98B]" />
                          <span className="text-[#35C98B]">Connected</span>
                        </div>
                      </div>
                    </div>
                  ))
                )}
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
                        deckforgeApi.holdAutoSwitch().catch(() => {});
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

            <div className="flex flex-col gap-2">
              <div className="text-[12px] font-semibold text-[#F4F5FA]">When an app is in front</div>
              <p className="text-[10.5px] leading-relaxed text-[#707B94]">
                The saved profile for that app is sent to the deck. Save after you edit a page.
              </p>
              {autoSwitchRules.map((rule) => (
                <div key={rule.id} className="flex flex-col gap-1 rounded-lg border border-[#272D3D] p-2">
                  <div className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={rule.enabled}
                      onChange={(event) =>
                        saveRules(autoSwitchRules.map((item) => (item.id === rule.id ? { ...item, enabled: event.target.checked } : item)))
                      }
                    />
                    <input
                      value={rule.processName}
                      onChange={(event) =>
                        saveRules(autoSwitchRules.map((item) => (item.id === rule.id ? { ...item, processName: event.target.value } : item)))
                      }
                      className="min-w-0 flex-1 h-7 px-2 rounded-md bg-[#111522] border border-[#272D3D] text-[11px] font-mono text-[#F4F5FA] outline-none"
                    />
                    <button
                      type="button"
                      onClick={() => saveRules(autoSwitchRules.filter((item) => item.id !== rule.id))}
                      className="text-[11px] text-[#A6AFC4] cursor-pointer"
                    >
                      Remove
                    </button>
                  </div>
                  <input
                    value={rule.windowTitleContains}
                    placeholder="Window title contains (optional)"
                    onChange={(event) =>
                      saveRules(autoSwitchRules.map((item) => (item.id === rule.id ? { ...item, windowTitleContains: event.target.value } : item)))
                    }
                    className="h-7 px-2 rounded-md bg-[#111522] border border-[#272D3D] text-[11px] text-[#F4F5FA] outline-none"
                  />
                  <select
                    value={rule.targetProfileId}
                    onChange={(event) =>
                      saveRules(autoSwitchRules.map((item) => (item.id === rule.id ? { ...item, targetProfileId: event.target.value } : item)))
                    }
                    className="h-7 px-2 rounded-md bg-[#111522] border border-[#272D3D] text-[11px] text-[#F4F5FA] outline-none"
                  >
                    {profiles.map((profile) => (
                      <option key={profile.id} value={profile.id}>
                        {profile.name}
                      </option>
                    ))}
                  </select>
                </div>
              ))}
              <button
                type="button"
                onClick={() =>
                  saveRules([
                    ...autoSwitchRules,
                    {
                      id: `rule-${Date.now()}`,
                      processName: "Spotify.exe",
                      windowTitleContains: "",
                      targetProfileId: profiles[0]?.id ?? "profile-streaming",
                      priority: autoSwitchRules.length + 1,
                      enabled: true,
                    },
                  ])
                }
                className="h-7 rounded-md border border-[#272D3D] text-[11px] text-[#A6AFC4] cursor-pointer"
              >
                Add app rule
              </button>
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
                  {connectedDevices.length > 1 ? (
                    <select
                      value={activeDevice.id}
                      onChange={(e) => {
                        setSelectedDeviceId(e.target.value);
                        setSelectedKeyIdx(0);
                        setSurface("key");
                        setSurfaceIndex(0);
                      }}
                      className="bg-transparent text-xs font-semibold text-[#F4F5FA] outline-none cursor-pointer"
                    >
                      {connectedDevices.map((d) => (
                        <option key={d.id} value={d.id} className="bg-[#151925] text-[#F4F5FA]">
                          {d.modelName} ({d.keys.count} keys)
                        </option>
                      ))}
                    </select>
                  ) : (
                    <span className="text-xs font-semibold text-[#F4F5FA]">
                      {connectedDevices[0]?.modelName ?? "None connected"}
                    </span>
                  )}
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
                    const isSelected = surface === "key" && k.index === activeKey.index;
                    const isLivePressed = livePressedKeyIdx === k.index;
                    const clip = clipFor(k.media);
                    const look = liveKeyLook(k.actionId, k.title, k.iconName, k.media ? null : face);
                    return (
                      <button
                        key={k.index}
                        onClick={() => {
                          setSurface("key");
                          setSurfaceIndex(k.index);
                          setSelectedKeyIdx(k.index);
                        }}
                        onDragOver={(event) => event.preventDefault()}
                        onDrop={(event) => {
                          event.preventDefault();
                          const file = event.dataTransfer.files?.[0];
                          if (file) void ingestFile(file, "key", k.index);
                        }}
                        style={{
                          borderRadius: `${k.cornerRadius}px`,
                          background:
                            clip
                              ? "#0B0D14"
                              : k.backgroundStyle === "Gradient"
                              ? `radial-gradient(circle at 50% 28%, ${k.accentColor}44, ${k.bgColor})`
                              : k.bgColor,
                        }}
                        className={`aspect-[1/0.9] border p-2 flex flex-col items-center justify-center gap-1.5 relative overflow-hidden cursor-pointer transition-all group ${
                          isLivePressed
                            ? "border-2 border-[#35C98B] shadow-[0_0_24px_rgba(53,201,139,0.75)] scale-95"
                            : isSelected
                            ? "border-2 border-[#8B5CF6] shadow-[0_0_20px_rgba(139,92,246,0.45)] -translate-y-0.5"
                            : "border-[#272D3D] hover:border-[#394158] hover:-translate-y-0.5"
                        }`}
                      >
                        {clip ? (
                          <ScreenFace
                            clip={clip}
                            config={k.media}
                            iconName={look.icon}
                            accent={k.accentColor}
                            title={look.title}
                            showTitle={k.showTitle}
                            titleSize={k.titleSize}
                            className="absolute inset-0"
                          />
                        ) : (
                          <>
                            <div
                              className="w-8 h-8 rounded-lg flex items-center justify-center transition-transform group-hover:scale-105"
                              style={{
                                backgroundColor: `${k.accentColor}22`,
                                border: `1px solid ${k.accentColor}55`,
                              }}
                            >
                              <DeckIcon name={look.icon} size={18} color={k.accentColor} />
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
                                {look.title}
                              </span>
                            )}
                            {look.level != null && (
                              <LevelMeter percent={look.level} accent={k.accentColor} muted={look.muted} />
                            )}
                          </>
                        )}
                      </button>
                    );
                  })}
                </div>

                {/* Secondary Touch Strip (if supported by device, e.g., Ajazz AKP05E Pro) */}
                {activeDevice.touchStrip && (
                  <div className="flex flex-col gap-1.5">
                    <button
                      onClick={() => setSurface("strip")}
                      onDragOver={(event) => event.preventDefault()}
                      onDrop={(event) => {
                        event.preventDefault();
                        const file = event.dataTransfer.files?.[0];
                        if (file) void ingestFile(file, "strip", 0);
                      }}
                      className={`h-7 rounded-md border text-[10.5px] font-semibold cursor-pointer ${
                        surface === "strip"
                          ? "border-[#8B5CF6] bg-[#30244F] text-white"
                          : "border-[#272D3D] bg-[#111522] text-[#A6AFC4]"
                      }`}
                    >
                      Whole touch strip · {activeDevice.touchStrip.pixelWidth}×{activeDevice.touchStrip.pixelHeight}
                      {activePage.stripMedia ? " · image" : ""}
                    </button>
                    <div
                      className="grid gap-2 bg-[#0B0D14] border border-[#272D3D] rounded-xl p-2"
                      style={{ gridTemplateColumns: `repeat(${activeDevice.touchStrip.zones}, minmax(0, 1fr))` }}
                    >
                      {activePage.touchZones.slice(0, activeDevice.touchStrip.zones).map((tz) => {
                        const zoneClip = clipFor(tz.media);
                        const zoneSelected = surface === "zone" && surfaceIndex === tz.index;
                        const zoneValue = liveZoneValue(tz.tapActionId, tz.valueText, tz.media ? null : face);
                        return (
                          <div
                            key={tz.index}
                            onClick={() => {
                              setSurface("zone");
                              setSurfaceIndex(tz.index);
                            }}
                            onDragOver={(event) => event.preventDefault()}
                            onDrop={(event) => {
                              event.preventDefault();
                              const file = event.dataTransfer.files?.[0];
                              if (file) void ingestFile(file, "zone", tz.index);
                            }}
                            className={`relative min-h-14 overflow-hidden px-2.5 py-1.5 rounded-lg bg-[#151925] border cursor-pointer flex items-center justify-between ${
                              zoneSelected ? "border-[#8B5CF6]" : "border-[#272D3D] hover:border-[#8B5CF6]/60"
                            }`}
                          >
                            {zoneClip ? (
                              <ScreenFace
                                clip={zoneClip}
                                config={tz.media}
                                iconName="Image"
                                accent={tz.accentColor}
                                title={tz.label}
                                showTitle
                                className="absolute inset-0"
                              />
                            ) : (
                              <div>
                                <div className="text-[9.5px] font-bold text-[#707B94] tracking-wider">
                                  {tz.label}
                                </div>
                                <div
                                  className="text-[11px] font-mono font-semibold"
                                  style={{ color: tz.accentColor }}
                                >
                                  {zoneValue}
                                </div>
                              </div>
                            )}
                          </div>
                        );
                      })}
                    </div>
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
                    {activePage.encoders.slice(0, activeDevice.encoders.count).map((enc) => {
                      const volumeKnob = [enc.cwAction, enc.ccwAction, enc.pushAction].some((action) =>
                        action.includes("volume")
                      );
                      const knobValue =
                        volumeKnob && face?.volumePercent != null
                          ? `${face.volumePercent}%${face.speakerMuted ? " muted" : ""}`
                          : enc.valueText;
                      return (
                      <div
                        key={enc.index}
                        onClick={() => {
                          setSurface("encoder");
                          setSurfaceIndex(enc.index);
                        }}
                        className={`px-2.5 py-1.5 rounded-xl bg-[#111522] border flex items-center gap-2.5 cursor-pointer ${
                          surface === "encoder" && surfaceIndex === enc.index
                            ? "border-[#8B5CF6]"
                            : "border-[#272D3D] hover:border-[#8B5CF6]/60"
                        }`}
                      >
                        <div className="w-6 h-6 rounded-full bg-gradient-to-br from-[#394158] to-[#10131D] border border-dashed border-[#A6AFC4]/60 shrink-0" />
                        <div className="min-w-0">
                          <div className="text-[11px] font-semibold text-[#F4F5FA] truncate">
                            {enc.label}
                          </div>
                          <div className="text-[9.5px] font-mono text-[#A6AFC4] truncate">
                            {knobValue}
                          </div>
                          {volumeKnob && face?.volumePercent != null && (
                            <LevelMeter percent={face.volumePercent} accent="#35C98B" muted={face.speakerMuted === true} />
                          )}
                        </div>
                      </div>
                      );
                    })}
                  </div>
                )}
              </div>

              <div className="text-[11px] text-[#707B94]">
                {activeDevice.keys.count} LCD screens · {activeDevice.keys.pixelWidth}×{activeDevice.keys.pixelHeight}px
                {activeDevice.keys.rotation ? ` · ${activeDevice.keys.rotation}°` : ""}
                {activeDevice.touchStrip
                  ? ` · strip ${activeDevice.touchStrip.pixelWidth}×${activeDevice.touchStrip.pixelHeight}`
                  : ""}
                . Drop a GIF onto any screen.
              </div>

              {/* Page Switcher Bar below the Deck Bezel */}
              <div className="flex items-center gap-1.5">
                <div className="flex items-center bg-[#151925] border border-[#272D3D] rounded-md h-7 px-1">
                  <button
                    onClick={() => {
                      deckforgeApi.holdAutoSwitch().catch(() => {});
                      setSelectedPageIdx((p) => Math.max(0, p - 1));
                    }}
                    className="px-1.5 text-[#A6AFC4] hover:text-white cursor-pointer"
                  >
                    <ChevronLeft size={13} />
                  </button>
                  <select
                    value={selectedPageIdx}
                    onChange={(e) => {
                      deckforgeApi.holdAutoSwitch().catch(() => {});
                      setSelectedPageIdx(Number(e.target.value));
                    }}
                    className="bg-transparent text-xs font-medium text-[#F4F5FA] px-2 outline-none cursor-pointer"
                  >
                    {activeProfile.pages.map((pg, idx) => (
                      <option key={pg.id} value={idx} className="bg-[#151925]">
                        {pg.name}
                      </option>
                    ))}
                  </select>
                  <button
                    onClick={() => {
                      deckforgeApi.holdAutoSwitch().catch(() => {});
                      setSelectedPageIdx((p) => Math.min(activeProfile.pages.length - 1, p + 1));
                    }}
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
                <button
                  onClick={() => setTemplatesOpen(true)}
                  title="Page templates"
                  aria-label="Page templates"
                  className="h-7 px-2 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6] text-[#A6AFC4] hover:text-white flex items-center justify-center gap-1.5 text-[11px] font-medium cursor-pointer"
                >
                  <LayoutTemplate size={13} /> Templates
                </button>
              </div>
            </div>

            {/* BOTTOM ACTION LIBRARY DRAWER (Matches Reference Screenshot Bottom Panel) */}
            <div className="h-[205px] bg-[#10131D] border-t border-[#272D3D] grid grid-cols-[165px_1fr] p-3 gap-4 shrink-0">
              {/* Left Category Filter Rail */}
              <div className="border-r border-[#272D3D] pr-3 flex flex-col gap-0.5 overflow-y-auto">
                <div className="text-[11px] font-semibold text-[#A6AFC4] px-2 mb-1">Actions</div>
                {["All", "Custom", "Macro", "OBS", "Media", "System", "Navigation"].map(
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
                        if (surface === "zone" && activeZone) {
                          updateSelectedZone(activeZone.index, {
                            label: act.title,
                            valueText: act.subtitle,
                            tapActionId: act.id,
                            tapActionParam: actionNeedsParam(act.id) ? act.defaultParam : "",
                            accentColor: act.accentColor,
                          });
                          triggerToast(`Assigned "${act.title}" to zone #${activeZone.index + 1}`, "info");
                          return;
                        }
                        if (surface === "encoder" && activeEncoder) {
                          updateSelectedEncoder(activeEncoder.index, {
                            label: act.title,
                            pushAction: act.id,
                            pushParam: actionNeedsParam(act.id) ? act.defaultParam : "",
                          });
                          triggerToast(`Assigned "${act.title}" to the knob press`, "info");
                          return;
                        }
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
                <h2 className="text-[14px] font-bold text-[#F4F5FA]">
                  {surface === "zone"
                    ? "Touch Zone"
                    : surface === "strip"
                    ? "Touch Strip"
                    : surface === "encoder"
                    ? "Rotary Encoder"
                    : "Button Properties"}
                </h2>
                <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-[#151925] border border-[#272D3D] text-[#A6AFC4]">
                  {surface === "zone"
                    ? `Zone #${(activeZone?.index ?? 0) + 1}`
                    : surface === "strip"
                    ? `${activeDevice.touchStrip?.pixelWidth ?? 0}×${activeDevice.touchStrip?.pixelHeight ?? 0}`
                    : surface === "encoder"
                    ? `Knob #${(activeEncoder?.index ?? 0) + 1}`
                    : `Key #${activeKey.index + 1}`}
                </span>
              </div>

              {surface === "key" && (
              <div className="flex items-center gap-3">
                <div
                  style={{
                    borderRadius: `${activeKey.cornerRadius}px`,
                    background:
                      clipFor(activeKey.media)
                        ? "#0B0D14"
                        : activeKey.backgroundStyle === "Gradient"
                        ? `radial-gradient(circle at 50% 30%, ${activeKey.accentColor}55, ${activeKey.bgColor})`
                        : activeKey.bgColor,
                  }}
                  className="relative w-16 h-16 overflow-hidden border border-[#8B5CF6] flex items-center justify-center shrink-0 shadow-[0_0_16px_rgba(139,92,246,0.25)]"
                  onDragOver={(event) => event.preventDefault()}
                  onDrop={(event) => {
                    event.preventDefault();
                    const file = event.dataTransfer.files?.[0];
                    if (file) void ingestFile(file, "key", activeKey.index);
                  }}
                >
                  {clipFor(activeKey.media) ? (
                    <ScreenFace
                      clip={clipFor(activeKey.media)}
                      config={activeKey.media}
                      iconName={activeKey.iconName}
                      accent={activeKey.accentColor}
                      title={activeKey.title}
                      showTitle={activeKey.showTitle}
                      titleSize={activeKey.titleSize}
                      iconSize={26}
                      className="absolute inset-0"
                    />
                  ) : (
                    <DeckIcon
                      name={liveKeyLook(activeKey.actionId, activeKey.title, activeKey.iconName, activeKey.media ? null : face).icon}
                      size={26}
                      color={activeKey.accentColor}
                    />
                  )}
                </div>
                <div className="flex-1">
                  <label className="block text-[11.5px] text-[#A6AFC4] mb-1">Title</label>
                  <input
                    value={activeKey.title}
                    onChange={(e) => updateSelectedKey({ title: e.target.value })}
                    className="w-full h-9 px-2.5 rounded-md bg-[#111522] border border-[#272D3D] focus:border-[#8B5CF6] text-xs text-[#F4F5FA] outline-none"
                  />
                  {(() => {
                    const look = liveKeyLook(activeKey.actionId, activeKey.title, activeKey.iconName, activeKey.media ? null : face);
                    if (look.title === activeKey.title && look.level == null) return null;
                    return (
                      <div className="mt-1.5 flex flex-col gap-1">
                        <div className="text-[10.5px] text-[#35C98B] truncate">Live · {look.title}</div>
                        {look.level != null && <LevelMeter percent={look.level} accent={activeKey.accentColor} muted={look.muted} />}
                      </div>
                    );
                  })()}
                </div>
              </div>
              )}

              {surface !== "encoder" && (
                <MediaConfigurator
                  title={
                    surface === "strip"
                      ? "Strip image"
                      : surface === "zone"
                      ? "Zone image"
                      : "Key screen"
                  }
                  pixelLabel={
                    surface === "strip" || surface === "zone"
                      ? `Touch strip ${activeDevice.touchStrip?.pixelWidth ?? 0}×${activeDevice.touchStrip?.pixelHeight ?? 0}px`
                      : `Key ${activeDevice.keys.pixelWidth}×${activeDevice.keys.pixelHeight}px · ${activeDevice.keys.rotation}°`
                  }
                  config={currentMedia()}
                  clip={clipFor(currentMedia())}
                  onUpload={() => fileRef.current?.click()}
                  onSample={() => {
                    const target: ScreenTarget = surface === "zone" || surface === "strip" ? surface : "key";
                    const index = target === "key" ? activeKey.index : surfaceIndex;
                    void useSampleAnimation(target, index);
                  }}
                  onChange={patchCurrentMedia}
                  onClear={() => {
                    const target: ScreenTarget = surface === "zone" || surface === "strip" ? surface : "key";
                    const index = target === "key" ? activeKey.index : surfaceIndex;
                    assignMedia(target, index, null);
                  }}
                />
              )}

              {surface === "zone" && activeZone && (
                <div className="flex flex-col gap-2">
                  <label className="text-[11.5px] text-[#A6AFC4]">Zone label</label>
                  <input
                    value={activeZone.label}
                    onChange={(e) => updateSelectedZone(activeZone.index, { label: e.target.value })}
                    className="w-full h-8 px-2.5 rounded-md bg-[#111522] border border-[#272D3D] text-xs text-[#F4F5FA] outline-none"
                  />
                  <label className="text-[11.5px] text-[#A6AFC4]">Value text</label>
                  <input
                    value={activeZone.valueText}
                    onChange={(e) => updateSelectedZone(activeZone.index, { valueText: e.target.value })}
                    className="w-full h-8 px-2.5 rounded-md bg-[#111522] border border-[#272D3D] text-xs text-[#F4F5FA] outline-none"
                  />
                  <ActionBinding
                    label="Tap"
                    actionId={activeZone.tapActionId}
                    actionParam={activeZone.tapActionParam ?? ""}
                    choices={allActionTemplates}
                    recording={hotkeyTarget === "zone"}
                    onRecord={() => setHotkeyTarget("zone")}
                    onChange={(actionId, actionParam) => {
                      const tpl = allActionTemplates.find((item) => item.id === actionId);
                      updateSelectedZone(activeZone.index, {
                        tapActionId: actionId,
                        tapActionParam: actionParam,
                        ...(tpl && actionId !== activeZone.tapActionId
                          ? { label: tpl.title, valueText: tpl.subtitle, accentColor: tpl.accentColor }
                          : {}),
                      });
                    }}
                    paramOptions={paramOptionsFor(activeZone.tapActionId)}
                  />
                  {activeZone.tapActionId.startsWith("obs.") && (
                    <ObsConnection obsSettings={obsSettings} onSave={saveObs} />
                  )}
                </div>
              )}

              {surface === "strip" && (
                <p className="text-[11px] leading-relaxed text-[#A6AFC4]">
                  A full-strip image covers all zones. Leave it empty to keep the zone cards, and upload a GIF on an individual zone if only that part should animate.
                </p>
              )}

              {surface === "encoder" && activeEncoder && (
                <div className="flex flex-col gap-2">
                  <label className="text-[11.5px] text-[#A6AFC4]">Label</label>
                  <input
                    value={activeEncoder.label}
                    onChange={(e) => updateSelectedEncoder(activeEncoder.index, { label: e.target.value })}
                    className="w-full h-8 px-2.5 rounded-md bg-[#111522] border border-[#272D3D] text-xs text-[#F4F5FA] outline-none"
                  />
                  {(["cw", "ccw", "push"] as const).map((field) => {
                    const stored = field === "cw" ? activeEncoder.cwAction : field === "ccw" ? activeEncoder.ccwAction : activeEncoder.pushAction;
                    const param = field === "cw" ? activeEncoder.cwParam : field === "ccw" ? activeEncoder.ccwParam : activeEncoder.pushParam;
                    const split = splitStoredBinding(stored, param, allActionTemplates.map((item) => item.id));
                    return (
                      <ActionBinding
                        key={field}
                        label={field === "cw" ? "Turn right" : field === "ccw" ? "Turn left" : "Press"}
                        actionId={split.actionId}
                        actionParam={split.actionParam}
                        choices={allActionTemplates}
                        recording={hotkeyTarget === field}
                        onRecord={() => setHotkeyTarget(field)}
                        onChange={(actionId, actionParam) => {
                          const patch =
                            field === "cw"
                              ? { cwAction: actionId, cwParam: actionParam }
                              : field === "ccw"
                              ? { ccwAction: actionId, ccwParam: actionParam }
                              : { pushAction: actionId, pushParam: actionParam };
                          updateSelectedEncoder(activeEncoder.index, patch);
                        }}
                        paramOptions={paramOptionsFor(split.actionId)}
                      />
                    );
                  })}
                  {[activeEncoder.cwAction, activeEncoder.ccwAction, activeEncoder.pushAction].some((action) =>
                    action.startsWith("obs.")
                  ) && <ObsConnection obsSettings={obsSettings} onSave={saveObs} />}
                </div>
              )}

              {surface === "key" && (
              <>
              <ActionBinding
                label="Action"
                actionId={activeKey.actionId}
                actionParam={activeKey.actionParam}
                choices={allActionTemplates}
                recording={hotkeyTarget === "key"}
                onRecord={() => setHotkeyTarget("key")}
                onChange={(actionId, actionParam) => {
                  const tpl = allActionTemplates.find((item) => item.id === actionId);
                  const macro = actionId === "macro.run" ? macros.find((item) => item.id === actionParam) : undefined;
                  updateSelectedKey({
                    actionId,
                    actionParam,
                    ...(macro
                      ? {
                          title: macro.name,
                          subtitle: "MACRO",
                          actionLabel: `Macro: ${macro.name}`,
                          actionCategory: "Macro",
                          iconName: "Zap",
                          accentColor: "#8B5CF6",
                          bgColor: "#231B38",
                        }
                      : tpl && actionId !== activeKey.actionId
                      ? {
                          actionLabel: tpl.actionLabel,
                          actionCategory: tpl.category,
                          iconName: tpl.iconName,
                          accentColor: tpl.accentColor,
                          bgColor: tpl.bgColor,
                          subtitle: actionNeedsParam(actionId) ? actionParam : tpl.subtitle,
                        }
                      : {}),
                  });
                }}
                paramOptions={paramOptionsFor(activeKey.actionId)}
              />
              {activeKey.actionId.startsWith("obs.") && (
                <ObsConnection obsSettings={obsSettings} onSave={saveObs} />
              )}

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
                      <option value="Image">Image / GIF</option>
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
              </>
              )}
            </div>

            {surface === "key" && (
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
            )}
          </aside>
        </div>
      )}

      {/* 3. SCREEN 02: DEVICE MANAGER & DIAGNOSTICS (Spec Section 5) */}
      {activeScreen === "devices" && (
        <div className="flex-1 grid grid-cols-[310px_1fr] overflow-hidden">
          <aside className="bg-[#10131D] border-r border-[#272D3D] p-4 flex flex-col gap-3 overflow-y-auto">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-bold">Connected</h2>
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
            {connectedDevices.length === 0 ? (
              <div className="px-3 py-4 rounded-lg border border-dashed border-[#272D3D] text-[11px] leading-relaxed text-[#707B94]">
                No device connected. Plug in a deck, then rescan.
              </div>
            ) : (
              connectedDevices.map((dev) => (
                <div
                  key={dev.id}
                  onClick={() => {
                    setSelectedDeviceId(dev.id);
                    setSelectedKeyIdx(0);
                    setSurface("key");
                    setSurfaceIndex(0);
                  }}
                  className={`p-3 rounded-lg border cursor-pointer transition-all ${
                    dev.id === activeDevice.id
                      ? "bg-[#30244F] border-[#8B5CF6]"
                      : "bg-[#151925] border-[#272D3D] hover:border-[#394158]"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="font-bold text-xs">{dev.modelName}</span>
                    <span className="text-[10.5px] px-2 py-0.5 rounded font-medium bg-[#35C98B]/15 text-[#35C98B]">
                      ● Connected
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
              ))
            )}
          </aside>

          <main className="p-6 overflow-y-auto flex flex-col gap-5">
            {connectedDevices.length === 0 ? (
              <p className="text-sm text-[#A6AFC4]">
                Plug in a deck to see its driver, brightness, and diagnostics.
              </p>
            ) : (
            <>
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
                  Key screens: {activeDevice.keys.count} × {activeDevice.keys.pixelWidth}×{activeDevice.keys.pixelHeight}px @{" "}
                  {activeDevice.keys.rotation}°
                  {activeDevice.touchStrip
                    ? ` • Touch strip ${activeDevice.touchStrip.pixelWidth}×${activeDevice.touchStrip.pixelHeight}px`
                    : ""}
                  . PNG, JPEG, WebP, and GIF uploads are fitted to those pixels. GIF playback is paced at 80ms or slower.
                </div>
              </div>
            )}
            </>
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
            const next = macros.some((item) => item.id === macro.id)
              ? macros.map((item) => (item.id === macro.id ? macro : item))
              : [macro, ...macros];
            setMacros(next);
            await deckforgeApi.saveMacros(next);
            const report = await deckforgeApi.executeMacroSequence(macro.id);
            setMacroRunReport(report);
            triggerToast(
              `${report.macroName} finished in ${report.totalElapsedMs}ms`,
              report.stepResults.some((step) => step.status !== "Passed") ? "warning" : "success"
            );
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
                connectedDevices.length > 0 ? "bg-[#35C98B]" : "bg-[#707B94]"
              }`}
            />
            <strong className="text-[#F4F5FA]">
              {connectedDevices.length > 0 ? activeDevice.modelName : "No device connected"}
            </strong>
            {connectedDevices.length > 0 && (
              <>
                <span>• Connected</span>
                <span>• {activeDevice.keys.count} keys</span>
              </>
            )}
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
