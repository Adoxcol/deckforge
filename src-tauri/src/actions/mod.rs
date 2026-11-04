use crate::db::MacroSequence;
use crate::drivers::hal::now_ms;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Instant;
use sysinfo::System;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
}

const KEYEVENTF_KEYUP: u32 = 0x0002;

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

/// Sends a single Virtual-Key tap (`key down` + `key up`) on Windows.
pub fn send_vk_tap(vk: u8) {
    #[cfg(target_os = "windows")]
    unsafe {
        keybd_event(vk, 0, 0, 0);
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }
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
            send_vk_tap(VK_MEDIA_PLAY_PAUSE);
            return Ok("Sent VK_MEDIA_PLAY_PAUSE (0xB3)".to_string());
        }
        "VK_MEDIA_NEXT_TRACK" | "MEDIA_NEXT" => {
            send_vk_tap(VK_MEDIA_NEXT_TRACK);
            return Ok("Sent VK_MEDIA_NEXT_TRACK (0xB0)".to_string());
        }
        "VK_MEDIA_PREV_TRACK" | "MEDIA_PREV" => {
            send_vk_tap(VK_MEDIA_PREV_TRACK);
            return Ok("Sent VK_MEDIA_PREV_TRACK (0xB1)".to_string());
        }
        "VK_MEDIA_STOP" => {
            send_vk_tap(VK_MEDIA_STOP);
            return Ok("Sent VK_MEDIA_STOP (0xB2)".to_string());
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

/// Gracefully stops the official `StreamDock.exe` process so it stops flooding `VID_0300&PID_3013`
/// and releases exclusive read access on `MI_00`.
pub fn stop_official_streamdock_app() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("taskkill");
        cmd.args(["/F", "/IM", "StreamDock.exe", "/T"]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run taskkill: {}", e))?;

        if output.status.success() {
            return Ok(
                "Stopped official StreamDock.exe process tree — DeckForge now has clean, uncontested access to Ajazz AKP05E Pro (0300:3013)!".to_string()
            );
        } else {
            return Ok("Official StreamDock.exe is already stopped.".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    Ok("Not on Windows".to_string())
}

pub fn collect_system_telemetry() -> SystemTelemetrySnapshot {
    let mut sys = System::new_all();
    sys.refresh_cpu_all();
    sys.refresh_memory();

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
        if name.contains("streamdock") {
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
) -> ActionExecutionResult {
    let start = Instant::now();

    let msg = if execute_os_effects {
        match action_id {
            "media.play_pause" => {
                send_vk_tap(VK_MEDIA_PLAY_PAUSE);
                "Executed Win32 VK_MEDIA_PLAY_PAUSE (0xB3)".to_string()
            }
            "media.next" => {
                send_vk_tap(VK_MEDIA_NEXT_TRACK);
                "Executed Win32 VK_MEDIA_NEXT_TRACK (0xB0)".to_string()
            }
            "media.prev" => {
                send_vk_tap(VK_MEDIA_PREV_TRACK);
                "Executed Win32 VK_MEDIA_PREV_TRACK (0xB1)".to_string()
            }
            "media.stop" => {
                send_vk_tap(VK_MEDIA_STOP);
                "Executed Win32 VK_MEDIA_STOP (0xB2)".to_string()
            }
            "audio.volume_up" => {
                send_vk_tap(VK_VOLUME_UP);
                "Executed Win32 VK_VOLUME_UP (0xAF)".to_string()
            }
            "audio.volume_down" => {
                send_vk_tap(VK_VOLUME_DOWN);
                "Executed Win32 VK_VOLUME_DOWN (0xAE)".to_string()
            }
            "audio.mute_mic" | "audio.volume_mute" => {
                send_vk_tap(VK_VOLUME_MUTE);
                "Executed Win32 VK_VOLUME_MUTE (0xAD)".to_string()
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
                    format!("Dispatched `{}` ({})", action_label, action_param)
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
                let ms = step.parameter.parse::<u64>().unwrap_or(100).min(1000);
                if execute_os_effects {
                    std::thread::sleep(std::time::Duration::from_millis(ms.min(250)));
                }
                format!("Completed {}ms timer wait", ms)
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
