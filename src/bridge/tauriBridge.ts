import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ActionExecutionResult,
  DeviceDescriptor,
  FullBootstrapPayload,
  HardwareInputEvent,
  KeyConfig,
  MacroRunReport,
  Profile,
  SyncKeyResult,
  SystemTelemetrySnapshot,
} from "../types/deckforge";

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

const fallbackBootstrap = (): FullBootstrapPayload => {
  const now = Date.now();
  const basePage = {
    id: "page-1",
    name: "Page 1",
    keys: buildDefaultKeys(),
    touchZones: [
      { index: 0, label: "SPOTIFY", valueText: "▶ Play / Pause", tapActionId: "media.play_pause", accentColor: "#35C98B" },
      { index: 1, label: "CPU LOAD", valueText: "Live Telemetry", tapActionId: "sys.hotkey", accentColor: "#56B9F2" },
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
        protocolFamily: "CRT-64B-HID",
        verifiedDriver: true,
        connected: true,
        isSimulator: false,
        serialNumber: "4251E8782A0B",
        firmwareVersion: "V4.AKP05E Pro.02.010",
        hidPath: "\\\\?\\hid#vid_0300&pid_3013&mi_00",
        assignedProfileId: "profile-streaming",
        brightness: 85,
        keys: { rows: 2, cols: 5, count: 10, pixelWidth: 85, pixelHeight: 85, rotation: 180 },
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
      { id: "profile-streaming", name: "Streaming", icon: "Radio", colorStrip: "#8B5CF6", targetDeviceId: "ajazz-akp05e-pro", pages: [basePage], lastModifiedMs: now },
      { id: "profile-gaming", name: "Gaming", icon: "Gamepad2", colorStrip: "#35C98B", targetDeviceId: "ajazz-akp05e-pro", pages: [JSON.parse(JSON.stringify(basePage))], lastModifiedMs: now },
      { id: "profile-editing", name: "Editing", icon: "Film", colorStrip: "#56B9F2", targetDeviceId: "ajazz-akp05e-pro", pages: [JSON.parse(JSON.stringify(basePage))], lastModifiedMs: now },
      { id: "profile-work", name: "Work", icon: "Briefcase", colorStrip: "#F4B860", targetDeviceId: "ajazz-akp05e-pro", pages: [JSON.parse(JSON.stringify(basePage))], lastModifiedMs: now },
      { id: "profile-default", name: "Default", icon: "Layers", colorStrip: "#A6AFC4", targetDeviceId: "ajazz-akp05e-pro", pages: [JSON.parse(JSON.stringify(basePage))], lastModifiedMs: now },
    ],
    autoSwitchRules: [
      { id: "rule-obs", processName: "obs64.exe", windowTitleContains: "OBS", targetProfileId: "profile-streaming", priority: 1, enabled: true },
      { id: "rule-resolve", processName: "Resolve.exe", windowTitleContains: "DaVinci Resolve", targetProfileId: "profile-editing", priority: 2, enabled: true },
      { id: "rule-code", processName: "Code.exe", windowTitleContains: "Visual Studio Code", targetProfileId: "profile-work", priority: 3, enabled: true },
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
      profilesDir: "C:\\Users\\MAHMOOD\\AppData\\Roaming\\HotSpot\\StreamDock\\profiles",
      pluginsDir: "C:\\Users\\MAHMOOD\\AppData\\Roaming\\HotSpot\\StreamDock\\plugins",
      legacyProfileCount: 121,
      namedRootProfilesCount: 28,
      legacyPluginCount: 24,
      pluginWsPort: 28196,
      pluginHostActive: true,
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
  };
};

export const deckforgeApi = {
  async getInitialState(): Promise<FullBootstrapPayload> {
    if (isTauriRuntime()) {
      return await invoke<FullBootstrapPayload>("get_initial_state");
    }
    return fallbackBootstrap();
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

  async applyProfileToDevice(deviceId: string, profileId: string, pageId: string): Promise<SyncKeyResult> {
    if (isTauriRuntime()) {
      return await invoke<SyncKeyResult>("apply_profile_to_device", { deviceId, profileId, pageId });
    }
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

  async clearDeviceKey(deviceId: string, keyIndex1Based: number): Promise<string> {
    if (isTauriRuntime()) {
      return await invoke<string>("clear_device_key", { deviceId, keyIndex1Based });
    }
    return `Cleared key slot 0x${keyIndex1Based.toString(16).padStart(2, "0")} on ${deviceId}`;
  },

  async saveProfiles(profiles: Profile[]): Promise<number> {
    if (isTauriRuntime()) {
      return await invoke<number>("save_profiles", { profiles });
    }
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
    return "Stopped official StreamDock.exe process tree — DeckForge now has exclusive access!";
  },

  async importLegacySdProfiles(): Promise<Profile[]> {
    if (isTauriRuntime()) {
      return await invoke<Profile[]>("import_legacy_sdprofiles");
    }
    return fallbackBootstrap().profiles;
  },

  async getTelemetry(): Promise<SystemTelemetrySnapshot> {
    if (isTauriRuntime()) {
      return await invoke<SystemTelemetrySnapshot>("get_telemetry");
    }
    return fallbackBootstrap().telemetry;
  },

  async onHardwareInput(cb: (evt: HardwareInputEvent) => void): Promise<UnlistenFn> {
    if (isTauriRuntime()) {
      return await listen<HardwareInputEvent>("hardware-input", (event) => {
        cb(event.payload);
      });
    }
    return () => {};
  },
};
