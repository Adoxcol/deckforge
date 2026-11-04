export interface KeyGridCapabilities {
  rows: number;
  cols: number;
  count: number;
  pixelWidth: number;
  pixelHeight: number;
  rotation: number;
}

export interface TouchStripCapabilities {
  zones: number;
  pixelWidth: number;
  pixelHeight: number;
  supportsSwipe: boolean;
}

export interface EncoderCapabilities {
  count: number;
  supportsPush: boolean;
  detentsPerRevolution: number;
}

export interface DeviceDescriptor {
  id: string;
  vendorName: string;
  modelName: string;
  vid: number;
  pid: number;
  vidPidHex: string;
  driverName: string;
  protocolFamily: string;
  verifiedDriver: boolean;
  connected: boolean;
  isSimulator: boolean;
  serialNumber: string;
  firmwareVersion: string;
  hidPath: string | null;
  assignedProfileId: string;
  brightness: number;
  keys: KeyGridCapabilities;
  touchStrip: TouchStripCapabilities | null;
  encoders: EncoderCapabilities | null;
  supportsBrightness: boolean;
  lastSeenMs: number;
}

export interface KeyConfig {
  index: number;
  title: string;
  subtitle: string;
  iconName: string;
  customImageDataUrl?: string;
  actionId: string;
  actionLabel: string;
  actionCategory: string;
  actionParam: string;
  accentColor: string;
  bgColor: string;
  showTitle: boolean;
  titleSize: "Small" | "Medium" | "Large";
  backgroundStyle: "Gradient" | "Solid" | "Image";
  cornerRadius: number;
}

export interface TouchZoneConfig {
  index: number;
  label: string;
  valueText: string;
  tapActionId: string;
  accentColor: string;
}

export interface EncoderConfig {
  index: number;
  label: string;
  valueText: string;
  cwAction: string;
  ccwAction: string;
  pushAction: string;
}

export interface ProfilePage {
  id: string;
  name: string;
  keys: KeyConfig[];
  touchZones: TouchZoneConfig[];
  encoders: EncoderConfig[];
}

export interface Profile {
  id: string;
  name: string;
  icon: string;
  colorStrip: string;
  targetDeviceId: string;
  pages: ProfilePage[];
  lastModifiedMs: number;
}

export interface AutoSwitchRule {
  id: string;
  processName: string;
  windowTitleContains: string;
  targetProfileId: string;
  priority: number;
  enabled: boolean;
}

export interface MacroStep {
  id: string;
  stepType: "LaunchApp" | "Delay" | "Shortcut" | "IfWindowExists" | "HttpRequest" | "SwitchProfile";
  label: string;
  parameter: string;
  timeoutMs: number;
  onError: "Stop" | "Continue" | "RetryOnce";
}

export interface MacroSequence {
  id: string;
  name: string;
  description: string;
  steps: MacroStep[];
  lastEditedMs: number;
}

export interface DiagnosticEntry {
  id: string;
  timestampMs: number;
  level: "info" | "success" | "warning" | "error";
  deviceId: string;
  stage: string;
  message: string;
  packetHex: string | null;
}

export interface FrameCacheStats {
  framesEvaluated: number;
  framesTransmitted: number;
  framesDeduplicatedSkipped: number;
  bytesTransmitted: number;
  bytesSaved: number;
  lastTransferLatencyMs: number;
}

export interface PluginPackageInfo {
  id: string;
  name: string;
  author: string;
  version: string;
  description: string;
  category: string;
  sourceKind: "BuiltIn" | "LocalSdPlugin" | "CommunityVerified";
  installed: boolean;
  enabled: boolean;
  permissions: string[];
  actionsProvided: string[];
  diskPath: string | null;
}

export interface LegacyStreamDockSummary {
  profilesDir: string;
  pluginsDir: string;
  legacyProfileCount: number;
  namedRootProfilesCount: number;
  legacyPluginCount: number;
  discoveredPlugins: PluginPackageInfo[];
  pluginWsPort: number;
  pluginHostActive: boolean;
}

export interface SystemTelemetrySnapshot {
  cpuUsagePercent: number;
  memoryUsedGb: number;
  memoryTotalGb: number;
  memoryUsagePercent: number;
  processCount: number;
  officialStreamdockRunning: boolean;
  obsRunning: boolean;
  timestampMs: number;
}

export interface HardwareInputEvent {
  deviceId: string;
  kind:
    | "lcdKey"
    | "rotaryKnobTurnCw"
    | "rotaryKnobTurnCcw"
    | "rotaryKnobPush"
    | "touchStripTap"
    | "touchStripSwipe"
    | "unknown";
  controlIndex: number;
  stateCode: number;
  pressed: boolean;
  rawHex: string;
  timestampMs: number;
}

export interface FullBootstrapPayload {
  devices: DeviceDescriptor[];
  profiles: Profile[];
  autoSwitchRules: AutoSwitchRule[];
  macros: MacroSequence[];
  diagnostics: DiagnosticEntry[];
  frameCacheStats: FrameCacheStats;
  legacySummary: LegacyStreamDockSummary;
  telemetry: SystemTelemetrySnapshot;
  savedLocallyAtMs: number;
  appliedToDeviceAtMs: number | null;
  recentInputEvents: HardwareInputEvent[];
}

export interface SyncKeyResult {
  deviceId: string;
  connectedPhysical: boolean;
  keysEvaluated: number;
  keysTransmitted: number;
  keysDeduplicatedSkipped: number;
  bytesTransmitted: number;
  elapsedMs: number;
  statusMessage: string;
}

export interface ActionExecutionResult {
  actionId: string;
  actionLabel: string;
  success: boolean;
  elapsedMs: number;
  outputMessage: string;
  timestampMs: number;
}

export interface MacroStepResult {
  stepId: string;
  label: string;
  stepType: string;
  status: string;
  elapsedMs: number;
  detail: string;
}

export interface MacroRunReport {
  macroId: string;
  macroName: string;
  totalElapsedMs: number;
  stepResults: MacroStepResult[];
}

export type NavScreenId =
  | "workspace"
  | "devices"
  | "profiles"
  | "plugins"
  | "macros"
  | "icons"
  | "settings";
