use super::*;

// ---------------------------------------------------------------------------
// Apps a mode starts, such as a recorder or a network booster
// ---------------------------------------------------------------------------

/// A program a mode starts when it turns on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct StartApp {
    pub label: String,
    /// Full path to the .exe.
    pub path: String,
    /// Extra command-line arguments, split on spaces. Overwolf apps need `-launchapp <id>`.
    pub args: String,
    /// Close it again when Game Mode ends, but only if Game Mode started it.
    pub close_on_end: bool,
}

/// One program Game Mode started, so it can close the same process later.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct StartedApp {
    pub label: String,
    pub pid: u32,
    pub close_on_end: bool,
}

pub(super) fn clean_start_apps(apps: Vec<StartApp>) -> Vec<StartApp> {
    let mut seen = HashSet::new();
    apps.into_iter()
        .filter_map(|mut app| {
            app.path = app.path.trim().trim_matches('"').to_string();
            let lower = app.path.to_lowercase();
            if !lower.ends_with(".exe") || app.path.len() > 400 || !seen.insert(lower) {
                return None;
            }
            let stem = exe_stem(&app.path);
            if NEVER_KILL_NAMES.contains(&stem.as_str()) {
                return None;
            }
            app.args = app.args.trim().chars().take(300).collect();
            app.label = clean_label(&app.label, &stem);
            Some(app)
        })
        .take(10)
        .collect()
}

/// Lowercase exe names (no ".exe") of a mode's start apps. Game Mode never closes these.
pub(super) fn start_app_stems(apps: &[StartApp]) -> HashSet<String> {
    apps.iter().map(|app| exe_stem(&app.path)).collect()
}

fn running_already(app: &StartApp, procs: &[Proc]) -> bool {
    let path = app.path.to_lowercase();
    let stem = exe_stem(&app.path);
    procs.iter().any(|p| p.exe_lower == path || p.name == stem)
}

const ERROR_ELEVATION_REQUIRED: i32 = 740;

/// Starts each app that is not running yet. An app that needs admin rights is started through
/// the Windows prompt, but never while something is full screen, since that prompt over a game
/// can freeze the display.
pub(super) fn start_apps(apps: &[StartApp], notes: &mut Vec<String>) -> Vec<StartedApp> {
    let running = snapshot();
    let mut started = Vec::new();
    for app in apps {
        if running_already(app, &running) {
            notes.push(format!("{} was already running.", app.label));
            continue;
        }
        let path = PathBuf::from(&app.path);
        if !path.exists() {
            notes.push(format!("{} was not started: {} no longer exists.", app.label, app.path));
            continue;
        }
        let args: Vec<&str> = app.args.split_whitespace().collect();
        let mut command = Command::new(&path);
        command.args(&args);
        if let Some(dir) = path.parent() {
            command.current_dir(dir);
        }
        match command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP).spawn() {
            Ok(child) => {
                notes.push(format!("Started {}.", app.label));
                started.push(StartedApp { label: app.label.clone(), pid: child.id(), close_on_end: app.close_on_end });
            }
            Err(err) if err.raw_os_error() == Some(ERROR_ELEVATION_REQUIRED) => {
                if something_is_fullscreen() {
                    notes.push(format!(
                        "{} needs admin rights, and the Windows prompt is not shown over a full-screen game. Turn Game Mode on before the game, or start {} yourself.",
                        app.label, app.label
                    ));
                    continue;
                }
                match start_elevated(&path, &app.args) {
                    Some(pid) => {
                        notes.push(format!("Started {} with admin rights.", app.label));
                        started.push(StartedApp { label: app.label.clone(), pid, close_on_end: app.close_on_end });
                    }
                    None => notes.push(format!("{} was not started: the admin prompt was declined.", app.label)),
                }
            }
            Err(err) => notes.push(format!("{} could not start: {err}", app.label)),
        }
        // One at a time, so they do not all load from disk together.
        std::thread::sleep(Duration::from_millis(800));
    }
    started
}

fn start_elevated(path: &std::path::Path, args: &str) -> Option<u32> {
    use windows::core::{w, HSTRING};
    use windows::Win32::System::Threading::GetProcessId;
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let file = HSTRING::from(path.as_os_str());
    let params = HSTRING::from(args);
    let dir = HSTRING::from(path.parent().map(|d| d.as_os_str()).unwrap_or_default());
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: windows::core::PCWSTR(file.as_ptr()),
        lpParameters: windows::core::PCWSTR(params.as_ptr()),
        lpDirectory: windows::core::PCWSTR(dir.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe {
        ShellExecuteExW(&mut info).ok()?;
        if info.hProcess.is_invalid() {
            return None;
        }
        let pid = GetProcessId(info.hProcess);
        let _ = CloseHandle(info.hProcess);
        (pid != 0).then_some(pid)
    }
}

/// Closes the apps Game Mode started and was told to close: a polite close first, then a
/// forced one for anything still open after a few seconds.
pub(super) fn stop_started_apps(apps: &[StartedApp], notes: &mut Vec<String>) {
    let pids: HashSet<u32> = apps.iter().filter(|a| a.close_on_end && a.pid != 0 && pid_alive(a.pid)).map(|a| a.pid).collect();
    if pids.is_empty() {
        return;
    }
    post_close(&pids);
    wait_until_gone(&pids, Duration::from_secs(5));
    for app in apps.iter().filter(|a| pids.contains(&a.pid)) {
        if pid_alive(app.pid) {
            terminate(app.pid);
        }
        notes.push(format!("Closed {}.", app.label));
    }
}
