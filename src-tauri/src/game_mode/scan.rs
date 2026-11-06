use super::*;

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

/// A running program the user can pick for a mode.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningApp {
    /// Exe name without ".exe".
    pub name: String,
    pub count: usize,
    pub memory_mb: u64,
    pub path: String,
    /// One of its processes owns a normal visible window, so it is an open app and not a
    /// background process.
    pub has_window: bool,
    /// Label of a target that already closes this program, if any.
    pub taken_by: Option<String>,
    pub taken_by_id: Option<String>,
}

/// A power plan installed on this PC.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerPlanInfo {
    pub guid: String,
    pub name: String,
    pub active: bool,
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
    /// Name of the plan Windows is using right now.
    pub power_plan: String,
    /// Every plan installed on this PC.
    pub power_plans: Vec<PowerPlanInfo>,
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

    // Programs the user could pick. Grouped by exe name, biggest first.
    let windowed = windowed_pids();
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
            has_window: false,
            taken_by: None,
            taken_by_id: None,
        });
        entry.count += 1;
        entry.memory_mb += p.mem / MB;
        entry.has_window |= windowed.contains(&p.pid);
        if entry.taken_by.is_none() {
            if let Some(def) = defs.iter().find(|def| has_process_rules(def) && matches(def, p)) {
                entry.taken_by = Some(def.label.to_string());
                entry.taken_by_id = Some(def.id.to_string());
            }
        }
    }
    let mut processes: Vec<RunningApp> = grouped.into_values().collect();
    processes.sort_by(|a, b| b.memory_mb.cmp(&a.memory_mb).then_with(|| a.name.cmp(&b.name)));
    // Every open app stays in the list. Only the long tail of tiny background processes is cut.
    let mut background_seen = 0usize;
    processes.retain(|app| {
        if app.has_window {
            return true;
        }
        background_seen += 1;
        background_seen <= 150
    });

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
        power_plans: installed_plans()
            .into_iter()
            .map(|p| PowerPlanInfo { guid: p.guid, name: p.name, active: p.active })
            .collect(),
        is_admin: is_admin(),
        protected: PROTECTED_NOTES.iter().map(|s| (*s).to_string()).collect(),
    }
}

