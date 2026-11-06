use super::*;

// ---------------------------------------------------------------------------
// Game detection and the background watcher
// ---------------------------------------------------------------------------

pub(super) struct Foreground {
    pub(super) pid: u32,
    pub(super) path: String,
    pub(super) fullscreen: bool,
}

pub(super) fn process_path(pid: u32) -> Option<String> {
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

pub(super) fn foreground() -> Option<Foreground> {
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

pub(super) fn exe_stem(path: &str) -> String {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path).to_lowercase();
    file.strip_suffix(".exe").unwrap_or(&file).to_string()
}

/// A full-screen window that is either exclusive-mode Direct3D or lives in a game install folder.
pub(super) fn detect_game() -> Option<(u32, String)> {
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
pub(super) fn running_mode_has(cfg: &GameModeConfig, mode_name: &str, tweak: &str) -> bool {
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
