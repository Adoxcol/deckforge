import { createSharedRead } from "../lib/sharedRead";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ActionExecutionResult,
  DeviceDescriptor,
  FullBootstrapPayload,
  HardwareInputEvent,
  KeyConfig,
  MacroRunReport,
  MacroSequence,
  DeckContextEvent,
  DeckFaceState,
  GameModeConfig,
  GameModePowerPlan,
  GameModeRunningApp,
  GameModeScan,
  GameModeServicePick,
  GameModeStatus,
  GameModeTarget,
  MediaPreview,
  CommonsGif,
  ObsSettings,
  Profile,
  AutoSwitchRule,
  SyncKeyResult,
  SystemTelemetrySnapshot,
  DeckStatus,
  ProfilePage,
  WorkspaceStatus,
  RestoredBackup,
  RecoveryDraft,
  DraftSnapshot,
  PluginView,
  PluginInstalled,
  GithubPlugin,
  AudioOutput,
} from "../types/deckforge";

/** A sample plugin so the browser preview has something to show. */
let mockPlugins: PluginView[] = [
  {
    uuid: "com.example.counter",
    name: "Counter",
    author: "Example",
    version: "1.0.0",
    description: "Counts key presses (browser preview sample).",
    source: "Stream Dock",
    dir: "",
    runtime: "node",
    enabled: false,
    running: false,
    problem: null,
    icon: null,
    actions: [{ uuid: "com.example.counter.add", name: "Add one", tooltip: "Adds one on each press" }],
  },
];

export const isTauriRuntime = (): boolean => {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
};

const buildDefaultKeys = (): KeyConfig[] => {
  const presets: Array<[string, string, string, string, string, string, string, string, string]> = [
    ["Start Stream", "LIVE", "Radio", "obs.start_stream", "OBS: Start Streaming", "OBS", "Main Scene", "#F16B79", "#2A1521"],
    ["Scene +", "NEXT", "Clapperboard", "obs.next_scene", "OBS: Next Scene", "OBS", "Scene Collection", "#56B9F2", "#132235"],
    ["Scene -", "PREV", "Film", "obs.prev_scene", "OBS: Previous Scene", "OBS", "Scene Collection", "#56B9F2", "#132235"],
    ["Mute Mic", "MUTE", "MicOff", "audio.mute_mic", "Audio: Toggle Mic Mute", "Media", "VK_VOLUME_MUTE", "#F16B79", "#2A1521"],
    ["Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Master Volume Up", "Media", "VK_VOLUME_UP", "#35C98B", "#112822"],
    ["Record", "REC", "CircleDot", "obs.toggle_record", "OBS: Toggle Recording", "OBS", "MKV High", "#F16B79", "#2A1521"],
    ["Clip 60s", "CLIP", "Scissors", "obs.save_replay", "OBS: Save Replay Buffer", "Streaming", "Last 60s", "#8B5CF6", "#231B38"],
    ["Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Capture Screen (Win+Shift+S)", "System", "Win+Shift+S", "#56B9F2", "#132235"],
    ["Task Mgr", "TASK", "Terminal", "sys.hotkey", "System: Task Manager (Ctrl+Shift+Esc)", "System", "Ctrl+Shift+Esc", "#8B5CF6", "#231B38"],
    ["Show Desktop", "DESK", "Monitor", "sys.show_desktop", "System: Show Desktop (Win+D)", "System", "Win+D", "#56B9F2", "#132235"],
    ["Explorer", "FILE", "Folder", "sys.hotkey", "System: File Explorer (Win+E)", "System", "Win+E", "#F4B860", "#2A2215"],
    ["Audio Mixer", "MIX", "Sliders", "audio.open_mixer", "Audio: Open Volume Mixer (sndvol.exe)", "Media", "sndvol.exe", "#8B5CF6", "#231B38"],
    ["Spotify", "PLAY", "Music", "media.play_pause", "Media: Play / Pause Track", "Media", "VK_MEDIA_PLAY_PAUSE", "#35C98B", "#112822"],
    ["Settings", "WIN+I", "Settings", "sys.hotkey", "System: Windows Settings (Win+I)", "System", "Win+I", "#56B9F2", "#132235"],
    ["Diagnostics", "SYS", "LayoutGrid", "profile.switch_page", "Navigation: Open Diagnostics Page", "System", "Page 2", "#8B5CF6", "#231B38"],
  ];

  return Array.from({ length: 32 }, (_, i) => {
    const p = presets[i % presets.length];
    return {
      index: i,
      title: p[0],
      subtitle: p[1],
      iconName: p[2],
      actionId: p[3],
      actionLabel: p[4],
      actionCategory: p[5],
      actionParam: p[6],
      accentColor: p[7],
      bgColor: p[8],
      showTitle: true,
      titleSize: "Medium",
      backgroundStyle: "Gradient",
      cornerRadius: 8,
    };
  });
};

const paintKeys = (
  specs: Array<[string, string, string, string, string, string, string, string, string]>,
): KeyConfig[] =>
  Array.from({ length: 32 }, (_, i) => {
    const p = specs[i % specs.length];
    return {
      index: i,
      title: p[0],
      subtitle: p[1],
      iconName: p[2],
      actionId: p[3],
      actionLabel: p[4],
      actionCategory: p[5],
      actionParam: p[6],
      accentColor: p[7],
      bgColor: p[8],
      showTitle: true,
      titleSize: "Medium",
      backgroundStyle: "Gradient",
      cornerRadius: 8,
    };
  });

const fallbackPage = (
  id: string,
  name: string,
  accent: string,
  bg: string,
  specs: Array<[string, string, string, string, string, string, string]>,
  zone: [string, string, string, string],
) => ({
  id,
  name,
  keys: paintKeys(
    specs.map(
      (spec) =>
        [spec[0], spec[1], spec[2], spec[3], spec[4], spec[5], spec[6], accent, bg] as [
          string,
          string,
          string,
          string,
          string,
          string,
          string,
          string,
          string,
        ],
    ),
  ),
  touchZones: [
    { index: 0, label: zone[0], valueText: zone[1], tapActionId: zone[2], accentColor: zone[3] },
    { index: 1, label: "PREV PAGE", valueText: "Previous page", tapActionId: "profile.prev_page", accentColor: "#56B9F2" },
    { index: 2, label: "PAGE", valueText: "Swipe to change", tapActionId: "profile.next_page", accentColor: "#8B5CF6" },
    { index: 3, label: "NEXT PAGE", valueText: "Next page", tapActionId: "profile.next_page", accentColor: "#F4B860" },
  ],
  encoders: [
    { index: 0, label: "Master Vol", valueText: "Win32 Vol ±", cwAction: "audio.volume_up", ccwAction: "audio.volume_down", pushAction: "audio.volume_mute" },
    { index: 1, label: "Media Track", valueText: "Prev / Next", cwAction: "media.next", ccwAction: "media.prev", pushAction: "media.play_pause" },
    { index: 2, label: "Timeline", valueText: "Left / Right", cwAction: "Right", ccwAction: "Left", pushAction: "Space" },
    { index: 3, label: "Brightness", valueText: "85% • CRT..LIG", cwAction: "brightness.up", ccwAction: "brightness.down", pushAction: "brightness.toggle" },
  ],
});

const fallbackBootstrap = (): FullBootstrapPayload => {
  const now = Date.now();
  const basePage = {
    id: "page-1",
    name: "Page 1",
    keys: buildDefaultKeys(),
    touchZones: [
      { index: 0, label: "SPOTIFY", valueText: "▶ Play / Pause", tapActionId: "media.play_pause", accentColor: "#35C98B" },
      { index: 1, label: "NEXT PAGE", valueText: "Next page", tapActionId: "profile.next_page", accentColor: "#56B9F2" },
      { index: 2, label: "MIXER", valueText: "Open Audio", tapActionId: "audio.open_mixer", accentColor: "#8B5CF6" },
      { index: 3, label: "MIC MUTE", valueText: "Toggle Mute", tapActionId: "audio.mute_mic", accentColor: "#F4B860" },
    ],
    encoders: [
      { index: 0, label: "Master Vol", valueText: "Win32 Vol ±", cwAction: "audio.volume_up", ccwAction: "audio.volume_down", pushAction: "audio.volume_mute" },
      { index: 1, label: "Media Track", valueText: "Prev / Next", cwAction: "media.next", ccwAction: "media.prev", pushAction: "media.play_pause" },
      { index: 2, label: "Timeline", valueText: "Left / Right", cwAction: "Right", ccwAction: "Left", pushAction: "Space" },
      { index: 3, label: "Brightness", valueText: "85% • CRT..LIG", cwAction: "brightness.up", ccwAction: "brightness.down", pushAction: "brightness.toggle" },
    ],
  };
  const musicPage = fallbackPage("page-music", "Music", "#35C98B", "#112822", [
    ["Play", "SPOTIFY", "Music", "media.play_pause", "Media: Play / Pause", "Media", ""],
    ["Next", "TRACK", "Music", "media.next", "Media: Next Track", "Media", ""],
    ["Previous", "TRACK", "Disc", "media.prev", "Media: Previous Track", "Media", ""],
    ["Stop", "MEDIA", "Disc", "media.stop", "Media: Stop", "Media", ""],
    ["Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""],
    ["Vol Down", "-2%", "Volume2", "audio.volume_down", "Audio: Volume Down", "Media", ""],
    ["Mute", "SOUND", "Volume2", "audio.volume_mute", "Audio: Mute Speakers", "Media", ""],
    ["Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""],
    ["Mixer", "AUDIO", "Volume2", "audio.open_mixer", "Audio: Open Mixer", "Media", ""],
    ["Stream", "PAGE", "Radio", "profile.switch_page", "Navigation: Stream Page", "Navigation", "Page 1"],
  ], ["SPOTIFY", "Play / Pause", "media.play_pause", "#35C98B"]);
  const desktopPage = fallbackPage("page-desktop", "Desktop", "#56B9F2", "#132235", [
    ["Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Screenshot", "System", "Win+Shift+S"],
    ["Desktop", "SHOW", "Monitor", "sys.show_desktop", "System: Show Desktop", "System", "Win+D"],
    ["Task Mgr", "TASK", "Terminal", "sys.hotkey", "System: Task Manager", "System", "Ctrl+Shift+Esc"],
    ["Files", "OPEN", "Folder", "sys.hotkey", "System: File Explorer", "System", "Win+E"],
    ["Lock", "PC", "Zap", "sys.lock_pc", "System: Lock PC", "System", ""],
    ["Settings", "WIN", "Settings", "sys.hotkey", "System: Windows Settings", "System", "Win+I"],
    ["Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""],
    ["Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""],
    ["Brighter", "DECK", "Zap", "brightness.up", "Deck: Brightness Up", "System", ""],
    ["Music", "PAGE", "Music", "profile.switch_page", "Navigation: Music Page", "Navigation", "Music"],
  ], ["SNAP", "Screenshot", "sys.screenshot", "#56B9F2"]);
  const gamePage = fallbackPage("page-game", "Game", "#35C98B", "#10241C", [
    ["Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""],
    ["Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""],
    ["Vol Down", "-2%", "Volume2", "audio.volume_down", "Audio: Volume Down", "Media", ""],
    ["Mute", "SOUND", "Volume2", "audio.volume_mute", "Audio: Mute Speakers", "Media", ""],
    ["Play", "MUSIC", "Music", "media.play_pause", "Media: Play / Pause", "Media", ""],
    ["Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Screenshot", "System", "Win+Shift+S"],
    ["Desktop", "SHOW", "Monitor", "sys.show_desktop", "System: Show Desktop", "System", "Win+D"],
    ["Task Mgr", "TASK", "Terminal", "sys.hotkey", "System: Task Manager", "System", "Ctrl+Shift+Esc"],
    ["Lock", "PC", "Zap", "sys.lock_pc", "System: Lock PC", "System", ""],
    ["Music", "PAGE", "Music", "profile.switch_page", "Navigation: Music Page", "Navigation", "Music"],
  ], ["MIC", "Mute mic", "audio.mute_mic", "#35C98B"]);
  const editPage = fallbackPage("page-edit", "Edit", "#56B9F2", "#10202C", [
    ["Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Screenshot", "System", "Win+Shift+S"],
    ["Play", "REVIEW", "Music", "media.play_pause", "Media: Play / Pause", "Media", ""],
    ["Previous", "CLIP", "Disc", "media.prev", "Media: Previous Track", "Media", ""],
    ["Next", "CLIP", "Music", "media.next", "Media: Next Track", "Media", ""],
    ["Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""],
    ["Vol Down", "-2%", "Volume2", "audio.volume_down", "Audio: Volume Down", "Media", ""],
    ["Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""],
    ["Files", "OPEN", "Folder", "sys.hotkey", "System: File Explorer", "System", "Win+E"],
    ["Desktop", "SHOW", "Monitor", "sys.show_desktop", "System: Show Desktop", "System", "Win+D"],
    ["Music", "PAGE", "Music", "profile.switch_page", "Navigation: Music Page", "Navigation", "Music"],
  ], ["SNAP", "Screenshot", "sys.screenshot", "#56B9F2"]);
  const workPage = fallbackPage("page-work", "Work", "#F4B860", "#2A2215", [
    ["Files", "OPEN", "Folder", "sys.hotkey", "System: File Explorer", "System", "Win+E"],
    ["Settings", "WIN", "Settings", "sys.hotkey", "System: Windows Settings", "System", "Win+I"],
    ["Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Screenshot", "System", "Win+Shift+S"],
    ["Desktop", "SHOW", "Monitor", "sys.show_desktop", "System: Show Desktop", "System", "Win+D"],
    ["Lock", "PC", "Zap", "sys.lock_pc", "System: Lock PC", "System", ""],
    ["Task Mgr", "TASK", "Terminal", "sys.hotkey", "System: Task Manager", "System", "Ctrl+Shift+Esc"],
    ["Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""],
    ["Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""],
    ["Play", "MUSIC", "Music", "media.play_pause", "Media: Play / Pause", "Media", ""],
    ["Music", "PAGE", "Music", "profile.switch_page", "Navigation: Music Page", "Navigation", "Music"],
  ], ["FILES", "Explorer", "sys.hotkey", "#F4B860"]);
  const homePage = fallbackPage("page-home", "Home", "#A6AFC4", "#1A1E27", [
    ["Desktop", "SHOW", "Monitor", "sys.show_desktop", "System: Show Desktop", "System", "Win+D"],
    ["Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Screenshot", "System", "Win+Shift+S"],
    ["Files", "OPEN", "Folder", "sys.hotkey", "System: File Explorer", "System", "Win+E"],
    ["Settings", "WIN", "Settings", "sys.hotkey", "System: Windows Settings", "System", "Win+I"],
    ["Lock", "PC", "Zap", "sys.lock_pc", "System: Lock PC", "System", ""],
    ["Task Mgr", "TASK", "Terminal", "sys.hotkey", "System: Task Manager", "System", "Ctrl+Shift+Esc"],
    ["Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""],
    ["Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""],
    ["Play", "MUSIC", "Music", "media.play_pause", "Media: Play / Pause", "Media", ""],
    ["Music", "PAGE", "Music", "profile.switch_page", "Navigation: Music Page", "Navigation", "Music"],
  ], ["DESK", "Show desktop", "sys.show_desktop", "#A6AFC4"]);

  return {
    devices: [
      {
        id: "ajazz-akp05e-pro",
        vendorName: "AJAZZ",
        modelName: "Ajazz AKP05E Pro",
        vid: 0x0300,
        pid: 0x3013,
        vidPidHex: "0300:3013",
        driverName: "CrtHidDriver v1.0 (Verified Disassembly)",
        protocolFamily: "CRT-1024B-HID",
        verifiedDriver: true,
        connected: true,
        isSimulator: false,
        serialNumber: "4251E8782A0B",
        firmwareVersion: "V4.AKP05E Pro.02.010",
        hidPath: "\\\\?\\hid#vid_0300&pid_3013&mi_00",
        assignedProfileId: "profile-streaming",
        brightness: 85,
        keys: { rows: 2, cols: 5, count: 10, pixelWidth: 112, pixelHeight: 112, rotation: 180 },
        touchStrip: { zones: 4, pixelWidth: 800, pixelHeight: 100, supportsSwipe: true },
        encoders: { count: 4, supportsPush: true, detentsPerRevolution: 24 },
        supportsBrightness: true,
        lastSeenMs: now,
      },
      {
        id: "ajazz-akp153",
        vendorName: "AJAZZ",
        modelName: "Ajazz AKP153 (15 Keys)",
        vid: 0x0300,
        pid: 0x1010,
        vidPidHex: "0300:1010",
        driverName: "CrtHidDriver v1.0 (CRT Family)",
        protocolFamily: "CRT-64B-HID",
        verifiedDriver: false,
        connected: false,
        isSimulator: true,
        serialNumber: "SIM-AKP153",
        firmwareVersion: "Simulator Profile",
        hidPath: null,
        assignedProfileId: "profile-streaming",
        brightness: 80,
        keys: { rows: 3, cols: 5, count: 15, pixelWidth: 85, pixelHeight: 85, rotation: 90 },
        touchStrip: null,
        encoders: null,
        supportsBrightness: true,
        lastSeenMs: now,
      },
      {
        id: "elgato-mk2",
        vendorName: "Elgato",
        modelName: "Elgato Stream Deck MK.2",
        vid: 0x0FD9,
        pid: 0x0080,
        vidPidHex: "0FD9:0080",
        driverName: "ElgatoHidDriver v1.0 (Report 0x02)",
        protocolFamily: "ELGATO-HID-V2",
        verifiedDriver: false,
        connected: false,
        isSimulator: true,
        serialNumber: "SIM-ELGATO-MK2",
        firmwareVersion: "Simulator Profile",
        hidPath: null,
        assignedProfileId: "profile-streaming",
        brightness: 80,
        keys: { rows: 3, cols: 5, count: 15, pixelWidth: 72, pixelHeight: 72, rotation: 0 },
        touchStrip: null,
        encoders: null,
        supportsBrightness: true,
        lastSeenMs: now,
      },
      {
        id: "mirabox-n3",
        vendorName: "MiraBox / AJAZZ",
        modelName: "MiraBox N3 / Ajazz AKP03",
        vid: 0x0300,
        pid: 0x1003,
        vidPidHex: "0300:1003",
        driverName: "CrtHidDriver v1.0 (CRT Family)",
        protocolFamily: "CRT-64B-HID",
        verifiedDriver: false,
        connected: false,
        isSimulator: true,
        serialNumber: "SIM-N3-KNOB",
        firmwareVersion: "Simulator Profile",
        hidPath: null,
        assignedProfileId: "profile-editing",
        brightness: 85,
        keys: { rows: 2, cols: 3, count: 6, pixelWidth: 72, pixelHeight: 72, rotation: 0 },
        touchStrip: null,
        encoders: { count: 3, supportsPush: true, detentsPerRevolution: 24 },
        supportsBrightness: true,
        lastSeenMs: now,
      },
      {
        id: "elgato-xl",
        vendorName: "Elgato",
        modelName: "Elgato Stream Deck XL (32 Keys)",
        vid: 0x0FD9,
        pid: 0x006C,
        vidPidHex: "0FD9:006C",
        driverName: "ElgatoHidDriver v1.0 (Report 0x02)",
        protocolFamily: "ELGATO-HID-V2",
        verifiedDriver: false,
        connected: false,
        isSimulator: true,
        serialNumber: "SIM-ELGATO-XL",
        firmwareVersion: "Simulator Profile",
        hidPath: null,
        assignedProfileId: "profile-work",
        brightness: 80,
        keys: { rows: 4, cols: 8, count: 32, pixelWidth: 96, pixelHeight: 96, rotation: 0 },
        touchStrip: null,
        encoders: null,
        supportsBrightness: true,
        lastSeenMs: now,
      },
    ],
    profiles: [
      { id: "profile-streaming", name: "Streaming", icon: "Radio", colorStrip: "#8B5CF6", targetDeviceId: "ajazz-akp05e-pro", pages: [basePage, desktopPage, musicPage], lastModifiedMs: now },
      { id: "profile-gaming", name: "Gaming", icon: "Gamepad2", colorStrip: "#35C98B", targetDeviceId: "ajazz-akp05e-pro", pages: [gamePage, musicPage, desktopPage], lastModifiedMs: now },
      { id: "profile-editing", name: "Editing", icon: "Film", colorStrip: "#56B9F2", targetDeviceId: "ajazz-akp05e-pro", pages: [editPage, musicPage, desktopPage], lastModifiedMs: now },
      { id: "profile-work", name: "Work", icon: "Briefcase", colorStrip: "#F4B860", targetDeviceId: "ajazz-akp05e-pro", pages: [workPage, musicPage, desktopPage], lastModifiedMs: now },
      { id: "profile-default", name: "Default", icon: "Layers", colorStrip: "#A6AFC4", targetDeviceId: "ajazz-akp05e-pro", pages: [homePage, musicPage, desktopPage], lastModifiedMs: now },
    ],
    autoSwitchRules: [
      { id: "rule-obs", processName: "obs64.exe", windowTitleContains: "OBS", targetProfileId: "profile-streaming", priority: 1, enabled: true },
      { id: "rule-resolve", processName: "Resolve.exe", windowTitleContains: "DaVinci Resolve", targetProfileId: "profile-editing", priority: 2, enabled: true },
      { id: "rule-code", processName: "Code.exe", windowTitleContains: "Visual Studio Code", targetProfileId: "profile-work", priority: 3, enabled: true },
      { id: "rule-spotify", processName: "Spotify.exe", windowTitleContains: "", targetProfileId: "profile-streaming", priority: 4, enabled: false },
    ],
    macros: [
      {
        id: "macro-go-live",
        name: "Go Live Broadcast Prep",
        description: "Checks OBS, switches to Starting Soon scene, waits 500ms, and switches to Streaming profile.",
        steps: [
          { id: "step-1", stepType: "IfWindowExists", label: "Check if OBS Studio (obs64.exe) is running", parameter: "obs64.exe", timeoutMs: 2000, onError: "Continue" },
          { id: "step-2", stepType: "Shortcut", label: "Send OBS Starting Soon Scene Hotkey", parameter: "Ctrl+Alt+F1", timeoutMs: 500, onError: "Stop" },
          { id: "step-3", stepType: "Delay", label: "Wait 500ms for scene transition", parameter: "500", timeoutMs: 1000, onError: "Continue" },
          { id: "step-4", stepType: "SwitchProfile", label: "Switch DeckForge Profile → Streaming", parameter: "profile-streaming", timeoutMs: 500, onError: "Stop" },
        ],
        lastEditedMs: now,
      },
      {
        id: "macro-dev-workspace",
        name: "Developer Focus Setup",
        description: "Launches Windows Terminal, waits 300ms, and snaps window left.",
        steps: [
          { id: "step-201", stepType: "LaunchApp", label: "Launch Windows Terminal", parameter: "wt.exe", timeoutMs: 3000, onError: "Continue" },
          { id: "step-202", stepType: "Delay", label: "Wait 300ms", parameter: "300", timeoutMs: 500, onError: "Continue" },
          { id: "step-203", stepType: "Shortcut", label: "Snap Window Left (Win+Left)", parameter: "Win+Left", timeoutMs: 500, onError: "Continue" },
        ],
        lastEditedMs: now,
      },
    ],
    diagnostics: [
      {
        id: "diag-boot-1",
        timestampMs: now - 2500,
        level: "info",
        deviceId: "system",
        stage: "BOOT",
        message: "DeckForge Rust Hardware & Win32 Input Service initialized",
        packetHex: null,
      },
      {
        id: "diag-usb-akp",
        timestampMs: now - 1200,
        level: "success",
        deviceId: "ajazz-akp05e-pro",
        stage: "USB_DETECTED",
        message: "Verified hardware connected: Ajazz AKP05E Pro (0300:3013) • Serial 4251E8782A0B",
        packetHex: "00 43 52 54 00 00 48 41 4E 00 00 00",
      },
    ],
    frameCacheStats: {
      framesEvaluated: 40,
      framesTransmitted: 10,
      framesDeduplicatedSkipped: 30,
      bytesTransmitted: 18420,
      bytesSaved: 55260,
      lastTransferLatencyMs: 11,
    },
    legacySummary: {
      profilesDir: "%APPDATA%\\HotSpot\\StreamDock\\profiles",
      pluginsDir: "%APPDATA%\\HotSpot\\StreamDock\\plugins",
      legacyProfileCount: 0,
      namedRootProfilesCount: 0,
      legacyPluginCount: 0,
      pluginWsPort: null,
      pluginHostActive: false,
      discoveredPlugins: [
        {
          id: "builtin.obs",
          name: "OBS Studio WebSocket",
          author: "DeckForge Core",
          version: "1.0.0",
          description: "Native scene switching, stream/record control, replay buffer, and audio source mute.",
          category: "Streaming",
          sourceKind: "BuiltIn",
          installed: true,
          enabled: true,
          permissions: ["Local Network (ws://127.0.0.1:4455)"],
          actionsProvided: ["Start/Stop Stream", "Switch Scene", "Save Replay"],
          diskPath: null,
        },
      ],
    },
    telemetry: {
      cpuUsagePercent: 14.2,
      memoryUsedGb: 11.4,
      memoryTotalGb: 32.0,
      memoryUsagePercent: 35.6,
      processCount: 218,
      officialStreamdockRunning: false,
      obsRunning: false,
      timestampMs: now,
    },
    savedLocallyAtMs: now,
    appliedToDeviceAtMs: now,
    recentInputEvents: [],
    mediaLibrary: [],
    face: {
      mediaPlaying: true,
      mediaTitle: "Night Drive",
      micMuted: false,
      speakerMuted: false,
      volumePercent: 72,
    },
    obs: { url: "ws://127.0.0.1:4455", password: "" },
  };
};

// Browser preview of Game Mode, so the screen can be checked without the desktop app. The data is
// made up and generic, like a PC nobody in particular owns.
type MockTarget = [string, string, GameModeTarget["group"], string, string[], boolean, boolean];
const MOCK_TARGETS: MockTarget[] = [
  // id, label, group, note, process names, reopens, force
  ["discord", "Discord", "apps", "You leave voice chat.", ["discord"], true, true],
  ["spotify", "Spotify", "apps", "Music stops.", ["spotify"], true, true],
  ["steam", "Steam client", "apps", "Keep it open if the game starts from Steam.", ["steam", "steamwebhelper"], true, true],
  ["epic", "Epic Games Launcher", "apps", "Keep it open if the game starts from Epic.", ["epicgameslauncher"], true, true],
  ["riotclient", "Riot Client launcher", "apps", "Closes the launcher only. Vanguard anti-cheat is never touched.", ["riotclientservices"], true, true],
  ["docker_wsl", "Docker Desktop + WSL", "apps", "Stops Docker and shuts the WSL VM down, which frees about 1.5 GB. Running containers stop.", ["docker desktop"], true, true],
];

const mockApp = (name: string, count: number, memoryMb: number, folder: string, hasWindow: boolean) => ({
  name,
  count,
  memoryMb,
  path: `c:\\users\\you\\appdata\\local\\${folder}\\${name}.exe`,
  hasWindow,
});

const MOCK_PROCESSES = [
  mockApp("chrome", 24, 3120, "google\\chrome\\application", true),
  mockApp("discord", 6, 821, "discord", true),
  mockApp("spotify", 8, 1012, "spotify", true),
  mockApp("obsidian", 5, 612, "obsidian", true),
  mockApp("figma", 7, 540, "figma", true),
  mockApp("notion", 6, 488, "programs\\notion", true),
  mockApp("steam", 10, 705, "steam", true),
  mockApp("onedrive", 1, 210, "microsoft\\onedrive", false),
  mockApp("dropbox", 3, 164, "dropbox", false),
  mockApp("steamwebhelper", 9, 1500, "steam", false),
  mockApp("nvidia share", 2, 96, "nvidia", false),
  mockApp("qbittorrent", 1, 128, "qbittorrent", false),
];

const MOCK_SERVICES: GameModeServicePick[] = [
  { name: "Fax", display: "Fax" },
  { name: "SharedAccess", display: "Internet Connection Sharing (ICS)" },
  { name: "OneSyncSvc", display: "Sync Host" },
  { name: "PhoneSvc", display: "Phone Service" },
  { name: "WSearch", display: "Windows Search" },
  { name: "SysMain", display: "SysMain" },
  { name: "Spooler", display: "Print Spooler" },
  { name: "MongoDB", display: "MongoDB Server" },
];

const MOCK_POWER_PLANS: GameModePowerPlan[] = [
  { guid: "381b4222-f694-41f0-9685-ff5bb260df2e", name: "Balanced", active: true },
  { guid: "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c", name: "High performance", active: false },
  { guid: "a1841308-3541-4fab-bc81-f71556f20b4a", name: "Power saver", active: false },
];

let mockHelperArmed = false;

const mockScan = (): GameModeScan => {
  const customs = mockGameModeConfig.customTargets;
  const covering = (name: string) => {
    const preset = MOCK_TARGETS.find((t) => t[4].includes(name));
    if (preset) return { id: preset[0], label: preset[1] };
    const custom = customs.find((c) => c.processes.includes(name));
    return custom ? { id: custom.id, label: custom.label } : null;
  };
  const processes: GameModeRunningApp[] = MOCK_PROCESSES.map((p) => {
    const taken = covering(p.name);
    return { ...p, takenBy: taken?.label ?? null, takenById: taken?.id ?? null };
  }).sort((a, b) => b.memoryMb - a.memoryMb);
  const usage = (names: string[]) => {
    const found = MOCK_PROCESSES.filter((p) => names.includes(p.name));
    return { count: found.reduce((sum, p) => sum + p.count, 0), memoryMb: found.reduce((sum, p) => sum + p.memoryMb, 0) };
  };
  return {
    processes,
    services: MOCK_SERVICES,
    helper: { armed: mockHelperArmed, covers: mockHelperArmed },
    targets: [
      ...MOCK_TARGETS.map(([id, label, group, note, names, reopens, force]) => {
        const used = usage(names);
        return {
          id,
          label,
          group,
          note,
          running: used.count > 0,
          processCount: used.count,
          memoryMb: used.memoryMb,
          services: [],
          needsAdmin: false,
          reopens,
          force,
          custom: false,
          processes: names,
        };
      }),
      ...customs.map((c) => {
        const used = usage(c.processes);
        return {
          id: c.id,
          label: c.label,
          group: c.group,
          note: "Added by you.",
          running: used.count > 0 || c.services.some((name) => MOCK_SERVICES.some((s) => s.name === name)),
          processCount: used.count,
          memoryMb: used.memoryMb,
          services: c.services.map((name) => ({ name, running: MOCK_SERVICES.some((s) => s.name === name) })),
          needsAdmin: c.services.length > 0,
          reopens: c.reopen,
          force: true,
          custom: true,
          processes: c.processes,
        };
      }),
    ],
    tweaks: [
      { id: "priority", label: "Raise the game's priority to High", note: "Applies to the full-screen game as soon as DeckForge sees it." },
      { id: "pause_updates", label: "Pause Windows Update and Delivery Optimization", note: "Stops the update services while you play. Needs the admin helper." },
    ],
    ramTotalMb: 32154,
    ramFreeMb: 8600,
    processCount: 382,
    powerPlan: MOCK_POWER_PLANS.find((p) => p.active)?.name ?? "",
    powerPlans: MOCK_POWER_PLANS,
    isAdmin: false,
    protected: [
      "Anti-cheat such as Riot Vanguard, EasyAntiCheat, BattlEye and FACEIT",
      "Graphics driver and overlay services",
      "Windows audio and audio routing software",
      "Windows Defender and security services",
      "Explorer, DWM and core Windows processes",
      "Process managers such as Process Lasso",
      "DeckForge itself",
    ],
  };
};

const mockEmptyStatus = (): GameModeStatus => ({
  active: false,
  busy: null,
  mode: "",
  startedAtMs: 0,
  autoStarted: false,
  game: null,
  gamePid: 0,
  closed: [],
  stoppedServices: [],
  failedServices: [],
  helperUsed: false,
  ramFreeBeforeMb: 0,
  ramFreeAfterMb: 0,
  processesBefore: 0,
  processesAfter: 0,
  powerPlan: "",
  notes: [],
  lastEndedMs: 0,
});

let mockGameModeConfig: GameModeConfig = {
  modes: [{ id: "default", name: "Default", targets: [], tweaks: ["priority"], powerPlan: "", startApps: [] }],
  activeMode: "default",
  customTargets: [],
  autoDetect: true,
  settleSeconds: 20,
  restore: "all",
  elevate: "ask",
  armOnLaunch: false,
};
let mockGameModeStatus: GameModeStatus = mockEmptyStatus();
const mockGameModeListeners = new Set<(status: GameModeStatus) => void>();
const emitMockGameMode = () => mockGameModeListeners.forEach((fn) => fn({ ...mockGameModeStatus }));

let mockDeckPaused = false;
let mockSavedProfiles: Profile[] | null = null;
let mockApplied: { deviceId: string; profileId: string; profileName: string; page: ProfilePage } | null = null;
let mockAppliedAt: number | null = null;

// A knob can mount three app pickers at once. Coalesce their OS scans while
// retaining the existing two-second polling cadence and manual refresh.
const readAudioApps = createSharedRead(
  () => invoke<import("../types/deckforge").AudioAppState[]>("audio_apps"), 500
);

export const deckforgeApi = {
  async getProfileDraft(): Promise<RecoveryDraft | null> {
    if (isTauriRuntime()) return invoke<RecoveryDraft | null>("get_profile_draft");
    const raw = localStorage.getItem("deckforge-preview-draft");
    if (!raw) return null;
    const draft = JSON.parse(raw);
    if (!draft) return null;
    if (draft.version !== 1 || !Array.isArray(draft.profiles) || !draft.profiles.length
      || draft.profiles.some((p: Profile) => !p.pages?.length || p.pages.some(page => !page.keys?.length))) {
      throw new Error("Invalid or unsupported preview recovery draft; it has been preserved.");
    }
    const content = (value: Profile[]) => JSON.stringify(value, (key, value) => key === "lastModifiedMs" ? undefined : value);
    const saved = content(mockSavedProfiles ?? fallbackBootstrap().profiles);
    if (content(draft.profiles) === saved) return null;
    return { draft, savedProfilesChanged: content(draft.baseProfiles) !== saved };
  },
  async persistProfileDraft(value: DraftSnapshot | null): Promise<void> {
    if (isTauriRuntime()) {
      if (value) await invoke("save_profile_draft", { ...value });
      else await invoke("discard_profile_draft");
    } else {
      localStorage.setItem("deckforge-preview-draft", JSON.stringify(value ? { version: 1, ...value, updatedAtMs: Date.now() } : null));
    }
  },

  async exportProfilesBackup(profiles: Profile[], macros: MacroSequence[]): Promise<string> {
    if (!isTauriRuntime()) throw new Error("Media backups require the desktop app.");
    return invoke<string>("export_profiles_backup", { profiles, macros });
  },
  async restoreProfilesBackup(contents: string): Promise<RestoredBackup> {
    if (!isTauriRuntime()) throw new Error("Media backups require the desktop app.");
    return invoke<RestoredBackup>("restore_profiles_backup", { contents });
  },

  async exportProfilePack(profile: Profile, macros: MacroSequence[], appRules: AutoSwitchRule[]): Promise<string> {
    if (!isTauriRuntime()) throw new Error("Portable profile packs require the desktop app.");
    return invoke<string>("export_profile_pack", { profile, macros, appRules });
  },
  async restoreProfilePack(contents: string): Promise<RestoredBackup> {
    if (!isTauriRuntime()) throw new Error("Portable profile packs require the desktop app.");
    return invoke<RestoredBackup>("restore_profile_pack", { contents });
  },

  async saveStartupProfile(profileId: string | null): Promise<void> {
    if (isTauriRuntime()) return invoke<void>("save_startup_profile", { profileId });
    if (profileId) localStorage.setItem("deckforge-preview-startup-profile", profileId);
    else localStorage.removeItem("deckforge-preview-startup-profile");
  },

  async getInitialState(): Promise<FullBootstrapPayload> {
    if (isTauriRuntime()) {
      return await invoke<FullBootstrapPayload>("get_initial_state");
    }
    return { ...fallbackBootstrap(), startupProfileId: localStorage.getItem("deckforge-preview-startup-profile") };
  },

  async refreshUsbDevices(): Promise<DeviceDescriptor[]> {
    if (isTauriRuntime()) {
      return await invoke<DeviceDescriptor[]>("refresh_usb_devices");
    }
    return fallbackBootstrap().devices;
  },

  async setDeviceBrightness(deviceId: string, percent: number): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("set_device_brightness", { deviceId, percent });
    }
    return `Brightness set to ${percent}% (CRT\\0\\0LIG 0x${percent.toString(16).toUpperCase().padStart(2, "0")})`;
  },

  async setKnobLeds(deviceId: string, colors: string[]): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("set_knob_leds", { deviceId, colors });
    }
    return `Knob LEDs updated (CRT\\0\\0SETLB ${colors.join(" ")})`;
  },

  async setKnobLedEffect(deviceId: string, effect: string, colors: string[]): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("set_knob_led_effect", { deviceId, effect, colors });
    }
    return `Knob LED effect set to ${effect || "static"}`;
  },

  async resetKnobLeds(deviceId: string): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("reset_knob_leds", { deviceId });
    }
    return "Knob LEDs reset to factory default (CRT\\0\\0DELED)";
  },

  async setLedBrightness(deviceId: string, percent: number): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("set_led_brightness", { deviceId, percent });
    }
    return `LED brightness set to ${percent}% (CRT\\0\\0LBLIG)`;
  },

  async setLedSleepMode(deviceId: string, followSystemSleep: boolean): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("set_led_sleep_mode", { deviceId, followSystemSleep });
    }
    return `LED sleep mode set to ${followSystemSleep ? "Follow PC sleep" : "Always on"} (CRT\\0\\0QUCMD)`;
  },

  async applyProfileToDevice(deviceId: string, profileId: string, pageId: string): Promise<SyncKeyResult> {
    if (isTauriRuntime()) {
      return await invoke<SyncKeyResult>("apply_profile_to_device", { deviceId, profileId, pageId });
    }
    const profile = (mockSavedProfiles ?? fallbackBootstrap().profiles).find(p => p.id === profileId);
    const page = profile?.pages.find(p => p.id === pageId);
    if (!profile || !page) throw new Error("Preview page not found");
    if (mockDeckPaused) throw new Error("Preview device control is paused");
    mockApplied = structuredClone({ deviceId, profileId, profileName: profile.name, page });
    mockAppliedAt = Date.now();
    return {
      deviceId,
      connectedPhysical: deviceId === "ajazz-akp05e-pro",
      keysEvaluated: 10,
      keysTransmitted: 2,
      keysDeduplicatedSkipped: 8,
      bytesTransmitted: 3680,
      elapsedMs: 9,
      statusMessage: "Synced 2/10 changed keys in 9ms (8 unchanged keys skipped via SHA-256 frame cache)",
    };
  },

  async onQuitRequested(callback: () => void): Promise<UnlistenFn> {
    if (isTauriRuntime()) return listen("quit-requested", callback);
    return () => {};
  },
  async quitApplication(): Promise<void> {
    if (isTauriRuntime()) await invoke("quit_application");
  },
  async getLaunchAtLogin(): Promise<boolean> {
    return isTauriRuntime() ? invoke<boolean>("get_launch_at_login") : false;
  },
  async setLaunchAtLogin(enabled: boolean): Promise<boolean> {
    if (isTauriRuntime()) return invoke<boolean>("set_launch_at_login", { enabled });
    throw new Error("Launch at sign-in is available in the desktop app");
  },

  async saveProfiles(profiles: Profile[]): Promise<number> {
    if (isTauriRuntime()) {
      return await invoke<number>("save_profiles", { profiles });
    }
    mockSavedProfiles = structuredClone(profiles);
    return Date.now();
  },

  async testKeyAction(actionId: string, actionLabel: string, actionParam: string): Promise<ActionExecutionResult> {
    if (isTauriRuntime()) {
      return await invoke<ActionExecutionResult>("test_key_action", { actionId, actionLabel, actionParam });
    }
    return {
      actionId,
      actionLabel,
      success: true,
      elapsedMs: 3,
      outputMessage: `Dispatched Win32 ${actionLabel} (${actionParam}) in 3ms`,
      timestampMs: Date.now(),
    };
  },

  async saveMacros(macros: MacroSequence[]): Promise<void> {
    if (isTauriRuntime()) {
      await invoke("save_macros", { macros });
    }
  },

  async stopMacroExecution(): Promise<string> {
    if (isTauriRuntime()) return await invoke<string>("stop_macro_execution");
    return "No macro is running.";
  },

  async executeMacroSequence(macroId: string): Promise<MacroRunReport> {
    if (isTauriRuntime()) {
      return await invoke<MacroRunReport>("execute_macro_sequence", { macroId });
    }
    const m = fallbackBootstrap().macros.find((x) => x.id === macroId) || fallbackBootstrap().macros[0];
    return {
      macroId: m.id,
      macroName: m.name,
      totalElapsedMs: 14,
      stepResults: m.steps.map((s) => ({
        stepId: s.id,
        label: s.label,
        stepType: s.stepType,
        status: "Passed",
        elapsedMs: 3,
        detail: `Verified ${s.stepType} (${s.parameter})`,
      })),
    };
  },

  async takeOverFromOfficialApp(): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("take_over_from_official_app");
    }
    mockDeckPaused = false;
    return "DeckForge will reconnect and restore your last applied layout.";
  },

  /** Restarts the deck like a replug. Windows asks the user for permission first. */
  async restartDeck(): Promise<string> {
    if (isTauriRuntime()) return await invoke<string>("restart_deck");
    throw new Error("Restarting the deck is available in the desktop app");
  },

  async pauseDeckControl(): Promise<string> {
    if (isTauriRuntime()) return await invoke<string>("pause_deck_control");
    mockDeckPaused = true;
    return "DeckForge paused. You can now open AJAZZ.";
  },

  async importLegacySdProfiles(): Promise<Profile[]> {
    if (isTauriRuntime()) {
      return await invoke<Profile[]>("import_legacy_sdprofiles");
    }
    return fallbackBootstrap().profiles;
  },

  async getWorkspaceStatus(deviceId: string, profileId: string, page: ProfilePage): Promise<WorkspaceStatus> {
    if (isTauriRuntime()) return invoke<WorkspaceStatus>("get_workspace_status", { deviceId, profileId, page });
    return {
      deck: await deckforgeApi.getDeckStatus(),
      applied: mockApplied ? {
        profileName: mockApplied.profileName, pageName: mockApplied.page.name,
        matchesEditor: mockApplied.deviceId === deviceId && mockApplied.profileId === profileId
          && JSON.stringify(mockApplied.page) === JSON.stringify(page),
        layoutReady: true,
      } : null,
    };
  },

  async getDeckStatus(): Promise<DeckStatus> {
    if (isTauriRuntime()) {
      return await invoke<DeckStatus>("get_deck_status");
    }
    return {
      state: mockDeckPaused ? "paused" : "connected",
      openedAtMs: Date.now() - 60_000,
      lastPushAtMs: mockAppliedAt,
      lastPushOk: mockAppliedAt ? true : null,
      lastError: null,
      reconnects: 0,
    };
  },

  /** Programs with an open window, as "name.exe"; empty in the browser preview. */
  async openApps(): Promise<string[]> {
    if (!isTauriRuntime()) return [];
    try {
      return await invoke<string[]>("open_apps");
    } catch {
      return [];
    }
  },

  /** Icon of a running app as a PNG data URL; null when it is not running or in the browser preview. */
  async appIcon(processName: string): Promise<string | null> {
    if (isTauriRuntime()) {
      return (await invoke<string | null>("app_icon", { processName })) ?? null;
    }
    return null;
  },

  async resendLayout(): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("resend_layout");
    }
    return "Sending the whole layout to the deck again.";
  },

  /**
   * Saves a share file (profile or Game Mode). In the app it goes to Documents\DeckForge Exports
   * and Explorer opens on it. In a plain browser it falls back to a normal download.
   * Returns the place it was saved.
   */
  async saveExportFile(fileName: string, contents: string): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("save_export_file", { fileName, contents });
    }
    const url = URL.createObjectURL(new Blob([contents], { type: "application/json" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = fileName;
    link.click();
    URL.revokeObjectURL(url);
    return `Downloads\\${fileName}`;
  },

  async getPerformanceSnapshot(reset = false): Promise<import("../types/deckforge").PerformanceSnapshot> {
    if (!isTauriRuntime()) throw new Error("Performance measurements require the desktop app.");
    return invoke("get_performance_snapshot", { reset });
  },

  async audioApps(force = false): Promise<import("../types/deckforge").AudioAppState[]> {
    if (!isTauriRuntime()) throw new Error("Audio app discovery requires the Windows desktop app.");
    return readAudioApps(force);
  },

  async getTelemetry(): Promise<SystemTelemetrySnapshot> {
    if (isTauriRuntime()) {
      return await invoke<SystemTelemetrySnapshot>("get_telemetry");
    }
    return fallbackBootstrap().telemetry;
  },

  async holdAutoSwitch(): Promise<void> {
    if (isTauriRuntime()) {
      await invoke<void>("hold_auto_switch");
    }
  },

  async saveAutoSwitchRules(rules: AutoSwitchRule[]): Promise<void> {
    if (isTauriRuntime()) {
      await invoke<void>("save_auto_switch_rules", { rules });
    }
  },

  async saveObsSettings(settings: ObsSettings): Promise<void> {
    if (isTauriRuntime()) {
      await invoke<void>("save_obs_settings", { settings });
    }
  },

  async onDeckFace(handler: (face: DeckFaceState) => void): Promise<UnlistenFn> {
    if (isTauriRuntime()) {
      return await listen<DeckFaceState>("deck-face", (event) => handler(event.payload));
    }
    return () => {};
  },

  async onDeckContext(handler: (ctx: DeckContextEvent) => void): Promise<UnlistenFn> {
    if (isTauriRuntime()) {
      return await listen<DeckContextEvent>("deck-context", (event) => handler(event.payload));
    }
    return () => {};
  },

  async importMediaUrl(url: string): Promise<MediaPreview> {
    if (!isTauriRuntime()) throw new Error("Link imports are available in the desktop app. You can still upload local files here.");
    return invoke<MediaPreview>("import_media_url", { id: `media-${crypto.randomUUID()}`, url });
  },
  async searchCommonsGifs(query: string): Promise<CommonsGif[]> {
    if (!isTauriRuntime()) throw new Error("GIF search requires the desktop app.");
    return invoke<CommonsGif[]>("search_commons_gifs", { query });
  },
  async openCommonsFilePage(url: string): Promise<void> {
    if (!isTauriRuntime()) { window.open(url, "_blank", "noopener,noreferrer"); return; }
    return invoke<void>("open_commons_file_page", { url });
  },

  async stageScreenMedia(id: string, fileName: string, dataBase64: string): Promise<MediaPreview> {
    if (isTauriRuntime()) {
      return await invoke<MediaPreview>("stage_screen_media", { id, fileName, dataBase64 });
    }
    const gif = /\.gif$/i.test(fileName) || dataBase64.startsWith("R0lGOD");
    return {
      id,
      fileName,
      kind: gif ? "gif" : "image",
      width: 0,
      height: 0,
      frameCount: 1,
      sourceFrameCount: 1,
      frameDelayMs: 100,
      delaysMs: [100],
      frames: [`data:${gif ? "image/gif" : "image/png"};base64,${dataBase64.split(",").pop()}`],
    };
  },

  async stageFrameSequence(
    id: string,
    fileName: string,
    framesBase64: string[],
    delayMs: number
  ): Promise<MediaPreview> {
    if (isTauriRuntime()) {
      return await invoke<MediaPreview>("stage_frame_sequence", {
        id,
        fileName,
        framesBase64,
        delayMs,
      });
    }
    return {
      id,
      fileName,
      kind: framesBase64.length > 1 ? "gif" : "image",
      width: 112,
      height: 112,
      frameCount: framesBase64.length,
      sourceFrameCount: framesBase64.length,
      frameDelayMs: delayMs,
      delaysMs: framesBase64.map(() => delayMs),
      frames: framesBase64.map((f) => (f.startsWith("data:") ? f : `data:image/jpeg;base64,${f}`)),
    };
  },

  async onHardwareInput(cb: (evt: HardwareInputEvent) => void): Promise<UnlistenFn> {
    if (isTauriRuntime()) {
      return await listen<HardwareInputEvent>("hardware-input", (event) => {
        cb(event.payload);
      });
    }
    return () => {};
  },

  async gameModeScan(): Promise<GameModeScan> {
    if (isTauriRuntime()) {
      return await invoke<GameModeScan>("game_mode_scan");
    }
    return mockScan();
  },

  async gameModeGetConfig(): Promise<GameModeConfig> {
    if (isTauriRuntime()) {
      return await invoke<GameModeConfig>("game_mode_get_config");
    }
    return { ...mockGameModeConfig };
  },

  async gameModeSaveConfig(config: GameModeConfig): Promise<GameModeConfig> {
    if (isTauriRuntime()) {
      return await invoke<GameModeConfig>("game_mode_save_config", { config });
    }
    mockGameModeConfig = { ...config };
    return { ...mockGameModeConfig };
  },

  async gameModeStatus(): Promise<GameModeStatus> {
    if (isTauriRuntime()) {
      return await invoke<GameModeStatus>("game_mode_status");
    }
    return { ...mockGameModeStatus };
  },

  async pluginList(): Promise<PluginView[]> {
    if (isTauriRuntime()) return await invoke<PluginView[]>("plugin_list");
    return mockPlugins.map((p) => ({ ...p }));
  },

  async pluginRescan(): Promise<PluginView[]> {
    if (isTauriRuntime()) return await invoke<PluginView[]>("plugin_rescan");
    return mockPlugins.map((p) => ({ ...p }));
  },

  async pluginSetEnabled(uuid: string, enabled: boolean): Promise<PluginView[]> {
    if (isTauriRuntime()) return await invoke<PluginView[]>("plugin_set_enabled", { uuid, enabled });
    mockPlugins = mockPlugins.map((p) => (p.uuid === uuid ? { ...p, enabled, running: enabled } : p));
    return mockPlugins.map((p) => ({ ...p }));
  },

  /** Installs a `.streamDeckPlugin` file, given as base64. */
  async pluginInstallFile(data: string): Promise<PluginInstalled> {
    if (isTauriRuntime()) return await invoke<PluginInstalled>("plugin_install_file", { data });
    throw new Error("Installing plugins needs the desktop app.");
  },

  async pluginGithubSearch(query: string): Promise<GithubPlugin[]> {
    if (isTauriRuntime()) return await invoke<GithubPlugin[]>("plugin_github_search", { query });
    return [
      { repo: "example/streamdeck-timer", name: "streamdeck-timer", description: "A countdown timer key (browser preview sample).", stars: 120, url: "https://github.com/example/streamdeck-timer", ownerAvatar: "", updatedAt: "2026-10-01T00:00:00Z" },
    ];
  },

  async pluginGithubInstall(repo: string): Promise<PluginInstalled> {
    if (isTauriRuntime()) return await invoke<PluginInstalled>("plugin_github_install", { repo });
    throw new Error("Installing plugins needs the desktop app.");
  },

  async audioOutputs(): Promise<AudioOutput[]> {
    if (isTauriRuntime()) return await invoke<AudioOutput[]>("audio_outputs");
    return [
      { id: "a", name: "Speakers (Realtek Audio)", isDefault: true },
      { id: "b", name: "Headphones (USB Audio)", isDefault: false },
    ];
  },

  async pluginOpenPage(url: string): Promise<void> {
    if (isTauriRuntime()) return await invoke<void>("plugin_open_page", { url });
    window.open(url, "_blank", "noopener");
  },

  async gameModeArmHelper(): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("game_mode_arm_helper");
    }
    window.setTimeout(() => {
      mockHelperArmed = true;
      emitMockGameMode();
    }, 700);
    return "Starting the admin helper";
  },

  async gameModeSet(enabled: boolean, mode?: string): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("game_mode_set", { enabled, mode: mode ?? null });
    }
    mockGameModeStatus = { ...mockGameModeStatus, busy: enabled ? "Starting Game Mode" : "Ending Game Mode" };
    emitMockGameMode();
    window.setTimeout(() => {
      if (enabled) {
        const scan = mockScan();
        const chosen =
          mockGameModeConfig.modes.find((m) => m.id === mode) ??
          mockGameModeConfig.modes.find((m) => m.id === mockGameModeConfig.activeMode) ??
          mockGameModeConfig.modes[0];
        const picked = scan.targets.filter((t) => chosen.targets.includes(t.id));
        const closed = picked
          .filter((t) => t.processCount > 0)
          .map((t) => ({ id: t.id, label: t.label, memoryMb: t.memoryMb, processes: t.processCount, reopen: t.reopens ? "exe" : "no" }));
        const stopped = picked.flatMap((t) => t.services.map((s) => s.name));
        mockGameModeStatus = {
          ...mockEmptyStatus(),
          active: true,
          mode: chosen.name,
          powerPlan: scan.powerPlans.find((p) => p.guid === chosen.powerPlan)?.name ?? scan.powerPlan,
          startedAtMs: Date.now(),
          closed,
          stoppedServices: stopped,
          helperUsed: stopped.length > 0,
          ramFreeBeforeMb: 8600,
          ramFreeAfterMb: 8600 + closed.reduce((sum, c) => sum + c.memoryMb, 0),
          processesBefore: 382,
          processesAfter: 382 - closed.reduce((sum, c) => sum + c.processes, 0),
          notes: [
            `Closed ${closed.length} apps and stopped ${stopped.length} services (browser preview, nothing was touched).`,
          ],
        };
      } else {
        mockGameModeStatus = { ...mockEmptyStatus(), lastEndedMs: Date.now(), notes: ["Reopened the apps it closed (browser preview)."] };
      }
      emitMockGameMode();
    }, 900);
    return enabled ? "Starting Game Mode" : "Ending Game Mode";
  },

  async onGameMode(handler: (status: GameModeStatus) => void): Promise<UnlistenFn> {
    if (isTauriRuntime()) {
      return await listen<GameModeStatus>("game-mode", (event) => handler(event.payload));
    }
    mockGameModeListeners.add(handler);
    return () => {
      mockGameModeListeners.delete(handler);
    };
  },
};
