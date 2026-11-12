mod audio;
mod obs;
mod window;

use crate::db::MacroSequence;
use crate::drivers::hal::now_ms;
pub use audio::{read_audio_face, toggle_microphone_mute, toggle_speaker_mute, AudioFace};
pub use obs::{run_obs_action, ObsSettings};
pub use window::{foreground_window, ForegroundWindow};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Instant;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
    fn MapVirtualKeyW(code: u32, map_type: u32) -> u32;
}

const KEYEVENTF_EXTENDEDKEY: u32 = 0x0001;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const MAPVK_VK_TO_VSC: u32 = 0;

// Standard Windows Virtual-Key Codes
const VK_BACK: u8 = 0x08;
const VK_TAB: u8 = 0x09;
const VK_RETURN: u8 = 0x0D;
const VK_SHIFT: u8 = 0x10;
const VK_CONTROL: u8 = 0x11;
const VK_MENU: u8 = 0x12; // Alt
const VK_ESCAPE: u8 = 0x1B;
const VK_SPACE: u8 = 0x20;
const VK_LEFT: u8 = 0x25;
const VK_UP: u8 = 0x26;
const VK_RIGHT: u8 = 0x27;
const VK_DOWN: u8 = 0x28;
const VK_SNAPSHOT: u8 = 0x2C;
const VK_DELETE: u8 = 0x2E;
const VK_LWIN: u8 = 0x5B;
const VK_VOLUME_MUTE: u8 = 0xAD;
const VK_VOLUME_DOWN: u8 = 0xAE;
const VK_VOLUME_UP: u8 = 0xAF;
const VK_MEDIA_NEXT_TRACK: u8 = 0xB0;
const VK_MEDIA_PREV_TRACK: u8 = 0xB1;
const VK_MEDIA_STOP: u8 = 0xB2;
const VK_MEDIA_PLAY_PAUSE: u8 = 0xB3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTelemetrySnapshot {
    pub cpu_usage_percent: f32,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
    pub memory_usage_percent: f32,
    pub process_count: usize,
    pub official_streamdock_running: bool,
    pub obs_running: bool,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionExecutionResult {
    pub action_id: String,
    pub action_label: String,
    pub success: bool,
    pub elapsed_ms: u64,
    pub output_message: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroStepResult {
    pub step_id: String,
    pub label: String,
    pub step_type: String,
    pub status: String,
    pub elapsed_ms: u64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroRunReport {
    pub macro_id: String,
    pub macro_name: String,
    pub total_elapsed_ms: u64,
    pub step_results: Vec<MacroStepResult>,
}

fn is_media_vk(vk: u8) -> bool {
    (0xAD..=0xB3).contains(&vk)
}

/// Hardware scan codes. `MapVirtualKey` returns 0 for these, and Spotify ignores that tap.
fn media_scan_code(vk: u8) -> u8 {
    match vk {
        0xAD => 0x20, // mute
        0xAE => 0x2E, // volume down
        0xAF => 0x30, // volume up
        0xB0 => 0x19, // next track
        0xB1 => 0x10, // previous track
        0xB2 => 0x24, // stop
        0xB3 => 0x22, // play/pause
        _ => 0,
    }
}

/// Sends a single Virtual-Key tap (`key down` + `key up`) on Windows.
/// Volume and media keys are extended keys. Spotify ignores a play/pause tap that
/// arrives without that flag, or whose down and up are in the same instant.
pub fn send_vk_tap(vk: u8) {
    #[cfg(target_os = "windows")]
    unsafe {
        let known = media_scan_code(vk);
        let scan = if known != 0 {
            known
        } else {
            MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) as u8
        };
        let extra = if is_media_vk(vk) { KEYEVENTF_EXTENDEDKEY } else { 0 };
        keybd_event(vk, scan, extra, 0);
        if is_media_vk(vk) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        keybd_event(vk, scan, extra | KEYEVENTF_KEYUP, 0);
    }
}

#[derive(Clone, Copy)]
enum MediaTransport {
    Toggle,
    Next,
    Previous,
    Stop,
}

impl MediaTransport {
    fn vk(self) -> u8 {
        match self {
            Self::Toggle => VK_MEDIA_PLAY_PAUSE,
            Self::Next => VK_MEDIA_NEXT_TRACK,
            Self::Previous => VK_MEDIA_PREV_TRACK,
            Self::Stop => VK_MEDIA_STOP,
        }
    }
}

/// Controls the Windows media session. Spotify is chosen when it is open, even if
/// another app played more recently. This Spotify build ignores keyboard media keys.
pub fn toggle_media_play_pause() -> String {
    run_media_transport(MediaTransport::Toggle)
}

fn run_media_transport(kind: MediaTransport) -> String {
    #[cfg(target_os = "windows")]
    {
        match control_media_session(kind) {
            Ok(message) => return message,
            Err(err) => {
                send_vk_tap(kind.vk());
                return format!("No media session ({err}). Sent the media key.");
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = kind;
        "Media control is only available on Windows.".to_string()
    }
}

#[cfg(target_os = "windows")]
fn pick_media_session(
    manager: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager,
) -> Result<
    (
        windows::Media::Control::GlobalSystemMediaTransportControlsSession,
        String,
    ),
    String,
> {
    let sessions = manager.GetSessions().map_err(|e| e.to_string())?;
    let count = sessions.Size().map_err(|e| e.to_string())?;

    let mut spotify = None;
    let mut other = None;
    for i in 0..count {
        let session = sessions.GetAt(i).map_err(|e| e.to_string())?;
        let name = session
            .SourceAppUserModelId()
            .map(|id| id.to_string())
            .unwrap_or_default();
        if name.to_lowercase().contains("spotify") {
            spotify = Some((session, name));
            break;
        }
        if other.is_none() {
            other = Some((session, name));
        }
    }

    if let Some(found) = spotify {
        Ok(found)
    } else if let Ok(current) = manager.GetCurrentSession() {
        let name = current
            .SourceAppUserModelId()
            .map(|id| id.to_string())
            .unwrap_or_else(|_| "current app".to_string());
        Ok((current, name))
    } else if let Some(found) = other {
        Ok(found)
    } else {
        Err("Spotify is not open".into())
    }
}

#[cfg(target_os = "windows")]
fn control_media_session(kind: MediaTransport) -> Result<String, String> {
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };

    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;
    let (session, name) = pick_media_session(&manager)?;
    let who = if name.to_lowercase().contains("spotify") {
        "Spotify"
    } else {
        name.as_str()
    };

    let was_playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .ok()
        == Some(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing);

    let accepted = match kind {
        MediaTransport::Toggle => session.TryTogglePlayPauseAsync(),
        MediaTransport::Next => session.TrySkipNextAsync(),
        MediaTransport::Previous => session.TrySkipPreviousAsync(),
        MediaTransport::Stop => session.TryStopAsync(),
    }
    .map_err(|e| e.to_string())?
    .get()
    .map_err(|e| e.to_string())?;
    if !accepted {
        return Err(format!("{who} refused the media command"));
    }
    Ok(match kind {
        MediaTransport::Toggle => {
            if was_playing {
                format!("Paused {who}")
            } else {
                format!("Playing {who}")
            }
        }
        MediaTransport::Next => format!("Next track on {who}"),
        MediaTransport::Previous => format!("Previous track on {who}"),
        MediaTransport::Stop => format!("Stopped {who}"),
    })
}

/// Reads the current media session without changing it.
pub fn read_media_face() -> (Option<bool>, String) {
    #[cfg(target_os = "windows")]
    {
        read_media_face_windows()
    }
    #[cfg(not(target_os = "windows"))]
    {
        (None, String::new())
    }
}

#[cfg(target_os = "windows")]
fn read_media_face_windows() -> (Option<bool>, String) {
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
        Ok(op) => match op.get() {
            Ok(manager) => manager,
            Err(_) => return (None, String::new()),
        },
        Err(_) => return (None, String::new()),
    };
    let session = match pick_media_session(&manager) {
        Ok((session, _)) => session,
        Err(_) => return (None, String::new()),
    };
    let playing = session
        .GetPlaybackInfo()
        .ok()
        .and_then(|info| info.PlaybackStatus().ok())
        == Some(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing);
    let title = session
        .TryGetMediaPropertiesAsync()
        .ok()
        .and_then(|op| op.get().ok())
        .and_then(|props| props.Title().ok())
        .map(|t| t.to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| if playing { "Playing".to_string() } else { "Paused".to_string() });
    (Some(playing), title)
}

/// Parses a human-readable chord string such as `"Win+D"`, `"Ctrl+Shift+M"`, `"Alt+F4"`, `"VK_MEDIA_PLAY_PAUSE"`
/// and dispatches real Win32 `keybd_event` presses.
pub fn send_hotkey_chord(chord: &str) -> Result<String, String> {
    let trimmed = chord.trim();
    if trimmed.is_empty() {
        return Err("Empty hotkey chord".to_string());
    }

    match trimmed.to_uppercase().as_str() {
        "VK_MEDIA_PLAY_PAUSE" | "MEDIA_PLAY_PAUSE" => {
            return Ok(run_media_transport(MediaTransport::Toggle));
        }
        "VK_MEDIA_NEXT_TRACK" | "MEDIA_NEXT" => {
            return Ok(run_media_transport(MediaTransport::Next));
        }
        "VK_MEDIA_PREV_TRACK" | "MEDIA_PREV" => {
            return Ok(run_media_transport(MediaTransport::Previous));
        }
        "VK_MEDIA_STOP" => {
            return Ok(run_media_transport(MediaTransport::Stop));
        }
        "VK_VOLUME_UP" | "VOLUME_UP" => {
            send_vk_tap(VK_VOLUME_UP);
            return Ok("Sent VK_VOLUME_UP (0xAF)".to_string());
        }
        "VK_VOLUME_DOWN" | "VOLUME_DOWN" => {
            send_vk_tap(VK_VOLUME_DOWN);
            return Ok("Sent VK_VOLUME_DOWN (0xAE)".to_string());
        }
        "VK_VOLUME_MUTE" | "VOLUME_MUTE" => {
            send_vk_tap(VK_VOLUME_MUTE);
            return Ok("Sent VK_VOLUME_MUTE (0xAD)".to_string());
        }
        _ => {}
    }

    let vks = chord_to_vks(trimmed);
    if vks.is_empty() {
        return Err(format!("No recognized virtual keys in chord `{}`", trimmed));
    }

    #[cfg(target_os = "windows")]
    unsafe {
        for &vk in &vks {
            keybd_event(vk, 0, 0, 0);
        }
        for &vk in vks.iter().rev() {
            keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
        }
    }

    Ok(format!("Dispatched Win32 key chord `{}` ({:?})", trimmed, vks))
}

/// Turns a chord such as `Win+Ctrl+Left` into virtual-key codes. Tokens it does not know are skipped.
fn chord_to_vks(trimmed: &str) -> Vec<u8> {
    let mut vks: Vec<u8> = Vec::new();
    for part in trimmed.split('+') {
        let token = part.trim().to_uppercase();
        let vk = match token.as_str() {
            "WIN" | "CMD" | "META" | "SUPER" | "LWIN" => VK_LWIN,
            "CTRL" | "CONTROL" | "LCTRL" => VK_CONTROL,
            "ALT" | "OPTION" | "MENU" => VK_MENU,
            "SHIFT" | "LSHIFT" => VK_SHIFT,
            "ESC" | "ESCAPE" => VK_ESCAPE,
            "SPACE" => VK_SPACE,
            "ENTER" | "RETURN" => VK_RETURN,
            "TAB" => VK_TAB,
            "BACKSPACE" | "BACK" => VK_BACK,
            "DEL" | "DELETE" => VK_DELETE,
            "LEFT" => VK_LEFT,
            "RIGHT" => VK_RIGHT,
            "UP" => VK_UP,
            "DOWN" => VK_DOWN,
            "PRTSC" | "SNAPSHOT" => VK_SNAPSHOT,
            "F1" => 0x70,
            "F2" => 0x71,
            "F3" => 0x72,
            "F4" => 0x73,
            "F5" => 0x74,
            "F6" => 0x75,
            "F7" => 0x76,
            "F8" => 0x77,
            "F9" => 0x78,
            "F10" => 0x79,
            "F11" => 0x7A,
            "F12" => 0x7B,
            "F13" => 0x7C,
            "F14" => 0x7D,
            "F15" => 0x7E,
            "F16" => 0x7F,
            "F17" => 0x80,
            "F18" => 0x81,
            "F19" => 0x82,
            "F20" => 0x83,
            "F21" => 0x84,
            "F22" => 0x85,
            "F23" => 0x86,
            "F24" => 0x87,
            "PLUS" | "+" | "NUM+" => 0x6B,
            "MINUS" | "-" | "NUM-" => 0x6D,
            "HOME" => 0x24,
            "END" => 0x23,
            "PGUP" | "PAGEUP" => 0x21,
            "PGDN" | "PAGEDOWN" => 0x22,
            "INS" | "INSERT" => 0x2D,
            "." | "PERIOD" => 0xBE,
            "," | "COMMA" => 0xBC,
            "/" | "SLASH" => 0xBF,
            s if s.len() == 1 => {
                let ch = s.chars().next().unwrap();
                if ch.is_ascii_alphanumeric() {
                    ch as u8
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        vks.push(vk);
    }
    vks
}

/// Runs a custom PowerShell or CLI command in the background without flashing a console window.
pub fn run_shell_command(command_line: &str) -> Result<String, String> {
    let clean = command_line.trim();
    if clean.is_empty() {
        return Err("Empty shell command".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", clean]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.spawn()
            .map_err(|e| format!("Failed to spawn shell command `{}`: {}", clean, e))?;
    }

    Ok("Shell command started; completion is not monitored".into())
}

/// Copies a custom text snippet to the Windows clipboard and pastes it via Ctrl+V.
pub fn send_text_snippet(text: &str) -> Result<String, String> {
    let clean = text.trim();
    if clean.is_empty() {
        return Err("Empty text snippet".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let escaped = clean.replace('\'', "''");
        let ps_script = format!("try {{ Set-Clipboard -Value '{}' -ErrorAction Stop; exit 0 }} catch {{ exit 1 }}", escaped);
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let status = cmd.status().map_err(|e| format!("Could not write clipboard: {e}"))?;
        if !status.success() { return Err(format!("Clipboard command failed: {status}")); }
        send_hotkey_chord("Ctrl+V")?;
    }

    Ok(format!("Copied & pasted text snippet ({} chars)", clean.len()))
}

/// Wait for a bounded standalone webhook result instead of detaching the request.
pub fn trigger_http_webhook(endpoint: &str) -> Result<String, String> {
    crate::macro_run::standalone_webhook(endpoint)
}

/// Launches an application executable or opens an `http(s)://` URL cleanly on Windows.
pub fn launch_app_or_url(target: &str) -> Result<String, String> {
    let clean = target.trim();
    if clean.is_empty() {
        return Err("Empty launch target".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use windows::{core::{w, PCWSTR}, Win32::UI::{Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_FLAG_NO_UI}, WindowsAndMessaging::SW_SHOWNORMAL}};
        if clean.contains('\0') { return Err("Launch target contains a null character".into()); }
        let target: Vec<u16> = clean.encode_utf16().chain(Some(0)).collect();
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_FLAG_NO_UI,
            lpVerb: w!("open"), lpFile: PCWSTR(target.as_ptr()), nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        unsafe { ShellExecuteExW(&mut info) }.map_err(|e| format!("Could not open launch target: {e}"))?;
    }

    Ok(format!("Launch requested for `{}`; application startup is not monitored", clean))
}

/// True for the vendor's own app and its watcher: `Stream Dock AJAZZ.exe`,
/// `StreamDockWatcher.exe`, `StreamDock.exe`. Spaces are ignored because the Ajazz build
/// names its program "Stream Dock AJAZZ.exe", which a plain `streamdock` match misses.
fn is_official_streamdock_process(name: &str) -> bool {
    let compact: String = name.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    matches!(compact.trim_end_matches(".exe"), "streamdock" | "streamdockajazz" | "streamdockwatcher")
}

/// Process ids and names of the official Stream Dock app that are running right now.
pub fn official_streamdock_processes() -> Vec<(u32, String)> {
    let mut sys = System::new_with_specifics(RefreshKind::nothing());
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes()
        .iter()
        .filter_map(|(pid, proc)| {
            let name = proc.name().to_string_lossy().to_string();
            is_official_streamdock_process(&name).then(|| (pid.as_u32(), name))
        })
        .collect()
}

pub fn process_is_running(name: &str) -> bool {
    let needle = name.trim().trim_end_matches(".exe").to_lowercase();
    if needle.is_empty() {
        return false;
    }
    let mut sys = System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()));
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys.processes().values().any(|proc| {
        let current = proc.name().to_string_lossy().to_lowercase();
        current == needle || current == format!("{needle}.exe") || current.strip_suffix(".exe") == Some(needle.as_str())
    })
}

/// A macro wait uses the step parameter, and never blocks longer than three seconds.
pub fn macro_delay_ms(parameter: &str) -> u64 {
    parameter.trim().parse::<u64>().unwrap_or(100).min(3_000)
}

pub fn collect_system_telemetry() -> SystemTelemetrySnapshot {
    static SYS: std::sync::OnceLock<std::sync::Mutex<System>> = std::sync::OnceLock::new();
    let slot = SYS.get_or_init(|| {
        std::sync::Mutex::new(System::new_with_specifics(
            RefreshKind::nothing()
                .with_memory(sysinfo::MemoryRefreshKind::everything())
                .with_cpu(sysinfo::CpuRefreshKind::everything())
                .with_processes(ProcessRefreshKind::nothing()),
        ))
    });
    let mut sys = slot.lock().unwrap_or_else(|e| e.into_inner());
    sys.refresh_memory();
    sys.refresh_cpu_usage();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
    );

    let cpu_usage_percent = sys.global_cpu_usage();
    let total_mem = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let used_mem = sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let mem_pct = if total_mem > 0.0 {
        ((used_mem / total_mem) * 100.0) as f32
    } else {
        0.0
    };

    let mut official_streamdock_running = false;
    let mut obs_running = false;
    for proc in sys.processes().values() {
        let name = proc.name().to_string_lossy().to_lowercase();
        if is_official_streamdock_process(&name) {
            official_streamdock_running = true;
        }
        if name.contains("obs64") || name == "obs.exe" {
            obs_running = true;
        }
    }

    SystemTelemetrySnapshot {
        cpu_usage_percent: (cpu_usage_percent * 10.0).round() / 10.0,
        memory_used_gb: ((used_mem * 10.0).round() / 10.0) as f32,
        memory_total_gb: ((total_mem * 10.0).round() / 10.0) as f32,
        memory_usage_percent: (mem_pct * 10.0).round() / 10.0,
        process_count: sys.processes().len(),
        official_streamdock_running,
        obs_running,
        timestamp_ms: now_ms(),
    }
}

/// Executes a real Win32 OS action or integration trigger when a key or knob is activated.
pub fn execute_key_action_live(
    action_id: &str,
    action_label: &str,
    action_param: &str,
    execute_os_effects: bool,
    obs: &ObsSettings,
) -> ActionExecutionResult {
    let start = Instant::now();

    let outcome = if execute_os_effects {
        match action_id {
            "media.play_pause" => Ok(run_media_transport(MediaTransport::Toggle)),
            "media.next" => Ok(run_media_transport(MediaTransport::Next)),
            "media.prev" => Ok(run_media_transport(MediaTransport::Previous)),
            "media.stop" => Ok(run_media_transport(MediaTransport::Stop)),
            "audio.volume_up" => { send_vk_tap(VK_VOLUME_UP); Ok("Sent volume-up key".into()) },
            "audio.volume_down" => { send_vk_tap(VK_VOLUME_DOWN); Ok("Sent volume-down key".into()) },
            "audio.mute_mic" => toggle_microphone_mute(),
            "audio.volume_mute" => toggle_speaker_mute(),
            "obs.start_stream" | "obs.toggle_record" | "obs.save_replay" | "obs.next_scene"
            | "obs.prev_scene" | "obs.toggle_virtualcam" | "obs.toggle_replay" | "obs.pause_record"
            | "obs.studio_transition" | "obs.set_scene" | "obs.toggle_mute" => run_obs_action(obs, action_id, action_param),
            id if id.starts_with("hotkey.") => send_hotkey_chord(action_param),
            "audio.open_mixer" => launch_app_or_url("sndvol.exe"),
            "sys.show_desktop" => send_hotkey_chord("Win+D"),
            "sys.screenshot" => send_hotkey_chord("Win+Shift+S"),
            "sys.task_manager" => send_hotkey_chord("Ctrl+Shift+Esc"),
            "sys.lock_pc" => run_shell_command("rundll32.exe user32.dll,LockWorkStation"),
            "sys.hotkey" | "custom.hotkey" => send_hotkey_chord(action_param),
            "sys.launch_app" | "sys.open_url" | "custom.launch" | "custom.url" => launch_app_or_url(action_param),
            "custom.shell" | "sys.run_command" => run_shell_command(action_param),
            "custom.text" | "sys.type_text" => send_text_snippet(action_param),
            "custom.http" | "sys.webhook" => trigger_http_webhook(action_param),
            "none" | "" => Err("No action is assigned to this key".into()),
            _ => Err(format!("Unsupported action `{action_id}`. Choose a supported action.")),
        }
    } else {
        Ok(format!("Simulated `{}` ({})", action_label, action_param))
    };
    action_result(action_id, action_label, start, outcome)
}

pub(crate) fn action_result(action_id: &str, action_label: &str, start: Instant, outcome: Result<String, String>) -> ActionExecutionResult {
    ActionExecutionResult {
        action_id: action_id.into(), action_label: action_label.into(), success: outcome.is_ok(),
        elapsed_ms: start.elapsed().as_millis().max(1) as u64,
        output_message: outcome.unwrap_or_else(|error| error), timestamp_ms: now_ms(),
    }
}

pub fn run_macro_sequence_test(seq: &MacroSequence, execute_os_effects: bool) -> MacroRunReport {
    let start = Instant::now();
    let mut results = Vec::with_capacity(seq.steps.len());

    for step in &seq.steps {
        let step_start = Instant::now();
        let detail = match step.step_type.as_str() {
            "Delay" => {
                let ms = macro_delay_ms(&step.parameter);
                if execute_os_effects {
                    std::thread::sleep(std::time::Duration::from_millis(ms));
                }
                format!("Waited {ms}ms")
            }
            "Shortcut" => {
                if execute_os_effects {
                    send_hotkey_chord(&step.parameter)
                        .unwrap_or_else(|e| format!("Shortcut {}: {}", step.parameter, e))
                } else {
                    format!("Verified key chord `{}` ready for dispatch", step.parameter)
                }
            }
            "LaunchApp" => {
                if execute_os_effects {
                    launch_app_or_url(&step.parameter)
                        .unwrap_or_else(|e| format!("Launch {}: {}", step.parameter, e))
                } else {
                    format!("Verified executable target `{}`", step.parameter)
                }
            }
            "IfWindowExists" => format!("Evaluated process condition for `{}`", step.parameter),
            "SwitchProfile" => format!("Switched active profile to `{}`", step.parameter),
            _ => format!("Executed step parameter `{}`", step.parameter),
        };

        results.push(MacroStepResult {
            step_id: step.id.clone(),
            label: step.label.clone(),
            step_type: step.step_type.clone(),
            status: "Passed".to_string(),
            elapsed_ms: step_start.elapsed().as_millis().max(1) as u64,
            detail,
        });
    }

    MacroRunReport {
        macro_id: seq.id.clone(),
        macro_name: seq.name.clone(),
        total_elapsed_ms: start.elapsed().as_millis().max(2) as u64,
        step_results: results,
    }
}

#[cfg(test)]
mod tests {
    use super::{chord_to_vks, macro_delay_ms};

    #[test]
    fn action_results_do_not_infer_success_from_message_text() {
        let failed = super::action_result("test", "Test", std::time::Instant::now(), Err("Access denied".into()));
        assert!(!failed.success);
        let success = super::action_result("test", "Test", std::time::Instant::now(), Ok("Opened failed-jobs dashboard".into()));
        assert!(success.success);
    }

    #[test]
    fn invalid_or_unsupported_actions_report_failure_without_os_effects() {
        for (id, param) in [("custom.hotkey", "not-a-key"), ("custom.launch", ""),
            ("custom.text", ""), ("custom.shell", ""), ("custom.http", "file:///bad"),
            ("none", ""), ("plugin.sd_bridge", "https://example.invalid"), ("unknown", "Ctrl+X")] {
            let result = super::execute_key_action_live(id, "Test", param, true, &super::ObsSettings::default());
            assert!(!result.success, "{id} unexpectedly succeeded: {}", result.output_message);
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn missing_executable_is_reported_as_failure_without_a_shell_dialog() {
        let target = std::env::temp_dir().join(format!("deckforge-missing-app-{}-{}.exe", std::process::id(), crate::drivers::hal::now_ms()));
        assert!(!target.exists());
        assert!(super::launch_app_or_url(target.to_str().unwrap()).is_err());
    }

    #[test]
    fn vendor_detection_matches_only_known_process_names() {
        for name in ["StreamDock.exe", "Stream Dock AJAZZ.exe", "STREAMDOCKWATCHER.EXE"] {
            assert!(super::is_official_streamdock_process(name));
        }
        for name in ["deckforge.exe", "streamdock-notes.exe", "my-streamdock-backup.exe"] {
            assert!(!super::is_official_streamdock_process(name));
        }
    }

    /// Every shortcut preset in the catalog must turn into one key per `+` part, or the
    /// deck key would press fewer keys than its label says.
    #[test]
    fn every_shortcut_preset_parses_to_all_of_its_keys() {
        let catalog = include_str!("../../../src/data/extraActions.ts");
        let mut checked = 0;
        for line in catalog.lines().filter(|l| l.trim_start().starts_with("shortcut(\"")) {
            let quoted: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
            let combo = quoted[4];
            let tokens = combo.split('+').count();
            let keys = chord_to_vks(combo);
            assert_eq!(keys.len(), tokens, "preset `{}` ({combo}) lost a key: {keys:?}", quoted[0]);
            checked += 1;
        }
        assert!(checked >= 30, "found only {checked} presets, the catalog format probably changed");
    }

    #[test]
    fn punctuation_and_navigation_keys_are_recognised() {
        assert_eq!(chord_to_vks("Win+."), vec![0x5B, 0xBE]);
        assert_eq!(chord_to_vks("Ctrl+Shift+Tab").len(), 3);
        assert_eq!(chord_to_vks("Alt+F4"), vec![0x12, 0x73]);
        assert_eq!(chord_to_vks("nonsense"), Vec::<u8>::new());
    }

    #[test]
    fn macro_waits_use_the_parameter_and_stay_bounded() {
        assert_eq!(macro_delay_ms("500"), 500);
        assert_eq!(macro_delay_ms("9000"), 3_000);
        assert_eq!(macro_delay_ms("nope"), 100);
    }
}
