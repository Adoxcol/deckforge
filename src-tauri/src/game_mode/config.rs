use super::*;

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
    /// GUID of an installed power plan to switch to while the mode runs. Empty leaves the
    /// plan the PC is already using alone.
    pub power_plan: String,
    /// Programs to start when the mode turns on, such as a clip recorder or a network booster.
    #[serde(default)]
    pub start_apps: Vec<StartApp>,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default".to_string(),
            // Nothing is closed until the user picks it from what is running on their PC.
            targets: Vec::new(),
            tweaks: vec!["priority".to_string()],
            power_plan: String::new(),
            start_apps: Vec::new(),
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
pub(super) const SERVICE_DENY_EXACT: &[&str] = &[
    "rpcss", "rpceptmapper", "dcomlaunch", "eventlog", "lsm", "samss", "wscsvc", "sense",
    "securityhealthservice", "mpssvc", "bfe", "winmgmt", "plugplay", "power", "profsvc",
    "cryptsvc", "trustedinstaller", "schedule", "dhcp", "dnscache", "nsi", "netprofm", "nlasvc",
    "themes", "usermanager", "gpsvc", "sens", "shellhwdetection", "lanmanserver",
    "lanmanworkstation", "keyiso", "hidserv", "uxsms", "sessionenv", "termservice", "wudfsvc",
    "audiosrv", "audioendpointbuilder", "vgc", "vgk", "beservice", "windefend", "wdnissvc",
    "wlansvc", "wcmsvc", "tcpip", "netlogon", "browser", "msiserver",
];
pub(super) const SERVICE_DENY_PREFIX: &[&str] = &["nv", "easyanticheat", "faceit", "vgc", "vgk", "wd", "mpssvc", "rpc"];

pub(super) fn service_denied(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    SERVICE_DENY_EXACT.contains(&lower.as_str()) || SERVICE_DENY_PREFIX.iter().any(|p| lower.starts_with(p))
}

pub(super) fn clean_process_name(raw: &str) -> Option<String> {
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

pub(super) fn clean_service_name(raw: &str) -> Option<String> {
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

pub(super) fn clean_id(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(48)
        .collect()
}

pub(super) fn clean_label(raw: &str, fallback: &str) -> String {
    let label: String = raw.trim().chars().filter(|c| !c.is_control()).take(40).collect();
    if label.is_empty() {
        fallback.to_string()
    } else {
        label
    }
}

/// Moves the old single-mode fields into a mode, drops anything unknown, and keeps the
/// user's own targets inside the safety rules.
pub(super) fn sanitize(mut cfg: GameModeConfig) -> GameModeConfig {
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
        mode.power_plan = clean_guid(&mode.power_plan);
        let mut seen = HashSet::new();
        mode.targets.retain(|id| known.contains(id) && seen.insert(id.clone()));
        let mut seen = HashSet::new();
        mode.tweaks.retain(|id| known_tweaks.contains(id.as_str()) && seen.insert(id.clone()));
        mode.start_apps = clean_start_apps(std::mem::take(&mut mode.start_apps));
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
pub(super) fn pick_mode<'a>(cfg: &'a GameModeConfig, wanted: Option<&str>) -> &'a ModeConfig {
    wanted
        .and_then(|id| cfg.modes.iter().find(|m| m.id == id))
        .or_else(|| cfg.modes.iter().find(|m| m.id == cfg.active_mode))
        .unwrap_or(&cfg.modes[0])
}

// Custom targets are turned into `TargetDef`s once and kept, so the matching code
// below works the same for built-in and user-added targets.
pub(super) static CUSTOM_DEFS: OnceLock<Mutex<HashMap<String, &'static TargetDef>>> = OnceLock::new();

pub(super) fn leak_str(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

pub(super) fn leak_list(items: &[String]) -> &'static [&'static str] {
    let list: Vec<&'static str> = items.iter().map(|s| leak_str(s)).collect();
    Box::leak(list.into_boxed_slice())
}

pub(super) fn custom_def(custom: &CustomTarget) -> &'static TargetDef {
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
        protect_dev: true,
        ..BASE
    }));
    cache.insert(key, def);
    def
}

/// Built-in presets followed by the user's own targets.
pub(super) fn all_targets(cfg: &GameModeConfig) -> Vec<&'static TargetDef> {
    TARGETS.iter().chain(cfg.custom_targets.iter().map(custom_def)).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ClosedApp {
    pub id: String,
    pub label: String,
    pub memory_mb: u64,
    pub processes: usize,
    /// "exe" or "no".
    pub reopen: String,
    pub exe: Option<String>,
    pub args: Vec<String>,
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
    /// Programs this run started, closed again at the end when the mode says so.
    #[serde(default)]
    pub started: Vec<StartedApp>,
    pub stopped_services: Vec<String>,
    pub failed_services: Vec<String>,
    pub helper_used: bool,
    pub ram_free_before_mb: u64,
    pub ram_free_after_mb: u64,
    pub processes_before: usize,
    pub processes_after: usize,
    /// Name of the power plan in use while Game Mode ran.
    pub power_plan: String,
    /// GUID of the plan to put back, and of the plan Game Mode switched to. Both are empty
    /// when Game Mode left the plan alone.
    pub previous_power_plan: String,
    pub applied_power_plan: String,
    pub notes: Vec<String>,
    pub last_ended_ms: u64,
}

pub(super) static CONFIG: OnceLock<Mutex<GameModeConfig>> = OnceLock::new();
pub(super) static STATE: OnceLock<Mutex<GameModeStatus>> = OnceLock::new();
pub(super) static ACTIVE: AtomicBool = AtomicBool::new(false);
pub(super) static BUSY: AtomicBool = AtomicBool::new(false);
pub(super) static GAME_PID: AtomicU32 = AtomicU32::new(0);
pub(super) static SUPPRESS_PID: AtomicU32 = AtomicU32::new(0);

pub(super) fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("DeckForge")
}

pub(super) fn config_path() -> PathBuf {
    data_dir().join("game-mode.json")
}

pub(super) fn state_path() -> PathBuf {
    data_dir().join("game-mode-state.json")
}

pub(super) fn helper_json_path() -> PathBuf {
    data_dir().join("game-mode-helper.json")
}

pub(super) fn cmd_path() -> PathBuf {
    data_dir().join("game-mode-cmd.json")
}

/// Reads a config file. A file from before modes existed has no `modes` key, which must
/// stay empty (not the default mode) so `sanitize` can move its old lists into a mode.
pub(super) fn parse_config(raw: &str) -> Option<GameModeConfig> {
    let mut value: serde_json::Value = serde_json::from_str(raw.trim_start_matches('\u{feff}').trim()).ok()?;
    if let Some(object) = value.as_object_mut() {
        if !object.contains_key("modes") {
            object.insert("modes".to_string(), serde_json::json!([]));
        }
    }
    serde_json::from_value(value).ok()
}

pub(super) fn config_slot() -> &'static Mutex<GameModeConfig> {
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
    // Serialize saves with readers and other saves, then replace the file only
    // after its complete contents have reached disk.
    let mut current = config_slot().lock().unwrap_or_else(|e| e.into_inner());
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    crate::db::write_atomic(&config_path(), &raw)?;
    *current = cfg.clone();
    Ok(cfg)
}

pub(super) fn state_slot() -> &'static Mutex<GameModeStatus> {
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

pub(super) fn persist_state(state: &GameModeStatus) {
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

pub(super) fn publish(app: &tauri::AppHandle) {
    let _ = app.emit("game-mode", status());
}

pub(super) fn with_state(update: impl FnOnce(&mut GameModeStatus)) {
    let mut state = state_slot().lock().unwrap_or_else(|e| e.into_inner());
    update(&mut state);
}

pub(super) struct BusyGuard<'a> {
    pub(super) app: &'a tauri::AppHandle,
}

pub(super) fn claim_busy<'a>(app: &'a tauri::AppHandle, label: &str) -> Option<BusyGuard<'a>> {
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

