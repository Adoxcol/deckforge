//! Game Mode: closes the apps and stops the services you picked, keeps the
//! Bitsum Highest Performance power plan active, and puts everything back
//! afterwards.
//!
//! Safety rules, enforced in code and not only in the settings:
//! * anti-cheat (Riot Vanguard, FACEIT, EasyAntiCheat, BattlEye), GPU and audio
//!   drivers, Defender, Explorer, Process Lasso and DeckForge itself are never
//!   matched, even when a target would otherwise catch them;
//! * stopping services needs administrator rights, so it goes through one
//!   elevated helper that DeckForge starts with a single UAC prompt. The helper
//!   only touches service names from a baked whitelist (never the deny list),
//!   restarts them when Game Mode ends, and stays resident until DeckForge exits,
//!   so there is only ever one prompt per session;
//! * a UAC prompt is never shown while a game is full-screen. The secure desktop
//!   switch resets the display driver and froze the PC for several seconds, so an
//!   automatic start only stops services when the helper is already running;
//! * work is spread out: one app at a time, on a background-priority thread, and
//!   an automatic start waits a few seconds after the game appears;
//! * IDEs get time to save and are left open if they still show a prompt.

use crate::drivers::hal::now_ms;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};
use tauri::Emitter;
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Threading::{
    GetCurrentThread, GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW, SetPriorityClass,
    SetThreadPriority, TerminateProcess, HIGH_PRIORITY_CLASS, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SET_INFORMATION, PROCESS_TERMINATE, THREAD_MODE_BACKGROUND_BEGIN, THREAD_MODE_BACKGROUND_END,
};
use windows::Win32::UI::Shell::{IsUserAnAdmin, SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible,
    PostMessageW, WM_CLOSE,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const STILL_ACTIVE: u32 = 259;
const MB: u64 = 1024 * 1024;

// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reopen {
    /// Start the recorded exe (with its original arguments) again.
    Exe,
    /// Start a Microsoft Store app. The text is a regex over Get-StartApps names.
    Store(&'static str),
    /// Nothing to reopen (services come back on their own, dev servers do not).
    No,
}

struct TargetDef {
    id: &'static str,
    label: &'static str,
    /// "apps", "ai" or "background".
    group: &'static str,
    note: &'static str,
    /// Lowercase exe names without ".exe". The first entry is the main program.
    procs: &'static [&'static str],
    /// Lowercase name prefixes.
    prefixes: &'static [&'static str],
    /// Lowercase exe path fragments that also match.
    path_any: &'static [&'static str],
    /// When set, name matches only count if the exe path contains this.
    path_required: Option<&'static str>,
    /// Names that are never matched (SYSTEM services that live next to the app).
    skip: &'static [&'static str],
    /// Windows services stopped (and restarted afterwards) through the helper.
    services: &'static [&'static str],
    /// Ask windows to close first, so apps can save and exit cleanly.
    graceful: bool,
    /// Kill whatever is still running after the grace period. Off for IDEs.
    force: bool,
    reopen: Reopen,
    /// Run `wsl --shutdown` once the processes are gone.
    wsl_shutdown: bool,
    /// Keep DeckForge's own dev stack (vite, tauri) alive.
    protect_dev: bool,
    default_on: bool,
}

const BASE: TargetDef = TargetDef {
    id: "",
    label: "",
    group: "apps",
    note: "",
    procs: &[],
    prefixes: &[],
    path_any: &[],
    path_required: None,
    skip: &[],
    services: &[],
    graceful: false,
    force: true,
    reopen: Reopen::No,
    wsl_shutdown: false,
    protect_dev: false,
    default_on: true,
};

const TARGETS: &[TargetDef] = &[
    TargetDef {
        id: "zen",
        label: "Zen Browser",
        note: "Closes every Zen window and reopens it afterwards.",
        procs: &["zen"],
        graceful: true,
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "cursor",
        label: "Cursor",
        note: "This chat runs inside Cursor, so it ends when Cursor closes. Cursor keeps open if it asks about unsaved files.",
        procs: &["cursor", "cursor-cua-sidecar"],
        graceful: true,
        force: false,
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "antigravity",
        label: "Antigravity",
        note: "Also closes its language server. Stays open if it asks about unsaved files.",
        procs: &["antigravity", "language_server"],
        path_required: Some("antigravity"),
        graceful: true,
        force: false,
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "docker_wsl",
        label: "Docker Desktop + WSL",
        note: "Stops Docker and shuts the WSL VM down, which frees about 1.5 GB. Running containers stop.",
        procs: &["docker desktop"],
        prefixes: &["com.docker."],
        reopen: Reopen::Exe,
        wsl_shutdown: true,
        ..BASE
    },
    TargetDef {
        id: "node",
        label: "Node dev servers",
        note: "Ends every node process except DeckForge's own dev tools. They are not reopened.",
        procs: &["node"],
        protect_dev: true,
        ..BASE
    },
    TargetDef {
        id: "qbit",
        label: "qBittorrent",
        note: "Torrents pause until it is reopened.",
        procs: &["qbittorrent"],
        graceful: true,
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "syncthing",
        label: "Syncthing",
        note: "File sync pauses while Game Mode is on.",
        group: "background",
        procs: &["syncthing"],
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "ai_claude",
        label: "Claude",
        note: "Closes the Claude app and stops its Cowork VM service.",
        group: "ai",
        path_any: &["\\windowsapps\\claude_", "\\anthropicclaude\\"],
        skip: &["cowork-svc"],
        services: &["CoworkVMService"],
        graceful: true,
        reopen: Reopen::Store("^Claude$"),
        ..BASE
    },
    TargetDef {
        id: "ai_codex",
        label: "Codex / ChatGPT",
        note: "Closes the Codex and ChatGPT app and stops its sandbox service.",
        group: "ai",
        procs: &["chatgpt"],
        path_any: &["\\windowsapps\\openai.codex_", "\\windowsapps\\openai.chatgpt_"],
        skip: &["codex-windows-sandbox-service"],
        services: &["CodexSandboxService.OpenAI.Codex"],
        graceful: true,
        reopen: Reopen::Store("Codex|ChatGPT"),
        ..BASE
    },
    TargetDef {
        id: "ai_copilot",
        label: "Microsoft Copilot",
        note: "Closes the Copilot app.",
        group: "ai",
        path_any: &["\\windowsapps\\microsoft.copilot_"],
        graceful: true,
        reopen: Reopen::Store("Copilot"),
        ..BASE
    },
    TargetDef {
        id: "ai_gemini",
        label: "Gemini app",
        note: "Closes the Gemini desktop launcher.",
        group: "ai",
        procs: &["geminiapplauncher"],
        path_any: &["\\google\\gemini\\"],
        graceful: true,
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "ai_local",
        label: "Ollama / LM Studio",
        note: "Stops local model servers and frees their GPU memory.",
        group: "ai",
        procs: &["ollama", "ollama app", "lm studio"],
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "mongo",
        label: "MongoDB",
        note: "Database server service.",
        group: "background",
        services: &["MongoDB"],
        ..BASE
    },
    TargetDef {
        id: "postgres",
        label: "PostgreSQL + PEM web server",
        note: "PostgreSQL 18 and the EDB PEM Apache service.",
        group: "background",
        services: &["postgresql-x64-18", "PEMHTTPD-x64"],
        ..BASE
    },
    TargetDef {
        id: "spooler",
        label: "Print Spooler",
        note: "Printing is unavailable until Game Mode ends.",
        group: "background",
        services: &["Spooler"],
        ..BASE
    },
    TargetDef {
        id: "bonjour",
        label: "Bonjour",
        note: "Apple network discovery service.",
        group: "background",
        services: &["Bonjour Service"],
        ..BASE
    },
    TargetDef {
        id: "sqlvss",
        label: "SQL Server VSS Writer",
        note: "Backup helper for SQL Server.",
        group: "background",
        services: &["SQLWriter"],
        ..BASE
    },
    TargetDef {
        id: "asc",
        label: "Advanced SystemCare",
        note: "Stops its tray app and background service.",
        group: "background",
        procs: &["asctray"],
        services: &["AdvancedSystemCareService19"],
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "pcmgr",
        label: "Microsoft PC Manager",
        note: "Stops its background service and window.",
        group: "background",
        procs: &["pcmanager"],
        services: &["PCManager Service Store"],
        ..BASE
    },
    TargetDef {
        id: "warp",
        label: "Cloudflare WARP",
        note: "Drops the WARP VPN connection while Game Mode is on.",
        group: "background",
        procs: &["cloudflare warp"],
        services: &["CloudflareWARP"],
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "spotify",
        label: "Spotify",
        note: "Music stops.",
        procs: &["spotify"],
        graceful: true,
        reopen: Reopen::Exe,
        default_on: false,
        ..BASE
    },
    TargetDef {
        id: "discord",
        label: "Discord",
        note: "You leave voice chat.",
        procs: &["discord"],
        graceful: true,
        reopen: Reopen::Exe,
        default_on: false,
        ..BASE
    },
    TargetDef {
        id: "steam",
        label: "Steam client",
        note: "Leave this on if the game starts from Steam.",
        procs: &["steam", "steamwebhelper"],
        graceful: true,
        reopen: Reopen::Exe,
        default_on: false,
        ..BASE
    },
    TargetDef {
        id: "dlmm",
        label: "Deadlock Mod Manager",
        note: "Mods stay as they are, the manager just closes.",
        procs: &["deadlock-mod-manager"],
        graceful: true,
        reopen: Reopen::Exe,
        default_on: false,
        ..BASE
    },
    TargetDef {
        id: "riotclient",
        label: "Riot Client launcher",
        note: "Closes the launcher only. Vanguard anti-cheat is never touched.",
        procs: &["riotclientservices", "riotclientux", "riotclientuxrender"],
        graceful: true,
        reopen: Reopen::Exe,
        default_on: false,
        ..BASE
    },
];

/// Services paused by the "pause_updates" tweak.
const UPDATE_SERVICES: &[&str] = &["wuauserv", "UsoSvc", "DoSvc", "BITS"];

struct TweakDef {
    id: &'static str,
    label: &'static str,
    note: &'static str,
}

const TWEAKS: &[TweakDef] = &[
    TweakDef {
        id: "priority",
        label: "Raise the game's priority to High",
        note: "Applies to the full-screen game as soon as DeckForge sees it.",
    },
    TweakDef {
        id: "pause_updates",
        label: "Pause Windows Update and Delivery Optimization",
        note: "Stops the update services while you play. Needs the admin helper.",
    },
];

/// Safety net: names that no target may ever touch.
const NEVER_KILL_NAMES: &[&str] = &[
    "explorer",
    "dwm",
    "csrss",
    "winlogon",
    "lsass",
    "services",
    "svchost",
    "system",
    "smss",
    "wininit",
    "audiodg",
    "deckforge",
    "processlasso",
    "processgovernor",
    "voicemeeter8x64",
    "vgc",
    "vgtray",
    "msmpeng",
    "nvcontainer",
    "nvdisplay.container",
];

const NEVER_KILL_PATHS: &[&str] = &[
    "\\riot vanguard\\",
    "\\easyanticheat",
    "\\battleye",
    "\\faceit anti",
    "\\nvidia corporation\\",
    "\\windows defender\\",
    "\\windows\\system32\\",
];

/// Shown in the UI so it is clear what Game Mode leaves alone.
pub const PROTECTED_NOTES: &[&str] = &[
    "Riot Vanguard, FACEIT and other anti-cheat",
    "NVIDIA drivers and overlay services",
    "Audio routing (Voicemeeter, Focusrite) and Windows audio",
    "Windows Defender and security services",
    "Explorer, DWM and core Windows processes",
    "Process Lasso",
    "DeckForge itself",
];

/// Programs that are never treated as a game, even when full-screen.
const NOT_GAMES: &[&str] = &[
    "explorer", "searchhost", "shellexperiencehost", "applicationframehost", "lockapp", "zen",
    "chrome", "msedge", "firefox", "brave", "opera", "vivaldi", "vlc", "mpc-hc64", "potplayermini64",
    "cursor", "code", "antigravity", "spotify", "discord", "obs64", "deckforge", "teams", "ms-teams",
    "zoom", "slack", "mstsc", "vmconnect",
];

/// Install folders that mark a full-screen window as a game.
const GAME_PATHS: &[&str] = &[
    "\\steamapps\\common\\",
    "\\riot games\\",
    "\\epic games\\",
    "\\faceit",
    "\\battle.net",
    "\\ubisoft",
    "\\ea games\\",
    "\\gog galaxy\\games\\",
    "\\xboxgames\\",
    "\\roblox\\",
    "\\rockstar games\\",
    "\\origin games\\",
];

// ---------------------------------------------------------------------------
// Persistent config and state
// ---------------------------------------------------------------------------

/// One Game Mode: a name and the targets and tweaks it uses. People keep several, for
/// example "Competitive" (close everything) and "Casual" (keep Discord and music).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ModeConfig {
    pub id: String,
    pub name: String,
    pub targets: Vec<String>,
    pub tweaks: Vec<String>,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default".to_string(),
            targets: TARGETS.iter().filter(|t| t.default_on).map(|t| t.id.to_string()).collect(),
            tweaks: TWEAKS.iter().map(|t| t.id.to_string()).collect(),
        }
    }
}

/// A program or service the user added themselves, picked from what is running.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CustomTarget {
    pub id: String,
    pub label: String,
    /// "apps" or "background".
    pub group: String,
    /// Lowercase exe names without ".exe".
    pub processes: Vec<String>,
    /// Windows service names, stopped through the admin helper.
    pub services: Vec<String>,
    /// Ask windows to close first. When off the program is ended straight away.
    pub graceful: bool,
    /// Start the program again when Game Mode ends.
    pub reopen: bool,
}

impl Default for CustomTarget {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            group: "apps".to_string(),
            processes: Vec::new(),
            services: Vec::new(),
            graceful: true,
            reopen: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct GameModeConfig {
    pub modes: Vec<ModeConfig>,
    /// The mode that starts automatically and that a deck key without a parameter uses.
    pub active_mode: String,
    pub custom_targets: Vec<CustomTarget>,
    pub auto_detect: bool,
    /// Seconds an automatic start waits after a game appears, so closing apps does not
    /// compete with the game loading.
    pub settle_seconds: u32,
    /// "all" reopens apps and restores services, "settings" restores services only, "none" restores nothing.
    pub restore: String,
    /// "ask" shows one Windows prompt to start the admin helper (never during a game), "skip" never stops services.
    pub elevate: String,
    /// Start the admin helper a few seconds after DeckForge opens, so no prompt is needed later.
    pub arm_on_launch: bool,
    /// Old single-mode fields, read once and moved into `modes`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tweaks: Option<Vec<String>>,
}

impl Default for GameModeConfig {
    fn default() -> Self {
        Self {
            modes: vec![ModeConfig::default()],
            active_mode: "default".to_string(),
            custom_targets: Vec::new(),
            auto_detect: true,
            settle_seconds: 20,
            restore: "all".to_string(),
            elevate: "ask".to_string(),
            arm_on_launch: false,
            targets: None,
            tweaks: None,
        }
    }
}

/// Services a user-added target may never touch, whatever the file says.
const SERVICE_DENY_EXACT: &[&str] = &[
    "rpcss", "rpceptmapper", "dcomlaunch", "eventlog", "lsm", "samss", "wscsvc", "sense",
    "securityhealthservice", "mpssvc", "bfe", "winmgmt", "plugplay", "power", "profsvc",
    "cryptsvc", "trustedinstaller", "schedule", "dhcp", "dnscache", "nsi", "netprofm", "nlasvc",
    "themes", "usermanager", "gpsvc", "sens", "shellhwdetection", "lanmanserver",
    "lanmanworkstation", "keyiso", "hidserv", "uxsms", "sessionenv", "termservice", "wudfsvc",
    "audiosrv", "audioendpointbuilder", "vgc", "vgk", "beservice", "windefend", "wdnissvc",
    "wlansvc", "wcmsvc", "tcpip", "netlogon", "browser", "msiserver",
];
const SERVICE_DENY_PREFIX: &[&str] = &["nv", "easyanticheat", "faceit", "vgc", "vgk", "wd", "mpssvc", "rpc"];

fn service_denied(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    SERVICE_DENY_EXACT.contains(&lower.as_str()) || SERVICE_DENY_PREFIX.iter().any(|p| lower.starts_with(p))
}

fn clean_process_name(raw: &str) -> Option<String> {
    let lower = raw.trim().to_lowercase();
    let stem = lower.strip_suffix(".exe").unwrap_or(&lower).trim().to_string();
    if stem.is_empty() || stem.len() > 60 {
        return None;
    }
    if !stem
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ' | '+' | '(' | ')'))
    {
        return None;
    }
    if NEVER_KILL_NAMES.contains(&stem.as_str()) {
        return None;
    }
    Some(stem)
}

fn clean_service_name(raw: &str) -> Option<String> {
    let name = raw.trim().to_string();
    if name.is_empty() || name.len() > 80 {
        return None;
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ' | '$'))
    {
        return None;
    }
    if service_denied(&name) {
        return None;
    }
    Some(name)
}

fn clean_id(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(48)
        .collect()
}

fn clean_label(raw: &str, fallback: &str) -> String {
    let label: String = raw.trim().chars().filter(|c| !c.is_control()).take(40).collect();
    if label.is_empty() {
        fallback.to_string()
    } else {
        label
    }
}

/// Moves the old single-mode fields into a mode, drops anything unknown, and keeps the
/// user's own targets inside the safety rules.
fn sanitize(mut cfg: GameModeConfig) -> GameModeConfig {
    // Custom targets first, because modes may refer to them.
    let builtin_ids: HashSet<&str> = TARGETS.iter().map(|t| t.id).collect();
    let mut seen_custom = HashSet::new();
    let mut customs: Vec<CustomTarget> = Vec::new();
    for mut c in std::mem::take(&mut cfg.custom_targets).into_iter().take(40) {
        c.id = clean_id(&c.id);
        if c.id.is_empty() || builtin_ids.contains(c.id.as_str()) || !seen_custom.insert(c.id.clone()) {
            continue;
        }
        let mut procs: Vec<String> = c.processes.iter().filter_map(|p| clean_process_name(p)).collect();
        let mut seen = HashSet::new();
        procs.retain(|p| seen.insert(p.clone()));
        procs.truncate(20);
        let mut services: Vec<String> = c.services.iter().filter_map(|s| clean_service_name(s)).collect();
        let mut seen = HashSet::new();
        services.retain(|s| seen.insert(s.to_lowercase()));
        services.truncate(20);
        if procs.is_empty() && services.is_empty() {
            continue;
        }
        c.label = clean_label(&c.label, procs.first().map(String::as_str).unwrap_or(&c.id));
        c.group = if c.group == "background" { "background".to_string() } else { "apps".to_string() };
        c.processes = procs;
        c.services = services;
        customs.push(c);
    }
    cfg.custom_targets = customs;

    let mut known: HashSet<String> = builtin_ids.iter().map(|s| s.to_string()).collect();
    known.extend(cfg.custom_targets.iter().map(|c| c.id.clone()));
    let known_tweaks: HashSet<&str> = TWEAKS.iter().map(|t| t.id).collect();

    // Old files kept one list of targets and tweaks. They become the first mode.
    if cfg.modes.is_empty() {
        let mut mode = ModeConfig::default();
        if let Some(targets) = cfg.targets.take() {
            mode.targets = targets;
        }
        if let Some(tweaks) = cfg.tweaks.take() {
            mode.tweaks = tweaks;
        }
        cfg.modes.push(mode);
    }
    cfg.targets = None;
    cfg.tweaks = None;

    let mut seen_modes = HashSet::new();
    let mut modes: Vec<ModeConfig> = Vec::new();
    for mut mode in std::mem::take(&mut cfg.modes).into_iter().take(12) {
        mode.id = clean_id(&mode.id);
        if mode.id.is_empty() || !seen_modes.insert(mode.id.clone()) {
            continue;
        }
        mode.name = clean_label(&mode.name, "Game Mode");
        let mut seen = HashSet::new();
        mode.targets.retain(|id| known.contains(id) && seen.insert(id.clone()));
        let mut seen = HashSet::new();
        mode.tweaks.retain(|id| known_tweaks.contains(id.as_str()) && seen.insert(id.clone()));
        modes.push(mode);
    }
    if modes.is_empty() {
        modes.push(ModeConfig::default());
    }
    cfg.modes = modes;
    if !cfg.modes.iter().any(|m| m.id == cfg.active_mode) {
        cfg.active_mode = cfg.modes[0].id.clone();
    }

    cfg.settle_seconds = cfg.settle_seconds.min(120);
    if !matches!(cfg.restore.as_str(), "all" | "settings" | "none") {
        cfg.restore = "all".to_string();
    }
    // "uac" was the old name for "ask".
    if cfg.elevate == "uac" {
        cfg.elevate = "ask".to_string();
    }
    if !matches!(cfg.elevate.as_str(), "ask" | "skip") {
        cfg.elevate = "ask".to_string();
    }
    cfg
}

/// The mode to run: the one asked for, else the active one, else the first.
fn pick_mode<'a>(cfg: &'a GameModeConfig, wanted: Option<&str>) -> &'a ModeConfig {
    wanted
        .and_then(|id| cfg.modes.iter().find(|m| m.id == id))
        .or_else(|| cfg.modes.iter().find(|m| m.id == cfg.active_mode))
        .unwrap_or(&cfg.modes[0])
}

// Custom targets are turned into `TargetDef`s once and kept, so the matching code
// below works the same for built-in and user-added targets.
static CUSTOM_DEFS: OnceLock<Mutex<HashMap<String, &'static TargetDef>>> = OnceLock::new();

fn leak_str(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

fn leak_list(items: &[String]) -> &'static [&'static str] {
    let list: Vec<&'static str> = items.iter().map(|s| leak_str(s)).collect();
    Box::leak(list.into_boxed_slice())
}

fn custom_def(custom: &CustomTarget) -> &'static TargetDef {
    let key = serde_json::to_string(custom).unwrap_or_default();
    let mut cache = CUSTOM_DEFS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(found) = cache.get(&key) {
        return *found;
    }
    let def: &'static TargetDef = Box::leak(Box::new(TargetDef {
        id: leak_str(&custom.id),
        label: leak_str(&custom.label),
        group: if custom.group == "background" { "background" } else { "apps" },
        note: "Added by you.",
        procs: leak_list(&custom.processes),
        services: leak_list(&custom.services),
        graceful: custom.graceful,
        force: true,
        reopen: if custom.reopen { Reopen::Exe } else { Reopen::No },
        default_on: false,
        ..BASE
    }));
    cache.insert(key, def);
    def
}

/// Built-in presets followed by the user's own targets.
fn all_targets(cfg: &GameModeConfig) -> Vec<&'static TargetDef> {
    TARGETS.iter().chain(cfg.custom_targets.iter().map(custom_def)).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ClosedApp {
    pub id: String,
    pub label: String,
    pub memory_mb: u64,
    pub processes: usize,
    /// "exe", "store" or "no".
    pub reopen: String,
    pub exe: Option<String>,
    pub args: Vec<String>,
    pub store: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GameModeStatus {
    pub active: bool,
    pub busy: Option<String>,
    /// Name of the mode that is running.
    pub mode: String,
    pub started_at_ms: u64,
    pub auto_started: bool,
    pub game: Option<String>,
    pub game_pid: u32,
    pub closed: Vec<ClosedApp>,
    pub stopped_services: Vec<String>,
    pub failed_services: Vec<String>,
    pub helper_used: bool,
    pub ram_free_before_mb: u64,
    pub ram_free_after_mb: u64,
    pub processes_before: usize,
    pub processes_after: usize,
    pub power_plan: String,
    pub notes: Vec<String>,
    pub last_ended_ms: u64,
}

static CONFIG: OnceLock<Mutex<GameModeConfig>> = OnceLock::new();
static STATE: OnceLock<Mutex<GameModeStatus>> = OnceLock::new();
static ACTIVE: AtomicBool = AtomicBool::new(false);
static BUSY: AtomicBool = AtomicBool::new(false);
static GAME_PID: AtomicU32 = AtomicU32::new(0);
static SUPPRESS_PID: AtomicU32 = AtomicU32::new(0);

fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("DeckForge")
}

fn config_path() -> PathBuf {
    data_dir().join("game-mode.json")
}

fn state_path() -> PathBuf {
    data_dir().join("game-mode-state.json")
}

fn helper_json_path() -> PathBuf {
    data_dir().join("game-mode-helper.json")
}

fn cmd_path() -> PathBuf {
    data_dir().join("game-mode-cmd.json")
}

/// Reads a config file. A file from before modes existed has no `modes` key, which must
/// stay empty (not the default mode) so `sanitize` can move its old lists into a mode.
fn parse_config(raw: &str) -> Option<GameModeConfig> {
    let mut value: serde_json::Value = serde_json::from_str(raw.trim_start_matches('\u{feff}').trim()).ok()?;
    if let Some(object) = value.as_object_mut() {
        if !object.contains_key("modes") {
            object.insert("modes".to_string(), serde_json::json!([]));
        }
    }
    serde_json::from_value(value).ok()
}

fn config_slot() -> &'static Mutex<GameModeConfig> {
    CONFIG.get_or_init(|| {
        let cfg = std::fs::read_to_string(config_path())
            .ok()
            .and_then(|raw| parse_config(&raw))
            .unwrap_or_default();
        Mutex::new(sanitize(cfg))
    })
}

pub fn config() -> GameModeConfig {
    config_slot().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn save_config(cfg: GameModeConfig) -> Result<GameModeConfig, String> {
    let cfg = sanitize(cfg);
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(config_path(), raw).map_err(|e| e.to_string())?;
    *config_slot().lock().unwrap_or_else(|e| e.into_inner()) = cfg.clone();
    Ok(cfg)
}

fn state_slot() -> &'static Mutex<GameModeStatus> {
    STATE.get_or_init(|| {
        let mut state = std::fs::read_to_string(state_path())
            .ok()
            .and_then(|raw| serde_json::from_str::<GameModeStatus>(&raw).ok())
            .unwrap_or_default();
        // A reboot already brought every app and service back, so there is nothing to restore.
        if state.active && state.started_at_ms / 1000 < System::boot_time() {
            state = GameModeStatus::default();
        }
        state.busy = None;
        ACTIVE.store(state.active, Ordering::SeqCst);
        GAME_PID.store(state.game_pid, Ordering::SeqCst);
        Mutex::new(state)
    })
}

fn persist_state(state: &GameModeStatus) {
    let _ = std::fs::create_dir_all(data_dir());
    if let Ok(raw) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(state_path(), raw);
    }
}

pub fn is_active() -> bool {
    let _ = state_slot();
    ACTIVE.load(Ordering::SeqCst)
}

pub fn is_busy() -> bool {
    BUSY.load(Ordering::SeqCst)
}

pub fn status() -> GameModeStatus {
    let mut state = state_slot().lock().unwrap_or_else(|e| e.into_inner()).clone();
    state.active = ACTIVE.load(Ordering::SeqCst);
    state
}

fn publish(app: &tauri::AppHandle) {
    let _ = app.emit("game-mode", status());
}

fn with_state(update: impl FnOnce(&mut GameModeStatus)) {
    let mut state = state_slot().lock().unwrap_or_else(|e| e.into_inner());
    update(&mut state);
}

struct BusyGuard<'a> {
    app: &'a tauri::AppHandle,
}

fn claim_busy<'a>(app: &'a tauri::AppHandle, label: &str) -> Option<BusyGuard<'a>> {
    let _ = state_slot();
    if BUSY.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return None;
    }
    with_state(|s| s.busy = Some(label.to_string()));
    publish(app);
    Some(BusyGuard { app })
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        with_state(|s| s.busy = None);
        BUSY.store(false, Ordering::SeqCst);
        publish(self.app);
    }
}

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Proc {
    pid: u32,
    parent: Option<u32>,
    /// Lowercase, without ".exe".
    name: String,
    exe: Option<PathBuf>,
    exe_lower: String,
    cmd: Vec<String>,
    mem: u64,
}

fn snapshot() -> Vec<Proc> {
    let mut sys = System::new_with_specifics(RefreshKind::nothing());
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_memory()
            .with_exe(UpdateKind::Always)
            .with_cmd(UpdateKind::Always),
    );
    sys.processes()
        .values()
        .map(|p| {
            let raw = p.name().to_string_lossy().to_lowercase();
            let name = raw.strip_suffix(".exe").unwrap_or(&raw).to_string();
            let exe = p.exe().map(|e| e.to_path_buf());
            let exe_lower = exe
                .as_ref()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            Proc {
                pid: p.pid().as_u32(),
                parent: p.parent().map(|x| x.as_u32()),
                name,
                exe,
                exe_lower,
                cmd: p.cmd().iter().map(|c| c.to_string_lossy().to_string()).collect(),
                mem: p.memory(),
            }
        })
        .collect()
}

/// This process and every parent of it, so Game Mode can never end the app that launched it.
fn protected_pids(procs: &[Proc]) -> HashSet<u32> {
    let by_pid: HashMap<u32, &Proc> = procs.iter().map(|p| (p.pid, p)).collect();
    let mut set = HashSet::new();
    let mut current = Some(std::process::id());
    while let Some(pid) = current {
        if !set.insert(pid) {
            break;
        }
        current = by_pid.get(&pid).and_then(|p| p.parent);
    }
    set
}

fn never_kill(p: &Proc) -> bool {
    NEVER_KILL_NAMES.contains(&p.name.as_str())
        || NEVER_KILL_PATHS.iter().any(|frag| p.exe_lower.contains(frag))
}

fn is_dev_stack(p: &Proc) -> bool {
    let joined = p.cmd.join(" ").to_lowercase();
    joined.contains("deckforge") || joined.contains("tauri") || joined.contains("vite")
}

fn matches(def: &TargetDef, p: &Proc) -> bool {
    if def.skip.contains(&p.name.as_str()) {
        return false;
    }
    let by_name = def.procs.contains(&p.name.as_str())
        || def.prefixes.iter().any(|prefix| p.name.starts_with(prefix));
    if by_name {
        return match def.path_required {
            Some(required) => p.exe_lower.contains(required),
            None => true,
        };
    }
    !p.exe_lower.is_empty() && def.path_any.iter().any(|frag| p.exe_lower.contains(frag))
}

fn has_process_rules(def: &TargetDef) -> bool {
    !(def.procs.is_empty() && def.prefixes.is_empty() && def.path_any.is_empty())
}

fn matched_procs<'a>(def: &TargetDef, procs: &'a [Proc], protect: &HashSet<u32>) -> Vec<&'a Proc> {
    if !has_process_rules(def) {
        return Vec::new();
    }
    procs
        .iter()
        .filter(|p| matches(def, p))
        .filter(|p| !protect.contains(&p.pid) && !never_kill(p))
        .filter(|p| !(def.protect_dev && is_dev_stack(p)))
        .collect()
}

fn free_ram_mb() -> u64 {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.available_memory() / MB
}

fn is_admin() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

fn pid_alive(pid: u32) -> bool {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let mut code = 0u32;
        let ok = GetExitCodeProcess(handle, &mut code).is_ok();
        let _ = CloseHandle(handle);
        ok && code == STILL_ACTIVE
    }
}

fn terminate(pid: u32) -> bool {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) else {
            return false;
        };
        let ok = TerminateProcess(handle, 1).is_ok();
        let _ = CloseHandle(handle);
        ok
    }
}

fn raise_priority(pid: u32) -> bool {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_SET_INFORMATION, false, pid) else {
            return false;
        };
        let ok = SetPriorityClass(handle, HIGH_PRIORITY_CLASS).is_ok();
        let _ = CloseHandle(handle);
        ok
    }
}

/// While alive, the current thread runs in Windows background mode: lowest CPU, disk and
/// memory priority, so closing and reopening apps yields to a game that is loading.
struct BackgroundWork;

impl BackgroundWork {
    fn begin() -> Self {
        unsafe {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
        }
        BackgroundWork
    }
}

impl Drop for BackgroundWork {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_END);
        }
    }
}

unsafe extern "system" fn close_enum(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let pids = &*(lparam.0 as *const HashSet<u32>);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pids.contains(&pid) && IsWindowVisible(hwnd).as_bool() {
        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
    BOOL(1)
}

/// Asks every visible window of these processes to close, the way the X button does.
fn post_close(pids: &HashSet<u32>) {
    unsafe {
        let _ = EnumWindows(Some(close_enum), LPARAM(pids as *const HashSet<u32> as isize));
    }
}

fn wait_until_gone(pids: &HashSet<u32>, limit: Duration) {
    let started = Instant::now();
    while started.elapsed() < limit {
        if !pids.iter().any(|pid| pid_alive(*pid)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

// ---------------------------------------------------------------------------
// Power plan
// ---------------------------------------------------------------------------

fn powercfg(args: &[&str]) -> String {
    Command::new("powercfg")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

fn guid_in(line: &str) -> Option<String> {
    line.split_whitespace()
        .map(|token| token.trim_matches(|c| c == '*' || c == '(' || c == ')'))
        .find(|token| token.len() == 36 && token.matches('-').count() == 4)
        .map(|token| token.to_lowercase())
}

fn plan_label(line: &str) -> String {
    match (line.find('('), line.rfind(')')) {
        (Some(start), Some(end)) if end > start => line[start + 1..end].trim().to_string(),
        _ => String::new(),
    }
}

fn active_plan() -> Option<(String, String)> {
    let out = powercfg(&["/getactivescheme"]);
    let line = out.lines().find(|l| guid_in(l).is_some())?;
    Some((guid_in(line)?, plan_label(line)))
}

fn bitsum_plan() -> Option<(String, String)> {
    let out = powercfg(&["/list"]);
    out.lines()
        .find(|l| l.to_lowercase().contains("bitsum highest performance"))
        .and_then(|l| Some((guid_in(l)?, plan_label(l))))
}

/// Game Mode always runs on Bitsum Highest Performance and never switches to another plan.
fn ensure_bitsum_plan(notes: &mut Vec<String>) -> String {
    let Some((guid, name)) = bitsum_plan() else {
        notes.push("Bitsum Highest Performance was not found, so the power plan was left alone.".to_string());
        return active_plan().map(|(_, n)| n).unwrap_or_default();
    };
    let active = active_plan();
    if active.as_ref().map(|(g, _)| g == &guid).unwrap_or(false) {
        return name;
    }
    let _ = powercfg(&["/setactive", &guid]);
    notes.push(format!("Switched the power plan to {name}."));
    name
}

// ---------------------------------------------------------------------------
// Services (elevated helper)
// ---------------------------------------------------------------------------

/// Every Win32 service as (name, display name, running), read once from `sc query`.
fn service_list() -> Vec<(String, String, bool)> {
    let out = Command::new("sc.exe")
        .args(["query", "type=", "service", "state=", "all"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let mut list = Vec::new();
    let mut name: Option<String> = None;
    let mut display = String::new();
    for line in out.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("SERVICE_NAME:") {
            name = Some(value.trim().to_string());
            display.clear();
        } else if let Some(value) = line.strip_prefix("DISPLAY_NAME:") {
            display = value.trim().to_string();
        } else if line.starts_with("STATE") {
            if let Some(found) = name.take() {
                let shown = if display.is_empty() { found.clone() } else { display.clone() };
                list.push((found, shown, line.contains("RUNNING")));
            }
        }
    }
    list
}

fn service_states() -> HashMap<String, bool> {
    service_list()
        .into_iter()
        .map(|(name, _, running)| (name.to_lowercase(), running))
        .collect()
}

fn ps_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Quoted PowerShell string list, for example `'a','b'`.
fn ps_list(items: &[&str]) -> String {
    items.iter().map(|s| ps_quote(s)).collect::<Vec<_>>().join(",")
}

/// The script the elevated helper runs. It stays resident until DeckForge exits, waits for
/// `game-mode-cmd.json`, and only ever touches the service names baked in below.
fn helper_script(services: &[String], parent: u32) -> String {
    let dir = data_dir();
    let names = services.iter().map(|s| ps_quote(s)).collect::<Vec<_>>().join(",");
    format!(
        r#"$ErrorActionPreference = 'SilentlyContinue'
Add-Type -AssemblyName System.ServiceProcess
$dir = {dir}
$out = Join-Path $dir 'game-mode-helper.json'
$cmdFile = Join-Path $dir 'game-mode-cmd.json'
$parentPid = {parent}
$allowed = @({names})
$denyExact = @({deny_exact})
$denyPrefix = @({deny_prefix})
try {{ (Get-Process -Id $PID).PriorityClass = 'BelowNormal' }} catch {{}}
$stopped = New-Object System.Collections.ArrayList
$failed = @()
$done = ''
function Test-Denied([string]$n) {{
  $l = $n.ToLower()
  if ($denyExact -contains $l) {{ return $true }}
  foreach ($p in $denyPrefix) {{ if ($l.StartsWith($p)) {{ return $true }} }}
  return $false
}}
function Save-State {{
  $o = [pscustomobject]@{{ armed = $true; pid = $PID; names = $allowed; done = $done; stopped = @($stopped); failed = @($failed) }}
  Set-Content -Path $out -Value ($o | ConvertTo-Json -Compress) -Encoding UTF8
}}
function Stop-One([string]$n) {{
  try {{ $sc = New-Object System.ServiceProcess.ServiceController($n); $sc.Stop() }} catch {{ try {{ Stop-Service -Name $n -Force -ErrorAction Stop }} catch {{}} }}
  $until = (Get-Date).AddSeconds(25)
  while ((Get-Date) -lt $until) {{
    $s = Get-Service -Name $n -ErrorAction SilentlyContinue
    if (-not $s -or $s.Status -eq 'Stopped') {{ return $true }}
    Start-Sleep -Milliseconds 300
  }}
  return $false
}}
function Restore-All([string]$mode) {{
  if ($mode -ne 'skip') {{
    $list = @($stopped)
    [array]::Reverse($list)
    foreach ($n in $list) {{ try {{ Start-Service -Name $n -ErrorAction Stop }} catch {{}}; Start-Sleep -Milliseconds 600 }}
  }}
  $stopped.Clear()
}}
Save-State
while ($true) {{
  if (-not (Get-Process -Id $parentPid -ErrorAction SilentlyContinue)) {{
    Restore-All 'restore'
    Remove-Item $out -Force -ErrorAction SilentlyContinue
    break
  }}
  if (Test-Path $cmdFile) {{
    $raw = Get-Content $cmdFile -Raw
    Remove-Item $cmdFile -Force -ErrorAction SilentlyContinue
    $c = $null
    try {{ $c = $raw | ConvertFrom-Json }} catch {{}}
    if ($c) {{
      if ($c.cmd -eq 'stop') {{
        $failed = @()
        foreach ($n in @($c.services)) {{
          if (-not ($allowed -contains $n) -or (Test-Denied $n)) {{ continue }}
          $s = Get-Service -Name $n -ErrorAction SilentlyContinue
          if (-not $s -or $s.Status -ne 'Running') {{ continue }}
          if (Stop-One $n) {{ [void]$stopped.Add($n) }} else {{ $failed += $n }}
          Start-Sleep -Milliseconds 700
        }}
      }} elseif ($c.cmd -eq 'restore') {{
        Restore-All ([string]$c.mode)
      }} elseif ($c.cmd -eq 'quit') {{
        Restore-All 'restore'
        Remove-Item $out -Force -ErrorAction SilentlyContinue
        break
      }}
      $done = [string]$c.id
      Save-State
    }}
  }}
  Start-Sleep -Milliseconds 500
}}
"#,
        dir = ps_quote(&dir.to_string_lossy()),
        parent = parent,
        names = names,
        deny_exact = ps_list(SERVICE_DENY_EXACT),
        deny_prefix = ps_list(SERVICE_DENY_PREFIX),
    )
}

fn encode_powershell(script: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let bytes: Vec<u8> = script.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect();
    STANDARD.encode(bytes)
}

fn read_helper_json() -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(helper_json_path()).ok()?;
    serde_json::from_str(raw.trim_start_matches('\u{feff}').trim()).ok()
}

fn json_names(value: &serde_json::Value, key: &str) -> Vec<String> {
    match &value[key] {
        serde_json::Value::Array(items) => items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        serde_json::Value::String(one) => vec![one.clone()],
        _ => Vec::new(),
    }
}

/// What the running admin helper last reported.
#[derive(Debug, Clone, Default)]
struct HelperInfo {
    names: Vec<String>,
    done: String,
    stopped: Vec<String>,
    failed: Vec<String>,
}

/// The helper, when it has started and its process is still alive.
fn helper_info() -> Option<HelperInfo> {
    let json = read_helper_json()?;
    if json["armed"].as_bool() != Some(true) {
        return None;
    }
    let pid = json["pid"].as_u64()? as u32;
    if !pid_alive(pid) {
        return None;
    }
    Some(HelperInfo {
        names: json_names(&json, "names"),
        done: json["done"].as_str().unwrap_or("").to_string(),
        stopped: json_names(&json, "stopped"),
        failed: json_names(&json, "failed"),
    })
}

/// Every service any target (built-in or yours) or the update tweak can stop. The helper
/// gets this list when it starts and refuses anything else.
fn helper_names(cfg: &GameModeConfig) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for def in all_targets(cfg) {
        names.extend(def.services.iter().map(|s| (*s).to_string()));
    }
    names.extend(UPDATE_SERVICES.iter().map(|s| (*s).to_string()));
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup_by_key(|n| n.to_lowercase());
    names
}

/// True when the running helper already covers every service in `wanted`.
fn helper_covers(info: &HelperInfo, wanted: &[String]) -> bool {
    let have: HashSet<String> = info.names.iter().map(|n| n.to_lowercase()).collect();
    wanted.iter().all(|n| have.contains(&n.to_lowercase()))
}

/// Starts the admin helper. When DeckForge is not elevated Windows shows one prompt, so this
/// must only run while the user is at the desktop, never during a full-screen game.
fn arm_helper(cfg: &GameModeConfig) -> Result<(), String> {
    let wanted = helper_names(cfg);
    if let Some(info) = helper_info() {
        if helper_covers(&info, &wanted) {
            return Ok(());
        }
        // The service list grew since it started. Replace it, which restores nothing in
        // between because Game Mode is not running a stop at this point.
        if !is_active() {
            if helper_send(serde_json::json!({ "cmd": "quit" })).is_ok() {
                let started = Instant::now();
                while started.elapsed() < Duration::from_secs(8) && helper_info().is_some() {
                    std::thread::sleep(Duration::from_millis(250));
                }
            }
        } else {
            return Ok(());
        }
    }
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(helper_json_path());
    let _ = std::fs::remove_file(cmd_path());
    let encoded = encode_powershell(&helper_script(&wanted, std::process::id()));
    let mut command = Command::new("powershell.exe");
    command.creation_flags(CREATE_NO_WINDOW);
    if is_admin() {
        command.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-EncodedCommand",
            &encoded,
        ]);
    } else {
        let launcher = format!(
            "try {{ Start-Process -FilePath 'powershell.exe' -Verb RunAs -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-EncodedCommand','{encoded}' -ErrorAction Stop }} catch {{ exit 3 }}"
        );
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &launcher,
        ]);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        if helper_info().is_some() {
            return Ok(());
        }
        if let Ok(Some(exit)) = child.try_wait() {
            if !exit.success() && helper_info().is_none() {
                return Err("The Windows permission prompt was declined, so services were left running.".to_string());
            }
        }
        if started.elapsed() > Duration::from_secs(90) {
            return Err("The admin helper did not start in time, so services were left running.".to_string());
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

static COMMAND_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Hands the helper one command and returns its id.
fn helper_send(mut command: serde_json::Value) -> Result<String, String> {
    let id = format!("{}-{}", now_ms(), COMMAND_COUNTER.fetch_add(1, Ordering::SeqCst));
    command["id"] = serde_json::Value::String(id.clone());
    let raw = serde_json::to_string(&command).map_err(|e| e.to_string())?;
    let temp = data_dir().join("game-mode-cmd.tmp");
    std::fs::write(&temp, raw).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, cmd_path()).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Waits until the helper reports it finished the command with this id.
fn helper_wait(id: &str, limit: Duration) -> Result<HelperInfo, String> {
    let started = Instant::now();
    loop {
        match helper_info() {
            Some(info) if info.done == id => return Ok(info),
            None => return Err("The admin helper stopped running.".to_string()),
            _ => {}
        }
        if started.elapsed() > limit {
            return Err("The admin helper did not report back in time.".to_string());
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

// ---------------------------------------------------------------------------
// Enable / disable
// ---------------------------------------------------------------------------

fn reopen_label(reopen: Reopen) -> &'static str {
    match reopen {
        Reopen::Exe => "exe",
        Reopen::Store(_) => "store",
        Reopen::No => "no",
    }
}

fn closed_record(def: &TargetDef, matched: &[&Proc]) -> ClosedApp {
    let main = matched
        .iter()
        .find(|p| def.procs.first().map(|first| *first == p.name).unwrap_or(false) && !p.cmd.iter().any(|a| a.contains("--type=")))
        .or_else(|| matched.iter().find(|p| !p.cmd.iter().any(|a| a.contains("--type="))))
        .or_else(|| matched.first());
    let exe = main.and_then(|p| p.exe.as_ref()).map(|e| e.to_string_lossy().to_string());
    let args = main
        .map(|p| p.cmd.iter().skip(1).take(8).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    ClosedApp {
        id: def.id.to_string(),
        label: def.label.to_string(),
        memory_mb: matched.iter().map(|p| p.mem).sum::<u64>() / MB,
        processes: matched.len(),
        reopen: reopen_label(def.reopen).to_string(),
        exe,
        args,
        store: match def.reopen {
            Reopen::Store(pattern) => Some(pattern.to_string()),
            _ => None,
        },
    }
}

fn run_wsl_shutdown() {
    let _ = Command::new("wsl.exe")
        .arg("--shutdown")
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

/// True while something is full-screen in front. A UAC prompt there would switch to the
/// secure desktop, which reset the display driver and froze the PC.
fn something_is_fullscreen() -> bool {
    foreground().map(|f| f.fullscreen).unwrap_or(false)
}

/// Starts Game Mode. Runs on its own thread because closing apps takes a while. Work goes
/// one app at a time at background priority so a game that is loading is not starved.
pub fn enable(
    app: &tauri::AppHandle,
    auto: bool,
    game: Option<(u32, String)>,
    mode_id: Option<&str>,
) -> String {
    if is_active() {
        return "Game Mode is already on.".to_string();
    }
    let Some(_busy) = claim_busy(app, "Starting Game Mode") else {
        return "Game Mode is busy, try again in a moment.".to_string();
    };
    let _background = BackgroundWork::begin();
    let started = Instant::now();
    let cfg = config();
    let mode = pick_mode(&cfg, mode_id).clone();
    let mut notes: Vec<String> = Vec::new();
    let before = snapshot();
    let ram_before = free_ram_mb();
    let protect = protected_pids(&before);

    let plan = ensure_bitsum_plan(&mut notes);

    let selected: Vec<&'static TargetDef> = all_targets(&cfg)
        .into_iter()
        .filter(|t| mode.targets.iter().any(|id| id == t.id))
        .collect();

    // Services go through the admin helper. It is only started here when the user is at the
    // desktop. An automatic start or a full-screen game never triggers the Windows prompt.
    let mut wanted: Vec<String> = selected
        .iter()
        .flat_map(|t| t.services.iter().map(|s| (*s).to_string()))
        .collect();
    if mode.tweaks.iter().any(|t| t == "pause_updates") {
        wanted.extend(UPDATE_SERVICES.iter().map(|s| (*s).to_string()));
    }
    wanted.sort_by_key(|n| n.to_lowercase());
    wanted.dedup_by_key(|n| n.to_lowercase());
    let states = service_states();
    let running: Vec<String> = wanted
        .into_iter()
        .filter(|name| states.get(&name.to_lowercase()).copied().unwrap_or(false))
        .collect();
    let mut stop_command: Option<String> = None;
    if !running.is_empty() {
        if cfg.elevate != "ask" {
            notes.push("Services were skipped because the admin helper is turned off.".to_string());
        } else {
            let already = helper_info().map(|info| helper_covers(&info, &running)).unwrap_or(false);
            let ready = if already {
                true
            } else if auto || something_is_fullscreen() {
                notes.push(
                    "Services were skipped: the admin helper is not running, and a Windows prompt over a full-screen game can freeze the display. Start the helper from the Game Mode tab."
                        .to_string(),
                );
                false
            } else {
                match arm_helper(&cfg) {
                    Ok(()) => true,
                    Err(message) => {
                        notes.push(message);
                        false
                    }
                }
            };
            if ready {
                match helper_send(serde_json::json!({ "cmd": "stop", "services": running })) {
                    Ok(id) => stop_command = Some(id),
                    Err(message) => notes.push(format!("Could not talk to the admin helper: {message}")),
                }
            }
        }
    }

    if let Some((pid, name)) = &game {
        GAME_PID.store(*pid, Ordering::SeqCst);
        notes.push(format!("Game detected: {name}."));
    }

    // Close apps one at a time, with a pause between them so the disk and CPU are not hit at once.
    let mut closed: Vec<ClosedApp> = Vec::new();
    let mut keepers: HashMap<&'static str, HashSet<u32>> = HashMap::new();
    let mut wsl = false;
    for def in &selected {
        let matched = matched_procs(def, &before, &protect);
        if matched.is_empty() {
            continue;
        }
        closed.push(closed_record(def, &matched));
        let pids: HashSet<u32> = matched.iter().map(|p| p.pid).collect();
        if def.graceful {
            post_close(&pids);
            wait_until_gone(&pids, Duration::from_secs(if def.force { 5 } else { 6 }));
            if def.force {
                for pid in &pids {
                    if pid_alive(*pid) {
                        terminate(*pid);
                    }
                }
            } else {
                keepers.insert(def.id, pids.clone());
            }
        } else {
            for pid in &pids {
                terminate(*pid);
            }
        }
        wsl |= def.wsl_shutdown;
        let pause = if def.wsl_shutdown { 1500 } else { 600 };
        std::thread::sleep(Duration::from_millis(pause));
    }
    // Apps that refused to close (an unsaved-changes prompt) stay open and are not reopened later.
    closed.retain(|app| match keepers.get(app.id.as_str()) {
        Some(pids) => {
            let still: usize = pids.iter().filter(|pid| pid_alive(**pid)).count();
            if still > 0 {
                notes.push(format!("{} is still open because it is waiting on you (unsaved changes?).", app.label));
            }
            still < pids.len()
        }
        None => true,
    });
    if wsl {
        run_wsl_shutdown();
        notes.push("Shut down the WSL VM.".to_string());
    }

    let mut stopped: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    let helper_used = stop_command.is_some();
    if let Some(id) = stop_command {
        match helper_wait(&id, Duration::from_secs(150)) {
            Ok(info) => {
                stopped = info.stopped;
                failed = info.failed;
            }
            Err(message) => notes.push(message),
        }
    }

    std::thread::sleep(Duration::from_millis(900));
    let ram_after = free_ram_mb();
    let processes_after = snapshot().len();
    let freed_mb: u64 = closed.iter().map(|c| c.memory_mb).sum();
    notes.push(format!(
        "Closed {} apps (about {} MB) and stopped {} services in {:.1}s.",
        closed.len(),
        freed_mb,
        stopped.len(),
        started.elapsed().as_secs_f32()
    ));

    let snapshot_state = {
        let mut state = state_slot().lock().unwrap_or_else(|e| e.into_inner());
        *state = GameModeStatus {
            active: true,
            busy: state.busy.clone(),
            mode: mode.name.clone(),
            started_at_ms: now_ms(),
            auto_started: auto,
            game: game.as_ref().map(|(_, name)| name.clone()),
            game_pid: game.as_ref().map(|(pid, _)| *pid).unwrap_or(0),
            closed,
            stopped_services: stopped,
            failed_services: failed,
            helper_used,
            ram_free_before_mb: ram_before,
            ram_free_after_mb: ram_after,
            processes_before: before.len(),
            processes_after,
            power_plan: plan,
            notes,
            last_ended_ms: state.last_ended_ms,
        };
        state.clone()
    };
    ACTIVE.store(true, Ordering::SeqCst);
    persist_state(&snapshot_state);
    format!(
        "{} on: {} apps closed, {} services stopped, free RAM {:.1} to {:.1} GB.",
        mode.name,
        snapshot_state.closed.len(),
        snapshot_state.stopped_services.len(),
        ram_before as f64 / 1024.0,
        ram_after as f64 / 1024.0
    )
}

fn spawn_detached(command: &mut Command) -> bool {
    command
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .is_ok()
}

fn reopen_app(app: &ClosedApp) -> bool {
    match app.reopen.as_str() {
        "exe" => {
            let Some(exe) = &app.exe else { return false };
            let path = PathBuf::from(exe);
            if !path.exists() {
                return false;
            }
            let mut command = Command::new(&path);
            command.args(&app.args);
            if let Some(dir) = path.parent() {
                command.current_dir(dir);
            }
            spawn_detached(&mut command)
        }
        "store" => {
            let Some(pattern) = &app.store else { return false };
            let script = format!(
                "$a = Get-StartApps | Where-Object {{ $_.Name -match {} }} | Select-Object -First 1; if ($a) {{ Start-Process ('shell:AppsFolder\\' + $a.AppID) }}",
                ps_quote(pattern)
            );
            Command::new("powershell.exe")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .is_ok()
        }
        _ => false,
    }
}

/// Ends Game Mode and puts back what the config says to put back.
pub fn disable(app: &tauri::AppHandle, auto: bool) -> String {
    if !is_active() {
        return "Game Mode is already off.".to_string();
    }
    let Some(_busy) = claim_busy(app, "Ending Game Mode") else {
        return "Game Mode is busy, try again in a moment.".to_string();
    };
    let _background = BackgroundWork::begin();
    let cfg = config();
    let current = status();
    let mut notes: Vec<String> = Vec::new();

    // A manual end while the game is still running must not trigger auto start again.
    if !auto && current.game_pid != 0 && pid_alive(current.game_pid) {
        SUPPRESS_PID.store(current.game_pid, Ordering::SeqCst);
    }

    if current.helper_used {
        let mode = if cfg.restore == "none" { "skip" } else { "restore" };
        match helper_send(serde_json::json!({ "cmd": "restore", "mode": mode })) {
            Ok(id) => {
                if let Err(message) = helper_wait(&id, Duration::from_secs(120)) {
                    notes.push(format!("{message} Services come back on their own when DeckForge restarts."));
                } else if cfg.restore == "none" {
                    notes.push("Services were left stopped (restore is off). They return after a restart.".to_string());
                } else {
                    notes.push(format!("Restarted {} services.", current.stopped_services.len()));
                }
            }
            Err(message) => notes.push(format!("Could not talk to the admin helper: {message}")),
        }
    }

    let mut reopened = 0usize;
    if cfg.restore == "all" {
        let running_now = snapshot();
        let protect = protected_pids(&running_now);
        let defs = all_targets(&cfg);
        for app_info in &current.closed {
            if app_info.reopen == "no" {
                continue;
            }
            // Skip anything the user already started again by hand.
            let already = defs
                .iter()
                .find(|t| t.id == app_info.id)
                .map(|def| !matched_procs(def, &running_now, &protect).is_empty())
                .unwrap_or(false);
            if already {
                continue;
            }
            if reopen_app(app_info) {
                reopened += 1;
                // Start one app at a time so they do not all load from disk together.
                std::thread::sleep(Duration::from_millis(2500));
            } else {
                notes.push(format!("{} could not be reopened, start it yourself.", app_info.label));
            }
        }
        notes.push(format!("Reopened {reopened} apps."));
    } else if cfg.restore == "settings" {
        notes.push("Apps were left closed (restore is set to settings only).".to_string());
    }

    GAME_PID.store(0, Ordering::SeqCst);
    let snapshot_state = {
        let mut state = state_slot().lock().unwrap_or_else(|e| e.into_inner());
        let busy = state.busy.clone();
        *state = GameModeStatus {
            busy,
            last_ended_ms: now_ms(),
            notes,
            power_plan: current.power_plan.clone(),
            ..GameModeStatus::default()
        };
        state.clone()
    };
    ACTIVE.store(false, Ordering::SeqCst);
    persist_state(&snapshot_state);
    format!("Game Mode off: {reopened} apps reopened.")
}

pub fn toggle(app: &tauri::AppHandle, mode: Option<&str>) -> String {
    if is_active() {
        disable(app, false)
    } else {
        enable(app, false, None, mode)
    }
}

/// Starts the admin helper from the UI, while the user is at the desktop. The one Windows
/// prompt for the session appears here instead of in the middle of a game.
pub fn arm(app: &tauri::AppHandle) -> String {
    if something_is_fullscreen() {
        return "Leave the full-screen app first. The Windows prompt can freeze the display over a game.".to_string();
    }
    let cfg = config();
    if cfg.elevate != "ask" {
        return "The admin helper is turned off in the Game Mode settings.".to_string();
    }
    let _ = app.emit("game-mode-message", "Waiting for the Windows permission prompt...");
    match arm_helper(&cfg) {
        Ok(()) => "Admin helper is running. Game Mode can stop services without another prompt.".to_string(),
        Err(message) => message,
    }
}

// ---------------------------------------------------------------------------
// Scan for the UI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: String,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    pub id: String,
    pub label: String,
    pub group: String,
    pub note: String,
    pub default_on: bool,
    pub running: bool,
    pub process_count: usize,
    pub memory_mb: u64,
    pub services: Vec<ServiceInfo>,
    pub needs_admin: bool,
    pub reopens: bool,
    pub force: bool,
    /// Added by the user, so it can be edited and removed.
    pub custom: bool,
    pub processes: Vec<String>,
}

/// A running program the user can add as their own target.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningApp {
    /// Exe name without ".exe".
    pub name: String,
    pub count: usize,
    pub memory_mb: u64,
    pub path: String,
    /// Label of a target that already closes this program, if any.
    pub taken_by: Option<String>,
}

/// A running Windows service the user can add as their own target.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServicePick {
    pub name: String,
    pub display: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperStatus {
    /// The elevated helper is running, so services can be stopped without a prompt.
    pub armed: bool,
    /// It already knows every service the current config can stop.
    pub covers: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TweakInfo {
    pub id: String,
    pub label: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub targets: Vec<TargetInfo>,
    pub tweaks: Vec<TweakInfo>,
    pub ram_total_mb: u64,
    pub ram_free_mb: u64,
    pub process_count: usize,
    pub power_plan: String,
    pub bitsum_found: bool,
    pub is_admin: bool,
    pub protected: Vec<String>,
    pub processes: Vec<RunningApp>,
    pub services: Vec<ServicePick>,
    pub helper: HelperStatus,
}

pub fn scan() -> ScanReport {
    let cfg = config();
    let procs = snapshot();
    let protect = protected_pids(&procs);
    let service_rows = service_list();
    let states: HashMap<String, bool> = service_rows
        .iter()
        .map(|(name, _, running)| (name.to_lowercase(), *running))
        .collect();
    let mut sys = System::new();
    sys.refresh_memory();
    let defs = all_targets(&cfg);
    let custom_ids: HashSet<&str> = cfg.custom_targets.iter().map(|c| c.id.as_str()).collect();
    let targets = defs
        .iter()
        .map(|def| {
            let matched = matched_procs(def, &procs, &protect);
            let services: Vec<ServiceInfo> = def
                .services
                .iter()
                .map(|name| ServiceInfo {
                    name: (*name).to_string(),
                    running: states.get(&name.to_lowercase()).copied().unwrap_or(false),
                })
                .collect();
            let running = !matched.is_empty() || services.iter().any(|s| s.running);
            TargetInfo {
                id: def.id.to_string(),
                label: def.label.to_string(),
                group: def.group.to_string(),
                note: def.note.to_string(),
                default_on: def.default_on,
                running,
                process_count: matched.len(),
                memory_mb: matched.iter().map(|p| p.mem).sum::<u64>() / MB,
                needs_admin: !def.services.is_empty(),
                services,
                reopens: def.reopen != Reopen::No,
                force: def.force,
                custom: custom_ids.contains(def.id),
                processes: def.procs.iter().map(|p| (*p).to_string()).collect(),
            }
        })
        .collect();

    // Programs the user could add. Grouped by exe name, biggest first.
    let windows_dir = std::env::var("windir").map(|w| format!("{}\\", w.to_lowercase())).unwrap_or_default();
    let mut grouped: HashMap<String, RunningApp> = HashMap::new();
    for p in &procs {
        if p.pid <= 4 || protect.contains(&p.pid) || never_kill(p) || p.exe_lower.is_empty() {
            continue;
        }
        if !windows_dir.is_empty() && p.exe_lower.starts_with(&windows_dir) {
            continue;
        }
        let entry = grouped.entry(p.name.clone()).or_insert_with(|| RunningApp {
            name: p.name.clone(),
            count: 0,
            memory_mb: 0,
            path: p.exe_lower.clone(),
            taken_by: None,
        });
        entry.count += 1;
        entry.memory_mb += p.mem / MB;
        if entry.taken_by.is_none() {
            entry.taken_by = defs.iter().find(|def| has_process_rules(def) && matches(def, p)).map(|def| def.label.to_string());
        }
    }
    let mut processes: Vec<RunningApp> = grouped.into_values().collect();
    processes.sort_by(|a, b| b.memory_mb.cmp(&a.memory_mb).then_with(|| a.name.cmp(&b.name)));
    processes.truncate(150);

    let mut services: Vec<ServicePick> = service_rows
        .iter()
        .filter(|(name, _, running)| *running && !service_denied(name) && clean_service_name(name).is_some())
        .map(|(name, display, _)| ServicePick { name: name.clone(), display: display.clone() })
        .collect();
    services.sort_by_key(|s| s.display.to_lowercase());

    let helper = {
        let info = helper_info();
        let wanted = helper_names(&cfg);
        HelperStatus {
            armed: info.is_some(),
            covers: info.as_ref().map(|i| helper_covers(i, &wanted)).unwrap_or(false),
        }
    };

    ScanReport {
        processes,
        services,
        helper,
        targets,
        tweaks: TWEAKS
            .iter()
            .map(|t| TweakInfo {
                id: t.id.to_string(),
                label: t.label.to_string(),
                note: t.note.to_string(),
            })
            .collect(),
        ram_total_mb: sys.total_memory() / MB,
        ram_free_mb: sys.available_memory() / MB,
        process_count: procs.len(),
        power_plan: active_plan().map(|(_, name)| name).unwrap_or_default(),
        bitsum_found: bitsum_plan().is_some(),
        is_admin: is_admin(),
        protected: PROTECTED_NOTES.iter().map(|s| (*s).to_string()).collect(),
    }
}

// ---------------------------------------------------------------------------
// Game detection and the background watcher
// ---------------------------------------------------------------------------

struct Foreground {
    pid: u32,
    path: String,
    fullscreen: bool,
}

fn process_path(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 520];
        let mut size = buf.len() as u32;
        let queried = QueryFullProcessImageNameW(handle, Default::default(), PWSTR(buf.as_mut_ptr()), &mut size);
        let _ = CloseHandle(handle);
        queried.ok()?;
        Some(String::from_utf16_lossy(&buf[..size as usize]))
    }
}

fn foreground() -> Option<Foreground> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).ok()?;
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let m = info.rcMonitor;
        let fullscreen = rect.left <= m.left && rect.top <= m.top && rect.right >= m.right && rect.bottom >= m.bottom;
        Some(Foreground {
            pid,
            path: process_path(pid)?,
            fullscreen,
        })
    }
}

fn exe_stem(path: &str) -> String {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path).to_lowercase();
    file.strip_suffix(".exe").unwrap_or(&file).to_string()
}

/// A full-screen window that is either exclusive-mode Direct3D or lives in a game install folder.
fn detect_game() -> Option<(u32, String)> {
    let fg = foreground()?;
    if !fg.fullscreen {
        return None;
    }
    let stem = exe_stem(&fg.path);
    if NOT_GAMES.contains(&stem.as_str()) {
        return None;
    }
    let lower = fg.path.to_lowercase();
    let in_game_folder = GAME_PATHS.iter().any(|frag| lower.contains(frag));
    let exclusive = unsafe { SHQueryUserNotificationState().map(|s| s == QUNS_RUNNING_D3D_FULL_SCREEN).unwrap_or(false) };
    if in_game_folder || exclusive {
        Some((fg.pid, stem))
    } else {
        None
    }
}

/// True when the mode that is running has this tweak on.
fn running_mode_has(cfg: &GameModeConfig, mode_name: &str, tweak: &str) -> bool {
    cfg.modes
        .iter()
        .find(|m| m.name == mode_name)
        .unwrap_or_else(|| pick_mode(cfg, None))
        .tweaks
        .iter()
        .any(|t| t == tweak)
}

/// Polls for a full-screen game. Starts Game Mode for it when auto detect is on (after the
/// settle delay, so closing apps does not compete with the game loading), raises the game's
/// priority, and ends Game Mode when it exits.
pub fn spawn_watcher(app: tauri::AppHandle, nudge: Arc<dyn Fn() + Send + Sync>) {
    let _ = state_slot();

    // Optional: start the admin helper once, a little after launch, while the user is at the desktop.
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(12));
        let cfg = config();
        if cfg.arm_on_launch && cfg.elevate == "ask" && helper_info().is_none() && !something_is_fullscreen() {
            let _ = arm_helper(&cfg);
        }
    });

    std::thread::spawn(move || {
        let mut pending: Option<(u32, Instant)> = None;
        let mut gone_since: Option<Instant> = None;
        let mut boosted: HashSet<u32> = HashSet::new();
        loop {
            std::thread::sleep(Duration::from_secs(2));
            if BUSY.load(Ordering::SeqCst) {
                continue;
            }
            let cfg = config();
            let detected = detect_game();
            if !is_active() {
                match &detected {
                    Some((pid, _)) if cfg.auto_detect && SUPPRESS_PID.load(Ordering::SeqCst) != *pid => {
                        let since = match pending {
                            Some((seen_pid, since)) if seen_pid == *pid => since,
                            _ => {
                                let now = Instant::now();
                                pending = Some((*pid, now));
                                now
                            }
                        };
                        let wait = Duration::from_secs(u64::from(cfg.settle_seconds).max(6));
                        if since.elapsed() >= wait {
                            pending = None;
                            let _ = enable(&app, true, detected.clone(), None);
                            nudge();
                        }
                    }
                    _ => pending = None,
                }
                gone_since = None;
                continue;
            }
            pending = None;

            if let Some((pid, name)) = &detected {
                let mode_name = status().mode;
                if running_mode_has(&cfg, &mode_name, "priority") && boosted.insert(*pid) {
                    let ok = raise_priority(*pid);
                    with_state(|s| {
                        s.notes.push(if ok {
                            format!("{name} runs at High priority.")
                        } else {
                            format!("Could not raise the priority of {name}.")
                        });
                        if s.game.is_none() {
                            s.game = Some(name.clone());
                            s.game_pid = *pid;
                        }
                    });
                    GAME_PID.store(*pid, Ordering::SeqCst);
                    publish(&app);
                }
            }

            let state = status();
            if state.auto_started && state.game_pid != 0 {
                if pid_alive(state.game_pid) {
                    gone_since = None;
                } else {
                    let since = *gone_since.get_or_insert_with(Instant::now);
                    if since.elapsed() >= Duration::from_secs(15) {
                        gone_since = None;
                        boosted.clear();
                        let _ = disable(&app, true);
                        nudge();
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(name: &str, path: &str, cmd: &[&str]) -> Proc {
        Proc {
            pid: 100,
            parent: None,
            name: name.to_string(),
            exe: Some(PathBuf::from(path)),
            exe_lower: path.to_lowercase(),
            cmd: cmd.iter().map(|s| s.to_string()).collect(),
            mem: 10 * MB,
        }
    }

    fn def(id: &str) -> &'static TargetDef {
        TARGETS.iter().find(|t| t.id == id).expect("target exists")
    }

    #[test]
    fn target_ids_are_unique_and_services_are_plain_names() {
        let mut seen = HashSet::new();
        for t in TARGETS {
            assert!(seen.insert(t.id), "duplicate target id {}", t.id);
            assert!(!t.label.is_empty());
            for name in t.services {
                assert!(!name.contains('\'') && !name.contains('"'), "unsafe service name {name}");
            }
        }
    }

    fn custom(id: &str, processes: &[&str], services: &[&str]) -> CustomTarget {
        CustomTarget {
            id: id.to_string(),
            label: id.to_string(),
            processes: processes.iter().map(|s| s.to_string()).collect(),
            services: services.iter().map(|s| s.to_string()).collect(),
            ..CustomTarget::default()
        }
    }

    #[test]
    fn default_config_has_one_mode_and_safe_settings() {
        let cfg = GameModeConfig::default();
        assert_eq!(cfg.modes.len(), 1);
        assert_eq!(cfg.active_mode, cfg.modes[0].id);
        let on = &cfg.modes[0].targets;
        for id in ["zen", "cursor", "docker_wsl", "node", "qbit", "mongo", "warp"] {
            assert!(on.iter().any(|t| t == id), "{id} should default on");
        }
        for id in ["spotify", "discord", "steam", "dlmm", "riotclient"] {
            assert!(!on.iter().any(|t| t == id), "{id} should default off");
        }
        assert!(cfg.auto_detect);
        assert!(!cfg.arm_on_launch);
        assert_eq!(cfg.restore, "all");
        assert_eq!(cfg.elevate, "ask");
        assert!(cfg.settle_seconds >= 10, "an automatic start must wait for the game to settle");
    }

    #[test]
    fn sanitize_drops_unknown_ids_and_bad_values() {
        let cfg = sanitize(GameModeConfig {
            modes: vec![ModeConfig {
                id: "Comp!".into(),
                name: "  Competitive  ".into(),
                targets: vec!["zen".into(), "zen".into(), "format_disk".into()],
                tweaks: vec!["priority".into(), "nope".into()],
            }],
            active_mode: "missing".into(),
            settle_seconds: 9999,
            restore: "weird".into(),
            elevate: "root".into(),
            ..GameModeConfig::default()
        });
        assert_eq!(cfg.modes[0].id, "comp");
        assert_eq!(cfg.modes[0].name, "Competitive");
        assert_eq!(cfg.modes[0].targets, vec!["zen".to_string()]);
        assert_eq!(cfg.modes[0].tweaks, vec!["priority".to_string()]);
        assert_eq!(cfg.active_mode, "comp");
        assert_eq!(cfg.settle_seconds, 120);
        assert_eq!(cfg.restore, "all");
        assert_eq!(cfg.elevate, "ask");
    }

    #[test]
    fn old_config_files_become_the_first_mode() {
        let raw = r#"{"targets":["zen","bogus"],"tweaks":["priority"],"autoDetect":false,"restore":"all","elevate":"uac"}"#;
        let cfg = sanitize(parse_config(raw).expect("old file parses"));
        assert_eq!(cfg.modes.len(), 1);
        assert_eq!(cfg.modes[0].targets, vec!["zen".to_string()]);
        assert_eq!(cfg.modes[0].tweaks, vec!["priority".to_string()]);
        assert!(!cfg.auto_detect);
        assert_eq!(cfg.elevate, "ask");
        assert!(cfg.targets.is_none() && cfg.tweaks.is_none());
        // A file with modes keeps them.
        let kept = sanitize(parse_config(r#"{"modes":[{"id":"a","name":"A","targets":["node"],"tweaks":[]}]}"#).unwrap());
        assert_eq!(kept.modes[0].targets, vec!["node".to_string()]);
    }

    #[test]
    fn custom_targets_cannot_reach_protected_programs_or_services() {
        let cfg = sanitize(GameModeConfig {
            custom_targets: vec![
                custom("mine", &["Obsidian.exe", "explorer", "vgc", "bad;name"], &["MyService", "Audiosrv", "vgk", "nvdisplay.containerlocalsystem", "x'y"]),
                custom("empty", &["explorer"], &["WinDefend"]),
                custom("zen", &["zen"], &[]),
                custom("dup", &["a"], &[]),
                custom("dup", &["b"], &[]),
            ],
            ..GameModeConfig::default()
        });
        let ids: Vec<&str> = cfg.custom_targets.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["mine", "dup"], "built-in ids, duplicates and targets with nothing left are dropped");
        assert_eq!(cfg.custom_targets[0].processes, vec!["obsidian".to_string()]);
        assert_eq!(cfg.custom_targets[0].services, vec!["MyService".to_string()]);
        assert!(service_denied("EasyAntiCheat_EOS") && service_denied("NVDisplay.ContainerLocalSystem") && service_denied("WinDefend"));
        assert!(!service_denied("MyService"));
    }

    #[test]
    fn modes_only_reference_known_targets_and_a_custom_target_matches_by_name() {
        let cfg = sanitize(GameModeConfig {
            custom_targets: vec![custom("notes", &["obsidian"], &[])],
            modes: vec![
                ModeConfig { id: "a".into(), name: "A".into(), targets: vec!["notes".into(), "ghost".into()], tweaks: vec![] },
                ModeConfig { id: "a".into(), name: "dup".into(), targets: vec![], tweaks: vec![] },
                ModeConfig { id: "b".into(), name: "B".into(), targets: vec![], tweaks: vec!["priority".into()] },
            ],
            active_mode: "b".into(),
            ..GameModeConfig::default()
        });
        assert_eq!(cfg.modes.len(), 2);
        assert_eq!(cfg.modes[0].targets, vec!["notes".to_string()]);
        assert_eq!(pick_mode(&cfg, Some("a")).name, "A");
        assert_eq!(pick_mode(&cfg, Some("nope")).name, "B");
        assert_eq!(pick_mode(&cfg, None).name, "B");

        let defs = all_targets(&cfg);
        assert_eq!(defs.len(), TARGETS.len() + 1);
        let notes = defs.iter().find(|d| d.id == "notes").expect("custom target is listed");
        assert!(matches(notes, &proc("obsidian", "C:\\Apps\\Obsidian\\Obsidian.exe", &[])));
        assert!(!matches(notes, &proc("zen", "C:\\Apps\\zen.exe", &[])));
        // The same config gives the same leaked definition instead of leaking again.
        assert!(std::ptr::eq(*notes, *all_targets(&cfg).iter().find(|d| d.id == "notes").unwrap()));
    }

    #[test]
    fn claude_matches_the_store_app_but_not_its_service_or_the_cli() {
        let claude = def("ai_claude");
        assert!(matches(claude, &proc("claude", "C:\\Program Files\\WindowsApps\\Claude_4.1_x64__abc\\app\\Claude.exe", &[])));
        assert!(!matches(claude, &proc("cowork-svc", "C:\\Program Files\\WindowsApps\\Claude_4.1_x64__abc\\app\\cowork-svc.exe", &[])));
        assert!(!matches(claude, &proc("claude", "C:\\Users\\me\\.local\\bin\\claude.exe", &[])));
    }

    #[test]
    fn codex_service_process_is_skipped() {
        let codex = def("ai_codex");
        assert!(matches(codex, &proc("codex", "C:\\Program Files\\WindowsApps\\OpenAI.Codex_26.1_x64__abc\\app\\Codex.exe", &[])));
        assert!(!matches(
            codex,
            &proc("codex-windows-sandbox-service", "C:\\Program Files\\WindowsApps\\OpenAI.Codex_26.1_x64__abc\\bin\\codex-windows-sandbox-service.exe", &[])
        ));
    }

    #[test]
    fn language_server_only_matches_inside_antigravity() {
        let antigravity = def("antigravity");
        assert!(matches(antigravity, &proc("language_server", "C:\\Users\\me\\AppData\\Local\\Programs\\Antigravity\\resources\\bin\\language_server.exe", &[])));
        assert!(!matches(antigravity, &proc("language_server", "C:\\Tools\\other\\language_server.exe", &[])));
    }

    #[test]
    fn node_dev_stack_is_protected() {
        let node = def("node");
        let vite = proc("node", "C:\\Program Files\\nodejs\\node.exe", &["node", "vite.js"]);
        let tauri = proc("node", "C:\\Program Files\\nodejs\\node.exe", &["node", "tauri", "dev"]);
        let other = proc("node", "C:\\Program Files\\nodejs\\node.exe", &["node", "server.js"]);
        assert!(matches(node, &vite) && is_dev_stack(&vite));
        assert!(is_dev_stack(&tauri));
        assert!(!is_dev_stack(&other));
        let procs = vec![vite, tauri, other];
        assert_eq!(matched_procs(node, &procs, &HashSet::new()).len(), 1);
    }

    #[test]
    fn anti_cheat_and_system_processes_are_never_matched() {
        assert!(never_kill(&proc("vgc", "C:\\Program Files\\Riot Vanguard\\vgc.exe", &[])));
        assert!(never_kill(&proc("explorer", "C:\\Windows\\explorer.exe", &[])));
        assert!(never_kill(&proc("anything", "C:\\Windows\\System32\\anything.exe", &[])));
        assert!(never_kill(&proc("easyanticheat_eos", "C:\\Games\\EasyAntiCheat\\x.exe", &[])));
        assert!(!never_kill(&proc("zen", "C:\\Program Files\\Zen Browser\\zen.exe", &[])));
        // The Riot launcher target must not be able to reach Vanguard.
        let riot = def("riotclient");
        assert!(!matches(riot, &proc("vgc", "C:\\Program Files\\Riot Vanguard\\vgc.exe", &[])));
        assert!(!riot.procs.iter().any(|p| p.starts_with("vg")));
    }

    #[test]
    fn docker_prefix_matches_backend_processes() {
        let docker = def("docker_wsl");
        assert!(matches(docker, &proc("docker desktop", "C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe", &[])));
        assert!(matches(docker, &proc("com.docker.backend", "C:\\Program Files\\Docker\\Docker\\resources\\com.docker.backend.exe", &[])));
        assert!(!matches(docker, &proc("dockerd", "C:\\x\\dockerd.exe", &[])));
        assert!(docker.wsl_shutdown);
    }

    #[test]
    fn parses_power_plans() {
        let line = "Power Scheme GUID: 29020666-3a1b-4b66-9d4c-b33d99302449  (Bitsum Highest Performance) *";
        assert_eq!(guid_in(line).as_deref(), Some("29020666-3a1b-4b66-9d4c-b33d99302449"));
        assert_eq!(plan_label(line), "Bitsum Highest Performance");
        assert_eq!(guid_in("no guid here"), None);
    }

    #[test]
    fn helper_script_is_resident_limited_to_its_list_and_restores_when_deckforge_exits() {
        let services = vec!["MongoDB".to_string(), "Bonjour Service".to_string()];
        let script = helper_script(&services, 4242);
        assert!(script.contains("$allowed = @('MongoDB','Bonjour Service')"));
        assert!(script.contains("$parentPid = 4242"));
        assert!(script.contains("game-mode-cmd.json"));
        assert!(script.contains("Restore-All 'restore'"), "restores when the parent dies");
        assert!(script.contains("'vgc'") && script.contains("'audiosrv'") && script.contains("'windefend'"));
        assert!(script.contains("BelowNormal"), "the helper must not compete with the game");
        assert!(!script.contains("Set-ExecutionPolicy"));
        assert!(!script.contains("{{") && !script.contains("}}"), "format escapes were left in the script");
        assert!(!encode_powershell(&script).is_empty());
        assert_eq!(ps_quote("a'b"), "'a''b'");
    }

    #[test]
    fn helper_script_is_valid_powershell() {
        let path = std::env::temp_dir().join("deckforge-helper-check.ps1");
        std::fs::write(&path, helper_script(&["MongoDB".to_string()], 1)).unwrap();
        let check = format!(
            "$e=$null; [void][System.Management.Automation.Language.Parser]::ParseFile('{}',[ref]$null,[ref]$e); if ($e.Count) {{ $e | ForEach-Object {{ $_.Message }}; exit 1 }}",
            path.display()
        );
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &check])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        let _ = std::fs::remove_file(&path);
        if let Ok(output) = output {
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
        }
    }

    #[test]
    fn helper_covers_checks_names_case_insensitively() {
        let info = HelperInfo { names: vec!["MongoDB".into(), "Spooler".into()], ..HelperInfo::default() };
        assert!(helper_covers(&info, &["mongodb".to_string()]));
        assert!(!helper_covers(&info, &["mongodb".to_string(), "syncthing".to_string()]));
    }

    #[test]
    fn every_builtin_service_passes_the_deny_list() {
        for t in TARGETS {
            for name in t.services {
                assert!(!service_denied(name), "{} lists the protected service {name}", t.id);
            }
        }
        for name in UPDATE_SERVICES {
            assert!(!service_denied(name), "update service {name} is denied");
        }
    }

    #[test]
    fn game_paths_cover_the_installed_games() {
        let hit = |p: &str| GAME_PATHS.iter().any(|f| p.to_lowercase().contains(f));
        assert!(hit("D:\\SteamLibrary\\steamapps\\common\\Deadlock\\game\\bin\\win64\\project8.exe"));
        assert!(hit("G:\\Riot Games\\VALORANT\\live\\ShooterGame\\Binaries\\Win64\\VALORANT-Win64-Shipping.exe"));
        assert!(!hit("C:\\Program Files\\Zen Browser\\zen.exe"));
        assert!(NOT_GAMES.contains(&"zen"));
    }
}
