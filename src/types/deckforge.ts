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

export interface ScreenMediaConfig {
  mediaId: string;
  fit: "cover" | "contain" | "stretch";
  loopPlayback: boolean;
  speed: number;
}

export interface MediaPreview {
  id: string;
  fileName: string;
  kind: string;
  width: number;
  height: number;
  frameCount: number;
  sourceFrameCount: number;
  frameDelayMs: number;
  delaysMs: number[];
  frames: string[];
}

export interface KeyConfig {
  index: number;
  title: string;
  subtitle: string;
  iconName: string;
  customImageDataUrl?: string;
  media?: ScreenMediaConfig | null;
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
  tapActionParam?: string;
  media?: ScreenMediaConfig | null;
}

export interface EncoderConfig {
  index: number;
  label: string;
  valueText: string;
  cwAction: string;
  ccwAction: string;
  pushAction: string;
  cwParam?: string;
  ccwParam?: string;
  pushParam?: string;
}

export interface ProfilePage {
  id: string;
  name: string;
  keys: KeyConfig[];
  touchZones: TouchZoneConfig[];
  encoders: EncoderConfig[];
  stripMedia?: ScreenMediaConfig | null;
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
  /** Page to open in that profile; empty means the first page. */
  targetPageId?: string;
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
  pluginWsPort: number | null;
  pluginHostActive: boolean;
}

/** Connection state of the physical deck and how the last layout send went. */
export interface DeckStatus {
  state: "searching" | "connected" | "reconnecting" | "paused" | "pausing" | "conflict";
  openedAtMs: number | null;
  lastPushAtMs: number | null;
  lastPushOk: boolean | null;
  lastError: string | null;
  /** Times the deck came back after dropping off USB. */
  reconnects: number;
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
    | "touchStripPoint"
    | "unknown";
  controlIndex: number;
  stateCode: number;
  pressed: boolean;
  rawHex: string;
  timestampMs: number;
}

export interface DeckFaceState {
  mediaPlaying: boolean | null;
  mediaTitle: string;
  micMuted: boolean | null;
  speakerMuted: boolean | null;
  volumePercent: number | null;
  gameMode?: boolean | null;
  gameModeBusy?: boolean;
  cpuPercent?: number | null;
  ramPercent?: number | null;
  clock?: string;
}

export type GameModeGroup = "apps" | "background";

export interface GameModeServiceInfo {
  name: string;
  running: boolean;
}

export interface GameModeTarget {
  id: string;
  label: string;
  group: GameModeGroup;
  note: string;
  running: boolean;
  processCount: number;
  memoryMb: number;
  services: GameModeServiceInfo[];
  needsAdmin: boolean;
  reopens: boolean;
  force: boolean;
  /** Added by the user, so it can be edited and removed. */
  custom: boolean;
  processes: string[];
}

export interface GameModeRunningApp {
  name: string;
  count: number;
  memoryMb: number;
  path: string;
  /** One of its processes owns a normal visible window: an open app, not a background process. */
  hasWindow: boolean;
  /** Label and id of a target that already closes this program. */
  takenBy: string | null;
  takenById: string | null;
}

/** A power plan installed on this PC. */
export interface GameModePowerPlan {
  guid: string;
  name: string;
  active: boolean;
}

export interface GameModeServicePick {
  name: string;
  display: string;
}

export interface GameModeHelperStatus {
  /** The admin helper is running, so services stop without a prompt. */
  armed: boolean;
  /** It already knows every service the current config can stop. */
  covers: boolean;
}

export interface GameModeTweak {
  id: string;
  label: string;
  note: string;
}

export interface GameModeScan {
  targets: GameModeTarget[];
  tweaks: GameModeTweak[];
  ramTotalMb: number;
  ramFreeMb: number;
  processCount: number;
  /** Name of the plan Windows is using right now. */
  powerPlan: string;
  /** Every plan installed on this PC. */
  powerPlans: GameModePowerPlan[];
  isAdmin: boolean;
  protected: string[];
  processes: GameModeRunningApp[];
  services: GameModeServicePick[];
  helper: GameModeHelperStatus;
}

/** One Game Mode: a name plus the targets and tweaks it uses. */
export interface GameModeMode {
  id: string;
  name: string;
  targets: string[];
  tweaks: string[];
  /** GUID of an installed power plan to use while the mode runs. Empty leaves the plan alone. */
  powerPlan: string;
  /** Programs Game Mode starts when it turns on, such as Outplayed or ExitLag. */
  startApps: GameModeStartApp[];
}

/** One action a plugin offers. */
export interface PluginActionView {
  uuid: string;
  name: string;
  tooltip: string;
}

/** An installed Stream Deck / Stream Dock style plugin. */
export interface PluginView {
  uuid: string;
  name: string;
  author: string;
  version: string;
  description: string;
  /** Where it was found: "Stream Dock", "Stream Deck" or "DeckForge". */
  source: string;
  dir: string;
  runtime: "node" | "html" | "exe" | "unknown";
  enabled: boolean;
  /** Connected to DeckForge right now. */
  running: boolean;
  problem: string | null;
  icon: string | null;
  actions: PluginActionView[];
}

/** A plugin that was just installed, plus the updated list. */
export interface PluginInstalled {
  uuid: string;
  name: string;
  plugins: PluginView[];
}

/** An open-source plugin project on GitHub. */
/** A sound output Windows can play to. */
export interface AudioOutput {
  id: string;
  name: string;
  isDefault: boolean;
}

export interface GithubPlugin {
  /** "owner/name". */
  repo: string;
  name: string;
  description: string;
  stars: number;
  url: string;
  ownerAvatar: string;
  updatedAt: string;
}

/** A program Game Mode starts when it turns on. */
export interface GameModeStartApp {
  label: string;
  path: string;
  args: string;
  /** Close it again when Game Mode ends. */
  closeOnEnd: boolean;
}

/** A program or service the user added from what is running. */
export interface GameModeCustomTarget {
  id: string;
  label: string;
  group: "apps" | "background";
  processes: string[];
  services: string[];
  graceful: boolean;
  reopen: boolean;
}

export interface GameModeConfig {
  modes: GameModeMode[];
  /** Used for auto start and for a deck key without a mode. */
  activeMode: string;
  customTargets: GameModeCustomTarget[];
  autoDetect: boolean;
  settleSeconds: number;
  restore: "all" | "settings" | "none";
  elevate: "ask" | "skip";
  armOnLaunch: boolean;
}

export interface GameModeClosedApp {
  id: string;
  label: string;
  memoryMb: number;
  processes: number;
  reopen: string;
}

export interface GameModeStatus {
  active: boolean;
  busy: string | null;
  /** Name of the mode that is running. */
  mode: string;
  startedAtMs: number;
  autoStarted: boolean;
  game: string | null;
  gamePid: number;
  closed: GameModeClosedApp[];
  /** Programs Game Mode started this time. */
  started?: { label: string; pid: number; closeOnEnd: boolean }[];
  stoppedServices: string[];
  failedServices: string[];
  helperUsed: boolean;
  ramFreeBeforeMb: number;
  ramFreeAfterMb: number;
  processesBefore: number;
  processesAfter: number;
  powerPlan: string;
  notes: string[];
  lastEndedMs: number;
}

export interface ObsSettings {
  url: string;
  password: string;
}

export interface DeckContextEvent {
  profileId: string;
  pageIndex: number;
  message: string;
}

export interface FullBootstrapPayload {
  appliedContext?: DeckContextEvent | null;
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
  mediaLibrary: MediaPreview[];
  face?: DeckFaceState;
  obs?: ObsSettings;
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
  | "themes"
  | "gamemode"
  | "settings";

/** Which screen of the deck a picture or action is being edited for. */
export type ScreenTarget = "key" | "zone" | "strip";

export interface WorkspaceStatus {
  deck: DeckStatus;
  applied: {
    profileName: string;
    pageName: string;
    matchesEditor: boolean;
    layoutReady: boolean;
  } | null;
}

export interface RestoredBackup {
  profiles: Profile[];
  macros: MacroSequence[];
  mediaLibrary: MediaPreview[];
}

export interface RecoveryDraft {
  draft: { version: number; profiles: Profile[]; baseProfiles: Profile[]; updatedAtMs: number };
  savedProfilesChanged: boolean;
}
export interface DraftSnapshot { profiles: Profile[]; baseProfiles: Profile[] }
