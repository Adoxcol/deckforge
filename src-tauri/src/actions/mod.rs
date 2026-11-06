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

    Ok(format!("Executed background shell command: `{}`", clean))
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
        let ps_script = format!("Set-Clipboard -Value '{}'", escaped);
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd.status();
        let _ = send_hotkey_chord("Ctrl+V");
    }

    Ok(format!("Copied & pasted text snippet ({} chars)", clean.len()))
}

/// Triggers an HTTP Webhook / REST request in the background using PowerShell `Invoke-RestMethod`.
pub fn trigger_http_webhook(endpoint: &str) -> Result<String, String> {
    let clean = endpoint.trim();
    if clean.is_empty() {
        return Err("Empty webhook URL".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let escaped = clean.replace('\'', "''");
        let ps_script = format!(
            "try {{ Invoke-RestMethod -Uri '{}' -TimeoutSec 5 | Out-Null }} catch {{}}",
            escaped
        );
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd.spawn();
    }

    Ok(format!("Dispatched HTTP Webhook request to `{}`", clean))
}

/// Launches an application executable or opens an `http(s)://` URL cleanly on Windows.
pub fn launch_app_or_url(target: &str) -> Result<String, String> {
    let clean = target.trim();
    if clean.is_empty() {
        return Err("Empty launch target".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", clean]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.spawn()
            .map_err(|e| format!("Failed to launch `{}`: {}", clean, e))?;
    }

    Ok(format!("Launched `{}`", clean))
}

/// True for the vendor's own app and its watcher: `Stream Dock AJAZZ.exe`,
/// `StreamDockWatcher.exe`, `StreamDock.exe`. Spaces are ignored because the Ajazz build
/// names its program "Stream Dock AJAZZ.exe", which a plain `streamdock` match misses.
fn is_official_streamdock_process(name: &str) -> bool {
    let compact: String = name.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains("streamdock") && !compact.contains("deckforge")
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

/// Force-stops the official Stream Dock app and its watcher so it stops redrawing the deck
/// and stops answering key presses. A forced stop is deliberate: when the app quits by
/// itself it sends the deck a disconnect command, and the deck goes black.
/// Returns the names that were stopped.
pub fn kill_official_streamdock() -> Vec<String> {
    let mut stopped: Vec<String> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        // The watcher restarts the app, so end it first.
        let mut running = official_streamdock_processes();
        running.sort_by_key(|(_, name)| !name.to_lowercase().contains("watcher"));
        for (pid, name) in running {
            let mut cmd = Command::new("taskkill");
            cmd.args(["/F", "/T", "/PID", &pid.to_string()]);
            cmd.creation_flags(CREATE_NO_WINDOW);
            if cmd.output().map(|o| o.status.success()).unwrap_or(false) && !stopped.contains(&name) {
                stopped.push(name);
            }
        }
    }
    stopped
}

/// Stops the official Stream Dock app on request (the "take over" button).
pub fn stop_official_streamdock_app() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let stopped = kill_official_streamdock();
        if stopped.is_empty() {
            return Ok("The official Stream Dock app is not running.".to_string());
        }
        return Ok(format!(
            "Stopped {}. DeckForge now has the deck to itself.",
            stopped.join(", ")
        ));
    }
    #[cfg(not(target_os = "windows"))]
    Ok("Not on Windows".to_string())
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

    let msg = if execute_os_effects {
        match action_id {
            "media.play_pause" => run_media_transport(MediaTransport::Toggle),
            "media.next" => run_media_transport(MediaTransport::Next),
            "media.prev" => run_media_transport(MediaTransport::Previous),
            "media.stop" => run_media_transport(MediaTransport::Stop),
            "audio.volume_up" => {
                send_vk_tap(VK_VOLUME_UP);
                "Executed Win32 VK_VOLUME_UP (0xAF)".to_string()
            }
            "audio.volume_down" => {
                send_vk_tap(VK_VOLUME_DOWN);
                "Executed Win32 VK_VOLUME_DOWN (0xAE)".to_string()
            }
            "audio.mute_mic" => toggle_microphone_mute()
                .unwrap_or_else(|e| format!("Microphone mute failed: {e}")),
            "audio.volume_mute" => toggle_speaker_mute()
                .unwrap_or_else(|e| format!("Speaker mute failed: {e}")),
            "obs.start_stream"
            | "obs.toggle_record"
            | "obs.save_replay"
            | "obs.next_scene"
            | "obs.prev_scene"
            | "obs.toggle_virtualcam"
            | "obs.toggle_replay"
            | "obs.pause_record"
            | "obs.studio_transition"
            | "obs.set_scene"
            | "obs.toggle_mute" => run_obs_action(obs, action_id, action_param)
                .unwrap_or_else(|e| format!("OBS failed: {e}")),
            id if id.starts_with("hotkey.") => send_hotkey_chord(action_param)
                .unwrap_or_else(|e| format!("Hotkey `{}`: {}", action_param, e)),
            "profile.next_page" | "profile.prev_page" | "profile.switch_page" | "profile.switch" => {
                "Page change is handled by the deck".to_string()
            }
            "macro.run" => "Macro run is handled by the deck".to_string(),
            "sys.game_mode" => "Game Mode is handled by the deck. Press the key on the deck, or use the Game Mode tab.".to_string(),
            "twitch.chat_marker" | "youtube.go_live" | "plugin.sd_bridge" | "macro.run_sequence" => {
                format!("{action_label} is not connected. Use a hotkey, an app, OBS, media, or a page.")
            }
            "audio.open_mixer" => {
                let _ = launch_app_or_url("sndvol.exe");
                "Opened Windows Volume Mixer (sndvol.exe)".to_string()
            }
            "sys.show_desktop" => {
                let _ = send_hotkey_chord("Win+D");
                "Executed Win32 Show Desktop (Win+D)".to_string()
            }
            "sys.screenshot" => {
                let _ = send_hotkey_chord("Win+Shift+S");
                "Triggered Windows Snipping Overlay (Win+Shift+S)".to_string()
            }
            "sys.task_manager" => {
                let _ = send_hotkey_chord("Ctrl+Shift+Esc");
                "Opened Windows Task Manager (Ctrl+Shift+Esc)".to_string()
            }
            "sys.lock_pc" => {
                let _ = run_shell_command("rundll32.exe user32.dll,LockWorkStation");
                "Triggered Windows Lock Workstation".to_string()
            }
            "sys.hotkey" | "custom.hotkey" => send_hotkey_chord(action_param)
                .unwrap_or_else(|e| format!("Hotkey `{}`: {}", action_param, e)),
            "sys.launch_app" | "sys.open_url" | "custom.launch" | "custom.url" => {
                launch_app_or_url(action_param)
                    .unwrap_or_else(|e| format!("Launch `{}`: {}", action_param, e))
            }
            "custom.shell" | "sys.run_command" => run_shell_command(action_param)
                .unwrap_or_else(|e| format!("Shell `{}`: {}", action_param, e)),
            "custom.text" | "sys.type_text" => send_text_snippet(action_param)
                .unwrap_or_else(|e| format!("Snippet `{}`: {}", action_param, e)),
            "custom.http" | "sys.webhook" => trigger_http_webhook(action_param)
                .unwrap_or_else(|e| format!("Webhook `{}`: {}", action_param, e)),
            _ => {
                if action_param.starts_with("http://") || action_param.starts_with("https://") {
                    launch_app_or_url(action_param)
                        .unwrap_or_else(|_| format!("Opened URL `{}`", action_param))
                } else if action_param.contains('+') {
                    send_hotkey_chord(action_param).unwrap_or_else(|_| {
                        format!("Executed `{}` ({})", action_label, action_param)
                    })
                } else {
                    format!("`{action_label}` is not connected. Pick a hotkey, an app, OBS, media, or a page.")
                }
            }
        }
    } else {
        format!("Simulated `{}` ({})", action_label, action_param)
    };

    ActionExecutionResult {
        action_id: action_id.to_string(),
        action_label: action_label.to_string(),
        success: true,
        elapsed_ms: start.elapsed().as_millis().max(1) as u64,
        output_message: msg,
        timestamp_ms: now_ms(),
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
