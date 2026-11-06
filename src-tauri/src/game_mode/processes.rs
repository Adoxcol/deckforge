use super::*;

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(super) struct Proc {
    pub(super) pid: u32,
    pub(super) parent: Option<u32>,
    /// Lowercase, without ".exe".
    pub(super) name: String,
    pub(super) exe: Option<PathBuf>,
    pub(super) exe_lower: String,
    pub(super) cmd: Vec<String>,
    pub(super) mem: u64,
}

pub(super) fn snapshot() -> Vec<Proc> {
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
pub(super) fn protected_pids(procs: &[Proc]) -> HashSet<u32> {
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

pub(super) fn never_kill(p: &Proc) -> bool {
    NEVER_KILL_NAMES.contains(&p.name.as_str())
        || NEVER_KILL_PATHS.iter().any(|frag| p.exe_lower.contains(frag))
}

pub(super) fn is_dev_stack(p: &Proc) -> bool {
    let joined = p.cmd.join(" ").to_lowercase();
    joined.contains("deckforge") || joined.contains("tauri") || joined.contains("vite")
}

pub(super) fn matches(def: &TargetDef, p: &Proc) -> bool {
    def.procs.contains(&p.name.as_str()) || def.prefixes.iter().any(|prefix| p.name.starts_with(prefix))
}

pub(super) fn has_process_rules(def: &TargetDef) -> bool {
    !(def.procs.is_empty() && def.prefixes.is_empty())
}

pub(super) fn matched_procs<'a>(def: &TargetDef, procs: &'a [Proc], protect: &HashSet<u32>) -> Vec<&'a Proc> {
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

pub(super) fn free_ram_mb() -> u64 {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.available_memory() / MB
}

pub(super) fn is_admin() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

pub(super) fn pid_alive(pid: u32) -> bool {
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

pub(super) fn terminate(pid: u32) -> bool {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) else {
            return false;
        };
        let ok = TerminateProcess(handle, 1).is_ok();
        let _ = CloseHandle(handle);
        ok
    }
}

pub(super) fn raise_priority(pid: u32) -> bool {
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
pub(super) struct BackgroundWork;

impl BackgroundWork {
    pub(super) fn begin() -> Self {
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

unsafe extern "system" fn window_enum(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let pids = &mut *(lparam.0 as *mut HashSet<u32>);
    let top_level = GetWindow(hwnd, GW_OWNER).map(|owner| owner.0.is_null()).unwrap_or(true);
    let tool_window = (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0) != 0;
    if top_level && !tool_window && IsWindowVisible(hwnd).as_bool() && GetWindowTextLengthW(hwnd) > 0 {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != 0 {
            pids.insert(pid);
        }
    }
    BOOL(1)
}

/// Processes that own a normal visible window, which is what the user thinks of as an open app.
pub(super) fn windowed_pids() -> HashSet<u32> {
    let mut pids: HashSet<u32> = HashSet::new();
    unsafe {
        let _ = EnumWindows(Some(window_enum), LPARAM(&mut pids as *mut HashSet<u32> as isize));
    }
    pids
}

/// Asks every visible window of these processes to close, the way the X button does.
pub(super) fn post_close(pids: &HashSet<u32>) {
    unsafe {
        let _ = EnumWindows(Some(close_enum), LPARAM(pids as *const HashSet<u32> as isize));
    }
}

pub(super) fn wait_until_gone(pids: &HashSet<u32>, limit: Duration) {
    let started = Instant::now();
    while started.elapsed() < limit {
        if !pids.iter().any(|pid| pid_alive(*pid)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

