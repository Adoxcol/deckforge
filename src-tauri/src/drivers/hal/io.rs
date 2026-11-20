use super::catalog::now_ms;
use crate::drivers::akp05e_protocol::{
    build_brightness_packet, build_button_mode_packet, build_clear_key_packet, build_heartbeat_packet, build_screen_on_packet,
};
use serde::Serialize;
use crate::drivers::frame_cache::{AntiFreezeFrameCache, FrameCacheStats};
use crate::drivers::scheduler::{self, IoStep, Transport};
use crate::LockExt;
use hidapi::HidApi;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

// The deck is opened exactly once, by one thread. That thread reads button
// reports and performs every write that other threads queue. Opening the device
// from several places at once is what made the firmware reset, drop icons, and
// sleep the panel on a press.
struct DeckJob {
    queued_at: Instant,
    steps: Vec<IoStep>,
    reply: mpsc::Sender<Result<(), String>>,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
    connection_epoch: u64,
}

impl DeckJob {
    fn validate(&self, now: Instant, connection_epoch: u64) -> Result<(), String> {
        scheduler::check_deadline(now, self.deadline, &self.cancelled)?;
        if self.connection_epoch != connection_epoch {
            return Err("Deck connection changed; stale output discarded".into());
        }
        Ok(())
    }
}

struct DeckIo {
    jobs: mpsc::SyncSender<DeckJob>,
    reports: Mutex<mpsc::Receiver<Vec<u8>>>,
}

static DECK_IO: OnceLock<DeckIo> = OnceLock::new();
static DECK_GENERATION: AtomicU64 = AtomicU64::new(0);
// Unlike DECK_GENERATION this changes only on a real connection open, not a resend.
static CONNECTION_EPOCH: AtomicU64 = AtomicU64::new(0);
static CONTROL: Mutex<ControlPolicy> = Mutex::new(ControlPolicy { paused: false, vendor_running: false });
static CONTROL_OPEN: AtomicBool = AtomicBool::new(false);

struct ControlPolicy {
    paused: bool,
    vendor_running: bool,
}

impl ControlPolicy {
    fn observe_vendor(&mut self, running: bool) {
        self.vendor_running = running;
        // Once a conflict is seen, only an explicit resume may reclaim the deck.
        if running { self.paused = true; }
    }
    fn available(&self) -> bool { !self.paused && !self.vendor_running }
    fn resume(&mut self) -> Result<(), String> {
        if self.vendor_running {
            return Err("Close the AJAZZ/Stream Dock app and its tray watcher, then choose Use DeckForge. DeckForge will not close them for you.".into());
        }
        self.paused = false;
        Ok(())
    }
    fn suspended_state(&self) -> &'static str {
        if self.vendor_running { "conflict" } else { "paused" }
    }
}

#[cfg(test)]
mod control_tests {
    use super::ControlPolicy;

    #[test]
    fn vendor_conflict_requires_explicit_resume_after_vendor_exits() {
        let mut policy = ControlPolicy { paused: false, vendor_running: false };
        assert!(policy.available());
        policy.observe_vendor(true);
        assert!(!policy.available());
        assert_eq!(policy.suspended_state(), "conflict");
        assert!(policy.resume().is_err());
        policy.observe_vendor(false);
        assert!(!policy.available());
        assert_eq!(policy.suspended_state(), "paused");
        policy.resume().unwrap();
        assert!(policy.available());
    }

    #[test]
    fn manual_pause_survives_vendor_scans() {
        let mut policy = ControlPolicy { paused: true, vendor_running: false };
        policy.observe_vendor(false);
        assert!(!policy.available());
        policy.resume().unwrap();
        assert!(policy.available());
    }
}

pub fn deck_control_available() -> bool { CONTROL.lock_safe().available() }

fn refresh_control_policy(last_scan: &mut Instant) {
    if last_scan.elapsed() >= Duration::from_secs(1) {
        let running = crate::actions::vendor_app_running(false);
        CONTROL.lock_safe().observe_vendor(running);
        *last_scan = Instant::now();
    }
}

pub fn pause_deck_control() -> String {
    CONTROL.lock_safe().paused = true;
    update_status(|s| s.state = if CONTROL_OPEN.load(Ordering::SeqCst) { "pausing" } else { "paused" });
    let _ = deck_io();
    "Pause requested. Wait for DeckForge paused before opening AJAZZ.".into()
}

pub fn resume_deck_control() -> Result<String, String> {
    let running = crate::actions::vendor_app_running(true);
    {
        let mut control = CONTROL.lock_safe();
        control.observe_vendor(running);
        control.resume()?;
    }
    request_full_push("user chose Use DeckForge");
    Ok("DeckForge will reconnect and restore your last applied layout.".into())
}

/// Waits until the I/O thread has closed the deck after a pause, so Windows can restart it.
pub fn wait_deck_released(timeout: Duration) -> bool {
    let until = Instant::now() + timeout;
    while CONTROL_OPEN.load(Ordering::SeqCst) {
        if Instant::now() >= until { return false; }
        std::thread::sleep(Duration::from_millis(50));
    }
    true
}

/// Windows device instance id of the deck's USB device, from its serial number.
/// Only letters and digits are accepted: the id is passed to an elevated command.
pub fn deck_usb_instance_id(serial: &str) -> Result<String, String> {
    let serial = serial.trim();
    if serial.is_empty() || serial.len() > 64 || !serial.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("The deck's serial number is not known, so Windows cannot find it. Plug the deck in and try again.".into());
    }
    Ok(format!("USB\\VID_0300&PID_3013\\{serial}"))
}

/// Exit codes of the elevated `--cycle-deck-port` helper.
pub const CYCLE_OK: u32 = 0;

/// Resets the USB port the deck is plugged into: Windows drops it and enumerates it again,
/// the closest software gets to a replug. Needs administrator rights, so DeckForge starts a
/// second copy of itself elevated (Windows asks the user) that does only this and exits.
#[cfg(target_os = "windows")]
pub fn restart_usb_device(serial: &str) -> Result<(), String> {
    use windows::{
        core::{w, PCWSTR},
        Win32::{
            Foundation::{CloseHandle, WAIT_OBJECT_0},
            System::Threading::{GetExitCodeProcess, WaitForSingleObject},
            UI::{Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW}, WindowsAndMessaging::SW_HIDE},
        },
    };
    deck_usb_instance_id(serial)?; // Only a plain serial reaches the elevated copy.
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let file: Vec<u16> = exe.as_os_str().to_string_lossy().encode_utf16().chain(Some(0)).collect();
    let params: Vec<u16> = format!("--cycle-deck-port {serial}").encode_utf16().chain(Some(0)).collect();
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    if unsafe { ShellExecuteExW(&mut info) }.is_err() {
        return Err("Windows did not allow the restart. It needs you to approve the permission prompt.".into());
    }
    let process = info.hProcess;
    if process.is_invalid() { return Err("Windows did not start the restart.".into()); }
    let waited = unsafe { WaitForSingleObject(process, 30_000) };
    let mut code = 1u32;
    let _ = unsafe { GetExitCodeProcess(process, &mut code) };
    let _ = unsafe { CloseHandle(process) };
    if waited != WAIT_OBJECT_0 { return Err("Windows took too long to restart the deck.".into()); }
    match code {
        CYCLE_OK => Ok(()),
        2 => Err("Windows could not find the deck. Is it plugged in?".into()),
        3 => Err("Windows could not find the USB port the deck is on.".into()),
        _ => Err(format!("Windows could not reset the deck's USB port (code {code}).")),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn restart_usb_device(_serial: &str) -> Result<(), String> {
    Err("Restarting the deck is only available on Windows.".into())
}

/// Runs in the elevated helper copy: finds the deck's hub and port and asks the hub to
/// cycle that port. Returns the process exit code.
#[cfg(target_os = "windows")]
pub fn cycle_deck_port(serial: &str) -> u32 {
    use windows::{
        core::{GUID, PCWSTR},
        Win32::{
            Devices::DeviceAndDriverInstallation::*,
            Foundation::{CloseHandle, GENERIC_WRITE},
            Storage::FileSystem::{CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING},
            System::IO::DeviceIoControl,
        },
    };
    const IOCTL_USB_HUB_CYCLE_PORT: u32 = 0x0022_0444;
    const GUID_DEVINTERFACE_USB_HUB: GUID = GUID::from_u128(0xf18a0e88_c30c_11d0_8815_00a0c906bed8);
    let Ok(id) = deck_usb_instance_id(serial) else { return 2 };
    let id: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let mut dev = 0u32;
        if CM_Locate_DevNodeW(&mut dev, PCWSTR(id.as_ptr()), CM_LOCATE_DEVNODE_NORMAL) != CR_SUCCESS { return 2; }
        let mut hub = 0u32;
        if CM_Get_Parent(&mut hub, dev, 0) != CR_SUCCESS { return 3; }
        let mut port = 0u32;
        let mut len = 4u32;
        if CM_Get_DevNode_Registry_PropertyW(dev, CM_DRP_ADDRESS, None, Some((&mut port as *mut u32).cast()), &mut len, 0) != CR_SUCCESS {
            return 3;
        }
        let mut hub_id = vec![0u16; 512];
        if CM_Get_Device_IDW(hub, &mut hub_id, 0) != CR_SUCCESS { return 3; }
        let mut size = 0u32;
        if CM_Get_Device_Interface_List_SizeW(&mut size, &GUID_DEVINTERFACE_USB_HUB, PCWSTR(hub_id.as_ptr()), CM_GET_DEVICE_INTERFACE_LIST_PRESENT) != CR_SUCCESS || size < 2 {
            return 3;
        }
        let mut list = vec![0u16; size as usize];
        if CM_Get_Device_Interface_ListW(&GUID_DEVINTERFACE_USB_HUB, PCWSTR(hub_id.as_ptr()), &mut list, CM_GET_DEVICE_INTERFACE_LIST_PRESENT) != CR_SUCCESS {
            return 3;
        }
        let Ok(handle) = CreateFileW(PCWSTR(list.as_ptr()), GENERIC_WRITE.0, FILE_SHARE_READ | FILE_SHARE_WRITE, None, OPEN_EXISTING, FILE_FLAGS_AND_ATTRIBUTES(0), None) else {
            return 4;
        };
        // USB_CYCLE_PORT_PARAMS { ConnectionIndex, StatusReturned }
        let mut params = [port, 0u32];
        let mut returned = 0u32;
        let ok = DeviceIoControl(
            handle, IOCTL_USB_HUB_CYCLE_PORT,
            Some(params.as_ptr().cast()), 8, Some(params.as_mut_ptr().cast()), 8,
            Some(&mut returned), None,
        ).is_ok();
        let _ = CloseHandle(handle);
        if ok { CYCLE_OK } else { 5 }
    }
}

/// What the deck connection is doing, for the status line in the app.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckStatus {
    /// `searching` (never found yet), `connected`, or `reconnecting` (it dropped and the
    /// app is waiting for it to come back).
    pub state: &'static str,
    pub opened_at_ms: Option<u64>,
    pub last_push_at_ms: Option<u64>,
    pub last_push_ok: Option<bool>,
    pub last_error: Option<String>,
    /// How many times the deck came back after dropping off USB.
    pub reconnects: u32,
}

static DECK_STATUS: Mutex<DeckStatus> = Mutex::new(DeckStatus {
    state: "searching",
    opened_at_ms: None,
    last_push_at_ms: None,
    last_push_ok: None,
    last_error: None,
    reconnects: 0,
});

fn update_status(change: impl FnOnce(&mut DeckStatus)) {
    let mut status = DECK_STATUS.lock().unwrap_or_else(|e| e.into_inner());
    change(&mut status);
}

/// Records how a send to the deck went. A failed send is always recorded. A successful
/// one only counts when it was a full layout, so frame-by-frame animation writes do not
/// hide an earlier failure.
pub(crate) fn note_push(result: &Result<(), String>, full_layout: bool) {
    match result {
        Ok(()) if full_layout => update_status(|s| {
            s.last_push_at_ms = Some(now_ms());
            s.last_push_ok = Some(true);
            s.last_error = None;
        }),
        Ok(()) => {}
        Err(err) => update_status(|s| {
            s.last_push_at_ms = Some(now_ms());
            s.last_push_ok = Some(false);
            s.last_error = Some(err.clone());
        }),
    }
}

/// Current connection state and the outcome of the last send.
pub fn deck_status() -> DeckStatus {
    let _ = deck_io();
    DECK_STATUS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn deck_io() -> &'static DeckIo {
    DECK_IO.get_or_init(|| {
        let (job_tx, job_rx) = mpsc::sync_channel::<DeckJob>(scheduler::QUEUE_CAPACITY);
        let (report_tx, report_rx) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || deck_io_loop(job_rx, report_tx));
        DeckIo {
            jobs: job_tx,
            reports: Mutex::new(report_rx),
        }
    })
}

fn open_akp05e(api: &HidApi) -> Option<hidapi::HidDevice> {
    api.device_list().find_map(|info| {
        if info.vendor_id() != 0x0300 || info.product_id() != 0x3013 {
            return None;
        }
        let path = info.path().to_string_lossy().to_lowercase();
        if path.contains("mi_00") || info.interface_number() == 0 {
            info.open_device(api).ok()
        } else {
            None
        }
    })
}

/// Appends one line to `%APPDATA%\DeckForge\deck-io.log` so a drop or a black panel
/// can be matched to what the connection was doing.
fn deck_log(line: &str) {
    use std::io::Write;
    let Some(dir) = std::env::var_os("APPDATA") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("DeckForge");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("deck-io.log");
    if std::fs::metadata(&path).map(|m| m.len() > 1_000_000).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{} {}", now_ms(), line);
    }
}

fn debug_flag_path(name: &str) -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("APPDATA")?;
    Some(std::path::PathBuf::from(dir).join("DeckForge").join(name))
}

/// True while a marker file with this name exists in `%APPDATA%\DeckForge`.
pub(crate) fn debug_flag(name: &str) -> bool {
    debug_flag_path(name).map(|p| p.exists()).unwrap_or(false)
}

/// True once, then deletes the marker file.
fn debug_flag_take(name: &str) -> bool {
    match debug_flag_path(name) {
        Some(path) if path.exists() => {
            let _ = std::fs::remove_file(path);
            true
        }
        _ => false,
    }
}

/// Reads and deletes a marker file that carries text, or None when it is absent.
fn debug_file_take(name: &str) -> Option<String> {
    let path = debug_flag_path(name)?;
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(path);
    Some(text)
}

/// Runs a protocol experiment from `%APPDATA%\DeckForge\deck-cmd.txt`: space-separated
/// tokens such as `LIG95 CON W3000 MOD1`. Each result goes to `deck-io.log`. This is how
/// a wake sequence is tried on the real deck without rebuilding the app.
fn run_debug_script(script: &str) {
    let mut steps = Vec::new();
    for token in script.split_whitespace().take(256) {
        let upper = token.to_ascii_uppercase();
        if let Some(ms) = upper.strip_prefix('W').and_then(|v| v.parse::<u64>().ok()) {
            steps.push(IoStep::command(Vec::new(), ms));
            continue;
        }
        let report = if let Some(percent) = upper.strip_prefix("LIG").and_then(|v| v.parse::<u8>().ok()) {
            build_brightness_packet(percent)
        } else {
            match upper.as_str() {
                "DIS" => build_screen_on_packet(),
                "MOD1" => build_button_mode_packet(),
                "CON" => build_heartbeat_packet(),
                "STP" => crate::drivers::akp05e_protocol::build_finish_packet(),
                "CLE" => build_clear_key_packet(0xFF),
                "VER" => crate::drivers::akp05e_protocol::build_firmware_version_packet(),
                other => { deck_log(&format!("debug unknown token {other}")); continue; }
            }
        };
        steps.push(IoStep::command(report.to_vec(), 150));
    }
    // Debug waits use the same bounded, responsive scheduler as normal traffic.
    if let Err(err) = scheduler::validate_steps(&steps) {
        deck_log(&format!("debug job rejected: {err}"));
        return;
    }
    let (reply, _) = mpsc::channel();
    let job = DeckJob {
        queued_at: Instant::now(),
        steps, reply, deadline: Instant::now() + scheduler::JOB_TIMEOUT,
        cancelled: Arc::new(AtomicBool::new(false)),
        connection_epoch: CONNECTION_EPOCH.load(Ordering::SeqCst),
    };
    if enqueue_job(&deck_io().jobs, job).is_err() { deck_log("debug queue is busy"); }
}

fn validate_report_length(written: usize, expected: usize) -> Result<(), String> {
    if written == expected { Ok(()) }
    else { Err(format!("Short HID write: {written}/{expected} bytes")) }
}

fn write_complete_report(handle: &hidapi::HidDevice, data: &[u8]) -> Result<(), String> {
    let written = handle.write(data).map_err(|e| e.to_string())?;
    validate_report_length(written, data.len())
}

struct HidTransport<'a> {
    handle: &'a hidapi::HidDevice,
    reports: &'a mpsc::Sender<Vec<u8>>,
    heartbeat_enabled: bool,
    last_control_scan: &'a mut Instant,
}

impl Transport for HidTransport<'_> {
    fn now(&self) -> Instant { Instant::now() }
    fn write(&mut self, report: &[u8]) -> Result<(), String> {
        if !deck_control_available() { return Err("DeckForge device control is paused".into()); }
        write_complete_report(self.handle, report)
    }
    fn poll_input(&mut self, timeout_ms: i32) -> Result<(), String> {
        refresh_control_policy(self.last_control_scan);
        if !deck_control_available() { return Err("DeckForge device control is paused".into()); }
        let mut buf = [0u8; 2048];
        let n = self.handle.read_timeout(&mut buf, timeout_ms).map_err(|e| e.to_string())?;
        if n > 0 {
            self.reports.send(buf[..n].to_vec()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn heartbeat(&mut self) -> Result<(), String> {
        if !deck_control_available() { return Err("DeckForge device control is paused".into()); }
        if self.heartbeat_enabled { write_complete_report(self.handle, &build_heartbeat_packet())?; }
        Ok(())
    }
}

fn enqueue_job(jobs: &mpsc::SyncSender<DeckJob>, job: DeckJob) -> Result<(), String> {
    jobs.try_send(job).map_err(|err| match err {
        mpsc::TrySendError::Full(_) => "Deck output queue is busy; try again".into(),
        mpsc::TrySendError::Disconnected(_) => "Deck output worker stopped".into(),
    })
}

fn reject_queued_jobs(jobs: &mpsc::Receiver<DeckJob>, message: &str) {
    // Bound the drain too: new producers cannot keep the I/O loop here forever.
    for _ in 0..scheduler::QUEUE_CAPACITY {
        let Ok(job) = jobs.try_recv() else { break };
        let _ = job.reply.send(Err(message.to_string()));
    }
}

fn deck_io_loop(jobs: mpsc::Receiver<DeckJob>, reports: mpsc::Sender<Vec<u8>>) {
    let mut open: Option<(HidApi, hidapi::HidDevice)> = None;
    let mut errors = 0u8;
    let mut buf = [0u8; 2048];
    let mut last_heartbeat = Instant::now();
    let mut last_full_push: Option<Instant> = None;
    let mut last_control_scan = Instant::now() - Duration::from_secs(2);
    deck_log("io thread started");
    loop {
        refresh_control_policy(&mut last_control_scan);
        if !deck_control_available() {
            open = None; // Drop the HID handle; do not send a disconnect or heartbeat.
            CONTROL_OPEN.store(false, Ordering::SeqCst);
            reject_queued_jobs(&jobs, "DeckForge device control is paused");
            let state = CONTROL.lock_safe().suspended_state();
            update_status(|s| s.state = state);
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        if open.is_none() {
            let opened = HidApi::new()
                .ok()
                .and_then(|api| open_akp05e(&api).map(|handle| (api, handle)));
            match opened {
                Some(pair) => {
                    errors = 0;
                    CONNECTION_EPOCH.fetch_add(1, Ordering::SeqCst);
                    last_full_push = None;
                    last_heartbeat = Instant::now() - std::time::Duration::from_secs(60);
                    open = Some(pair);
                    CONTROL_OPEN.store(true, Ordering::SeqCst);
                    deck_log("opened deck, waiting for firmware to settle");
                    // A deck that just re-enumerated is still booting. Pictures sent in
                    // that window are dropped and nothing sends them again, which left the
                    // panel black.
                    std::thread::sleep(std::time::Duration::from_millis(2500));
                    refresh_control_policy(&mut last_control_scan);
                    if !deck_control_available() { continue; }
                    let generation = DECK_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
                    update_status(|s| {
                        if s.opened_at_ms.is_some() {
                            s.reconnects = s.reconnects.saturating_add(1);
                        }
                        s.state = "connected";
                        s.opened_at_ms = Some(now_ms());
                    });
                    deck_log(&format!("deck ready, generation {generation}"));
                }
                None => {
                    reject_queued_jobs(&jobs, "Deck is not open");
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    continue;
                }
            }
        }
        let handle = &open.as_ref().expect("open deck").1;
        let mut lost = false;
        // The firmware reboots itself about 45s after the last heartbeat. The official
        // app sends `CRT CONNECT` on a timer. Without it the deck dropped off the bus
        // every 46 seconds, which looked like it connecting and reconnecting.
        if last_heartbeat.elapsed() >= std::time::Duration::from_secs(8) {
            last_heartbeat = Instant::now();
            if debug_flag("no-heartbeat") {
                deck_log("heartbeat skipped (debug flag)");
            } else if let Err(err) = write_complete_report(handle, &build_heartbeat_packet()) {
                deck_log(&format!("heartbeat write failed: {err}"));
                lost = true;
            } else {
                deck_log("heartbeat sent");
            }
        }
        if debug_flag_take("wake-now") {
            let lig = build_brightness_packet(95);
            let a = handle.write(&build_screen_on_packet());
            let b = handle.write(&lig);
            deck_log(&format!("debug wake-now: DIS={a:?} LIG={b:?}"));
        }
        if debug_flag_take("brightness-0") {
            let r = handle.write(&build_brightness_packet(0));
            deck_log(&format!("debug brightness-0: {r:?}"));
        }
        if let Some(script) = debug_file_take("deck-cmd.txt") {
            run_debug_script(&script);
        }
        // Service one job per outer iteration. The job itself reads input during
        // all pauses and between packets, and checks heartbeat deadlines at safe boundaries.
        if !lost {
            if let Ok(job) = jobs.try_recv() {
                crate::performance::record(crate::performance::Stage::UsbQueue, job.queued_at.elapsed());
                let valid = job.validate(Instant::now(), CONNECTION_EPOCH.load(Ordering::SeqCst));
                if let Err(err) = valid {
                    let _ = job.reply.send(Err(err));
                } else {
                    let full = job.steps.first().is_some_and(|s| s.report.get(6..9) == Some(b"DIS".as_slice()));
                    let settle = if full {
                        last_full_push.map(|done| Duration::from_millis(1500).saturating_sub(done.elapsed())).unwrap_or_default()
                    } else { Duration::ZERO };
                    let started = Instant::now();
                    let mut transport = HidTransport { handle, reports: &reports,
                        heartbeat_enabled: !debug_flag("no-heartbeat"), last_control_scan: &mut last_control_scan };
                    let result = scheduler::run_job(&mut transport, &job.steps, settle,
                        job.deadline, &job.cancelled, &mut last_heartbeat);
                    crate::performance::record(crate::performance::Stage::UsbWrite, started.elapsed());
                    if job.steps.len() > 1 || result.is_err() {
                        deck_log(&format!("job packets={} took={}ms result={result:?}", job.steps.len(), started.elapsed().as_millis()));
                    }
                    if full && result.is_ok() { last_full_push = Some(Instant::now()); }
                    // A timeout or error may leave a partial image on the wire. Close
                    // this session before accepting another job; reconnect restores it.
                    lost = result.is_err();
                    let _ = job.reply.send(result);
                }
            }
        }
        if !lost {
            match handle.read_timeout(&mut buf, 15) {
                Ok(0) => errors = 0,
                Ok(n) => {
                    errors = 0;
                    let hex = buf[..n.min(14)]
                        .iter()
                        .map(|b| format!("{b:02X}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    deck_log(&format!("report {hex}"));
                    let _ = reports.send(buf[..n].to_vec());
                }
                Err(err) => {
                    errors = errors.saturating_add(1);
                    deck_log(&format!("read error #{errors}: {err}"));
                    lost = errors >= 3;
                }
            }
        }
        if lost {
            deck_log("deck connection lost, closing");
            update_status(|s| s.state = "reconnecting");
            open = None;
            CONTROL_OPEN.store(false, Ordering::SeqCst);
            reject_queued_jobs(&jobs, "Deck connection lost; queued output discarded");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}

/// Queues packets for the deck's single connection and waits for them to be sent.
pub fn deck_write(steps: Vec<(Vec<u8>, u64)>) -> Result<(), String> {
    deck_write_steps(steps.into_iter().map(|(report, pause)| IoStep::command(report, pause)).collect())
}

pub(in crate::drivers) fn deck_write_steps(steps: Vec<IoStep>) -> Result<(), String> {
    if steps.is_empty() { return Ok(()); }
    if !deck_control_available() { return Err("DeckForge is paused. Close AJAZZ, then choose Use DeckForge.".into()); }
    scheduler::validate_steps(&steps)?;
    let io = deck_io();
    let (reply, wait) = mpsc::channel();
    let deadline = Instant::now() + scheduler::JOB_TIMEOUT;
    let cancelled = Arc::new(AtomicBool::new(false));
    enqueue_job(&io.jobs, DeckJob {
        queued_at: Instant::now(),
        steps, reply, deadline, cancelled: Arc::clone(&cancelled),
        connection_epoch: CONNECTION_EPOCH.load(Ordering::SeqCst),
    })?;
    match wait.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(result) => result,
        Err(_) => {
            cancelled.store(true, Ordering::Release);
            Err("Deck write timed out or worker stopped".into())
        }
    }
}

/// Next button/knob/touch report read from the deck, if one arrives in time.
pub fn deck_next_report(timeout_ms: u64) -> Option<Vec<u8>> {
    let io = deck_io();
    let rx = io.reports.lock_safe();
    rx.recv_timeout(std::time::Duration::from_millis(timeout_ms)).ok()
}

/// Increases each time the deck is opened. A change means the layout must be sent again.
pub fn deck_generation() -> u64 {
    let _ = deck_io();
    DECK_GENERATION.load(Ordering::SeqCst)
}

/// Makes the input worker send the whole layout again, as if the deck had been replugged.
/// Used after another program (the official Stream Dock app) was drawing on the deck.
pub fn request_full_push(reason: &str) {
    let _ = deck_io();
    DECK_GENERATION.fetch_add(1, Ordering::SeqCst);
    deck_log(&format!("full push requested: {reason}"));
}

/// Writes a line to `%APPDATA%\DeckForge\deck-io.log` for events outside the IO thread.
pub fn deck_note(line: &str) {
    deck_log(line);
}


pub fn get_cache_stats(cache: &AntiFreezeFrameCache) -> FrameCacheStats {
    cache.stats.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::catalog::*;
    use super::super::panel::*;
    use super::super::publish::*;

    fn queued_test_job(epoch: u64) -> (DeckJob, mpsc::Receiver<Result<(), String>>) {
        let (reply, wait) = mpsc::channel();
        (DeckJob { queued_at: Instant::now(), steps: vec![IoStep::command(vec![0], 0)], reply,
            deadline: Instant::now() + scheduler::JOB_TIMEOUT,
            cancelled: Arc::new(AtomicBool::new(false)), connection_epoch: epoch }, wait)
    }

    #[test]
    fn output_queue_rejects_overload_and_disconnect_rejects_pending_work() {
        let (tx, rx) = mpsc::sync_channel(scheduler::QUEUE_CAPACITY);
        let mut replies = Vec::new();
        for _ in 0..scheduler::QUEUE_CAPACITY {
            let (job, reply) = queued_test_job(1);
            enqueue_job(&tx, job).unwrap();
            replies.push(reply);
        }
        assert!(enqueue_job(&tx, queued_test_job(1).0).unwrap_err().contains("busy"));
        reject_queued_jobs(&rx, "connection lost");
        for reply in replies { assert!(reply.try_recv().unwrap().is_err()); }
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn queued_jobs_cannot_cross_connection_epochs_or_outlive_cancellation() {
        let (job, _) = queued_test_job(7);
        assert!(job.validate(Instant::now(), 7).is_ok());
        assert!(job.validate(Instant::now(), 8).is_err());
        assert!(job.validate(job.deadline, 7).is_err());
        job.cancelled.store(true, Ordering::Release);
        assert!(job.validate(Instant::now(), 7).is_err());
    }
    use crate::drivers::frame_cache::AntiFreezeFrameCache;
    use crate::media::MediaStore;

    #[test]
    fn verified_akp_key_burst_preserves_existing_packets() {
        let dev = builtin_device_catalog().remove(0);
        let jpeg = [0xFF, 0xD8, 0x01, 0x02, 0xFF, 0xD9];
        let actual = key_image_burst(&dev, key_image_slot(&dev, 0), &jpeg);
        let expected = crate::drivers::akp05e_protocol::build_key_image_packets(
            key_image_slot(&dev, 0), &jpeg,
        );
        assert_eq!(actual.reports, expected.into_iter().map(|packet| packet.to_vec()).collect::<Vec<_>>());
    }

    #[test]
    fn unverified_device_cannot_write_to_the_akp_connection() {
        let mut catalog = builtin_device_catalog();
        let mut other = catalog.remove(2);
        other.connected = true;
        other.is_simulator = false;
        let mut diagnostics = Vec::new();
        assert!(send_hardware_brightness(&other, 50, &mut diagnostics).unwrap_err().contains("not yet supported"));
        assert!(write_animation_frames(&other, &[key_image_burst(&other, 1, &[1, 2, 3])]).is_err());
        assert!(write_one_report(&other, &[0; 32]).is_err());
    }

    #[test]
    fn short_writes_and_disconnected_decks_cannot_report_success() {
        assert!(validate_report_length(1024, 1025).is_err());
        assert!(validate_report_length(0, 1025).is_err());
        assert!(validate_report_length(1025, 1025).is_ok());
        let dev = builtin_device_catalog().remove(0);
        assert!(write_animation_frames(&dev, &[]).is_err());
        assert!(write_one_report(&dev, &[0; 1025]).is_err());
    }

    fn job(index: u8) -> ScreenJob {
        ScreenJob {
            index,
            title: format!("Key {index}"),
            show_title: true,
            icon_name: "circle".into(),
            accent: "#8B5CF6".into(),
            bg: "#151925".into(),
            media_id: None,
            fit: "cover".into(),
            speed: 1.0,
            loop_playback: false,
            level: None,
            art: None,
        }
    }

    fn zone(label: &str) -> ZoneJob {
        ZoneJob {
            label: label.into(),
            value_text: "Live".into(),
            accent: "#22D3EE".into(),
            media_id: None,
            fit: "cover".into(),
            speed: 1.0,
            loop_playback: false,
        }
    }

    fn bat_slots(bursts: &[HidBurst]) -> Vec<u8> {
        bursts
            .iter()
            .filter_map(|burst| {
                let report = burst.reports.first()?;
                if report.len() > 13 && &report[6..9] == b"BAT" {
                    Some(report[13])
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn akp05e_publish_puts_each_row_on_its_own_keys() {
        let dev = builtin_device_catalog()
            .into_iter()
            .find(|d| d.pid == 0x3013)
            .expect("akp05e catalog entry");
        let keys: Vec<ScreenJob> = (0..10).map(job).collect();
        let zones = vec![
            zone("SPOTIFY"),
            zone("CPU LOAD"),
            zone("MIXER"),
            zone("MIC MUTE"),
        ];
        let media = MediaStore::default();
        let mut cache = AntiFreezeFrameCache::new();
        let mut diags = Vec::new();
        let outcome = publish_screens(&dev, &keys, &zones, None, &media, &mut cache, &mut diags);
        assert_eq!(
            bat_slots(&outcome.bursts),
            vec![11, 12, 13, 14, 15, 6, 7, 8, 9, 10, 1, 2, 3, 4]
        );
        assert!(outcome.bursts.iter().all(|burst| {
            burst
                .reports
                .first()
                .map(|report| &report[6..9] != b"DRA")
                .unwrap_or(true)
        }));
    }
}

#[cfg(test)]
mod restart_tests {
    use super::deck_usb_instance_id;

    #[test]
    fn instance_id_only_from_a_plain_serial() {
        assert_eq!(deck_usb_instance_id("4251E8782A0B").unwrap(), r"USB\VID_0300&PID_3013\4251E8782A0B");
        assert!(deck_usb_instance_id("").is_err());
        assert!(deck_usb_instance_id("Not detected").is_err());
        assert!(deck_usb_instance_id("AB\" & calc").is_err());
    }
}
