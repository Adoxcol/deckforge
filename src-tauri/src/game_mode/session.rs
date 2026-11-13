use super::*;

// ---------------------------------------------------------------------------
// Enable / disable
// ---------------------------------------------------------------------------

pub(super) fn reopen_label(reopen: Reopen) -> &'static str {
    match reopen {
        Reopen::Exe => "exe",
        Reopen::No => "no",
    }
}

pub(super) fn closed_record(def: &TargetDef, matched: &[&Proc]) -> ClosedApp {
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
    }
}

pub(super) fn run_wsl_shutdown() {
    let _ = Command::new("wsl.exe")
        .arg("--shutdown")
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

/// True while something is full-screen in front. A UAC prompt there would switch to the
/// secure desktop, which reset the display driver and froze the PC.
pub(super) fn something_is_fullscreen() -> bool {
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

    let (plan, previous_plan, applied_plan) = apply_plan(&mode.power_plan, &mut notes);

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
    // Apps this mode starts are never closed by it, even when a close target would match them.
    let keep = start_app_stems(&mode.start_apps);
    for def in &selected {
        let mut matched = matched_procs(def, &before, &protect);
        matched.retain(|p| !keep.contains(&p.name));
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

    // Start apps after closing, so the freed memory goes to them and the game.
    let started_apps = start_apps(&mode.start_apps, &mut notes);

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
            started: started_apps,
            stopped_services: stopped,
            failed_services: failed,
            helper_used,
            ram_free_before_mb: ram_before,
            ram_free_after_mb: ram_after,
            processes_before: before.len(),
            processes_after,
            power_plan: plan,
            previous_power_plan: previous_plan,
            applied_power_plan: applied_plan,
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

pub(super) fn spawn_detached(command: &mut Command) -> bool {
    command
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .is_ok()
}

pub(super) fn reopen_app(app: &ClosedApp) -> bool {
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

    stop_started_apps(&current.started, &mut notes);

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

    // The power plan goes back whenever the mode switched it, except when the user chose to
    // restore nothing.
    if cfg.restore != "none" {
        restore_plan(&current.previous_power_plan, &current.applied_power_plan, &mut notes);
    } else if !current.applied_power_plan.is_empty() {
        notes.push("The power plan was left as it is (restore is off).".to_string());
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

